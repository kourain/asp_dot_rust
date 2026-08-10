use std::{
    fs::File,
    io::{BufReader, Error, ErrorKind},
    path::Path,
    sync::Arc,
};

use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};

/// Build a `rustls::ServerConfig` from a PEM-encoded certificate chain file
/// and a PEM-encoded private key file.
///
/// This is called once at application build time (not per-connection), so the
/// (small) cost of reading and parsing the files from disk is paid only once.
pub(crate) fn build_tls_server_config(cert_path: &Path, key_path: &Path) -> Result<Arc<ServerConfig>, Error> {
    // rustls 0.23 requires a process-wide default crypto provider to be
    // installed before a ServerConfig can be built. We only compile the
    // "ring" backend (see Cargo.toml), so this is the only provider that
    // could ever be installed here. install_default() returns an Err if a
    // provider was already installed (e.g. by another dependency, or if this
    // function is somehow called twice); that's not a real error for us, we
    // just want *a* provider to be present, so we ignore the result.
    let _ = rustls::crypto::ring::default_provider().install_default();

    let cert_file = File::open(cert_path).map_err(|e| Error::new(e.kind(), format!("Không thể mở file chứng chỉ TLS '{}': {}", cert_path.display(), e)))?;
    let key_file = File::open(key_path).map_err(|e| Error::new(e.kind(), format!("Không thể mở file khóa riêng TLS '{}': {}", key_path.display(), e)))?;

    let mut cert_reader = BufReader::new(cert_file);
    let mut key_reader = BufReader::new(key_file);

    let cert_chain = certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Không thể phân tích chứng chỉ TLS trong '{}': {}", cert_path.display(), e)))?;

    if cert_chain.is_empty() {
        return Err(Error::new(ErrorKind::InvalidData, format!("Không tìm thấy chứng chỉ nào trong file '{}'", cert_path.display())));
    }

    let key = private_key(&mut key_reader)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Không thể phân tích khóa riêng TLS trong '{}': {}", key_path.display(), e)))?
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, format!("Không tìm thấy khóa riêng nào trong file '{}'", key_path.display())))?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .map_err(|e| Error::new(ErrorKind::InvalidData, format!("Cặp chứng chỉ/khóa TLS không hợp lệ: {}", e)))?;

    Ok(Arc::new(config))
}
