use crate::{
    Application,
    http_context::{HttpContext, http_header::HttpHeader},
    middleware::{Middleware, MiddlewareNext},
    services::service_provider::ServiceProviderScope,
};
pub(crate) struct AuthorizeMiddleware {
    next: MiddlewareNext,
    schema: String,
}
/// written by hand, `schema` is not a service: `use_authorize` creates it with `add_middleware_factory`.
#[async_trait::async_trait]
impl Middleware for AuthorizeMiddleware {
    fn inject(_service_scope: &ServiceProviderScope, next: MiddlewareNext) -> Self {
        Self { next, schema: String::new() }
    }
    async fn inject_invoke_async(&self, http_context: &mut HttpContext) {
        self.invoke_async(http_context).await;
    }
}
impl AuthorizeMiddleware {
    async fn invoke_async(&self, http_context: &mut HttpContext) {
        let auth_header: Option<String> = http_context.request.headers.authorization();
        if let Some(auth_header) = auth_header {
            let split_token: Vec<&str> = auth_header.trim().split(" ").collect::<Vec<&str>>();
            if let Some(schema) = split_token.first()
                && *schema == self.schema
            {
                self.next.next_async(http_context).await;
                return;
            }
        }
        // No Authorization header present
        http_context.response.status_code = http::StatusCode::UNAUTHORIZED; // Unauthorized
        http_context.response.body = http::StatusCode::UNAUTHORIZED.canonical_reason().unwrap_or("Unauthorized").as_bytes().to_vec();
    }
}
impl Application {
    pub fn use_authorize(&mut self, schema: impl Into<String>) -> &mut Self {
        let schema: String = schema.into();
        self.add_middleware_factory(move |next| AuthorizeMiddleware { next, schema: schema.clone() });
        self
    }
    pub fn use_authorize_bearer(&mut self) -> &mut Self {
        self.use_authorize("Bearer")
    }
    pub fn use_authorize_basic(&mut self) -> &mut Self {
        self.use_authorize("Basic")
    }
    pub fn use_authorize_badge(&mut self) -> &mut Self {
        self.use_authorize("Badge")
    }
}
