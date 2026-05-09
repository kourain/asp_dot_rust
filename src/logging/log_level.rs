use colored::Colorize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Verbose = 0,
    Trace   = 1,
    Debug   = 2,
    Info    = 3,
    Warn    = 4,
    Error   = 5,
    None    = 6,
}
impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Verbose => "VERBOSE",
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::None => "NONE",
        }
    }
    pub fn as_colored_string(&self) -> String {
        let str = &self.as_str()[..3];
        match self {
            LogLevel::Verbose => str.purple().to_string(),
            LogLevel::Trace => str.blue().to_string(),
            LogLevel::Debug => str.cyan().to_string(),
            LogLevel::Info => str.green().to_string(),
            LogLevel::Warn => str.yellow().to_string(),
            LogLevel::Error => str.red().to_string(),
            LogLevel::None => str.bold().to_string(),
        }
    }
}