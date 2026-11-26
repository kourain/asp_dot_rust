use std::{any::TypeId, str::FromStr};

use super::marker::{OptionalField, RequiredField};

/// Parse a single path/query/form string value into `T`.
/// A type that cannot implement `FromStr` can implement this trait directly, as long as it does not
/// also implement `FromStr`.
///
/// `Option<T>` has no `FromParamValue`: a route parameter is always present, so `FromRoute<Option<T>>` is
/// rejected. Query and form fields accept `Option<T>` through [`FromParamField`].
///
/// # Flags
///
/// `bool` is the only type that accepts an empty value: `?verbose` and `?verbose=` both bind to `true`.
/// Read the parameter as `FromOptionQuery<bool>`, a missing parameter is `None` (`FromOptionQuery::<bool>::flag()` turns it into `false`).
/// Every other type fails to parse an empty value, except those whose `FromStr` accepts it (`String` gives `""`).
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be parsed from a single path, query or form value",
    label = "implement `FromStr` (or `FromParamValue`) for this type",
    note = "a route parameter is always present, so `FromRoute<Option<T>>` is not allowed. Use `FromOptionQuery<T>` / `FromOptionForm<T>` for an optional field"
)]
pub trait FromParamValue: Sized {
    fn from_param_value(value: &str) -> Result<Self, ()>;
}

impl<T: FromStr + 'static> FromParamValue for T {
    fn from_param_value(value: &str) -> Result<Self, ()> {
        if value.is_empty() && TypeId::of::<T>() == TypeId::of::<bool>() {
            return "true".parse().map_err(|_| ());
        }
        value.parse().map_err(|_| ())
    }
}

/// Why a query or form field could not be turned into the target type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldError {
    /// The field is not in the request and the target type requires it.
    Missing,
    /// The field is there but its value does not parse.
    Invalid,
}

/// Turn the raw value of a query or form field (`None` when the field is absent) into `T`.
///
/// - `T: FromParamValue` is a **required** field: absent is [`FieldError::Missing`].
/// - `Option<T>` is an **optional** field: absent is `None`. A value that is there must still parse,
///   it never turns into `None`.
///
/// The `Marker` parameter only keeps the two impls apart, it is always inferred.
/// `FromQuery<T>`, `FromForm<T>` and the aliases `FromOptionQuery<T>` / `FromOptionForm<T>` use this trait.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be bound from a single query or form field",
    label = "implement `FromStr` (or `FromParamValue`) for this type, or wrap it in `Option<_>`",
    note = "an optional field is `Option<T>` where `T` has `FromStr`"
)]
pub trait FromParamField<Marker>: Sized {
    fn from_param_field(raw: Option<&str>) -> Result<Self, FieldError>;
}

impl<T: FromParamValue> FromParamField<RequiredField> for T {
    fn from_param_field(raw: Option<&str>) -> Result<Self, FieldError> {
        T::from_param_value(raw.ok_or(FieldError::Missing)?).map_err(|()| FieldError::Invalid)
    }
}

impl<T: FromParamValue> FromParamField<OptionalField> for Option<T> {
    fn from_param_field(raw: Option<&str>) -> Result<Self, FieldError> {
        raw.map(T::from_param_value).transpose().map_err(|()| FieldError::Invalid)
    }
}
