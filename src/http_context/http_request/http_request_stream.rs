use crate::http_context::http_request::TcpReadStreamRef;
use crate::io::AsyncRead;
use tokio::io::AsyncReadExt;
const BUFFER_SIZE: usize = 8192; // 8KB
pub struct RequestStream {
    pub read_size: usize,
    pub read_buffer: Vec<u8>,
    pub tcp_read_stream: TcpReadStreamRef,
}

impl RequestStream {
    pub fn new(tcp_read_stream: TcpReadStreamRef) -> Self {
        Self {
            read_size: 0,
            read_buffer: Vec::with_capacity(BUFFER_SIZE + 1),
            tcp_read_stream,
        }
    }
    pub async fn read_until_whitespace_async(&mut self) -> std::io::Result<Vec<u8>> {
        self.read_until_async(&[b' ', b'\t', b'\r', b'\n']).await
    }
}

impl AsyncRead for RequestStream {
    async fn read_until_async(&mut self, delimiters: &[u8]) -> std::io::Result<Vec<u8>> {
        self.read_until_maxsize_async(delimiters, usize::MAX).await.map(|(bytes, _)| bytes)
    }
    async fn read_until_maxsize_async(&mut self, delimiters: &[u8], max_size: usize) -> std::io::Result<(Vec<u8>, bool)> {
        let mut result = Vec::new();
        let mut out_max_size = false;

        loop {
            let delimiter_pos = delimiters.iter().filter_map(|&delim| memchr::memchr(delim, &self.read_buffer)).min();

            if let Some(pos) = delimiter_pos {
                let remaining = max_size - result.len();
                let to_copy = std::cmp::min(pos, remaining);
                result.extend_from_slice(&self.read_buffer[..to_copy]);
                out_max_size = result.len() >= max_size;
                self.read_buffer.drain(..=pos);
                break;
            }

            if !self.read_buffer.is_empty() {
                let remaining = max_size - result.len();
                let to_copy = std::cmp::min(self.read_buffer.len(), remaining);
                result.extend_from_slice(&self.read_buffer[..to_copy]);
                self.read_buffer.drain(..to_copy);
                out_max_size = result.len() >= max_size;
                if out_max_size {
                    break;
                }
            }

            let mut temp = vec![0u8; BUFFER_SIZE];
            let n = self.tcp_read_stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            self.read_buffer.extend_from_slice(&temp[..n]);
        }

        Ok((result, out_max_size))
    }
    async fn read_async(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let buffered = self.read_buffer.len();

        if buffered > 0 {
            let to_copy = std::cmp::min(buffered, buffer.len());
            buffer[..to_copy].copy_from_slice(&self.read_buffer[..to_copy]);
            self.read_buffer.drain(..to_copy);
            return Ok(to_copy);
        }

        let n = self.tcp_read_stream.read(buffer).await?;
        self.read_size += n;
        Ok(n)
    }
    async fn read_all_async(&mut self) -> std::io::Result<Vec<u8>> {
        let mut temp = vec![0u8; BUFFER_SIZE];
        loop {
            let n = self.tcp_read_stream.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            self.read_buffer.extend_from_slice(&temp[..n]);
            self.read_size += n;
        }
        let result = std::mem::take(&mut self.read_buffer);
        Ok(result)
    }
    async fn read_exact_async(&mut self, size: usize) -> std::io::Result<Vec<u8>> {
        let mut buffer = vec![0u8; size];
        let mut total_read = 0;

        if !self.read_buffer.is_empty() {
            let to_copy = std::cmp::min(self.read_buffer.len(), size);
            buffer[..to_copy].copy_from_slice(&self.read_buffer[..to_copy]);
            self.read_buffer.drain(..to_copy);
            total_read += to_copy;
        }

        while total_read < size {
            let n = self.tcp_read_stream.read(&mut buffer[total_read..]).await?;
            if n == 0 {
                break;
            }
            total_read += n;
        }

        self.read_size += total_read;
        Ok(buffer)
    }
}
