use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct CorsConfiguration {
    pub allowed_origins: HashSet<&'static str>,
    pub allowed_methods: HashSet<http::Method>,
    pub allowed_headers: HashSet<&'static str>,
    pub exposed_headers: HashSet<&'static str>,
    pub allow_credentials: bool,
    pub max_age: u32,
}
impl Default for CorsConfiguration {
    fn default() -> Self {
        Self {
            allowed_origins: ["*"].into(),
            allowed_methods: [http::Method::GET, http::Method::POST, http::Method::PUT, http::Method::DELETE, http::Method::OPTIONS].into(),
            allowed_headers: ["Content-Type", "Authorization", "X-Request-Id"].into(),
            exposed_headers: ["X-Request-Id"].into(),
            allow_credentials: false,
            max_age: 86400,
        }
    }
}
impl CorsConfiguration {
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        self.allowed_origins.get(&origin).is_some() || self.allowed_origins.contains(&"*")
    }
    pub fn allow_origin(mut self, origin: impl Into<&'static str>) -> Self {
        let o = origin.into();
        // if origin is *, clear all other origins
        if self.allowed_origins.contains(&"*") {
            self.allowed_origins.clear();
        }
        self.allowed_origins.insert(o);
        self
    }

    pub fn allow_any_origin(mut self) -> Self {
        self.allowed_origins = ["*"].into();
        self.allow_credentials = false;
        self
    }

    pub fn allow_credentials(mut self) -> Self {
        // credentials and origin * are mutually exclusive
        if self.allowed_origins.contains(&"*") {
            self.allowed_origins.clear();
        }
        self.allow_credentials = true;
        self
    }

    pub fn max_age(mut self, secs: u32) -> Self {
        self.max_age = secs;
        self
    }

    pub fn expose_header(mut self, header: impl Into<&'static str>) -> Self {
        self.exposed_headers.insert(header.into());
        self
    }
}
