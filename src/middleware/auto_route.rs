use crate::http_context::HttpContextRef;
use std::pin::Pin;

pub fn invoke_async<'a>(http_context: &'a mut HttpContextRef) -> Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async {
        match &http_context.routing_info {
            Some(route_info) => {
                let controller = route_info.router_info.get(&http_context.request.method);
                match controller {
                    Some(controller) => {
                        _ = (controller.invoke_async)(http_context, controller.action_name.into()).await;
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
    })
}
