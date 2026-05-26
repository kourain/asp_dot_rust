use crate::{
    Application,
    http_context::HttpContext,
    logging::LOGGER,
    middleware::{DynMiddleware, MiddlewareKind, MiddlewareNext},
};

pub(crate) struct ApplicationMiddlewares {
    middlewares: Vec<MiddlewareKind>,
}

impl ApplicationMiddlewares {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    /// Add a pre-built MiddlewareKind (for internal middleware)
    pub fn add_kind(&mut self, kind: MiddlewareKind) {
        self.middlewares.push(kind);
    }

    /// Add a dynamic middleware (for external/user-defined middleware)
    pub fn add_dynamic<M>(&mut self, middleware: M)
    where
        M: DynMiddleware + 'static,
    {
        self.middlewares.push(MiddlewareKind::Dynamic(Box::new(middleware)));
    }

    /// Log the pipeline for debugging
    pub fn log_pipeline(&self) {
        LOGGER::info(format!(
            "Middleware pipeline: {} middlewares",
            self.middlewares.len()
        ));
        for mw in &self.middlewares {
            LOGGER::trace(format!("  → {}", mw.type_name()));
        }
    }

    /// Execute the middleware pipeline. Zero-alloc dispatch for internal middleware.
    pub async fn execute(&self, http_context: &mut HttpContext) {
        let next = MiddlewareNext::new(&self.middlewares);
        next.invoke(http_context).await;
    }
}

impl Application {
    pub fn add_middleware_dynamic<M>(&mut self) -> &mut Self
    where
        M: DynMiddleware + Default + 'static,
    {
        LOGGER::info(format!(
            "Adding dynamic middleware: {}",
            std::any::type_name::<M>()
        ));
        let mut middleware_instance = M::default();
        middleware_instance.with_application(self);
        self._middlewares.add_dynamic(middleware_instance);
        self
    }

    pub fn add_middleware_dynamic_instance<M>(&mut self, mut middleware: M) -> &mut Self
    where
        M: DynMiddleware + 'static,
    {
        LOGGER::info(format!(
            "Adding dynamic middleware instance: {}",
            std::any::type_name::<M>()
        ));
        middleware.with_application(self);
        self._middlewares.add_dynamic(middleware);
        self
    }

    pub async fn call_middlewares_async(&self, http_context: &mut HttpContext) {
        self._middlewares.execute(http_context).await;
    }
}
