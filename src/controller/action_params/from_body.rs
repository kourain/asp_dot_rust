use serde::de::DeserializeOwned;

use crate::http_context::HttpContext;

use super::{
    body_fields::{json_field, read_body_for_binding, xml_field},
    body_reader::content_type_media,
    error::BindError,
};

/// Binds **one field** of the request body.
///
/// The field name is the action argument name, like `FromQuery<T>`. Use [`Body`](super::Body)
/// to bind the whole body into a DTO (or to read it as raw text).
///
/// Supported `Content-Type` values (JSON is assumed when it is missing):
/// - `application/json`, `text/json`: a member of the top-level JSON object.
/// - `application/xml`, `text/xml`: a child element of the root element.
///
/// `T` is any serde `Deserialize` type. A missing field is HTTP 422, unless `T` is an `Option`
/// (then it is `None`). A wrong type, a body that is not an object or a Content-Type mismatch is HTTP 422 too.
///
/// # Warning
///
/// - **A repeated key keeps only its last value.**
/// - Every field binder reads and parses the whole body again. For many fields prefer `Body<Dto>`.
pub struct FromBody<T>(pub T);

impl<T: DeserializeOwned> FromBody<T> {
    /// Bind body field `name`.
    pub async fn bind(name: &str, http_context: &mut HttpContext) -> Result<Self, BindError> {
        let media = content_type_media(&http_context.request).unwrap_or_else(|| "application/json".to_string());
        let bytes = read_body_for_binding(&mut http_context.request).await?;
        let value = match media.as_str() {
            "application/json" | "text/json" => json_field(&bytes, name)?,
            "application/xml" | "text/xml" => xml_field(&bytes, name)?,
            other => return Err(BindError::UnsupportedMediaType(other.to_string())),
        };
        Ok(Self(value))
    }
}

impl<T> FromBody<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for FromBody<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for FromBody<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
