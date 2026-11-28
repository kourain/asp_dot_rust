//! Route attributes written with a full path (`#[asp_dot_rust::controller::get("/")]`) must work like the imported ones.

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

api_controller!(pub FullPathController {});

#[controller_route("full")]
#[asp_dot_rust::controller::attr(zone = "impl_zone")]
#[controller_inject]
impl FullPathController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    #[asp_dot_rust::controller::get("/get")]
    pub async fn get_action(&self) -> impl ActionResult {
        results::ok("get")
    }

    #[asp_dot_rust::controller::route(["GET", "POST"], "/route")]
    pub async fn route_action(&self) -> impl ActionResult {
        results::ok("route")
    }

    #[asp_dot_rust::controller::get("/attr")]
    #[asp_dot_rust::controller::attr(limit = 7)]
    pub async fn attr_action(&self) -> impl ActionResult {
        let request = &self.http_context.request;
        let zone = request.route_attribute::<&'static str>("zone").unwrap_or("none");
        let limit = request.route_attribute::<i32>("limit").unwrap_or(-1);
        results::ok(format!("{zone} {limit}"))
    }
}

async fn start_server() -> u16 {
    LOGGER::disable();
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut builder = ApplicationBuilder::new("RouteFullPathTest");
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

async fn request(port: u16, method: &str, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 0\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut buffer = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut buffer)).await;
    let response = String::from_utf8_lossy(&buffer).to_string();
    let status = response.split_whitespace().nth(1).and_then(|code| code.parse().ok()).unwrap_or(0);
    (status, response.split_once("\r\n\r\n").map(|(_, body)| body.to_string()).unwrap_or_default())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_path_http_method_attribute_registers_the_route() {
    let port = start_server().await;
    assert_eq!(request(port, "GET", "/full/get").await, (200, "get".to_string()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_path_route_attribute_registers_every_method() {
    let port = start_server().await;
    assert_eq!(request(port, "GET", "/full/route").await, (200, "route".to_string()));
    assert_eq!(request(port, "POST", "/full/route").await, (200, "route".to_string()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_path_attr_attribute_is_applied_on_impl_and_action() {
    let port = start_server().await;
    assert_eq!(request(port, "GET", "/full/attr").await, (200, "impl_zone 7".to_string()));
}
