use crate::http_context::HttpContext;

use super::{error::BindError, from_param_value::FromParamValue};

/// Binds a required value from the query string.
///
/// The parameter name is the action argument name. Missing or unparsable values
/// map to HTTP 422.
pub struct FromQuery<T>(pub T);

impl<T: FromParamValue> FromQuery<T> {
    /// Bind query parameter `name` into `T`.
    pub fn bind(name: &str, http_context: &HttpContext) -> Result<Self, BindError> {
        let raw = http_context
            .request
            .routing_info
            .query_params
            .get(name)
            .ok_or_else(|| BindError::MissingQueryParam(name.to_string()))?;
        let value = T::from_param_value(raw).map_err(|_| BindError::ParseQueryParam {
            name: name.to_string(),
            value: raw.clone(),
        })?;
        Ok(FromQuery(value))
    }

    pub fn into_inner(self) -> T {
        self.0
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

/// Binds an optional value from the query string.
///
/// Missing or unparsable values become `None` (no error).
pub struct FromOptionQuery<T>(pub Option<T>);

impl<T: FromParamValue> FromOptionQuery<T> {
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
