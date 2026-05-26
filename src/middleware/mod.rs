pub(crate) mod app_middlewares;
pub(crate) mod authorize;
pub(crate) mod auto_route;
pub(crate) mod cors;
pub(crate) mod rate_limit;
pub(crate) mod request_timeout;
pub(crate) mod static_file;
use crate::http_context::http_context::HttpContext;
use core::any::type_name;
use core::{future::Future, pin::Pin};

// ─── Future type alias (kept for DynMiddleware fallback) ───
pub type MiddlewareFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

// ─── DynMiddleware: object-safe trait for external/user-defined middleware ───
// Uses async_trait for dynamic dispatch. Only external middleware pays Box::pin cost.
#[async_trait::async_trait]
pub trait DynMiddleware: Send + Sync {
    async fn invoke_async<'a>(&'a self, http_context: &'a mut HttpContext, next: MiddlewareNext<'a>);
    fn with_application(&mut self, application: &crate::application::Application);
    fn type_name(&self) -> &'static str {
        "DynMiddleware"
    }
}

// ─── MiddlewareNext: zero-alloc stack reference into pipeline ───
// Replaces Arc<dyn Fn -> Pin<Box<...>>>. Cost: 1 Box::pin per step (to break recursion).
pub struct MiddlewareNext<'a> {
    pipeline: &'a [MiddlewareKind],
    index: usize,
}

impl<'a> MiddlewareNext<'a> {
    pub(crate) fn new(pipeline: &'a [MiddlewareKind]) -> Self {
        Self { pipeline, index: 0 }
    }

    /// Advance to the next middleware. Uses Box::pin to break the recursive future type.
    pub fn invoke(self, ctx: &'a mut HttpContext) -> MiddlewareFuture<'a> {
        Box::pin(async move {
            if self.index < self.pipeline.len() {
                let next = MiddlewareNext {
                    pipeline: self.pipeline,
                    index: self.index + 1,
                };
                self.pipeline[self.index].invoke_async(ctx, next).await;
            }
        })
    }
}

// ─── MiddlewareKind: enum-based static dispatch ───
// Internal middleware is dispatched via match (no vtable, no async_trait Box::pin).
// External middleware goes through DynMiddleware trait (1 extra Box::pin).
pub(crate) enum MiddlewareKind {
    Cors(cors::CorsMiddleware),
    RateLimit(rate_limit::RateLimitMiddleware),
    RequestTimeout(request_timeout::RequestTimeoutMiddleware),
    Authorize(authorize::AuthorizeMiddleware),
    StaticFile(static_file::StaticFileMiddleware),
    AutoRoute(auto_route::AutoRouteMiddleware),
    /// Fallback for external/user-defined middleware
    Dynamic(Box<dyn DynMiddleware>),
}

impl MiddlewareKind {
    /// Dispatch to the concrete middleware's invoke method.
    /// Internal variants call inherent async methods (zero extra Box::pin).
    /// Dynamic variant calls through async_trait (1 extra Box::pin).
    pub(crate) async fn invoke_async<'a>(
        &'a self,
        ctx: &'a mut HttpContext,
        next: MiddlewareNext<'a>,
    ) {
        match self {
            Self::Cors(m) => m.invoke_async(ctx, next).await,
            Self::RateLimit(m) => m.invoke_async(ctx, next).await,
            Self::RequestTimeout(m) => m.invoke_async(ctx, next).await,
            Self::Authorize(m) => m.invoke_async(ctx, next).await,
            Self::StaticFile(m) => m.invoke_async(ctx, next).await,
            Self::AutoRoute(m) => m.invoke_async(ctx, next).await,
            Self::Dynamic(m) => m.invoke_async(ctx, next).await,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn with_application(&mut self, app: &crate::application::Application) {
        match self {
            Self::Cors(m) => m.with_application(app),
            Self::RateLimit(m) => m.with_application(app),
            Self::RequestTimeout(m) => m.with_application(app),
            Self::Authorize(m) => m.with_application(app),
            Self::StaticFile(m) => m.with_application(app),
            Self::AutoRoute(m) => m.with_application(app),
            Self::Dynamic(m) => m.with_application(app),
        }
    }

    pub(crate) fn type_name(&self) -> &'static str {
        match self {
            Self::Cors(_) => type_name::<cors::CorsMiddleware>(),
            Self::RateLimit(_) => type_name::<rate_limit::RateLimitMiddleware>(),
            Self::RequestTimeout(_) => type_name::<request_timeout::RequestTimeoutMiddleware>(),
            Self::Authorize(_) => type_name::<authorize::AuthorizeMiddleware>(),
            Self::StaticFile(_) => type_name::<static_file::StaticFileMiddleware>(),
            Self::AutoRoute(_) => type_name::<auto_route::AutoRouteMiddleware>(),
            Self::Dynamic(m) => m.type_name(),
        }
    }
}

// ─── middleware! macro: generates DynMiddleware impl for quick external middleware ───
#[macro_export]
macro_rules! middleware {
    ($vis:vis $name:ident, |$ctx:ident, $next:ident| $body:block) => {
        #[derive(Default)]
        $vis struct $name;

        #[async_trait::async_trait]
        impl $crate::middleware::DynMiddleware for $name {
            fn with_application(&mut self, _: &crate::application::Application) {
                // Default implementation does nothing, but can be overridden if needed
            }
            async fn invoke_async<'a>(
                &'a self,
                $ctx: &'a mut $crate::http_context::HttpContext,
                $next: $crate::middleware::MiddlewareNext<'a>,
            ) $body

            fn type_name(&self) -> &'static str {
                stringify!($name)
            }
        }
    };
}
