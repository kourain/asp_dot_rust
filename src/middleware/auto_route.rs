use http::{
    HeaderMap, Method, StatusCode,
    header::{ALLOW, CONTENT_TYPE},
};

use crate::{
    controller::{BindError, RoutingError},
    dependency_injection::DependencyInjectableMiddleware,
    http_context::{HttpContext, HttpContextRef, HttpHeaderExt, HttpResponse},
    logging::LOGGER,
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
impl DependencyInjectableMiddleware for RoutingMiddleware {
    async fn inject(_: &ServiceProviderScope, _: MiddlewareNext) -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Middleware for RoutingMiddleware {
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
                        if http_context.response.has_finished() {
                            return;
                        }
                        if !http_context.response.has_started() {
                            let status_code = result.status_code();
                            let mut headers = HeaderMap::new();
                            result.write_header(&mut headers);
                            let response = &mut http_context.response;
                            response.status_code = status_code;
                            response.headers.extend(headers);
                        }
                        let response = &mut http_context.response;
                        response.body.clear();
                        let written = result.write_body_async(&http_context.request.headers, response).await;
                        drop(result);
                        if let Err(error) = written {
                            LOGGER::debug(format!("Action result body not fully written: {error}"));
                            if response.has_started() {
                                response.abort();
                            } else {
                                set_status_response(response, StatusCode::INTERNAL_SERVER_ERROR);
                            }
                        }
                        response.set_buffered_content_length();
                    }
                    Err(RoutingError::Bind(error)) => {
                        let response = &mut http_context.response;
                        response.status_code = match &error {
                            BindError::UnsupportedMediaType(_) if is_query => StatusCode::UNSUPPORTED_MEDIA_TYPE,
                            _ => error.status_code(),
                        };
                        response.body = error.to_string().into_bytes();
                        response.headers.set_content_type("text/plain");
                        response.headers.set_content_length(response.body.len());
                    }
                    Err(RoutingError::ActionNotFound) => set_status_response(&mut http_context.response, StatusCode::NOT_FOUND),
                }
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
