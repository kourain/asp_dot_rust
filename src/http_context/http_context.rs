use crate::http_context::http_request::HttpRequest;
use crate::http_context::http_response::HttpResponse;
use crate::services::service_provider::service_provider_scope::ServiceProviderScope;
use std::any::Any;
use std::collections::HashMap;

/// Key used by authorize middleware to store the authenticated user in `context_data`.
pub const USER_CONTEXT_KEY: &str = "user";

pub struct HttpContext {
    pub request: HttpRequest,
    pub response: HttpResponse,
    pub service_provider: ServiceProviderScope,
    /// Per-request bag for middleware and handlers (e.g. authenticated user under `"user"`).
    pub context_data: HashMap<String, Box<dyn Any + Send + Sync>>,
}

impl HttpContext {
    pub(crate) fn new(request: HttpRequest, response: HttpResponse, service_provider: ServiceProviderScope) -> Self {
        Self {
            request,
            response,
            service_provider,
            context_data: HashMap::new(),
        }
    }

    /// Breaks the response stream when the response has started,
    /// Does nothing when the response has not started, set the status and body instead.
    pub fn abort(&mut self) {
        self.response.abort();
    }

    /// Stores a value in `context_data` under `key`.
    pub fn set_item<T: Any + Send + Sync + 'static>(&mut self, key: impl Into<String>, value: T) {
        self.context_data.insert(key.into(), Box::new(value));
    }

    /// Returns a reference to a value previously stored with `set_item`.
    pub fn get_item<T: Any + Send + Sync + 'static>(&self, key: &str) -> Option<&T> {
        self.context_data.get(key).and_then(|v| v.downcast_ref::<T>())
    }

    /// Returns a mutable reference to a value previously stored with `set_item`.
    pub fn get_item_mut<T: Any + Send + Sync + 'static>(&mut self, key: &str) -> Option<&mut T> {
        self.context_data.get_mut(key).and_then(|v| v.downcast_mut::<T>())
    }

    /// Removes and returns a value previously stored with `set_item`.
    pub fn take_item<T: Any + Send + Sync + 'static>(&mut self, key: &str) -> Option<T> {
        self.context_data.remove(key).and_then(|v| v.downcast::<T>().ok().map(|b| *b))
    }
}
