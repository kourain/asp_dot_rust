use crate::http_context::http_request::HttpRequest;
use crate::http_context::http_response::HttpResponse;
use crate::services::service_provider::ServiceProviderScope;

pub struct HttpContext {
    pub request: HttpRequest,
    pub response: HttpResponse,
    pub service_provider: ServiceProviderScope,
}

impl HttpContext {
    pub(crate) fn new(request: HttpRequest, response: HttpResponse, service_provider: ServiceProviderScope) -> Self {
        Self { request, response, service_provider }
    }
}
