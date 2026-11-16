use asp_dot_rust::{ApplicationBuilder, configuration::RateLimitConfiguration, logging::LOGGER};
use std::{process::Command, time::{Duration, Instant}};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream};

static ENTERED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

asp_dot_rust::middleware!(pub ConcurrentHandler, |ctx, _next| {
    use std::sync::atomic::Ordering;
    ENTERED.fetch_add(1, Ordering::SeqCst);
    while ENTERED.load(Ordering::SeqCst) < 2 {
        tokio::task::yield_now().await;
    }
    ctx.response.status_code = http::StatusCode::OK;
});

#[test]
fn concurrent_same_ip_requests_complete_and_limit_is_enforced() {
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "rate_limit_child", "--nocapture"])
        .env("ASP_RATE_LIMIT_CHILD", "1")
        .spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "rate limiter regression child failed");
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("same-IP requests deadlocked the single-thread runtime");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

async fn request(port: u16) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    stream.write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response
}

#[test]
fn rate_limit_child() {
    if std::env::var_os("ASP_RATE_LIMIT_CHILD").is_none() { return; }
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    runtime.block_on(async {
        LOGGER::disable();
        let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
        let mut builder = ApplicationBuilder::new("rate-limit-regression");
        builder.with_loopback_ip().with_http_port(port).add_controllers();
        builder.configuration.insert::<RateLimitConfiguration>(|c| {
            c.max_requests = 2;
            c.limit_seconds = 60;
            c.block_duration_seconds = 60;
        });
        let mut app = builder.build();
        app.use_rate_limit().add_middleware::<ConcurrentHandler>();
        let server = tokio::spawn(app.run());
        loop {
            if TcpStream::connect(("127.0.0.1", port)).await.is_ok() { break; }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let (first, second) = tokio::join!(request(port), request(port));
        assert!(first.starts_with("HTTP/1.1 200"), "{first}");
        assert!(second.starts_with("HTTP/1.1 200"), "{second}");
        for _ in 0..2 {
            let rejected = request(port).await;
            assert!(rejected.starts_with("HTTP/1.1 429"), "{rejected}");
        }
        server.abort();
    });
}
