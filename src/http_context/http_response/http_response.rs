use http::{HeaderMap, HeaderValue, StatusCode};
use http_body_util::channel::{Channel, Sender};
use hyper::body::Bytes;
use std::io;
use tokio::sync::oneshot;

use crate::http_context::http_header::HttpHeader;

/// Chunk size (64KB) used to write a complete body into the stream.
const BODY_CHUNK_SIZE: usize = 64 * 1024;
/// Chunks buffered between the handler and the connection, a full buffer makes `write_async` wait for a slow client.
const STREAM_BUFFER_CHUNKS: usize = 4;

/// Body of the response, filled by `HttpResponse` and read by hyper.
pub(crate) type ResponseBody = Channel<Bytes, io::Error>;
/// Status line, headers and the body stream, given to hyper when the response head is committed.
pub(crate) type ResponseHead = http::Response<ResponseBody>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum WritenPhase {
    NONE = 0,
    HTTP_HEADERS = 1,
    HTTP_BODY = 2,
    END = 255,
}

/// The response of a request, it is written in one of two ways:
/// - complete: set `status_code`, `headers` and `body` (a controller result does this), everything is written to the stream when the pipeline ends.
/// - streamed: call `write_async` (or `start_async`), the status and headers are sent with the first call and can not be changed after it.
///   A status that is still the default (204) is sent as 200.
pub struct HttpResponse {
    pub status_code: StatusCode,
    pub headers: HeaderMap<HeaderValue>,
    pub body: Vec<u8>,
    pub written_phase: WritenPhase,
    /// Waits for the head, it is `None` once the head is sent.
    head_tx: Option<oneshot::Sender<ResponseHead>>,
    /// Receiver half of the stream, moved into the head when it is sent.
    pending_body: Option<ResponseBody>,
    /// Sender half of the stream, it is `None` once the stream is closed.
    body_tx: Option<Sender<Bytes, io::Error>>,
    /// The status that was really sent, `status_code` can still be changed after the head is sent.
    committed_status: Option<StatusCode>,
}

fn closed_error() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "response stream is closed (the client disconnected or the response is finished)")
}

/// Statuses that never have a body (RFC 9110).
fn status_forbids_body(status: StatusCode) -> bool {
    status.is_informational() || status == StatusCode::NO_CONTENT || status == StatusCode::NOT_MODIFIED
}

impl HttpResponse {
    /// Creates the response and the receiver of its head, the receiver gets the head when the first byte is written or the response ends.
    pub(crate) fn new_streaming() -> (Self, oneshot::Receiver<ResponseHead>) {
        let mut headers = HeaderMap::new();
        headers.insert("server", "ASP.RS".parse().unwrap());
        let (head_tx, head_rx) = oneshot::channel();
        let (body_tx, body_rx) = Channel::new(STREAM_BUFFER_CHUNKS);
        let response = Self {
            status_code: StatusCode::NO_CONTENT,
            headers,
            body: Vec::new(),
            written_phase: WritenPhase::NONE,
            head_tx: Some(head_tx),
            pending_body: Some(body_rx),
            body_tx: Some(body_tx),
            committed_status: None,
        };
        (response, head_rx)
    }

    /// A response with status 500 and an empty body, used when the pipeline ends without any response.
    pub(crate) fn internal_server_error() -> ResponseHead {
        let (_, empty_body) = Channel::new(1);
        let mut response = http::Response::new(empty_body);
        *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
        response.headers_mut().set_content_length(0);
        response
    }

    /// `true` once the status and headers are sent (like `HasStarted` of ASP.NET), they can not be changed after that.
    pub fn has_started(&self) -> bool {
        self.written_phase >= WritenPhase::HTTP_HEADERS
    }

    /// The status that is on the wire, or `status_code` if the head is not sent yet.
    pub(crate) fn final_status(&self) -> StatusCode {
        self.committed_status.unwrap_or(self.status_code)
    }

    /// Sends the status and headers, `content_length` (when known) replaces the Content-Length header.
    fn commit(&mut self, content_length: Option<usize>) -> io::Result<()> {
        if self.has_started() {
            return Ok(());
        }
        let (Some(head_tx), Some(body)) = (self.head_tx.take(), self.pending_body.take()) else {
            return Err(closed_error());
        };
        match content_length {
            // streaming: a status that was never set is still the default 204, which can not have a body
            None if self.status_code == StatusCode::NO_CONTENT => self.status_code = StatusCode::OK,
            None => {}
            Some(_) if status_forbids_body(self.status_code) => {
                self.headers.remove(http::header::CONTENT_LENGTH);
            }
            Some(length) => self.headers.set_content_length(length),
        }
        let mut head = http::Response::new(body);
        *head.status_mut() = self.status_code;
        *head.headers_mut() = self.headers.clone();
        // the connection and the Keep-Alive header are managed by hyper, never set them here
        if head_tx.send(head).is_err() {
            self.body_tx = None;
            return Err(closed_error());
        }
        self.committed_status = Some(self.status_code);
        self.written_phase = WritenPhase::HTTP_HEADERS;
        Ok(())
    }

    async fn send_chunk(&mut self, chunk: Bytes) -> io::Result<()> {
        let Some(body_tx) = self.body_tx.as_mut() else {
            return Err(closed_error());
        };
        if body_tx.send_data(chunk).await.is_err() {
            self.body_tx = None;
            return Err(closed_error());
        }
        if self.written_phase < WritenPhase::HTTP_BODY {
            self.written_phase = WritenPhase::HTTP_BODY;
        }
        Ok(())
    }

    /// Sends the status and headers now (like `StartAsync` of ASP.NET), without a body. The headers are sent as they are, no Content-Length is added.
    pub async fn start_async(&mut self) -> io::Result<()> {
        self.commit(None)
    }

    /// Writes `data` to the stream. The first call sends the status and headers (no Content-Length is added, so the body is chunked unless you set it).
    /// Waits when the client is slow, returns an error when the client is gone, so a loop can stop.
    pub async fn write_async(&mut self, data: impl AsRef<[u8]>) -> io::Result<()> {
        let data = data.as_ref();
        self.commit(None)?;
        if data.is_empty() {
            return Ok(());
        }
        self.send_chunk(Bytes::copy_from_slice(data)).await
    }

    /// Same as `write_async` but takes the bytes without copying them (`Vec<u8>`, `String`, `&'static str`, `Bytes`).
    pub async fn write_bytes_async(&mut self, data: impl Into<Bytes>) -> io::Result<()> {
        let data = data.into();
        self.commit(None)?;
        if data.is_empty() {
            return Ok(());
        }
        self.send_chunk(data).await
    }

    /// Writes the bytes of `body` to the stream, the head is sent first with Content-Length = `body.len()` if it is not sent yet.
    async fn flush_body_async(&mut self) -> io::Result<()> {
        let body = Bytes::from(std::mem::take(&mut self.body));
        let content_length = if self.has_started() { None } else { Some(body.len()) };
        self.commit(content_length)?;
        if status_forbids_body(self.final_status()) {
            return Ok(());
        }
        let mut offset = 0;
        while offset < body.len() {
            let end = (offset + BODY_CHUNK_SIZE).min(body.len());
            // slice shares the memory of `body`, nothing is copied
            self.send_chunk(body.slice(offset..end)).await?;
            offset = end;
        }
        Ok(())
    }

    /// Writes what is left (head and `body`) and ends the stream, like `CompleteAsync` of ASP.NET.
    pub async fn complete_async(&mut self) -> io::Result<()> {
        let result = self.flush_body_async().await;
        self.body_tx = None;
        self.written_phase = WritenPhase::END;
        result
    }

    /// Ends the stream with an error, the client sees a broken response instead of a complete one. Use `HttpContext::abort`.
    /// Does nothing when the head is not sent yet, set the status and body instead.
    pub(crate) fn abort(&mut self) {
        if !self.has_started() {
            return;
        }
        if let Some(body_tx) = self.body_tx.take() {
            body_tx.abort(io::Error::new(io::ErrorKind::ConnectionAborted, "response aborted"));
        }
        self.written_phase = WritenPhase::END;
    }
}
