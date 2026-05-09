pub struct JwtAuthConfiguration {
    pub secret_key: String,
    pub token_expiration_seconds: u64,
    pub token_issuer: String,
    pub token_audience: String,
}
impl Default for JwtAuthConfiguration {
    fn default() -> Self {
        Self {
            secret_key: "your_secret_key".to_string(),
            token_expiration_seconds: 3600, // 1 hour
            token_issuer: "your_app".to_string(),
            token_audience: "your_audience".to_string(),
        }
    }
}