use std::sync::Arc;

use crate::{
    Application,
    http_context::HttpContext,
    logging::LOGGER,
    middleware::{Middleware, MiddlewareNext, auto_route},
};

pub(crate) struct ApplicationMiddlewares {
    middlewares: Vec<Arc<dyn Middleware>>,
    pipeline: Option<MiddlewareNext>,
}
impl ApplicationMiddlewares {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
            pipeline: None,
        }
    }
    pub fn add<M>(&mut self, middleware: M)
    where
        M: Middleware + 'static,
    {
        self.middlewares.push(Arc::new(middleware));
    }
    pub fn build_pipeline(&mut self) {
        LOGGER::info(format!("Building middleware pipeline {} middlewares", self.middlewares.len()));
        let mut next: MiddlewareNext = Arc::new(|http_ctx| auto_route::invoke_async(http_ctx));
        for middleware in self.middlewares.iter().rev() {
            let middleware = middleware.clone();
            let next_handler = next.clone();
            LOGGER::trace(format!("Adding middleware to pipeline: {}", middleware.type_name()));
            next = Arc::new(move |http_context: &mut HttpContext| {
                let middleware = middleware.clone();
                LOGGER::trace(format!("Nexting to middleware: {}", middleware.type_name()));
                let next = next_handler.clone();
                Box::pin(async move {
                    LOGGER::debug(format!("Executing middleware: {}", middleware.type_name()));
                    middleware.invoke_async(http_context, next).await;
                })
            });
        }
        self.pipeline = Some(next);
    }
    pub async fn execute(&self, http_context: &mut HttpContext) {
        if let Some(root) = self.pipeline.as_ref() {
            root(http_context).await;
        }
    }
}

impl Application {
    pub fn add_middleware<M>(&mut self) -> &mut Self
    where
        M: Middleware + 'static,
    {
        LOGGER::info(format!("Adding middleware: {}", std::any::type_name::<M>()));
        let middleware_instance = M::inject_service(&self.service_provider);
        self._middlewares.add(middleware_instance);
        self
    }
    pub fn add_middleware_instance<M>(&mut self, middleware: M) -> &mut Self
    where
        M: Middleware + 'static,
    {
        LOGGER::info(format!("Adding middleware instance: {}", std::any::type_name::<M>()));
        let middleware_instance = middleware;
        self._middlewares.add(middleware_instance);
        self
    }
    pub async fn call_middlewares_async(&self, http_context: &mut HttpContext) {
        self._middlewares.execute(http_context).await;
    }
}
