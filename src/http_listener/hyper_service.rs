use hyper::{Response, service::service_fn, body::Bytes};
use http_body::Body as _;
use http_body_util::channel::Channel;
use std::convert::Infallible;
use std::sync::Arc;
use tokio::net::TcpStream;

use crate::{
    Application,
    http_context::{HttpContext, http_request::HttpRequest, http_response::HttpResponse},
    logging::LOGGER,
};
use std::pin::Pin;

/// Maximum size to buffer request body in memory (10MB)
const MAX_BUFFER_SIZE: u64 = 10 * 1024 * 1024;

/// Chunk size for streaming response body (64KB per chunk)
const RESPONSE_CHUNK_SIZE: usize = 64 * 1024;

/// Create a streaming response body from a Vec<u8>, sending chunks progressively
fn create_streaming_body(body_vec: Vec<u8>) -> Channel<Bytes, Infallible> {
    let (mut sender, body) = Channel::<Bytes, Infallible>::new(4);
    
    if body_vec.is_empty() {
        drop(sender);
        return body;
    }
    
    // Spawn task to send chunks progressively
    tokio::spawn(async move {
        for chunk in body_vec.chunks(RESPONSE_CHUNK_SIZE) {
            if let Err(e) = sender.send_data(Bytes::copy_from_slice(chunk)).await {
                LOGGER::warn(format!("Error sending response chunk: {:?}", e));
                break;
            }
        }
        drop(sender); // Signal end of body
    });
    
    body
}

pub(crate) async fn hyper_service(stream: TcpStream, app: Arc<Application>) -> std::io::Result<()> {
    let app_clone = app.clone();
    let service = service_fn(move |req| {
        let app = app_clone.clone();
        async move {
            let start = std::time::Instant::now();
            // Extract request metadata before consuming the body
            let method = req.method().clone();
            let uri = req.uri().clone();
            let version = req.version();
            let headers_http = req.headers().clone();
            let content_length: u64 = headers_http
                .get(hyper::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            LOGGER::info(format!("Hyper received {} {} {:?} (Content-Length: {})", method, uri, version, content_length));

            // Read full body bytes by polling frames from the request body
            let mut body: hyper::body::Incoming = req.into_body();
            let mut whole_vec: Vec<u8> = Vec::new();
            let mut total_read: u64 = 0;

            loop {
                let frame_opt = futures_util::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await;
                match frame_opt {
                    Some(Ok(frame)) => {
                        if frame.is_data() {
                            if let Some(data) = frame.data_ref() {
                                total_read += data.len() as u64;
                                
                                // Safety check: don't buffer more than MAX_BUFFER_SIZE
                                if total_read > MAX_BUFFER_SIZE {
                                    LOGGER::warn(format!("Request body exceeded max buffer size ({}MB)", MAX_BUFFER_SIZE / (1024 * 1024)));
                                    let response_body = create_streaming_body(Vec::new());
                                    let resp = Response::builder().status(413).body(response_body).unwrap();
                                    return Ok::<_, hyper::Error>(resp);
                                }
                                
                                whole_vec.extend_from_slice(&data);
                            }
                        } else if frame.is_trailers() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        LOGGER::error(format!("Error reading request body: {}", e));
                        let response_body = create_streaming_body(Vec::new());
                        let resp = Response::builder().status(500).body(response_body).unwrap();
                        return Ok::<_, hyper::Error>(resp);
                    }
                    None => break,
                }
            }

            // Build http::Request<Vec<u8>> and convert to our HttpRequest
            let http_req = http::Request::builder()
                .method(method.clone())
                .uri(uri.clone())
                .version(version)
                .body(whole_vec.clone())
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
                .unwrap();
            let mut custom_req = HttpRequest::from_http(http_req);
            custom_req.body = whole_vec.clone();

            // Create an in-memory response to run through existing pipeline
            let custom_resp = match HttpResponse::new_in_memory().await {
                Ok(r) => r,
                Err(e) => {
                    LOGGER::error(format!("Error creating in-memory response: {}", e));
                    let response_body = create_streaming_body(Vec::new());
                    let resp = Response::builder().status(500).body(response_body).unwrap();
                    return Ok::<_, hyper::Error>(resp);
                }
            };

            // Build HttpContext and run middlewares/handlers
            let mut http_context = HttpContext::new(custom_req, custom_resp, app._config.clone(), app.service.clone());
            app.call_middlewares_async(&mut http_context).await;

            // Convert internal response to http::Response<Vec<u8>> and then to hyper::Response<Body>
            let http_response = http_context.response.move_to_http_response();
            let mut builder = Response::builder().status(http_response.status());
            
            // Copy response headers
            for (k, v) in http_response.headers().iter() {
                if let Ok(val) = v.to_str() {
                    builder = builder.header(k.as_str(), val);
                }
            }
            
            let body_vec = http_response.body().clone();
            let body_len = body_vec.len();
            
            // Stream response body chunk-by-chunk (64KB per chunk)
            let response_body = create_streaming_body(body_vec);
            let response = builder.body(response_body).unwrap_or_else(|e| {
                LOGGER::error(format!("Failed to build response: {}", e));
                Response::builder().status(500).body(create_streaming_body(Vec::new())).unwrap()
            });
            let duration = start.elapsed();
            LOGGER::info(format!("Handled in {:.3} ms, response size: {} bytes", duration.as_secs_f64() * 1000.0, body_len));
            Ok::<_, hyper::Error>(response)
        }
    });

    use hyper_util::rt::TokioIo;
    use hyper_util::server::conn::auto as auto_conn;

    let builder = auto_conn::Builder::new(hyper_util::rt::TokioExecutor::new());
    // wrap the tokio TcpStream so hyper-util can use hyper RT traits
    let io = TokioIo::new(stream);
    builder.serve_connection(io, service).await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
}
