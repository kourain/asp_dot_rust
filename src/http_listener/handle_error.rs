#[cfg(panic = "abort")]
use crate::http_context::HttpHeaderExt;
use crate::{
    http_context::{HttpContext, HttpResponse, ResponseHead},
    logging::LOGGER,
    middleware::MiddlewareNext,
};
use futures::future::{Either, select};
#[cfg(panic = "unwind")]
use futures_util::FutureExt;
#[cfg(panic = "unwind")]
use http::{HeaderValue, StatusCode, header::CONTENT_TYPE};
#[cfg(panic = "unwind")]
use std::{
    any::Any,
    backtrace::Backtrace,
    cell::Cell,
    panic::{self, AssertUnwindSafe, Location},
    sync::{Arc, Once},
};
use tokio::sync::oneshot;
#[cfg(panic = "unwind")]
use ulid::Ulid;

#[cfg(panic = "unwind")]
pub(super) struct HandleError {
    connection_id: Arc<Ulid>,
    request_id: Ulid,
    /// The report built by the panic hook, read after the panic is caught.
    report: Cell<Option<String>>,
}

#[cfg(panic = "abort")]
pub(super) struct HandleError;

impl HandleError {
    pub(super) async fn run(middleware: MiddlewareNext, mut http_context: HttpContext, head_rx: oneshot::Receiver<ResponseHead>) -> ResponseHead {
        let start = std::time::Instant::now();
        let pipeline = async move {
            #[cfg(panic = "unwind")]
            {
                let request = Self {
                    connection_id: http_context.request.connection_id.clone(),
                    request_id: *http_context.request.request_id,
                    report: Cell::new(None),
                };
                REQUEST_PANIC
                    .scope(request, async {
                        if let Err(payload) = AssertUnwindSafe(async { middleware.next_async(&mut http_context).await }).catch_unwind().await {
                            let report = REQUEST_PANIC.with(|request| {
                                request
                                    .report
                                    .take()
                                    .unwrap_or_else(|| Self::format_message(&request.connection_id, &request.request_id, payload.as_ref(), None))
                            });
                            LOGGER::error(report);
                            if http_context.response.has_started() {
                                http_context.abort();
                            } else {
                                http_context.response.status_code = StatusCode::INTERNAL_SERVER_ERROR;
                                http_context.response.headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
                                http_context.response.body = b"Internal Server Error".to_vec();
                            }
                        }
                    })
                    .await;
            }
            #[cfg(panic = "abort")]
            {
                // Set the build strategy before middleware can send the response headers.
                http_context.response.headers.insert_str("X-AspRs-Panic", "abort");
                middleware.next_async(&mut http_context).await;
            }

            LOGGER::info(format!(
                "{} {} {:?} {} {} in {:.3} ms",
                http_context.request.client_socket_addr,
                http_context.request.method,
                http_context.request.http_version,
                http_context.response.final_status(),
                http_context.request.uri,
                start.elapsed().as_secs_f64() * 1000.0
            ));
            if let Err(error) = http_context.response.complete_async().await {
                LOGGER::error(format!("Response not fully written: {error}"));
            }
        };

        // Run inline until the pipeline finishes or Hyper can start consuming the body.
        let response = match select(Box::pin(pipeline), head_rx).await {
            Either::Left((_, head_rx)) => head_rx.await,
            Either::Right((head, pipeline)) => {
                if head.is_ok() {
                    tokio::spawn(pipeline);
                }
                head
            }
        };
        response.unwrap_or_else(|_| {
            LOGGER::error("Pipeline ended without a response");
            HttpResponse::internal_server_error()
        })
    }

    #[cfg(panic = "unwind")]
    fn format_message(connection_id: &Ulid, request_id: &Ulid, payload: &(dyn Any + Send), location: Option<&Location<'_>>) -> String {
        let message = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("non-string panic payload");
        let origin = location.map(|location| format!(" at {location}")).unwrap_or_default();
        // The panic and the catch run on the same thread, so the current thread is the one that panicked.
        let thread = std::thread::current();
        let thread = thread.name().unwrap_or("unnamed");
        format!("Request {}:{}, Unhandled panic{origin} (thread {thread}): {message}", connection_id, request_id)
    }

    #[cfg(panic = "unwind")]
    fn clean_request_backtrace(backtrace: &str) -> String {
        use std::fmt::Write as _;

        fn frame_symbol(line: &str) -> Option<&str> {
            let (index, symbol) = line.split_once(": ")?;
            index.trim().parse::<usize>().ok()?;
            Some(symbol)
        }

        let after_hook = backtrace
            .lines()
            .position(|line| frame_symbol(line).is_some_and(|symbol| symbol.starts_with("std::panicking::panic_with_hook")))
            .map_or(0, |index| index + 1);
        let handler_module = concat!(module_path!(), "::");
        let mut output = String::new();
        let mut frame_count = 0;
        let mut keep_location = false;
        for line in backtrace.lines().skip(after_hook) {
            if let Some(symbol) = frame_symbol(line) {
                // MSVC uses an impl number where other targets print the HandleError type.
                if symbol.strip_prefix(handler_module).and_then(|symbol| symbol.split("::").nth(1)) == Some("run") {
                    break;
                }
                let panic_runtime = ["std::panicking::", "core::panicking::", "std::sys::backtrace::", "std::backtrace::", "rust_begin_unwind"]
                    .iter()
                    .any(|prefix| symbol.starts_with(prefix));
                let future_adapter = (symbol.starts_with("core::future::future::") && symbol.contains("::poll"))
                    || ((symbol.starts_with("<alloc::boxed::Box<") || symbol.starts_with("<core::pin::Pin<")) && symbol.contains(" as core::future::future::Future>::poll"));
                keep_location = !panic_runtime && !future_adapter;
                if keep_location {
                    let _ = writeln!(output, "{frame_count:>4}: {symbol}");
                    frame_count += 1;
                }
            } else if keep_location && line.trim_start().starts_with("at ") {
                let _ = writeln!(output, "{line}");
            }
        }
        if frame_count == 0 {
            return backtrace.trim_end().to_owned();
        }
        output.truncate(output.trim_end().len());
        output
    }

    #[cfg(panic = "unwind")]
    pub(super) fn install_request_panic_hook() {
        static INSTALL: Once = Once::new();
        INSTALL.call_once(|| {
            let previous_hook = panic::take_hook();
            panic::set_hook(Box::new(move |info| {
                let in_request = REQUEST_PANIC
                    .try_with(|request| {
                        let message = Self::format_message(&request.connection_id, &request.request_id, info.payload(), info.location());
                        let backtrace = Backtrace::force_capture().to_string();
                        let report = format!("{message}\nBacktrace:\n{}", Self::clean_request_backtrace(&backtrace));
                        request.report.set(Some(report));
                    })
                    .is_ok();
                if !in_request {
                    previous_hook(info);
                }
            }));
        });
    }
}

#[cfg(panic = "unwind")]
tokio::task_local! {
    static REQUEST_PANIC: HandleError;
}
