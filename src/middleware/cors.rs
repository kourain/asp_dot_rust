use http::header;

use crate::{
    Application,
    configuration::CorsConfiguration,
    http_context::{HttpContext, http_header::AspDotRustHttpHeader},
    middleware::Middleware,
};
use std::sync::Arc;
#[derive(Debug, Clone, Default)]
pub struct CorsMiddleware {
    routing_service: Arc<crate::services::routing::RoutingService>,
    configuration: CorsConfiguration,
}
#[async_trait::async_trait]
impl Middleware for CorsMiddleware {
    fn with_application(&mut self, application: &crate::application::Application) {
        self.routing_service = application.get_service::<crate::services::routing::RoutingService>();
        self.configuration = application.get_configuration::<CorsConfiguration>().clone();
    }
    async fn invoke_async<'a>(&self, http_context: &'a mut HttpContext, next: crate::middleware::MiddlewareNext) {
        {
            let request_origin = http_context.request.headers().origin();

            // server-to-server
            let Some(origin) = request_origin else {
                next(http_context).await;
                return;
            };

            // Origin not allowed
            if !self.configuration.is_origin_allowed(&origin) {
                http_context.response.status_code = http::StatusCode::FORBIDDEN;
                http_context.response.body = http::StatusCode::FORBIDDEN.canonical_reason().unwrap_or("Forbidden").as_bytes().to_vec();
                return;
            }

            // cors header for allowed origin
            let allow_origin = if self.configuration.is_origin_allowed(&origin) { &origin } else { "" };
            let route_method = self.routing_service.get_allowed_methods(&http_context.request.path);
            let route_method = self.configuration.allowed_methods.intersection(&route_method).cloned().collect::<Vec<http::Method>>();

            http_context.response.headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, allow_origin.parse().unwrap());
            http_context
                .response
                .headers
                .insert(header::ACCESS_CONTROL_ALLOW_METHODS, route_method.iter().map(|m| m.as_str()).collect::<Vec<_>>().join(", ").parse().unwrap());
            http_context
                .response
                .headers
                .insert(header::ACCESS_CONTROL_ALLOW_HEADERS, self.configuration.allowed_headers.iter().cloned().collect::<Vec<_>>().join(", ").parse().unwrap());
            if !self.configuration.exposed_headers.is_empty() {
                http_context
                    .response
                    .headers
                    .insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, self.configuration.exposed_headers.iter().cloned().collect::<Vec<_>>().join(", ").parse().unwrap());
            }
            if self.configuration.allow_credentials {
                http_context.response.headers.insert(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, "true".parse().unwrap());
            }
            // vary header to indicate response varies based on Origin
            http_context.response.headers.insert(header::VARY, "Origin".parse().unwrap());

            // Preflight OPTIONS
            let is_options = { http_context.request.method() == http::Method::OPTIONS };
            if is_options {
                http_context.response.headers.insert_str("Access-Control-Max-Age", &self.configuration.max_age.to_string());
                http_context.response.status_code = http::StatusCode::NO_CONTENT;
                return;
            }
        }
        // normal request
        next(http_context).await;
    }
}
impl Application {
    pub fn use_cors(&mut self) -> &mut Self {
        self.add_middleware::<CorsMiddleware>();
        self
    }
}
