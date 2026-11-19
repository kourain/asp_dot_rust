use http::{
    HeaderValue, Method, StatusCode,
    header::{ALLOW, CONTENT_TYPE},
};

use crate::{
    controller::{BindError, RoutingError},
    http_context::{HttpContext, HttpContextRef, HttpHeaderExt, HttpResponse},
    middleware::{Middleware, MiddlewareNext},
    services::{
        routing::RoutingResult::{Found, MethodNotAllowed, NotFound},
        service_provider::ServiceProviderScope,
    },
    utils::ShareMutPtr,
};

/// Invokes the matched action and writes its result into the response.
pub struct RoutingMiddleware;

fn set_status_response(response: &mut HttpResponse, status: StatusCode) {
    response.status_code = status;
    response.body = status.canonical_reason().unwrap_or_default().as_bytes().to_vec();
}

#[async_trait::async_trait]
impl Middleware for RoutingMiddleware {
    async fn inject(_: &ServiceProviderScope, _: MiddlewareNext) -> Self {
        Self {}
    }
    async fn inject_invoke_async(&self, http_context: &mut HttpContext) {
        match &http_context.request.routing_info.router_info {
            Found(controller) => {
                let is_query = http_context.request.method == Method::QUERY;
                if is_query
                    && http_context.request.headers.get(CONTENT_TYPE).and_then(|value| value.to_str().ok()).is_none_or(|value| value.trim().is_empty())
                {
                    set_status_response(&mut http_context.response, StatusCode::BAD_REQUEST);
                    return;
                }

                let activator_async = controller.activator_async;
                let action_name = controller.action_name;
                let state = http_context.is_shareable.clone();
                let ctx_ref = HttpContextRef(ShareMutPtr::new(http_context, state));
                let mut controller = activator_async(ctx_ref).await;
                match controller.routing(action_name).await {
                    Ok(result) => {
                        let body = result.get_body_async().await;
                        let content_type = result.content_type();
                        let content_type = (!content_type.is_empty()).then(|| HeaderValue::from_str(content_type).unwrap());
                        let status_code = result.status_code();
                        let headers = result.headers();
                        drop(result);

                        let response = &mut http_context.response;
                        response.status_code = status_code;
                        response.body = body;
                        response.headers.extend(headers);
                        if let Some(content_type) = content_type {
                            response.headers.insert(CONTENT_TYPE, content_type);
                        }
                    }
                    Err(RoutingError::Bind(error)) => {
                        let response = &mut http_context.response;
                        response.status_code = match &error {
                            BindError::UnsupportedMediaType(_) if is_query => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                            _ => error.status_code(),
                        };
                        response.body = error.to_string().into_bytes();
                        response.headers.set_content_type("text/plain");
                    }
                    Err(RoutingError::ActionNotFound) => set_status_response(&mut http_context.response, StatusCode::NOT_FOUND),
                }
                let response = &mut http_context.response;
                response.headers.set_content_length(response.body.len());
            }
            MethodNotAllowed(allowed_methods) => {
                let response = &mut http_context.response;
                set_status_response(response, StatusCode::METHOD_NOT_ALLOWED);
                response.headers.insert_str(ALLOW, allowed_methods.iter().map(|m| m.as_str()).collect::<Vec<_>>().join(", "));
            }
            NotFound => set_status_response(&mut http_context.response, StatusCode::NOT_FOUND),
        }
    }
}
