use serde::Deserialize;

/// Compression level shared by all algorithms.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionLevel {
    /// Fastest compression, bigger output.
    Fastest,
    /// Default level of each algorithm.
    #[default]
    Default,
    /// Best compression, slowest.
    Best,
}

/// Settings of `CompressionMiddleware`. Missing fields fall back to the defaults.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CompressionConfiguration {
    pub enable_gzip: bool,
    pub enable_deflate: bool,
    pub enable_brotli: bool,
    pub enable_zstd: bool,
    /// Bodies smaller than this many bytes are not compressed.
    pub min_size: usize,
    pub level: CompressionLevel,
}
impl Default for CompressionConfiguration {
    fn default() -> Self {
        Self {
            enable_gzip: true,
            enable_deflate: true,
            enable_brotli: true,
            enable_zstd: true,
            min_size: 32,
            level: CompressionLevel::Default,
        }
    }
}
