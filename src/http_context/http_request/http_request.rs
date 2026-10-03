use crate::services::routing::ResolvedRoute;
use http::{HeaderMap, Uri};
use hyper::body::Incoming;

pub struct HttpRequest {
    pub http_version: http::Version,
    pub keep_alive: bool,
    pub method: http::Method,
    pub uri: Uri,
    pub headers: HeaderMap<http::HeaderValue>,
    pub body: Incoming,
    pub client_socket_addr: std::net::SocketAddr,
    pub local_socket_addr: std::net::SocketAddr,
    pub routing_info: ResolvedRoute,
}

impl HttpRequest {
    /// move the http::Request<Incoming> into HttpRequest, extracting the necessary fields
    pub fn from_http(http: http::Request<Incoming>, routing_info: ResolvedRoute, client_socket_addr: std::net::SocketAddr, local_socket_addr: std::net::SocketAddr) -> Self {
        let (part, body) = http.into_parts();
        let method = part.method;
        let uri = part.uri;
        let http_version = part.version;
        let headers = part.headers;
        Self {
            method,
            uri,
            http_version,
            body,
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
