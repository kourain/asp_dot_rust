use crate::controller::{
    ValidationError,
    validation::{QueryBounds, QueryNumber, QueryValidationTarget},
};
use crate::http_context::HttpContext;

use super::{
    error::BindError,
    from_param_value::{FieldError, FromParamField, OptionalField, RequiredField},
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

impl<T: FromParamField<RequiredField>> QueryValidationTarget<RequiredField> for FromQuery<T> {
    type Value = T;

    fn validation_value(&self) -> Option<&T> {
        Some(&self.0)
    }
}

impl<T> QueryValidationTarget<OptionalField> for FromQuery<Option<T>> {
    type Value = T;

    fn validation_value(&self) -> Option<&T> {
        self.0.as_ref()
    }
}

impl FromQuery<String> {
    /// Check the number of Unicode scalar values in the decoded query string.
    ///
    /// `#[validate(length(2))]` requires exactly two. `#[validate(length(1, 80))]`
    /// accepts one through eighty, inclusive. Use `length(min = 1)` or
    /// `length(max = 80)` for a one-sided limit.
    pub fn length(value: &str, field: &str, bounds: impl Into<QueryBounds<usize>>) -> Result<(), ValidationError> {
        bounds.into().check(&value.chars().count(), field, "length")
    }
}

impl<T: QueryNumber> FromQuery<T> {
    /// Check a finite numeric query value against inclusive bounds in its own type.
    ///
    /// `#[validate(range(min = -10, max = 10))]` accepts both endpoints.
    /// Supply just `min` or `max` for a one-sided range.
    pub fn range(value: &T, field: &str, bounds: impl Into<QueryBounds<T>>) -> Result<(), ValidationError> {
        if !value.is_finite() {
            return Err(ValidationError::new(field, "value must be finite"));
        }
        bounds.into().check(value, field, "value")
    }
}

impl<T> FromQuery<T> {
    /// Bind query parameter `name` into `T`, or into `Option<U>` for an optional parameter.
    ///
    /// `Marker` is inferred, it selects the required or the optional behaviour from `T`.
    pub fn bind<Marker>(name: &str, http_context: &HttpContext) -> Result<Self, BindError>
    where
        T: FromParamField<Marker>,
    {
        let routing_info = &http_context.request.routing_info;
        if let Some(raw) = routing_info.invalid_query_params.get(name) {
            return Err(BindError::ParseQueryParam {
                name: name.to_string(),
                value: raw.clone(),
            });
        }
        let raw = routing_info.query_params.get(name);
        match T::from_param_field(raw.map(String::as_str)) {
            Ok(value) => Ok(Self(value)),
            Err(FieldError::Missing) => Err(BindError::MissingQueryParam(name.to_string())),
            Err(FieldError::Invalid) => Err(BindError::ParseQueryParam {
                name: name.to_string(),
                value: raw.cloned().unwrap_or_default(),
            }),
        }
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl FromQuery<Option<bool>> {
    /// The query parameter as a flag: `true` when it is present (`?verbose`, `?verbose=`, `?verbose=true`),
    /// `false` when it is absent or `?verbose=false`.
    pub fn flag(&self) -> bool {
        self.0 == Some(true)
    }
}

impl<T> std::ops::Deref for FromQuery<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for FromQuery<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
