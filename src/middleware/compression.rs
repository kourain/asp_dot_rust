use crate::{
    Application,
    configuration::{CompressionConfiguration, CompressionLevel},
    dependency_injection::Cfg,
    http_context::{HttpContext, http_header::HttpHeader, http_response::HttpResponse},
    logging::LOGGER,
    macros::middleware,
    middleware::MiddlewareNext,
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
    next: MiddlewareNext,
    configuration: Arc<CompressionConfiguration>,
}
#[middleware]
impl CompressionMiddleware {
    pub fn new(next: MiddlewareNext, configuration: Cfg<CompressionConfiguration>) -> Self {
        Self {
            next,
            configuration: configuration.unwrap_or_default(),
        }
    }
    pub async fn invoke_async(&self, http_context: &mut HttpContext) {
        self.next.next_async(http_context).await;
        self.compress_response(&http_context.request.headers, &mut http_context.response).await;
    }
}
impl CompressionMiddleware {
    fn should_compress(&self, response: &HttpResponse) -> bool {
        let status = response.status_code;
        // the head (and maybe a part of the body) is already sent, the headers can not describe a compressed body any more
        if response.has_started()
            || response.body.is_empty()
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
impl Application {
    pub fn use_compression(&mut self) -> &mut Self {
        self.add_middleware::<CompressionMiddleware>();
        self
    }
}
