//! A macro of another crate that is also named `get` (and is used as `#[get("value")]`) is not a route of this framework.

use std::time::Duration;

use asp_dot_rust::{
    ApplicationBuilder, api_controller,
    controller::{ActionResult, results},
    http_context::HttpContextRef,
    logging::LOGGER,
    macros::{controller_inject, controller_route},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

api_controller!(pub ForeignController {});

#[controller_route("foreign")]
#[controller_inject]
impl ForeignController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    // a macro of another crate, written with its own path: it must be left alone
    #[other_get_macro::get("/theirs")]
    pub async fn theirs(&self) -> impl ActionResult {
        results::ok("theirs")
    }

    #[asp_dot_rust::controller::get("/ours")]
    pub async fn ours(&self) -> impl ActionResult {
        results::ok("ours")
    }
}

async fn start_server() -> u16 {
    LOGGER::disable();
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut builder = ApplicationBuilder::new("RouteForeignAttributesTest");
    builder.with_loopback_ip().with_http_port(port);
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

async fn status_of(port: u16, path: &str) -> u16 {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 0\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut buffer = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut buffer)).await;
    String::from_utf8_lossy(&buffer).split_whitespace().nth(1).and_then(|code| code.parse().ok()).unwrap_or(0)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn macro_of_another_crate_is_not_registered_as_a_route() {
    let port = start_server().await;
    assert_eq!(status_of(port, "/foreign/ours").await, 200);
    assert_eq!(status_of(port, "/foreign/theirs").await, 404);
}

#[test]
fn macro_of_another_crate_still_expands() {
    // the other macro ran: it generated this function from its own argument
    assert_eq!(ForeignController::__other_get_path_theirs(), "/theirs");
}
