use crate::http_context::HttpContext;
use std::pin::Pin;

pub fn invoke_async<'a>(http_context: &'a mut HttpContext) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async {
        match &http_context.request.routing_info.router_info {
            Some(controller) => {
                _ = (controller.invoke_async)(http_context, controller.action_name).await;
            }
            None => {
                http_context.response.status_code = http::StatusCode::METHOD_NOT_ALLOWED;
                http_context.response.body = http::StatusCode::METHOD_NOT_ALLOWED.canonical_reason().unwrap_or("Method Not Allowed").as_bytes().to_vec();
            }
        }
    })
}
