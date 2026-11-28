use serde::de::DeserializeOwned;

use crate::http_context::HttpContext;

use super::{
    body_reader::{content_type_media, read_body_for_binding},
    error::BindError,
};

/// Deserializes the whole JSON or XML body into `T`. A missing Content-Type defaults to JSON.
/// `Body<String>` reads raw UTF-8 and also accepts `text/plain`. Invalid data or an unsupported media type returns HTTP 422.
/// The body stays available to the action and other binders. Use [`FromBody`](super::FromBody) for one field.
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
        let text = std::str::from_utf8(bytes).map_err(|_| BindError::InvalidUtf8)?.to_owned();
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
                serde_json::from_slice(bytes).map_err(|e| BindError::Deserialize(e.to_string()))?
            }
            "application/xml" | "text/xml" => quick_xml::de::from_reader(bytes)
                .map_err(|e| BindError::Deserialize(e.to_string()))?,
            other => return Err(BindError::UnsupportedMediaType(other.to_string())),
        };

        Ok(Body(value))
    }
}

impl<T: DeserializeOwned> super::BindSerdeBody for Body<T> {
    fn bind(ctx: &mut HttpContext) -> impl std::future::Future<Output = Result<Self, BindError>> + Send {
        Self::bind_serde(ctx)
    }
}
