use asp_dot_rust::{ApplicationBuilder, logging::{LOGGER, LogCommand}};
use std::time::Duration;

/// Every start-up used to log a bogus warning: the built-in `StartupAppConfiguration`
/// was inserted twice and the second insert warned about replacing the first.
#[tokio::test]
async fn building_an_application_does_not_log_a_replace_configuration_warning() {
    let mut receiver = LOGGER::spawn_log_receiver();
    let _builder = ApplicationBuilder::new("NoSpuriousWarning");
    tokio::time::sleep(Duration::from_millis(50)).await;

    while let Ok(command) = receiver.try_recv() {
        if let LogCommand::Log(info) = command {
            assert!(!info.message.contains("Replace configuration of type"), "unexpected warning: {}", info.message);
        }
    }
}

/// `with_https_port` loads the certificate and key eagerly, so a missing file
/// must fail at startup instead of on the first HTTPS request.
#[test]
#[should_panic(expected = "Failed to load TLS certificate/key for HTTPS")]
fn registering_an_https_port_with_missing_tls_files_panics_at_startup() {
    LOGGER::disable();
    let mut builder = ApplicationBuilder::new("HttpsMissingFiles");
    builder.with_loopback_ip().with_http_port(18921).with_https_port(18922, "missing_cert.pem", "missing_key.pem");
}

/// Registering the same port for HTTP and HTTPS is rejected.
#[test]
#[should_panic(expected = "is already in use by HTTP")]
fn registering_the_same_port_for_http_and_https_panics() {
    LOGGER::disable();
    let mut builder = ApplicationBuilder::new("HttpsPortClash");
    builder.with_loopback_ip().with_http_port(18923).with_https_port(18923, "missing_cert.pem", "missing_key.pem");
}
