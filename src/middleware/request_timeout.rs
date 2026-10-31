use crate::{Application, configuration::RequestTimeoutConfiguration, dependency_injection::CfgRequire, http_context::HttpContext, logging::LOGGER, macros::middleware, middleware::MiddlewareNext};
use std::sync::Arc;

pub struct RequestTimeoutMiddleware {
    next: MiddlewareNext,
    request_timeout_config: Arc<RequestTimeoutConfiguration>,
}
#[middleware]
impl RequestTimeoutMiddleware {
    fn new(next: MiddlewareNext, request_timeout_cfg: CfgRequire<RequestTimeoutConfiguration>) -> RequestTimeoutMiddleware {
        Self {
            next,
            request_timeout_config: request_timeout_cfg.unwrap(),
        }
    }
    async fn invoke_async(&self, http_context: &mut HttpContext) {
        LOGGER::debug("RequestTimeoutMiddleware: Checking request timeout");
        let timeout = std::time::Duration::from_secs(self.request_timeout_config.timeout_seconds);
        if tokio::time::timeout(timeout, self.next.next_async(http_context)).await.is_err() {
            LOGGER::warn("Request timed out");
            if http_context.response.is_committed() {
                // the status is already sent, the only way to tell the client is to break the stream
                http_context.response.abort();
            } else {
                http_context.response.status_code = http::StatusCode::REQUEST_TIMEOUT;
                http_context.response.body = http::StatusCode::REQUEST_TIMEOUT.canonical_reason().unwrap_or("Request Timeout").as_bytes().to_vec();
            }
        } else {
            // Request completed within the timeout
        }
    }
}
impl Application {
    pub fn use_request_timeout(&mut self) -> &mut Self {
        self.add_middleware::<RequestTimeoutMiddleware>();
        self
    }
}
