use std::io::{Write, stdout};

use crate::code::{
    app::ServiceProviderScope,
    mw::{Middleware, MiddlewareNext},
};

pub struct CountNoDIMiddleware {
    next: MiddlewareNext,
}
/// manual write or macro #[middleware] generate
#[async_trait::async_trait]
impl Middleware for CountNoDIMiddleware {
    fn inject(_: &ServiceProviderScope, next: MiddlewareNext) -> Self {
        Self { next }
    }
    async fn inject_invoke_async(&self, ctx: &mut crate::code::app::HttpContext) {
        self.invoke_async(ctx).await;
    }
}

/// #[middleware]
impl CountNoDIMiddleware {
    pub async fn invoke_async(&self, ctx: &mut crate::code::app::HttpContext) {
        _ = stdout().write("call CountNoDIMiddleware\n".as_bytes());
        // use demo_inject or do something
        self.next.call_async(ctx).await;
    }
}
