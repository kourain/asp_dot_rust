# Stream Response

A response is written to the connection in one of two ways.

## Complete response

Return a value from the controller (`&str`, `String`, `Vec<u8>`, ...) or set `response.body`. Middlewares can still change the status, headers and body (for example compression). When the pipeline ends, the head and the whole body are written to the stream in 64KB chunks, with `Content-Length` set.

```rust
#[get("/")]
pub async fn index(&mut self) -> impl ActionResult {
    "<html><body>Hello world</body></html>"
}
```

## Streamed response

Call `write_async` on `http_context.response`. It takes anything that is `AsRef<[u8]>` (`&[u8]`, `&str`, `Vec<u8>`, `String`). `write_bytes_async` takes `Vec<u8>`, `String`, `&'static str` or `Bytes` without copying.

```rust
#[get("/stream")]
pub async fn stream(&mut self) -> impl ActionResult {
    self.http_context.response.headers.insert_str("Content-Type", "text/plain");
    for part in ["one\n", "two\n"] {
        if self.http_context.response.write_async(part).await.is_err() {
            break; // the client is gone
        }
    }
    ""
}
```

## Rules

- The first `write_async` (or `write_headers_async`) sends the status and headers. Changes after that are ignored.
- A status that is still the default (204) is sent as 200 when you stream.
- No `Content-Length` is added when streaming, the body is chunked (HTTP/1.1) unless you set the header yourself.
- `write_async` waits when the client is slow and returns an error when the client has disconnected.
- Bytes still in `response.body` when the pipeline ends are written after the streamed bytes. The returned `""` adds nothing.
- `CompressionMiddleware` skips a response that is already streamed.
- A panic or a request timeout after the first write breaks the stream, so the client does not see a body that looks complete. Before the first write they give 500 and 408.
- `response.abort()` breaks the stream by hand.
