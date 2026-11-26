use serde::de::DeserializeOwned;

use crate::http_context::HttpContext;

use super::{
    body_fields::{check_form_media, read_body_for_binding},
    error::BindError,
};

/// Binds the **whole** `application/x-www-form-urlencoded` body into `T`.
///
/// Use [`FromForm`](super::FromForm) to bind a single field instead.
///
/// Content-Type mismatch → HTTP 422.
/// `Form<String>` reads the body as raw UTF-8 text (no serde).
/// The body is read from its start and rewound afterwards, so other binders can read it too.
pub struct Form<T>(pub T);

impl Form<String> {
    /// Read the form body as a UTF-8 string (no serde).
    pub async fn bind(http_context: &mut HttpContext) -> Result<Self, BindError> {
        check_form_media(&http_context.request)?;
        let bytes = read_body_for_binding(&mut http_context.request).await?;
        let text = String::from_utf8(bytes).map_err(|_| BindError::InvalidUtf8)?;
        Ok(Form(text))
    }
}

impl<T: DeserializeOwned> Form<T> {
    /// Deserialize form-urlencoded body into `T` with serde.
    pub async fn bind_serde(http_context: &mut HttpContext) -> Result<Self, BindError> {
        check_form_media(&http_context.request)?;
        let bytes = read_body_for_binding(&mut http_context.request).await?;
        let text = std::str::from_utf8(&bytes).map_err(|_| BindError::InvalidUtf8)?;
        let value =
            serde_urlencoded::from_str(text).map_err(|e| BindError::Deserialize(e.to_string()))?;
        Ok(Form(value))
    }
}

impl<T> Form<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Form<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Form<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
