use crate::controller::HttpContextRef;
use core::future::Future;

pub trait ActionResult: Sync {
    fn get_status_code(&self) -> http::StatusCode {
        http::StatusCode::OK
    }
    fn get_body_async<'a>(&'a self) -> impl Future<Output = Vec<u8>> + Send;
    fn write_to_http_context_async<'a>(&'a self, http_context: &mut HttpContextRef) -> impl Future<Output = ()> + Send {
        async move {
            let body = self.get_body_async().await;
            http_context.response.body = body;
        }
    }
    fn content_type(&self) -> &str {
        ""
    }
    fn content_length(&self) -> usize;
}
