use std::io;

use crate::{
    configuration::GetClientIpConfiguration,
    extensions::Str,
    http_context::{RequestStream, http_header::HttpHeader},
    io::AsyncRead,
    logging::LOGGER,
    services::configuration::ApplicationConfiguration,
};

pub struct HttpRequest {
    pub method: http::Method,
    pub path: String,
    pub version: http::Version,
    pub body: Vec<u8>,
    pub headers: HttpHeader,
    pub keep_alive: bool,
    pub client_addr: std::net::IpAddr,
    request_stream: Option<RequestStream>,
}

impl HttpRequest {
    pub fn from_http(http: http::Request<Vec<u8>>) -> Self {
        let method = http.method().clone();
        let path = http.uri().path().to_string();
        let version = http.version();
        let mut headers = HttpHeader::new();
        for (key, value) in http.headers().iter() {
            if let Ok(value_str) = value.to_str() {
                headers.add(key.as_str(), value_str);
            }
        }
        Self {
            method,
            path,
            version,
            body: Vec::new(),
            headers,
            keep_alive: true,                                                   // default to true, can be updated based on headers
            client_addr: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED), // placeholder, should be set from actual connection info
            request_stream: None,                                               // not used in this constructor
        }
    }
    pub(crate) async fn from_raw(socket_client_ip: std::net::IpAddr, method: http::Method, app_config: ApplicationConfiguration, request_stream: RequestStream) -> io::Result<Self> {
        let mut http_request = Self {
            body: Vec::new(),
            method: method,
            path: String::new(),
            version: http::Version::HTTP_11,
            headers: HttpHeader::new(),
            keep_alive: true,
            client_addr: socket_client_ip,
            request_stream: Some(request_stream),
        };
        let mut tokens: Vec<String> = Vec::with_capacity(3);
        loop {
            let token = http_request.request_stream.as_mut().unwrap().read_until_maxsize_async(&[b' ', b'\r', b'\n', b'\0'], 1024).await?;
            let token = String::from_utf8_lossy(&token.0).into_owned();
            tokens.push(token);
            if tokens.len() == 3 {
                break;
            }
        }
        http_request.path = tokens.get(0).cloned().ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "invalid HTTP request: missing path"))?;
        http_request.version = http::Version::from_str(tokens.get(1).cloned().unwrap_or_else(|| "HTTP/1.1".into()).as_str()).unwrap_or(http::Version::HTTP_11);

        http_request.parse_headers_async().await?;
        let ip_header = app_config
            .get(std::any::type_name::<GetClientIpConfiguration>())
            .and_then(|v| v.downcast_ref::<GetClientIpConfiguration>())
            .map(|config| &config.header_name);
        if ip_header.is_some() {
            if let Ok(ip) = http_request.headers.get(ip_header.as_ref().unwrap()).unwrap().parse::<std::net::IpAddr>() {
                http_request.client_addr = ip
            }
        }
        Ok(http_request)
    }

    async fn parse_headers_async(&mut self) -> io::Result<()> {
        let mut total_header_size = 0;
        // max 4kb/1header
        loop {
            let (mut line, is_maxsize) = self.read_string_until_maxsize_async(&[b'\n'], 4097).await?;
            line = line.trim().to_string();
            if is_maxsize {
                LOGGER::error("Header line exceeded maximum size of 4KB");
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Header line too large"));
            }
            total_header_size += line.len();
            if total_header_size > 8192 {
                LOGGER::error("Total header size exceeded maximum of 8KB");
                return Err(io::Error::new(io::ErrorKind::InvalidData, "Total header size too large"));
            }
            if line.is_empty() {
                // A single blank line marks the end of HTTP headers.
                break;
            }

            if let Some((key, value)) = line.split_once(':') {
                self.headers.add(key.trim(), value.trim());
            }
        }
        self.keep_alive = match self.version {
            http::Version::HTTP_10 => {
                // default close
                self.headers.get("connection").map(|v| v.to_lowercase() == "keep-alive").unwrap_or(false)
            }
            _ => true,
        };
        #[allow(unreachable_code)]
        Ok(())
    }
    pub async fn read_vec_until_whitespace_async(&mut self) -> io::Result<Vec<u8>> {
        self.request_stream.as_mut().unwrap().read_until_whitespace_async().await
    }
    pub async fn read_string_until_whitespace_async(&mut self) -> io::Result<String> {
        let bytes = self.read_vec_until_whitespace_async().await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
    pub async fn read_vec_until_async(&mut self, delimiter: &[u8]) -> io::Result<Vec<u8>> {
        self.request_stream.as_mut().unwrap().read_until_async(delimiter).await
    }
    pub async fn read_string_until_async(&mut self, delimiter: &[u8]) -> io::Result<String> {
        let bytes = self.read_vec_until_async(delimiter).await?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
    pub async fn read_vec_until_maxsize_async(&mut self, delimiter: &[u8], max_size: usize) -> io::Result<(Vec<u8>, bool)> {
        self.request_stream.as_mut().unwrap().read_until_maxsize_async(delimiter, max_size).await
    }
    pub async fn read_string_until_maxsize_async(&mut self, delimiter: &[u8], max_size: usize) -> io::Result<(String, bool)> {
        let bytes = self.read_vec_until_maxsize_async(delimiter, max_size).await?;
        Ok((String::from_utf8_lossy(&bytes.0).into_owned(), bytes.1))
    }
    pub async fn read_all_body_async(&mut self) -> io::Result<Vec<u8>> {
        if let Some(content_length_str) = self.headers.get("Content-Length") {
            if let Ok(content_length) = content_length_str.parse::<usize>() {
                let mut body = vec![0u8; content_length];
                let mut total_read = 0;
                while total_read < content_length {
                    let n = self.request_stream.as_mut().unwrap().read_async(&mut body[total_read..]).await?;
                    if n == 0 {
                        break; // Connection closed
                    }
                    total_read += n;
                }
                Ok(body)
            } else {
                LOGGER::error(format!("Invalid Content-Length header value: {}", content_length_str));
                Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid Content-Length"))
            }
        } else {
            LOGGER::error("Missing Content-Length header");
            Err(io::Error::new(io::ErrorKind::InvalidData, "Missing Content-Length"))
        }
    }
    pub async fn read_all_async(&mut self) -> io::Result<Vec<u8>> {
        self.request_stream.as_mut().unwrap().read_all_async().await
    }
    pub async fn read_exact_async(&mut self, size: usize) -> io::Result<Vec<u8>> {
        self.request_stream.as_mut().unwrap().read_exact_async(size).await
    }
    pub async fn read_async(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.request_stream.as_mut().unwrap().read_async(buffer).await
    }
}
