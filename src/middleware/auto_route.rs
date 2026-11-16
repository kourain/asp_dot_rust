use crate::{
    http_context::{HttpContext, HttpContextRef},
    middleware::{Middleware, MiddlewareNext},
    services::{
        routing::RoutingResult::{Found, MethodNotAllowed, NotFound},
        service_provider::ServiceProviderScope,
    },
    utils::ShareMutPtr,
};

/// Last middleware of the chain, calls the router.
pub struct RoutingMiddleware;
#[async_trait::async_trait]
impl Middleware for RoutingMiddleware {
    async fn inject(_: &ServiceProviderScope, _: MiddlewareNext) -> Self {
        Self {}
    }
    async fn inject_invoke_async(&self, http_context: &mut HttpContext) {
        match &http_context.request.routing_info.router_info {
            Found(controller) => {
                let invoke_async = controller.invoke_async;
                let action_name = controller.action_name;
                let state = http_context.is_shareable.clone();
                let ctx_ref = HttpContextRef(ShareMutPtr::new(http_context, state));
                invoke_async(ctx_ref, action_name).await;
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
