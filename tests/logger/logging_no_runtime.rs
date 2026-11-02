//! The first `LOGGER` call used to panic with "there is no reactor running" when no Tokio
//! runtime was active. `ApplicationBuilder::new()` logs immediately, so building an
//! application from a plain `fn main()` was impossible.
//!
//! This file must keep a single test: the logger is process-global, so the very first
//! call has to happen here, outside of any runtime.

use asp_dot_rust::logging::{LOGGER, LogCommand};
use std::time::{Duration, Instant};

#[test]
fn first_log_call_works_without_a_tokio_runtime() {
    let mut receiver = LOGGER::spawn_log_receiver();
    LOGGER::info("logging_no_runtime: probe message");

    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match receiver.try_recv() {
            Ok(LogCommand::Log(info)) if info.message == "logging_no_runtime: probe message" => return,
            Ok(_) => {}
            Err(_) if Instant::now() > deadline => panic!("log message was never delivered to the receiver"),
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

#[test]
fn application_builder_can_be_created_without_a_tokio_runtime() {
    let mut builder = asp_dot_rust::ApplicationBuilder::new("NoRuntime");
    builder.with_loopback_ip().with_http_port(18901);
}
