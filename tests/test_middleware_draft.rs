//! Behavior of the middleware chain in the style of ASP.NET: order, stop and wrappers.
use asp_dot_rust::{
    ApplicationBuilder, api_controller,
    controller::{ActionResult, get},
    http_context::{HttpContext, HttpContextRef},
    logging::LOGGER,
    macros::{controller_inject, controller_route},
    middleware::{Middleware, MiddlewareNext},
};
use std::{
    io::{Read, Write},
    net::TcpStream,
    sync::Mutex,
    time::Duration,
};

static EVENTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
fn event(text: &str) {
    EVENTS.lock().unwrap().push(text.to_string());
}

asp_dot_rust::middleware!(pub Outer, |ctx, next| {
    event("outer.before");
    next.next_async(ctx).await;
    event("outer.after");
});

// shape Before: next is the last action, `return;` stops the request
asp_dot_rust::middleware!(pub Guard, |ctx, next| {
    if ctx.request.path() == "/blocked" {
        event("guard.stop");
        ctx.response.status_code = http::StatusCode::FORBIDDEN;
        return;
    }
    event("guard.pass");
    next.next_async(ctx).await;
});

// shape Split with an async part
asp_dot_rust::middleware!(pub Slow, |ctx, next| {
    tokio::time::sleep(Duration::from_millis(1)).await;
    event("slow.before");
    next.next_async(ctx).await;
    tokio::time::sleep(Duration::from_millis(1)).await;
    event("slow.after");
});

// shape Wrap: `next` is not a statement
asp_dot_rust::middleware!(pub Wrapper, |ctx, next| {
    event("wrapper.before");
    let future = next.next_async(ctx);
    future.await;
    event("wrapper.after");
});

// shape Split where a local is shared, becomes a wrapper
asp_dot_rust::middleware!(pub Shared, |ctx, next| {
    let started = std::time::Instant::now();
    next.next_async(ctx).await;
    event(if started.elapsed().as_secs() < 5 { "shared.fast" } else { "shared.slow" });
});

api_controller!(pub HookController {});
#[controller_route]
#[controller_inject]
impl HookController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }
    #[get("/ok")]
    pub async fn ok(&mut self) -> impl ActionResult {
        event("handler");
        "ok"
    }
    #[get("/blocked")]
    pub async fn blocked(&mut self) -> impl ActionResult {
        event("handler");
        "blocked"
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
async fn draft_runs_in_order_and_stop() {
    LOGGER::disable();
    let port = 18935;
    let mut builder = ApplicationBuilder::new("draft");
    builder.with_loopback_ip().with_http_port(port);
    builder.add_controllers();
    let mut app = builder.build();
    app.add_middleware::<Outer>()
        .add_middleware::<Guard>()
        .add_middleware::<Slow>()
        .add_middleware::<Wrapper>()
        .add_middleware::<Shared>();
    tokio::spawn(app.run());
    for _ in 0..100 {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    EVENTS.lock().unwrap().clear();
    let response = tokio::task::spawn_blocking(move || request(port, "/ok")).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
    assert_eq!(
        *EVENTS.lock().unwrap(),
        [
            "outer.before",
            "guard.pass",
            "slow.before",
            "wrapper.before",
            "handler",
            "shared.fast",
            "wrapper.after",
            "slow.after",
            "outer.after"
        ]
    );

    // Guard stops: later middlewares and the handler do not run, only the outer `after` runs
    EVENTS.lock().unwrap().clear();
    let response = tokio::task::spawn_blocking(move || request(port, "/blocked")).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 403"), "{response}");
    assert_eq!(*EVENTS.lock().unwrap(), ["outer.before", "guard.stop", "outer.after"]);
}

// the old signature still compiles when `next` is renamed and unused types are imported
struct Unused;
#[allow(dead_code)]
fn _signature_check(_: MiddlewareNext, _: &mut HttpContext) {
    let _ = Unused;
    fn assert_middleware<T: Middleware>() {}
    assert_middleware::<Outer>();
}
