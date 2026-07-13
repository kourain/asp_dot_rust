use crate::{
    Application,
    dependcy_injection::InjectableService,
    http_context::HttpContext,
    http_context::{HttpContext, http_header::AspDotRustHttpHeader},
    middleware::{Middleware, MiddlewareNext},
};
#[derive(Default)]
pub(crate) struct AuthorizeMiddleware {
    schema: String,
}
impl InjectableService for AuthorizeMiddleware {
    fn inject_service(_service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
        AuthorizeMiddleware::default()
    }
}
#[async_trait::async_trait]
impl Middleware for AuthorizeMiddleware {
    async fn invoke_async<'a>(&self, http_context: &'a mut HttpContext, next: MiddlewareNext) {
        let auth_header: Option<String> = http_context.request.headers.authorization();
        if let Some(auth_header) = auth_header {
            let split_token: Vec<&str> = auth_header.trim().split(" ").collect::<Vec<&str>>();
            if let Some(schema) = split_token.get(0) {
                if *schema == self.schema {
                    next(http_context).await;
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
        self.add_middleware::<AuthorizeMiddleware>();
        self
    }
    pub fn use_authorize_bearer(&mut self) -> &mut Self {
        let mut middleware = AuthorizeMiddleware::default();
        middleware.schema = "Bearer".to_string();
        self.add_middleware_instance(middleware);
        self
    }
    pub fn use_authorize_basic(&mut self) -> &mut Self {
        let mut middleware = AuthorizeMiddleware::default();
        middleware.schema = "Basic".to_string();
        self.add_middleware_instance(middleware);
        self
    }
    pub fn use_authorize_badge(&mut self) -> &mut Self {
        let mut middleware = AuthorizeMiddleware::default();
        middleware.schema = "Badge".to_string();
        self.add_middleware_instance(middleware);
        self
    }
}
