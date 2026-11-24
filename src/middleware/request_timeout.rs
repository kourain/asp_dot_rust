use crate::{Application, configuration::RequestTimeoutConfiguration, dependency_injection::CfgRequire, http_context::HttpContext, logging::LOGGER, macros::middleware, middleware::MiddlewareNext};
use http::{HeaderValue, header};

pub struct RequestTimeoutMiddleware {
    next: MiddlewareNext,
    request_timeout_config: CfgRequire<RequestTimeoutConfiguration>,
}
#[middleware]
impl RequestTimeoutMiddleware {
    fn new(next: MiddlewareNext, request_timeout_cfg: CfgRequire<RequestTimeoutConfiguration>) -> RequestTimeoutMiddleware {
        Self {
            next,
            request_timeout_config: request_timeout_cfg,
        }
    }
    async fn invoke_async(&self, http_context: &mut HttpContext) {
        LOGGER::debug("RequestTimeoutMiddleware: Checking request timeout");
        let timeout = std::time::Duration::from_secs(self.request_timeout_config.timeout_seconds);
        if tokio::time::timeout(timeout, self.next.next_async(http_context)).await.is_err() {
            LOGGER::warn("Request timed out");
            if http_context.response.has_started() {
                // the status is already sent, the only way to tell the client is to break the stream
                http_context.abort();
            } else {
                let response = &mut http_context.response;
                response.status_code = http::StatusCode::REQUEST_TIMEOUT;
                response.body = b"Request Timeout".to_vec();
                // The handler's representation and framing no longer describe the timeout body.
                for name in [
                    header::CONTENT_ENCODING,
                    header::CONTENT_LENGTH,
                    header::CONTENT_RANGE,
                    header::CONTENT_DISPOSITION,
                    header::CONTENT_LANGUAGE,
                    header::CONTENT_LOCATION,
                    header::ETAG,
                    header::LAST_MODIFIED,
                    header::ACCEPT_RANGES,
                    header::TRANSFER_ENCODING,
                    header::TRAILER,
                    header::LOCATION,
                    header::EXPIRES,
                ] {
                    response.headers.remove(name);
                }
                for name in ["content-digest", "repr-digest", "digest", "content-md5"] {
                    response.headers.remove(name);
                }
                response.headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("text/plain; charset=utf-8"));
                response.headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
            }
        }
    }
}
impl Application {
    pub fn use_request_timeout(&mut self) -> &mut Self {
        self.add_middleware::<RequestTimeoutMiddleware>();
        self
    }
}
