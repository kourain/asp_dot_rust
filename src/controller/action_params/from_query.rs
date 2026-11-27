use crate::http_context::HttpContext;

use super::{
    error::BindError,
    from_param_value::{FieldError, FromParamField},
};

/// Binds one value from the query string. The parameter name is the action argument name.
///
/// - `FromQuery<T>` is a **required** parameter: missing, unparsable or not valid UTF-8 is HTTP 422.
/// - `FromQuery<Option<T>>`, written [`FromOptionQuery<T>`], is an **optional** parameter: missing is
///   `None`. A supplied value must parse, an unparsable one is HTTP 422 and never turns into `None`.
///
/// `T` is any type with [`FromParamValue`](super::FromParamValue), which includes every `FromStr` type
/// (numbers, `String`, `uuid::Uuid`, `ulid::Ulid`, ...).
///
/// # Warning
///
/// - **A repeated key keeps only its last value.** `?id=1&id=2` binds `2` and drops `1`
///   without an error. To collect all the values or to reject a repeated key, read
///   `http_context.request.uri.query()` yourself.
/// - **A key without a value is an empty value** (`?name` and `?name=` are the same).
///   `String` accepts it as `""`, numbers reject it with HTTP 422, `bool` reads it as `true`.
///   `FromOptionQuery<bool>` is the flag binder, see [`FromQuery::<Option<bool>>::flag`].
/// - Keys and values are decoded as form data: `+` is a space and `%2B` is a plus.
pub struct FromQuery<T>(pub T);

/// An optional query parameter, an alias of `FromQuery<Option<T>>`.
///
/// A missing parameter is `None`, see [`FromQuery`] for the warnings about repeated keys and flags.
/// It is a type alias, so it cannot be used as a constructor or a pattern, write `FromQuery(Some(1))` instead.
pub type FromOptionQuery<T> = FromQuery<Option<T>>;

impl<T> FromQuery<T> {
    /// Bind query parameter `name` into `T`, or into `Option<U>` for an optional parameter.
    ///
    /// `Marker` is inferred, it selects the required or the optional behaviour from `T`.
    pub fn bind<Marker>(name: &str, http_context: &HttpContext) -> Result<Self, BindError>
    where
        T: FromParamField<Marker>,
    {
        let raw = http_context.request.routing_info.query_params.get(name)
            .map(Result::as_deref).transpose()
            .map_err(|raw| BindError::ParseQueryParam { name: name.to_owned(), value: raw.to_owned() })?;
        match T::from_param_field(raw) {
            Ok(value) => Ok(Self(value)),
            Err(FieldError::Missing) => Err(BindError::MissingQueryParam(name.to_string())),
            Err(FieldError::Invalid) => Err(BindError::ParseQueryParam {
                name: name.to_string(),
                value: raw.unwrap_or_default().to_owned(),
            }),
        }
    }
}

impl FromQuery<Option<bool>> {
    /// The query parameter as a flag: `true` when it is present (`?verbose`, `?verbose=`, `?verbose=true`),
    /// `false` when it is absent or `?verbose=false`.
    pub fn flag(&self) -> bool {
        self.0 == Some(true)
    }
}
