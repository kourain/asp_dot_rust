#[derive(Default, Debug, Clone)]
pub struct RateLimitConfiguration {
    pub max_requests: usize,
    pub limit_seconds: u32,
    pub block_duration_seconds: u32,
}
