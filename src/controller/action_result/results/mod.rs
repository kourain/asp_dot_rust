//! HTTP results returned by controller actions.

use std::io;

use http::{
    HeaderMap, HeaderValue, StatusCode,
    header::{InvalidHeaderValue, LOCATION},
};

use super::{ActionResult, content_negotiation};
use crate::http_context::HttpResponse;

mod body;
use body::ResponseBody;
pub use body::{Json, Raw, Xml};

/// A buffered controller response. Use the status functions in this module to construct it.
#[derive(Debug)]
pub struct RouteResult<T> {
    status: StatusCode,
    body: T,
    headers: Option<HeaderMap>,
    write_body: fn(&T, &HeaderMap, &mut HttpResponse) -> io::Result<()>,
}

impl<T> RouteResult<T> {
    /// Writes text, negotiates a model or collection, or uses the representation selected by Raw, Json or Xml.
    pub fn new(status: StatusCode, body: T) -> Self
    where
        T: ResponseBody,
    {
        Self {
            status,
            body,
            headers: None,
            write_body: if status.is_informational() || matches!(status, StatusCode::NO_CONTENT | StatusCode::RESET_CONTENT | StatusCode::NOT_MODIFIED) {
                |_, _, _| Result::Ok(())
            } else {
                T::write
            },
        }
    }

    /// Returns the response headers. Use `insert` to replace a value or `append` to add another.
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.headers.get_or_insert_with(HeaderMap::new)
    }

    /// Adds a validated Location header.
    pub fn with_location(mut self, location: impl AsRef<str>) -> Result<Self, InvalidHeaderValue> {
        self.headers_mut().insert(LOCATION, HeaderValue::from_str(location.as_ref())?);
        Result::Ok(self)
    }
}

impl<T> RouteResult<Raw<T>>
where
    Raw<T>: ResponseBody,
{
    /// Writes a binary response with Content-Type application/octet-stream.
    pub fn raw(status: StatusCode, body: T) -> Self {
        Self::new(status, Raw(body))
    }
}

impl RouteResult<()> {
    /// Returns a status and an empty body.
    pub const fn empty(status: StatusCode) -> Self {
        Self {
            status,
            body: (),
            headers: None,
            write_body: |_, _, _| Result::Ok(()),
        }
    }
}

#[async_trait::async_trait]
impl<T: Send + Sync> ActionResult for RouteResult<T> {
    fn status_code(&self) -> StatusCode {
        self.status
    }

    fn write_header(&self, headers: &mut HeaderMap) {
        if let Some(result_headers) = &self.headers {
            headers.extend(result_headers.clone());
        }
    }

    async fn write_body_async(&self, request_headers: &HeaderMap, response: &mut HttpResponse) -> io::Result<()> {
        (self.write_body)(&self.body, request_headers, response)
    }
}

macro_rules! body_results {
    ($($name:ident as $alias:ident => $status:ident),+ $(,)?) => {
        $(
            #[doc = concat!("Buffered response with status `", stringify!($status), "`.")]
            pub fn $name<T: ResponseBody>(body: T) -> RouteResult<T> {
                RouteResult::new(StatusCode::$status, body)
            }
            pub use self::$name as $alias;
        )+
    };
}

body_results! {
    ok as Ok => OK,
    created as Created => CREATED,
    accepted as Accepted => ACCEPTED,
    bad_request as BadRequest => BAD_REQUEST,
    unauthorized as Unauthorized => UNAUTHORIZED,
    forbidden as Forbidden => FORBIDDEN,
    not_found as NotFound => NOT_FOUND,
    conflict as Conflict => CONFLICT,
    gone as Gone => GONE,
    unsupported_media_type as UnsupportedMediaType => UNSUPPORTED_MEDIA_TYPE,
    unprocessable_entity as UnprocessableEntity => UNPROCESSABLE_ENTITY,
    too_many_requests as TooManyRequests => TOO_MANY_REQUESTS,
    internal_server_error as InternalServerError => INTERNAL_SERVER_ERROR,
    service_unavailable as ServiceUnavailable => SERVICE_UNAVAILABLE,
}

macro_rules! redirects {
    ($($name:ident as $alias:ident => $status:ident),+ $(,)?) => {
        $(
            #[doc = concat!("Empty redirect with status `", stringify!($status), "` and a Location header.")]
            pub fn $name(location: impl AsRef<str>) -> Result<RouteResult<()>, InvalidHeaderValue> {
                RouteResult::empty(StatusCode::$status).with_location(location)
            }
            pub use self::$name as $alias;
        )+
    };
}

redirects! {
    moved_permanently as MovedPermanently => MOVED_PERMANENTLY,
    redirect as Redirect => FOUND,
    see_other as SeeOther => SEE_OTHER,
    temporary_redirect as TemporaryRedirect => TEMPORARY_REDIRECT,
    permanent_redirect as PermanentRedirect => PERMANENT_REDIRECT,
}

/// HTTP 204 No Content.
pub const fn no_content() -> RouteResult<()> {
    RouteResult::empty(StatusCode::NO_CONTENT)
}

/// HTTP 304 Not Modified.
pub const fn not_modified() -> RouteResult<()> {
    RouteResult::empty(StatusCode::NOT_MODIFIED)
}

pub use self::{no_content as NoContent, not_modified as NotModified};
