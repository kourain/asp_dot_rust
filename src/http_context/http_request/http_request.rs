use crate::services::routing::ResolvedRoute;
use http::{HeaderMap, Uri};
use hyper::body::Incoming;
use std::sync::Arc;
use ulid::Ulid;

use super::request_body::RequestBody;

pub struct HttpRequest {
    pub connection_id: Arc<Ulid>,
    pub request_id: Ulid,
    pub http_version: http::Version,
    pub keep_alive: bool,
    pub method: http::Method,
    pub uri: Uri,
    pub headers: HeaderMap<http::HeaderValue>,
    /// Request body: live stream or in-memory buffer after `enable_buffering`.
    /// Reading more than `max_body_size` bytes from the stream fails with `LengthLimitError`.
    pub body: RequestBody,
    /// Body size limit applied to this request (route `max_body_size` or the global default).
    pub max_body_size: usize,
    pub client_socket_addr: std::net::SocketAddr,
    pub local_socket_addr: std::net::SocketAddr,
    pub routing_info: ResolvedRoute,
}

impl HttpRequest {
    /// Move the `http::Request<Incoming>` into `HttpRequest`, extracting the necessary fields.
    pub fn from_http(
        http: http::Request<Incoming>,
        routing_info: ResolvedRoute,
        max_body_size: usize,
        client_socket_addr: std::net::SocketAddr,
        local_socket_addr: std::net::SocketAddr,
        connection_id: Arc<Ulid>,
    ) -> Self {
        let (part, body) = http.into_parts();
        Self {
            connection_id,
            request_id: Ulid::generate(),
            method: part.method,
            uri: part.uri,
            http_version: part.version,
            body: RequestBody::from_incoming(body, max_body_size),
            max_body_size,
            headers: part.headers,
            keep_alive: true,
            client_socket_addr,
            local_socket_addr,
            routing_info,
        }
    }

    /// Load the entire body into memory so it can be re-read (ASP.NET `Request.EnableBuffering`).
    ///
    /// After this succeeds, [`seek_body`](Self::seek_body)`(0)` resets the read position.
    pub async fn enable_buffering(&mut self) -> Result<(), String> {
        self.body
            .enable_buffering()
            .await
            .map_err(|e| e.to_string())
    }

    /// Set the body read position (only after buffering). Use `0` to read again from the start.
    pub fn seek_body(&mut self, pos: usize) -> Result<(), String> {
        self.body.seek(pos).map_err(|e| e.to_string())
    }

    /// Buffered body bytes when buffering is active; ignores the current seek position.
    pub fn buffered_body(&self) -> Option<&[u8]> {
        self.body.as_buffered_slice()
    }

    /// The attributes of the route this request resolved to, `None` when it resolved to no route (not found, method not allowed).
    pub fn route_attributes(&self) -> Option<&crate::services::routing::RouteAttribute> {
        match &self.routing_info.router_info {
            crate::services::routing::RoutingResult::Found(route) => Some(&route.route_attr),
            _ => None,
        }
    }

    /// The value of the route attribute `key` when it was stored as exactly the type `T`, see `RouteAttribute::get`.
    pub fn route_attribute<T: Send + Sync + Clone + 'static>(&self, key: &str) -> Option<T> {
        self.route_attributes().and_then(|attributes| attributes.get(key))
    }

    pub fn path(&self) -> &str {
        self.uri.path()
    }

    pub fn query_string(&self) -> &str {
        self.uri.query().unwrap_or("")
    }
}
