use crate::http_context::HttpRequest;
use crate::http_context::HttpHeaderExt;

use super::error::BindError;

/// Read the request body from the current position to the end.
///
/// The body is buffered in memory (ASP.NET-style). After a successful read the position is at
/// the end; call `request.seek_body(0)` to read the same bytes again.
pub async fn read_body_bytes(request: &mut HttpRequest) -> Result<Vec<u8>, BindError> {
    request
        .body
        .read_to_end()
        .await
        .map_err(|e| BindError::BodyRead(e.to_string()))
}

/// Return the media type part of Content-Type (without parameters), lowercased.
pub fn content_type_media(request: &HttpRequest) -> Option<String> {
    request.headers.content_type().map(|ct| {
        ct.split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase()
    })
}
