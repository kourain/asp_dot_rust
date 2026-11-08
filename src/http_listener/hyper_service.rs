use crate::{
    Application,
    configuration::HyperConfig,
    dependency_injection::Serv,
    http_context::{HttpContext, http_request::HttpRequest, http_response::HttpResponse},
    logging::LOGGER,
    services::routing::{MAX_BODY_SIZE_ATTRIBUTE, ResolvedRoute, RoutingResult, RoutingService},
};
use futures::{FutureExt, future::Either};
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::server::conn::auto as auto_conn;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::{convert::Infallible, panic::AssertUnwindSafe};
use tokio::io::{AsyncRead, AsyncWrite};
use ulid::Ulid;

/// Size of the headers: name + value + 4 bytes (`: ` and CRLF) for each of them.
fn headers_size(headers: &http::HeaderMap) -> usize {
    headers.iter().map(|(name, value)| name.as_str().len() + value.len() + 4).sum()
}

/// The body limit of the request: the limit of the route it resolves to, or `max_request_body_size` when the route has none.
fn max_body_size_of(route: &ResolvedRoute, limits: &HyperConfig) -> usize {
    match &route.router_info {
        RoutingResult::Found(info) => info.route_attr.get::<usize>(MAX_BODY_SIZE_ATTRIBUTE).unwrap_or(limits.max_request_body_size),
        _ => limits.max_request_body_size,
    }
}

/// The status to refuse the request with when it is over `max_request_headers_size` (431)
/// or its declared `Content-Length` is over `max_body_size` (413).
fn exceeds_limits<B>(req: &http::Request<B>, limits: &HyperConfig, max_body_size: usize) -> Option<http::StatusCode> {
    if headers_size(req.headers()) > limits.max_request_headers_size {
        return Some(http::StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE);
    }
    let declared_length = req
        .headers()
        .get(http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u128>().ok());
    if declared_length.is_some_and(|length| length > max_body_size as u128) {
        return Some(http::StatusCode::PAYLOAD_TOO_LARGE);
    }
    None
}

/// Serve a single accepted connection over HTTP/1 or HTTP/2 (negotiated
/// automatically). `io` can be a plain `TcpStream` (HTTP) or a TLS-wrapped
/// stream such as `tokio_rustls::server::TlsStream<TcpStream>` (HTTPS) —
/// both implement `AsyncRead + AsyncWrite`, so this function is shared by
/// the HTTP and HTTPS listeners instead of being duplicated.
///
/// The peer/local socket addresses are taken as parameters (rather than
/// queried from `io` itself) because a TLS stream no longer exposes
/// `peer_addr`/`local_addr` directly; callers grab these from the raw
/// `TcpStream` before wrapping it (e.g. before the TLS handshake).
pub(crate) async fn hyper_service<S>(
    io: S,
    client_socket_addr: SocketAddr,
    local_listen_socket_addr: SocketAddr,
    app: Arc<Application>,
    routing_service: Serv<RoutingService>,
    hypercfg: Arc<HyperConfig>,
) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let connection_id: Arc<Ulid> = Arc::new(Ulid::generate());
    let limits = hypercfg.clone();
    let hyper_service = service_fn(move |req| {
        let app = app.clone();
        let limits = limits.clone();
        let start = std::time::Instant::now();
        // Extract request metadata before consuming the body
        let routing_service = routing_service.clone();
        let connection_id = connection_id.clone();
        let routing_info = routing_service.resolve(req.uri(), req.method());
        async move {
            // Refuse a request over the limits before it reaches the pipeline
            let max_body_size = max_body_size_of(&routing_info, &limits);
            if let Some(status) = exceeds_limits(&req, &limits, max_body_size) {
                LOGGER::warn(format!("{} {} {} refused with {}", client_socket_addr, req.method(), req.uri(), status));
                // the request body is not read, an HTTP/1.x connection can not be reused
                return Ok::<_, Infallible>(HttpResponse::status_only(status, req.version() <= http::Version::HTTP_11));
            }
            let custom_req = HttpRequest::from_http(req, routing_info, max_body_size, client_socket_addr, local_listen_socket_addr, connection_id);

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
        .keep_alive_timeout(Duration::from_secs(hypercfg.http2_keep_alive_timeout))
        .max_header_list_size(u32::try_from(hypercfg.max_request_headers_size).unwrap_or(u32::MAX));
    // wrap the async stream so hyper-util can use hyper RT traits
    let io = TokioIo::new(io);
    builder.serve_connection(io, hyper_service).await.map_err(std::io::Error::other)
}
