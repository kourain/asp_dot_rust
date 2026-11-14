use serde::Serialize;
use std::collections::BTreeMap;

/// Minimal OpenAPI 3.0 document built from registered routes.
#[derive(Debug, Clone, Serialize)]
pub struct OpenApiDocument {
    pub openapi: &'static str,
    pub info: OpenApiInfo,
    pub paths: BTreeMap<String, OpenApiPathItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct OpenApiPathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<OpenApiOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<OpenApiOperation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiOperation {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(rename = "operationId")]
    pub operation_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<OpenApiParameter>,
    pub responses: BTreeMap<String, OpenApiResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiParameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: &'static str,
    pub required: bool,
    pub schema: OpenApiSchema,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiSchema {
    #[serde(rename = "type")]
    pub schema_type: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiResponse {
    pub description: String,
}

/// One registered operation used to build the document.
#[derive(Debug, Clone)]
pub struct RegisteredOperation {
    pub path: String,
    pub method: http::Method,
    pub controller_name: &'static str,
    pub action_name: &'static str,
}

impl OpenApiDocument {
    /// Builds an OpenAPI 3.0 document from the list of registered operations.
    pub fn from_operations(title: impl Into<String>, version: impl Into<String>, operations: &[RegisteredOperation]) -> Self {
        let mut paths: BTreeMap<String, OpenApiPathItem> = BTreeMap::new();

        for op in operations {
            let openapi_path = op.path.clone();
            let parameters = path_parameters(&op.path);
            let operation = OpenApiOperation {
                tags: vec![op.controller_name.to_string()],
                operation_id: format!("{}_{}", op.controller_name, op.action_name),
                parameters,
                responses: BTreeMap::from([(
                    "200".to_string(),
                    OpenApiResponse {
                        description: "OK".to_string(),
                    },
                )]),
            };

            let item = paths.entry(openapi_path).or_default();
            match op.method.as_str() {
                "GET" => item.get = Some(operation),
                "POST" => item.post = Some(operation),
                "PUT" => item.put = Some(operation),
                "DELETE" => item.delete = Some(operation),
                "PATCH" => item.patch = Some(operation),
                "OPTIONS" => item.options = Some(operation),
                "HEAD" => item.head = Some(operation),
                _ => {}
            }
        }

        Self {
            openapi: "3.0.3",
            info: OpenApiInfo {
                title: title.into(),
                version: version.into(),
                description: None,
            },
            paths,
        }
    }

    /// Serializes the document to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

/// Extracts `{name}` segments from a matchit path as OpenAPI path parameters.
fn path_parameters(path: &str) -> Vec<OpenApiParameter> {
    let mut params = Vec::new();
    let mut rest = path;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        if let Some(end) = after.find('}') {
            let name = &after[..end];
            if !name.is_empty() {
                params.push(OpenApiParameter {
                    name: name.to_string(),
                    location: "path",
                    required: true,
                    schema: OpenApiSchema {
                        schema_type: "string",
                    },
                });
            }
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    params
}
