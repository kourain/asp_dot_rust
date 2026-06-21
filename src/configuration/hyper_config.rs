pub struct HyperConfig {
    pub max_request_body_size: usize,
    pub max_request_headers_size: usize,
    pub max_response_headers_size: usize,
}
impl Default for HyperConfig {
    fn default() -> Self {
        Self {
            max_request_body_size: 100 * 1024 * 1024, // 100 MB
            max_request_headers_size: 8 * 1024,       // 8 KB
            max_response_headers_size: 8 * 1024,      // 8 KB
        }
    }
}
