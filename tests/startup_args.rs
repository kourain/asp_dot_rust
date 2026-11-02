use asp_dot_rust::{ApplicationBuilder, logging::{LOGGER, LogCommand}};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

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

/// `with_https_port` used to end in `todo!()`, so it always panicked (and so did the
/// `--https-port` flag, from inside `ApplicationBuilder::new()`).
#[tokio::test]
async fn registering_an_https_port_does_not_panic_and_http_keeps_working() {
    LOGGER::disable();
    let mut builder = ApplicationBuilder::new("HttpsStub");
    builder.with_loopback_ip().with_http_port(18921).with_https_port(18922);
    builder.add_controllers();
    tokio::spawn(builder.build().run());

    let mut stream = connect_with_retry(18921).await;
    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 404"), "unexpected response: {response}");

    // nothing listens on the HTTPS port yet
    assert!(TcpStream::connect(("127.0.0.1", 18922)).await.is_err());
}

async fn connect_with_retry(port: u16) -> TcpStream {
    for _ in 0..50 {
        if let Ok(stream) = TcpStream::connect(("127.0.0.1", port)).await {
            return stream;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server on port {port} did not start");
}
