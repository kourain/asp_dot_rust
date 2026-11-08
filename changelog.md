# Change log

## 0.2.10

### 0.2.10 New

- Model binding: action parameters `FromRoute<T>`, `FromQuery<T>`, `FromOptionQuery<T>`, `FromBody<T>`, `FromForm<T>` are resolved by the `#[controller_route]` macro (argument name = key). Route miss/parse → 404; query/body/form miss/parse or Content-Type mismatch → 422. At most one `FromBody` or `FromForm` per action. `String` body is raw UTF-8; other types use serde. See `docs/model_binding.md`.
- Request body buffering: `HttpRequest::enable_buffering` loads the body into memory; `seek_body(0)` rewinds for another read (ASP.NET EnableBuffering). `request.body` is `RequestBody` (`Stream` | `Buffered`).
- Dependencies: `serde_json` 1.0.154, `quick-xml` 0.42.0 (`serialize`), `serde_urlencoded` 0.7.1.

### 0.2.10 Change

| old | new | why |
| --- | --- | --- |
| body limit only inside `Limited` | `HttpRequest::max_body_size` on the request | so the action can read the limit in use for that request |

### 0.2.10 Breaking Change

| old | new | why |
| --- | --- | --- |
| `HttpRequest::body: Limited<Incoming>` | `HttpRequest::body: RequestBody` (`Stream` \| `Buffered`) | allow buffering and `seek_body(0)` so the body can be read more than once |

## 0.2.9

### 0.2.9 New

- `max_request_headers_size` (431) and `max_request_body_size` (413, by the declared `Content-Length`) are now applied.
- `#[max_body_size(bytes)]` sets the request body limit of a route (it is the route attribute `max_body_size`, so it can also be set on the controller impl as a default), routes without it use `max_request_body_size`. See `docs/routing.md`.
- Built-in attribute macros such as `#[max_body_size(..)]` are `#[attr(name = value)]` with a typed value; they are declared in one place (`for_each_attr_alias!` in `controller/extra_attribute.rs` of the macros crate) and need no import of `attr`.
- `#[attr(key = value, ...)]` sets attributes of a route, on the controller impl as defaults for its routes or on an action to override a key. A value is stored as its own type and read back as exactly that type (`5` is an `i32`, `5u8` a `u8`) with `HttpRequest::route_attribute::<T>(key)`, `HttpRequest::route_attributes()` or `Route::attributes()`. See `docs/routing.md`.

### 0.2.9 Fix

- Fixed: with several ports only the first one was served (ip and port were zipped), now every ip is served on every port.
- Fixed: `with_any_ip` failed on a host without IPv6 or when `[::]` already covers IPv4; `0.0.0.0` is bound first and `[::]` is skipped with a warning if it can not be bound.

### 0.2.9 Breaking Change

| old | new | why |
| --- | --- | --- |
| `HttpRequest::body: Incoming` | `HttpRequest::body: Limited<Incoming>`; `from_http` takes the body limit | enforce per-request body size limit |
| `RouteAttribute::get(&'static str)` | `RouteAttribute::get(&str)` | allow non-static attribute keys |

## 0.2.8

### 0.2.8 New

- Stream response: a complete response (controller result, `response.body`) is written into the stream when the pipeline ends; other data is sent with `response.write_async(bytes)`. See `docs/stream_response.md`.
- Response API in the style of ASP.NET: `has_started` (HasStarted), `start_async` (StartAsync), `write_async` (WriteAsync), `complete_async` (CompleteAsync) and `HttpContext::abort` (Abort). `write_bytes_async` writes owned bytes without copying them.

### 0.2.8 Change

| old | new | why |
| --- | --- | --- |
| pipeline always in one task | pipeline runs in its own task only when it waits after the head is sent | avoid an extra task when the response is complete |
| status 204 / 304 could still get `Content-Length` | no `Content-Length` for 204 or 304 | match HTTP rules for bodies |

### 0.2.8 Breaking Change

| old | new | why |
| --- | --- | --- |
| `write_async` did not truly write to the connection | `write_async(impl AsRef<[u8]>) -> io::Result<()>` writes to the connection | real streaming like ASP.NET WriteAsync |
| `write_headers_async` / `write_response_async` / `write_body_async` | `start_async` / `complete_async`; `write_body_async` removed | clearer ASP.NET-style names |
| `HttpResponse::keep_alive`, `move_to_http_response` | removed | connection is managed by hyper |

## 0.2.7

### 0.2.7 New

- Middleware in the style of ASP.NET: it holds the next middleware (`MiddlewareNext`) given to its constructor and the pipeline is built once at run. See `docs/middleware.md`.
- New `#[middleware]` on `impl T { fn new(next, ..); async fn invoke_async(&self, ctx, ..) }`, it generates `Middleware`, `Middleware` and `Middleware`. Parameters of `invoke_async` are injected on every request (as scoped from HttpContext).
- New `CompressionMiddleware` (`app.use_compression()`): gzip, deflate, brotli and zstd response compression chosen from `Accept-Encoding`, built on `async-compression`. See `docs/compression.md`.
- New `CompressionConfiguration` and `CompressionLevel` (`enable_gzip`, `enable_deflate`, `enable_brotli`, `enable_zstd`, `min_size`, `level`), can be bound from a TOML section.

### 0.2.7 Change

| old | new | why |
| --- | --- | --- |
| `add_middleware_instance` | `add_middleware_factory` | pipeline is built once; middleware holds `MiddlewareNext` |
| `middleware!` bodies and per-request `LOGGER::trace` of each middleware | `next.call_async(ctx)`; per-request trace removed | less noise, clearer next-call style |

### 0.2.7 Breaking Change

| old | new | why |
| --- | --- | --- |
| previous middleware registration style | ASP.NET Core style (`MiddlewareNext`, factory registration) | one pipeline model aligned with ASP.NET |

## 0.2.6

### 0.2.6 Fix

- Fixed: logging disable via `LOGGER::disable()` at start did not work.

## 0.2.5

### 0.2.5 New

- `LOGGER::enable_default_logger()` / `LOGGER::disable_default_logger()` let you turn off the built-in console sink while still sending logs to your own receiver via `LOGGER::spawn_log_receiver()`.
- `HttpRequest` now carries a `request_id: Ulid` (per request) and `connection_id: Arc<Ulid>` (shared across all requests on the same keep-alive connection), currently surfaced in the "Unhandled panic" error log line.
- New `HyperConfig` settings: `header_read_timeout`, `http2_keep_alive_interval`, `http2_keep_alive_timeout` (seconds; default 10 / 20 / 10). HTTP/1.1 connections now time out while waiting for request headers, and HTTP/2 connections use PING-based keep-alive to detect dead peers. Falls back to `HyperConfig::default()` if not explicitly registered.
- Added `HttpHeader::append_str` / `append_string` to append a header value instead of replacing it.

### 0.2.5 Change

| old | new | why |
| --- | --- | --- |
| `LOGGER::set_enable(bool)` | `LOGGER::enable()` / `LOGGER::disable()` | disable no longer tears down the background logger task |
| log format with optional time | default `[{level}] {timestamp} {message}`; time always on | simpler, consistent format |
| `Application::run()` ignored listener bind/serve errors | panics with a clear message | fail fast at startup |
| manual `AuthorizeMiddleware` DI | derives `DependencyInjectableService` | less boilerplate |

### 0.2.5 Breaking Change

| old | new | why |
| --- | --- | --- |
| `AspDotRustHttpHeader` | `HttpHeader` | shorter public name |
| public `http_context::http_context` / `http_context::http_header` modules | re-exports `HttpContext` / `HttpHeader` only | hide internal module layout |
| `LOGGER::set_enable`, `with_time`, `with_request_id`, `with_connection_id` | removed (see New / Change) | simpler logger API; timestamp always present |

## 0.2.4

### 0.2.4 New

- Dynamic path parameters (`/{id}`) are now supported and extracted per request into `http_context.request.routing_info.path_params`.
- Route resolution now distinguishes `404 Not Found` from `405 Method Not Allowed` instead of collapsing both into one outcome. A `405` response now also sets the `Allow` header with the methods that are actually registered for that path.
- `#[controller_route]` can now be used with no argument for an empty root prefix, instead of requiring `#[controller_route("")]`.
- Added unit tests for `RoutingServiceBuilder` and `RoutingService` (duplicate route/controller panics, route conflict panics, path parameter extraction, query string parsing, `404`/`405` resolution, `get_allowed_methods`).
- `HttpResponse` sends a `Server: ASP.RS` header by default.

### 0.2.4 Change

| old | new | why |
| --- | --- | --- |
| conflicting dynamic segments dropped silently | `RoutingServiceBuilder::build()` panics with the conflict | catch route design errors at startup |
| query decode only values; flags without `=` dropped | decode key and value; flag without `=` kept as `""` | correct query semantics |
| higher per-request routing allocations | reduced allocations on the routing path | lower memory use per request |

### 0.2.4 Breaking Change

| old | new | why |
| --- | --- | --- |
| `HttpRequest` path/version getters; `client_addr` / `local_addr` | public `uri` / `http_version`; `path()` / `query_string()`; `client_socket_addr` / `local_socket_addr`; `routing_info` | clearer fields; routing resolved before request is built |
| `HttpContext::routing_info` | removed; lives on `HttpRequest::routing_info` | single place for route resolution |
| `HttpResponse::version` | removed | unused |
