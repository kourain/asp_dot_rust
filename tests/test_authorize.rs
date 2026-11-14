//! Basic and JWT authorize middleware: parse header and store AuthUser under "user".
use asp_dot_rust::{
    ApplicationBuilder, api_controller,
    configuration::JwtAuthConfiguration,
    controller::{ActionResult, get},
    http_context::{HttpContextRef, USER_CONTEXT_KEY},
    logging::LOGGER,
    macros::{controller_inject, controller_route},
    middleware::AuthUser,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use jsonwebtoken::{EncodingKey, Header, encode};
use serde_json::json;
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

api_controller!(pub AuthController {});
#[controller_route]
#[controller_inject]
impl AuthController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    #[get("/whoami")]
    pub async fn whoami(&mut self) -> impl ActionResult {
        match self.http_context.get_item::<AuthUser>(USER_CONTEXT_KEY) {
            Some(user) => format!("{}:{}", user.scheme, user.name),
            None => "none".to_string(),
        }
    }
}

fn free_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port()
}

fn wait_ready(port: u16) {
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("server on port {port} did not start");
}

fn http_get(port: u16, path: &str, auth: Option<&str>) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let auth_line = auth.map(|a| format!("Authorization: {a}\r\n")).unwrap_or_default();
    let req = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\n{auth_line}Connection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).unwrap();
    let mut buf = Vec::new();
    let _ = stream.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);
    let status = text
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = text.split("\r\n\r\n").nth(1).unwrap_or("").trim().to_string();
    (status, body)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn basic_auth_stores_user() {
    LOGGER::disable();
    let port = free_port();
    let mut builder = ApplicationBuilder::new("basic_auth_test");
    builder.with_loopback_ip().with_http_port(port);
    builder.add_controllers();
    let mut app = builder.build();
    app.use_authorize_basic();
    tokio::spawn(app.run());
    wait_ready(port);

    let (status, _) = http_get(port, "/whoami", None);
    assert_eq!(status, 401);

    let credentials = BASE64.encode(b"alice:secret");
    let (status, body) = http_get(port, "/whoami", Some(&format!("Basic {credentials}")));
    assert_eq!(status, 200);
    assert_eq!(body, "Basic:alice");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn jwt_auth_stores_user() {
    LOGGER::disable();
    let port = free_port();
    let mut builder = ApplicationBuilder::new("jwt_auth_test");
    builder.with_loopback_ip().with_http_port(port);
    builder.configuration.insert::<JwtAuthConfiguration>(|value| {
        value.secret_key = "test-secret-key-32-chars-minimum!".to_string();
        value.token_expiration_seconds = 3600;
        value.token_issuer = "test-issuer".to_string();
        value.token_audience = "test-audience".to_string();
    });
    builder.add_controllers();
    let mut app = builder.build();
    app.use_authorize_bearer();
    tokio::spawn(app.run());
    wait_ready(port);

    let (status, _) = http_get(port, "/whoami", None);
    assert_eq!(status, 401);

    let claims = json!({
        "sub": "bob",
        "iss": "test-issuer",
        "aud": "test-audience",
        "exp": (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
    });
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(b"test-secret-key-32-chars-minimum!").unwrap(),
    )
    .unwrap();
    let (status, body) = http_get(port, "/whoami", Some(&format!("Bearer {token}")));
    assert_eq!(status, 200);
    assert_eq!(body, "Bearer:bob");
}
