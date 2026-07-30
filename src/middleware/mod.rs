pub(crate) mod app_middlewares;
pub(crate) mod authorize;
pub(crate) mod auto_route;
pub(crate) mod cors;
pub(crate) mod request_timeout;
pub(crate) mod static_file;
use crate::dependcy_injection::DependcyInjectableService;
use crate::http_context::HttpContextRef;
use async_trait::async_trait;
use core::any::type_name;
use core::{future::Future, pin::Pin};
use std::sync::Arc;

pub type MiddlewareNext = Arc<dyn for<'a> Fn(&'a mut HttpContextRef) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>> + Send + Sync>;
#[async_trait]
pub trait Middleware: DependcyInjectableService + Send + Sync {
    async fn invoke_async(&self, http_context: &mut HttpContextRef, next: MiddlewareNext);
    fn type_name(&self) -> &'static str {
        type_name::<Self>()
    }
}

#[macro_export]
macro_rules! middleware {
    ($vis:vis $name:ident, |$ctx:ident, $next:ident| $body:block) => {
        #[derive(Default)]
        $vis struct $name;

        #[async_trait::async_trait]
        impl $crate::middleware::Middleware for $name {
            async fn invoke_async<'a>(
                &self,
                $ctx: &'a mut $crate::http_context::HttpContext,
                $next: $crate::middleware::MiddlewareNext,
            ) $body
        }
        impl $crate::dependcy_injection::DependcyInjectableService for $name {
            fn inject(
                _service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope,
            ) -> Self {
                Self
            }
        }
    };
}
