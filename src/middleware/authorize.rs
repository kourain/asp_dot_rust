use crate::{
    Application,
    configuration::JwtAuthConfiguration,
    dependency_injection::{Cfg,CfgRequire},
    http_context::{HttpContext, USER_CONTEXT_KEY, http_header::HttpHeader},
    macros::middleware,
    middleware::{Middleware, MiddlewareNext},
    services::service_provider::service_provider_scope::ServiceProviderScope,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;

/// Authenticated principal parsed from the Authorization header and stored under `"user"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    /// Authentication scheme: `"Basic"` or `"Bearer"`.
    pub scheme: String,
    /// Username (Basic) or subject claim (JWT).
    pub name: String,
    /// Password from Basic credentials; empty for JWT.
    pub password: String,
    /// JWT claims as JSON when scheme is Bearer; `None` for Basic.
    pub claims: Option<JsonValue>,
}

const BASIC_SCHEME: &str = "Basic";
const BEARER_SCHEME: &str = "Bearer";

// ---- Basic (factory-based: scheme is not a DI service) ----

pub(crate) struct BasicAuthorizeMiddleware {
    next: MiddlewareNext,
}

#[async_trait::async_trait]
impl Middleware for BasicAuthorizeMiddleware {
    async fn inject(_service_scope: &ServiceProviderScope, next: MiddlewareNext) -> Self {
        Self { next }
    }
    async fn inject_invoke_async(&self, http_context: &mut HttpContext) {
        self.invoke_async(http_context).await;
    }
}

impl BasicAuthorizeMiddleware {
    async fn invoke_async(&self, http_context: &mut HttpContext) {
        match try_parse_basic(http_context) {
            Ok(user) => {
                http_context.set_item(USER_CONTEXT_KEY, user);
                self.next.next_async(http_context).await;
            }
            Err(()) => write_unauthorized(http_context),
        }
    }
}

/// Decode `Authorization: Basic base64(username:password)` into `AuthUser`.
fn try_parse_basic(http_context: &HttpContext) -> Result<AuthUser, ()> {
    let auth_header = http_context.request.headers.authorization().ok_or(())?;
    let mut parts = auth_header.splitn(2, char::is_whitespace);
    let scheme = parts.next().ok_or(())?;
    let token = parts.next().map(str::trim).filter(|s| !s.is_empty()).ok_or(())?;
    if !scheme.eq_ignore_ascii_case(BASIC_SCHEME) {
        return Err(());
    }
    let decoded = BASE64.decode(token.as_bytes()).map_err(|_| ())?;
    let credentials = String::from_utf8(decoded).map_err(|_| ())?;
    let (username, password) = credentials.split_once(':').ok_or(())?;
    if username.is_empty() {
        return Err(());
    }
    Ok(AuthUser {
        scheme: BASIC_SCHEME.to_string(),
        name: username.to_string(),
        password: password.to_string(),
        claims: None,
    })
}

// ---- JWT / Bearer (config from DI) ----

pub struct JwtAuthorizeMiddleware {
    next: MiddlewareNext,
    jwt_config: CfgRequire<JwtAuthConfiguration>,
}

#[middleware]
impl JwtAuthorizeMiddleware {
    pub fn new(next: MiddlewareNext, jwt_config: Cfg<JwtAuthConfiguration>) -> Self {
        Self {
            next,
            jwt_config: jwt_config.unwrap_or_default(),
        }
    }

    pub async fn invoke_async(&self, http_context: &mut HttpContext) {
        match try_parse_jwt(http_context, &self.jwt_config) {
            Ok(user) => {
                http_context.set_item(USER_CONTEXT_KEY, user);
                self.next.next_async(http_context).await;
            }
            Err(()) => write_unauthorized(http_context),
        }
    }
}

/// Validate `Authorization: Bearer <jwt>` with `JwtAuthConfiguration` and build `AuthUser`.
fn try_parse_jwt(http_context: &HttpContext, config: &JwtAuthConfiguration) -> Result<AuthUser, ()> {
    let auth_header = http_context.request.headers.authorization().ok_or(())?;
    let mut parts = auth_header.splitn(2, char::is_whitespace);
    let scheme = parts.next().ok_or(())?;
    let token = parts.next().map(str::trim).filter(|s| !s.is_empty()).ok_or(())?;
    if !scheme.eq_ignore_ascii_case(BEARER_SCHEME) {
        return Err(());
    }

    let mut validation = Validation::new(Algorithm::HS256);
    validation.set_issuer(&[config.token_issuer.as_str()]);
    validation.set_audience(&[config.token_audience.as_str()]);
    // expiration is enforced by default when `exp` is present

    let key = DecodingKey::from_secret(config.secret_key.as_bytes());
    let data = decode::<JsonValue>(token, &key, &validation).map_err(|_| ())?;
    let claims = data.claims;
    let name = claims
        .get("sub")
        .and_then(|v| v.as_str())
        .or_else(|| claims.get("name").and_then(|v| v.as_str()))
        .unwrap_or("")
        .to_string();

    Ok(AuthUser {
        scheme: BEARER_SCHEME.to_string(),
        name,
        password: String::new(),
        claims: Some(claims),
    })
}

fn write_unauthorized(http_context: &mut HttpContext) {
    http_context.response.status_code = http::StatusCode::UNAUTHORIZED;
    http_context.response.body = http::StatusCode::UNAUTHORIZED
        .canonical_reason()
        .unwrap_or("Unauthorized")
        .as_bytes()
        .to_vec();
}

impl Application {
    /// Require HTTP Basic authentication. On success, stores `AuthUser` under `"user"`.
    pub fn use_authorize_basic(&mut self) -> &mut Self {
        self.add_middleware_factory(|next| BasicAuthorizeMiddleware { next });
        self
    }

    /// Require JWT Bearer authentication using `JwtAuthConfiguration`. On success, stores `AuthUser` under `"user"`.
    pub fn use_authorize_jwt(&mut self) -> &mut Self {
        self.add_middleware::<JwtAuthorizeMiddleware>();
        self
    }

    /// Alias for `use_authorize_jwt`.
    pub fn use_authorize_bearer(&mut self) -> &mut Self {
        self.use_authorize_jwt()
    }
}
