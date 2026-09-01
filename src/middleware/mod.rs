pub(crate) mod app_middlewares;
pub(crate) mod authorize;
pub(crate) mod auto_route;
pub(crate) mod compression;
pub(crate) mod cors;
pub(crate) mod rate_limit;
pub(crate) mod request_timeout;
pub(crate) mod static_file;
use crate::http_context::HttpContext;
use crate::services::service_provider::ServiceProviderScope;
use std::sync::Arc;

/// The next middleware of the chain, given to the constructor of a middleware (like `RequestDelegate` in ASP.NET).
pub struct MiddlewareNext(Arc<dyn Middleware>);
impl MiddlewareNext {
    pub(crate) fn new<M: Middleware + 'static>(middleware: M) -> Self {
        Self(Arc::new(middleware))
    }
    pub async fn next_async(&self, http_context: &mut HttpContext) {
        self.0.inject_invoke_async(http_context).await;
    }
}

/// Creates a middleware from the services and the next middleware.
/// A middleware of the chain, `#[middleware]` implements it.
#[async_trait::async_trait]
pub trait Middleware: Send + Sync {
    fn inject(service_scope: &ServiceProviderScope, next: MiddlewareNext) -> Self
    where
        Self: Sized;
    async fn inject_invoke_async(&self, http_context: &mut HttpContext);
}

/// Defines a middleware that has only the field `next`: `middleware!(pub Name, |ctx, next| { next.call_async(ctx).await; });`
#[macro_export]
macro_rules! middleware {
    ($vis:vis $name:ident, |$ctx:ident, $next:ident| $body:block) => {
        $vis struct $name {
            next: $crate::middleware::MiddlewareNext,
        }
        #[$crate::macros::middleware]
        impl $name {
            pub async fn invoke_async(&self, $ctx: &mut $crate::http_context::HttpContext) {
                let $next = &self.next;
                $body
            }
        }
    };
}
