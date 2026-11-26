use crate::{
    http_context::HttpContext,
    services::routing::query_string::{decode_query_component, query_pairs},
};

use super::{
    body_reader::{check_form_media, read_body_for_binding},
    error::BindError,
    from_param_value::FromParamValue,
};

/// Binds **one field** of an `application/x-www-form-urlencoded` body.
///
/// The field name is the action argument name, like `FromQuery<T>`. Use [`Form`](super::Form)
/// to bind the whole form into a DTO.
///
/// `T` is any type with [`FromParamValue`] (every `FromStr` type: `String`, numbers, `bool`,
/// `uuid::Uuid`, ...). A missing or unparsable field, or a Content-Type mismatch, is HTTP 422.
///
/// # Warning
///
/// - **A repeated key keeps only its last value.** `a=1&a=2` binds `2`.
/// - **A key without a value is an empty value** (`a` and `a=` are the same): `String` gets `""`,
///   `bool` gets `true`, numbers fail.
/// - Keys and values are decoded as form data: `+` is a space and `%2B` is a plus.
/// - Each field binder searches the buffered form from the end. For many fields prefer `Form<Dto>`.
pub struct FromForm<T>(pub T);

impl<T: FromParamValue> FromForm<T> {
    /// Bind form field `name`.
    pub async fn bind(name: &str, http_context: &mut HttpContext) -> Result<Self, BindError> {
        check_form_media(&http_context.request)?;
        let bytes = read_body_for_binding(&mut http_context.request).await?;
        let text = std::str::from_utf8(bytes).map_err(|_| BindError::InvalidUtf8)?;
        let (_, raw) = query_pairs(text).rfind(|(key, _)| key == name).ok_or_else(|| BindError::MissingField(name.to_owned()))?;
        let value = decode_query_component(raw).ok_or_else(|| BindError::ParseField {
            name: name.to_owned(),
            value: raw.to_owned(),
        })?;
        T::from_param_value(&value).map(Self).map_err(|()| BindError::ParseField { name: name.to_owned(), value })
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
