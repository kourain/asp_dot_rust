use crate::{
    Application,
    configuration::StaticFileConfiguration,
    dependency_injection::CfgOption,
    http_context::http_header::HttpHeader,
    macros::middleware,
    middleware::MiddlewareNext,
};

pub struct StaticFileMiddleware {
    next: MiddlewareNext,
    static_dir_path: std::path::PathBuf,
}

#[middleware]
impl StaticFileMiddleware {
    pub fn new(next: MiddlewareNext, config: CfgOption<StaticFileConfiguration>) -> Self {
        let config = config.unwrap_or_default();
        let static_dir_path = std::path::Path::new(&config.static_files_directory);
        if !static_dir_path.is_dir() {
            panic!(
                "Static files directory {} does not exist or is not a directory, static file middleware will not serve any files",
                static_dir_path.display()
            );
        }
        StaticFileMiddleware {
            next,
            static_dir_path: static_dir_path.to_path_buf(),
        }
    }

    pub async fn invoke_async(&self, http_context: &mut crate::http_context::HttpContext) {
        let request_path = http_context.request.path().trim_start_matches('/');
        // Reject empty path and path traversal segments before joining.
        if request_path.is_empty() || request_path.split(['/', '\\']).any(|segment| segment == ".." || segment == ".") {
            self.next.next_async(http_context).await;
            return;
        }

        let candidate = self.static_dir_path.join(request_path);
        let Ok(canonical) = candidate.canonicalize() else {
            self.next.next_async(http_context).await;
            return;
        };
        let Ok(root) = self.static_dir_path.canonicalize() else {
            self.next.next_async(http_context).await;
            return;
        };
        if !canonical.starts_with(&root) || !canonical.is_file() {
            self.next.next_async(http_context).await;
            return;
        }

        match tokio::fs::read(&canonical).await {
            Ok(file_content) => {
                http_context.response.body = file_content;
                if let Some(extension) = canonical.extension().and_then(|e| e.to_str()) {
                    let content_type = match extension {
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
                        "svg" => "image/svg+xml",
                        "txt" => "text/plain",
                        "wasm" => "application/wasm",
                        "woff" => "font/woff",
                        "woff2" => "font/woff2",
                        _ => "application/octet-stream",
                    };
                    http_context.response.headers.insert_str("Content-Type", content_type);
                }
            }
            Err(_) => {
                self.next.next_async(http_context).await;
            }
        }
    }
}
impl Application {
    pub fn use_static_files(&mut self) -> &mut Self {
        self.add_middleware::<StaticFileMiddleware>();
        self
    }
}
