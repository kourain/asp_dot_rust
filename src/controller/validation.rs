use std::fmt;

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
        Self { field: field.into(), message: message.into() }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

impl std::error::Error for ValidationError {}

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
