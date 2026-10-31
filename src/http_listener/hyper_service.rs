use crate::{
    Application,
    configuration::HyperConfig,
    http_context::{HttpContext, http_request::HttpRequest, http_response::HttpResponse},
    logging::LOGGER,
    services::routing::RoutingService,
};
use futures::{FutureExt, future::Either};
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::server::conn::auto as auto_conn;
use std::sync::Arc;
use std::time::Duration;
use std::{convert::Infallible, panic::AssertUnwindSafe};
use tokio::net::TcpStream;
use ulid::Ulid;

pub(crate) async fn hyper_service(stream: TcpStream, app: Arc<Application>, routing_service: Arc<RoutingService>, hypercfg: Arc<HyperConfig>) -> std::io::Result<()> {
    let client_socket_addr = stream.peer_addr().unwrap();
    let local_listen_socket_addr = stream.local_addr().unwrap();
    let connection_id: Arc<Ulid> = Arc::new(Ulid::generate());
    let hyper_service = service_fn(move |req| {
        let app = app.clone();
        let start = std::time::Instant::now();
        // Extract request metadata before consuming the body
        let routing_service = routing_service.clone();
        let connection_id = connection_id.clone();
        let routing_info = routing_service.resolve(req.uri(), req.method());
        async move {
            let custom_req = HttpRequest::from_http(req, routing_info, client_socket_addr, local_listen_socket_addr, connection_id);

            // The response sends its head through `head_rx` as soon as the first byte is written or the response is complete
            let (custom_resp, head_rx) = HttpResponse::new_streaming();

            // Build HttpContext and run middlewares/handlers, then write the complete response (status, headers and body) into the stream
            let mut http_context = HttpContext::new(custom_req, custom_resp, app.service_provider.create_scope());
            let pipeline = async move {
                match AssertUnwindSafe(app.call_middlewares_async(&mut http_context)).catch_unwind().await {
                    Ok(()) => {}
                    Err(panic_payload) => {
                        LOGGER::error(format!(
                            "Request {}:{}, Unhandled panic: {:?}",
                            http_context.request.connection_id, http_context.request.request_id, panic_payload
                        ));
                        if http_context.response.has_started() {
                            // the status is already sent, the only way to tell the client is to break the stream
                            http_context.abort();
                        } else {
                            http_context.response.status_code = http::StatusCode::INTERNAL_SERVER_ERROR;
                            http_context.response.body = b"Internal Server Error".to_vec();
                        }
                    }
                }
                let duration = start.elapsed();
                LOGGER::info(format!(
                    "{} {} {:?} {} {} in {:.3} ms",
                    http_context.request.client_socket_addr,
                    http_context.request.method,
                    http_context.request.http_version,
                    http_context.request.uri,
                    http_context.response.final_status(),
                    duration.as_secs_f64() * 1000.0
                ));
                // Writes what is still in `response.body` (the whole body of a complete response) and ends the stream
                if let Err(e) = http_context.response.complete_async().await {
                    LOGGER::debug(format!("Response not fully written: {}", e));
                }
            };
            // Run the pipeline here: a complete response of normal size is written in a single poll and needs no task.
            // When the pipeline waits after the head is sent (streaming, slow client), it continues in its own task while hyper reads the body.
            let response = match futures::future::select(Box::pin(pipeline), head_rx).await {
                Either::Left((_, head_rx)) => head_rx.await.ok(),
                Either::Right((head, pipeline)) => {
                    if head.is_ok() {
                        tokio::spawn(pipeline);
                    }
                    head.ok()
                }
            };
            Ok::<_, Infallible>(response.unwrap_or_else(|| {
                LOGGER::error("Pipeline ended without a response");
                HttpResponse::internal_server_error()
            }))
        }
    });
    let mut builder = auto_conn::Builder::new(hyper_util::rt::TokioExecutor::new());
    // HTTP/1.1 keep-alive is already on by default, but without a timer it
    // never times out: a client that opens a connection and stops sending
    // requests (or headers) would hold the socket/task open indefinitely.
    builder.http1().timer(TokioTimer::new()).header_read_timeout(Duration::from_secs(hypercfg.header_read_timeout));
    // HTTP/2 multiplexes over a single long-lived connection by design, so
    // enable PING-based keep-alive to detect and drop dead peers.
    builder
        .http2()
        .timer(TokioTimer::new())
        .keep_alive_interval(Duration::from_secs(hypercfg.http2_keep_alive_interval))
        .keep_alive_timeout(Duration::from_secs(hypercfg.http2_keep_alive_timeout));
    // wrap the tokio TcpStream so hyper-util can use hyper RT traits
    let io = TokioIo::new(stream);
    builder.serve_connection(io, hyper_service).await.map_err(std::io::Error::other)
}
