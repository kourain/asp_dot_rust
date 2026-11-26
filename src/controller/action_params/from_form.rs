use crate::{
    http_context::HttpContext,
    services::routing::query_string::{decode_query_component, query_pairs},
};

use super::{
    body_reader::{check_form_media, read_body_for_binding},
    error::BindError,
    from_param_value::{FieldError, FromParamField},
};

/// Binds **one field** of an `application/x-www-form-urlencoded` body.
///
/// The field name is the action argument name, like `FromQuery<T>`. Use [`Form`](super::Form)
/// to bind the whole form into a DTO.
///
/// - `FromForm<T>` is a **required** field: missing, unparsable or not valid UTF-8 is HTTP 422.
/// - `FromForm<Option<T>>`, written [`FromOptionForm<T>`], is an **optional** field: missing is `None`.
///   A value that is there must parse, it never turns into `None`.
///
/// `T` is any type with [`FromParamValue`](super::FromParamValue) (every `FromStr` type: `String`,
/// numbers, `bool`, `uuid::Uuid`, ...). A Content-Type mismatch is HTTP 422.
///
/// # Warning
///
/// - **A repeated key keeps only its last value.** `a=1&a=2` binds `2`.
/// - **A key without a value is an empty value** (`a` and `a=` are the same): `String` gets `""`,
///   `bool` gets `true`, numbers fail.
/// - Keys and values are decoded as form data: `+` is a space and `%2B` is a plus.
/// - Each field binder searches the buffered form from the end. For many fields prefer `Form<Dto>`.
pub struct FromForm<T>(pub T);

/// An optional form field, an alias of `FromForm<Option<T>>`.
///
/// It is a type alias, so it cannot be used as a constructor or a pattern, write `FromForm(Some(1))` instead.
pub type FromOptionForm<T> = FromForm<Option<T>>;

impl<T> FromForm<T> {
    /// Bind form field `name` into `T`, or into `Option<U>` for an optional field.
    ///
    /// `Marker` is inferred, it selects the required or the optional behaviour from `T`.
    pub async fn bind<Marker>(name: &str, http_context: &mut HttpContext) -> Result<Self, BindError>
    where
        T: FromParamField<Marker>,
    {
        check_form_media(&http_context.request)?;
        let bytes = read_body_for_binding(&mut http_context.request).await?;
        let text = std::str::from_utf8(bytes).map_err(|_| BindError::InvalidUtf8)?;
        let value = match query_pairs(text).rfind(|(key, _)| key == name) {
            Some((_, raw)) => Some(decode_query_component(raw).ok_or_else(|| BindError::ParseField {
                name: name.to_owned(),
                value: raw.to_owned(),
            })?),
            None => None,
        };
        match T::from_param_field(value.as_deref()) {
            Ok(value) => Ok(Self(value)),
            Err(FieldError::Missing) => Err(BindError::MissingField(name.to_owned())),
            Err(FieldError::Invalid) => Err(BindError::ParseField { name: name.to_owned(), value: value.unwrap_or_default() }),
        }
    }
}

impl FromForm<Option<bool>> {
    /// The form field as a flag: `true` when it is present (`flag`, `flag=`, `flag=true`),
    /// `false` when it is absent or `flag=false`.
    pub fn flag(&self) -> bool {
        self.0 == Some(true)
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
