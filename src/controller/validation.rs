use std::fmt;

/// `#[validate(rule(...))]` checks a bound action field before the controller action runs.
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

/// Built-in action field rules. Applications can add associated functions through a local extension trait.
pub struct ControllerActionFieldValidate;

impl ControllerActionFieldValidate {
    /// Check the number of Unicode scalar values in the decoded string.
    ///
    /// `#[validate(length(2))]` requires exactly two. `#[validate(length(1, 80))]`
    /// accepts one through eighty, inclusive. Use `length(min = 1)` or
    /// `length(max = 80)` for a one-sided limit.
    pub fn length(value: &str, field: &str, bounds: impl Into<ValidateBounds<usize>>) -> Result<(), ValidationError> {
        bounds.into().check(&value.chars().count(), field, "length")
    }

    /// Check a finite numeric value against inclusive bounds in its own type.
    ///
    /// `#[validate(range(min = -10, max = 10))]` accepts both endpoints.
    /// Supply just `min` or `max` for a one-sided range.
    pub fn range<T: Number>(value: &T, field: &str, bounds: impl Into<ValidateBounds<T>>) -> Result<(), ValidationError> {
        if !value.is_finite() {
            return Err(ValidationError::new(field, "value must be finite"));
        }
        bounds.into().check(value, field, "value")
    }

    /// Accept a finite number at or above the inclusive lower bound.
    pub fn min<T: Number>(value: &T, field: &str, min: T) -> Result<(), ValidationError> {
        Self::range(value, field, ValidateBounds::Min(min))
    }

    /// Accept a finite number at or below the inclusive upper bound.
    pub fn max<T: Number>(value: &T, field: &str, max: T) -> Result<(), ValidationError> {
        Self::range(value, field, ValidateBounds::Max(max))
    }
}

// Resolved at the concrete action parameter, including type aliases and serde-only DTOs.
#[doc(hidden)]
pub struct FieldValue<'a, T>(pub &'a T);

impl<'a, T> FieldValue<'a, Option<T>> {
    pub fn present(&self) -> Option<&'a T> { self.0.as_ref() }
}

#[doc(hidden)]
pub trait RequiredFieldValue {
    type Value;
    fn present(&self) -> Option<&Self::Value>;
}

impl<T> RequiredFieldValue for FieldValue<'_, T> {
    type Value = T;
    fn present(&self) -> Option<&T> { Some(self.0) }
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
pub enum ValidateBounds<T> {
    Exact(T),
    Between(T, T),
    Min(T),
    Max(T),
}

/// Numeric query values supported by the range rule.
pub trait Number: Copy + PartialOrd + PartialEq + fmt::Display {
    fn is_finite(self) -> bool;
}

macro_rules! impl_integer_query_number {
    ($($ty:ty),* $(,)?) => {$(
        impl Number for $ty {
            fn is_finite(self) -> bool { true }
        }
    )*};
}

impl_integer_query_number!(i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize);

impl Number for f32 {
    fn is_finite(self) -> bool {
        f32::is_finite(self)
    }
}

impl Number for f64 {
    fn is_finite(self) -> bool {
        f64::is_finite(self)
    }
}

impl<T> From<T> for ValidateBounds<T> {
    fn from(value: T) -> Self {
        Self::Exact(value)
    }
}

impl<T> From<(T, T)> for ValidateBounds<T> {
    fn from((min, max): (T, T)) -> Self {
        Self::Between(min, max)
    }
}

impl<T: PartialOrd + fmt::Display> ValidateBounds<T> {
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
