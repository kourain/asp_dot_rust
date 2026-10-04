use crate::{
    Application,
    http_context::HttpContext,
    logging::LOGGER,
    middleware::{Middleware, MiddlewareNext, auto_route::RoutingMiddleware},
    services::service_provider::service_provider_scope::ServiceProviderScope,
};
use std::sync::Arc;

/// Creates a middleware from the next one, called once when the pipeline is built.
type Factory = Box<dyn Fn(&ServiceProviderScope, MiddlewareNext) -> MiddlewareNext + Send + Sync>;

pub(crate) struct ApplicationMiddlewares {
    builder: Vec<Factory>,
    root: Option<MiddlewareNext>,
}
impl ApplicationMiddlewares {
    pub fn new() -> Self {
        Self { builder: Vec::new(), root: None }
    }
    pub fn add(&mut self, factory: Factory) {
        self.builder.push(factory);
    }
    /// Creates the middlewares from the last to the first, so each one receives the one after it as `next`.
    pub fn build_pipeline(&mut self, service_scope: &ServiceProviderScope) {
        LOGGER::info(format!("Building middleware pipeline {} middlewares", self.builder.len()));
        let routing = MiddlewareNext::new(RoutingMiddleware);
        let root = self.builder.iter().rev().fold(routing, |next, factory| factory(service_scope, next));
        self.builder.clear();
        self.root = Some(root);
    }
    pub async fn invoke_async(&self, http_context: &mut HttpContext) {
        if let Some(root) = self.root.as_ref() {
            root.next_async(http_context).await;
        }
    }
}

impl Application {
    pub fn add_middleware<T>(&mut self) -> &mut Self
    where
        T: Middleware + 'static,
    {
        LOGGER::info(format!("Adding middleware: {}", std::any::type_name::<T>()));
        self._middlewares.add(Box::new(|service_scope, next| MiddlewareNext::new(T::inject(service_scope, next))));
        self
    }
    /// Adds a middleware created by your own function, `factory` receives the next middleware.
    pub fn add_middleware_factory<M, F>(&mut self, factory: F) -> &mut Self
    where
        M: Middleware + 'static,
        F: Fn(MiddlewareNext) -> M + Send + Sync + 'static,
    {
        LOGGER::info(format!("Adding middleware factory: {}", std::any::type_name::<M>()));
        self._middlewares.add(Box::new(move |_, next| MiddlewareNext::new(factory(next))));
        self
    }
    pub(crate) async fn call_middlewares_async(&self, http_context: &mut HttpContext) {
        self._middlewares.invoke_async(http_context).await;
    }
}
