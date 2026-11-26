use std::{any::TypeId, str::FromStr};

/// Parse a single path/query string value into `T`.
/// A type that cannot implement `FromStr` can implement this trait directly, as long as it does not
/// also implement `FromStr`.
///
/// # Flags
///
/// `bool` is the only type that accepts an empty value: `?verbose` and `?verbose=` both bind to `true`.
/// Read the parameter as `FromOptionQuery<bool>`, a missing parameter is `None` (`FromOptionQuery::<bool>::flag()` turns it into `false`).
/// Every other type fails to parse an empty value, except those whose `FromStr` accepts it (`String` gives `""`).
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
