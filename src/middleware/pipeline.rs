use crate::{
    Application,
    http_context::HttpContext,
    logging::LOGGER,
    middleware::{BoxedService, Middleware, MiddlewareService, WithApplication},
};

type MiddlewareFactory = Box<dyn FnOnce(BoxedService) -> BoxedService + Send + Sync>;

pub(crate) struct ApplicationMiddlewares {
    factories: Vec<(String, MiddlewareFactory)>,
    pipeline: Option<BoxedService>,
}

impl ApplicationMiddlewares {
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
            pipeline: None,
        }
    }

    pub fn add<M>(&mut self, middleware: M)
    where
        M: Middleware<BoxedService> + Send + Sync + 'static,
        M::Service: MiddlewareService + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<M>().to_string();
        self.factories.push((
            type_name,
            Box::new(move |inner: BoxedService| {
                BoxedService::new(middleware.wrap(inner))
            }),
        ));
    }

    pub fn build_pipeline(&mut self) {
        LOGGER::info(format!(
            "Building middleware pipeline {} middlewares",
            self.factories.len()
        ));

        let no_op = BoxedService::new_fn(|_ctx| async {});

        let pipeline = self.factories.drain(..).rev().fold(
            no_op,
            |inner, (name, factory)| {
                LOGGER::trace(format!("Adding middleware to pipeline: {}", name));
                factory(inner)
            },
        );

        self.pipeline = Some(pipeline);
    }

    pub async fn execute(&self, http_context: &mut HttpContext) {
        if let Some(root) = self.pipeline.as_ref() {
            root.invoke_async(http_context).await;
        }
    }
}

impl Application {
    pub fn add_middleware<M>(&mut self) -> &mut Self
    where
        M: Middleware<BoxedService> + WithApplication + Default + Send + Sync + 'static,
        M::Service: MiddlewareService + Send + Sync + 'static,
    {
        LOGGER::info(format!("Adding middleware: {}", std::any::type_name::<M>()));
        let mut instance = M::default();
        instance.with_application(self);
        self._middlewares.add(instance);
        self
    }

    pub fn add_middleware_instance<M>(&mut self, middleware: M) -> &mut Self
    where
        M: Middleware<BoxedService> + WithApplication + Send + Sync + 'static,
        M::Service: MiddlewareService + Send + Sync + 'static,
    {
        LOGGER::info(format!(
            "Adding middleware instance: {}",
            std::any::type_name::<M>()
        ));
        let mut instance = middleware;
        instance.with_application(self);
        self._middlewares.add(instance);
        self
    }

    pub async fn call_middlewares_async(&self, http_context: &mut HttpContext) {
        self._middlewares.execute(http_context).await;
    }
}