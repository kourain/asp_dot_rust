use crate::{
    configuration::HyperConfig,
    dependency_injection::Serv,
    http_context::{HttpContext, HttpRequest, HttpResponse},
    logging::LOGGER,
    middleware::MiddlewareNext,
    services::{
        routing::{MAX_BODY_SIZE_ATTRIBUTE, ResolvedRoute, RoutingResult, RoutingService},
        service_provider::ServiceProviderScope,
    },
};
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::server::conn::auto as auto_conn;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::net::TcpStream;
use ulid::Ulid;

use super::handle_error::HandleError;

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

pub(crate) async fn hyper_service(
    stream: TcpStream,
    service_provider: Arc<ServiceProviderScope>,
    middleware_pipeline: MiddlewareNext,
    routing_service: Serv<RoutingService>,
    hypercfg: Arc<HyperConfig>,
) -> std::io::Result<()> {
    let client_socket_addr = stream.peer_addr().unwrap();
    let local_listen_socket_addr = stream.local_addr().unwrap();
    let connection_id: Arc<Ulid> = Arc::new(Ulid::generate());
    let limits = hypercfg.clone();
    let hyper_service = service_fn(move |req| {
        let service_provider = service_provider.clone();
        let middleware_pipeline = middleware_pipeline.clone();
        let limits = limits.clone();
        let routing_service = routing_service.clone();
        let connection_id = connection_id.clone();
        let routing_info = routing_service.resolve(req.uri(), req.method());
        async move {
            let max_body_size = max_body_size_of(&routing_info, &limits);
            if let Some(status) = exceeds_limits(&req, &limits, max_body_size) {
                LOGGER::warn(format!("{} {} {} refused with {}", client_socket_addr, req.method(), req.uri(), status));
                // the request body is not read, an HTTP/1.x connection can not be reused
                return Ok::<_, Infallible>(HttpResponse::status_only(status, req.version() <= http::Version::HTTP_11));
            }
            let custom_req = HttpRequest::from_http(req, routing_info, max_body_size, client_socket_addr, local_listen_socket_addr, connection_id);

            // The response sends its head through `head_rx` as soon as the first byte is written or the response is complete
            let (custom_resp, head_rx) = HttpResponse::new_streaming();

            let http_context = HttpContext::new(custom_req, custom_resp, service_provider.create_scope());
            Ok::<_, Infallible>(HandleError::run(middleware_pipeline, http_context, head_rx).await)
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
    let io = TokioIo::new(stream);
    builder.serve_connection(io, hyper_service).await.map_err(std::io::Error::other)
}
