use hyper::{Request, Response, StatusCode, service::service_fn};
use std::sync::Arc;
use tokio::net::TcpStream;

use crate::{
    Application,
    http_context::{HttpContext, http_request::HttpRequest, http_response::HttpResponse},
    logging::LOGGER,
};
use std::pin::Pin;
use http_body::Body as _;
pub(crate) async fn hyper_service(stream: TcpStream, app: Arc<Application>) -> std::io::Result<()> {
    let app_clone = app.clone();
    let service = service_fn(move |req| {
        let app = app_clone.clone();
        async move {
            // Inline request handling to avoid Body type ambiguity
            let method = req.method().clone();
            let uri = req.uri().clone();
            let version = req.version();
            LOGGER::info(format!("Hyper received {} {} {:?}", method, uri, version));

            // Read full body bytes by polling frames from the request body
            let mut body: hyper::body::Incoming = req.into_body();
            let mut whole_vec: Vec<u8> = Vec::new();
            loop {
                let frame_opt = futures_util::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await;
                match frame_opt {
                    Some(Ok(frame)) => {
                        if frame.is_data() {
                            if let Some(data) = frame.data_ref() {
                                whole_vec.extend_from_slice(&data);
                            }
                        } else if frame.is_trailers() {
                            break;
                        }
                    }
                    Some(Err(e)) => {
                        LOGGER::error(format!("Error reading request body: {}", e));
                        let response_body = http_body_util::Full::new(hyper::body::Bytes::from("Internal Server Error"));
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
            let mut custom_resp = HttpResponse::new_in_memory().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)).unwrap();

            // Build HttpContext and run middlewares/handlers
            let mut http_context = HttpContext::new(custom_req, custom_resp, app._config.clone(), app.service.clone());
            let start = std::time::Instant::now();
            app.call_middlewares_async(&mut http_context).await;

            // Convert internal response to http::Response<Vec<u8>> and then to hyper::Response<Body>
            let http_response = http_context.response.move_to_http_response();
            let mut builder = Response::builder().status(http_response.status());
            for (k, v) in http_response.headers().iter() {
                if let Ok(val) = v.to_str() {
                    builder = builder.header(k.as_str(), val);
                }
            }
            let body_vec = http_response.body().clone();
            let duration = start.elapsed();
            LOGGER::info(format!("Handled in {:.3} ms", duration.as_secs_f64() * 1000.0));
            // Use http-body-util Full to produce a body implementing http_body::Body
            let response_body = http_body_util::Full::new(hyper::body::Bytes::from(body_vec));
            let response = builder.body(response_body).unwrap_or_else(|e| {
                LOGGER::error(format!("Failed to build response: {}", e));
                Response::builder()
                    .status(500)
                    .body(http_body_util::Full::new(hyper::body::Bytes::from("Internal Server Error")))
                    .unwrap()
            });
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
