use http::{HeaderMap, HeaderValue};
pub trait AspDotRustHttpHeader {
    /// get the Authorization header value, if present
    fn authorization(&self) -> Option<String>;
    /// get the Content-Type header value, if present
    fn content_type(&self) -> Option<String>;
    /// get the Content-Length header value as a u64, if present and valid
    fn content_length(&self) -> Option<u64>;
    /// get the Origin header value, if present
    fn origin(&self) -> Option<String>;
    /// set the Content-Length header, replacing any existing value
    fn set_content_length(&mut self, length: usize);
    /// set the Content-Type header, replacing any existing value
    fn set_content_type(&mut self, content_type: &str);
    /// insert a header with a static key and a string value
    fn insert_str(&mut self, key: &'static str, value: &str);
    /// insert a header with a static key and a string value
    fn insert_string(&mut self, key: &'static str, value: String);
}
impl AspDotRustHttpHeader for HeaderMap<HeaderValue> {
    fn authorization(&self) -> Option<String> {
        self.get("Authorization").and_then(|value| value.to_str().ok()).map(|s| s.to_string())
    }
    fn content_type(&self) -> Option<String> {
        self.get("Content-Type").and_then(|value| value.to_str().ok()).map(|s| s.to_string())
    }
    fn content_length(&self) -> Option<u64> {
        self.get("Content-Length").and_then(|value| value.to_str().ok()).and_then(|s| s.parse::<u64>().ok())
    }
    fn set_content_length(&mut self, length: usize) {
        self.insert("Content-Length", HeaderValue::from_str(&length.to_string()).unwrap());
    }
    fn set_content_type(&mut self, content_type: &str) {
        self.insert("Content-Type", HeaderValue::from_str(content_type).unwrap());
    }
    fn origin(&self) -> Option<String> {
        self.get("Origin").and_then(|value| value.to_str().ok()).map(|s| s.to_string())
    }
    fn insert_str(&mut self, key: &'static str, value: &str) {
        self.insert(key, HeaderValue::from_str(value).unwrap());
    }
    fn insert_string(&mut self, key: &'static str, value: String) {
        self.insert(key, HeaderValue::from_str(&value).unwrap());
    }
}
