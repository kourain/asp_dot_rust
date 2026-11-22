use crate::{
    Application,
    dependency_injection::Serv,
    http_context::HttpContext,
    macros::middleware,
    middleware::MiddlewareNext,
    services::openapi::{OpenApiDocument, OpenApiService},
};

/// Serves Swagger UI and the generated OpenAPI JSON document.
///
/// - `GET /swagger` or `GET /swagger/`  HTML page (Swagger UI from CDN)
/// - `GET /swagger/v1/swagger.json`  OpenAPI 3.2 JSON built from registered routes
pub struct SwaggerMiddleware {
    next: MiddlewareNext,
    open_api_service: Serv<OpenApiService>,
    title: String,
    version: String,
    route_prefix: String,
}

#[middleware]
impl SwaggerMiddleware {
    pub fn new(next: MiddlewareNext, open_api_service: Serv<OpenApiService>) -> Self {
        Self {
            next,
            open_api_service,
            title: "API".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            route_prefix: "/swagger".to_string(),
        }
    }

    pub async fn invoke_async(&self, http_context: &mut HttpContext) {
        let path = http_context.request.path();
        let prefix = self.route_prefix.as_str();
        let json_path = format!("{}/v1/swagger.json", prefix);

        if path == json_path || path == format!("{}/v1/swagger.json/", prefix.trim_end_matches('/')) {
            self.serve_json(http_context).await;
            return;
        }

        if path == prefix || path == format!("{}/", prefix) || path == format!("{}/index.html", prefix) {
            self.serve_ui(http_context).await;
            return;
        }

        self.next.next_async(http_context).await;
    }

    async fn serve_json(&self, http_context: &mut HttpContext) {
        let operations = self.open_api_service.registered_operations();
        let doc = OpenApiDocument::from_operations(&self.title, &self.version, &operations);
        match doc.to_json() {
            Ok(json) => {
                http_context.response.status_code = http::StatusCode::OK;
                http_context
                    .response
                    .headers
                    .insert(http::header::CONTENT_TYPE, http::HeaderValue::from_static("application/json; charset=utf-8"));
                http_context.response.body = json.into_bytes();
            }
            Err(_) => {
                http_context.response.status_code = http::StatusCode::INTERNAL_SERVER_ERROR;
                http_context.response.body = b"Failed to serialize OpenAPI document".to_vec();
            }
        }
    }

    async fn serve_ui(&self, http_context: &mut HttpContext) {
        let json_url = format!("{}/v1/swagger.json", self.route_prefix);
        let html = self.swagger_ui_html(&json_url);
        http_context.response.status_code = http::StatusCode::OK;
        http_context
            .response
            .headers
            .insert(http::header::CONTENT_TYPE, http::HeaderValue::from_static("text/html; charset=utf-8"));
        http_context.response.body = html.into_bytes();
    }
    fn swagger_ui_html(&self, swagger_json_url: &str) -> String {
        format!(
            r##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>{title}</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.34.1/swagger-ui.css">
  <style>body {{ margin: 0; background: #fafafa; }}</style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5.34.1/swagger-ui-bundle.js"></script>
  <script>
    window.onload = function () {{
      window.ui = SwaggerUIBundle({{
        url: "{swagger_json_url}",
        dom_id: "#swagger-ui",
        presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
        layout: "BaseLayout"
      }});
    }};
  </script>
</body>
</html>
"##,
            title = self.title
        )
    }
}

impl Application {
    /// Enables automatic Swagger UI and OpenAPI document.
    ///
    /// UI: `/swagger`
    /// JSON: `/swagger/v1/swagger.json`
    ///
    /// Attention: you must call `builder.add_swagger_gen()` first else pannic
    pub fn use_swagger(&mut self) -> &mut Self {
        self.add_middleware::<SwaggerMiddleware>();
        self
    }
}
