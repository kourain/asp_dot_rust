mod document;
mod open_api_service;

pub use document::{OpenApiDocument, OpenApiInfo, OpenApiOperation, OpenApiParameter, OpenApiPathItem, OpenApiResponse, RegisteredOperation};
pub use open_api_service::OpenApiService;

use crate::ApplicationBuilder;

impl ApplicationBuilder {
    pub fn add_swagger_gen(&mut self) {
        self.service.add_singleton::<OpenApiService>();
    }
}
