# WebSockets

Register the middleware with `app.use_web_sockets()`. The application chooses which endpoints accept a WebSocket. Register both `GET` and `CONNECT` on the same controller action so it handles HTTP/1.1 Upgrade and HTTP/2 extended CONNECT. The handshake commits HTTP 101 on HTTP/1.1 and HTTP 200 on HTTP/2. [WebSockets in ASP.NET Core 10](https://learn.microsoft.com/en-us/aspnet/core/fundamentals/websockets?view=aspnetcore-10.0)

```rust
use asp_dot_rust::{
    ApplicationBuilder, api_controller,
    controller::{ActionResult, results, route},
    http_context::HttpContextRef,
    macros::controller,
};

api_controller!(pub EchoController {});

#[controller]
impl EchoController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    #[route(["GET", "CONNECT"], "/ws")]
    async fn echo(&mut self) -> impl ActionResult {
        let mut ws = self.http_context.web_sockets();
        if !ws.is_web_socket_request() {
            return results::bad_request(());
        }
        let Ok(mut socket) = ws.accept_web_socket_async().await else {
            return results::internal_server_error(());
        };
        while let Ok(Some(message)) = socket.receive_async().await {
            if message.is_close() || socket.send_async(message).await.is_err() {
                break;
            }
        }
        results::ok(())
    }
}

#[tokio::main]
async fn main() {
    let mut builder = ApplicationBuilder::new("Echo");
    builder.add_controllers();
    let mut app = builder.build();
    app.use_web_sockets();
    app.run().await;
}
```

Connect to `ws://127.0.0.1:8080/ws` and send a text or binary message. The handler awaits the session, keeping request-scoped services available. The socket acknowledges a peer's Close automatically. To initiate a close, call `socket.close_async(frame).await`. Ending the request or calling `HttpContext::abort()` terminates an active socket. Once the upgrade is committed, the socket owns the wire and the router skips writing the action result.

## Options

```rust
use std::time::Duration;

app.use_web_sockets_with_options(|options| {
    options.allowed_origins = vec!["https://chat.example.com".into()];
    options.keep_alive_interval = Duration::from_secs(30);
    options.keep_alive_timeout = Some(Duration::from_secs(10));
    options.close_timeout = Duration::from_secs(3);
    options.max_message_size = Some(1 << 20);
    options.max_frame_size = Some(1 << 20);
});
```

- **Origins.** The default list contains `"*"` and allows all origins. To restrict origins, replace the list as shown above. A valid handshake with an unlisted Origin receives HTTP 403. An empty list allows all origins, and clients may omit Origin. Apply authentication and authorization before accepting the socket.
- **Keep-alive.** By default, the server sends a Ping every two minutes and waits one second for a matching Pong. Setting `keep_alive_timeout` to `None` or zero selects unsolicited Pong frames. A zero interval disables heartbeats. Request timeouts apply until the handshake commits.
- **Subprotocols.** Read the client's offers with `ws.requested_protocols()`. Pass the selected token in `WebSocketAcceptContext { sub_protocol: Some("chat"), ..Default::default() }` to `accept_web_socket_with_context_async`. An invalid or unoffered selection returns an error before the response starts. The same context can override the keep-alive interval and timeout per connection.
- **Size limits.** `max_message_size` (default 64 MiB) and `max_frame_size` (default 16 MiB) limit incoming data, `None` removes the limit. A larger message ends the connection and `receive_async` returns an error.
- **Closing.** `close_timeout` limits how long `close_async` waits for the closing handshake. It defaults to one second. If the deadline expires, the server drops the connection and returns `Ok(())`.
- **Compression.** WebSocket frames are uncompressed. The HTTP response compression middleware skips the committed handshake.
