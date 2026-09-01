use crate::{
    Application,
    configuration::StaticFileConfiguration,
    http_context::http_header::AspDotRustHttpHeader,
    macros::inject_require,
    middleware::{self, Middleware},
    utils,
};
use std::sync::Arc;

pub struct StaticFileMiddleware {
    static_dir_path: std::path::PathBuf,
}

#[inject_require]
impl StaticFileMiddleware {
    pub fn new(config: Option<Arc<StaticFileConfiguration>>) -> Self {
        let config = config.unwrap_or_default();
        let static_dir_path = std::path::Path::new(&config.static_files_directory);
        if !static_dir_path.is_dir() {
            panic!(
                "Static files directory {} does not exist or is not a directory, static file middleware will not serve any files",
                static_dir_path.display()
            );
        }
        StaticFileMiddleware {
            static_dir_path: static_dir_path.to_path_buf(),
        }
    }
}

#[async_trait::async_trait]
impl Middleware for StaticFileMiddleware {
    async fn invoke_async(&self, http_context: &mut crate::http_context::HttpContext, next: middleware::MiddlewareNext) {
        let request_path = http_context.request.path().to_string();
        let static_file_path = format!("{}/{}", self.static_dir_path.display(), request_path.trim_start_matches('/'));
        let fpath = std::path::Path::new(&static_file_path);

        if !fpath.exists() {
            if utils::path::is_path_in_folder(fpath.canonicalize().unwrap().to_str().unwrap(), self.static_dir_path.canonicalize().unwrap().to_str().unwrap()) == false {
                http_context.response.status_code = http::StatusCode::FORBIDDEN;
                http_context.response.body = http::StatusCode::FORBIDDEN.canonical_reason().unwrap_or("Forbidden").as_bytes().to_vec();
                return;
            }

            match std::fs::read(&static_file_path) {
                Ok(file_content) => {
                    http_context.response.body = file_content;
                    if let Some(extension) = fpath.extension() {
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
                }
                Err(_) => {}
            }
        } else {
            next(http_context).await;
        }
    }
}
impl Application {
    pub fn use_static_files(&mut self) -> &mut Self {
        self.add_middleware::<StaticFileMiddleware>();
        self
    }
}
