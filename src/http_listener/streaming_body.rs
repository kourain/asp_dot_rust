use hyper::body::Frame;
use http_body::Body;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

/// A streaming body that can progressively yield chunks
pub struct StreamingBody {
    rx: mpsc::Receiver<Result<hyper::body::Bytes, hyper::Error>>,
}

impl StreamingBody {
    /// Create a new streaming body with a given channel capacity
    pub fn new(capacity: usize) -> (StreamingBodySender, Self) {
        let (tx, rx) = mpsc::channel(capacity);
        let sender = StreamingBodySender { tx };
        let body = StreamingBody { rx };
        (sender, body)
    }
}

/// Sender half for streaming body
pub struct StreamingBodySender {
    tx: mpsc::Sender<Result<hyper::body::Bytes, hyper::Error>>,
}

impl StreamingBodySender {
    /// Send a chunk of data
    pub async fn send(&mut self, chunk: hyper::body::Bytes) -> Result<(), mpsc::error::SendError<Result<hyper::body::Bytes, hyper::Error>>> {
        self.tx.send(Ok(chunk)).await
    }

    /// Send an error
    pub async fn send_error(&mut self, error: hyper::Error) -> Result<(), mpsc::error::SendError<Result<hyper::body::Bytes, hyper::Error>>> {
        self.tx.send(Err(error)).await
    }
}

impl Body for StreamingBody {
    type Data = hyper::body::Bytes;
    type Error = hyper::Error;

    fn poll_frame(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(Ok(bytes))) => Poll::Ready(Some(Ok(Frame::data(bytes)))),
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(e))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
