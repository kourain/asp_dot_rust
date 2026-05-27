pub(crate) mod app_middlewares;
pub(crate) mod middleware_service;
pub(crate) mod pipeline;
pub(crate) mod rate_limit;
pub(crate) mod request_timeout;
pub(crate) mod static_file;

use crate::http_context::HttpContext;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// --- Core traits ---

pub trait MiddlewareService: Send + Sync {
    fn invoke_async<'a>(&'a self, ctx: &'a mut HttpContext) -> impl Future<Output = ()> + Send + 'a;
}

pub trait Middleware<S: MiddlewareService>: Send + Sync {
    type Service: MiddlewareService;
    fn wrap(self, inner: S) -> Self::Service;
}

pub trait WithApplication {
    fn with_application(&mut self, _app: &crate::Application) {}
}

// --- BoxedService ---

pub type MiddlewareFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

pub struct BoxedService(Arc<dyn for<'a> Fn(&'a mut HttpContext) -> MiddlewareFuture<'a> + Send + Sync>);

impl BoxedService {
    pub fn new<S>(svc: S) -> Self
    where
        S: MiddlewareService + Send + Sync + 'static,
    {
        Self(Arc::new(move |ctx| Box::pin(svc.invoke_async(ctx))))
    }

    pub fn new_fn<F, Fut>(f: F) -> Self
    where
        F: Fn(&mut HttpContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        Self(Arc::new(move |ctx| Box::pin(f(ctx))))
    }
}

impl MiddlewareService for BoxedService {
    fn invoke_async<'a>(&'a self, ctx: &'a mut HttpContext) -> impl Future<Output = ()> + Send + 'a {
        (self.0)(ctx)
    }
}

impl Clone for BoxedService {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
