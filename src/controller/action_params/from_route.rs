use crate::http_context::HttpContext;

use super::{error::BindError, from_param_value::FromParamValue};

/// Binds a required value from a path parameter of the matched route.
///
/// The parameter name is the action argument name. Missing or unparsable values
/// map to HTTP 404.
pub struct FromRoute<T>(pub T);

impl<T: FromParamValue> FromRoute<T> {
    /// Bind path parameter `name` into `T`.
    pub fn bind(name: &str, http_context: &HttpContext) -> Result<Self, BindError> {
        let raw = http_context
            .request
            .routing_info
            .path_params
            .get(name)
            .ok_or_else(|| BindError::MissingRouteParam(name.to_string()))?;
        let value = T::from_param_value(raw).map_err(|_| BindError::ParseRouteParam {
            name: name.to_string(),
            value: raw.clone(),
        })?;
        Ok(FromRoute(value))
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for FromRoute<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for FromRoute<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
