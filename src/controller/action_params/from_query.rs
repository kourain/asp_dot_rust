use crate::controller::{
    ValidationError,
    validation::{QueryBounds, QueryNumber},
};
use crate::http_context::HttpContext;

use super::{error::BindError, from_param_value::FromParamValue};

/// Binds a required value from the query string.
///
/// The parameter name is the action argument name. Missing, unparsable or not valid UTF-8 values
/// map to HTTP 422.
///
/// `T` is any type with [`FromParamValue`], which includes every `FromStr` type
/// (numbers, `String`, `uuid::Uuid`, `ulid::Ulid`, ...).
///
/// # Warning
///
/// - **A repeated key keeps only its last value.** `?id=1&id=2` binds `2` and drops `1`
///   without an error. To collect all the values or to reject a repeated key, read
///   `http_context.request.uri.query()` yourself.
/// - **A key without a value is an empty value** (`?name` and `?name=` are the same).
///   `String` accepts it as `""`, numbers reject it with HTTP 422, `bool` reads it as `true`.
/// - Keys and values are decoded as form data: `+` is a space and `%2B` is a plus.
pub struct FromQuery<T>(pub T);

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

impl<T: FromParamValue> FromQuery<T> {
    /// Bind query parameter `name` into `T`.
    pub fn bind(name: &str, http_context: &HttpContext) -> Result<Self, BindError> {
        let routing_info = &http_context.request.routing_info;
        if let Some(raw) = routing_info.invalid_query_params.get(name) {
            return Err(BindError::ParseQueryParam {
                name: name.to_string(),
                value: raw.clone(),
            });
        }
        let raw = routing_info.query_params.get(name).ok_or_else(|| BindError::MissingQueryParam(name.to_string()))?;
        parse_query_value(name, raw).map(Self)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

fn parse_query_value<T: FromParamValue>(name: &str, raw: &str) -> Result<T, BindError> {
    T::from_param_value(raw).map_err(|_| BindError::ParseQueryParam {
        name: name.to_string(),
        value: raw.to_string(),
    })
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

/// Binds an optional value from the query string.
///
/// A missing parameter is `None`. A supplied value must parse: a value that cannot be parsed
/// into `T`, or that is not valid UTF-8 after decoding, is an error (HTTP 422) and never turns into `None`.
///
/// `T` is any type with [`FromParamValue`], which includes every `FromStr` type.
///
/// # Warning
///
/// - **A repeated key keeps only its last value.** `?id=1&id=2` binds `Some(2)` and drops `1`
///   without an error. To collect all the values or to reject a repeated key, read
///   `http_context.request.uri.query()` yourself.
/// - **A key without a value is an empty value** (`?name` and `?name=` are the same), it is
///   `Some("")` for `String`, an error for numbers and `Some(true)` for `bool`.
///   `FromOptionQuery<bool>` is the flag binder, see [`FromOptionQuery::<bool>::flag`].
/// - Keys and values are decoded as form data: `+` is a space and `%2B` is a plus.
pub struct FromOptionQuery<T>(pub Option<T>);

impl<T: FromParamValue> FromOptionQuery<T> {
    /// Bind optional query parameter `name` into `Option<T>`.
    ///
    /// `Ok(None)` when the parameter is absent, an error when it is present but cannot be parsed.
    pub fn bind(name: &str, http_context: &HttpContext) -> Result<Self, BindError> {
        let routing_info = &http_context.request.routing_info;
        if let Some(raw) = routing_info.invalid_query_params.get(name) {
            return Err(BindError::ParseQueryParam {
                name: name.to_string(),
                value: raw.clone(),
            });
        }
        match routing_info.query_params.get(name) {
            Some(raw) => parse_query_value(name, raw).map(|value| Self(Some(value))),
            None => Ok(Self(None)),
        }
    }

    #[doc(hidden)]
    #[deprecated(note = "`bind` reports an unparsable value as an error too, use it instead")]
    pub fn bind_if_present(name: &str, http_context: &HttpContext) -> Result<Self, BindError> {
        Self::bind(name, http_context)
    }

    pub fn into_inner(self) -> Option<T> {
        self.0
    }
}

impl FromOptionQuery<bool> {
    /// The query parameter as a flag: `true` when it is present (`?verbose`, `?verbose=`, `?verbose=true`),
    /// `false` when it is absent or `?verbose=false`.
    pub fn flag(&self) -> bool {
        self.0 == Some(true)
    }
}

impl<T> std::ops::Deref for FromOptionQuery<T> {
    type Target = Option<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for FromOptionQuery<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
