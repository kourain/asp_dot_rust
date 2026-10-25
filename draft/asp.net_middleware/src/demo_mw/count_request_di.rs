use std::{
    io::{Write, stdout},
    sync::{Arc, atomic::AtomicU64},
};

use crate::code::{
    app::ServiceProviderScope,
    mw::{Middleware, MiddlewareNext},
};

pub struct CountRequestDIMiddleware {
    next: MiddlewareNext,
}
/// manual write or macro #[middleware] generate
#[async_trait::async_trait]
impl Middleware for CountRequestDIMiddleware {
    fn inject(_: &ServiceProviderScope, next: MiddlewareNext) -> Self {
        Self::new(next)
    }
    async fn inject_invoke_async(&self, ctx: &mut crate::code::app::HttpContext) {
        let svp = &ctx.serv_provider;
        let demo_sv = svp.get_service_demo();
        self.invoke_async(ctx, demo_sv).await;
    }
}
/// #[middleware]
impl CountRequestDIMiddleware {
    pub fn new(next: MiddlewareNext) -> Self {
        Self { next }
    }
    pub async fn invoke_async(&self, ctx: &mut crate::code::app::HttpContext, demo_inject: Arc<AtomicU64>) {
        _ = stdout().write("call CountRequestDIMiddleware\n".as_bytes());
        // use demo_inject or do something
        self.next.call_async(ctx).await;
    }
}
