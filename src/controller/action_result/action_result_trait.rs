use http::HeaderMap;

use crate::{http_context::HttpContext, http_context::HttpHeaderExt};

#[async_trait::async_trait]
pub trait ActionResult: Send + Sync {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::OK
    }
    async fn get_body_async(&self) -> Vec<u8>;
    fn set_headers(&self, http_context: &mut HttpContext) {
        let response = &mut http_context.response;
        response.status_code = self.status_code();
        response.headers.extend(self.headers());
        response.headers.set_content_length(self.content_length());
        let content_type = self.content_type();
        if !content_type.is_empty() {
            response.headers.set_content_type(content_type);
        }
    }
    async fn write_to_http_context_async(&self, http_context: &mut HttpContext) {
        self.set_headers(http_context);
        http_context.response.body = self.get_body_async().await;
    }
    /// Additional headers to apply to the response.
    fn headers(&self) -> HeaderMap {
        HeaderMap::new()
    }
    fn content_type(&self) -> &str {
        ""
    }
    fn content_length(&self) -> usize;
}

impl<'a, T: ActionResult + 'a> From<T> for Box<dyn ActionResult + 'a> {
    fn from(result: T) -> Self {
        Box::new(result)
    }
}
