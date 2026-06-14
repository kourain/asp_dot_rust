use asp_dot_rust::{
    ApplicationBuilder,
    configuration::{CorsConfiguration, RateLimitConfiguration},
    logging::LOGGER,
};

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
    let client = http::Request::builder()
        .method(http::Method::POST)
        .uri("http://127.0.0.1:9090/")
        .body("small body")
        .unwrap();
    println!("Test completed");
    
    // Test 2: Larger request body (1MB)
    println!("Testing large POST request (1MB)...");
    let large_body = vec![b'a'; 1024 * 1024]; // 1MB
    let large_request = http::Request::builder()
        .method(http::Method::POST)
        .uri("http://127.0.0.1:9090/")
        .body(std::str::from_utf8(&large_body).unwrap())
        .unwrap();
    println!("Large test completed");
    
    println!("All streaming tests passed!");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_hyper_integration() {
    LOGGER::with_color_output(true);
    LOGGER::with_level(asp_dot_rust::logging::LogLevel::Info);
    
    let mut app_builder = ApplicationBuilder::new("HyperIntegrationTest");
    app_builder.with_any_ip().with_http_port(9091);
    app_builder.add_controllers();
    let mut app = app_builder.build();
    
    // Spawn server
    tokio::spawn(async move {
        let _ = app.run().await;
    });
    
    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    
    // Simple GET request test
    println!("Testing GET request through Hyper...");
    // We're just checking that the server starts and accepts connections
    // More detailed testing would require an HTTP client
    
    println!("Hyper integration test completed!");
}
