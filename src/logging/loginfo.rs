use crate::logging::LogLevel;

#[derive(Clone, Debug)]
pub struct LogInfo {
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum LogCommand {
    Log(LogInfo),
    SetLogFormat(String),
    SetLogLevel(LogLevel),
    SetTimeFormat(String),
    SetEnable(bool),
    SetUseTime(bool),
    SetUseColorOutput(bool),
    SetUseRequestId(bool),
    SetUseConnectionId(bool),
}