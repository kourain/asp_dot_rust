use std::{
    io::{Write, stdout},
    sync::{Arc, atomic::AtomicU64},
};

use crate::code::{
    app::ServiceProviderScope,
    mw::{Middleware, MiddlewareNext},
};

pub struct CountStartupDIMiddleware {
    next: MiddlewareNext,
    demo_inject: Arc<AtomicU64>,
}
/// manual write or macro #[middleware] generate
#[async_trait::async_trait]
impl Middleware for CountStartupDIMiddleware {
    fn inject(sp: &ServiceProviderScope, next: MiddlewareNext) -> Self {
        Self::new(next, sp.get_service_demo())
    }
    async fn inject_invoke_async(&self, ctx: &mut crate::code::app::HttpContext) {
        self.invoke_async(ctx).await;
    }
}
/// #[middleware]
impl CountStartupDIMiddleware {
    pub fn new(next: MiddlewareNext, demo_inject: Arc<AtomicU64>) -> Self {
        Self { next, demo_inject }
    }
    pub async fn invoke_async(&self, ctx: &mut crate::code::app::HttpContext) {
        _ = stdout().write("call CountStartupDIMiddleware\n".as_bytes());
        // use demo_inject or do something
        self.next.call_async(ctx).await;
    }
}
