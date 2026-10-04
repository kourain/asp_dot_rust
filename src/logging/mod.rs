mod log_level;
mod logger;
mod loginfo;

pub use log_level::LogLevel;
pub use logger::Logger;
pub use loginfo::{LogCommand, LogInfo};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

type LogSender = broadcast::Sender<LogCommand>;
pub type LogReceiver = broadcast::Receiver<LogCommand>;

static LOG_SENDER: OnceLock<LogSender> = OnceLock::new();
static LOG_RUNNING: AtomicBool = AtomicBool::new(true);
static DEFAULT_LOG_ENABLE: AtomicBool = AtomicBool::new(true);
static DEFAULT_LOG_RUNNING: AtomicBool = AtomicBool::new(false);

pub struct LOGGER;
impl LOGGER {
    /// Log a message (non-blocking, sends through channel)
    pub fn log(level: LogLevel, message: impl Into<String>) {
        let sender = Self::get_sender();
        if LOG_RUNNING.load(Ordering::Relaxed) {
            let log_info = LogInfo {
                timestamp: chrono::Utc::now(),
                level,
                message: message.into(),
            };

            _ = sender.send(LogCommand::Log(log_info));
        }
    }

    /// set format of log output, e.g. "[{level}] {requestid} {timestamp} {message}"
    pub fn with_format(format: impl Into<String>) {
        _ = Self::get_sender().send(LogCommand::SetLogFormat(format.into()));
    }

    /// set whether to use UTC time for timestamps
    pub fn with_time_format(format: impl Into<String>) {
        _ = Self::get_sender().send(LogCommand::SetTimeFormat(format.into()));
    }

    /// set whether to use colored output
    pub fn with_color_output(true_or_false: bool) {
        _ = Self::get_sender().send(LogCommand::SetUseColorOutput(true_or_false));
    }

    /// set the log level
    pub fn with_level(level: LogLevel) {
        _ = Self::get_sender().send(LogCommand::SetLogLevel(level));
    }

    /// Enable logger
    pub fn enable() {
        LOG_RUNNING.store(true, Ordering::Release);
        _ = Self::get_sender().send(LogCommand::SetEnable(true));
        Self::start_default_logger(None);
    }

    /// Disable logger
    pub fn disable() {
        LOG_RUNNING.store(false, Ordering::Release);
        _ = Self::get_sender().send(LogCommand::SetEnable(false));
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

    /// get the log sender, initializing it if necessary
    fn get_sender() -> &'static LogSender {
        LOG_SENDER.get_or_init(|| {
            // Fallback if not initialized (shouldn't happen in normal usage)
            let (tx, rx) = broadcast::channel::<LogCommand>(1_000_000);
            Self::start_default_logger(Some(rx));
            tx
        })
    }

    /// spawn a log receiver for a specific task
    pub fn spawn_log_receiver() -> LogReceiver {
        Self::get_sender().subscribe()
    }

    /// start the default logger in a background task
    pub fn enable_default_logger() {
        Self::use_default_logger(true);
    }

    /// disable the default logger
    pub fn disable_default_logger() {
        Self::use_default_logger(false);
    }

    fn use_default_logger(true_or_false: bool) {
        if true_or_false {
            DEFAULT_LOG_ENABLE.store(true, Ordering::Release);
            Self::start_default_logger(None)
        } else {
            DEFAULT_LOG_ENABLE.store(false, Ordering::Release);
        }
    }
    // default console logger, runs in background task
    fn start_default_logger(mut _rx: Option<LogReceiver>) {
        if !DEFAULT_LOG_ENABLE.load(Ordering::Relaxed) || DEFAULT_LOG_RUNNING.swap(true, Ordering::SeqCst) {
            return; // Already initialized
        }
        let mut rx;
        if let Some(_rx) = _rx {
            rx = _rx;
        } else {
            rx = Self::spawn_log_receiver();
        }
        // Spawn background logging task
        tokio::spawn(async move {
            let mut logger = Logger::new();
            loop {
                let received = rx.recv().await;
                if DEFAULT_LOG_ENABLE.load(Ordering::Relaxed) {
                    match received {
                        Ok(command) => match command {
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
                                logger.set_level(level);
                            }
                            LogCommand::SetEnable(_) => {
                                // Do nothing, handled by outer loop
                            }
                            LogCommand::SetDefaultLogger(enable) => {
                                DEFAULT_LOG_ENABLE.store(enable, Ordering::Release);
                            }
                            LogCommand::SetUseColorOutput(enable) => {
                                logger.set_use_color_output(enable);
                            }
                        },
                        Err(RecvError::Lagged(missed)) => {
                            let log = LogInfo {
                                timestamp: chrono::Utc::now(),
                                level: LogLevel::Error,
                                message: format!("LOGGER Missed {} log messages due to lag", missed),
                            };
                            logger.write_log(&log).await;
                        }
                        Err(RecvError::Closed) => {}
                    }
                } else {
                    break;
                }
            }
            DEFAULT_LOG_RUNNING.store(false, Ordering::Release);
        });
    }
}
