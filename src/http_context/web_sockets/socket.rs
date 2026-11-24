use futures_util::{
    SinkExt, StreamExt,
    stream::{SplitSink, SplitStream},
};
use hyper::{body::Bytes, upgrade::Upgraded};
use hyper_util::rt::TokioIo;
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::{
    sync::{Mutex, mpsc, oneshot},
    task::{AbortHandle, JoinHandle},
    time::{Instant, sleep_until},
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Error, Message,
        protocol::{CloseFrame, Role},
    },
};

type Transport = WebSocketStream<TokioIo<Upgraded>>;

struct Write {
    message: Message,
    flushed: oneshot::Sender<()>,
}

/// An accepted connection. Dropping it releases the transport and stops its heartbeat.
pub struct WebSocket {
    outgoing: mpsc::Sender<Write>,
    incoming: mpsc::Receiver<Message>,
    driver: Option<JoinHandle<io::Result<()>>>,
}

fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "WebSocket is closed")
}

impl WebSocket {
    pub(super) async fn new(transport: Upgraded, interval: Duration, timeout: Option<Duration>) -> (Self, AbortHandle) {
        let stream = WebSocketStream::from_raw_socket(TokioIo::new(transport), Role::Server, None).await;
        let (outgoing, writes) = mpsc::channel(1);
        let (messages, incoming) = mpsc::channel(1);
        let driver = tokio::spawn(run(stream, writes, messages, interval, timeout));
        let abort_handle = driver.abort_handle();
        let socket = Self {
            outgoing,
            incoming,
            driver: Some(driver),
        };
        (socket, abort_handle)
    }

    /// Completes when the message has been flushed to the transport.
    pub async fn send_async(&self, message: Message) -> io::Result<()> {
        let (flushed, ack) = oneshot::channel();
        self.outgoing.send(Write { message, flushed }).await.map_err(|_| closed())?;
        ack.await.map_err(|_| closed())
    }

    /// Receives one complete text, binary or close message. Control frames are handled in the background.
    pub async fn receive_async(&mut self) -> io::Result<Option<Message>> {
        if let Some(message) = self.incoming.recv().await {
            return Ok(Some(message));
        }
        if let Some(driver) = &mut self.driver {
            let result = driver.await;
            self.driver = None;
            result.map_err(io::Error::other)??;
        }
        Ok(None)
    }

    /// Initiates a close handshake and waits for the peer's acknowledgement.
    pub async fn close_async(&mut self, frame: Option<CloseFrame>) -> io::Result<()> {
        // The peer may have already closed. Draining the reader also reports any transport failure.
        let _ = self.send_async(Message::Close(frame)).await;
        while self.receive_async().await?.is_some() {}
        Ok(())
    }
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        if let Some(driver) = &self.driver {
            driver.abort();
        }
    }
}

async fn wait_until(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

async fn run(stream: Transport, mut writes: mpsc::Receiver<Write>, messages: mpsc::Sender<Message>, interval: Duration, timeout: Option<Duration>) -> io::Result<()> {
    let (sink, reader) = stream.split();
    let sink = Mutex::new(sink);
    let closing = AtomicBool::new(false);
    // Continue reading when a large application write is waiting for the peer's receive window.
    let writer = async {
        while let Some(write) = writes.recv().await {
            let mut sink = sink.lock().await;
            if write.message.is_close() {
                closing.store(true, Ordering::Release);
            }
            sink.send(write.message).await.map_err(io::Error::other)?;
            let _ = write.flushed.send(());
        }
        Ok(())
    };
    tokio::select! {
        result = writer => result,
        result = receive(reader, &sink, &closing, messages, interval, timeout) => result,
    }
}

async fn receive(
    mut reader: SplitStream<Transport>,
    sink: &Mutex<SplitSink<Transport, Message>>,
    closing: &AtomicBool,
    messages: mpsc::Sender<Message>,
    interval: Duration,
    timeout: Option<Duration>,
) -> io::Result<()> {
    let next_heartbeat = || if interval.is_zero() { None } else { Instant::now().checked_add(interval) };
    let mut deadline = next_heartbeat();
    let mut awaiting_pong = None;
    let mut pending_message = None;
    loop {
        tokio::select! {
            // A slow handler bounds reads, but can still send a reply while the receive queue is full.
            permit = messages.reserve(), if pending_message.is_some() => {
                permit.map_err(|_| closed())?.send(pending_message.take().unwrap());
            }
            received = reader.next(), if pending_message.is_none() => {
                let Some(message) = received else { return Ok(()) };
                match message.map_err(io::Error::other)? {
                    // Tungstenite queues a matching Pong and flushes it on the next read.
                    Message::Ping(_) => {}
                    Message::Pong(payload) => {
                        if awaiting_pong.as_ref() == Some(&payload) {
                            awaiting_pong = None;
                            deadline = next_heartbeat();
                        }
                    }
                    message @ Message::Close(_) => {
                        // Tungstenite queued the close reply while reading. Flush it before releasing the transport.
                        match sink.lock().await.flush().await {
                            Ok(()) | Err(Error::ConnectionClosed) => {}
                            Err(error) => return Err(io::Error::other(error)),
                        }
                        let _ = messages.send(message).await;
                        return Ok(());
                    }
                    message => pending_message = Some(message),
                }
            }
            () = wait_until(deadline) => {
                if closing.load(Ordering::Acquire) {
                    deadline = None;
                    continue;
                }
                if awaiting_pong.is_some() {
                    return Err(io::Error::new(io::ErrorKind::TimedOut, "WebSocket keep-alive Pong timed out"));
                }
                if let Some(timeout) = timeout {
                    let payload = Bytes::copy_from_slice(uuid::Uuid::now_v7().as_bytes());
                    tokio::time::timeout(timeout, send_heartbeat(sink, closing, Message::Ping(payload.clone()))).await
                        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "WebSocket keep-alive Ping write timed out"))??;
                    awaiting_pong = Some(payload);
                    deadline = Instant::now().checked_add(timeout);
                } else {
                    send_heartbeat(sink, closing, Message::Pong(Bytes::new())).await?;
                    deadline = next_heartbeat();
                }
            }
        }
    }
}

async fn send_heartbeat(sink: &Mutex<SplitSink<Transport, Message>>, closing: &AtomicBool, message: Message) -> io::Result<()> {
    let mut sink = sink.lock().await;
    if !closing.load(Ordering::Acquire) {
        sink.send(message).await.map_err(io::Error::other)?;
    }
    Ok(())
}
