use std::sync::Arc;

use crate::{dependcy_injection::InjectableService, http_context::HttpContext, middleware::Middleware};
#[derive(Default)]
pub(crate) struct AutoRouteMiddleware {
    routing_service: Arc<crate::services::routing::RoutingService>,
}
impl InjectableService for AutoRouteMiddleware {
    fn inject_service(service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
        let routing_service = service_scope.get_service::<crate::services::routing::RoutingService>();
        AutoRouteMiddleware { routing_service }
    }
}
#[async_trait::async_trait]
impl Middleware for AutoRouteMiddleware {
    async fn invoke_async<'a>(&self, http_context: &'a mut HttpContext, _next: crate::middleware::MiddlewareNext) {
        let (request_path, request_method) = { (&http_context.request.path, &http_context.request.method) };
        match self.routing_service.resolve(request_path) {
            Some(route_info) => {
                let controller = route_info.router_info.get(&request_method);
                match controller {
                    Some(controller) => {
                        _ = (controller.invoke)(http_context, controller.action_name.into()).await;
                    }
                    None => {
                        http_context.response.status_code = http::StatusCode::METHOD_NOT_ALLOWED;
                        http_context.response.body = http::StatusCode::METHOD_NOT_ALLOWED.canonical_reason().unwrap_or("Method Not Allowed").as_bytes().to_vec();
                        return;
                    }
                }
            }
            None => {
                http_context.response.status_code = http::StatusCode::NOT_FOUND;
                http_context.response.body = http::StatusCode::NOT_FOUND.canonical_reason().unwrap_or("Not Found").as_bytes().to_vec();
                return;
            }
        }
    }
}
