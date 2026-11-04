use http::StatusCode;
use std::fmt;

/// Error returned when binding a request parameter fails.
#[derive(Debug)]
pub enum BindError {
    /// Path parameter is missing from the matched route.
    MissingRouteParam(String),
    /// Path parameter could not be parsed into the target type.
    ParseRouteParam { name: String, value: String },
    /// Required query parameter is missing.
    MissingQueryParam(String),
    /// Query parameter could not be parsed into the target type.
    ParseQueryParam { name: String, value: String },
    /// Request body could not be read (limit exceeded, I/O, or connection closed).
    BodyRead(String),
    /// `Content-Type` is missing or not supported for this binder.
    UnsupportedMediaType(String),
    /// Body bytes could not be decoded as UTF-8.
    InvalidUtf8,
    /// Deserialization failed (JSON, XML, or form).
    Deserialize(String),
}

impl BindError {
    /// HTTP status suggested for this bind failure.
    /// Route params → 404; query / body / form → 422; body read → 400.
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::MissingRouteParam(_) | Self::ParseRouteParam { .. } => StatusCode::NOT_FOUND,
            Self::MissingQueryParam(_)
            | Self::ParseQueryParam { .. }
            | Self::UnsupportedMediaType(_)
            | Self::InvalidUtf8
            | Self::Deserialize(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::BodyRead(_) => StatusCode::BAD_REQUEST,
        }
    }
}

impl fmt::Display for BindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRouteParam(name) => write!(f, "missing route parameter `{name}`"),
            Self::ParseRouteParam { name, value } => {
                write!(f, "cannot parse route parameter `{name}` from `{value}`")
            }
            Self::MissingQueryParam(name) => write!(f, "missing query parameter `{name}`"),
            Self::ParseQueryParam { name, value } => {
                write!(f, "cannot parse query parameter `{name}` from `{value}`")
            }
            Self::BodyRead(msg) => write!(f, "failed to read request body: {msg}"),
            Self::UnsupportedMediaType(ct) => write!(f, "unsupported Content-Type: {ct}"),
            Self::InvalidUtf8 => write!(f, "request body is not valid UTF-8"),
            Self::Deserialize(msg) => write!(f, "failed to deserialize body: {msg}"),
        }
    }
}

impl std::error::Error for BindError {}
