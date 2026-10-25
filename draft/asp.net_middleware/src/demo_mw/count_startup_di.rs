use std::{io::{Write, stdout}, sync::{Arc, atomic::AtomicU64}};

use crate::code::{
    app::ServiceProviderScope,
    mw::{MiddlewareInjectable, Middleware, MiddlewareNext, MiddwareInjectInvoke},
};

pub struct CountStartupDIMiddleware {
    next: MiddlewareNext,
    demo_inject: Arc<AtomicU64>,
}
/// manual write or macro #[middleware] generate from new
impl MiddlewareInjectable for CountStartupDIMiddleware {
    fn inject(sp: &ServiceProviderScope, next: MiddlewareNext) -> Self {
        Self::new(next, sp.get_service_demo())
    }
}
///marco #[middleware] generate
#[async_trait::async_trait]
impl MiddwareInjectInvoke for CountStartupDIMiddleware {
    async fn inject_invoke_async(&self, ctx: &mut crate::code::app::HttpContext) {
        self.invoke_async(ctx).await;
    }
}
/// deriven marco generate (#[derive(Middleware)] at top struct) or manual 
impl Middleware for CountStartupDIMiddleware {}

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
