use crate::services::routing::ResolvedRoute;
use http::{HeaderMap, Uri};
use http_body_util::Limited;
use hyper::body::Incoming;
use std::sync::Arc;
use ulid::Ulid;

pub struct HttpRequest {
    pub connection_id: Arc<Ulid>,
    pub request_id: Ulid,
    pub http_version: http::Version,
    pub keep_alive: bool,
    pub method: http::Method,
    pub uri: Uri,
    pub headers: HeaderMap<http::HeaderValue>,
    /// The request body, reading more than `max_body_size` bytes fails with `http_body_util::LengthLimitError`.
    pub body: Limited<Incoming>,
    pub client_socket_addr: std::net::SocketAddr,
    pub local_socket_addr: std::net::SocketAddr,
    pub routing_info: ResolvedRoute,
}

impl HttpRequest {
    /// move the http::Request<Incoming> into HttpRequest, extracting the necessary fields
    pub fn from_http(http: http::Request<Incoming>, routing_info: ResolvedRoute, max_body_size: usize, client_socket_addr: std::net::SocketAddr, local_socket_addr: std::net::SocketAddr, connection_id: Arc<Ulid>) -> Self {
        let (part, body) = http.into_parts();
        let method = part.method;
        let uri = part.uri;
        let http_version = part.version;
        let headers = part.headers;
        Self {
            connection_id,
            request_id: Ulid::generate(),
            method,
            uri,
            http_version,
            body: Limited::new(body, max_body_size),
            headers,
            keep_alive: true, // default to true, can be updated based on headers
            client_socket_addr,
            local_socket_addr,
            routing_info,
        }
    }
    pub fn path(&self) -> &str {
        self.uri.path()
    }
    pub fn query_string(&self) -> &str {
        self.uri.query().unwrap_or("")
    }
}
