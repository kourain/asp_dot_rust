use crate::http_context::http_request::HttpRequest;
use crate::http_context::http_response::HttpResponse;
use crate::services::service_provider::service_provider_scope::ServiceProviderScope;
use crate::utils::ReadOnly;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

/// Key used by authorize middleware to store the authenticated user in `context_data`.
pub const USER_CONTEXT_KEY: &str = "user";

pub struct HttpContext {
    pub request: HttpRequest,
    pub response: HttpResponse,
    pub service_provider: ReadOnly<ServiceProviderScope>,
    /// Per-request bag for middleware and handlers (e.g. authenticated user under `"user"`).
    pub context_data: HashMap<String, Box<dyn Any + Send + Sync>>,
    pub(crate) is_shareable: Arc<AtomicBool>,
    pub(crate) web_socket_handshake: Option<super::WebSocketHandshake>,
}

impl HttpContext {
    pub(crate) fn new(request: HttpRequest, response: HttpResponse, service_provider: ServiceProviderScope, web_socket_handshake: Option<super::WebSocketHandshake>) -> Self {
        Self {
            request,
            web_socket_handshake,
            response,
            service_provider: ReadOnly::new(service_provider),
            context_data: HashMap::new(),
            is_shareable: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Breaks the response stream when the response has started,
    /// Does nothing when the response has not started, set the status and body instead.
    pub fn abort(&mut self) {
        if let Some(handshake) = &mut self.web_socket_handshake {
            handshake.abort_socket();
        }
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

impl Drop for HttpContext {
    fn drop(&mut self) {
        self.is_shareable.store(false, std::sync::atomic::Ordering::Release);
    }
}
