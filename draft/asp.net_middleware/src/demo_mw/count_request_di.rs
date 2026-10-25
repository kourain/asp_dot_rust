use std::{
    io::{Write, stdout},
    sync::{Arc, atomic::AtomicU64},
};

use crate::code::{
    app::ServiceProviderScope,
    mw::{Middleware, MiddlewareInjectable, MiddlewareNext, MiddwareInjectInvoke},
};

pub struct CountRequestDIMiddleware {
    next: MiddlewareNext,
}
/// manual write or macro #[middleware] generate from new
impl MiddlewareInjectable for CountRequestDIMiddleware {
    fn inject(_: &ServiceProviderScope, next: MiddlewareNext) -> Self {
        Self::new(next)
    }
}
///marco #[middleware] generate
#[async_trait::async_trait]
impl MiddwareInjectInvoke for CountRequestDIMiddleware {
    async fn inject_invoke_async(&self, ctx: &mut crate::code::app::HttpContext) {
        let svp = &ctx.serv_provider;
        let demo_sv = svp.get_service_demo();
        self.invoke_async(ctx, demo_sv).await;
    }
}
/// deriven marco generate (#[derive(Middleware)] at top struct) or manual
impl Middleware for CountRequestDIMiddleware {}

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
