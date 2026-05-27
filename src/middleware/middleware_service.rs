use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::http_context::HttpContext;

// Thay MiddlewareNext bằng generic S: Service
pub trait MiddlewareService: Send + Sync {
    fn invoke_async<'a>(&'a self, ctx: &'a mut HttpContext) -> impl Future<Output = ()> + Send + 'a;
}

// Inner service — endpoint cuối chuỗi
pub struct EndpointService {}

impl MiddlewareService for EndpointService {
    async fn invoke_async<'a>(&'a self, ctx: &'a mut HttpContext) {}
}
