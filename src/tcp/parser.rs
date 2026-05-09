use std::sync::Arc;

use crate::{
    Application,
    http_context::{
        RequestStream, ResponseStream,
        http_context::HttpContext,
        http_request::{HttpRequest, TcpReadStreamRef},
        http_response::{HttpResponse, TcpWriteStreamRef},
    },
    io::AsyncRead,
    logging::LOGGER,
    services::configuration::ApplicationConfiguration,
};
/// Parses the TCP stream to extract HTTP request information and prepares the HTTP response. return bool : Connection == keep-alive
pub(crate) async fn parse_tcp_stream_async(app: Arc<Application>, _socket_addr: std::net::SocketAddr, read_half: TcpReadStreamRef, write_half: TcpWriteStreamRef) -> std::io::Result<bool> {
    let mut request_stream = RequestStream::new(read_half);
    let response_stream = ResponseStream {
        written_size: 0,
        tcp_write_stream: write_half,
    };
    let first_token = request_stream.read_until_maxsize_async(&[b' ', b'\r', b'\n', b'\0'], 10).await?;
    let first_token_string = String::from_utf8_lossy(&first_token.0).into_owned();
    if first_token.0.is_empty() && !first_token.1 {
        LOGGER::error(format!("{:?}: Empty first token {:?}", _socket_addr, request_stream.read_until_maxsize_async(&[], 1024).await));
        return Ok(false);
    }
    if !["get", "post", "put", "delete", "head", "options", "patch"]
        .iter()
        .any(|method| method.to_string() == first_token_string.to_ascii_lowercase())
    {
        return Ok(false);
    }
    let (request, response) = parse_http_request_async(first_token_string, _socket_addr.ip(), app._config.clone(), request_stream, response_stream).await?;
    let mut http_context = HttpContext::new(request, response, app._config.clone(), app.service.clone());
    // Process the http_context with middlewares and handlers here
    // let arc_httpcontext: &'a mut HttpContext = Arc::new(crate::MutexAsync::new(http_context));
    app.call_middlewares_async(&mut http_context).await;
    let keep_alive = http_context.request.keep_alive;
    http_context.response.keep_alive = keep_alive;
    http_context.response.write_response_async().await;
    return Ok(keep_alive);
}
async fn parse_http_request_async(
    first_token: String,
    socket_ip: std::net::IpAddr,
    app_config: ApplicationConfiguration,
    request_stream: RequestStream,
    response_stream: ResponseStream,
) -> std::io::Result<(HttpRequest, HttpResponse)> {
    // This parser currently handles request line + headers and stores body as a view into the same raw buffer.
    let method = &first_token;
    let request = HttpRequest::from_raw(socket_ip, method, app_config, request_stream).await?;
    let response = HttpResponse::new(response_stream).await?;
    Ok((request, response))
}
