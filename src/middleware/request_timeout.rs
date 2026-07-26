use std::sync::Arc;

use crate::{
    Application, configuration::RequestTimeoutConfiguration, dependcy_injection::DependcyInjectableService, http_context::HttpContext, logging::LOGGER, middleware::Middleware, services::configuration::ConfigurationService,
};

#[derive(Debug, Clone)]
pub struct RequestTimeoutMiddleware {
    request_timeout_config: Arc<RequestTimeoutConfiguration>,
}
impl DependcyInjectableService for RequestTimeoutMiddleware {
    fn inject_service(service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
        let config = service_scope
            .get_service::<ConfigurationService>()
            .get::<RequestTimeoutConfiguration>().unwrap();
        RequestTimeoutMiddleware {
            request_timeout_config: config,
        }
    }
}
#[async_trait::async_trait]
impl Middleware for RequestTimeoutMiddleware {
    async fn invoke_async<'a>(&self, http_context: &'a mut HttpContext, next: crate::middleware::MiddlewareNext) {
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