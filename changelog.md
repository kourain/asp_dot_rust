# Change log

## 0.2.7

- Middleware in the style of ASP.NET: it holds the next middleware (`MiddlewareNext`) given to its constructor and the pipeline is built once at run. See `docs/middleware.md`.
- New `#[middleware]` on `impl T { fn new(next, ..); async fn invoke_async(&self, ctx, ..) }`, it generates `Middleware`, `Middleware` and `Middleware`. Parameters of `invoke_async` are injected on every request(as scoped from HttpContext).
- `add_middleware_instance` is replaced by `add_middleware_factory`. The macro `middleware!` bodies call `next.call_async(ctx)`.
- Removed the per request `LOGGER::trace` of each middleware.
- New `CompressionMiddleware` (`app.use_compression()`): gzip, deflate, brotli and zstd response compression chosen from `Accept-Encoding`, built on `async-compression`. See `docs/compression.md`.
- New `CompressionConfiguration` and `CompressionLevel` (`enable_gzip`, `enable_deflate`, `enable_brotli`, `enable_zstd`, `min_size`, `level`), can be bound from a TOML section.

- Stream response: a complete response (controller result, `response.body`) is written into the stream when the pipeline ends; other data is sent with `response.write_async(bytes)`. See `docs/stream_response.md`.
- New `write_bytes_async`, `is_committed` and `abort` on the response. The pipeline now runs in its own task only when it waits after the head is sent.
- A response with status 204 or 304 no longer has `Content-Length`.

### Breaking changes 0.2.7

- `HttpResponse::write_async`, `write_headers_async`, `write_body_async` and `write_response_async` now really write to the connection and return `std::io::Result<()>`; `write_async` takes `impl AsRef<[u8]>`.
- Removed `HttpResponse::keep_alive` and `move_to_http_response`; the connection is managed by hyper.

### Breaking changes 0.2.6

- now you must write middleware like asp.net core

## 0.2.6

- fix logging disable `LOGGER::disable()` at start not work

## 0.2.5

### Logging

- `LOGGER::set_enable(bool)` is replaced by `LOGGER::enable()` / `LOGGER::disable()`. Disabling no longer tears down the background logger task, so re-enabling resumes logging correctly.
- `LOGGER::enable_default_logger()` / `LOGGER::disable_default_logger()` let you turn off the built-in console sink while still sending logs to your own receiver via `LOGGER::spawn_log_receiver()`.
- `LOGGER::with_time(bool)` is removed; every log entry now always carries a timestamp.
- Default log format simplified to `[{level}] {timestamp} {message}`.
- `HttpRequest` now carries a `request_id: Ulid` (per request) and `connection_id: Arc<Ulid>` (shared across all requests on the same keep-alive connection), currently surfaced in the "Unhandled panic" error log line.

### HTTP server

- New `HyperConfig` settings: `header_read_timeout`, `http2_keep_alive_interval`, `http2_keep_alive_timeout` (seconds; default 10 / 20 / 10). HTTP/1.1 connections now time out while waiting for request headers, and HTTP/2 connections use PING-based keep-alive to detect dead peers. Falls back to `HyperConfig::default()` if not explicitly registered.
- `Application::run()` now panics with a clear error message if the HTTP listener fails to bind or serve, instead of silently ignoring the error.

### Breaking changes 0.2.5

- `AspDotRustHttpHeader` trait is renamed to `HttpHeader`.
- `http_context::http_context` and `http_context::http_header` are no longer public modules; use the re-exported `http_context::HttpContext`  `http_context::HttpHeader` instead.
- `LOGGER::set_enable(bool)`, `LOGGER::with_time(bool)`, `LOGGER::with_request_id(bool)`, and `LOGGER::with_connection_id(bool)` are removed (see Logging section above for replacements).

### Internal changes 0.2.5

- `AuthorizeMiddleware` now derives `DependencyInjectableService` via macro instead of a manual `impl`.
- Added `HttpHeader::append_str` / `append_string` to append a header value instead of replacing it.

## 0.2.4

### Routing

- Dynamic path parameters (`/{id}`) are now supported and extracted per request into `http_context.request.routing_info.path_params`.
- Route resolution now distinguishes `404 Not Found` from `405 Method Not Allowed` instead of collapsing both into one outcome. A `405` response now also sets the `Allow` header with the methods that are actually registered for that path.
- `RoutingServiceBuilder::build()` now panics immediately, with the conflicting route and the underlying error, if two routes conflict on their dynamic path segments (e.g. `/{id}` vs `/{name}` at the same position). Previously such a conflict was silently dropped and the affected route would 404 at request time with no indication why.
- Query string parsing now percent-decodes both the key and the value (previously only the value was decoded), and a flag-style parameter with no `=` (e.g. `?debug`) is now kept with an empty-string value instead of being silently dropped.
- `#[controller_route]` can now be used with no argument for an empty root prefix, instead of requiring `#[controller_route("")]`.

### Breaking changes 0.2.4

- `HttpRequest`: `path: String` and `version: http::Version` fields are replaced by `uri: http::Uri` and `http_version: http::Version`; the `method()`, `uri()`, `version()`, `headers()`, `headers_mut()`, and `body_mut()` getter methods are removed in favor of public fields and `path()`/`query_string()` helpers. `client_addr`/`local_addr` are renamed to `client_socket_addr`/`local_socket_addr`. A new `routing_info: ResolvedRoute` field carries the result of route resolution.
- `HttpContext`: the `routing_info: Option<ResolvedRoute>` field is removed; routing is now resolved before the request is constructed and lives on `HttpRequest::routing_info` instead.
- `HttpResponse`: the unused `version: http::Version` field is removed.

### Internal changes 0.2.4

- Added unit tests for `RoutingServiceBuilder` and `RoutingService` (duplicate route/controller panics, route conflict panics, path parameter extraction, query string parsing, `404`/`405` resolution, `get_allowed_methods`).
- Reduced per-request allocations in the routing path (`use less memory`).
- `HttpResponse` sends a `Server: ASP.RS` header by default.
