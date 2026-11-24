//! Integration tests of the response stream: a complete response (controller result) and a streamed response (`write_async`).
//! Every test starts a real server on a free loopback port and talks to it with raw TCP (HTTP/1.1) or the hyper client (HTTP/2).

#[path = "stream_response/action_result_body.rs"]
mod action_result_body;

use std::{
    io,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use asp_dot_rust::{
    Application, ApplicationBuilder, api_controller,
    configuration::CompressionConfiguration,
    controller::{ActionResult, FromQuery, FromRoute, get, results, route},
    http_context::{HttpContext, HttpContextRef, HttpHeaderExt, HttpResponse},
    logging::{LOGGER, LogCommand, LogLevel},
    macros::{controller_inject, controller_route},
    middleware::{Middleware, MiddlewareNext},
    services::{configuration::CfgEntryAllowReloadFile::NotAllowed, service_provider::service_provider_scope::ServiceProviderScope},
};
use http_body_util::{BodyExt, Empty};
use hyper::body::Bytes;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::Semaphore,
};

// ---------------------------------------------------------------------------
// Controller under test
// ---------------------------------------------------------------------------

static DISCONNECT_SEEN: AtomicBool = AtomicBool::new(false);
static PRODUCED_CHUNKS: AtomicUsize = AtomicUsize::new(0);
static NEXT_RESULT_CHUNK: Semaphore = Semaphore::const_new(0);

const TEXT_BODY: &str = "<html><body>Hello world</body></html>";
const LARGE_BODY_SIZE: usize = 1_000_000;
const BACKPRESSURE_CHUNKS: usize = 400;
const BACKPRESSURE_CHUNK_SIZE: usize = 64 * 1024;

fn large_body() -> Vec<u8> {
    (0..LARGE_BODY_SIZE).map(|i| (i % 251) as u8).collect()
}

struct BorrowedText<'a>(&'a str);

#[derive(serde::Serialize)]
pub struct Item {
    name: &'static str,
}

#[async_trait::async_trait]
impl ActionResult for BorrowedText<'_> {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::CREATED
    }

    fn write_header(&self, headers: &mut http::HeaderMap) {
        headers.insert(http::header::CONTENT_TYPE, http::HeaderValue::from_static("text/plain; charset=utf-8"));
    }

    async fn write_body_async(&self, _request_headers: &http::HeaderMap, response: &mut HttpResponse) -> io::Result<()> {
        response.body.extend_from_slice(self.0.as_bytes());
        Ok(())
    }
}

struct StreamedText<'a>(&'a str);

#[async_trait::async_trait]
impl ActionResult for StreamedText<'_> {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::ACCEPTED
    }

    fn write_header(&self, headers: &mut http::HeaderMap) {
        headers.insert_str("Content-Type", "text/plain");
    }

    async fn write_body_async(&self, _request_headers: &http::HeaderMap, response: &mut HttpResponse) -> io::Result<()> {
        response.write_async(self.0.as_bytes()).await?;
        NEXT_RESULT_CHUNK.acquire().await.unwrap().forget();
        response.write_async(b"last").await
    }
}

api_controller!(pub StreamController {});

#[controller_route]
#[controller_inject]
impl StreamController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    // ----- complete responses -----

    #[get("/complete/model")]
    pub fn complete_model(&self) -> results::RouteResult<Item> {
        results::Created(Item { name: "A&B" }).with_location("/items/1").unwrap()
    }

    #[get("/complete/collection/{kind}")]
    pub fn complete_collection(&self, kind: FromRoute<String>) -> Box<dyn ActionResult> {
        match kind.as_str() {
            "items" => results::ok(vec![Item { name: "A&B" }]).into(),
            "vec" => results::ok(vec![0u8, 255]).into(),
            "array" => results::ok([0u8, 255]).into(),
            "slice" => results::ok(&[0u8, 255][..]).into(),
            "binary" => results::ok(results::Raw([0u8, 255])).into(),
            _ => results::not_found("unknown collection").into(),
        }
    }

    #[get("/complete/model/{format}")]
    pub fn complete_model_format(&self, format: FromRoute<String>) -> Box<dyn ActionResult> {
        let model = Item { name: "A&B" };
        match format.as_str() {
            "json" => results::created(results::Json(model)).into(),
            "xml" => results::created(results::Xml(model)).into(),
            _ => results::not_found("unknown format").into(),
        }
    }

    #[get("/complete/text")]
    pub async fn complete_text(&mut self) -> impl ActionResult {
        self.http_context.response.headers.insert_str("Content-Type", "text/html");
        results::Ok(TEXT_BODY)
    }

    #[get("/complete/borrowed")]
    pub fn complete_borrowed(&self, id: FromQuery<u32>) -> Box<dyn ActionResult + '_> {
        if *id != 42 {
            return results::BadRequest("invalid id").into();
        }
        BorrowedText(self.http_context.request.uri.path()).into()
    }

    #[get("/stream/result")]
    pub fn stream_result(&self) -> Box<dyn ActionResult + '_> {
        StreamedText(self.http_context.request.uri.path()).into()
    }

    #[get("/complete/string")]
    pub async fn complete_string(&mut self) -> impl ActionResult {
        results::ok("{\"ok\":true}".to_string())
    }

    #[get("/complete/large")]
    pub async fn complete_large(&mut self) -> impl ActionResult {
        results::ok(results::Raw(large_body()))
    }

    #[get("/complete/compressible")]
    pub async fn complete_compressible(&mut self) -> impl ActionResult {
        results::ok("a".repeat(10_000))
    }

    #[get("/complete/empty")]
    pub async fn complete_empty(&mut self) -> impl ActionResult {
        results::ok("")
    }

    #[get("/complete/no_content")]
    pub async fn complete_no_content(&mut self) -> impl ActionResult {
        results::NoContent()
    }

    #[get("/complete/status/{status}")]
    pub fn complete_status(&self, status: FromRoute<u16>) -> Box<dyn ActionResult> {
        let location = "/complete/text";
        match *status {
            201 => {
                let mut result = results::created("created").with_location(location).unwrap();
                let headers = result.headers_mut();
                headers.insert_str("Cache-Control", "no-store");
                headers.append_str("Set-Cookie", "a=1");
                headers.append_str("Set-Cookie", "b=2");
                result.into()
            }
            301 => results::moved_permanently(location).unwrap().into(),
            302 => results::Redirect(location).unwrap().into(),
            303 => results::see_other(location).unwrap().into(),
            304 => results::not_modified().into(),
            307 => results::temporary_redirect(location).unwrap().into(),
            308 => results::permanent_redirect(location).unwrap().into(),
            _ => results::bad_request("unsupported status").into(),
        }
    }

    #[route(["get", "head"], "/complete/head")]
    pub async fn complete_head(&mut self) -> impl ActionResult {
        results::ok(TEXT_BODY)
    }

    // ----- streamed responses -----

    #[get("/stream/chunks")]
    pub async fn stream_chunks(&mut self) -> impl ActionResult {
        self.http_context.response.headers.insert_str("Content-Type", "text/plain");
        for part in ["one\n", "two\n", "three\n"] {
            self.http_context.response.write_async(part).await.unwrap();
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        results::ok("")
    }

    #[get("/stream/status")]
    pub async fn stream_status(&mut self) -> impl ActionResult {
        self.http_context.response.status_code = http::StatusCode::CREATED;
        self.http_context.response.headers.insert_str("X-Test", "yes");
        self.http_context.response.write_bytes_async("created".to_string()).await.unwrap();
        results::ok("")
    }

    #[get("/stream/mixed")]
    pub async fn stream_mixed(&mut self) -> impl ActionResult {
        self.http_context.response.write_async(b"prefix-").await.unwrap();
        results::ok("tail")
    }

    #[get("/stream/mixed_large")]
    pub async fn stream_mixed_large(&mut self) -> impl ActionResult {
        self.http_context.response.write_async(b"prefix-").await.unwrap();
        results::ok("a".repeat(10_000))
    }

    #[get("/stream/headers_only")]
    pub async fn stream_headers_only(&mut self) -> impl ActionResult {
        self.http_context.response.headers.insert_str("X-Early", "1");
        self.http_context.response.start_async().await.unwrap();
        tokio::time::sleep(Duration::from_millis(300)).await;
        self.http_context.response.write_async("late").await.unwrap();
        results::ok("")
    }

    #[get("/stream/has_started")]
    pub async fn stream_has_started(&mut self) -> impl ActionResult {
        let before = self.http_context.response.has_started();
        self.http_context.response.write_async("x").await.unwrap();
        let after = self.http_context.response.has_started();
        self.http_context.response.write_async(format!("{before},{after}")).await.unwrap();
        results::ok("")
    }

    #[get("/stream/complete")]
    pub async fn stream_complete(&mut self) -> impl ActionResult {
        self.http_context.response.write_async("a").await.unwrap();
        self.http_context.response.body = b"b".to_vec();
        self.http_context.response.complete_async().await.unwrap();
        // the client must not wait for this
        tokio::time::sleep(Duration::from_millis(500)).await;
        results::ok("")
    }

    #[get("/stream/abort")]
    pub async fn stream_abort(&mut self) -> impl ActionResult {
        self.http_context.response.write_async("partial").await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        self.http_context.abort();
        results::ok("")
    }

    #[get("/stream/abort_before")]
    pub async fn stream_abort_before(&mut self) -> impl ActionResult {
        self.http_context.abort();
        results::ok("still complete")
    }

    #[get("/stream/disconnect")]
    pub async fn stream_disconnect(&mut self) -> impl ActionResult {
        loop {
            if self.http_context.response.write_async([b'x'; 1024]).await.is_err() {
                DISCONNECT_SEEN.store(true, Ordering::SeqCst);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        results::ok("")
    }

    #[get("/stream/backpressure")]
    pub async fn stream_backpressure(&mut self) -> impl ActionResult {
        let chunk = vec![7u8; BACKPRESSURE_CHUNK_SIZE];
        for _ in 0..BACKPRESSURE_CHUNKS {
            if self.http_context.response.write_bytes_async(chunk.clone()).await.is_err() {
                break;
            }
            PRODUCED_CHUNKS.fetch_add(1, Ordering::SeqCst);
        }
        results::ok("")
    }

    #[get("/stream/panic_before")]
    pub async fn stream_panic_before(&mut self) -> impl ActionResult {
        if std::hint::black_box(true) {
            panic!("test panic before any write");
        }
        results::ok("")
    }

    #[get("/stream/panic_json")]
    pub async fn stream_panic_json(&mut self) -> impl ActionResult {
        self.http_context.response.headers.insert_str("Content-Type", "application/json");
        if std::hint::black_box(true) {
            panic!("test panic after a header");
        }
        results::ok("")
    }

    #[get("/stream/panic_after")]
    pub async fn stream_panic_after(&mut self) -> impl ActionResult {
        self.http_context.response.write_async("partial").await.unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        if std::hint::black_box(true) {
            panic!("test panic after the first write");
        }
        results::ok("")
    }
}

// ---------------------------------------------------------------------------
// Server and client helpers
// ---------------------------------------------------------------------------

/// Starts the application on a free loopback port, `configure` adds the middlewares.
async fn start_server(configure: impl FnOnce(&mut Application)) -> u16 {
    LOGGER::disable_default_logger();
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let mut builder = ApplicationBuilder::new("StreamResponseTest");
    builder.with_loopback_ip().with_http_port(port);
    builder.add_controllers();
    builder.configuration.configure_default_entry::<CompressionConfiguration>("compress_middleware", NotAllowed);
    let mut app = builder.build();
    configure(&mut app);
    tokio::spawn(app.run());
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).await.is_ok() {
            return port;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("server did not start on port {port}");
}

struct Head {
    status: u16,
    headers: Vec<(String, String)>,
}
impl Head {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(key, _)| key.eq_ignore_ascii_case(name)).map(|(_, value)| value.as_str())
    }
    fn is_chunked(&self) -> bool {
        self.header("transfer-encoding").is_some_and(|value| value.eq_ignore_ascii_case("chunked"))
    }
}

/// A raw HTTP/1.1 connection.
struct Conn {
    reader: BufReader<TcpStream>,
}
impl Conn {
    async fn connect(port: u16) -> Self {
        Self {
            reader: BufReader::new(TcpStream::connect(("127.0.0.1", port)).await.unwrap()),
        }
    }

    async fn send(&mut self, method: &str, path: &str, extra_headers: &[(&str, &str)]) {
        let mut request = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n");
        for (key, value) in extra_headers {
            request.push_str(&format!("{key}: {value}\r\n"));
        }
        request.push_str("\r\n");
        self.reader.get_mut().write_all(request.as_bytes()).await.unwrap();
    }

    async fn read_line(&mut self) -> io::Result<String> {
        let mut line = String::new();
        if self.reader.read_line(&mut line).await? == 0 {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        Ok(line.trim_end_matches(['\r', '\n']).to_string())
    }

    async fn read_head(&mut self) -> Head {
        let status_line = self.read_line().await.unwrap();
        let status = status_line.split(' ').nth(1).unwrap().parse().unwrap();
        let mut headers = Vec::new();
        loop {
            let line = self.read_line().await.unwrap();
            if line.is_empty() {
                break;
            }
            let (key, value) = line.split_once(':').unwrap();
            headers.push((key.trim().to_string(), value.trim().to_string()));
        }
        Head { status, headers }
    }

    /// Reads one chunk of a chunked body, `Ok(None)` is the proper end of the body, `Err` is a broken stream.
    async fn read_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        let size_line = self.read_line().await?;
        let size = usize::from_str_radix(size_line.split(';').next().unwrap().trim(), 16).map_err(io::Error::other)?;
        if size == 0 {
            // trailers, then the empty line
            while !self.read_line().await?.is_empty() {}
            return Ok(None);
        }
        let mut chunk = vec![0u8; size];
        self.reader.read_exact(&mut chunk).await?;
        if !self.read_line().await?.is_empty() {
            return Err(io::Error::other("missing CRLF after the chunk"));
        }
        Ok(Some(chunk))
    }

    /// Reads the whole body, by chunks or by Content-Length.
    async fn read_body(&mut self, head: &Head) -> io::Result<Vec<u8>> {
        let mut body = Vec::new();
        if head.is_chunked() {
            while let Some(chunk) = self.read_chunk().await? {
                body.extend_from_slice(&chunk);
            }
        } else if let Some(length) = head.header("content-length") {
            body.resize(length.parse().unwrap(), 0);
            self.reader.read_exact(&mut body).await?;
        }
        Ok(body)
    }

    /// Request and the whole response.
    async fn get(&mut self, path: &str, extra_headers: &[(&str, &str)]) -> (Head, Vec<u8>) {
        self.send("GET", path, extra_headers).await;
        let head = self.read_head().await;
        let body = self.read_body(&head).await.unwrap();
        (head, body)
    }
}

async fn gunzip(data: &[u8]) -> Vec<u8> {
    let mut decoder = async_compression::tokio::bufread::GzipDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out).await.unwrap();
    out
}

// ---------------------------------------------------------------------------
// Complete responses
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn negotiates_the_model_from_accept() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    let (head, body) = conn.get("/complete/collection/items", &[]).await;
    assert_eq!(head.header("content-type"), Some("application/json"));
    assert_eq!(body, br#"[{"name":"A&B"}]"#);
    let (head, body) = conn.get("/complete/collection/items", &[("Accept", "application/xml")]).await;
    assert_eq!(head.header("content-type"), Some("application/xml"));
    assert_eq!(body, b"<response><value><name>A&amp;B</name></value></response>");
    for headers in [vec![], vec![("Accept", "*/*")], vec![("Content-Type", "application/xml")]] {
        let (head, body) = conn.get("/complete/model", &headers).await;
        assert_eq!(head.status, 201);
        assert_eq!(head.header("location"), Some("/items/1"));
        assert_eq!(head.header("vary"), Some("Accept"));
        assert_eq!(head.header("content-type"), Some("application/json"));
        assert_eq!(body, br#"{"name":"A&B"}"#);
    }
    for headers in [
        vec![("Accept", "application/json;q=0.2, application/xml;q=0.9")],
        vec![("Accept", "application/json;q=0"), ("Accept", "application/*")],
    ] {
        let (head, body) = conn.get("/complete/model", &headers).await;
        assert_eq!(head.status, 201);
        assert_eq!(head.header("content-type"), Some("application/xml"));
        assert_eq!(body, b"<response><value><name>A&amp;B</name></value></response>");
    }
    for accept in ["text/html", "application/*;q=0, */*;q=1"] {
        let (head, body) = conn.get("/complete/model", &[("Accept", accept)]).await;
        assert_eq!(head.status, 406);
        assert_eq!(head.header("vary"), Some("Accept"));
        assert!(head.header("content-type").is_none());
        assert!(head.header("location").is_none());
        assert!(body.is_empty());
    }
    for (path, content_type, expected) in [
        ("/complete/model/json", "application/json", br#"{"name":"A&B"}"#.as_slice()),
        ("/complete/model/xml", "application/xml", b"<response><value><name>A&amp;B</name></value></response>".as_slice()),
    ] {
        let (head, body) = conn.get(path, &[("Accept", "text/html")]).await;
        assert_eq!(head.status, 201);
        assert_eq!(head.header("content-type"), Some(content_type));
        assert!(head.header("vary").is_none());
        assert_eq!(body, expected);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn byte_collections_are_negotiated_until_wrapped_in_raw() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    for kind in ["vec", "array", "slice"] {
        let path = format!("/complete/collection/{kind}");
        let (head, body) = conn.get(&path, &[]).await;
        assert_eq!(head.header("content-type"), Some("application/json"));
        assert_eq!(body, b"[0,255]");
        let (head, body) = conn.get(&path, &[("Accept", "application/xml")]).await;
        assert_eq!(head.header("content-type"), Some("application/xml"));
        assert_eq!(body, b"<response><value>0</value><value>255</value></response>");
    }
    let (head, body) = conn.get("/complete/collection/binary", &[("Accept", "application/json")]).await;
    assert_eq!(head.header("content-type"), Some("application/octet-stream"));
    assert_eq!(body, [0, 255]);
    let (head, body) = conn.get("/complete/collection/missing", &[("Accept", "application/xml")]).await;
    assert_eq!(head.status, 404);
    assert_eq!(head.header("content-type"), Some("text/plain"));
    assert_eq!(body, b"unknown collection");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn complete_text_response_is_written_with_content_length() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/complete/text", &[]).await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-type"), Some("text/html"));
    assert_eq!(head.header("content-length"), Some(TEXT_BODY.len().to_string().as_str()));
    assert_eq!(head.header("server"), Some("ASP.RS"));
    assert!(head.header("connection").is_none(), "hyper manages the connection header");
    assert!(!head.is_chunked());
    assert_eq!(body, TEXT_BODY.as_bytes());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn routing_writes_borrowed_results_and_binding_errors() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;

    let (head, body) = conn.get("/complete/borrowed?id=42", &[]).await;
    assert_eq!(head.status, 201);
    assert_eq!(head.header("content-type"), Some("text/plain; charset=utf-8"));
    assert_eq!(body, b"/complete/borrowed");

    let (head, body) = conn.get("/complete/borrowed?id=0", &[]).await;
    assert_eq!(head.status, 400);
    assert_eq!(body, b"invalid id");

    let (head, body) = conn.get("/complete/borrowed", &[]).await;
    assert_eq!(head.status, 422);
    assert_eq!(head.header("content-type"), Some("text/plain"));
    assert_eq!(body, b"missing query parameter `id`");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn created_and_redirect_results_send_their_location_header() {
    assert!(results::redirect("/target\r\nX-Test: injected").is_err());
    assert!(results::created("created").with_location("/target\r\nX-Test: injected").is_err());

    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    for status in [201, 301, 302, 303, 304, 307, 308] {
        let (head, body) = conn.get(&format!("/complete/status/{status}"), &[]).await;
        assert_eq!(head.status, status);
        if status == 304 {
            assert!(head.header("location").is_none());
            assert!(head.header("content-length").is_none());
        } else {
            assert_eq!(head.header("location"), Some("/complete/text"));
        }
        let expected_body: &[u8] = if status == 201 { b"created" } else { b"" };
        assert_eq!(body, expected_body);
        if status == 201 {
            assert_eq!(head.header("cache-control"), Some("no-store"));
            let cookies: Vec<_> = head.headers.iter().filter(|(key, _)| key.eq_ignore_ascii_case("set-cookie")).map(|(_, value)| value.as_str()).collect();
            assert_eq!(cookies, ["a=1", "b=2"]);
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn complete_large_body_is_written_in_full() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/complete/large", &[]).await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-length"), Some(LARGE_BODY_SIZE.to_string().as_str()));
    assert_eq!(body.len(), LARGE_BODY_SIZE);
    assert!(body == large_body(), "the body content is different");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn complete_responses_keep_the_connection_alive() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    let (head, body) = conn.get("/complete/text", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, TEXT_BODY.as_bytes()));
    let (head, body) = conn.get("/complete/string", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, "{\"ok\":true}".as_bytes()));
    let (head, body) = conn.get("/complete/empty", &[]).await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-length"), Some("0"));
    assert!(body.is_empty());
    let (head, body) = conn.get("/complete/large", &[]).await;
    assert_eq!((head.status, body.len()), (200, LARGE_BODY_SIZE));
    let (head, _) = conn.get("/not_found_route", &[]).await;
    assert_eq!(head.status, 404);
    let (head, body) = conn.get("/complete/text", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, TEXT_BODY.as_bytes()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn no_content_response_has_no_body_and_keeps_the_connection_alive() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    let (head, body) = conn.get("/complete/no_content", &[]).await;
    assert_eq!(head.status, 204);
    assert!(head.header("content-length").is_none(), "204 must not have Content-Length");
    assert!(!head.is_chunked());
    assert!(body.is_empty());
    let (head, body) = conn.get("/complete/text", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, TEXT_BODY.as_bytes()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn head_request_gets_the_headers_without_body() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("HEAD", "/complete/head", &[]).await;
    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-length"), Some(TEXT_BODY.len().to_string().as_str()));
    // no body bytes may follow, the next response must start right after the head
    let (head, body) = conn.get("/complete/text", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, TEXT_BODY.as_bytes()));
}

// ---------------------------------------------------------------------------
// Streamed responses
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn custom_action_result_streams_through_the_response() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("GET", "/stream/result", &[]).await;
    let head = tokio::time::timeout(Duration::from_secs(2), conn.read_head()).await.unwrap();
    assert_eq!(head.status, 202);
    assert_eq!(head.header("content-type"), Some("text/plain"));
    assert!(head.is_chunked());
    let chunk = tokio::time::timeout(Duration::from_secs(2), conn.read_chunk()).await.unwrap().unwrap().unwrap();
    assert_eq!(chunk, b"/stream/result");
    NEXT_RESULT_CHUNK.add_permits(1);
    assert_eq!(conn.read_chunk().await.unwrap().unwrap(), b"last");
    assert!(conn.read_chunk().await.unwrap().is_none());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn streamed_response_sends_chunks_when_they_are_written() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("GET", "/stream/chunks", &[]).await;
    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-type"), Some("text/plain"));
    assert!(head.is_chunked(), "a streamed response has no Content-Length");
    assert!(head.header("content-length").is_none());

    // the handler sleeps 300ms after each write, a buffered response would deliver all chunks at the same time
    let first = conn.read_chunk().await.unwrap().unwrap();
    let first_at = Instant::now();
    let second = conn.read_chunk().await.unwrap().unwrap();
    let second_at = Instant::now();
    let third = conn.read_chunk().await.unwrap().unwrap();
    assert!(conn.read_chunk().await.unwrap().is_none(), "the stream must end properly");
    assert_eq!(
        (first.as_slice(), second.as_slice(), third.as_slice()),
        (b"one\n".as_slice(), b"two\n".as_slice(), b"three\n".as_slice())
    );
    assert!(
        second_at - first_at >= Duration::from_millis(200),
        "the first chunk was held until the second one: {:?}",
        second_at - first_at
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn streamed_response_keeps_the_status_and_headers_set_before_the_first_write() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/stream/status", &[]).await;
    // the controller result (status 200) is returned after the head is sent, it must not change what the client got
    assert_eq!(head.status, 201);
    assert_eq!(head.header("x-test"), Some("yes"));
    assert_eq!(body, b"created");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn streamed_bytes_are_followed_by_the_returned_body() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/stream/mixed", &[]).await;
    assert_eq!(head.status, 200);
    assert_eq!(body, b"prefix-tail");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn headers_can_be_sent_before_the_body() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    let started = Instant::now();
    conn.send("GET", "/stream/headers_only", &[]).await;
    let head = conn.read_head().await;
    assert!(started.elapsed() < Duration::from_millis(250), "the head was held until the body: {:?}", started.elapsed());
    assert_eq!(head.header("x-early"), Some("1"));
    assert_eq!(conn.read_body(&head).await.unwrap(), b"late");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn writer_gets_an_error_when_the_client_disconnects() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("GET", "/stream/disconnect", &[]).await;
    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    assert!(conn.read_chunk().await.unwrap().is_some());
    drop(conn);
    for _ in 0..100 {
        if DISCONNECT_SEEN.load(Ordering::SeqCst) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("write_async never returned an error after the client disconnected");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn slow_client_makes_the_writer_wait() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("GET", "/stream/backpressure", &[]).await;
    // do not read: the socket buffers and the channel fill up, then write_bytes_async must wait
    tokio::time::sleep(Duration::from_millis(800)).await;
    let produced = PRODUCED_CHUNKS.load(Ordering::SeqCst);
    assert!(produced > 0, "the handler did not start");
    assert!(produced < BACKPRESSURE_CHUNKS, "the writer was not slowed down by the client: {produced} chunks written");

    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    let mut total = 0;
    while let Some(chunk) = conn.read_chunk().await.unwrap() {
        total += chunk.len();
    }
    assert_eq!(total, BACKPRESSURE_CHUNKS * BACKPRESSURE_CHUNK_SIZE);
    assert_eq!(PRODUCED_CHUNKS.load(Ordering::SeqCst), BACKPRESSURE_CHUNKS);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn has_started_is_true_after_the_first_write() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/stream/has_started", &[]).await;
    assert_eq!(head.status, 200);
    assert_eq!(body, b"xfalse,true");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn complete_async_ends_the_stream_without_waiting_for_the_handler() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    let started = Instant::now();
    conn.send("GET", "/stream/complete", &[]).await;
    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    assert_eq!(conn.read_body(&head).await.unwrap(), b"ab");
    assert!(started.elapsed() < Duration::from_millis(400), "the stream ended with the handler: {:?}", started.elapsed());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn abort_after_the_first_write_breaks_the_stream() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("GET", "/stream/abort", &[]).await;
    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    assert_eq!(conn.read_chunk().await.unwrap().unwrap(), b"partial");
    assert!(conn.read_chunk().await.is_err(), "the stream ended as if it was complete");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn abort_before_the_first_write_does_nothing() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/stream/abort_before", &[]).await;
    assert_eq!(head.status, 200);
    assert_eq!(body, b"still complete");
}

// ---------------------------------------------------------------------------
// Panics
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn panic_before_any_write_returns_500() {
    let mut logs = LOGGER::spawn_log_receiver();
    let port = start_server(|app| {
        app.use_compression();
    })
    .await;
    let mut conn = Conn::connect(port).await;
    let client_addr = conn.reader.get_ref().local_addr().unwrap();
    let (head, body) = conn.get("/stream/panic_before", &[]).await;
    assert_eq!(head.status, 500);
    assert_eq!(body, b"Internal Server Error");

    // The connection is still usable after a panic.
    let (head, body) = conn.get("/complete/text", &[]).await;
    assert_eq!((head.status, body.as_slice()), (200, TEXT_BODY.as_bytes()));

    let followup_access_log = format!("{client_addr} GET HTTP/1.1 ");
    println!("--- LOGGER BEGIN ---");
    let panic_log = tokio::time::timeout(Duration::from_secs(5), async {
        let mut panic_log = None;
        loop {
            if let LogCommand::Log(log) = logs.recv().await.unwrap() {
                println!("[{}] {} {}", &log.level.as_str()[..3], log.timestamp.format("%Y-%m-%d %H:%M:%S"), log.message);
                if log.message.contains("test panic before any write") {
                    panic_log = Some(log);
                } else if log.message.starts_with(&followup_access_log) && log.message.split_ascii_whitespace().any(|part| part == "/complete/text") {
                    break panic_log;
                }
            }
        }
    })
    .await
    .expect("follow-up request was not logged")
    .expect("request panic was not logged");
    println!("--- LOGGER END ---");
    assert_eq!(panic_log.level, LogLevel::Error);
    let (location, backtrace) = panic_log.message.split_once(": test panic before any write\nBacktrace:\n").unwrap();
    let (request, origin) = location.split_once(", Unhandled panic at ").unwrap();
    let (connection_id, request_id) = request.strip_prefix("Request ").unwrap().split_once(':').unwrap();
    assert!(connection_id.parse::<ulid::Ulid>().is_ok(), "{connection_id}");
    assert!(request_id.parse::<ulid::Ulid>().is_ok(), "{request_id}");
    assert!(origin.starts_with(file!()), "{origin}");
    assert!(origin.contains(" (thread "), "{origin}");
    assert!(!backtrace.trim().is_empty(), "{panic_log:?}");
    if cfg!(debug_assertions) {
        let first_frame = backtrace.lines().next().unwrap();
        assert!(first_frame.trim_start().starts_with("0: ") && first_frame.contains("stream_panic_before"), "{backtrace}");
        assert!(backtrace.contains("::middleware::compression::"), "{backtrace}");
        assert!(backtrace.contains("::middleware::auto_route::"), "{backtrace}");
        assert!(!backtrace.contains("install_request_panic_hook"), "{backtrace}");
        assert!(!backtrace.contains("core::panicking::"), "{backtrace}");
        assert!(!backtrace.contains("core::future::future::"), "{backtrace}");
        assert!(!backtrace.lines().any(|line| line.contains("::handle_error::") && line.contains("::run::")), "{backtrace}");
        assert!(!backtrace.contains("hyper::"), "{backtrace}");
        assert!(!backtrace.contains("tokio::runtime"), "{backtrace}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn panic_while_creating_a_middleware_future_returns_500() {
    struct PanicBeforeFuture;

    #[async_trait::async_trait]
    impl asp_dot_rust::dependency_injection::DependencyInjectableMiddleware for PanicBeforeFuture {
        async fn inject(_: &ServiceProviderScope, _: MiddlewareNext) -> Self {
            Self
        }
    }

    #[async_trait::async_trait]
    impl Middleware for PanicBeforeFuture {
        async fn inject_invoke_async(&self, _: &mut HttpContext) {
            panic!("test panic before creating a middleware future");
        }
    }

    let port = start_server(|app| {
        app.add_middleware::<PanicBeforeFuture>();
    })
    .await;
    let (head, body) = Conn::connect(port).await.get("/complete/text", &[]).await;
    assert_eq!(head.status, 500);
    assert_eq!(body, b"Internal Server Error");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn panic_response_does_not_keep_the_content_type_of_the_handler() {
    let port = start_server(|_| {}).await;
    let (head, body) = Conn::connect(port).await.get("/stream/panic_json", &[]).await;
    assert_eq!(head.status, 500);
    assert_eq!(head.header("content-type"), Some("text/plain; charset=utf-8"));
    assert_eq!(body, b"Internal Server Error");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn panic_after_the_first_write_breaks_the_stream() {
    let port = start_server(|_| {}).await;
    let mut conn = Conn::connect(port).await;
    conn.send("GET", "/stream/panic_after", &[]).await;
    let head = conn.read_head().await;
    assert_eq!(head.status, 200);
    assert_eq!(conn.read_chunk().await.unwrap().unwrap(), b"partial");
    // a clean end (`Ok(None)`) would make the client believe the body is complete
    assert!(conn.read_chunk().await.is_err(), "the stream ended as if it was complete");
}

// ---------------------------------------------------------------------------
// Compression middleware
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn compression_still_compresses_a_complete_response() {
    let port = start_server(|app| {
        app.use_compression();
    })
    .await;
    let (head, body) = Conn::connect(port).await.get("/complete/compressible", &[("Accept-Encoding", "gzip")]).await;
    assert_eq!(head.status, 200);
    assert_eq!(head.header("content-encoding"), Some("gzip"));
    assert_eq!(head.header("content-length"), Some(body.len().to_string().as_str()));
    assert!(body.len() < 10_000);
    assert_eq!(gunzip(&body).await, "a".repeat(10_000).as_bytes());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn compression_does_not_touch_a_response_that_is_already_streamed() {
    let port = start_server(|app| {
        app.use_compression();
    })
    .await;
    let (head, body) = Conn::connect(port).await.get("/stream/mixed_large", &[("Accept-Encoding", "gzip")]).await;
    assert_eq!(head.status, 200);
    assert!(head.header("content-encoding").is_none());
    assert_eq!(body, format!("prefix-{}", "a".repeat(10_000)).as_bytes());
}

// ---------------------------------------------------------------------------
// HTTP/2
// ---------------------------------------------------------------------------

async fn h2_get(port: u16, path: &str) -> hyper::Response<hyper::body::Incoming> {
    let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let (mut sender, connection) = hyper::client::conn::http2::handshake(hyper_util::rt::TokioExecutor::new(), hyper_util::rt::TokioIo::new(stream))
        .await
        .unwrap();
    tokio::spawn(connection);
    let request = hyper::Request::get(format!("http://127.0.0.1:{port}{path}")).body(Empty::<Bytes>::new()).unwrap();
    sender.send_request(request).await.unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http2_streamed_response_sends_chunks_when_they_are_written() {
    let port = start_server(|_| {}).await;
    let response = h2_get(port, "/stream/chunks").await;
    assert_eq!(response.status(), 200);
    let mut body = response.into_body();
    let first = body.frame().await.unwrap().unwrap().into_data().unwrap();
    let first_at = Instant::now();
    let second = body.frame().await.unwrap().unwrap().into_data().unwrap();
    let second_at = Instant::now();
    let third = body.frame().await.unwrap().unwrap().into_data().unwrap();
    // the end of the stream may come as an empty data frame, anything else is a failure
    while let Some(frame) = body.frame().await {
        let frame = frame.expect("the stream must end properly");
        assert!(frame.into_data().is_ok_and(|data| data.is_empty()), "unexpected frame after the last chunk");
    }
    assert_eq!((&first[..], &second[..], &third[..]), (&b"one\n"[..], &b"two\n"[..], &b"three\n"[..]));
    assert!(
        second_at - first_at >= Duration::from_millis(200),
        "the first chunk was held until the second one: {:?}",
        second_at - first_at
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http2_complete_response_is_written_in_full() {
    let port = start_server(|_| {}).await;
    let response = h2_get(port, "/complete/large").await;
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers().get("content-length").unwrap(), LARGE_BODY_SIZE.to_string().as_str());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    assert!(body[..] == large_body()[..], "the body content is different");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn http2_panic_after_the_first_write_breaks_the_stream() {
    let port = start_server(|_| {}).await;
    let response = h2_get(port, "/stream/panic_after").await;
    assert_eq!(response.status(), 200);
    // a clean end would make the client believe the body is complete
    assert!(response.into_body().collect().await.is_err(), "the stream ended as if it was complete");
}
