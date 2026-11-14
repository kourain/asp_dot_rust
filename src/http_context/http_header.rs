use std::str::FromStr;

use http::{
    HeaderMap, HeaderValue,
    header::{AUTHORIZATION, AsHeaderName, CONTENT_LENGTH, CONTENT_TYPE, IntoHeaderName, ORIGIN},
};

/// extension for HeaderMap<HeaderValue>
pub trait HttpHeaderExt {
    /// get the Authorization header value, if present
    fn authorization(&self) -> Option<String>;
    /// get the Content-Type header value, if present
    fn content_type(&self) -> Option<String>;
    /// get the Content-Length header value as a u64, if present and valid
    fn content_length(&self) -> Option<u64>;
    /// get the Origin header value, if present
    fn origin(&self) -> Option<String>;
    fn get_parse<T>(&self, key: impl AsHeaderName) -> Option<T>
    where
        T: FromStr;
    /// set the Content-Length header, replacing any existing value
    fn set_content_length(&mut self, length: usize);
    /// set the Content-Type header, replacing any existing value
    fn set_content_type(&mut self, content_type: &str);
    fn insert_val(&mut self, key: impl IntoHeaderName, value: impl ToString);
    /// insert a header with a static key and a string value
    fn insert_str(&mut self, key: impl IntoHeaderName, value: impl AsRef<str>);
    /// append a string value to an existing header or create a new one
    fn append_str(&mut self, key: impl IntoHeaderName, value: impl AsRef<str>);
}
impl HttpHeaderExt for HeaderMap<HeaderValue> {
    fn authorization(&self) -> Option<String> {
        self.get_parse::<String>(AUTHORIZATION)
    }
    fn content_type(&self) -> Option<String> {
        self.get_parse::<String>(CONTENT_TYPE)
    }
    fn content_length(&self) -> Option<u64> {
        self.get_parse::<u64>(CONTENT_LENGTH)
    }
    fn origin(&self) -> Option<String> {
        self.get_parse::<String>(ORIGIN)
    }
    fn get_parse<T>(&self, key: impl AsHeaderName) -> Option<T>
    where
        T: FromStr,
    {
        self.get(key).and_then(|value| value.to_str().ok()).and_then(|str| T::from_str(str).ok())
    }
    fn set_content_length(&mut self, length: usize) {
        self.insert_val(CONTENT_LENGTH, length);
    }
    fn set_content_type(&mut self, content_type: &str) {
        self.insert_str(CONTENT_TYPE, content_type);
    }
    fn insert_val(&mut self, key: impl IntoHeaderName, value: impl ToString) {
        self.insert_str(key, value.to_string())
    }
    fn insert_str(&mut self, key: impl IntoHeaderName, value: impl AsRef<str>) {
        self.insert(key, HeaderValue::from_str(value.as_ref()).unwrap());
    }
    fn append_str(&mut self, key: impl IntoHeaderName, value: impl AsRef<str>) {
        self.append(key, HeaderValue::from_str(value.as_ref()).unwrap());
    }
}
