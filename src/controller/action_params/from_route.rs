use crate::http_context::HttpContext;

use super::{error::BindError, from_param_value::FromParamValue};

/// Binds a required value from a path parameter of the matched route.
///
/// The parameter name is the action argument name. The value is percent-decoded
/// (`/users/a%20b` binds `a b`) after the route matched, so an encoded `%2F` stays inside its segment.
/// Missing, unparsable or not valid UTF-8 values map to HTTP 404.
///
/// `T` is any type with [`FromParamValue`], which includes every `FromStr` type
/// (numbers, `String`, `uuid::Uuid`, `ulid::Ulid`, ...).
pub struct FromRoute<T>(pub T);

impl<T: FromParamValue> FromRoute<T> {
    /// Bind path parameter `name` into `T`.
    pub fn bind(name: &str, http_context: &HttpContext) -> Result<Self, BindError> {
        let value = http_context.request.routing_info.path_params.get(name)
            .ok_or_else(|| BindError::MissingRouteParam(name.to_owned()))?;
        let parse_error = |raw: &str| BindError::ParseRouteParam { name: name.to_owned(), value: raw.to_owned() };
        let raw = value.as_deref().map_err(|raw| parse_error(raw))?;
        T::from_param_value(raw).map(Self).map_err(|_| parse_error(raw))
    }
}
