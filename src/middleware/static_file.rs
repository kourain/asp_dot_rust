use crate::{
    Application,
    configuration::StaticFileConfiguration, http_context::http_header::AspDotRustHttpHeader,
    dependcy_injection::DependcyInjectableService,
    middleware::{self, Middleware},
    services::configuration::ConfigurationService,
};
pub struct StaticFileMiddleware {
    config: StaticFileConfiguration,
}
impl DependcyInjectableService for StaticFileMiddleware {
    fn inject_service(service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
        let config = service_scope.get_service::<ConfigurationService>().get::<StaticFileConfiguration>().unwrap_or_default();
        StaticFileMiddleware { config }
    }
}
#[async_trait::async_trait]
impl Middleware for StaticFileMiddleware {
    async fn invoke_async<'a>(&self, http_context: &'a mut crate::http_context::HttpContext, next: middleware::MiddlewareNext) {
        let request_path = http_context.request.path().to_string();
        let static_file_path = format!("{}/{}", self.config.static_files_directory, request_path.trim_start_matches('/'));

        if std::path::Path::new(&static_file_path).exists() {
            if let Ok(file_content) = std::fs::read(&static_file_path) {
                http_context.response.body = file_content;
                // Optionally set the Content-Type header based on the file extension
                if let Some(extension) = std::path::Path::new(&static_file_path).extension() {
                    let content_type = match extension.to_str().unwrap_or("") {
                        "html" => "text/html",
                        "css" => "text/css",
                        "js" => "application/javascript",
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        "gif" => "image/gif",
                        "json" => "application/json",
                        "mp3" => "audio/mpeg",
                        "mp4" => "video/mp4",
                        "opus" => "audio/opus",
                        _ => "application/octet-stream",
                    };
                    http_context.response.headers.insert_str("Content-Type", content_type);
                }
                return; // Return early since we've handled the response
            }
        }

        next(http_context).await;
    }
}

impl Application {
    pub fn use_static_files(&mut self) -> &mut Self {
        self.add_middleware::<StaticFileMiddleware>();
        self
    }
}
