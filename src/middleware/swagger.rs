use crate::{
    Application,
    dependency_injection::Serv,
    http_context::{HttpContext, HttpHeaderExt},
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
}

#[middleware]
impl SwaggerMiddleware {
    pub fn new(next: MiddlewareNext, open_api_service: Serv<OpenApiService>) -> Self {
        Self { next, open_api_service }
    }

    pub async fn invoke_async(&self, http_context: &mut HttpContext) {
        match http_context.request.path() {
            "/swagger/v1/swagger.json" | "/swagger/v1/swagger.json/" => self.serve_json(http_context),
            "/swagger" | "/swagger/" | "/swagger/index.html" => Self::serve_ui(http_context),
            _ => self.next.next_async(http_context).await,
        }
    }

    fn serve_json(&self, http_context: &mut HttpContext) {
        let doc = OpenApiDocument::from_operations("API", env!("CARGO_PKG_VERSION"), &self.open_api_service._operations);
        match doc.to_json() {
            Ok(json) => {
                http_context.response.status_code = http::StatusCode::OK;
                http_context.response.headers.set_content_type("application/json; charset=utf-8");
                http_context.response.body = json.into_bytes();
            }
            Err(_) => {
                http_context.response.status_code = http::StatusCode::INTERNAL_SERVER_ERROR;
                http_context.response.body = b"Failed to serialize OpenAPI document".to_vec();
            }
        }
    }

    fn serve_ui(http_context: &mut HttpContext) {
        http_context.response.status_code = http::StatusCode::OK;
        http_context.response.headers.set_content_type("text/html; charset=utf-8");
        http_context.response.body = br##"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>API</title>
  <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5.34.1/swagger-ui.css">
  <style>body { margin: 0; background: #fafafa; }</style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5.34.1/swagger-ui-bundle.js"></script>
  <script>
    window.onload = function () {
      window.ui = SwaggerUIBundle({
        url: "/swagger/v1/swagger.json",
        dom_id: "#swagger-ui",
        presets: [SwaggerUIBundle.presets.apis, SwaggerUIBundle.SwaggerUIStandalonePreset],
        layout: "BaseLayout"
      });
    };
  </script>
</body>
</html>
"##.to_vec();
    }
}

impl Application {
    /// Enables automatic Swagger UI and OpenAPI document.
    ///
    /// UI: `/swagger`
    /// JSON: `/swagger/v1/swagger.json`
    ///
    /// Register the OpenAPI service with `builder.add_swagger_gen()` before building the application.
    pub fn use_swagger(&mut self) -> &mut Self {
        self.add_middleware::<SwaggerMiddleware>();
        self
    }
}
