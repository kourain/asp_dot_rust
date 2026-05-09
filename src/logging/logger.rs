use crate::logging::{LogInfo, LogLevel};
use tokio::io::AsyncWriteExt;
#[derive(Clone, Debug)]
enum FormatPart {
    Literal(String),
    Level,
    Timestamp,
    Message,
    RequestID,
    ConnectionID,
}
pub struct Logger {
    pub level: LogLevel,
    pub enable: bool,
    log_format: String,
    date_time_format: String,
    format_parts: Vec<FormatPart>,
    pub use_time: bool,
    pub use_color_output: bool,
    pub use_request_id: bool,
    pub use_connection_id: bool,
}
impl Logger {
    pub fn new() -> Self {
        let mut log = Logger {
            level: LogLevel::Info,
            enable: true,
            log_format: "[{level}] {timestamp} {connectionid} {requestid} {message}".to_string(),
            date_time_format: "%Y-%m-%d %H:%M:%S".to_string(),
            format_parts: Vec::new(),
            use_time: false,
            use_color_output: true,
            use_request_id: false,
            use_connection_id: false,
        };
        log.format_parts = Self::parse_format(&log.log_format);
        log
    }
    /// write a log message
    pub async fn write_log(&mut self, log_info: &LogInfo) {
        if self.should_log(log_info.level) {
            self.output_log_aysnc(log_info).await;
        }
    }
    fn should_log(&self, level: LogLevel) -> bool {
        // hide Debug in release build
        #[cfg(not(debug_assertions))]
        if level == LogLevel::Debug {
            return false;
        }
        level >= self.level
    }
    /// get the current timestamp as a string
    #[allow(dead_code)]
    fn get_timestamp(&self) -> String {
        use chrono::prelude::*;
        if self.use_time {
            let now: DateTime<Utc> = Utc::now();
            return now.format(&self.date_time_format).to_string();
        }
        let now: DateTime<Local> = Local::now();
        now.format(&self.date_time_format).to_string()
    }
    pub fn set_log_format(&mut self, format: impl Into<String>) {
        self.log_format = format.into();
        self.format_parts = Self::parse_format(&self.log_format);
    }

    pub fn set_date_time_format(&mut self, format: impl Into<String>) {
        self.date_time_format = format.into();
    }
    fn parse_format(format: &str) -> Vec<FormatPart> {
        let mut parts = Vec::new();
        let mut remaining = format;

        while let Some(pos) = remaining.find('{') {
            if pos > 0 {
                parts.push(FormatPart::Literal(remaining[..pos].to_string()));
            }
            let rest = &remaining[pos..];

            if rest.starts_with("{level}") {
                parts.push(FormatPart::Level);
                remaining = &rest["{level}".len()..];
            } else if rest.starts_with("{timestamp}") {
                parts.push(FormatPart::Timestamp);
                remaining = &rest["{timestamp}".len()..];
            } else if rest.starts_with("{message}") {
                parts.push(FormatPart::Message);
                remaining = &rest["{message}".len()..];
            } else if rest.starts_with("{requestid}") {
                parts.push(FormatPart::RequestID);
                remaining = &rest["{requestid}".len()..];
            } else if rest.starts_with("{connectionid}") {
                parts.push(FormatPart::ConnectionID);
                remaining = &rest["{connectionid}".len()..];
            } else {
                parts.push(FormatPart::Literal("{".to_string()));
                remaining = &rest[1..];
            }
        }

        if !remaining.is_empty() {
            parts.push(FormatPart::Literal(remaining.to_string()));
        }
        parts
    }

    /// output the log message to the configured outputs
    async fn output_log_aysnc(&mut self, log_info: &LogInfo) {
        let level_str = if self.use_color_output {
            log_info.level.as_colored_string()
        } else {
            log_info.level.as_str().to_string()
        };
        let timestamp = log_info.timestamp.unwrap_or(chrono::prelude::Utc::now()).format(&self.date_time_format).to_string();
        let capacity = timestamp.len() + log_info.message.len() + level_str.len() + self.log_format.len() + 1;

        let mut output = String::with_capacity(capacity);

        for part in &self.format_parts {
            match part {
                FormatPart::Literal(s) => output.push_str(s),
                FormatPart::Level => output.push_str(&level_str),
                FormatPart::Timestamp => output.push_str(&timestamp),
                FormatPart::Message => output.push_str(&log_info.message),
                FormatPart::RequestID => output.push_str(&self.get_http_request_id()),
                FormatPart::ConnectionID => output.push_str(&self.get_connection_id()),
            }
        }
        output.push('\n');

        let bytes = output.as_bytes();
        _ = tokio::io::stderr().write_all(bytes).await;
    }
    fn get_http_request_id(&self) -> String {
        if self.use_request_id {
            crate::threading::HTTP_REQUEST_ID.try_with(|id| id.clone()).unwrap_or_else(|_| "".to_string())
        } else {
            "".to_string()
        }
    }
    fn get_connection_id(&self) -> String {
        if self.use_connection_id {
            crate::threading::TCP_CONNECTION_ID.try_with(|id| id.clone()).unwrap_or_else(|_| "".to_string())
        } else {
            "".to_string()
        }
    }
}
