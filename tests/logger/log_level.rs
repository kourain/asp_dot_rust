use asp_dot_rust::logging::{LogLevel, Logger};


fn verbose_logger() -> Logger {
    let mut logger = Logger::new();
    logger.set_level(LogLevel::Verbose);
    logger
}

#[test]
fn levels_at_or_above_threshold_are_logged() {
    let mut logger = Logger::new();
    logger.set_level(LogLevel::Warn);
    assert!(!logger.should_log(&LogLevel::Info));
    assert!(logger.should_log(&LogLevel::Warn));
    assert!(logger.should_log(&LogLevel::Error));
}

#[test]
#[cfg(not(debug_assertions))]
fn debug_level_is_hidden_in_release_builds() {
    let logger = verbose_logger();
    assert!(!logger.should_log(&LogLevel::Debug));
    assert!(logger.should_log(&LogLevel::Trace));
    assert!(logger.should_log(&LogLevel::Info));
}

#[test]
#[cfg(debug_assertions)]
fn debug_level_is_visible_in_debug_builds() {
    let logger = verbose_logger();
    assert!(logger.should_log(&LogLevel::Debug));
}
