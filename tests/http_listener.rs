#[path = "http_listener/body_fields.rs"]
mod body_fields;

use asp_dot_rust::{
    Application, ApplicationBuilder, api_controller,
    controller::{ActionResult, Body, Form, FromBody, FromForm, FromOptionQuery, FromQuery, FromRoute, Validate, ValidationError, get, post, query, results, route},
    http_context::{HttpContextRef, HttpHeaderExt},
    logging::LOGGER,
    macros::{controller_inject, controller_route},
};
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use asp_dot_rust::configuration::{CorsConfiguration, HyperConfig, RequestTimeoutConfiguration};

api_controller!(pub ListenerController {});

// A rule defined outside the framework follows the same dispatch path as length and range.
trait PrefixRule {
    /// Require the decoded query string to start with the specified prefix.
    /// For example, `#[validate(prefix("ok"))]` accepts `okay` and rejects `no`.
    fn prefix(value: &str, field: &str, prefix: &str) -> Result<(), ValidationError>;
}

impl PrefixRule for FromQuery<String> {
    fn prefix(value: &str, field: &str, prefix: &str) -> Result<(), ValidationError> {
        if value.starts_with(prefix) { Ok(()) } else { Err(ValidationError::new(field, "invalid prefix")) }
    }
}

#[derive(serde::Deserialize)]
struct SearchFilter {
    term: String,
}

#[derive(serde::Deserialize)]
struct LimitedSearch {
    term: String,
}

impl Validate for LimitedSearch {
    fn validate(&self) -> Result<(), ValidationError> {
        if self.term.chars().count() > 3 {
            return Err(ValidationError::new("term", "maximum length is 3"));
        }
        Ok(())
    }
}

#[controller_route]
#[controller_inject]
impl ListenerController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }
    #[get("/listener")]
    pub async fn listener(&self) -> impl ActionResult {
        results::ok("listening")
    }
    #[post("/upload")]
    pub async fn upload(&self) -> impl ActionResult {
        results::ok("uploaded")
    }
    #[query("/search")]
    #[route(["QuErY"], "/search-alias")]
    async fn search(&self, filter: Body<SearchFilter>) -> impl ActionResult {
        results::ok(filter.into_inner().term)
    }
    #[post("/search-form")]
    fn search_form(&self, filter: Form<SearchFilter>) -> impl ActionResult {
        results::ok(filter.into_inner().term)
    }
    #[post("/limited-form")]
    fn limited_form(&self, filter: Form<LimitedSearch>) -> impl ActionResult {
        results::ok(filter.into_inner().term)
    }
    #[post("/limited-body")]
    async fn limited_body(&self, filter: Body<LimitedSearch>) -> impl ActionResult {
        results::ok(filter.into_inner().term)
    }
    #[query("/search-term")]
    async fn search_term(&self, term: FromBody<String>) -> impl ActionResult {
        results::ok(term.into_inner())
    }
    #[post("/fields-form")]
    fn fields_form(&self, name: FromForm<String>, age: FromForm<u32>, admin: FromForm<bool>, id: FromForm<uuid::Uuid>) -> impl ActionResult {
        results::ok(format!("{} {} {} {}", *name, *age, *admin, *id))
    }
    #[post("/fields-body")]
    fn fields_body(&self, name: FromBody<String>, age: FromBody<u32>, nick: FromBody<Option<String>>, tags: FromBody<Vec<String>>) -> impl ActionResult {
        results::ok(format!("{} {} {:?} {:?}", *name, *age, *nick, *tags))
    }
    #[post("/body-flag")]
    fn body_flag(&self, flag: FromBody<bool>) -> impl ActionResult {
        results::ok(flag.to_string())
    }
    #[post("/whole-and-field-form")]
    fn whole_and_field_form(&self, user: Form<SearchFilter>, extra: FromForm<String>) -> impl ActionResult {
        results::ok(format!("{} {}", user.term, *extra))
    }
    #[post("/field-then-raw-body")]
    fn field_then_raw_body(&self, term: FromBody<String>, raw: Body<String>) -> impl ActionResult {
        results::ok(format!("{}|{}", *term, *raw))
    }
    #[post("/field-then-action-reads")]
    async fn field_then_action_reads(&mut self, term: FromBody<String>) -> impl ActionResult {
        // the binder rewound the body, so the action reads it from the start
        let raw = self.http_context.request.body.read_to_end().await.unwrap();
        results::ok(format!("{}|{}", *term, String::from_utf8(raw).unwrap()))
    }
    #[get("/bounded-query")]
    fn bounded_query(
        &self,
        #[validate(length(1, 80))] term: FromQuery<String>,
        #[validate(range(min = 1, max = 100))] limit: FromQuery<u8>,
        #[validate(range(min = -10, max = 10))] offset: FromQuery<i8>,
        #[validate(range(min = 0.0, max = 1.0))] score: FromQuery<f64>,
    ) -> impl ActionResult {
        results::ok(format!("{} {} {} {}", *term, *limit, *offset, *score))
    }
    #[get("/optional-query")]
    fn optional_query(
        &self,
        #[validate(length(1))] term: FromOptionQuery<String>,
        #[validate(range(max = 0.5))] score: FromOptionQuery<f32>,
        #[validate(range(min = 340282366920938463463374607431768211454))] count: FromOptionQuery<u128>,
        legacy: FromOptionQuery<u8>,
    ) -> impl ActionResult {
        results::ok(format!("{:?} {:?} {:?} {:?}", *term, *score, *count, *legacy))
    }
    #[get("/query-rule-method")]
    fn query_rule_method(
        &self,
        #[validate(length(2))] code: FromQuery<String>,
        #[validate(prefix("ok"))] tag: FromOptionQuery<String>,
    ) -> impl ActionResult {
        results::ok(format!("{} {:?}", *code, *tag))
    }
    #[get("/typed/{id}")]
    fn typed(&self, id: FromRoute<uuid::Uuid>, ulid: FromQuery<ulid::Ulid>, ip: FromOptionQuery<std::net::IpAddr>) -> impl ActionResult {
        results::ok(format!("{} {} {:?}", *id, *ulid, *ip))
    }
    #[get("/names/{name}")]
    fn names(&self, name: FromRoute<String>, note: FromOptionQuery<String>) -> impl ActionResult {
        results::ok(format!("{} {:?}", *name, *note))
    }
    #[get("/flag")]
    fn flag(&self, verbose: FromOptionQuery<bool>, count: FromOptionQuery<u32>) -> impl ActionResult {
        results::ok(format!("{} {:?} {:?}", verbose.flag(), *verbose, *count))
    }
    #[get("/utf8-header")]
    pub async fn utf8_header(&mut self) -> impl ActionResult {
        // a UTF-8 value (e.g. a file name) is a valid header value (obs-text)
        self.http_context.response.headers.insert("x-file-name", http::HeaderValue::from_bytes("caf\u{e9}.txt".as_bytes()).unwrap());
        self.http_context.response.headers.insert("x-plain", http::HeaderValue::from_static("plain"));
        results::ok("h")
    }
}

asp_dot_rust::middleware!(pub RuntimeMiddleware, |ctx, next| {
    ctx.response.headers.insert_str("X-Runtime-Middleware", "active");
    next.next_async(ctx).await;
});

asp_dot_rust::middleware!(pub VaryByLanguage, |ctx, next| {
    ctx.response.headers.insert_str("Vary", "Accept-Language");
    next.next_async(ctx).await;
});

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cors_preserves_vary_and_routes_ordinary_options() {
    let port = 18710;
    start(
        port,
        |builder| {
            builder.configuration.insert(|cors: &mut CorsConfiguration| {
                cors.allow_origin("https://example.test");
            });
        },
        |app| {
            app.add_middleware::<VaryByLanguage>().use_cors();
        },
    )
    .await;

    for (headers, expected_status) in [("", 405), ("Origin: https://example.test\r\n", 405), ("Origin: https://other.test\r\n", 403)] {
        let response = raw_request(port, "OPTIONS", "/listener", headers).await;
        assert_eq!(status_of(&response), expected_status, "{response}");
        let vary: Vec<_> = response
            .lines()
            .filter_map(|line| line.split_once(':'))
            .filter(|(key, _)| key.eq_ignore_ascii_case("vary"))
            .flat_map(|(_, value)| value.split(',').map(str::trim))
            .collect();
        assert!(vary.contains(&"Accept-Language") && vary.contains(&"Origin"), "{response}");
    }
}

asp_dot_rust::middleware!(pub StalledResponse, |ctx, _next| {
    ctx.response.headers.insert_str("Content-Encoding", "gzip");
    ctx.response.headers.insert_str("Content-Type", "application/json");
    ctx.response.headers.insert_str("ETag", "\"original\"");
    ctx.response.headers.insert_str("Content-Length", "999");
    std::future::pending::<()>().await;
});

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn timeout_sends_headers_for_its_own_body() {
    let port = 18711;
    start(
        port,
        |builder| {
            builder.configuration.insert(|cfg: &mut RequestTimeoutConfiguration| cfg.timeout_seconds = 0);
        },
        |app| {
            app.use_request_timeout().add_middleware::<StalledResponse>();
        },
    )
    .await;
    let response = raw_request(port, "GET", "/listener", "").await;
    assert_eq!(status_of(&response), 408, "{response}");
    assert_eq!(body_of(&response), "Request Timeout");
    assert_eq!(header_of(&response, "content-type").as_deref(), Some("text/plain; charset=utf-8"));
    assert_eq!(header_of(&response, "content-length").as_deref(), Some("15"));
    assert!(header_of(&response, "content-encoding").is_none());
    assert!(header_of(&response, "etag").is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_factory_data_is_released_while_the_pipeline_keeps_serving() {
    let startup_data = Arc::new(vec![0_u8; 1024]);
    let weak = Arc::downgrade(&startup_data);
    start(
        18708,
        |_| {},
        move |app| {
            app.add_middleware_factory(move |next| {
                assert_eq!(startup_data.len(), 1024);
                RuntimeMiddleware { next }
            });
        },
    )
    .await;

    assert!(weak.upgrade().is_none(), "middleware factories must be dropped before serving requests");
    for _ in 0..2 {
        let response = raw_request(18708, "GET", "/listener", "").await;
        assert_eq!(status_of(&response), 200, "{response}");
        assert_eq!(body_of(&response), "listening");
        assert_eq!(header_of(&response, "x-runtime-middleware").as_deref(), Some("active"));
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
        builder.configuration.insert(|c: &mut HyperConfig| {
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
    raw_request_with_body(port, method, path, extra_headers, "").await
}

async fn raw_request_with_body(port: u16, method: &str, path: &str, extra_headers: &str, body: impl AsRef<[u8]>) -> String {
    let mut stream = match TcpStream::connect(("127.0.0.1", port)).await {
        Ok(stream) => stream,
        Err(e) => return format!("ERROR connect: {e}"),
    };
    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n{extra_headers}\r\n").into_bytes();
    request.extend_from_slice(body.as_ref());
    if let Err(e) = stream.write_all(&request).await {
        return format!("ERROR write: {e}");
    }
    let mut buffer = Vec::new();
    match tokio::time::timeout(Duration::from_secs(5), stream.read_to_end(&mut buffer)).await {
        Ok(Ok(_)) => String::from_utf8_lossy(&buffer).to_string(),
        Ok(Err(e)) => format!("ERROR read: {e}"),
        Err(_) => "TIMEOUT: no complete response within 5s".to_string(),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_routes_bind_body_and_are_discoverable() {
    let port = 18709;
    start(
        port,
        |builder| {
            builder.add_swagger_gen();
            builder.configuration.insert(|_: &mut CorsConfiguration| {});
        },
        |app| {
            app.use_cors().use_swagger();
        },
    )
    .await;

    let method = http::Method::QUERY.as_str();
    let body = r#"{"term":"hello"}"#;
    let length = format!("Content-Length: {}\r\n", body.len());
    let json_headers = format!("{length}Content-Type: application/json\r\n");
    for path in ["/search", "/search-alias"] {
        let response = raw_request_with_body(port, method, path, &json_headers, body).await;
        assert_eq!(status_of(&response), 200, "{response}");
        assert_eq!(body_of(&response), "hello");
    }

    let missing_type = raw_request_with_body(port, method, "/search", &length, body).await;
    assert_eq!(status_of(&missing_type), 400, "{missing_type}");
    let text_headers = format!("{length}Content-Type: text/plain\r\n");
    let unsupported = raw_request_with_body(port, method, "/search", &text_headers, body).await;
    assert_eq!(status_of(&unsupported), 415, "{unsupported}");

    let wrong_method = raw_request(port, "GET", "/search", "").await;
    assert_eq!(status_of(&wrong_method), 405, "{wrong_method}");
    assert_eq!(header_of(&wrong_method, "allow").as_deref(), Some(method));

    let preflight = raw_request(port, "OPTIONS", "/search", "Origin: https://example.test\r\nAccess-Control-Request-Method: QUERY\r\n").await;
    assert_eq!(status_of(&preflight), 204, "{preflight}");
    assert_eq!(header_of(&preflight, "access-control-allow-methods").as_deref(), Some(method));

    let swagger = raw_request(port, "GET", "/swagger/v1/swagger.json", "").await;
    assert_eq!(status_of(&swagger), 200, "{swagger}");
    let document: serde_json::Value = serde_json::from_str(body_of(&swagger)).unwrap();
    assert_eq!(document["openapi"], "3.2.1");
    let search = &document["paths"]["/search"]["query"];
    let alias = &document["paths"]["/search-alias"]["query"];
    assert!(search.is_object());
    assert!(alias.is_object());
    assert_ne!(search["operationId"], alias["operationId"]);
    assert!(document["paths"]["/listener"]["get"].is_object());
    assert!(document["paths"]["/search"].get("additionalOperations").is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn controller_validates_only_dtos_that_implement_validate() {
    let port = 18712;
    start(port, |_| {}, |_| {}).await;
    let form = "application/x-www-form-urlencoded";
    let json = "application/json";
    let error = "term: maximum length is 3";
    for (method, path, media, body, status, expected) in [
        ("POST", "/search-form", form, "term=hello", 200, "hello"),
        ("QUERY", "/search", json, r#"{"term":"hello"}"#, 200, "hello"),
        ("POST", "/limited-form", form, "term=abc", 200, "abc"),
        ("POST", "/limited-form", form, "term=abcd", 422, error),
        ("POST", "/limited-body", json, r#"{"term":"abc"}"#, 200, "abc"),
        ("POST", "/limited-body", json, r#"{"term":"abcd"}"#, 422, error),
        ("POST", "/limited-body", "application/xml", "<root><term>abcd</term></root>", 422, error),
    ] {
        let headers = format!("Content-Type: {media}\r\nContent-Length: {}\r\n", body.len());
        let response = raw_request_with_body(port, method, path, &headers, body).await;
        assert_eq!(status_of(&response), status, "{path}: {response}");
        assert_eq!(body_of(&response), expected, "{path}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn query_rules_check_typed_values_and_skip_absent_optional_fields() {
    let port = 18713;
    start(port, |_| {}, |_| {}).await;
    for (length, status) in [(80, 200), (81, 422)] {
        let term = "%E1%BA%BF".repeat(length);
        let path = format!("/bounded-query?term={term}&limit=1&offset=0&score=0");
        let response = raw_request(port, "GET", &path, "").await;
        assert_eq!(status_of(&response), status, "{response}");
    }
    for (query, expected) in [
        ("/bounded-query?term=x&limit=1&offset=-10&score=0.0", "x 1 -10 0"),
        ("/bounded-query?term=y&limit=100&offset=10&score=1.0", "y 100 10 1"),
        ("/optional-query", "None None None None"),
        ("/query-rule-method?code=%E1%BA%BFx", "ếx None"),
        ("/query-rule-method?code=ab&tag=okay", "ab Some(\"okay\")"),
        ("/optional-query?term=%E1%BA%BF&score=0.5", "Some(\"ế\") Some(0.5) None None"),
        ("/optional-query?count=340282366920938463463374607431768211454", "None None Some(340282366920938463463374607431768211454) None"),
    ] {
        let response = raw_request(port, "GET", query, "").await;
        assert_eq!(status_of(&response), 200, "{query}: {response}");
        assert_eq!(body_of(&response), expected, "{query}");
    }
    for (query, field) in [
        ("/bounded-query?term=&limit=1&offset=0&score=0", "term"),
        ("/bounded-query?term=x&offset=0&score=0", "limit"),
        ("/bounded-query?term=x&limit=101&offset=0&score=0", "limit"),
        ("/bounded-query?term=x&limit=1&offset=-11&score=0", "offset"),
        ("/bounded-query?term=x&limit=1&offset=0&score=1.1", "score"),
        ("/optional-query?term=", "term"),
        ("/optional-query?term=ab", "term"),
        ("/query-rule-method?code=x", "code"),
        ("/query-rule-method?code=xyz", "code"),
        ("/query-rule-method?code=xy&tag=no", "tag"),
        ("/optional-query?term=%FF", "term"),
        ("/optional-query?legacy=bad", "legacy"),
        ("/optional-query?legacy=", "legacy"),
        ("/optional-query?legacy=256", "legacy"),
        ("/optional-query?legacy=%FF", "legacy"),
        ("/optional-query?score=bad", "score"),
        ("/optional-query?score=NaN", "score"),
        ("/optional-query?score=-inf", "score"),
        ("/optional-query?count=340282366920938463463374607431768211453", "count"),
    ] {
        let response = raw_request(port, "GET", query, "").await;
        assert_eq!(status_of(&response), 422, "{query}: {response}");
        assert!(body_of(&response).contains(field), "{query}: {response}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_and_query_binders_accept_any_from_str_type() {
    let port = 18714;
    start(port, |_| {}, |_| {}).await;
    let uuid = "0190d9d0-7b1e-7c3a-8f2e-123456789abc";
    let ulid = "01ARZ3NDEKTSV4RRFFQ69G5FAV";
    for (path, expected) in [
        (format!("/typed/{uuid}?ulid={ulid}"), format!("{uuid} {ulid} None")),
        (format!("/typed/{uuid}?ulid={ulid}&ip=127.0.0.1"), format!("{uuid} {ulid} Some(127.0.0.1)")),
        (format!("/typed/{uuid}?ulid={ulid}&ip=%3A%3A1"), format!("{uuid} {ulid} Some(::1)")),
    ] {
        let response = raw_request(port, "GET", &path, "").await;
        assert_eq!(status_of(&response), 200, "{path}: {response}");
        assert_eq!(body_of(&response), expected, "{path}");
    }
    // A route value that does not parse is 404, a query value that does not parse (or is missing) is 422.
    for (path, status, field) in [
        ("/typed/not-a-uuid?ulid=01ARZ3NDEKTSV4RRFFQ69G5FAV".to_string(), 404, "id"),
        (format!("/typed/{uuid}?ulid=nope"), 422, "ulid"),
        (format!("/typed/{uuid}"), 422, "ulid"),
        (format!("/typed/{uuid}?ulid={ulid}&ip=999.1.1.1"), 422, "ip"),
    ] {
        let response = raw_request(port, "GET", &path, "").await;
        assert_eq!(status_of(&response), status, "{path}: {response}");
        assert!(body_of(&response).contains(field), "{path}: {response}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn route_values_are_decoded_and_invalid_utf8_is_an_error() {
    let port = 18715;
    start(port, |_| {}, |_| {}).await;
    for (path, expected) in [
        ("/names/a%20b", "a b None"),
        ("/names/%E1%BA%BF", "ế None"),
        ("/names/a+b", "a+b None"),
        ("/names/a%2Fb", "a/b None"),
        ("/names/x?note=a+b", "x Some(\"a b\")"),
        ("/names/x?note=1%2B1", "x Some(\"1+1\")"),
        ("/names/x?note", "x Some(\"\")"),
        ("/names/x?note=1&note=2", "x Some(\"2\")"),
    ] {
        let response = raw_request(port, "GET", path, "").await;
        assert_eq!(status_of(&response), 200, "{path}: {response}");
        assert_eq!(body_of(&response), expected, "{path}");
    }
    for (path, status, field) in [("/names/%FF", 404, "name"), ("/names/x?note=%FF", 422, "note")] {
        let response = raw_request(port, "GET", path, "").await;
        assert_eq!(status_of(&response), status, "{path}: {response}");
        assert!(body_of(&response).contains(field), "{path}: {response}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn optional_bool_query_works_as_a_flag() {
    let port = 18716;
    start(port, |_| {}, |_| {}).await;
    for (path, expected) in [
        ("/flag", "false None None"),
        ("/flag?verbose", "true Some(true) None"),
        ("/flag?verbose=", "true Some(true) None"),
        ("/flag?verbose=true", "true Some(true) None"),
        ("/flag?verbose=false", "false Some(false) None"),
        ("/flag?verbose&count=3", "true Some(true) Some(3)"),
    ] {
        let response = raw_request(port, "GET", path, "").await;
        assert_eq!(status_of(&response), 200, "{path}: {response}");
        assert_eq!(body_of(&response), expected, "{path}");
    }
    // Only `bool` reads an empty value as a flag, a number needs a digit.
    for (path, field) in [("/flag?verbose=yes", "verbose"), ("/flag?count", "count"), ("/flag?count=-1", "count")] {
        let response = raw_request(port, "GET", path, "").await;
        assert_eq!(status_of(&response), 422, "{path}: {response}");
        assert!(body_of(&response).contains(field), "{path}: {response}");
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
