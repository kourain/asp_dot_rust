use std::sync::Arc;

use crate::{
    Application,
    configuration::WebSocketOptions,
    http_context::HttpContext,
    macros::{DependencyInjectableMiddleware, middleware},
    middleware::MiddlewareNext,
};
use http::{HeaderMap, StatusCode, header::ORIGIN};

#[derive(DependencyInjectableMiddleware)]
struct WebSocketMiddleware {
    next: MiddlewareNext,
    #[di(default)]
    options: Arc<WebSocketOptions>,
}

fn origin_allowed(headers: &HeaderMap, allowed: &[String]) -> bool {
    if allowed.is_empty() || allowed.iter().any(|origin| origin == "*") {
        return true;
    }
    let mut origins = headers.get_all(ORIGIN).iter();
    let Some(origin) = origins.next() else { return true };
    origins.next().is_none() && origin.to_str().is_ok_and(|value| allowed.iter().any(|origin| origin.eq_ignore_ascii_case(value)))
}

#[middleware]
impl WebSocketMiddleware {
    async fn invoke_async(&self, ctx: &mut HttpContext) {
        if let Some(handshake) = &mut ctx.web_socket_handshake
            && handshake.options.is_none()
        {
            if !origin_allowed(&ctx.request.headers, &self.options.allowed_origins) {
                ctx.response.status_code = StatusCode::FORBIDDEN;
                return;
            }
            handshake.options = Some(self.options.clone());
        }
        self.next.next_async(ctx).await;
    }
}

impl Application {
    pub fn use_web_sockets(&mut self) -> &mut Self {
        self.use_web_sockets_with_options(WebSocketOptions::default())
    }

    pub fn use_web_sockets_with_options(&mut self, options: WebSocketOptions) -> &mut Self {
        self.web_sockets_enabled = true;
        let options = Arc::new(options);
        self.add_middleware_factory(move |next| WebSocketMiddleware { next, options: options.clone() })
    }
}
