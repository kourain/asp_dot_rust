use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct CorsConfiguration {
    pub allowed_origins: HashSet<String>,
    pub allowed_methods: HashSet<http::Method>,
    pub allowed_headers: HashSet<String>,
    pub exposed_headers: HashSet<String>,
    pub allow_credentials: bool,
    pub max_age: u32,
}
impl Default for CorsConfiguration {
    fn default() -> Self {
        Self {
            allowed_origins: ["*".into()].into(),
            allowed_methods: [http::Method::GET, http::Method::POST, http::Method::PUT, http::Method::DELETE, http::Method::OPTIONS].into(),
            allowed_headers: ["*".into()].into(),
            exposed_headers: ["*".into()].into(),
            allow_credentials: false,
            max_age: 86400,
        }
    }
}
impl CorsConfiguration {
    pub fn is_origin_allowed(&self, origin: impl Into<String>) -> bool {
        let origin = origin.into();
        self.allowed_origins.get(&origin).is_some() || self.allowed_origins.contains("*".into())
    }
    pub fn allow_origin(&mut self, origin: impl Into<String>) -> &Self {
        let o = origin.into();
        // if origin is *, clear all other origins
        if self.allowed_origins.contains("*".into()) {
            self.allowed_origins.clear();
        }
        self.allowed_origins.insert(o);
        self
    }

    pub fn allow_any_origin(&mut self) -> &Self {
        self.allowed_origins = ["*".into()].into();
        self.allow_credentials = false;
        self
    }

    pub fn allow_credentials(&mut self) -> &Self {
        // credentials and origin * are mutually exclusive
        if self.allowed_origins.contains("*".into()) {
            self.allowed_origins.clear();
        }
        self.allow_credentials = true;
        self
    }

    pub fn max_age(&mut self, secs: u32) -> &Self {
        self.max_age = secs;
        self
    }

    pub fn expose_header(&mut self, header: impl Into<String>) -> &Self {
        self.exposed_headers.insert(header.into());
        self
    }
}
