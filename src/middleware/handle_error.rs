#[cfg(panic = "unwind")]
use crate::logging::LOGGER;
use crate::{
    http_context::HttpContext,
    middleware::{Middleware, MiddlewareNext},
    services::service_provider::ServiceProviderScope,
};
#[cfg(panic = "unwind")]
use futures::FutureExt;
#[cfg(panic = "abort")]
use std::io::Write;
use std::{
    any::Any,
    backtrace::Backtrace,
    panic::{self, Location},
    sync::Once,
};
#[cfg(panic = "unwind")]
use std::{cell::Cell, panic::AssertUnwindSafe};
use ulid::Ulid;

/// The outermost request middleware, responsible for panic diagnostics and recovery.
pub(super) struct HandleErrorMiddleware {
    next: MiddlewareNext,
}

#[async_trait::async_trait]
impl Middleware for HandleErrorMiddleware {
    async fn inject(_: &ServiceProviderScope, next: MiddlewareNext) -> Self {
        install_request_panic_hook();
        Self { next }
    }

    async fn inject_invoke_async(&self, http_context: &mut HttpContext) {
        let request = RequestPanic {
            connection_id: *http_context.request.connection_id,
            request_id: http_context.request.request_id,
            #[cfg(panic = "unwind")]
            report: Cell::new(None),
        };
        REQUEST_PANIC
            .scope(request, async {
                #[cfg(panic = "unwind")]
                if let Err(payload) = AssertUnwindSafe(async { self.next.next_async(http_context).await }).catch_unwind().await {
                    let report = REQUEST_PANIC.with(|request| request.report.take().unwrap_or_else(|| request.format_message(payload.as_ref(), None)));
                    LOGGER::error(report);
                    if http_context.response.has_started() {
                        http_context.abort();
                    } else {
                        http_context.response.status_code = http::StatusCode::INTERNAL_SERVER_ERROR;
                        http_context.response.body = b"Internal Server Error".to_vec();
                    }
                }

                #[cfg(panic = "abort")]
                self.next.next_async(http_context).await;
            })
            .await;
    }
}

struct RequestPanic {
    connection_id: Ulid,
    request_id: Ulid,
    #[cfg(panic = "unwind")]
    report: Cell<Option<String>>,
}

impl RequestPanic {
    fn format_message(&self, payload: &(dyn Any + Send), location: Option<&Location<'_>>) -> String {
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload");
        let origin = location.map(|location| format!(" at {location}")).unwrap_or_default();
        format!("Request {}:{}, Unhandled panic{origin}: {message}", self.connection_id, self.request_id)
    }
}

tokio::task_local! {
    static REQUEST_PANIC: RequestPanic;
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

fn install_request_panic_hook() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let previous_hook = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            let _ = REQUEST_PANIC.try_with(|request| {
                let message = request.format_message(info.payload(), info.location());
                let backtrace = Backtrace::force_capture().to_string();
                let report = format!("{message}\nBacktrace:\n{}", middleware_backtrace(&backtrace));
                #[cfg(panic = "unwind")]
                request.report.set(Some(report));
                #[cfg(panic = "abort")]
                {
                    // The asynchronous logger may not run again before the process exits.
                    let _ = writeln!(std::io::stderr().lock(), "{report}");
                }
            });
            previous_hook(info);
        }));
    });
}
