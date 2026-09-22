use crate::controller::ActionResult;

impl ActionResult for &str {
    async fn get_body_async(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }
    async fn write_to_http_context_async(&self, http_context: &mut crate::http_context::HttpContext) {
        self.set_headers(http_context);
        *http_context.response.body_mut() = self.as_bytes().to_vec();
    }
    fn content_length(&self) -> usize {
        self.len()
    }
}
