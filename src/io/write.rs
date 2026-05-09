#[allow(async_fn_in_trait)]
pub trait AsyncWrite {
    async fn write_async(&mut self, data: &[u8]) -> std::io::Result<()>;
}
