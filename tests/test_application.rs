mod controllers;
use asp_dot_rust::{
    ApplicationBuilder,
    configuration::{CorsConfiguration, RateLimitConfiguration},
    logging::LOGGER,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 16)]
async fn test_application() {
    LOGGER::with_color_output(true);
    LOGGER::with_level(asp_dot_rust::logging::LogLevel::None);
    // LOGGER::with_chrono_time_format("%Y-%m-%d %H:%M:%S%.9f");
    // LOGGER::with_request_id(true);
    let mut app_builder = ApplicationBuilder::new("TestApp");
    app_builder.with_any_ip().with_http_port(8080);
    app_builder
        .add_custom_configuration(|config: &mut CorsConfiguration| {
            config.allowed_origins = ["*".into()].into();
            config.allowed_methods = [http::Method::GET, http::Method::POST].into();
            config.allowed_headers = ["Content-Type".into()].into();
        })
        .add_custom_configuration::<RateLimitConfiguration>(|cfg| {
            cfg.max_requests = 5000000000;
            cfg.limit_seconds = 1;
            cfg.block_duration_seconds = 60;
        });
    app_builder.add_controllers();
    // app_builder.add_memory_cache();
    let mut app = app_builder.build();
    //app.use_cors();//.use_rate_limit();
    let _ = app.run().await;
}
