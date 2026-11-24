use asp_dot_rust::{
    ApplicationBuilder, api_controller,
    configuration::{RequestTimeoutConfiguration, WebSocketOptions},
    controller::{ActionResult, results, route},
    http_context::{HttpContextRef, WebSocketAcceptContext, WebSocketMessage},
    macros::controller,
};
use futures_util::{SinkExt, StreamExt};
use http::{Request, StatusCode, header};
use http_body_util::Empty;
use hyper::body::Bytes;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::time::Duration;
use tokio::{
    net::TcpStream,
    time::{sleep, timeout},
};
use tokio_tungstenite::{
    WebSocketStream, client_async,
    tungstenite::{
        Error,
        client::IntoClientRequest,
        protocol::{CloseFrame, Role, frame::coding::CloseCode},
    },
};

api_controller!(pub SocketController {});

#[controller]
impl SocketController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    #[route(["GET", "CONNECT"], "/ws")]
    async fn echo(&mut self) -> impl ActionResult {
        let mut manager = self.http_context.web_sockets();
        if !manager.is_web_socket_request() {
            return results::bad_request(());
        }
        let protocol = manager.requested_protocols().find(|p| *p == "chat").map(str::to_owned);
        let context = WebSocketAcceptContext {
            sub_protocol: protocol.as_deref(),
            ..Default::default()
        };
        let mut socket = manager.accept_web_socket_with_context_async(context).await.unwrap();
        assert!(manager.accept_web_socket_async().await.is_err());
        while let Ok(Some(message)) = socket.receive_async().await {
            if message.is_close() {
                break;
            }
            if message == WebSocketMessage::text("close") {
                socket
                    .close_async(Some(CloseFrame {
                        code: CloseCode::Normal,
                        reason: "done".into(),
                    }))
                    .await
                    .unwrap();
                break;
            }
            if socket.send_async(message).await.is_err() {
                break;
            }
        }
        results::ok(())
    }

    #[route(["GET", "CONNECT"], "/optional-protocol")]
    async fn optional_protocol(&mut self) -> impl ActionResult {
        let mut ws = self.http_context.web_sockets();
        let preferred = WebSocketAcceptContext {
            sub_protocol: Some("chat"),
            ..Default::default()
        };
        let mut socket = match ws.accept_web_socket_with_context_async(preferred).await {
            Ok(socket) => socket,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => ws.accept_web_socket_async().await.unwrap(),
            Err(_) => return results::bad_request(()),
        };
        socket.close_async(None).await.unwrap();
        results::ok(())
    }

    #[route(["GET"], "/request-lifetime")]
    async fn request_lifetime(&mut self) -> impl ActionResult {
        let mut socket = self.http_context.web_sockets().accept_web_socket_async().await.unwrap();
        tokio::spawn(async move { socket.receive_async().await });
        results::ok(())
    }
}

async fn server(options: Option<WebSocketOptions>) -> u16 {
    let port = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
    let mut builder = ApplicationBuilder::new("WebSocketTest");
    builder.with_loopback_ip().with_http_port(port).add_controllers();
    builder.configuration.insert::<RequestTimeoutConfiguration>(|cfg| cfg.timeout_seconds = 1);
    let mut app = builder.build();
    app.use_request_timeout();
    if let Some(options) = options {
        app.use_web_sockets_with_options(options);
    }
    tokio::spawn(app.run());
    timeout(Duration::from_secs(5), async {
        while TcpStream::connect(("127.0.0.1", port)).await.is_err() {
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    port
}

fn request(port: u16) -> Request<()> {
    format!("ws://127.0.0.1:{port}/ws").into_client_request().unwrap()
}

async fn handshake(port: u16, req: Request<()>) -> Result<(WebSocketStream<TcpStream>, http::Response<Option<Vec<u8>>>), Error> {
    let stream = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    timeout(Duration::from_secs(5), client_async(req, stream)).await.unwrap()
}

#[tokio::test]
async fn http1_subprotocol_echo_ping_and_close_survive_request_timeout() {
    let port = server(Some(WebSocketOptions::default())).await;
    let mut req = request(port);
    req.headers_mut().insert(header::SEC_WEBSOCKET_PROTOCOL, "other, chat".parse().unwrap());
    let (mut socket, response) = handshake(port, req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);
    assert_eq!(response.headers()[header::SEC_WEBSOCKET_PROTOCOL], "chat");
    sleep(Duration::from_millis(1100)).await;
    timeout(Duration::from_secs(5), async {
        for message in [WebSocketMessage::text("hello"), WebSocketMessage::binary(vec![0, 255])] {
            socket.send(message.clone()).await.unwrap();
            assert_eq!(socket.next().await.unwrap().unwrap(), message);
        }
        socket.send(WebSocketMessage::Ping("client".into())).await.unwrap();
        assert_eq!(socket.next().await.unwrap().unwrap(), WebSocketMessage::Pong("client".into()));
        let frame = CloseFrame {
            code: CloseCode::Normal,
            reason: "bye".into(),
        };
        socket.close(Some(frame.clone())).await.unwrap();
        assert_eq!(socket.next().await.unwrap().unwrap(), WebSocketMessage::Close(Some(frame)));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn middleware_is_opt_in_and_only_filters_origins_of_valid_handshakes() {
    let port = server(Some(WebSocketOptions {
        allowed_origins: vec!["https://example.org".into()],
        ..Default::default()
    }))
    .await;
    let mut req = request(port);
    req.headers_mut().insert(header::ORIGIN, "https://other.test".parse().unwrap());
    assert!(matches!(handshake(port, req.clone()).await, Err(Error::Http(resp)) if resp.status() == StatusCode::FORBIDDEN));
    req.headers_mut().insert(header::SEC_WEBSOCKET_KEY, "invalid".parse().unwrap());
    assert!(matches!(handshake(port, req).await, Err(Error::Http(resp)) if resp.status() == StatusCode::BAD_REQUEST));
    let mut allowed = request(port);
    allowed.headers_mut().insert(header::ORIGIN, "https://example.org".parse().unwrap());
    assert!(handshake(port, allowed).await.is_ok());
    assert!(handshake(port, request(port)).await.is_ok());
    let disabled = server(None).await;
    assert!(matches!(handshake(disabled, request(disabled)).await, Err(Error::Http(resp)) if resp.status() == StatusCode::BAD_REQUEST));
}

#[tokio::test]
async fn returning_from_the_action_terminates_a_socket_held_by_a_background_task() {
    let port = server(Some(Default::default())).await;
    let req = format!("ws://127.0.0.1:{port}/request-lifetime").into_client_request().unwrap();
    let (mut socket, _) = handshake(port, req).await.unwrap();
    assert!(matches!(timeout(Duration::from_secs(3), socket.next()).await.unwrap(), Some(Err(_)) | None));
}

#[tokio::test]
async fn an_unoffered_subprotocol_can_be_declined_before_the_response_is_committed() {
    let port = server(Some(Default::default())).await;
    let req = format!("ws://127.0.0.1:{port}/optional-protocol").into_client_request().unwrap();
    let (mut socket, response) = handshake(port, req).await.unwrap();
    assert!(!response.headers().contains_key(header::SEC_WEBSOCKET_PROTOCOL));
    assert_eq!(timeout(Duration::from_secs(3), socket.next()).await.unwrap().unwrap().unwrap(), WebSocketMessage::Close(None));
    socket.flush().await.unwrap();
}

#[tokio::test]
async fn http2_extended_connect_uses_the_same_action_and_keeps_other_streams_available() {
    let port = server(Some(WebSocketOptions::default())).await;
    let tcp = TcpStream::connect(("127.0.0.1", port)).await.unwrap();
    let (mut sender, connection) = hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(tcp)).await.unwrap();
    tokio::spawn(connection);
    let uri = format!("http://127.0.0.1:{port}/ws");
    timeout(Duration::from_secs(5), async {
        let plain_get = || Request::get(&uri).body(Empty::<Bytes>::new()).unwrap();
        assert_eq!(sender.send_request(plain_get()).await.unwrap().status(), StatusCode::BAD_REQUEST);
        let req = Request::connect(&uri)
            .version(http::Version::HTTP_2)
            .extension(hyper::ext::Protocol::from_static("websocket"))
            .header(header::SEC_WEBSOCKET_VERSION, "13")
            .body(Empty::<Bytes>::new())
            .unwrap();
        let response = sender.send_request(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert!(!response.headers().contains_key(header::SEC_WEBSOCKET_ACCEPT));
        let upgraded = hyper::upgrade::on(response).await.unwrap();
        let mut socket = WebSocketStream::from_raw_socket(TokioIo::new(upgraded), Role::Client, None).await;
        let message = WebSocketMessage::text("hello from HTTP/2");
        socket.send(message.clone()).await.unwrap();
        assert_eq!(socket.next().await.unwrap().unwrap(), message);
        assert_eq!(sender.send_request(plain_get()).await.unwrap().status(), StatusCode::BAD_REQUEST);
        socket.send(WebSocketMessage::text("close")).await.unwrap();
        assert_eq!(
            socket.next().await.unwrap().unwrap(),
            WebSocketMessage::Close(Some(CloseFrame {
                code: CloseCode::Normal,
                reason: "done".into()
            }))
        );
        socket.flush().await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn keep_alive_requires_the_matching_pong_only_when_a_timeout_is_configured() {
    let mut opts = WebSocketOptions {
        keep_alive_interval: Duration::from_millis(50),
        ..Default::default()
    };
    let port = server(Some(opts.clone())).await;
    let (mut socket, _) = handshake(port, request(port)).await.unwrap();
    assert!(matches!(timeout(Duration::from_secs(3), socket.next()).await.unwrap(), Some(Ok(WebSocketMessage::Pong(_)))));
    opts.keep_alive_timeout = Some(Duration::from_millis(250));
    let port = server(Some(opts)).await;
    let (mut socket, _) = handshake(port, request(port)).await.unwrap();
    timeout(Duration::from_secs(3), async {
        assert!(matches!(socket.next().await, Some(Ok(WebSocketMessage::Ping(_)))));
        socket.flush().await.unwrap(); // tungstenite replies to this Ping.
        assert!(matches!(socket.next().await, Some(Ok(WebSocketMessage::Ping(_)))));
        socket.send(WebSocketMessage::Pong("unrelated".into())).await.unwrap();
        // Replacing the queued automatic Pong leaves the second Ping unanswered.
        assert!(matches!(socket.next().await, Some(Err(_)) | None));
    })
    .await
    .unwrap();
}
