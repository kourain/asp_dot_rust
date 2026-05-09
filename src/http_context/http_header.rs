use std::collections::HashMap;
#[derive(Debug)]
pub struct HttpHeader {
    headers: HashMap<String, String>,
    pub total_bytes: usize,
}
impl HttpHeader {
    pub fn new() -> Self {
        Self { headers: HashMap::new(), total_bytes: 0 }
    }

    pub fn add(&mut self, key: &str, value: &str) {
        if let Some(current_value) = self.headers.get(key) {
            let new_value = format!("{}, {}", current_value, value);
            self.total_bytes += key.len() + new_value.len() + 2; // +2 for ": "
            self.headers.insert(key.to_string(), new_value);
        } else {
            self.total_bytes += key.len() + value.len() + 2; // +2 for ": "
            self.headers.insert(key.to_string(), value.to_string());
        }
    }
    pub fn count(&self) -> usize {
        self.headers.len()
    }
    pub fn unpack_hashmap(self) -> HashMap<String, String> {
        self.headers
    }
    pub fn clone_hashmap(&self) -> HashMap<String, String> {
        self.headers.clone()
    }
    pub fn get(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }
    pub fn authorization(&self) -> Option<&String> {
        self.get("Authorization")
    }
    pub fn content_type(&self) -> Option<&String> {
        self.get("Content-Type")
    }
    pub fn content_length(&self) -> Option<u64> {
        self.get("Content-Length")
            .and_then(|value| value.parse::<u64>().ok())
    }
    pub fn set_content_length(&mut self, length: usize) {
        self.add("Content-Length", &length.to_string());
    }
    pub fn set_content_type(&mut self, content_type: &str) {
        self.add("Content-Type", content_type);
    }
    pub fn to_string(&self) -> String {
        self.headers.iter().map(|(k, v)| format!("{}: {}", k, v)).collect::<Vec<String>>().join("\r\n")
    }
}