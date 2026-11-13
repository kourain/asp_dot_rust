mod controllers;
mod middleware;
use std::time::Duration;

use asp_dot_rust::{
    ApplicationBuilder,
    configuration::{CorsConfiguration, RateLimitConfiguration},
    logging::LOGGER,
};

use crate::middleware::TestMidware;

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_application() {
    LOGGER::disable();
    let mut app_builder = ApplicationBuilder::new("TestApp");
    app_builder.with_any_ip().with_http_port(8080).with_http_port(9999);
    app_builder
        .configuration.update(|config: &mut CorsConfiguration| {
            config.allowed_origins = ["*".into()].into();
            config.allowed_methods = [http::Method::GET, http::Method::POST].into();
            config.allowed_headers = ["Content-Type".into()].into();
        })
        .update(|config: &mut RateLimitConfiguration| {
            config.max_requests = 5000000000;
            config.limit_seconds = 1;
            config.block_duration_seconds = 60;
        });
    app_builder.add_controllers();
    // app_builder.add_memory_cache();
    let mut app = app_builder.build();
    app.add_middleware::<TestMidware>();
    // app.run().await;
    if tokio::time::timeout(Duration::from_secs(5), app.run()).await.is_err() {
        assert!(true)
    }
}
