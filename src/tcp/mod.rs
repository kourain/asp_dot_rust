use crate::{
    http_context::{http_request::TcpReadStreamRef, http_response::TcpWriteStreamRef},
    logging::LOGGER,
};
use tokio::net::TcpStream;
pub mod parser;

pub(crate) async fn run_with_split_tcp_stream_async<F, Fut>(tcp_stream: TcpStream, f: F)
where
    F: Fn(TcpReadStreamRef, TcpWriteStreamRef) -> Fut,
    Fut: std::future::Future<Output = std::io::Result<bool>>,
{
    let (mut read_half, mut write_half) = tokio::io::split(tcp_stream);
    loop {
        match f(TcpReadStreamRef::new(&mut read_half), TcpWriteStreamRef::new(&mut write_half)).await {
            Ok(bconnection) => {
                if !bconnection {
                    break; // exit the loop and end the task if the connection should be closed
                }
            }
            Err(error) => {
                match error.kind() {
                    std::io::ErrorKind::ConnectionAborted | std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::BrokenPipe => {
                        LOGGER::warn(format!("TCP connection aborted during processing: {}", error));
                        break; // exit the loop and end the task
                    }
                    _ => {
                        LOGGER::error(format!("Error processing TCP stream: {}", error));
                        break; // exit the loop and end the task
                    }
                }
            }
        }
    }
}
