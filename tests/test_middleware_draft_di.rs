//! Middleware and controller dependency injection with startup and request arguments.
use asp_dot_rust::{
    ApplicationBuilder, api_controller,
    controller::{ActionResult, get},
    dependency_injection::{CfgOption, CfgReload, CfgRequire, Serv},
    http_context::{HttpContext, HttpContextRef},
    logging::LOGGER,
    macros::{controller_inject, controller_route, middleware},
    middleware::MiddlewareNext,
    services::routing::RoutingService,
};
use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering::SeqCst},
    },
    time::Duration,
};

static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static REQUESTS: AtomicUsize = AtomicUsize::new(0);

#[derive(Default)]
struct DiSettings {
    enabled: bool,
}

pub struct DiMiddleware {
    next: MiddlewareNext,
    startup: Serv<RoutingService>,
}
#[middleware]
impl DiMiddleware {
    fn new(config: CfgRequire<DiSettings>, next: MiddlewareNext, startup: Serv<RoutingService>) -> Self {
        assert!(config.enabled);
        EVENTS.lock().unwrap().push("new".to_string());
        Self { next, startup }
    }
    async fn invoke_async(&self, http_context: &mut HttpContext, request: Serv<RoutingService>, config: CfgReload<DiSettings>) {
        // both services are the same registered service, the second one is resolved on every request
        let methods = (self.startup.get_allowed_methods("/ok").len(), request.get_allowed_methods("/ok").len());
        assert_eq!(methods.0, methods.1);
        assert!(config.capture().unwrap().enabled);
        REQUESTS.fetch_add(1, SeqCst);
        self.next.next_async(http_context).await;
    }
}

api_controller!(pub DiController {
    routing: Serv<RoutingService>,
});
#[controller_route]
#[controller_inject]
impl DiController {
    fn new(http_context: HttpContextRef, routing: Serv<RoutingService>, config: CfgOption<DiSettings>) -> Self {
        assert!(config.unwrap().enabled);
        Self { http_context, routing }
    }
    #[get("/ok")]
    pub async fn ok(&mut self) -> impl ActionResult {
        assert!(self.routing.get_allowed_methods(self.http_context.request.path()).contains(&http::Method::GET));
        "ok"
    }
}

fn request(port: u16, path: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    write!(stream, "GET {path} HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n").unwrap();
    let mut out = String::new();
    let _ = stream.read_to_string(&mut out);
    out
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn startup_and_request_dependencies_are_injected() {
    LOGGER::disable();
    let port = 18936;
    let mut builder = ApplicationBuilder::new("draft_di");
    builder.with_loopback_ip().with_http_port(port);
    builder.configuration.insert::<DiSettings>(|config| config.enabled = true);
    builder.add_controllers();
    let mut app = builder.build();
    app.add_middleware::<DiMiddleware>();
    // the middleware is created once when the application runs, not when it is added
    assert!(EVENTS.lock().unwrap().is_empty());
    tokio::spawn(app.run());
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    for _ in 0..3 {
        let response = tokio::task::spawn_blocking(move || request(port, "/ok")).await.unwrap();
        assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    }
    assert_eq!(*EVENTS.lock().unwrap(), ["new"]);
    assert_eq!(REQUESTS.load(SeqCst), 3);
}
