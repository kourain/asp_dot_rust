use asp_dot_rust::{macros::inject_require, middleware::Middleware};
use async_trait::async_trait;
use std::sync::atomic::AtomicUsize;

pub struct TestMidware {
    test_state: AtomicUsize,
}
#[inject_require]
impl TestMidware {
    fn new() -> Self {
        Self { test_state: AtomicUsize::new(0) }
    }
}
#[async_trait]
impl Middleware for TestMidware {
    async fn invoke_async(&self, http_context: &mut asp_dot_rust::http_context::HttpContext, next: asp_dot_rust::middleware::MiddlewareNext) {
        self.test_state.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        next(http_context).await;
    }
}
