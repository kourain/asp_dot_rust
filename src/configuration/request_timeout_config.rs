use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RequestTimeoutConfiguration{
    pub timeout_seconds: u64,
}