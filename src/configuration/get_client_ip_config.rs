pub struct GetClientIpConfiguration {
    pub header_name: String,
}

impl Default for GetClientIpConfiguration {
    fn default() -> Self {
        Self {
            header_name: "X-Forwarded-For".to_string(),
        }
    }
}
