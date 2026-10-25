use std::io::{Write, stdout};

use crate::code::{
    app::ServiceProviderScope,
    mw::{Middleware, MiddlewareInjectable, MiddlewareNext, MiddwareInjectInvoke},
};

pub struct CountNoDIMiddleware {
    next: MiddlewareNext,
}
/// manual write or #[derive(Middleware)] at top struct
impl MiddlewareInjectable for CountNoDIMiddleware {
    fn inject(_: &ServiceProviderScope, next: MiddlewareNext) -> Self {
        Self { next }
    }
}
///marco #[middleware] generate
#[async_trait::async_trait]
impl MiddwareInjectInvoke for CountNoDIMiddleware {
    async fn inject_invoke_async(&self, ctx: &mut crate::code::app::HttpContext) {
        self.invoke_async(ctx).await;
    }
}
/// deriven marco generate (#[derive(Middleware)] at top struct) or manual
impl Middleware for CountNoDIMiddleware {}

/// #[middleware]
impl CountNoDIMiddleware {
    pub async fn invoke_async(&self, ctx: &mut crate::code::app::HttpContext) {
        _ = stdout().write("call CountNoDIMiddleware\n".as_bytes());
        // use demo_inject or do something
        self.next.call_async(ctx).await;
    }
}
