use crate::{Application, configuration::RequestTimeoutConfiguration, dependcy_injection::CfgRequire, http_context::HttpContext, logging::LOGGER, macros::inject, middleware::Middleware};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RequestTimeoutMiddleware {
    request_timeout_config: Arc<RequestTimeoutConfiguration>,
}
#[inject]
impl RequestTimeoutMiddleware {
    fn new(request_timeout_cfg: CfgRequire<RequestTimeoutConfiguration>) -> RequestTimeoutMiddleware {
        Self {
            request_timeout_config: request_timeout_cfg.unwrap(),
        }
    }
}
#[async_trait::async_trait]
impl Middleware for RequestTimeoutMiddleware {
    async fn invoke_async(&self, http_context: &mut HttpContext, next: crate::middleware::MiddlewareNext) {
        LOGGER::debug("RequestTimeoutMiddleware: Checking request timeout");
        let timeout = std::time::Duration::from_secs(self.request_timeout_config.timeout_seconds);
        if let Err(_) = tokio::time::timeout(timeout, next(http_context)).await {
            LOGGER::warn("Request timed out");
            http_context.response.status_code = http::StatusCode::REQUEST_TIMEOUT;
            http_context
                .response
                .write_async(http::StatusCode::REQUEST_TIMEOUT.canonical_reason().unwrap_or("Request Timeout").as_bytes())
                .await;
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
