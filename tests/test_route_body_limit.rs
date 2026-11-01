//! The request body limit of a route (`#[max_body_size]`), and the default limit (`max_request_body_size`) for the routes without it.
//! The default limit of these tests is 100 bytes.

use std::time::Duration;

use asp_dot_rust::{
    ApplicationBuilder, api_controller,
    configuration::HyperConfig,
    controller::{ActionResult, max_body_size, post, route},
    http_context::HttpContextRef,
    logging::LOGGER,
    macros::{controller_inject, controller_route},
};
use http_body_util::BodyExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

const DEFAULT_LIMIT: usize = 100;

api_controller!(pub LimitController {});

#[controller_route]
#[controller_inject]
impl LimitController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    /// Reads the whole body, tells if the limit stopped it.
    async fn read_all(&mut self) -> &'static str {
        let body = &mut self.http_context.request.body;
        while let Some(frame) = body.frame().await {
            if frame.is_err() {
                return "limit";
            }
        }
        "ok"
    }

    #[post("/small")]
    pub async fn small(&self) -> impl ActionResult {
        "small"
    }

    // the limit may be any expression of usize, and be above the default limit
    #[post("/big")]
    pub async fn big(&self) -> impl ActionResult {
        "big"
    }

    #[post("/default")]
    pub async fn default_limit(&self) -> impl ActionResult {
        "default"
    }

    #[route(["post", "put"], "/multi")]
    #[max_body_size(10)]
    pub async fn multi(&self) -> impl ActionResult {
        "multi"
    }

    #[post("/read_small")]
    #[max_body_size(10)]
    pub async fn read_small(&mut self) -> impl ActionResult {
        self.read_all().await
    }

    #[post("/read_default")]
    pub async fn read_default(&mut self) -> impl ActionResult {
        self.read_all().await
    }

    #[post("/value_small")]
    #[max_body_size(10)]
    pub async fn value_small(&self) -> impl ActionResult {
        self.http_context.request.path()
    }

    #[post("/value_default")]
    pub async fn value_default(&self) -> impl ActionResult {
        self.http_context.request.path()
    }
}

async fn start_server() -> u16 {
    LOGGER::disable();
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut builder = ApplicationBuilder::new("RouteBodyLimitTest");
    builder.with_loopback_ip().with_http_port(port);
    builder.add_custom_configuration(|config: &mut HyperConfig| {
        config.max_request_body_size = DEFAULT_LIMIT;
    });
    builder.add_controllers();
    tokio::spawn(builder.build().run());
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return port;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server did not start on port {port}");
}

/// Sends a request and returns (status, body of the response).
async fn send(port: u16, method: &str, path: &str, framing: &str, body: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n{framing}\r\n\r\n{body}");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut buffer = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut buffer)).await;
    let response = String::from_utf8_lossy(&buffer).to_string();
    let status = response.split_whitespace().nth(1).and_then(|code| code.parse().ok()).unwrap_or(0);
    let body = response.split_once("\r\n\r\n").map(|(_, body)| body.to_string()).unwrap_or_default();
    (status, body)
}

/// A request that declares the length of its body.
async fn post_sized(port: u16, path: &str, size: usize) -> (u16, String) {
    send(port, "POST", path, &format!("Content-Length: {size}"), &"a".repeat(size)).await
}

/// A request with a chunked body, there is no length to check before reading it.
async fn post_chunked(port: u16, path: &str, size: usize) -> (u16, String) {
    send(port, "POST", path, "Transfer-Encoding: chunked", &format!("{size:x}\r\n{}\r\n0\r\n\r\n", "a".repeat(size))).await
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_limit_is_used_for_the_declared_length() {
    let port = start_server().await;
    assert_eq!(post_sized(port, "/small", 10).await, (200, "small".into()));
    assert_eq!(post_sized(port, "/small", 11).await.0, 413);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_without_limit_uses_the_default_limit() {
    let port = start_server().await;
    assert_eq!(post_sized(port, "/default", DEFAULT_LIMIT).await, (200, "default".into()));
    assert_eq!(post_sized(port, "/default", DEFAULT_LIMIT + 1).await.0, 413);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_limit_can_be_above_the_default_limit() {
    let port = start_server().await;
    assert_eq!(post_sized(port, "/big", 1000).await, (200, "big".into()));
    assert_eq!(post_sized(port, "/big", 1001).await.0, 413);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn limit_applies_to_every_method_of_the_route() {
    let port = start_server().await;
    for method in ["POST", "PUT"] {
        let (status, body) = send(port, method, "/multi", "Content-Length: 10", &"a".repeat(10)).await;
        assert_eq!((status, body.as_str()), (200, "multi"), "{method}");
        let (status, _) = send(port, method, "/multi", "Content-Length: 11", &"a".repeat(11)).await;
        assert_eq!(status, 413, "{method}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unknown_route_uses_the_default_limit() {
    let port = start_server().await;
    assert_eq!(post_sized(port, "/nothing_here", DEFAULT_LIMIT).await.0, 404);
    assert_eq!(post_sized(port, "/nothing_here", DEFAULT_LIMIT + 1).await.0, 413);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn chunked_body_is_cut_at_the_limit_of_the_route() {
    let port = start_server().await;
    assert_eq!(post_chunked(port, "/read_small", 10).await, (200, "ok".into()));
    assert_eq!(post_chunked(port, "/read_small", 50).await, (200, "limit".into()));
    // the same body is under the default limit of a route without its own
    assert_eq!(post_chunked(port, "/read_default", 50).await, (200, "ok".into()));
    assert_eq!(post_chunked(port, "/read_default", DEFAULT_LIMIT + 1).await, (200, "limit".into()));
}
