//! `#[validate(...)]` on an action parameter written with the full path of this crate must work like the imported one.

use std::time::Duration;

use asp_dot_rust::{
    ApplicationBuilder,
    controller::{ActionResult, FromOptionQuery, FromQuery, get, results},
    http_context::HttpContextRef,
    logging::LOGGER,
    macros::{controller_inject, controller_route},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

mod inner {
    pub struct FullPathValidateController {
        pub(super) http_context: super::HttpContextRef,
    }
}

#[controller_route("fullvalidate")]
#[controller_inject]
impl inner::FullPathValidateController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    #[get("/required")]
    pub async fn required(&self, #[asp_dot_rust::controller::validate(length(2))] r#type: FromQuery<String>) -> impl ActionResult {
        results::ok(r#type.into_inner())
    }

    #[get("/optional")]
    pub async fn optional(&self, #[asp_dot_rust::controller::validate(length(2))] code: FromOptionQuery<String>) -> impl ActionResult {
        results::ok(format!("{:?}", code.into_inner()))
    }
}

async fn start_server() -> u16 {
    LOGGER::disable();
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut builder = ApplicationBuilder::new("RouteFullPathValidateTest");
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

async fn get_status(port: u16, path: &str) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: 0\r\n\r\n");
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut buffer = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut buffer)).await;
    let response = String::from_utf8_lossy(&buffer).to_string();
    let status = response.split_whitespace().nth(1).and_then(|code| code.parse().ok()).unwrap_or(0);
    (status, response.split_once("\r\n\r\n").map(|(_, body)| body.to_string()).unwrap_or_default())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_path_validate_checks_the_rule_on_a_required_parameter() {
    let port = start_server().await;
    assert_eq!(get_status(port, "/fullvalidate/required?type=ab").await, (200, "ab".to_string()));
    let (status, body) = get_status(port, "/fullvalidate/required?type=a").await;
    assert_eq!(status, 422, "{body}");
    assert!(body.starts_with("type:"), "{body}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn full_path_validate_checks_the_rule_on_an_optional_parameter_and_skips_a_missing_one() {
    let port = start_server().await;
    assert_eq!(get_status(port, "/fullvalidate/optional").await, (200, "None".to_string()));
    assert_eq!(get_status(port, "/fullvalidate/optional?code=ab").await, (200, "Some(\"ab\")".to_string()));
    assert_eq!(get_status(port, "/fullvalidate/optional?code=a").await.0, 422);
}
