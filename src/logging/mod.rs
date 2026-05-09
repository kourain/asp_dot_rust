mod log_level;
mod logger;
mod loginfo;

pub use log_level::LogLevel;
pub use logger::Logger;
pub use loginfo::{LogCommand, LogInfo};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;

type LogSender = broadcast::Sender<LogCommand>;
pub type LogReceiver = broadcast::Receiver<LogCommand>;

static LOG_SENDER: OnceLock<LogSender> = OnceLock::new();
static LOG_RUNNING: AtomicBool = AtomicBool::new(false);

fn initialize_logging(mut rx: LogReceiver) {
    if LOG_RUNNING.swap(true, Ordering::SeqCst) {
        return; // Already initialized
    }
    // Spawn background logging task
    tokio::spawn(async move {
        let mut logger = Logger::new();
        while let Ok(command) = rx.recv().await {
            match command {
                LogCommand::Log(log_info) => {
                    logger.write_log(&log_info).await;
                }
                LogCommand::SetLogFormat(format) => {
                    logger.set_log_format(format);
                }
                LogCommand::SetTimeFormat(format) => {
                    logger.set_date_time_format(format);
                }
                LogCommand::SetLogLevel(level) => {
                    logger.level = level;
                }
                LogCommand::SetEnable(enable) => {
                    logger.enable = enable;
                }
                LogCommand::SetUseTime(enable) => {
                    logger.use_time = enable;
                }
                LogCommand::SetUseColorOutput(enable) => {
                    logger.use_color_output = enable;
                }
                LogCommand::SetUseRequestId(enable) => {
                    logger.use_request_id = enable;
                }
                LogCommand::SetUseConnectionId(enable) => {
                    logger.use_connection_id = enable;
                }
            }
        }
    });
}

fn get_sender() -> &'static LogSender {
    LOG_SENDER.get_or_init(|| {
        // Fallback if not initialized (shouldn't happen in normal usage)
        let (tx, rx) = broadcast::channel::<LogCommand>(1_000_000);
        initialize_logging(rx);
        tx
    })
}
pub fn spawn_log_receiver() -> LogReceiver {
    get_sender().subscribe()
}
pub struct LOGGER;
impl LOGGER {
    /// Log a message (non-blocking, sends through channel)
    pub fn log(level: LogLevel, message: impl Into<String>) {
        let log_info = LogInfo {
            timestamp: Some(chrono::Utc::now()),
            level,
            message: message.into(),
        };

        _ = get_sender().send(LogCommand::Log(log_info));
    }

    /// set format of log output, e.g. "[{level}] {requestid} {timestamp} {message}"
    pub fn with_format(format: impl Into<String>) {
        _ = get_sender().send(LogCommand::SetLogFormat(format.into()));
    }

    /// set whether to use UTC time for timestamps
    pub fn with_time(true_or_false: bool) {
        _ = get_sender().send(LogCommand::SetUseTime(true_or_false));
    }

    /// set whether to use UTC time for timestamps
    pub fn with_chrono_time_format(format: impl Into<String>) {
        _ = get_sender().send(LogCommand::SetTimeFormat(format.into()));
    }

    /// set whether to use colored output
    pub fn with_color_output(true_or_false: bool) {
        _ = get_sender().send(LogCommand::SetUseColorOutput(true_or_false));
    }

    /// set log with request_id
    pub fn with_request_id(true_or_false: bool) {
        _ = get_sender().send(LogCommand::SetUseRequestId(true_or_false));
    }

    /// set log with connection_id
    pub fn with_connection_id(true_or_false: bool) {
        _ = get_sender().send(LogCommand::SetUseConnectionId(true_or_false));
    }

    /// set the log level
    pub fn with_level(level: LogLevel) {
        _ = get_sender().send(LogCommand::SetLogLevel(level));
    }

    /// set whether to log to console
    pub fn set_enable(true_or_false: bool) {
        _ = get_sender().send(LogCommand::SetEnable(true_or_false));
    }

    pub fn trace(message: impl Into<String>) {
        Self::log(LogLevel::Trace, message);
    }

    pub fn debug(message: impl Into<String>) {
        Self::log(LogLevel::Debug, message);
    }

    pub fn info(message: impl Into<String>) {
        Self::log(LogLevel::Info, message);
    }

    pub fn warn(message: impl Into<String>) {
        Self::log(LogLevel::Warn, message);
    }

    pub fn error(message: impl Into<String>) {
        Self::log(LogLevel::Error, message);
    }

    pub fn verbose(message: impl Into<String>) {
        Self::log(LogLevel::Verbose, message);
    }
}
