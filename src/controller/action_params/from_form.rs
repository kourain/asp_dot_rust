use crate::http_context::HttpContext;

use super::{
    body_fields::{check_form_media, form_field, read_body_for_binding},
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
/// - Every field binder reads and parses the whole body again. For many fields prefer `Form<Dto>`.
pub struct FromForm<T>(pub T);

impl<T: FromParamValue> FromForm<T> {
    /// Bind form field `name`.
    pub async fn bind(name: &str, http_context: &mut HttpContext) -> Result<Self, BindError> {
        check_form_media(&http_context.request)?;
        let bytes = read_body_for_binding(&mut http_context.request).await?;
        let raw = form_field(&bytes, name)?;
        match T::from_param_value(&raw) {
            Ok(value) => Ok(Self(value)),
            Err(()) => Err(BindError::ParseField { name: name.to_string(), value: raw }),
        }
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
