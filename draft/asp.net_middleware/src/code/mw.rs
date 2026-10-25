use std::{
    io::{Write, stdout},
    sync::Arc,
};

use crate::code::app::{HttpContext, ServiceProviderScope};

pub struct ApplicationMiddwareBuilder {
    builder: Vec<Box<dyn Fn(&ServiceProviderScope, MiddlewareNext) -> MiddlewareNext>>,
}
impl ApplicationMiddwareBuilder {
    pub fn new() -> Self {
        Self { builder: Vec::new() }
    }
    pub fn add_middleware<T: Middleware + 'static>(&mut self) {
        self.builder.push(Box::new(move |_sp, next| MiddlewareNext::new(T::inject(_sp, next))));
    }
    pub fn build(&mut self, sp: &ServiceProviderScope) -> MiddlewareNext {
        let next: MiddlewareNext = MiddlewareNext::new(RoutingMiddleware::default());
        let result = self.builder.iter().rev().fold(next, |acc, f| f(sp, acc));
        self.builder.clear();
        result
    }
}

pub trait MiddlewareInjectable {
    fn inject(sp: &ServiceProviderScope, next: MiddlewareNext) -> Self
    where
        Self: Sized;
}
// pub type MiddlewareNextFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
#[async_trait::async_trait]
pub trait MiddwareInjectInvoke: Send + Sync {
    async fn inject_invoke_async(&self, ctx: &mut HttpContext);
}
pub trait Middleware: MiddwareInjectInvoke + MiddlewareInjectable + Send + Sync {}

#[derive(Default)]
pub struct RoutingMiddleware;

impl RoutingMiddleware {
    async fn invoker_async(&self, ctx: &mut HttpContext) {
        _ = stdout().write(ctx.http_request.as_bytes());
    }
}
#[async_trait::async_trait]
impl MiddwareInjectInvoke for RoutingMiddleware {
    async fn inject_invoke_async(&self, ctx: &mut HttpContext) {
        self.invoker_async(ctx).await;
    }
}

pub struct MiddlewareNext(Arc<dyn MiddwareInjectInvoke>);

impl MiddlewareNext {
    pub fn new<M: MiddwareInjectInvoke + 'static>(middleware: M) -> Self {
        Self(Arc::new(middleware))
    }
    pub async fn call_async(&self, ctx: &mut HttpContext) {
        self.0.inject_invoke_async(ctx).await;
    }
}
