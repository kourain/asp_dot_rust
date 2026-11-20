use http::{HeaderMap, StatusCode};
use std::io;

use crate::http_context::HttpResponse;

#[async_trait::async_trait]
pub trait ActionResult: Send + Sync {
    /// Returns the HTTP status for this result.
    fn status_code(&self) -> StatusCode {
        StatusCode::OK
    }

    /// Writes headers to merge into the response.
    fn write_header(&self, _headers: &mut HeaderMap) {}

    /// Sets the buffered body or streams bytes directly through the response.
    /// If the response has not started, the router applies the result's status and headers first.
    async fn write_body_async(&self, _response: &mut HttpResponse) -> io::Result<()> {
        Ok(())
    }
}

impl<'a, T: ActionResult + 'a> From<T> for Box<dyn ActionResult + 'a> {
    fn from(result: T) -> Self {
        Box::new(result)
    }
}
