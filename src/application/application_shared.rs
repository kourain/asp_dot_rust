pub struct ApplicationSharedInfo {
    pub service_provider: ApplicationServiceProvider,
    pub config: ApplicationServiceProvider,
    pub hosted_services_signal: Vec<crate::hosted_service::ApplicationHostedService>,
}