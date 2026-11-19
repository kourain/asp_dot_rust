//! HTTP results returned by controller actions.
//!
//! Results with a body accept any `AsRef<[u8]> + Send + Sync` value, such as
//! `&str`, `String`, or `Vec<u8>`. Use `Ok("")` for an empty HTTP 200 response.

use http::{
    HeaderMap, HeaderValue, StatusCode,
    header::{InvalidHeaderValue, LOCATION},
};

use super::ActionResult;

macro_rules! body_results {
    ($($(#[$meta:meta])* $name:ident => $status:ident),+ $(,)?) => {
        $(
            $(#[$meta])*
            #[derive(Debug)]
            pub struct $name<T>(pub T);

            #[async_trait::async_trait]
            impl<T: AsRef<[u8]> + Send + Sync> ActionResult for $name<T> {
                fn status_code(&self) -> StatusCode {
                    StatusCode::$status
                }

                async fn get_body_async(&self) -> Vec<u8> {
                    self.0.as_ref().to_vec()
                }

                fn content_length(&self) -> usize {
                    self.0.as_ref().len()
                }
            }
        )+
    };
}

body_results! {
    /// HTTP 200 OK.
    Ok => OK,
    /// HTTP 201 Created.
    Created => CREATED,
    /// HTTP 202 Accepted.
    Accepted => ACCEPTED,
    /// HTTP 400 Bad Request.
    BadRequest => BAD_REQUEST,
    /// HTTP 401 Unauthorized.
    Unauthorized => UNAUTHORIZED,
    /// HTTP 403 Forbidden.
    Forbidden => FORBIDDEN,
    /// HTTP 404 Not Found.
    NotFound => NOT_FOUND,
    /// HTTP 409 Conflict.
    Conflict => CONFLICT,
    /// HTTP 410 Gone.
    Gone => GONE,
    /// HTTP 415 Unsupported Media Type.
    UnsupportedMediaType => UNSUPPORTED_MEDIA_TYPE,
    /// HTTP 422 Unprocessable Entity.
    UnprocessableEntity => UNPROCESSABLE_ENTITY,
    /// HTTP 429 Too Many Requests.
    TooManyRequests => TOO_MANY_REQUESTS,
    /// HTTP 500 Internal Server Error.
    InternalServerError => INTERNAL_SERVER_ERROR,
    /// HTTP 503 Service Unavailable.
    ServiceUnavailable => SERVICE_UNAVAILABLE,
}

impl<T> Created<T> {
    /// Adds a `Location` header identifying the created resource.
    pub fn with_location(self, location: impl AsRef<str>) -> Result<CreatedAt<T>, InvalidHeaderValue> {
        HeaderValue::from_str(location.as_ref()).map(|location| CreatedAt { body: self.0, location })
    }
}

fn location_headers(location: &HeaderValue) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(LOCATION, location.clone());
    headers
}

/// HTTP 201 Created with a `Location` header. Construct with [`Created::with_location`].
#[derive(Debug)]
pub struct CreatedAt<T> {
    body: T,
    location: HeaderValue,
}

#[async_trait::async_trait]
impl<T: AsRef<[u8]> + Send + Sync> ActionResult for CreatedAt<T> {
    fn status_code(&self) -> StatusCode {
        StatusCode::CREATED
    }

    async fn get_body_async(&self) -> Vec<u8> {
        self.body.as_ref().to_vec()
    }

    fn headers(&self) -> HeaderMap {
        location_headers(&self.location)
    }

    fn content_length(&self) -> usize {
        self.body.as_ref().len()
    }
}

macro_rules! redirects {
    ($($(#[$meta:meta])* $name:ident => $status:ident),+ $(,)?) => {
        $(
            $(#[$meta])*
            #[derive(Debug)]
            pub struct $name(HeaderValue);

            impl $name {
                /// Creates a redirect with the given `Location` header value.
                pub fn new(location: impl AsRef<str>) -> Result<Self, InvalidHeaderValue> {
                    HeaderValue::from_str(location.as_ref()).map(Self)
                }
            }

            #[async_trait::async_trait]
            impl ActionResult for $name {
                fn status_code(&self) -> StatusCode {
                    StatusCode::$status
                }

                async fn get_body_async(&self) -> Vec<u8> {
                    Vec::new()
                }

                fn headers(&self) -> HeaderMap {
                    location_headers(&self.0)
                }

                fn content_length(&self) -> usize {
                    0
                }
            }
        )+
    };
}

redirects! {
    /// HTTP 301 Moved Permanently.
    MovedPermanently => MOVED_PERMANENTLY,
    /// HTTP 302 Found.
    Redirect => FOUND,
    /// HTTP 303 See Other.
    SeeOther => SEE_OTHER,
    /// HTTP 307 Temporary Redirect.
    TemporaryRedirect => TEMPORARY_REDIRECT,
    /// HTTP 308 Permanent Redirect.
    PermanentRedirect => PERMANENT_REDIRECT,
}

/// HTTP 204 No Content.
#[derive(Debug)]
pub struct NoContent;

#[async_trait::async_trait]
impl ActionResult for NoContent {
    fn status_code(&self) -> StatusCode {
        StatusCode::NO_CONTENT
    }

    async fn get_body_async(&self) -> Vec<u8> {
        Vec::new()
    }

    fn content_length(&self) -> usize {
        0
    }
}

/// HTTP 304 Not Modified.
#[derive(Debug)]
pub struct NotModified;

#[async_trait::async_trait]
impl ActionResult for NotModified {
    fn status_code(&self) -> StatusCode {
        StatusCode::NOT_MODIFIED
    }

    async fn get_body_async(&self) -> Vec<u8> {
        Vec::new()
    }

    fn content_length(&self) -> usize {
        0
    }
}
