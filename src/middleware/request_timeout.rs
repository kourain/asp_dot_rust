use crate::{Application, configuration::RequestTimeoutConfiguration, dependency_injection::CfgRequire, http_context::HttpContext, logging::LOGGER, macros::middleware, middleware::MiddlewareNext};

pub struct RequestTimeoutMiddleware {
    next: MiddlewareNext,
    request_timeout_config: CfgRequire<RequestTimeoutConfiguration>,
}
#[middleware]
impl RequestTimeoutMiddleware {
    fn new(next: MiddlewareNext, request_timeout_cfg: CfgRequire<RequestTimeoutConfiguration>) -> RequestTimeoutMiddleware {
        Self {
            next,
            request_timeout_config: request_timeout_cfg,
        }
    }
    async fn invoke_async(&self, http_context: &mut HttpContext) {
        LOGGER::debug("RequestTimeoutMiddleware: Checking request timeout");
        let timeout = std::time::Duration::from_secs(self.request_timeout_config.timeout_seconds);
        let accepted = http_context.web_socket_handshake.as_ref().map(|handshake| handshake.accepted.clone());
        {
            let mut pipeline = self.next.next_async(http_context);
            if tokio::time::timeout(timeout, &mut pipeline).await.is_ok() {
                return;
            }
            if accepted.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Acquire)) {
                pipeline.await;
                return;
            }
        }
        LOGGER::warn("Request timed out");
        if http_context.response.has_started() {
            // the status is already sent, the only way to tell the client is to break the stream
            http_context.abort();
        } else {
            http_context.response.status_code = http::StatusCode::REQUEST_TIMEOUT;
            http_context.response.body = http::StatusCode::REQUEST_TIMEOUT.canonical_reason().unwrap_or("Request Timeout").as_bytes().to_vec();
        }
    }
}
impl Application {
    pub fn use_request_timeout(&mut self) -> &mut Self {
        self.add_middleware::<RequestTimeoutMiddleware>();
        self
    }
}
