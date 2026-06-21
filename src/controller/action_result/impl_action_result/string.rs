use crate::{controller::ActionResult, http_context::http_header::AspDotRustHttpHeader};
use core::future::Future;

impl ActionResult for String {
    fn get_body_async<'a>(&'a self) -> impl Future<Output = Vec<u8>> + Send {
        async move { self.as_bytes().to_vec() }
    }
    fn write_to_http_context_async<'a>(&'a self, http_context: &mut crate::controller::HttpContextRef) -> impl Future<Output = ()> + Send {
        async move {
            http_context.response.status_code = self.status_code();
            http_context.response.headers.set_content_length(self.content_length());
            http_context.response.headers.set_content_type(self.content_type());
            http_context.response.write_async(self.as_bytes()).await;
        }
    }
    fn content_length(&self) -> usize {
        self.len()
    }
}
