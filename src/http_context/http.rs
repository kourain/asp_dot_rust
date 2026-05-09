use crate::extensions::Str;

impl Str for http::Version {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "HTTP/0.9" => Some(http::Version::HTTP_09),
            "HTTP/1.0" => Some(http::Version::HTTP_10),
            "HTTP/1.1" => Some(http::Version::HTTP_11),
            "HTTP/2.0" => Some(http::Version::HTTP_2),
            "HTTP/3.0" => Some(http::Version::HTTP_3),
            _ => None,
        }
    }
    fn as_str(&self) -> &'static str {
        match self {
            &http::Version::HTTP_09 => "HTTP/0.9",
            &http::Version::HTTP_10 => "HTTP/1.0",
            &http::Version::HTTP_11 => "HTTP/1.1",
            &http::Version::HTTP_2 => "HTTP/2.0",
            &http::Version::HTTP_3 => "HTTP/3.0",
            _ => "HTTP/1.1", // default to HTTP/1.1 for unknown versions
        }
    }
}
