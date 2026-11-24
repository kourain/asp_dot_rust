use std::time::Duration;

#[derive(Clone, Debug)]
pub struct WebSocketOptions {
    /// An empty list or "*" allows every origin. Clients may omit Origin.
    pub allowed_origins: Vec<String>,
    /// Heartbeat interval. Zero disables heartbeats.
    pub keep_alive_interval: Duration,
    /// When set, send Ping and require its matching Pong before this timeout.
    /// None or zero sends unsolicited Pong heartbeats instead.
    pub keep_alive_timeout: Option<Duration>,
}

impl Default for WebSocketOptions {
    fn default() -> Self {
        Self {
            allowed_origins: Vec::new(),
            keep_alive_interval: Duration::from_secs(120),
            keep_alive_timeout: None,
        }
    }
}
