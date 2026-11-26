use crate::http_context::{HttpHeaderExt, HttpRequest};

use super::error::BindError;

/// Buffer once and leave the request cursor at the start for the next binder or the action.
pub(super) async fn read_body_for_binding(request: &mut HttpRequest) -> Result<&[u8], BindError> {
    request.enable_buffering().await.map_err(BindError::BodyRead)?;
    request.seek_body(0).map_err(BindError::BodyRead)?;
    Ok(request.buffered_body().expect("enable_buffering leaves the body buffered"))
}

/// A form binder accepts `application/x-www-form-urlencoded`, a missing or blank media type is accepted too.
pub(super) async fn read_form_text(request: &mut HttpRequest) -> Result<&str, BindError> {
    let media = content_type_media(request).unwrap_or_default();
    if !media.is_empty() && media != "application/x-www-form-urlencoded" {
        return Err(BindError::UnsupportedMediaType(media));
    }
    std::str::from_utf8(read_body_for_binding(request).await?).map_err(|_| BindError::InvalidUtf8)
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
