use serde::de::DeserializeOwned;

use crate::http_context::HttpContext;

use super::{
    body_fields::read_body_for_binding,
    body_reader::content_type_media,
    error::BindError,
};

/// Binds the **whole** request body into `T`.
///
/// Use [`FromBody`](super::FromBody) to bind a single field instead.
/// The body is read from its start and rewound afterwards, so other binders can read it too.
///
/// Supported `Content-Type` values:
/// - `application/json`, `text/json` → JSON (`serde_json`)
/// - `application/xml`, `text/xml` → XML (`quick-xml`)
///
/// When `Content-Type` is missing, JSON is assumed.
/// Mismatch or deserialize failure → HTTP 422.
///
/// `Body<String>` reads the body as raw UTF-8 text (no serde).
/// Other types use serde; primitives are not useful as the sole body type.
pub struct Body<T>(pub T);

impl Body<String> {
    /// Read the request body as a UTF-8 string (no serde).
    pub async fn bind(http_context: &mut HttpContext) -> Result<Self, BindError> {
        let media = content_type_media(&http_context.request)
            .unwrap_or_else(|| "text/plain".to_string());
        match media.as_str() {
            "application/json" | "text/json" | "application/xml" | "text/xml" | "text/plain" => {}
            other => return Err(BindError::UnsupportedMediaType(other.to_string())),
        }
        let bytes = read_body_for_binding(&mut http_context.request).await?;
        let text = String::from_utf8(bytes).map_err(|_| BindError::InvalidUtf8)?;
        Ok(Body(text))
    }
}

impl<T: DeserializeOwned> Body<T> {
    /// Read and deserialize the request body into `T` with serde (JSON / XML).
    pub async fn bind_serde(http_context: &mut HttpContext) -> Result<Self, BindError> {
        let media = content_type_media(&http_context.request)
            .unwrap_or_else(|| "application/json".to_string());
        let bytes = read_body_for_binding(&mut http_context.request).await?;

        let value = match media.as_str() {
            "application/json" | "text/json" => {
                serde_json::from_slice(&bytes).map_err(|e| BindError::Deserialize(e.to_string()))?
            }
            "application/xml" | "text/xml" => quick_xml::de::from_reader(bytes.as_slice())
                .map_err(|e| BindError::Deserialize(e.to_string()))?,
            other => return Err(BindError::UnsupportedMediaType(other.to_string())),
        };

        Ok(Body(value))
    }
}

impl<T> Body<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Body<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Body<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
