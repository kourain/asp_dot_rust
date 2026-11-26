use std::fmt;

/// `#[validate(rule(...))]` checks a query parameter before the controller action runs.
///
/// Use `#[validate(length(2))]` for exactly two Unicode scalar values or
/// `#[validate(length(1, 80))]` for an inclusive interval. Numeric parameters
/// support rules such as `#[validate(range(min = 1, max = 100))]`.
///
/// A missing `FromOptionQuery<T>` skips the rule. A supplied value must parse
/// and pass validation, or the router responds with HTTP 422.
pub fn validate(result: Result<(), ValidationError>) -> Result<(), super::BindError> {
    result.map_err(super::BindError::Validation)
}

/// The scalar passed to a query validation rule. The binder selects the implementation by type.
#[doc(hidden)]
pub trait QueryValidationTarget<Marker> {
    type Value;
    fn validation_value(&self) -> Option<&Self::Value>;
}

/// Implement on a request DTO to validate it after a controller action binds its body or form.
pub trait Validate {
    fn validate(&self) -> Result<(), ValidationError>;
}

/// A validation failure reported to the client as HTTP 422.
#[derive(Debug)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

/// Accepted values for a query rule. A single value denotes an exact match and
/// a pair denotes an inclusive interval. Named bounds can also be one-sided.
pub enum QueryBounds<T> {
    Exact(T),
    Between(T, T),
    Min(T),
    Max(T),
}

/// Numeric query values supported by the range rule.
pub trait QueryNumber: Copy + PartialOrd + fmt::Display {
    fn is_finite(self) -> bool;
}

macro_rules! impl_integer_query_number {
    ($($ty:ty),* $(,)?) => {$(
        impl QueryNumber for $ty {
            fn is_finite(self) -> bool { true }
        }
    )*};
}

impl_integer_query_number!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

impl QueryNumber for f32 {
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }
}

impl QueryNumber for f64 {
    fn is_finite(self) -> bool {
        f64::is_finite(self)
    }
}

impl<T> From<T> for QueryBounds<T> {
    fn from(value: T) -> Self {
        Self::Exact(value)
    }
}

impl<T> From<(T, T)> for QueryBounds<T> {
    fn from((min, max): (T, T)) -> Self {
        Self::Between(min, max)
    }
}

impl<T: PartialOrd + fmt::Display> QueryBounds<T> {
    pub(crate) fn check(&self, value: &T, field: &str, label: &str) -> Result<(), ValidationError> {
        let message = match self {
            Self::Exact(n) if value != n => format!("{label} must be exactly {n}"),
            Self::Between(min, max) if !(min <= value && value <= max) => format!("{label} must be between {min} and {max}"),
            Self::Min(min) if !(value >= min) => format!("{label} must be at least {min}"),
            Self::Max(max) if !(value <= max) => format!("{label} must be at most {max}"),
            _ => return Ok(()),
        };
        Err(ValidationError::new(field, message))
    }
}

// The route macro calls check() with the concrete DTO. Method lookup prefers
// the inherent method when T implements Validate and uses the trait otherwise.
#[doc(hidden)]
pub struct ValidationCheck<'a, T>(pub &'a T);

impl<T: Validate> ValidationCheck<'_, T> {
    pub fn check(&self) -> Result<(), ValidationError> {
        self.0.validate()
    }
}

#[doc(hidden)]
pub trait SkipValidation {
    fn check(&self) -> Result<(), ValidationError>;
}

impl<T> SkipValidation for ValidationCheck<'_, T> {
    fn check(&self) -> Result<(), ValidationError> {
        Ok(())
    }
}
