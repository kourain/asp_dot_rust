use std::{future::Future, pin::Pin, sync::Arc};

use crate::{
    Application,
    logging::LOGGER,
    middleware::{Middleware, MiddlewareNext, auto_route::RoutingMiddleware, handle_error::HandleErrorMiddleware},
    services::service_provider::service_provider_scope::ServiceProviderScope,
};

/// Creates a middleware from the next one, called once when the pipeline is built.
/// The factory is async so that `Middleware::inject` (which may resolve services) can await.
type Factory = Box<dyn for<'a> Fn(&'a ServiceProviderScope, MiddlewareNext) -> Pin<Box<dyn Future<Output = MiddlewareNext> + Send + 'a>> + Send + Sync>;

pub(crate) struct ApplicationMiddlewaresBuilder {
    builder: Vec<Factory>,
}
impl ApplicationMiddlewaresBuilder {
    pub fn new() -> Self {
        Self { builder: Vec::new() }
    }
    pub fn add(&mut self, factory: Factory) {
        self.builder.push(factory);
    }
    /// Creates the middlewares from the last to the first, so each one receives the one after it as `next`.
    /// Must complete before the HTTP server opens any port.
    pub async fn build_pipeline(self, service_scope: &ServiceProviderScope) -> MiddlewareNext {
        LOGGER::info(format!("Building middleware pipeline {} middlewares", self.builder.len()));
        let routing = MiddlewareNext::new(RoutingMiddleware);
        let mut root = routing;
        for factory in self.builder.into_iter().rev() {
            root = factory(service_scope, root).await;
        }
        MiddlewareNext::new(HandleErrorMiddleware::inject(service_scope, root).await)
    }
}

impl Application {
    pub fn add_middleware<T>(&mut self) -> &mut Self
    where
        T: Middleware + 'static,
    {
        LOGGER::info(format!("Adding middleware: {}", std::any::type_name::<T>()));
        self._middlewares
            .add(Box::new(|service_scope, next| Box::pin(async move { MiddlewareNext::new(T::inject(service_scope, next).await) })));
        self
    }
    /// Adds a middleware created by your own function, `factory` receives the next middleware.
    pub fn add_middleware_factory<M, F>(&mut self, factory: F) -> &mut Self
    where
        M: Middleware + 'static,
        F: Fn(MiddlewareNext) -> M + Send + Sync + 'static,
    {
        LOGGER::info(format!("Adding middleware factory: {}", std::any::type_name::<M>()));
        let factory = Arc::new(factory);
        self._middlewares.add(Box::new(move |_, next| {
            let factory = factory.clone();
            Box::pin(async move { MiddlewareNext::new(factory(next)) })
        }));
        self
    }
}
