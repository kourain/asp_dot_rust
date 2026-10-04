use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct HyperConfig {
    pub max_request_body_size: usize,
    pub max_request_headers_size: usize,
    pub max_response_headers_size: usize,

    /// How long to wait for a client to finish sending request headers on a kept-alive HTTP/1.1 connection before closing it.
    pub header_read_timeout: u64,
    /// How often to send HTTP/2 PING frames on an otherwise idle connection to detect dead peers.
    pub http2_keep_alive_interval: u64,
    /// How long to wait for an HTTP/2 keep-alive PING to be acknowledged before the connection is considered dead and closed.
    pub http2_keep_alive_timeout: u64,
}
impl Default for HyperConfig {
    fn default() -> Self {
        Self {
            max_request_body_size: 100 * 1024 * 1024, // 100 MB
            max_request_headers_size: 8 * 1024,       // 8 KB
            max_response_headers_size: 8 * 1024,      // 8 KB
            header_read_timeout: 10,
            http2_keep_alive_interval: 20,
            http2_keep_alive_timeout: 10,
        }
    }
}
