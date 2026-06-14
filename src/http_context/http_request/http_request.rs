
use crate::{
    http_context::{RequestStream, http_header::HttpHeader},
};

pub struct HttpRequest {
    pub method: http::Method,
    pub path: String,
    pub version: http::Version,
    pub body: Vec<u8>,
    pub headers: HttpHeader,
    pub keep_alive: bool,
    pub client_addr: std::net::IpAddr,
    request_stream: Option<RequestStream>,
}

impl HttpRequest {
    pub fn from_http(http: http::Request<Vec<u8>>) -> Self {
        let method = http.method().clone();
        let path = http.uri().path().to_string();
        let version = http.version();
        let mut headers = HttpHeader::new();
        for (key, value) in http.headers().iter() {
            if let Ok(value_str) = value.to_str() {
                headers.add(key.as_str(), value_str);
            }
        }
        Self {
            method,
            path,
            version,
            body: Vec::new(),
            headers,
            keep_alive: true,                                                   // default to true, can be updated based on headers
            client_addr: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), // placeholder, should be set from actual connection info
            request_stream: None,                                               // not used in this constructor
        }
    }
    pub fn method(&self) -> &http::Method {
        &self.method
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn uri(&self) -> http::Uri {
        http::Uri::try_from(self.path.as_str()).unwrap_or_else(|_| http::Uri::from_static("/"))
    }
    pub fn version(&self) -> http::Version {
        self.version
    }
    pub fn headers(&self) -> &crate::http_context::http_header::HttpHeader {
        &self.headers
    }
    pub fn headers_mut(&mut self) -> &mut crate::http_context::http_header::HttpHeader {
        &mut self.headers
    }
    pub fn body_mut(&mut self) -> &mut Vec<u8> {
        &mut self.body
    }
}
