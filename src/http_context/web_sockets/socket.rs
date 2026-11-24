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
        protocol::{CloseFrame, Role, WebSocketConfig},
    },
};

type Transport = WebSocketStream<TokioIo<Upgraded>>;

/// Heartbeat settings of one connection.
#[derive(Clone, Copy)]
pub(super) struct KeepAlive {
    /// Zero disables heartbeats.
    pub(super) interval: Duration,
    /// `Some` sends Ping and requires the matching Pong in time, `None` sends unsolicited Pong.
    pub(super) timeout: Option<Duration>,
}

impl KeepAlive {
    fn next_deadline(self) -> Option<Instant> {
        if self.interval.is_zero() { None } else { Instant::now().checked_add(self.interval) }
    }
}

struct Write {
    message: Message,
    flushed: oneshot::Sender<()>,
}

/// An accepted connection. Dropping it releases the transport and stops its heartbeat.
pub struct WebSocket {
    close_timeout: Duration,
    outgoing: mpsc::Sender<Write>,
    incoming: mpsc::Receiver<Message>,
    driver: Option<JoinHandle<io::Result<()>>>,
}

fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "WebSocket is closed")
}

impl WebSocket {
    pub(super) async fn new(transport: Upgraded, keep_alive: KeepAlive, config: WebSocketConfig, close_timeout: Duration) -> (Self, AbortHandle) {
        let stream = WebSocketStream::from_raw_socket(TokioIo::new(transport), Role::Server, Some(config)).await;
        let (outgoing, writes) = mpsc::channel(1);
        let (messages, incoming) = mpsc::channel(1);
        let driver = tokio::spawn(run(stream, writes, messages, keep_alive));
        let abort_handle = driver.abort_handle();
        let socket = Self {
            close_timeout,
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
    /// When the configured close timeout expires, the connection is dropped and `Ok` is returned.
    pub async fn close_async(&mut self, frame: Option<CloseFrame>) -> io::Result<()> {
        let close_timeout = self.close_timeout;
        let handshake = async {
            // The peer may have already closed. Draining the reader also reports any transport failure.
            let _ = self.send_async(Message::Close(frame)).await;
            while self.receive_async().await?.is_some() {}
            Ok(())
        };
        match tokio::time::timeout(close_timeout, handshake).await {
            Ok(result) => result,
            Err(_) => {
                if let Some(driver) = self.driver.take() {
                    driver.abort();
                }
                Ok(())
            }
        }
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

/// State shared by the write loop and the read loop of one connection.
struct Driver {
    sink: Mutex<SplitSink<Transport, Message>>,
    closing: AtomicBool,
    keep_alive: KeepAlive,
}

async fn run(stream: Transport, writes: mpsc::Receiver<Write>, messages: mpsc::Sender<Message>, keep_alive: KeepAlive) -> io::Result<()> {
    let (sink, reader) = stream.split();
    let driver = Driver {
        sink: Mutex::new(sink),
        closing: AtomicBool::new(false),
        keep_alive,
    };
    // Reading continues while a large application write waits for the peer's receive window.
    tokio::select! {
        result = driver.write_loop(writes) => result,
        result = driver.read_loop(reader, messages) => result,
    }
}

impl Driver {
    async fn write_loop(&self, mut writes: mpsc::Receiver<Write>) -> io::Result<()> {
        while let Some(write) = writes.recv().await {
            let mut sink = self.sink.lock().await;
            if write.message.is_close() {
                self.closing.store(true, Ordering::Release);
            }
            sink.send(write.message).await.map_err(io::Error::other)?;
            let _ = write.flushed.send(());
        }
        Ok(())
    }

    async fn read_loop(&self, mut reader: SplitStream<Transport>, messages: mpsc::Sender<Message>) -> io::Result<()> {
        let mut deadline = self.keep_alive.next_deadline();
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
                                deadline = self.keep_alive.next_deadline();
                            }
                        }
                        message @ Message::Close(_) => {
                            self.flush_close_reply().await?;
                            let _ = messages.send(message).await;
                            return Ok(());
                        }
                        message => pending_message = Some(message),
                    }
                }
                () = wait_until(deadline) => deadline = self.on_heartbeat_deadline(&mut awaiting_pong).await?,
            }
        }
    }

    /// Tungstenite queued the close reply while reading. Flush it before releasing the transport.
    async fn flush_close_reply(&self) -> io::Result<()> {
        match self.sink.lock().await.flush().await {
            Ok(()) | Err(Error::ConnectionClosed) => Ok(()),
            Err(error) => Err(io::Error::other(error)),
        }
    }

    /// Runs when the heartbeat deadline passes and returns the next deadline.
    async fn on_heartbeat_deadline(&self, awaiting_pong: &mut Option<Bytes>) -> io::Result<Option<Instant>> {
        if self.closing.load(Ordering::Acquire) {
            return Ok(None);
        }
        if awaiting_pong.is_some() {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "WebSocket keep-alive Pong timed out"));
        }
        let Some(limit) = self.keep_alive.timeout else {
            self.send_heartbeat(Message::Pong(Bytes::new())).await?;
            return Ok(self.keep_alive.next_deadline());
        };
        let payload = Bytes::copy_from_slice(uuid::Uuid::now_v7().as_bytes());
        tokio::time::timeout(limit, self.send_heartbeat(Message::Ping(payload.clone())))
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "WebSocket keep-alive Ping write timed out"))??;
        *awaiting_pong = Some(payload);
        Ok(Instant::now().checked_add(limit))
    }

    /// Skips the frame when a Close is already queued, nothing may follow it.
    async fn send_heartbeat(&self, message: Message) -> io::Result<()> {
        let mut sink = self.sink.lock().await;
        if !self.closing.load(Ordering::Acquire) {
            sink.send(message).await.map_err(io::Error::other)?;
        }
        Ok(())
    }
}
