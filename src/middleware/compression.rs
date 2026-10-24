use crate::{
    Application,
    configuration::{CompressionConfiguration, CompressionLevel},
    dependency_injection::Cfg,
    http_context::{HttpContext, http_header::HttpHeader, http_response::HttpResponse},
    logging::LOGGER,
    macros::inject,
    middleware::{Middleware, MiddlewareNext},
};
use async_compression::{
    Level,
    tokio::write::{BrotliEncoder, GzipEncoder, ZlibEncoder, ZstdEncoder},
};
use http::{HeaderMap, header};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;

/// Supported encodings, ordered from the least to the most preferred.
/// When the client gives the same q-value, the later variant wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Encoding {
    Deflate,
    Gzip,
    Brotli,
    Zstd,
}
impl Encoding {
    fn parse(token: &str) -> Option<Self> {
        match token.to_ascii_lowercase().as_str() {
            "deflate" => Some(Self::Deflate),
            "gzip" => Some(Self::Gzip),
            "br" => Some(Self::Brotli),
            "zstd" => Some(Self::Zstd),
            _ => None,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Deflate => "deflate",
            Self::Gzip => "gzip",
            Self::Brotli => "br",
            Self::Zstd => "zstd",
        }
    }
}

/// Parse a q-value as a number between 0 and 1000, `None` when it is invalid.
fn parse_qvalue(params: Option<&str>) -> Option<u16> {
    let Some(params) = params else { return Some(1000) };
    let value = params.trim().strip_prefix("q=").or_else(|| params.trim().strip_prefix("Q="))?;
    let q = value.trim().parse::<f32>().ok()?;
    if !(0.0..=1.0).contains(&q) {
        return None;
    }
    Some((q * 1000.0).round() as u16)
}

/// Pick the best encoding from the `Accept-Encoding` headers, `None` means no compression.
fn select_encoding(request_headers: &HeaderMap, configuration: &CompressionConfiguration) -> Option<Encoding> {
    let mut explicit: Vec<(Encoding, u16)> = Vec::new();
    let mut wildcard: Option<u16> = None;
    for item in request_headers
        .get_all(header::ACCEPT_ENCODING)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
    {
        let mut parts = item.splitn(2, ';');
        let coding = parts.next().unwrap_or("").trim();
        let Some(q) = parse_qvalue(parts.next()) else { continue };
        if coding == "*" {
            wildcard = Some(q);
        } else if let Some(encoding) = Encoding::parse(coding) {
            explicit.push((encoding, q));
        }
    }
    [
        (Encoding::Deflate, configuration.enable_deflate),
        (Encoding::Gzip, configuration.enable_gzip),
        (Encoding::Brotli, configuration.enable_brotli),
        (Encoding::Zstd, configuration.enable_zstd),
    ]
    .into_iter()
    .filter(|(_, enabled)| *enabled)
    .filter_map(|(encoding, _)| {
        let q = explicit.iter().find(|(e, _)| *e == encoding).map(|(_, q)| *q).or(wildcard)?;
        (q > 0).then_some((q, encoding))
    })
    .max()
    .map(|(_, encoding)| encoding)
}

/// Compress `data` fully in memory.
async fn encode(encoding: Encoding, level: CompressionLevel, data: &[u8]) -> std::io::Result<Vec<u8>> {
    let level = match level {
        CompressionLevel::Fastest => Level::Fastest,
        CompressionLevel::Default => Level::Default,
        CompressionLevel::Best => Level::Best,
    };
    macro_rules! run {
        ($encoder:ident) => {{
            let mut encoder = $encoder::with_quality(Vec::with_capacity(data.len() / 2), level);
            encoder.write_all(data).await?;
            encoder.shutdown().await?;
            encoder.into_inner()
        }};
    }
    Ok(match encoding {
        Encoding::Gzip => run!(GzipEncoder),
        // The HTTP "deflate" coding is the zlib format (RFC 9110), not raw deflate.
        Encoding::Deflate => run!(ZlibEncoder),
        Encoding::Brotli => run!(BrotliEncoder),
        Encoding::Zstd => run!(ZstdEncoder),
    })
}

/// Compresses the response body using the encoding chosen from the `Accept-Encoding` request header.
///
/// Register it before the middlewares that produce the body (static files, routing),
/// because the first registered middleware is the outermost one.
pub struct CompressionMiddleware {
    configuration: Arc<CompressionConfiguration>,
}
#[inject]
impl CompressionMiddleware {
    pub fn new(configuration: Cfg<CompressionConfiguration>) -> Self {
        Self {
            configuration: configuration.unwrap_or_default(),
        }
    }
}
impl CompressionMiddleware {
    fn should_compress(&self, response: &HttpResponse) -> bool {
        let status = response.status_code;
        if response.body.is_empty()
            || response.body.len() < self.configuration.min_size
            || status.is_informational()
            || status == http::StatusCode::NO_CONTENT
            || status == http::StatusCode::NOT_MODIFIED
            || status == http::StatusCode::PARTIAL_CONTENT
        {
            return false;
        }
        let headers = &response.headers;
        // never compress twice, never compress ranges
        if headers.contains_key(header::CONTENT_ENCODING) || headers.contains_key(header::CONTENT_RANGE) {
            return false;
        }
        if let Some(cache_control) = headers.get(header::CACHE_CONTROL).and_then(|v| v.to_str().ok())
            && cache_control.to_ascii_lowercase().contains("no-transform")
        {
            return false;
        }
        if let Some(content_type) = headers.get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()) {
            let content_type = content_type.trim().to_ascii_lowercase();
            // images (except svg), audio and video are already compressed; sse and grpc cannot be buffered
            let is_svg = content_type.starts_with("image/svg+xml");
            if (content_type.starts_with("image/") && !is_svg)
                || content_type.starts_with("audio/")
                || content_type.starts_with("video/")
                || content_type.starts_with("text/event-stream")
                || content_type.starts_with("application/grpc")
            {
                return false;
            }
        }
        true
    }

    async fn compress_response(&self, request_headers: &HeaderMap, response: &mut HttpResponse) {
        if !self.should_compress(response) {
            return;
        }
        // the response depends on Accept-Encoding even when this request is not compressed
        let has_vary = response
            .headers
            .get_all(header::VARY)
            .iter()
            .any(|value| value.to_str().is_ok_and(|v| v.to_ascii_lowercase().contains("accept-encoding")));
        if !has_vary {
            response.headers.append_str("Vary", "Accept-Encoding");
        }
        let Some(encoding) = select_encoding(request_headers, &self.configuration) else {
            return;
        };
        let compressed = match encode(encoding, self.configuration.level, &response.body).await {
            Ok(compressed) => compressed,
            Err(error) => {
                LOGGER::warn(format!("CompressionMiddleware: {} compression failed: {}", encoding.as_str(), error));
                return;
            }
        };
        // keep the original body when compression does not help
        if compressed.len() >= response.body.len() {
            return;
        }
        response.body = compressed;
        response.headers.remove(header::ACCEPT_RANGES);
        response.headers.set_content_length(response.body.len());
        response.headers.insert_str("Content-Encoding", encoding.as_str());
    }
}
#[async_trait::async_trait]
impl Middleware for CompressionMiddleware {
    async fn invoke_async(&self, http_context: &mut HttpContext, next: MiddlewareNext) {
        next(http_context).await;
        self.compress_response(&http_context.request.headers, &mut http_context.response).await;
    }
}
impl Application {
    pub fn use_compression(&mut self) -> &mut Self {
        self.add_middleware::<CompressionMiddleware>();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_compression::tokio::bufread::{BrotliDecoder, GzipDecoder, ZlibDecoder, ZstdDecoder};
    use tokio::io::AsyncReadExt;

    fn request_headers(accept_encoding: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(value) = accept_encoding {
            headers.insert(header::ACCEPT_ENCODING, value.parse().unwrap());
        }
        headers
    }

    fn middleware(configuration: CompressionConfiguration) -> CompressionMiddleware {
        CompressionMiddleware {
            configuration: Arc::new(configuration),
        }
    }

    fn text_response(body: Vec<u8>) -> HttpResponse {
        let mut response = HttpResponse::new_in_memory();
        response.status_code = http::StatusCode::OK;
        response.headers.set_content_type("text/plain");
        response.headers.set_content_length(body.len());
        response.body = body;
        response
    }

    async fn decode(encoding: &str, data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        match encoding {
            "gzip" => GzipDecoder::new(data).read_to_end(&mut out).await.unwrap(),
            "deflate" => ZlibDecoder::new(data).read_to_end(&mut out).await.unwrap(),
            "br" => BrotliDecoder::new(data).read_to_end(&mut out).await.unwrap(),
            "zstd" => ZstdDecoder::new(data).read_to_end(&mut out).await.unwrap(),
            other => panic!("unknown encoding {other}"),
        };
        out
    }

    #[test]
    fn select_encoding_follows_accept_encoding() {
        let cfg = CompressionConfiguration::default();
        assert_eq!(select_encoding(&request_headers(None), &cfg), None);
        assert_eq!(select_encoding(&request_headers(Some("identity")), &cfg), None);
        assert_eq!(select_encoding(&request_headers(Some("gzip")), &cfg), Some(Encoding::Gzip));
        assert_eq!(select_encoding(&request_headers(Some("GZIP")), &cfg), Some(Encoding::Gzip));
        assert_eq!(select_encoding(&request_headers(Some("gzip, deflate, br")), &cfg), Some(Encoding::Brotli));
        assert_eq!(select_encoding(&request_headers(Some("gzip, br, zstd")), &cfg), Some(Encoding::Zstd));
        assert_eq!(select_encoding(&request_headers(Some("gzip;q=0.5, deflate")), &cfg), Some(Encoding::Deflate));
        assert_eq!(select_encoding(&request_headers(Some("gzip;q=0")), &cfg), None);
        assert_eq!(select_encoding(&request_headers(Some("*")), &cfg), Some(Encoding::Zstd));
        assert_eq!(select_encoding(&request_headers(Some("zstd;q=0, br;q=0, *")), &cfg), Some(Encoding::Gzip));
        assert_eq!(select_encoding(&request_headers(Some("gzip;q=abc")), &cfg), None);
        assert_eq!(select_encoding(&request_headers(Some("compress, sdch")), &cfg), None);
    }

    #[test]
    fn select_encoding_respects_configuration() {
        let cfg = CompressionConfiguration {
            enable_brotli: false,
            enable_zstd: false,
            ..Default::default()
        };
        assert_eq!(select_encoding(&request_headers(Some("gzip, br, zstd")), &cfg), Some(Encoding::Gzip));
        assert_eq!(select_encoding(&request_headers(Some("br, zstd")), &cfg), None);
    }

    #[tokio::test]
    async fn compress_round_trip_for_every_encoding() {
        let original = "hello compression ".repeat(500).into_bytes();
        for name in ["gzip", "deflate", "br", "zstd"] {
            let mw = middleware(CompressionConfiguration::default());
            let mut response = text_response(original.clone());
            mw.compress_response(&request_headers(Some(name)), &mut response).await;
            assert_eq!(response.headers.get(header::CONTENT_ENCODING).unwrap(), name);
            assert!(response.body.len() < original.len());
            assert_eq!(response.headers.content_length(), Some(response.body.len() as u64));
            assert_eq!(decode(name, &response.body).await, original);
        }
    }

    #[tokio::test]
    async fn adds_vary_header_once() {
        let mw = middleware(CompressionConfiguration::default());
        let mut response = text_response(vec![b'a'; 1000]);
        response.headers.insert(header::VARY, "Origin".parse().unwrap());
        mw.compress_response(&request_headers(Some("gzip")), &mut response).await;
        let vary: Vec<_> = response.headers.get_all(header::VARY).iter().map(|v| v.to_str().unwrap().to_string()).collect();
        assert_eq!(vary, ["Origin", "Accept-Encoding"]);

        let mut again = text_response(vec![b'a'; 1000]);
        again.headers.insert(header::VARY, "accept-encoding".parse().unwrap());
        mw.compress_response(&request_headers(Some("gzip")), &mut again).await;
        assert_eq!(again.headers.get_all(header::VARY).iter().count(), 1);
    }

    #[tokio::test]
    async fn keeps_body_when_client_does_not_accept_compression() {
        let mw = middleware(CompressionConfiguration::default());
        let mut response = text_response(vec![b'a'; 1000]);
        mw.compress_response(&request_headers(None), &mut response).await;
        assert!(response.headers.get(header::CONTENT_ENCODING).is_none());
        assert_eq!(response.body.len(), 1000);
        assert!(response.headers.get(header::VARY).is_some());
    }

    #[tokio::test]
    async fn skips_responses_that_must_not_be_compressed() {
        let mw = middleware(CompressionConfiguration::default());
        let accept = request_headers(Some("gzip"));
        let cases: Vec<(&str, Box<dyn Fn(&mut HttpResponse)>)> = vec![
            ("small body", Box::new(|r| r.body = vec![b'a'; 10])),
            ("empty body", Box::new(|r| r.body.clear())),
            ("png image", Box::new(|r| r.headers.set_content_type("image/png"))),
            ("video", Box::new(|r| r.headers.set_content_type("video/mp4"))),
            ("event stream", Box::new(|r| r.headers.set_content_type("text/event-stream"))),
            ("already encoded", Box::new(|r| r.headers.insert_str("Content-Encoding", "br"))),
            ("content range", Box::new(|r| r.headers.insert_str("Content-Range", "bytes 0-9/100"))),
            ("partial content", Box::new(|r| r.status_code = http::StatusCode::PARTIAL_CONTENT)),
            ("no-transform", Box::new(|r| r.headers.insert_str("Cache-Control", "public, No-Transform"))),
        ];
        for (name, change) in cases {
            let mut response = text_response(vec![b'a'; 1000]);
            change(&mut response);
            let body_before = response.body.clone();
            let encoding_before = response.headers.get(header::CONTENT_ENCODING).cloned();
            mw.compress_response(&accept, &mut response).await;
            assert_eq!(response.body, body_before, "{name}: body changed");
            assert_eq!(response.headers.get(header::CONTENT_ENCODING).cloned(), encoding_before, "{name}: encoding changed");
        }
    }

    #[tokio::test]
    async fn compresses_svg_and_json() {
        let mw = middleware(CompressionConfiguration::default());
        for content_type in ["image/svg+xml", "application/json; charset=utf-8"] {
            let mut response = text_response(vec![b'a'; 1000]);
            response.headers.set_content_type(content_type);
            mw.compress_response(&request_headers(Some("gzip")), &mut response).await;
            assert_eq!(response.headers.get(header::CONTENT_ENCODING).unwrap(), "gzip", "{content_type}");
        }
    }

    #[tokio::test]
    async fn keeps_original_when_compression_is_not_smaller() {
        let mw = middleware(CompressionConfiguration { min_size: 0, ..Default::default() });
        let mut response = text_response(b"x".to_vec());
        mw.compress_response(&request_headers(Some("gzip")), &mut response).await;
        assert_eq!(response.body, b"x");
        assert!(response.headers.get(header::CONTENT_ENCODING).is_none());
    }

    #[test]
    fn configuration_is_read_from_toml_with_defaults() {
        let cfg: CompressionConfiguration = toml::from_str("enable_brotli = false\nlevel = \"fastest\"\nmin_size = 256").unwrap();
        assert!(!cfg.enable_brotli && cfg.enable_gzip && cfg.enable_deflate && cfg.enable_zstd);
        assert_eq!(cfg.level, CompressionLevel::Fastest);
        assert_eq!(cfg.min_size, 256);
    }
}
