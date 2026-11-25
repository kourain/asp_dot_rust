use crate::controller::{
    ValidationError,
    validation::{QueryBounds, QueryNumber},
};
use crate::http_context::HttpContext;

use super::{error::BindError, from_param_value::FromParamValue};

/// Binds a required value from the query string.
///
/// The parameter name is the action argument name. Missing or unparsable values
/// map to HTTP 422.
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
        let raw = http_context.request.routing_info.query_params.get(name).ok_or_else(|| BindError::MissingQueryParam(name.to_string()))?;
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
/// Missing values become `None`. An unparsable value becomes `None` by default
/// and produces HTTP 422 when the action parameter has a validation rule.
pub struct FromOptionQuery<T>(pub Option<T>);

impl<T: FromParamValue> FromOptionQuery<T> {
    /// Used by validated action parameters. A supplied value must parse successfully.
    pub fn bind_if_present(name: &str, http_context: &HttpContext) -> Result<Self, BindError> {
        if let Some(raw) = http_context.request.routing_info.query_params.get(name) {
            return parse_query_value(name, raw).map(|value| Self(Some(value)));
        }
        // Routing omits pairs with invalid UTF-8. A supplied but undecodable field
        // is a parse error for a validated optional parameter.
        if let Some(query) = http_context.request.uri.query().filter(|q| q.contains('%')) {
            for (key, value) in query.split('&').filter_map(|pair| pair.split_once('=')) {
                if urlencoding::decode(key).as_deref() == Ok(name) && urlencoding::decode(value).is_err() {
                    return Err(BindError::ParseQueryParam {
                        name: name.to_string(),
                        value: value.to_string(),
                    });
                }
            }
        }
        Ok(Self(None))
    }

    /// Bind optional query parameter `name` into `Option<T>`.
    pub fn bind(name: &str, http_context: &HttpContext) -> Self {
        let Some(raw) = http_context.request.routing_info.query_params.get(name) else {
            return FromOptionQuery(None);
        };
        match T::from_param_value(raw) {
            Ok(value) => FromOptionQuery(Some(value)),
            Err(()) => FromOptionQuery(None),
        }
    }

    pub fn into_inner(self) -> Option<T> {
        self.0
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
