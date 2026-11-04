use serde::de::DeserializeOwned;

use crate::http_context::HttpContext;

use super::{
    body_reader::{content_type_media, read_body_bytes},
    error::BindError,
};

/// Binds a value from `application/x-www-form-urlencoded` form data.
///
/// Content-Type mismatch → HTTP 422.
/// `FromForm<String>` reads the body as raw UTF-8 text (no serde).
pub struct FromForm<T>(pub T);

impl FromForm<String> {
    /// Read the form body as a UTF-8 string (no serde).
    pub async fn bind(http_context: &mut HttpContext) -> Result<Self, BindError> {
        let media = content_type_media(&http_context.request).unwrap_or_default();
        if !media.is_empty() && media != "application/x-www-form-urlencoded" {
            return Err(BindError::UnsupportedMediaType(media));
        }
        let bytes = read_body_bytes(&mut http_context.request).await?;
        let text = String::from_utf8(bytes).map_err(|_| BindError::InvalidUtf8)?;
        Ok(FromForm(text))
    }
}

impl<T: DeserializeOwned> FromForm<T> {
    /// Deserialize form-urlencoded body into `T` with serde.
    pub async fn bind_serde(http_context: &mut HttpContext) -> Result<Self, BindError> {
        let media = content_type_media(&http_context.request).unwrap_or_default();
        if !media.is_empty() && media != "application/x-www-form-urlencoded" {
            return Err(BindError::UnsupportedMediaType(media));
        }
        let bytes = read_body_bytes(&mut http_context.request).await?;
        let text = std::str::from_utf8(&bytes).map_err(|_| BindError::InvalidUtf8)?;
        let value =
            serde_urlencoded::from_str(text).map_err(|e| BindError::Deserialize(e.to_string()))?;
        Ok(FromForm(value))
    }
}

impl<T> FromForm<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for FromForm<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for FromForm<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
