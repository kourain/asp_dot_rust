use http::{HeaderMap, HeaderValue};

use crate::{http_context::http_header::AspDotRustHttpHeader, logging::LOGGER};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum WritenPhase {
    NONE = 0,
    HTTP_HEADERS = 1,
    HTTP_BODY = 2,
    END = 255,
}
pub struct HttpResponse {
    pub status_code: http::StatusCode,
    pub headers: HeaderMap<HeaderValue>,
    pub body: Vec<u8>,
    pub version: http::Version,
    pub written_phase: WritenPhase,
    pub keep_alive: bool,
    in_memory_output: Option<Vec<u8>>,
}

impl HttpResponse {
    pub(crate) fn new_in_memory() -> Self {
        Self {
            status_code: http::StatusCode::OK,
            headers: HeaderMap::new(),
            body: Vec::new(),
            version: http::Version::HTTP_11,
            written_phase: WritenPhase::NONE,
            keep_alive: false,
            in_memory_output: Some(Vec::new()),
        }
    }
    pub async fn write_headers_async(&mut self) {
        if self.written_phase >= WritenPhase::HTTP_HEADERS {
            return;
        }
        if self.headers.content_length().is_none() {
            self.headers.set_content_length(self.body.len());
        }
        if self.headers.get("Connection").is_none() {
            let connection = if self.keep_alive { "keep-alive" } else { "close" };
            self.headers.insert("Connection", HeaderValue::from_str(connection).unwrap());
            self.headers.insert("Keep-Alive", HeaderValue::from_str("timeout=30, max=1000").unwrap());
        }
        self.written_phase = WritenPhase::HTTP_HEADERS;
    }
    pub async fn write_async<'a>(&mut self, data: impl Into<&'a [u8]>) -> &mut Self {
        let data = data.into();
        if self.headers.content_length().unwrap_or(0) < data.len() as u64 {
            self.headers.set_content_length(data.len());
        }
        self.write_headers_async().await;
        if let Some(buf) = self.in_memory_output.as_mut() {
            buf.extend_from_slice(data);
        }
        self
    }
    pub async fn write_body_async(&mut self) {
        let body = std::mem::replace(&mut self.body, Vec::new());
        self.write_async(body.as_slice()).await;
        self.written_phase = WritenPhase::HTTP_BODY;
    }
    pub async fn write_response_async(&mut self) {
        self.write_headers_async().await;
        self.write_body_async().await;
        self.written_phase = WritenPhase::END;
    }
    pub fn move_to_http_response(self) -> http::Response<Vec<u8>> {
        let mut response_builder = http::Response::builder().status(self.status_code);
        for (key, value) in self.headers() {
            response_builder = response_builder.header(key, value);
        }
        response_builder.body(self.body).unwrap_or_else(|e| {
            LOGGER::error(format!("Failed to build http::Response: {}", e));
            http::Response::new(Vec::new())
        })
    }
    pub fn body_mut(&mut self) -> &mut Vec<u8> {
        &mut self.body
    }
    pub fn headers_mut(&mut self) -> &mut HeaderMap<http::HeaderValue> {
        &mut self.headers
    }
    pub fn headers(&self) -> &HeaderMap<http::HeaderValue> {
        &self.headers
    }
    pub fn status_mut(&mut self) -> &mut http::StatusCode {
        &mut self.status_code
    }
}
