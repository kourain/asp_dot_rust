use asp_dot_rust::{
    Application, ApplicationBuilder, api_controller,
    controller::{ActionResult, get, post},
    http_context::HttpContextRef,
    logging::LOGGER,
    macros::{controller_inject, controller_route},
};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use asp_dot_rust::configuration::HyperConfig;

api_controller!(pub ListenerController {});

#[controller_route]
#[controller_inject]
impl ListenerController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }
    #[get("/listener")]
    pub async fn listener(&self) -> impl ActionResult {
        "listening"
    }
    #[post("/upload")]
    pub async fn upload(&self) -> impl ActionResult {
        "uploaded"
    }
    #[get("/utf8-header")]
    pub async fn utf8_header(&mut self) -> impl ActionResult {
        // a UTF-8 value (e.g. a file name) is a valid header value (obs-text)
        self.http_context.response.headers.insert("x-file-name", http::HeaderValue::from_bytes("caf\u{e9}.txt".as_bytes()).unwrap());
        self.http_context.response.headers.insert("x-plain", http::HeaderValue::from_static("plain"));
        "h"
    }
}

/// `with_loopback_ip()` + two ports: only the first port was ever opened (`ips.zip(ports)`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_registered_port_is_served() {
    LOGGER::disable();
    let mut builder = ApplicationBuilder::new("Ports");
    builder.with_loopback_ip().with_http_port(18701).with_http_port(18702);
    builder.add_controllers();
    tokio::spawn(builder.build().run());
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", 18702)).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    for port in [18701, 18702] {
        let response = raw_request(port, "GET", "/listener", "").await;
        assert_eq!(status_of(&response), 200, "port {port}: {response}");
        assert_eq!(body_of(&response), "listening");
    }
}

/// `with_any_ip()` binds `0.0.0.0` and `::`. Which one was served used to depend on `HashSet`
/// order, and a host without IPv6 crashed on some runs. It must always serve IPv4 and must not
/// fail because IPv6 is missing or because `[::]` already covers IPv4.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn with_any_ip_always_serves_ipv4() {
    LOGGER::disable();
    let mut builder = ApplicationBuilder::new("AnyIp");
    builder.with_any_ip().with_http_port(18703);
    builder.add_controllers();
    let handle = tokio::spawn(builder.build().run());
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", 18703)).await.is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let response = raw_request(18703, "GET", "/listener", "").await;
    assert_eq!(status_of(&response), 200, "{response}");
    assert!(!handle.is_finished(), "the server stopped");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_fails_loudly_when_a_port_is_taken() {
    LOGGER::disable();
    let taken = std::net::TcpListener::bind("127.0.0.1:18704").unwrap();
    let mut builder = ApplicationBuilder::new("Taken");
    builder.with_loopback_ip().with_http_port(18704);
    builder.add_controllers();
    let result = tokio::spawn(builder.build().run()).await;
    drop(taken);
    let error = result.expect_err("run() should have panicked");
    assert!(error.is_panic());
}

fn limits(max_headers: usize, max_body: usize) -> impl FnOnce(&mut ApplicationBuilder) {
    move |builder| {
        builder.add_custom_configuration(|c: &mut HyperConfig| {
            c.max_request_headers_size = max_headers;
            c.max_request_body_size = max_body;
        });
    }
}

async fn post(port: u16, declared_length: usize, body: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!("POST /upload HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {declared_length}\r\n\r\n{body}");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut buffer = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut buffer)).await;
    String::from_utf8_lossy(&buffer).to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn request_body_above_max_request_body_size_gets_413() {
    start(18705, limits(32 * 1024, 100), |_| {}).await;
    let accepted = post(18705, 5, "hello").await;
    assert_eq!(status_of(&accepted), 200, "{accepted}");
    // the declared length alone is enough, the body is not even read
    let rejected = post(18705, 101, "").await;
    assert_eq!(status_of(&rejected), 413, "{rejected}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn headers_above_max_request_headers_size_get_431() {
    start(18706, limits(1024, 1024), |_| {}).await;
    assert_eq!(status_of(&raw_request(18706, "GET", "/listener", "").await), 200);
    let big = format!("X-Big: {}\r\n", "a".repeat(2000));
    let rejected = raw_request(18706, "GET", "/listener", &big).await;
    assert_eq!(status_of(&rejected), 431, "{rejected}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn non_ascii_header_values_are_sent() {
    start(18707, |_| {}, |_| {}).await;
    let response = raw_request(18707, "GET", "/utf8-header", "").await;
    assert_eq!(status_of(&response), 200, "{response}");
    assert_eq!(header_of(&response, "x-plain").as_deref(), Some("plain"));
    // used to be dropped silently because HeaderValue::to_str() rejects non-ASCII bytes
    assert_eq!(header_of(&response, "x-file-name").as_deref(), Some("caf\u{e9}.txt"));
}

/// Sends one raw HTTP/1.1 request to 127.0.0.1 and returns the whole response as text
/// (or a string starting with `ERROR`/`TIMEOUT` when no complete response came back).
async fn raw_request(port: u16, method: &str, path: &str, extra_headers: &str) -> String {
    let mut stream = match TcpStream::connect(("127.0.0.1", port)).await {
        Ok(stream) => stream,
        Err(e) => return format!("ERROR connect: {e}"),
    };
    let request = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n{extra_headers}\r\n");
    if let Err(e) = stream.write_all(request.as_bytes()).await {
        return format!("ERROR write: {e}");
    }
    let mut buffer = Vec::new();
    match tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut buffer)).await {
        Ok(Ok(_)) => String::from_utf8_lossy(&buffer).to_string(),
        Ok(Err(e)) => format!("ERROR read: {e}"),
        Err(_) => "TIMEOUT: no complete response within 5s".to_string(),
    }
}

/// `HTTP/1.1 200 OK ...` -> 200 (0 when the response is not an HTTP response)
fn status_of(response: &str) -> u16 {
    response.split_whitespace().nth(1).and_then(|code| code.parse().ok()).unwrap_or(0)
}

fn header_of(response: &str, name: &str) -> Option<String> {
    let head = response.split("\r\n\r\n").next()?;
    head.lines().skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim().eq_ignore_ascii_case(name).then(|| value.trim().to_string())
    })
}

fn body_of(response: &str) -> &str {
    response.split_once("\r\n\r\n").map(|(_, body)| body).unwrap_or("")
}

async fn start(port: u16, configure: impl FnOnce(&mut ApplicationBuilder), setup: impl FnOnce(&mut Application)) {
    LOGGER::disable();
    let mut builder = ApplicationBuilder::new("Test");
    builder.with_loopback_ip().with_http_port(port);
    configure(&mut builder);
    builder.add_controllers();
    let mut app = builder.build();
    setup(&mut app);
    tokio::spawn(app.run());
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server on port {port} did not start");
}
