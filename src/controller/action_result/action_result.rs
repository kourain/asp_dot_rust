use crate::{controller::HttpContextRef, http_context::http_header::AspDotRustHttpHeader};
use core::future::Future;

pub trait ActionResult: Sync {
    fn status_code(&self) -> http::StatusCode {
        http::StatusCode::OK
    }
    fn get_body_async<'a>(&'a self) -> impl Future<Output = Vec<u8>> + Send;
    fn set_headers(&self, _http_context: &mut HttpContextRef) {
        let S = self.status_code();
        if S.as_u16() == 200 {
            *_http_context.response.status_mut() = self.status_code();
        }
        _http_context.response.headers_mut().set_content_length(self.content_length());
        _http_context.response.headers_mut().set_content_type(self.content_type());
    }
    fn write_to_http_context_async<'a>(&'a self, http_context: &mut HttpContextRef) -> impl Future<Output = ()> + Send {
        async move {
            self.set_headers(http_context);
            let body = self.get_body_async().await;
            *http_context.response.body_mut() = body;
        }
    }
    fn content_type(&self) -> &str {
        ""
    }
    fn content_length(&self) -> usize;
}
