use std::sync::Arc;

use crate::{
    Application, extensions::Str, http_context::{
        RequestStream, ResponseStream,
        http_context::HttpContext,
        http_request::{HttpRequest, TcpReadStreamRef},
        http_response::{HttpResponse, TcpWriteStreamRef},
    }, io::AsyncRead, logging::LOGGER, services::configuration::ApplicationConfiguration
};
/// Parses the TCP stream to extract HTTP request information and prepares the HTTP response. return bool : Connection == keep-alive
pub(crate) async fn parse_tcp_stream_async(app: Arc<Application>, _socket_addr: std::net::SocketAddr, read_half: TcpReadStreamRef, write_half: TcpWriteStreamRef) -> std::io::Result<bool> {
    let mut request_stream = RequestStream::new(read_half);
    let response_stream = ResponseStream::new(write_half);
    let first_token = request_stream.read_until_maxsize_async(&[b' ', b'\r', b'\n'], 10).await?;
    if first_token.0.is_empty() && !first_token.1 {
        LOGGER::error(format!("{:?}: Empty first token {:?}", _socket_addr, request_stream.read_until_maxsize_async(&[], 1024).await));
        return Ok(false);
    }
    let method = http::Method::from_bytes(&first_token.0).map_err(|e| {
        LOGGER::error(format!("{:?}: Invalid HTTP method: {:?}, error: {}", _socket_addr, first_token.0, e));
        std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid HTTP method")
    })?;
    let (request, response) = parse_http_request_async(method, _socket_addr.ip(), app._config.clone(), request_stream, response_stream).await?;
    let mut http_context = HttpContext::new(request, response, app._config.clone(), app.service.clone());
    // Process the http_context with middlewares and handlers here
    let keep_alive = http_context.request.keep_alive;
    http_context.response.keep_alive = keep_alive;
    let start_process_time = std::time::Instant::now();
    app.call_middlewares_async(&mut http_context).await;
    http_context.response.write_response_async().await;
    let duration = start_process_time.elapsed();
    LOGGER::info(format!(
        "{}: {} {} {}, keep_alive={}, taken {:.9} ms",
        http_context.request.client_addr,
        http_context.request.method,
        http_context.request.path,
        http_context.request.version.as_str(),
        http_context.request.keep_alive,
        duration.as_secs_f64() * 1000.0
    ));
    return Ok(keep_alive);
}
async fn parse_http_request_async(
    method: http::Method,
    socket_ip: std::net::IpAddr,
    app_config: ApplicationConfiguration,
    request_stream: RequestStream,
    response_stream: ResponseStream,
) -> std::io::Result<(HttpRequest, HttpResponse)> {
    // This parser currently handles request line + headers and stores body as a view into the same raw buffer.
    let request = HttpRequest::from_raw(socket_ip, method, app_config, request_stream).await?;
    let response = HttpResponse::new(response_stream).await?;
    Ok((request, response))
}
