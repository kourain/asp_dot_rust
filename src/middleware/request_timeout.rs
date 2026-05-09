use crate::{http_context::HttpContext, logging::LOGGER, middleware::Middleware};

#[derive(Debug, Clone, Default)]
pub struct RequestTimeoutMiddleware {
    timeout_seconds: u64,
}
#[async_trait::async_trait]
impl Middleware for RequestTimeoutMiddleware {
    fn with_application(&mut self, _: &crate::application::Application) {}
    async fn invoke_async<'a>(&self, http_context: &'a mut HttpContext, next: crate::middleware::MiddlewareNext) {
        LOGGER::debug("RequestTimeoutMiddleware: Checking request timeout");
        let timeout = std::time::Duration::from_secs(self.timeout_seconds);
        let is_error = tokio::time::timeout(timeout, next(http_context)).await.is_err();
        if is_error {
            LOGGER::warn("Request timed out");
            http_context.response.status_code = http::StatusCode::REQUEST_TIMEOUT; // Request Timeout
            http_context.response.write_async(http::StatusCode::REQUEST_TIMEOUT.canonical_reason().unwrap_or("Request Timeout").as_bytes()).await;
        }
    }
}
