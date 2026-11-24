mod socket;

use super::HttpContext;
use crate::configuration::WebSocketOptions;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use http::{HeaderMap, HeaderValue, Method, StatusCode, Version, header};
use hyper::upgrade::OnUpgrade;
use std::{
    io,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;

pub use socket::WebSocket;
pub use tokio_tungstenite::tungstenite::{Message as WebSocketMessage, protocol::CloseFrame as WebSocketCloseFrame};

pub(crate) struct WebSocketHandshake {
    upgrade: Option<OnUpgrade>,
    // HTTP/2 extended CONNECT does not use Sec-WebSocket-Accept.
    accept_key: Option<HeaderValue>,
    pub(crate) options: Option<Arc<WebSocketOptions>>,
    pub(crate) accepted: Arc<AtomicBool>,
    active_socket: Option<tokio::task::AbortHandle>,
}

fn tokens(headers: &HeaderMap, name: header::HeaderName) -> impl Iterator<Item = &str> {
    headers
        .get_all(name)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

impl WebSocketHandshake {
    pub(crate) fn from_request<B>(req: &mut http::Request<B>) -> Option<Self> {
        let headers = req.headers();
        if !tokens(headers, header::SEC_WEBSOCKET_VERSION).any(|version| version == "13") {
            return None;
        }
        let accept_key = match req.version() {
            Version::HTTP_11 if req.method() == Method::GET => {
                if !tokens(headers, header::CONNECTION).any(|token| token.eq_ignore_ascii_case("upgrade")) || !tokens(headers, header::UPGRADE).any(|token| token.eq_ignore_ascii_case("websocket")) {
                    return None;
                }
                let mut keys = headers.get_all(header::SEC_WEBSOCKET_KEY).iter();
                let key = keys.next()?.as_bytes();
                if keys.next().is_some() || key.len() != 24 || STANDARD.decode(key).ok()?.len() != 16 {
                    return None;
                }
                Some(HeaderValue::from_str(&derive_accept_key(key)).ok()?)
            }
            Version::HTTP_2 if req.method() == Method::CONNECT && req.extensions().get::<hyper::ext::Protocol>()?.as_str() == "websocket" => None,
            _ => return None,
        };
        Some(Self {
            upgrade: Some(req.extensions_mut().remove::<OnUpgrade>()?),
            accept_key,
            options: None,
            accepted: Arc::default(),
            active_socket: None,
        })
    }

    pub(crate) fn abort_socket(&mut self) {
        if let Some(socket) = self.active_socket.take() {
            socket.abort();
        }
    }
}

impl Drop for WebSocketHandshake {
    fn drop(&mut self) {
        self.abort_socket();
    }
}

/// Settings for an individual connection. Unset values inherit the middleware options.
#[derive(Debug, Default)]
pub struct WebSocketAcceptContext<'a> {
    pub sub_protocol: Option<&'a str>,
    pub keep_alive_interval: Option<Duration>,
    /// Zero disables the Pong deadline for this connection.
    pub keep_alive_timeout: Option<Duration>,
}

/// The current request's WebSocket feature, analogous to HttpContext.WebSockets.
pub struct WebSocketManager<'a>(&'a mut HttpContext);

impl HttpContext {
    pub fn web_sockets(&mut self) -> WebSocketManager<'_> {
        WebSocketManager(self)
    }
}

impl WebSocketManager<'_> {
    pub fn is_web_socket_request(&self) -> bool {
        self.0.web_socket_handshake.as_ref().is_some_and(|handshake| handshake.options.is_some())
    }

    pub fn requested_protocols(&self) -> impl Iterator<Item = &str> {
        tokens(&self.0.request.headers, header::SEC_WEBSOCKET_PROTOCOL)
    }

    pub async fn accept_web_socket_async(&mut self) -> io::Result<WebSocket> {
        self.accept_web_socket_with_context_async(WebSocketAcceptContext::default()).await
    }

    pub async fn accept_web_socket_with_context_async(&mut self, context: WebSocketAcceptContext<'_>) -> io::Result<WebSocket> {
        if self.0.response.has_started() {
            return Err(io::Error::other("HTTP response already started"));
        }
        let selected = match context.sub_protocol {
            Some(protocol) => {
                // A subprotocol uses the HTTP token grammar and is case-sensitive.
                if header::HeaderName::from_bytes(protocol.as_bytes()).is_err() || !self.requested_protocols().any(|offered| offered == protocol) {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid or unoffered WebSocket subprotocol"));
                }
                Some(HeaderValue::from_str(protocol).map_err(io::Error::other)?)
            }
            None => None,
        };
        let Some(WebSocketHandshake {
            options: Some(options),
            upgrade,
            accept_key,
            accepted,
            active_socket,
        }) = self.0.web_socket_handshake.as_mut()
        else {
            return Err(io::Error::other("WebSocket upgrade unavailable"));
        };
        let interval = context.keep_alive_interval.unwrap_or(options.keep_alive_interval);
        let timeout = context.keep_alive_timeout.or(options.keep_alive_timeout).filter(|timeout| !timeout.is_zero());
        let upgrade = upgrade.take().ok_or_else(|| io::Error::other("WebSocket upgrade already consumed"))?;
        let response = &mut self.0.response;
        response.headers.remove(header::SEC_WEBSOCKET_EXTENSIONS);
        response.headers.remove(header::SEC_WEBSOCKET_PROTOCOL);
        if let Some(protocol) = selected {
            response.headers.insert(header::SEC_WEBSOCKET_PROTOCOL, protocol);
        }
        if let Some(key) = accept_key.take() {
            response.status_code = StatusCode::SWITCHING_PROTOCOLS;
            response.headers.insert(header::CONNECTION, HeaderValue::from_static("Upgrade"));
            response.headers.insert(header::UPGRADE, HeaderValue::from_static("websocket"));
            response.headers.insert(header::SEC_WEBSOCKET_ACCEPT, key);
        } else {
            response.status_code = StatusCode::OK;
            response.headers.remove(header::CONNECTION);
            response.headers.remove(header::UPGRADE);
            response.headers.remove(header::SEC_WEBSOCKET_ACCEPT);
        }
        response.commit_upgrade()?;
        accepted.store(true, Ordering::Release);
        let transport = upgrade.await.map_err(io::Error::other)?;
        let (socket, abort_handle) = WebSocket::new(transport, interval, timeout).await;
        *active_socket = Some(abort_handle);
        Ok(socket)
    }
}
