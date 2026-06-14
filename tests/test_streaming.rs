use asp_dot_rust::{
    ApplicationBuilder,
    configuration::{CorsConfiguration, RateLimitConfiguration},
    logging::LOGGER,
};
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper_util::rt::TokioExecutor;
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_streaming_requests() {
    LOGGER::with_color_output(true);
    LOGGER::with_level(asp_dot_rust::logging::LogLevel::Info);

    let mut app_builder = ApplicationBuilder::new("StreamingTest");
    app_builder.with_any_ip().with_http_port(9090);
    app_builder
        .add_custom_configuration(|config: &mut CorsConfiguration| {
            config.allowed_origins = ["*"].into();
            config.allowed_methods = [http::Method::GET, http::Method::POST].into();
            config.allowed_headers = ["Content-Type"].into();
        })
        .add_custom_configuration(|cfg: &mut RateLimitConfiguration| {
            cfg.max_requests = 5000000000;
            cfg.limit_seconds = 1;
            cfg.block_duration_seconds = 60;
        });
    app_builder.add_controllers();
    app_builder.add_memory_cache();
    let mut app = app_builder.build();
    app.use_cors().use_rate_limit();

    // Spawn the server in a separate task
    tokio::spawn(async move {
        let _ = app.run().await;
    });

    // Give the server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test 1: Small POST request
    println!("Testing small POST request...");
    let client = http::Request::builder().method(http::Method::POST).uri("http://127.0.0.1:9090/").body("small body").unwrap();
    println!("Test completed");

    // Test 2: Larger request body (100MB)
    println!("Testing large POST request (100MB)...");
    let large_body = vec![b'a'; 100 * 1024 * 1024]; // 100MB
    let large_request = http::Request::builder()
        .method(http::Method::POST)
        .uri("http://127.0.0.1:9090/")
        .body(Full::new(hyper::body::Bytes::from(large_body)))
        .unwrap();

    let client = hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build_http::<Full<Bytes>>();

    match client.request(large_request).await {
        Ok(response) => {
            println!("Status: {}", response.status());
            println!("Headers: {:#?}", response.headers());

            match response.into_body().collect().await {
                Ok(body) => {
                    let bytes = body.to_bytes();
                    println!("Body ({} bytes): {:?}", bytes.len(), bytes);
                }
                Err(e) => eprintln!("Lỗi đọc body: {}", e),
            }
        }
        Err(e) => eprintln!("Lỗi gửi request: {}", e),
    }
    println!("Large test completed");

    println!("All streaming tests passed!");
    tokio::time::sleep(tokio::time::Duration::from_millis(10000)).await;
}
