use crate::{
    Application, http_context::HttpContext, middleware::MiddlewareNext
};

#[derive(Default)]
pub(crate) struct AuthorizeMiddleware {
    schema: String,
}

impl AuthorizeMiddleware {
    pub(crate) fn with_application(&mut self, _: &crate::application::Application) {
        // Default implementation does nothing, but can be overridden if needed
    }

    pub(crate) async fn invoke_async<'a>(&self, http_context: &'a mut HttpContext, next: MiddlewareNext<'a>) {
        let auth_header: Option<String> = {
            http_context.request.headers.authorization().map(|value: &String| value.to_string())
        };
        if let Some(auth_header) = auth_header {
            let split_token: Vec<&str> = auth_header.trim().split(" ").collect::<Vec<&str>>();
            if let Some(schema) = split_token.get(0) {
                if *schema == self.schema {
                    next.invoke(http_context).await;
                    return;
                }
            }
        }
        // No Authorization header present
        http_context.response.status_code = http::StatusCode::UNAUTHORIZED; // Unauthorized
        http_context.response.body = http::StatusCode::UNAUTHORIZED.canonical_reason().unwrap_or("Unauthorized").as_bytes().to_vec();
    }
}

impl Application {
    pub fn use_authorize(&mut self) -> &mut Self {
        let mut mw = AuthorizeMiddleware::default();
        mw.with_application(self);
        self._middlewares.add_kind(crate::middleware::MiddlewareKind::Authorize(mw));
        self
    }
    pub fn use_authorize_bearer(&mut self) -> &mut Self {
        let mut middleware = AuthorizeMiddleware::default();
        middleware.schema = "Bearer".to_string();
        middleware.with_application(self);
        self._middlewares.add_kind(crate::middleware::MiddlewareKind::Authorize(middleware));
        self
    }
    pub fn use_authorize_basic(&mut self) -> &mut Self {
        let mut middleware = AuthorizeMiddleware::default();
        middleware.schema = "Basic".to_string();
        middleware.with_application(self);
        self._middlewares.add_kind(crate::middleware::MiddlewareKind::Authorize(middleware));
        self
    }
    pub fn use_authorize_badge(&mut self) -> &mut Self {
        let mut middleware = AuthorizeMiddleware::default();
        middleware.schema = "Badge".to_string();
        middleware.with_application(self);
        self._middlewares.add_kind(crate::middleware::MiddlewareKind::Authorize(middleware));
        self
    }
}
