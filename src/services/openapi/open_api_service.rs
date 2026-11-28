use crate::{
    dependency_injection::DependencyInjectableService,
    services::{openapi::RegisteredOperation, service_provider::ServiceProviderScope},
};

pub struct OpenApiService {
    /// Flat list of registered operations for OpenAPI generation.
    pub(crate) _operations: Vec<RegisteredOperation>,
}

#[async_trait::async_trait]
impl DependencyInjectableService for OpenApiService {
    async fn inject(_: &ServiceProviderScope) -> Self {
        Self { _operations: Default::default() }
    }
    fn forgot_register_message() -> String {
        "OpenApiService not registered yet, did you forget `builder.add_swagger_gen()` ?".to_string()
    }
}

impl OpenApiService {
    /// Returns all registered operations for OpenAPI document generation.
    pub fn registered_operations(&self) -> Vec<RegisteredOperation> {
        self._operations.clone()
    }
}
