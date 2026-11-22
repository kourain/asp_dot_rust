# Stream Response

A response is written to the connection in one of two ways.

## Complete response

Return an action result such as `results::ok(body)` to send a buffered controller response. The router clears `response.body` before the result writes its body. Middleware can set the status and body itself when it handles a request, or modify the buffered response after `next_async` returns. At completion the framework sends `Content-Length` and writes the buffer in chunks of up to 64 KiB. For 1xx, 204 and 304 it removes `Content-Length` and skips the body.

```rust
#[get("/")]
pub async fn index(&mut self) -> impl ActionResult {
    results::ok("<html><body>Hello world</body></html>")
}
```

## Streamed response

Call `write_async` on `http_context.response`. It takes anything that is `AsRef<[u8]>` (`&[u8]`, `&str`, `Vec<u8>`, `String`). `write_bytes_async` takes `Vec<u8>`, `String`, `&'static str` or `Bytes` without copying.

```rust
#[get("/stream")]
pub async fn stream(&mut self) -> impl ActionResult {
    use asp_dot_rust::http_context::HttpHeaderExt;
    self.http_context.response.headers.insert_str("Content-Type", "text/plain");
    for part in ["one\n", "two\n"] {
        if self.http_context.response.write_async(part).await.is_err() {
            break; // the client is gone
        }
    }
    results::ok("")
}
```

## ASP.NET style API

| ASP.RS | ASP.NET Core |
|---|---|
| `response.has_started()` | `Response.HasStarted` |
| `response.start_async()` | `Response.StartAsync()` |
| `response.write_async(bytes)` | `Response.WriteAsync(text)` / `Response.Body.WriteAsync(bytes)` |
| `response.complete_async()` | `Response.CompleteAsync()` |
| `http_context.abort()` | `HttpContext.Abort()` |

## Rules

- The first `write_async` (or `start_async`) sends the status and headers. Changes after that are ignored.
- A status that is still the default (204) is sent as 200 when you stream.
- No `Content-Length` is added when streaming, the body is chunked (HTTP/1.1) unless you set the header yourself.
- `write_async` waits when the client is slow and returns an error when the client has disconnected.
- To append text after streamed bytes, return `results::ok("tail")`. The router clears the action’s buffer, writes the result, and flushes the remaining bytes at pipeline completion. `results::ok("")` appends nothing. If the action calls `complete_async()` first, the router skips the returned result.
- `complete_async` writes the head and `response.body` and ends the stream now, instead of when the pipeline ends.
- `CompressionMiddleware` skips a response that is already streamed.
- When `use_request_timeout()` is enabled, a timeout returns HTTP 408 before the headers are sent and breaks the stream afterward.
- With `panic = "unwind"`, a panic returns HTTP 500 before the headers are sent and breaks the stream afterward. With `panic = "abort"`, a panic terminates the process.
- `http_context.abort()` breaks the stream by hand. It does nothing before the first write.
