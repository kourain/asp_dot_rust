mod controllers;
use std::time::Duration;

use asp_dot_rust::{
    ApplicationBuilder,
    logging::LOGGER,
};

#[tokio::test(flavor = "multi_thread", worker_threads = 20)]
async fn test_application() {
    LOGGER::disable();
    let mut app_builder = ApplicationBuilder::new("TestApp");
    app_builder.with_http_port(8080);
    app_builder.add_controllers();
    let app = app_builder.build();
    if tokio::time::timeout(Duration::from_secs(5), app.run()).await.is_err() {
        assert!(true)
    }
}
