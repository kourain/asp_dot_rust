use serde::de::DeserializeOwned;

use crate::http_context::HttpContext;

use super::{body_reader::read_form_text, error::BindError};

/// Deserializes the whole `application/x-www-form-urlencoded` body into `T`.
/// `Form<String>` reads the raw UTF-8 text. Invalid data or a conflicting Content-Type returns HTTP 422.
/// The body stays available to the action and other binders. Use [`FromForm`](super::FromForm) for one field.
pub struct Form<T>(pub T);

impl Form<String> {
    /// Read the form body as a UTF-8 string (no serde).
    pub async fn bind(http_context: &mut HttpContext) -> Result<Self, BindError> {
        let text = read_form_text(&mut http_context.request).await?.to_owned();
        Ok(Form(text))
    }
}

impl<T: DeserializeOwned> Form<T> {
    /// Deserialize form-urlencoded body into `T` with serde.
    pub async fn bind_serde(http_context: &mut HttpContext) -> Result<Self, BindError> {
        let text = read_form_text(&mut http_context.request).await?;
        let value =
            serde_urlencoded::from_str(text).map_err(|e| BindError::Deserialize(e.to_string()))?;
        Ok(Form(value))
    }
}

impl<T: DeserializeOwned> super::BindSerdeBody for Form<T> {
    fn bind(ctx: &mut HttpContext) -> impl std::future::Future<Output = Result<Self, BindError>> + Send {
        Self::bind_serde(ctx)
    }
}
