use http_body_util::{BodyExt, Limited};
use hyper::body::{Bytes, Frame, Incoming};
use std::io;

/// Request body: either the live hyper stream or an in-memory buffer (ASP.NET-style buffering).
///
/// After [`HttpRequest::enable_buffering`](super::HttpRequest::enable_buffering), the body stays in
/// memory and [`HttpRequest::seek_body`](super::HttpRequest::seek_body) can reset the read position
/// so the same bytes can be read again.
pub enum RequestBody {
    /// Not fully buffered yet; reading advances the network stream once.
    Stream(Limited<Incoming>),
    /// Fully loaded into memory; `position` is the next byte to read (like `Stream.Position`).
    Buffered { data: Bytes, position: usize },
}

impl RequestBody {
    pub fn from_incoming(body: Incoming, max_body_size: usize) -> Self {
        Self::Stream(Limited::new(body, max_body_size))
    }

    /// `true` when the body has been fully copied into memory.
    pub fn is_buffered(&self) -> bool {
        matches!(self, Self::Buffered { .. })
    }

    /// Current read position when buffered; `0` when still a stream.
    pub fn position(&self) -> usize {
        match self {
            Self::Stream(_) => 0,
            Self::Buffered { position, .. } => *position,
        }
    }

    /// Total buffered length when buffered.
    pub fn len_buffered(&self) -> Option<usize> {
        match self {
            Self::Stream(_) => None,
            Self::Buffered { data, .. } => Some(data.len()),
        }
    }

    /// All buffered bytes (ignores position). `None` if not buffered yet.
    pub fn as_buffered_slice(&self) -> Option<&[u8]> {
        match self {
            Self::Stream(_) => None,
            Self::Buffered { data, .. } => Some(data.as_ref()),
        }
    }

    /// Seek the read cursor (only when buffered). `pos` may be `0..=len`.
    pub fn seek(&mut self, pos: usize) -> io::Result<()> {
        match self {
            Self::Stream(_) => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "request body is not buffered; call enable_buffering first",
            )),
            Self::Buffered { data, position } => {
                if pos > data.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "seek position past end of buffered body",
                    ));
                }
                *position = pos;
                Ok(())
            }
        }
    }

    /// Read the next body frame (stream or remaining buffered slice).
    pub async fn frame(
        &mut self,
    ) -> Option<Result<Frame<Bytes>, Box<dyn std::error::Error + Send + Sync>>> {
        match self {
            Self::Stream(limited) => match limited.frame().await {
                Some(Ok(frame)) => Some(Ok(frame)),
                Some(Err(e)) => Some(Err(Box::new(e))),
                None => None,
            },
            Self::Buffered { data, position } => {
                if *position >= data.len() {
                    return None;
                }
                let chunk = data.slice(*position..);
                *position = data.len();
                Some(Ok(Frame::data(chunk)))
            }
        }
    }

    /// Drain the stream into memory and keep a seekable buffer. No-op if already buffered.
    pub async fn enable_buffering(
        &mut self,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if matches!(self, Self::Buffered { .. }) {
            return Ok(());
        }
        let Self::Stream(limited) = std::mem::replace(
            self,
            Self::Buffered {
                data: Bytes::new(),
                position: 0,
            },
        ) else {
            return Ok(());
        };

        let mut out = Vec::new();
        let mut limited = limited;
        loop {
            match limited.frame().await {
                Some(Ok(frame)) => {
                    if let Ok(data) = frame.into_data() {
                        out.extend_from_slice(&data);
                    }
                }
                Some(Err(e)) => return Err(Box::new(e)),
                None => break,
            }
        }
        *self = Self::Buffered {
            data: Bytes::from(out),
            position: 0,
        };
        Ok(())
    }

    /// Read from the current position to the end into a `Vec<u8>`.
    ///
    /// Buffers the stream if needed. After return the position is at the end;
    /// call [`Self::seek`]`(0)` to read again.
    pub async fn read_to_end(&mut self) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        self.enable_buffering().await?;
        match self {
            Self::Buffered { data, position } => {
                let bytes = data[*position..].to_vec();
                *position = data.len();
                Ok(bytes)
            }
            Self::Stream(_) => unreachable!("enable_buffering leaves Buffered"),
        }
    }
}
