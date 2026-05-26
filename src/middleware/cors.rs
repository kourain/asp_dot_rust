use crate::{Application, configuration::CorsConfiguration, http_context::HttpContext, middleware::MiddlewareNext};
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
pub struct CorsMiddleware {
    routing_service: Arc<crate::services::routing::RoutingService>,
    configuration: CorsConfiguration,
}

impl CorsMiddleware {
    pub(crate) fn with_application(&mut self, application: &crate::application::Application) {
        self.routing_service = application.get_service::<crate::services::routing::RoutingService>();
        self.configuration = application.get_configuration::<CorsConfiguration>().clone();
    }

    pub(crate) async fn invoke_async<'a>(&self, http_context: &'a mut HttpContext, next: MiddlewareNext<'a>) {
        let request_origin = http_context.request.headers.origin();

        // server-to-server
        let Some(origin) = request_origin else {
            next.invoke(http_context).await;
            return;
        };

        // Origin not allowed
        if !self.configuration.is_origin_allowed(&origin) {
            http_context.response.status_code = http::StatusCode::FORBIDDEN;
            http_context.response.body = http::StatusCode::FORBIDDEN.canonical_reason().unwrap_or("Forbidden").as_bytes().to_vec();
            return;
        }

        // cors header for allowed origin
        let allow_origin = if self.configuration.is_origin_allowed(&origin) { origin } else { "" };
        let route_method = self.routing_service.get_allowed_methods(&http_context.request.path);
        let route_method = self.configuration.allowed_methods.intersection(&route_method).cloned().collect::<Vec<http::Method>>();

        http_context.response.headers.add("Access-Control-Allow-Origin", &allow_origin);
        http_context
            .response
            .headers
            .add("Access-Control-Allow-Methods", &route_method.iter().map(|m| m.as_str()).collect::<Vec<_>>().join(", "));
        http_context
            .response
            .headers
            .add("Access-Control-Allow-Headers", &self.configuration.allowed_headers.iter().cloned().collect::<Vec<_>>().join(", "));
        if !self.configuration.exposed_headers.is_empty() {
            http_context
                .response
                .headers
                .add("Access-Control-Expose-Headers", &self.configuration.exposed_headers.iter().cloned().collect::<Vec<_>>().join(", "));
        }
        if self.configuration.allow_credentials {
            http_context.response.headers.add("Access-Control-Allow-Credentials", "true");
        }
        // vary header to indicate response varies based on Origin
        http_context.response.headers.add("Vary", "Origin");

        // Preflight OPTIONS
        let is_options = { http_context.request.method == http::Method::OPTIONS };
        if is_options {
            http_context.response.headers.add("Access-Control-Max-Age", &self.configuration.max_age.to_string());
            http_context.response.status_code = http::StatusCode::NO_CONTENT;
            return;
        }

        // normal request
        next.invoke(http_context).await;
    }
}

impl Application {
    pub fn use_cors(&mut self) -> &mut Self {
        let mut mw = CorsMiddleware::default();
        mw.with_application(self);
        self._middlewares.add_kind(crate::middleware::MiddlewareKind::Cors(mw));
        self
    }
}
