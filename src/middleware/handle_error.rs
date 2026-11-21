use crate::{
    http_context::{HttpContext, HttpHeaderExt},
    logging::LOGGER,
    middleware::{Middleware, MiddlewareNext},
    services::service_provider::ServiceProviderScope,
};
use futures_util::FutureExt;
use http::{HeaderValue, StatusCode, header::CONTENT_TYPE};
use std::{
    any::Any,
    panic::{AssertUnwindSafe, Location},
};
use ulid::Ulid;

const PANIC_STRATEGY: &'static str = if cfg!(panic = "abort") { "abort" } else { "unwind" };

/// The outermost request middleware, responsible for panic diagnostics and recovery.
pub(super) struct HandleErrorMiddleware {
    next: MiddlewareNext,
}

#[async_trait::async_trait]
impl Middleware for HandleErrorMiddleware {
    async fn inject(_: &ServiceProviderScope, next: MiddlewareNext) -> Self {
        Self { next }
    }

    async fn inject_invoke_async(&self, http_context: &mut HttpContext) {
        // With panic = "abort" the future never returns an error, the hook has already written the report.
        let Err(payload) = AssertUnwindSafe(self.next.next_async(http_context)).catch_unwind().await else {
            http_context.response.headers.insert_str("X-AspRs-Panic", PANIC_STRATEGY);
            return;
        };
        let report = format_message(&http_context.request.connection_id, &http_context.request.request_id, payload.as_ref(), None);
        LOGGER::error(report);
        if http_context.response.has_started() {
            http_context.abort();
        } else {
            http_context.response.status_code = StatusCode::INTERNAL_SERVER_ERROR;
            // Headers set by the failed handler (such as a JSON content type) do not describe this body.
            http_context.response.headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
            http_context.response.body = b"Internal Server Error".to_vec();
        }
    }
}

fn format_message(connection_id: &Ulid, request_id: &Ulid, payload: &(dyn Any + Send), location: Option<&Location<'_>>) -> String {
    let message = payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("non-string panic payload");
    let origin = location.map(|location| format!(" at {location}")).unwrap_or_default();
    format!("Request {}:{}, Unhandled panic{origin}: {message}", connection_id, request_id)
}
fn middleware_backtrace(backtrace: &str) -> &str {
    let mut offset = 0;
    for line in backtrace.split_inclusive('\n') {
        // Rust symbol names differ across platforms, but include the module and method.
        if line.contains(module_path!()) && line.contains("::inject_invoke_async") {
            return backtrace[..offset].trim_end();
        }
        offset += line.len();
    }
    backtrace.trim_end()
}
