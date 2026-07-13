use crate::{
    configuration::RequestTimeoutConfiguration, dependcy_injection::DependcyInjectableService, http_context::HttpContext, logging::LOGGER, middleware::Middleware,
    services::configuration::ConfigurationService,
};

#[derive(Debug, Clone)]
pub struct RequestTimeoutMiddleware {
    timeout_seconds: u64,
}
impl DependcyInjectableService for RequestTimeoutMiddleware {
    fn inject_service(service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
        let config = service_scope
            .get_service::<ConfigurationService>()
            .get::<RequestTimeoutConfiguration>()
            .unwrap_or(RequestTimeoutConfiguration { timeout_seconds: 30 });
        RequestTimeoutMiddleware {
            timeout_seconds: config.timeout_seconds,
        }
    }
}
#[async_trait::async_trait]
impl Middleware for RequestTimeoutMiddleware {
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
