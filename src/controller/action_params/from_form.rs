use crate::{
    http_context::HttpContext,
    services::routing::query_string::{decode_query_component, query_pairs},
};

use super::{
    body_reader::read_form_text,
    error::BindError,
    from_param_value::{FieldError, FromParamField},
};

/// Binds the URL-encoded form field named after the action argument using [`FromParamValue`](super::FromParamValue).
/// A missing required field or invalid value returns HTTP 422. [`FromOptionForm<T>`] accepts a missing field.
/// The last occurrence wins. A bare key has an empty value (`true` for `bool`). `+` decodes to a space and `%2B` to a plus.
///
/// Each binder searches the buffered form separately. Use [`Form<Dto>`](super::Form) to deserialize the whole form.
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
        Self::bind_with_default(name, http_context, None::<fn() -> T>).await.map(|(bound, _)| bound)
    }

    #[doc(hidden)]
    pub async fn bind_with_default<Marker>(name: &str, http_context: &mut HttpContext, default: Option<impl FnOnce() -> T>) -> Result<(Self, bool), BindError>
    where
        T: FromParamField<Marker>,
    {
        let text = read_form_text(&mut http_context.request).await?;
        let value = match query_pairs(text).rfind(|(key, _)| key == name) {
            Some((_, raw)) => Some(decode_query_component(raw).ok_or_else(|| BindError::ParseField {
                name: name.to_owned(),
                value: raw.to_owned(),
            })?),
            None => None,
        };
        if let (None, Some(default)) = (&value, default) {
            return Ok((Self(default()), true));
        }
        match T::from_param_field(value.as_deref()) {
            Ok(value) => Ok((Self(value), false)),
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
