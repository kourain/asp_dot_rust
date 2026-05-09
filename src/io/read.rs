#[allow(async_fn_in_trait)]
pub trait AsyncRead {
    /// Reads from the stream until the specified delimiter is encountered.
    async fn read_until_async(&mut self, delimiter: &[u8]) -> std::io::Result<Vec<u8>>;
    /// Reads from the stream until the specified delimiter is encountered or the maximum size is reached.
    async fn read_until_maxsize_async(&mut self, delimiter: &[u8], max_size: usize) -> std::io::Result<(Vec<u8>, bool)>;
    /// Reads from the stream until any of the specified delimiters is encountered or the maximum size is reached.
    async fn read_async(&mut self, buffer: &mut [u8]) -> std::io::Result<usize>;
    /// Reads all remaining data from the stream until it is closed.
    async fn read_all_async(&mut self) -> std::io::Result<Vec<u8>>;
    /// Reads exactly the specified number of bytes from the stream.
    async fn read_exact_async(&mut self, size: usize) -> std::io::Result<Vec<u8>>;
}
