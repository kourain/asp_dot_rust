use crate::http_context::http_response::TcpWriteStreamRef;
use crate::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

pub struct ResponseStream {
    pub written_size: usize,
    pub tcp_write_stream: TcpWriteStreamRef,
}

impl AsyncWrite for ResponseStream {
    async fn write_async(&mut self, data: &[u8]) -> std::io::Result<()> {
        self.tcp_write_stream.write_all(data).await?;
        self.written_size += data.len();
        Ok(())
    }
}
