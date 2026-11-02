//! Route attributes: `#[attr(key = value, ...)]` on a controller impl (default for all its routes) and on an action (overrides the key).
//! A value is stored as its own type and is read back as exactly that type: `5` is an `i32`, `5u8` a `u8`.

use std::time::Duration;

use asp_dot_rust::{
    ApplicationBuilder, api_controller,
    controller::{ActionResult, attr, get, post},
    http_context::HttpContextRef,
    logging::LOGGER,
    macros::{controller_inject, controller_route},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

/// Reads every key the tests use, as every type they are stored as, and lists the ones that match: `name=value`.
fn probe(http_context: &HttpContextRef) -> String {
    let request = &http_context.request;
    let mut found = Vec::new();
    macro_rules! read {
        ($type:ty, $key:literal, $name:literal) => {
            if let Some(value) = request.route_attribute::<$type>($key) {
                found.push(format!("{}={:?}", $name, value));
            }
        };
    }
    read!(&'static str, "zone", "zone");
    read!(i32, "limit", "limit:i32");
    read!(u8, "limit", "limit:u8");
    read!(&'static str, "limit", "limit:str");
    read!(f64, "ratio", "ratio");
    read!(bool, "flag", "flag");
    read!(i8, "extra", "extra");
    read!(i32, "a", "a");
    read!(&'static str, "b", "b");
    read!(u32, "c", "c");
    found.join(" ")
}

api_controller!(pub AttrController {});

#[controller_route("attr")]
#[attr(zone = "impl_zone", limit = 100, ratio = 0.5, flag = true)]
#[controller_inject]
impl AttrController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    // nothing of its own: every attribute of the impl
    #[get("/inherit")]
    pub async fn inherit(&self) -> impl ActionResult {
        probe(&self.http_context)
    }

    // `limit` is overridden (and is a u8 now), `extra` is new, the others come from the impl
    #[get("/override")]
    #[attr(limit = 5u8, extra = -7i8)]
    pub async fn override_limit(&self) -> impl ActionResult {
        probe(&self.http_context)
    }

    // several #[attr] on one action, any expression of a supported type
    #[get("/two")]
    #[attr(a = 1)]
    #[attr(b = "two", c = 10 * 1024u32)]
    pub async fn two(&self) -> impl ActionResult {
        probe(&self.http_context)
    }

    // the same attributes for each route of the action
    #[get("/multi")]
    #[post("/multi")]
    #[attr(limit = 9)]
    pub async fn multi(&self) -> impl ActionResult {
        probe(&self.http_context)
    }

    // the action decides the type of the value too
    #[get("/retype")]
    #[attr(limit = "text")]
    pub async fn retype(&self) -> impl ActionResult {
        probe(&self.http_context)
    }
}

/// A controller without any #[attr].
mod plain {
    use super::*;

    api_controller!(pub PlainController {});

    #[controller_route("plain")]
    #[controller_inject]
    impl PlainController {
        fn new(http_context: HttpContextRef) -> Self {
            Self { http_context }
        }

        #[get("/none")]
        pub async fn none(&self) -> impl ActionResult {
            let attributes = self.http_context.request.route_attributes().expect("the request resolved to a route");
            format!("{} {}", attributes.len(), attributes.is_empty())
        }

        #[get("/own")]
        #[attr(a = 1)]
        pub async fn own(&self) -> impl ActionResult {
            super::probe(&self.http_context)
        }
    }
}

async fn start_server() -> u16 {
    LOGGER::disable();
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut builder = ApplicationBuilder::new("RouteAttributesTest");
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
async fn route_without_attributes_gets_the_ones_of_the_impl() {
    let port = start_server().await;
    let (status, body) = request(port, "GET", "/attr/inherit").await;
    assert_eq!(status, 200);
    // `limit` is an i32 (an unsuffixed integer), so it is not found as a u8 or a str
    assert_eq!(body, "zone=\"impl_zone\" limit:i32=100 ratio=0.5 flag=true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn attribute_of_the_action_overrides_the_same_key_of_the_impl() {
    let port = start_server().await;
    let (_, body) = request(port, "GET", "/attr/override").await;
    assert_eq!(body, "zone=\"impl_zone\" limit:u8=5 ratio=0.5 flag=true extra=-7");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn several_attr_on_one_action_are_all_applied() {
    let port = start_server().await;
    let (_, body) = request(port, "GET", "/attr/two").await;
    assert_eq!(body, "zone=\"impl_zone\" limit:i32=100 ratio=0.5 flag=true a=1 b=\"two\" c=10240");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn every_route_of_an_action_gets_its_attributes() {
    let port = start_server().await;
    let expected = "zone=\"impl_zone\" limit:i32=9 ratio=0.5 flag=true";
    assert_eq!(request(port, "GET", "/attr/multi").await.1, expected);
    assert_eq!(request(port, "POST", "/attr/multi").await.1, expected);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn override_can_change_the_type_of_the_value() {
    let port = start_server().await;
    let (_, body) = request(port, "GET", "/attr/retype").await;
    assert_eq!(body, "zone=\"impl_zone\" limit:str=\"text\" ratio=0.5 flag=true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn controller_without_attr_has_no_attributes_and_gets_nothing_from_another_controller() {
    let port = start_server().await;
    assert_eq!(request(port, "GET", "/plain/none").await.1, "0 true");
    assert_eq!(request(port, "GET", "/plain/own").await.1, "a=1");
}
