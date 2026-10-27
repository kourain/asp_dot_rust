use asp_dot_rust::{macros::middleware, middleware::MiddlewareNext};
use std::sync::atomic::AtomicUsize;

pub struct TestMidware {
    next: MiddlewareNext,
    test_state: AtomicUsize,
}
#[middleware]
impl TestMidware {
    fn new(next: MiddlewareNext) -> Self {
        Self {
            next,
            test_state: AtomicUsize::new(0),
        }
    }
    pub async fn invoke_async(&self, http_context: &mut asp_dot_rust::http_context::HttpContext) {
        self.test_state.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        self.next.next_async(http_context).await;
    }
}
