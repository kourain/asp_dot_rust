mod http_context;
mod http_header;
mod http_request;
mod http_response;

use crate::utils::ShareMutPtr;
pub use http_context::{HttpContext, USER_CONTEXT_KEY};
pub use http_header::HttpHeaderExt;
pub use http_request::{HttpRequest, RequestBody};
pub use http_response::HttpResponse;
pub(crate) use http_response::ResponseHead;
use std::ops::{Deref, DerefMut};

/// Access to the current request context supplied to a controller.
pub struct HttpContextRef(pub(crate) ShareMutPtr<HttpContext>);

impl Deref for HttpContextRef {
    type Target = HttpContext;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HttpContextRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
