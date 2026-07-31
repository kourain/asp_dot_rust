use http::HeaderMap;
use hyper::body::Incoming;

pub struct HttpRequest {
    pub method: http::Method,
    pub path: String,
    pub version: http::Version,
    pub body: Incoming,
    pub headers: HeaderMap<http::HeaderValue>,
    pub keep_alive: bool,
    pub client_addr: std::net::SocketAddr,
    pub local_addr: std::net::SocketAddr,
}

impl HttpRequest {
    pub fn from_http(http: http::Request<Incoming>, client_ip: std::net::SocketAddr, local_ip: std::net::SocketAddr) -> Self {
        let method = http.method().clone();
        let path = http.uri().path().to_string();
        let version = http.version();
        let mut headers = HeaderMap::new();
        for (key, value) in http.headers().iter() {
            headers.insert(key, value.clone());
        }
        Self {
            method,
            path,
            version,
            body: http.into_body(),
            headers,
            keep_alive: true, // default to true, can be updated based on headers
            client_addr: client_ip,
            local_addr: local_ip,
        }
    }
    pub fn method(&self) -> &http::Method {
        &self.method
    }
    pub fn path(&self) -> &str {
        &self.path
    }
    pub fn uri(&self) -> http::Uri {
        http::Uri::try_from(self.path.as_str()).unwrap_or_else(|_| http::Uri::from_static("/"))
    }
    pub fn version(&self) -> http::Version {
        self.version
    }
    pub fn headers(&self) -> &HeaderMap<http::HeaderValue> {
        &self.headers
    }
    pub fn headers_mut(&mut self) -> &mut HeaderMap<http::HeaderValue> {
        &mut self.headers
    }
    pub fn body_mut(&mut self) -> &mut Incoming {
        &mut self.body
    }
}
