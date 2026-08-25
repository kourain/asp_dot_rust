use serde::Deserialize;

#[derive(Default, Debug, Clone, Deserialize)]
pub struct RateLimitConfiguration {
    pub max_requests: usize,
    pub limit_seconds: u32,
    pub block_duration_seconds: u32,
}
