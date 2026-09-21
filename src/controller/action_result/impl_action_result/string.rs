use crate::{controller::ActionResult, http_context::http_header::AspDotRustHttpHeader};

impl ActionResult for String {
    async fn get_body_async(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    async fn write_to_http_context_async(&self, http_context: &mut crate::http_context::HttpContext) {
        http_context.response.status_code = self.status_code();
        http_context.response.headers.set_content_length(self.content_length());
        http_context.response.headers.set_content_type(self.content_type());
        http_context.response.write_async(self.as_bytes()).await;
    }
    fn content_length(&self) -> usize {
        self.len()
    }
}
