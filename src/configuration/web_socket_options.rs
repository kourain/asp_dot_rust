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
    /// Maximum time `close_async` allows for the closing handshake before dropping the connection.
    pub close_timeout: Duration,
    /// Largest accepted message in bytes. None means no limit.
    pub max_message_size: Option<usize>,
    /// Largest accepted frame payload in bytes. None means no limit.
    pub max_frame_size: Option<usize>,
}

impl Default for WebSocketOptions {
    fn default() -> Self {
        let mut rs = Self {
            allowed_origins: Vec::new(),
            keep_alive_interval: Duration::from_secs(120),
            keep_alive_timeout: Some(Duration::from_secs(1)),
            close_timeout: Duration::from_secs(1),
            max_message_size: Some(64 << 20),
            max_frame_size: Some(16 << 20),
        };
        rs.allowed_origins.push("*".to_string());
        rs
    }
}
