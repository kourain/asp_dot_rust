use std::sync::atomic::AtomicBool;

use asp_dot_rust::middleware::Middleware;
use asp_dot_rust_macros::inject_require;
use async_trait::async_trait;

pub struct TestMidware {
    test_state: AtomicBool,
}
#[inject_require]
impl TestMidware {
    fn new() -> Self {
        Self { test_state: AtomicBool::new(false) }
    }
}
#[async_trait]
impl Middleware for TestMidware {
    async fn invoke_async(&self, http_context: &mut asp_dot_rust::http_context::HttpContext, next: asp_dot_rust::middleware::MiddlewareNext) {
        next(http_context).await;
    }
}
