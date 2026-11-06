use std::pin::Pin;

use crate::{ApplicationBuilder, hosted_service::BackGroundService, services::service_provider::ServiceProviderScope};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub type HostedServiceFactory = Box<
    dyn FnOnce(&ServiceProviderScope) -> BoxFuture<'_, Box<dyn BackGroundService>> + Send + Sync
>;

pub type ApplicationHostedService = Vec<(&'static str, HostedServiceFactory)>;

impl ApplicationBuilder {
    pub fn add_hosted_service<T>(&mut self) -> &mut Self
    where
        T: BackGroundService + 'static,
    {
        self.hosted_services.push((
            std::any::type_name::<T>(),
            Box::new(|container| {
                Box::pin(async move {
                    let service = T::inject(container).await;
                    Box::new(service) as Box<dyn BackGroundService>
                })
            }),
        ));
        self
    }
}