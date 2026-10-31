//! Integration tests of the response stream: a complete response (controller result) and a streamed response (`write_async`).
//! Every test starts a real server on a free loopback port and talks to it with raw TCP (HTTP/1.1) or the hyper client (HTTP/2).

use std::{
    io,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use asp_dot_rust::{
    Application, ApplicationBuilder, api_controller,
    controller::{ActionResult, get, route},
    http_context::HttpContextRef,
    logging::LOGGER,
    macros::{controller_inject, controller_route},
};
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

// ---------------------------------------------------------------------------
// Controller under test
// ---------------------------------------------------------------------------

static DISCONNECT_SEEN: AtomicBool = AtomicBool::new(false);
static PRODUCED_CHUNKS: AtomicUsize = AtomicUsize::new(0);

const TEXT_BODY: &str = "<html><body>Hello world</body></html>";
const LARGE_BODY_SIZE: usize = 1_000_000;
const BACKPRESSURE_CHUNKS: usize = 400;
const BACKPRESSURE_CHUNK_SIZE: usize = 64 * 1024;

fn large_body() -> Vec<u8> {
    (0..LARGE_BODY_SIZE).map(|i| (i % 251) as u8).collect()
}

/// A result without body, status 204.
struct NoContent;
impl ActionResult for NoContent {
    async fn get_body_async(&self) -> Vec<u8> {
        Vec::new()
    }
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::NO_CONTENT
    }
    fn content_length(&self) -> usize {
        0
    }
}

api_controller!(pub StreamController {});

#[controller_route]
#[controller_inject]
impl StreamController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    // ----- complete responses -----

    #[get("/complete/text")]
    pub async fn complete_text(&mut self) -> impl ActionResult {
        self.http_context.response.headers.insert_str("Content-Type", "text/html");
        TEXT_BODY
    }

    #[get("/complete/string")]
    pub async fn complete_string(&mut self) -> impl ActionResult {
        "{\"ok\":true}".to_string()
    }

    #[get("/complete/large")]
    pub async fn complete_large(&mut self) -> impl ActionResult {
        large_body()
    }

    #[get("/complete/compressible")]
    pub async fn complete_compressible(&mut self) -> impl ActionResult {
        "a".repeat(10_000)
    }

    #[get("/complete/empty")]
    pub async fn complete_empty(&mut self) -> impl ActionResult {
        ""
    }

    #[get("/complete/no_content")]
    pub async fn complete_no_content(&mut self) -> impl ActionResult {
        NoContent
    }

    #[route(["get", "head"], "/complete/head")]
    pub async fn complete_head(&mut self) -> impl ActionResult {
        TEXT_BODY
    }

    // ----- streamed responses -----

    #[get("/stream/chunks")]
    pub async fn stream_chunks(&mut self) -> impl ActionResult {
        self.http_context.response.headers.insert_str("Content-Type", "text/plain");
        for part in ["one\n", "two\n", "three\n"] {
            self.http_context.response.write_async(part).await.unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        ""
    }

    #[get("/stream/status")]
    pub async fn stream_status(&mut self) -> impl ActionResult {
        self.http_context.response.status_code = http::StatusCode::CREATED;
        self.http_context.response.headers.insert_str("X-Test", "yes");
        self.http_context.response.write_bytes_async("created".to_string()).await.unwrap();
        ""
    }

    #[get("/stream/mixed")]
    pub async fn stream_mixed(&mut self) -> impl ActionResult {
        self.http_context.response.write_async(b"prefix-").await.unwrap();
        "tail"
    }

    #[get("/stream/mixed_large")]
    pub async fn stream_mixed_large(&mut self) -> impl ActionResult {
        self.http_context.response.write_async(b"prefix-").await.unwrap();
        "a".repeat(10_000)
    }

    #[get("/stream/headers_only")]
    pub async fn stream_headers_only(&mut self) -> impl ActionResult {
        self.http_context.response.headers.insert_str("X-Early", "1");
        self.http_context.response.write_headers_async().await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        self.http_context.response.write_async("late").await.unwrap();
        ""
    }

    #[get("/stream/disconnect")]
    pub async fn stream_disconnect(&mut self) -> impl ActionResult {
        loop {
            if self.http_context.response.write_async([b'x'; 1024]).await.is_err() {
                DISCONNECT_SEEN.store(true, Ordering::SeqCst);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        ""
    }

    #[get("/stream/backpressure")]
    pub async fn stream_backpressure(&mut self) -> impl ActionResult {
        let chunk = vec![7u8; BACKPRESSURE_CHUNK_SIZE];
        for _ in 0..BACKPRESSURE_CHUNKS {
            if self.http_context.response.write_bytes_async(chunk.clone()).await.is_err() {
                break;
            }
            PRODUCED_CHUNKS.fetch_add(1, Ordering::SeqCst);
        }
        ""
    }

    #[get("/stream/panic_before")]
    pub async fn stream_panic_before(&mut self) -> impl ActionResult {
        if std::hint::black_box(true) {
            panic!("test panic before any write");
        }
        ""
    }

    #[get("/stream/panic_after")]
    pub async fn stream_panic_after(&mut self) -> impl ActionResult {
        self.http_context.response.write_async("partial").await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        if std::hint::black_box(true) {
            panic!("test panic after the first write");
        }
        ""
    }
}

// ---------------------------------------------------------------------------
// Server and client helpers
// ---------------------------------------------------------------------------

/// Starts the application on a free loopback port, `configure` adds the middlewares.
async fn start_server(configure: impl FnOnce(&mut Application)) -> u16 {
    LOGGER::disable();
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut builder = ApplicationBuilder::new("StreamResponseTest");
    builder.with_loopback_ip().with_http_port(port);
    builder.add_controllers();
    let mut app = builder.build();
    configure(&mut app);
    tokio::spawn(app.run());
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return port;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server did not start on port {port}");
}

struct Head {
    status: u16,
    headers: Vec<(String, String)>,
}
impl Head {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value.as_str())
    }
    fn is_chunked(&self) -> bool {
        self.header("transfer-encoding").is_some_and(|value| value.eq_ignore_ascii_case("chunked"))
    }
}

/// A raw HTTP/1.1 connection.
struct Conn {
    reader: BufReader<TcpStream>,
}
impl Conn {
    async fn connect(port: u16) -> Self {
        Self {
            reader: BufReader::new(TcpStream::connect(("127.0.0.1", port)).await.unwrap()),
        }
    }

    async fn send(&mut self, method: &str, path: &str, extra_headers: &[(&str, &str)]) {
        let mut request = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n");
        for (key, value) in extra_headers {
            request.push_str(&format!("{key}: {value}\r\n"));
        }
        request.push_str("\r\n");
        self.reader.get_mut().write_all(request.as_bytes()).await.unwrap();
    }

    async fn read_line(&mut self) -> io::Result<String> {
        let mut line = String::new();
        if self.reader.read_line(&mut line).await? == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        Ok(line.trim_end_matches(['\r', '\n']).to_string())
    }

    async fn read_head(&mut self) -> Head {
        let status_line = self.read_line().await.unwrap();
        let status = status_line.split(' ').nth(1).unwrap().parse().unwrap();
        let mut headers = Vec::new();
        loop {
            let line = self.read_line().await.unwrap();
            if line.is_empty() {
                break;
            }
            let (key, value) = line.split_once(':').unwrap();
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
        Head { status, headers }
    }

    /// Reads one chunk of a chunked body, `Ok(None)` is the proper end of the body, `Err` is a broken stream.
    async fn read_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        let size_line = self.read_line().await?;
        let size = usize::from_str_radix(size_line.split(';').next().unwrap().trim(), 16).map_err(io::Error::other)?;
        if size == 0 {
            // trailers, then the empty line
            while !self.read_line().await?.is_empty() {}
            return Ok(None);
        }
        let mut chunk = vec![0u8; size];
        self.reader.read_exact(&mut chunk).await?;
        if !self.read_line().await?.is_empty() {
            return Err(io::Error::other("missing CRLF after the chunk"));
        }
        Ok(Some(chunk))
    }

    /// Reads the whole body, by chunks or by Content-Length.
    async fn read_body(&mut self, head: &Head) -> io::Result<Vec<u8>> {
        let mut body = Vec::new();
        if head.is_chunked() {
            while let Some(chunk) = self.read_chunk().await? {
                body.extend_from_slice(&chunk);
            }
        } else if let Some(length) = head.header("content-length") {
            body.resize(length.parse().unwrap(), 0);
            self.reader.read_exact(&mut body).await?;
        }
        Ok(body)
    }

    /// Request and the whole response.
    async fn get(&mut self, path: &str, extra_headers: &[(&str, &str)]) -> (Head, Vec<u8>) {
        self.send("GET", path, extra_headers).await;
        let head = self.read_head().await;
        let body = self.read_body(&head).await.unwrap();
        (head, body)
    }
}

async fn gunzip(data: &[u8]) -> Vec<u8> {
    let mut decoder = async_compression::tokio::bufread::GzipDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).await.unwrap();
    out
}

// ---------------------------------------------------------------------------
// Complete responses
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn complete_text_response_is_written_with_content_length() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/complete/text", &[]).await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-type"), Some("text/html"));
    assert_eq!(head.header("content-length"), Some(TEXT_BODY.len().to_string().as_str()));
    assert_eq!(head.header("server"), Some("ASP.RS"));
    assert!(head.header("connection").is_none(), "hyper manages the connection header");
    assert!(!head.is_chunked());
    assert_eq!(body, TEXT_BODY.as_bytes());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn complete_large_body_is_written_in_full() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/complete/large", &[]).await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-length"), Some(LARGE_BODY_SIZE.to_string().as_str()));
    assert_eq!(body.len(), LARGE_BODY_SIZE);
    assert!(body == large_body(), "the body content is different");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn complete_responses_keep_the_connection_alive() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    let (head, body) = conn.get("/complete/text", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, TEXT_BODY.as_bytes()));
    let (head, body) = conn.get("/complete/string", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, "{\"ok\":true}".as_bytes()));
    let (head, body) = conn.get("/complete/empty", &[]).await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-length"), Some("0"));
    assert!(body.is_empty());
    let (head, body) = conn.get("/complete/large", &[]).await;
    assert_eq!((head.status, body.len()), (200, LARGE_BODY_SIZE));
    let (head, _) = conn.get("/not_found_route", &[]).await;
    assert_eq!(head.status, 404);
    let (head, body) = conn.get("/complete/text", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, TEXT_BODY.as_bytes()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_content_response_has_no_body_and_keeps_the_connection_alive() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    let (head, body) = conn.get("/complete/no_content", &[]).await;
    assert_eq!(head.status, 204);
    assert!(head.header("content-length").is_none(), "204 must not have Content-Length");
    assert!(!head.is_chunked());
    assert!(body.is_empty());
    let (head, body) = conn.get("/complete/text", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, TEXT_BODY.as_bytes()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn head_request_gets_the_headers_without_body() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("HEAD", "/complete/head", &[]).await;
    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-length"), Some(TEXT_BODY.len().to_string().as_str()));
    // no body bytes may follow, the next response must start right after the head
    let (head, body) = conn.get("/complete/text", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, TEXT_BODY.as_bytes()));
}

// ---------------------------------------------------------------------------
// Streamed responses
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn streamed_response_sends_chunks_when_they_are_written() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("GET", "/stream/chunks", &[]).await;
    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-type"), Some("text/plain"));
    assert!(head.is_chunked(), "a streamed response has no Content-Length");
    assert!(head.header("content-length").is_none());

    // the handler sleeps 300ms after each write, a buffered response would deliver all chunks at the same time
    let first = conn.read_chunk().await.unwrap().unwrap();
    let first_at = Instant::now();
    let second = conn.read_chunk().await.unwrap().unwrap();
    let second_at = Instant::now();
    let third = conn.read_chunk().await.unwrap().unwrap();
    assert!(conn.read_chunk().await.unwrap().is_none(), "the stream must end properly");
    assert_eq!((first.as_slice(), second.as_slice(), third.as_slice()), (b"one\n".as_slice(), b"two\n".as_slice(), b"three\n".as_slice()));
    assert!(second_at - first_at >= Duration::from_millis(200), "the first chunk was held until the second one: {:?}", second_at - first_at);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn streamed_response_keeps_the_status_and_headers_set_before_the_first_write() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/stream/status", &[]).await;
    // the controller result (status 200) is returned after the head is sent, it must not change what the client got
    assert_eq!(head.status, 201);
    assert_eq!(head.header("x-test"), Some("yes"));
    assert_eq!(body, b"created");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn streamed_bytes_are_followed_by_the_returned_body() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/stream/mixed", &[]).await;
    assert_eq!(head.status, 200);
    assert_eq!(body, b"prefix-tail");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn headers_can_be_sent_before_the_body() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    let started = Instant::now();
    conn.send("GET", "/stream/headers_only", &[]).await;
    let head = conn.read_head().await;
    assert!(started.elapsed() < Duration::from_millis(250), "the head was held until the body: {:?}", started.elapsed());
    assert_eq!(head.header("x-early"), Some("1"));
    assert_eq!(conn.read_body(&head).await.unwrap(), b"late");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn writer_gets_an_error_when_the_client_disconnects() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("GET", "/stream/disconnect", &[]).await;
    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    assert!(conn.read_chunk().await.unwrap().is_some());
    drop(conn);
    for _ in 0..100 {
        if DISCONNECT_SEEN.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("write_async never returned an error after the client disconnected");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn slow_client_makes_the_writer_wait() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("GET", "/stream/backpressure", &[]).await;
    // do not read: the socket buffers and the channel fill up, then write_bytes_async must wait
    tokio::time::sleep(Duration::from_millis(800)).await;
    let produced = PRODUCED_CHUNKS.load(Ordering::SeqCst);
    assert!(produced > 0, "the handler did not start");
    assert!(produced < BACKPRESSURE_CHUNKS, "the writer was not slowed down by the client: {produced} chunks written");

    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    let mut total = 0;
    while let Some(chunk) = conn.read_chunk().await.unwrap() {
        total += chunk.len();
    }
    assert_eq!(total, BACKPRESSURE_CHUNKS * BACKPRESSURE_CHUNK_SIZE);
    assert_eq!(PRODUCED_CHUNKS.load(Ordering::SeqCst), BACKPRESSURE_CHUNKS);
}

// ---------------------------------------------------------------------------
// Panics
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn panic_before_any_write_returns_500() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    let (head, body) = conn.get("/stream/panic_before", &[]).await;
    assert_eq!(head.status, 500);
    assert_eq!(body, b"Internal Server Error");
    // the connection is still usable
    let (head, body) = conn.get("/complete/text", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, TEXT_BODY.as_bytes()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn panic_after_the_first_write_breaks_the_stream() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("GET", "/stream/panic_after", &[]).await;
    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    assert_eq!(conn.read_chunk().await.unwrap().unwrap(), b"partial");
    // a clean end (`Ok(None)`) would make the client believe the body is complete
    assert!(conn.read_chunk().await.is_err(), "the stream ended as if it was complete");
}

// ---------------------------------------------------------------------------
// Compression middleware
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn compression_still_compresses_a_complete_response() {
    let port = start_server(|app| {
        app.use_compression();
    })
    .await;
    let (head, body) = Conn::connect(port).await.get("/complete/compressible", &[("Accept-Encoding", "gzip")]).await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-encoding"), Some("gzip"));
    assert_eq!(head.header("content-length"), Some(body.len().to_string().as_str()));
    assert!(body.len() < 10_000);
    assert_eq!(gunzip(&body).await, "a".repeat(10_000).as_bytes());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn compression_does_not_touch_a_response_that_is_already_streamed() {
    let port = start_server(|app| {
        app.use_compression();
    })
    .await;
    let (head, body) = Conn::connect(port).await.get("/stream/mixed_large", &[("Accept-Encoding", "gzip")]).await;
    assert_eq!(head.status, 200);
    assert!(head.header("content-encoding").is_none());
    assert_eq!(body, format!("prefix-{}", "a".repeat(10_000)).as_bytes());
}

// ---------------------------------------------------------------------------
// HTTP/2
// ---------------------------------------------------------------------------

async fn h2_get(port: u16, path: &str) -> hyper::Response<hyper::body::Incoming> {
    let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let (mut sender, connection) = hyper::client::conn::http2::handshake(hyper_util::rt::TokioExecutor::new(), hyper_util::rt::TokioIo::new(stream)).await.unwrap();
    tokio::spawn(connection);
    let request = hyper::Request::get(format!("http://127.0.0.1:{port}{path}")).body(Empty::<Bytes>::new()).unwrap();
    sender.send_request(request).await.unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http2_streamed_response_sends_chunks_when_they_are_written() {
    let port = start_server(|_| {}).await;
    let response = h2_get(port, "/stream/chunks").await;
    assert_eq!(response.status(), 200);
    let mut body = response.into_body();
    let first = body.frame().await.unwrap().unwrap().into_data().unwrap();
    let first_at = Instant::now();
    let second = body.frame().await.unwrap().unwrap().into_data().unwrap();
    let second_at = Instant::now();
    let third = body.frame().await.unwrap().unwrap().into_data().unwrap();
    // the end of the stream may come as an empty data frame, anything else is a failure
    while let Some(frame) = body.frame().await {
        let frame = frame.expect("the stream must end properly");
        assert!(frame.into_data().is_ok_and(|data| data.is_empty()), "unexpected frame after the last chunk");
    }
    assert_eq!((&first[..], &second[..], &third[..]), (&b"one\n"[..], &b"two\n"[..], &b"three\n"[..]));
    assert!(second_at - first_at >= Duration::from_millis(200), "the first chunk was held until the second one: {:?}", second_at - first_at);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http2_complete_response_is_written_in_full() {
    let port = start_server(|_| {}).await;
    let response = h2_get(port, "/complete/large").await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-length").unwrap(), LARGE_BODY_SIZE.to_string().as_str());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body[..] == large_body()[..], "the body content is different");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http2_panic_after_the_first_write_breaks_the_stream() {
    let port = start_server(|_| {}).await;
    let response = h2_get(port, "/stream/panic_after").await;
    assert_eq!(response.status(), 200);
    // a clean end would make the client believe the body is complete
    assert!(response.into_body().collect().await.is_err(), "the stream ended as if it was complete");
}
