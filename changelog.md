# Change log

- Automatic Swagger / OpenAPI: `app.use_swagger()` after `add_controllers()` serves Swagger UI at `/swagger` and OpenAPI 3.0 JSON at `/swagger/v1/swagger.json`. The document is built from registered routes (path, method, controller tag, path parameters). See `docs/swagger.md`.

## 0.2.10

### 0.2.10 New

- Model binding: action parameters `FromRoute<T>`, `FromQuery<T>`, `FromOptionQuery<T>`, `FromBody<T>`, `FromForm<T>` are resolved by the `#[controller_route]` macro (argument name = key). Route miss/parse → 404; query/body/form miss/parse or Content-Type mismatch → 422. At most one `FromBody` or `FromForm` per action. `String` body is raw UTF-8; other types use serde. See `docs/model_binding.md`.
- Request body buffering: `HttpRequest::enable_buffering` loads the body into memory; `seek_body(0)` rewinds for another read (ASP.NET EnableBuffering). `request.body` is `RequestBody` (`Stream` | `Buffered`).
- Async dependency injection: `DependencyInjectableService::inject` is `async` and has an `on_service_created` hook that runs after the service is created, `ServiceProviderScope::get_service::<T>()` is `async` and returns `Serv<T>`. The middleware pipeline and the hosted services are built when the application runs, `#[derive(DependencyInjectableService)]`, `#[inject]` and `#[middleware]` generate the async code.
- Every configuration is reloadable (the `draft/reloadable_cfg` design): `ConfigurationService` has `add_toml_file(path, watch_file_change)`, `configure::<T>(key)`, `configure_entry::<T>(key, allow_file)`, `insert::<T>(|cfg| ...)` and `get::<T>()` that returns a `CfgReload<T>`.
- `configure::<T>()` reads nothing, the value is empty until `ConfigurationService::reload_all_async()` runs, `Application::run` calls it before the middleware pipeline is built and a port is opened.
- TOML files are watched by default by `ConfigurationWatcherBackgroundService`, which `ApplicationBuilder::build()` adds. It watches the folder of each file, so a file replaced by an editor or created later is reloaded too, and it waits 2 seconds after the last change. Only the entries that are allowed to follow the file are updated.
- `CfgEntryAllowReloadFile` (`NotAllowed`, `Allowed`, `AllowedRegexPattern`, `AllowedAll`) tells which files may update an entry, `configure::<T>()` uses `AllowedAll`.
- A file that cannot be read or parsed, a section that does not match the type and a section that is not in the file keep the current values, the first two are logged as a warning.
- `CfgReload<T>::capture()` gives the value now as a `CfgOption<T>`, `CfgOption<T>` has `has_value()`, `unwrap()` (returns `CfgRequire<T>`, panics when there is no value) and `unwrap_or_default()`.
- The injection macros inject `CfgReload<T>` with `get::<T>()`, `CfgOption<T>` with `get::<T>().capture()` and `CfgRequire<T>` with `get::<T>().capture().unwrap()`.
- `asp_dot_rust::third_party` re-exports `tokio`, `toml` and `async_trait` for the generated code.
- Dependencies: `serde_json` 1.0.154, `quick-xml` 0.42.0 (`serialize`), `serde_urlencoded` 0.7.1, `notify` 8.2.1 and `notify-debouncer-full` 0.7.0 (file watcher), `regex` 1.13.1 (`AllowedRegexPattern`); updated `arc-swap` 1.9.4, `toml` 1.2.1, `uuid` 1.29.1.

### 0.2.10 Change

| Old | New | Note |
| --- | --- | --- |
| body limit only inside `Limited` | `HttpRequest::max_body_size` on the request | so the action can read the limit in use for that request |
| `Cfg<T>` | `CfgOption<T>` | `Cfg<T>` is a deprecated alias of `CfgOption<T>` and will be removed |
| `add_toml_cfg(path)` and `add_optional_toml_cfg(path)` | `add_toml_file(path, watch_file_change)` | nothing is read until `reload_all_async()`, a file that is missing or not valid TOML is skipped and logged instead of a panic |
| `insert::<T>(value)` | `insert::<T>(\|cfg\| ...)` | `T: Default`, the closure changes the default value |
| `add_custom_configuration(impl FnOnce(&mut T))` | `add_custom_configuration(impl Fn(&mut T))` | the closure is the one `insert` takes |
| several files with the same section: unspecified which one wins | the file added last wins | deterministic |
| `Cfg::unwrap()` returned `Arc<T>` and `CfgRequire::unwrap()` existed | `CfgOption::unwrap()` returns `CfgRequire<T>`, `CfgRequire::unwrap()` is removed | `CfgRequire<T>` derefs to `Arc<T>`, the fields of `T` are used directly |
| `ApplicationHostedService` held `Box<dyn BackGroundService>` | `ApplicationHostedService` holds a `HostedServiceFactory` | the service is created when the application runs |
| `asp_dot_rust::__async_trait` (hidden) | `asp_dot_rust::third_party::async_trait` | `third_party` also has `tokio` and `toml` |

### 0.2.10 Breaking Change

| Old | New | Note |
| --- | --- | --- |
| `HttpRequest::body: Limited<Incoming>` | `HttpRequest::body: RequestBody` (`Stream` \| `Buffered`) | allow buffering and `seek_body(0)` so the body can be read more than once |
| `DependencyInjectableService::inject(scope) -> Self` | `async fn inject(scope) -> Self` and `on_service_created` | a service can await while it is created, implement it with `third_party::async_trait` |
| `ServiceProviderScope::get_service::<T>() -> Arc<T>` | `async get_service::<T>() -> Serv<T>` | needs `.await`, `Serv<T>` derefs to `Arc<T>` |
| the tuple field of `Serv`, `Cfg`, `CfgRequire` and `CfgReload` is public | the field is `pub(crate)` | use the `Deref`, `unwrap()` or `capture()` |
| `ConfigurationService::get::<T>() -> Option<Arc<T>>`, `require::<T>()` and `get_reload::<T>()` | `get::<T>() -> CfgReload<T>` | the only getter, use `.capture()` or `.capture().unwrap()` |
| `configure::<T>(section)` parsed the section at once and panicked when it was missing | `configure::<T>(key)` reads nothing, the value is empty until `reload_all_async()` | a missing section leaves the configuration without a value, check `has_value()` |
| `configure_optional`, `configure_reload` and `reload_all()` | removed | `configure::<T>()` is reloadable and `reload_all_async()` reads every file |
| `CfgReload<T>` held `Arc<ArcSwap<T>>` | `CfgReload<T>` is a handle with `capture()` | its value is empty until the configuration is loaded |

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

| Old | New | Note |
| --- | --- | --- |
| `HttpRequest::body: Incoming` | `HttpRequest::body: Limited<Incoming>`; `from_http` takes the body limit | enforce per-request body size limit |
| `RouteAttribute::get(&'static str)` | `RouteAttribute::get(&str)` | allow non-static attribute keys |

## 0.2.8

### 0.2.8 New

- Stream response: a complete response (controller result, `response.body`) is written into the stream when the pipeline ends; other data is sent with `response.write_async(bytes)`. See `docs/stream_response.md`.
- Response API in the style of ASP.NET: `has_started` (HasStarted), `start_async` (StartAsync), `write_async` (WriteAsync), `complete_async` (CompleteAsync) and `HttpContext::abort` (Abort). `write_bytes_async` writes owned bytes without copying them.

### 0.2.8 Change

| Old | New | Note |
| --- | --- | --- |
| pipeline always in one task | pipeline runs in its own task only when it waits after the head is sent | avoid an extra task when the response is complete |
| status 204 / 304 could still get `Content-Length` | no `Content-Length` for 204 or 304 | match HTTP rules for bodies |

### 0.2.8 Breaking Change

| Old | New | Note |
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

| Old | New | Note |
| --- | --- | --- |
| `add_middleware_instance` | `add_middleware_factory` | pipeline is built once; middleware holds `MiddlewareNext` |
| `middleware!` bodies and per-request `LOGGER::trace` of each middleware | `next.call_async(ctx)`; per-request trace removed | less noise, clearer next-call style |

### 0.2.7 Breaking Change

| Old | New | Note |
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

| Old | New | Note |
| --- | --- | --- |
| `LOGGER::set_enable(bool)` | `LOGGER::enable()` / `LOGGER::disable()` | disable no longer tears down the background logger task |
| log format with optional time | default `[{level}] {timestamp} {message}`; time always on | simpler, consistent format |
| `Application::run()` ignored listener bind/serve errors | panics with a clear message | fail fast at startup |
| manual `AuthorizeMiddleware` DI | derives `DependencyInjectableService` | less boilerplate |

### 0.2.5 Breaking Change

| Old | New | Note |
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

| Old | New | Note |
| --- | --- | --- |
| conflicting dynamic segments dropped silently | `RoutingServiceBuilder::build()` panics with the conflict | catch route design errors at startup |
| query decode only values; flags without `=` dropped | decode key and value; flag without `=` kept as `""` | correct query semantics |
| higher per-request routing allocations | reduced allocations on the routing path | lower memory use per request |

### 0.2.4 Breaking Change

| Old | New | Note |
| --- | --- | --- |
| `HttpRequest` path/version getters; `client_addr` / `local_addr` | public `uri` / `http_version`; `path()` / `query_string()`; `client_socket_addr` / `local_socket_addr`; `routing_info` | clearer fields; routing resolved before request is built |
| `HttpContext::routing_info` | removed; lives on `HttpRequest::routing_info` | single place for route resolution |
| `HttpResponse::version` | removed | unused |
