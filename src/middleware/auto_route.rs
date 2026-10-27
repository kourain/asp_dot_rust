use crate::{
    http_context::HttpContext,
    middleware::{Middleware, MiddlewareNext},
    services::{
        routing::RoutingResult::{Found, MethodNotAllowed, NotFound},
        service_provider::ServiceProviderScope,
    },
};

/// Last middleware of the chain, calls the router.
pub struct RoutingMiddleware;
#[async_trait::async_trait]
impl Middleware for RoutingMiddleware {
    fn inject(_: &ServiceProviderScope, _: MiddlewareNext) -> Self {
        Self {}
    }
    async fn inject_invoke_async(&self, http_context: &mut HttpContext) {
        match &http_context.request.routing_info.router_info {
            Found(controller) => {
                _ = (controller.invoke_async)(http_context, controller.action_name).await;
            }
            MethodNotAllowed(allowed_methods) => {
                http_context.response.status_code = http::StatusCode::METHOD_NOT_ALLOWED;
                http_context.response.body = http::StatusCode::METHOD_NOT_ALLOWED.canonical_reason().unwrap_or("Method Not Allowed").as_bytes().to_vec();
                http_context
                    .response
                    .headers
                    .insert(http::header::ALLOW, allowed_methods.iter().map(|m| m.as_str()).collect::<Vec<_>>().join(", ").parse().unwrap());
            }
            NotFound => {
                http_context.response.status_code = http::StatusCode::NOT_FOUND;
                http_context.response.body = http::StatusCode::NOT_FOUND.canonical_reason().unwrap_or("Not Found").as_bytes().to_vec();
            }
        }
    }
}
