use crate::http_context::http_request::HttpRequest;
use crate::http_context::http_response::HttpResponse;
use crate::services::service_provider::service_provider_scope::ServiceProviderScope;
pub struct _HttpContext {
    pub request: HttpRequest,
    pub response: HttpResponse,
    pub routing_info: Option<crate::services::routing::ResolvedRoute>,
    pub service_provider: ServiceProviderScope,
}

impl _HttpContext {
    pub(crate) fn new(request: HttpRequest, response: HttpResponse, service_provider: ServiceProviderScope) -> Self {
        Self {
            request,
            response,
            routing_info: None,
            service_provider: service_provider,
        }
    }
}
