mod application_hosted_service;
pub use application_hosted_service::ApplicationHostedService;
#[async_trait::async_trait]
pub trait BackGroundService: Send + Sync {
    fn with_service_provider() -> Self
    where
        Self: Sized;
    async fn invoke_async(&mut self) -> std::io::Result<()>;
}
