use crate::http_context::http_request::HttpRequest;
use crate::http_context::HttpHeader;
use http_body_util::BodyExt;

use super::error::BindError;

/// Read the entire request body into a byte vector, respecting `max_body_size`.
///
/// Uses `BodyExt::frame` on `&mut Limited<Incoming>` so the body is not moved out of the request.
pub async fn read_body_bytes(request: &mut HttpRequest) -> Result<Vec<u8>, BindError> {
    let mut out = Vec::new();
    loop {
        match request.body.frame().await {
            Some(Ok(frame)) => {
                if let Ok(data) = frame.into_data() {
                    out.extend_from_slice(&data);
                }
            }
            Some(Err(e)) => return Err(BindError::BodyRead(e.to_string())),
            None => break,
        }
    }
    Ok(out)
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
