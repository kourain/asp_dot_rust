use crate::{
    extensions::Str,
    http_context::{ResponseStream, http_header::HttpHeader},
    io::AsyncWrite,
    logging::LOGGER,
};

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
    pub headers: HttpHeader,
    pub body: Vec<u8>,
    pub version: http::Version,
    pub written_phase: WritenPhase,
    pub keep_alive: bool,
    output_stream: Option<ResponseStream>,
}

impl HttpResponse {
    pub(crate) async fn new(output_stream: ResponseStream) -> std::io::Result<Self> {
        Ok(Self {
            status_code: http::StatusCode::OK,
            headers: HttpHeader::new(),
            body: Vec::new(),
            version: http::Version::HTTP_11,
            written_phase: WritenPhase::NONE,
            keep_alive: false,
            output_stream: Some(output_stream),
        })
    }
    pub async fn write_headers_async(&mut self) {
        if self.written_phase >= WritenPhase::HTTP_HEADERS {
            return;
        }
        if self.headers.content_length().is_none() {
            self.headers.add("Content-Length", self.body.len().to_string().as_str());
        }
        if self.headers.get("Connection").is_none() {
            let connection = if self.keep_alive { "keep-alive" } else { "close" };
            self.headers.add("Connection", connection);
            self.headers.add("Keep-Alive", "timeout=30, max=1000");
        }
        let response_string = format!(
            "{} {} {}\r\n{}\r\n\r\n",
            self.version.as_str(),
            self.status_code,
            self.status_code.canonical_reason().unwrap_or(""),
            self.headers.to_string()
        );
        self.output_stream.as_mut().unwrap().write_async(response_string.as_bytes()).await.unwrap_or_else(|e| {
            LOGGER::error(format!("Failed to write headers to TCP stream: {}", e));
        });
        self.written_phase = WritenPhase::HTTP_HEADERS;
    }
    pub async fn write_async<'a>(&mut self, data: impl Into<&'a [u8]>) -> &mut Self {
        let data = data.into();
        if self.headers.content_length().unwrap_or(0) < data.len() as u64 {
            self.headers.set_content_length(data.len());
        }
        self.write_headers_async().await;
        self.output_stream.as_mut().unwrap().write_async(data).await.unwrap_or_else(|e| {
            LOGGER::error(format!("Failed to write data to TCP stream: {}", e));
        });
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
    pub async fn get_total_written_size(&self) -> usize {
        self.output_stream.as_ref().unwrap().written_size
    }
    pub fn to_http_response(&self) -> http::Response<Vec<u8>> {
        let mut response_builder = http::Response::builder().status(self.status_code);
        for (key, value) in self.headers.clone_hashmap() {
            response_builder = response_builder.header(key, value);
        }
        response_builder.body(self.body.clone()).unwrap_or_else(|e| {
            LOGGER::error(format!("Failed to build http::Response: {}", e));
            http::Response::new(Vec::new())
        })
    }
    pub fn move_to_http_response(self) -> http::Response<Vec<u8>> {
        let mut response_builder = http::Response::builder().status(self.status_code);
        for (key, value) in self.headers.unpack_hashmap() {
            response_builder = response_builder.header(key, value);
        }
        response_builder.body(self.body).unwrap_or_else(|e| {
            LOGGER::error(format!("Failed to build http::Response: {}", e));
            http::Response::new(Vec::new())
        })
    }
}
