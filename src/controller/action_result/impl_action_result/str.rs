use crate::controller::ActionResult;
use core::future::Future;

impl ActionResult for &str {
    fn get_body_async<'a>(&'a self) -> impl Future<Output = Vec<u8>> + Send {
        async move { self.as_bytes().to_vec() }
    }
    fn write_to_http_context_async<'a>(&'a self, http_context: &mut crate::controller::HttpContextRef) -> impl Future<Output = ()> + Send {
        async move {
            self.set_headers(http_context);
            *http_context.response.body_mut() = self.as_bytes().to_vec();
        }
    }
    fn content_length(&self) -> usize {
        self.len()
    }
}
