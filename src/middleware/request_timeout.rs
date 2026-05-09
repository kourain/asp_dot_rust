use crate::{configuration::RequestTimeoutConfiguration, http_context::HttpContext, logging::LOGGER, middleware::Middleware};

#[derive(Debug, Clone)]
pub struct RequestTimeoutMiddleware {
    timeout_seconds: u64,
}
impl Default for RequestTimeoutMiddleware {
    fn default() -> Self {
        Self {
            timeout_seconds: 30, // default to 30 seconds
        }
    }
}
#[async_trait::async_trait]
impl Middleware for RequestTimeoutMiddleware {
    fn with_application(&mut self, app: &crate::application::Application) {
        match app.try_get_configuration::<RequestTimeoutConfiguration>() {
            Some(config) => {
                self.timeout_seconds = config.timeout_seconds;
                LOGGER::info(format!("RequestTimeoutMiddleware configured with timeout_seconds: {}", self.timeout_seconds));
            }
            None => {
                LOGGER::warn(format!("RequestTimeoutConfiguration not found, using default timeout_seconds: {}", self.timeout_seconds));
            }
        };
    }
    async fn invoke_async<'a>(&self, http_context: &'a mut HttpContext, next: crate::middleware::MiddlewareNext) {
        LOGGER::debug("RequestTimeoutMiddleware: Checking request timeout");
        let timeout = std::time::Duration::from_secs(self.timeout_seconds);
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
