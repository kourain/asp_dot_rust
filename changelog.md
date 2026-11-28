# Change log

## 0.2.12

### 0.2.12 New

- `#[query("/search")]` and `#[route(["QUERY"], "/search")]` register HTTP QUERY actions. A missing or blank `Content-Type` produces 400. A media type rejected by the body binder produces 415.
- Route registration supports multiple paths for an action and reports duplicate controllers, duplicate method/path registrations and conflicting path patterns at startup. Registration errors identify the action and point to its route attribute. `#[cfg]` on an action also controls its registration and dispatch.
- `FromBody<T>` binds one top-level JSON member or XML child element. `FromForm<T>` binds one URL-encoded form field. `Body<T>` and `Form<T>` bind whole payloads. An action can combine several binders of the same body format, including whole-payload and field binders. The request is buffered once and remains readable from the start. Each body field binder parses the document separately.
- `#[bind(name = "abcd", default = "aaaa", validate_default = false)]` selects a request key and supplies a fallback for an absent route, query, form or body field. `validate_default` defaults to `true`. Supplied values still go through binding and validation, even when they equal the fallback. Empty query/form values and explicit JSON `null` count as supplied. A route default applies only after the URL matches. Raw Rust argument names such as `r#type` bind the key `type`.
- `#[validate(...)]` checks route, query, form and body fields before the action runs. `length` counts Unicode scalar values. `range`, `min` and `max` check finite numbers with inclusive bounds. Extension traits on `ControllerActionFieldValidate` can supply custom rules. Whole `Body<T>` and `Form<T>` DTOs can implement `Validate`. A failed rule returns 422 and identifies the field.
- Path, query and URL-encoded form fields accept types implementing `FromStr + 'static`, including `Uuid`, `IpAddr` and user types. Types with a custom parser can implement `FromParamValue`. `FromParamField`, `RequiredField`, `OptionalField` and `FieldError` distinguish missing and invalid query/form fields.
- `FromOptionQuery<T>`, `FromOptionForm<T>` and `FromOptionBody<T>` are aliases for the corresponding binder of `Option<T>`. An absent optional field yields `None` unless it has a default. A supplied value must parse. An optional JSON field accepts explicit `null`, while a missing required `FromBody<serde_json::Value>` yields `MissingField`. Field rules skip `None`. Path parameters require a concrete value.
- `controller::results` provides typed `RouteResult<T>` status helpers such as `ok`, `created`, `bad_request` and `no_content`, with PascalCase aliases. Use `headers_mut()` for response headers and `with_location()` or a redirect helper for a validated `Location` header. Custom results can borrow request data and write a buffered or streaming body. `HttpResponse::has_finished()` lets middleware detect a completed or aborted stream.
- Serializable models and collections negotiate JSON or XML using `Accept`, including weights and wildcards. JSON is the default. Negotiated responses set `Vary: Accept` and return 406 when no offered format is acceptable. Strings produce UTF-8 text. `Json`, `Xml` and `Raw` select a fixed representation.
- Controllers and middleware support field injection through `#[derive(DependencyInjectableController)]` and `#[derive(DependencyInjectableMiddleware)]`. Middleware constructor dependencies resolve when the pipeline is built. Extra parameters on `invoke_async` resolve from the request scope. Controller injection receives the current `HttpContextRef`.
- `third_party` now also re-exports `chrono`, `http`, `http_body`, `http_body_util`, `jsonwebtoken`, `matchit`, `quick_xml` and `serde_urlencoded`.

### 0.2.12 Fix

- Route lookup trims trailing slashes just as registration does. `/items`, `/items/` and `/items///` reach the same action. Path parameters are percent-decoded after matching, so `/files/a%2Fb` binds `a/b` as one parameter.
- Query and URL-encoded form fields decode `+` as a space and `%2B` as a literal plus. Repeated keys select their last value. The query parser skips empty or undecodable keys and preserves invalid UTF-8 values for binding errors. A bare boolean flag or an empty boolean value binds `true`. The optional query and form bool binders expose `.flag()`.
- CORS handles `OPTIONS` as a preflight when `Access-Control-Request-Method` is present on a qualifying cross-origin request. Ordinary OPTIONS requests reach routing. The default CORS method set includes QUERY. CORS and compression preserve existing `Vary` entries.
- Compression can use its defaults without a registered `CompressionConfiguration` and removes strong ETags when encoding changes the body. A timeout before headers are sent returns a plain-text 408 with `Cache-Control: no-store` and clears stale representation, framing and redirect headers. A timeout after headers have been sent aborts the stream. A successfully served static file sets status 200 and its content length.
- Rate limiting updates the sliding window and block state together for each client IP and periodically evicts inactive entries. Concurrent requests from the same client share the same limit.
- HTTP listeners bind the configured IP/port combinations in a deterministic order. Startup reports a required bind failure. When IPv4 wildcard binding succeeds, an unavailable IPv6 wildcard listener is skipped with a warning. Port 8080 is selected only when both port lists are empty. This version serves HTTP listeners only.

### 0.2.12 Change

| Old | New | Note |
| --- | --- | --- |
| A HEAD request required an explicitly registered HEAD action | HEAD selects that action when available and otherwise dispatches to GET | Buffered and streaming responses send headers only. Buffered GET fallbacks retain the representation length. `Allow` and `get_allowed_methods()` still list only explicitly registered methods. |
| Swagger generated OpenAPI 3.0.3 and used the controller and action names for each operation ID | OpenAPI 3.2.1 supports `query`, `trace` and `additionalOperations`. Duplicate operation IDs include the method and path. | Register `builder.add_swagger_gen()` before `build()`, then call `app.use_swagger()` to serve `/swagger` and `/swagger/v1/swagger.json`. Read registered operations through `OpenApiService::registered_operations()`. |
| Service cycles were checked during `build()` | `Application::run()` loads configuration and installs routing before checking the service graph for cycles, missing services and configuration dependencies | Errors identify the dependency path before listeners open. All `Cfg*` wrappers require a registered type, and `CfgRequire<T>` requires a populated entry. Generated controller and middleware invocation code checks direct registrations. |
| The running server held the application after pipeline construction | Startup consumes the middleware and hosted-service factories | Data used only during construction can be released while the pipeline serves requests. |
| The default watched filename was `appsetting.toml` | The default is `appsettings.toml` | Rename the file or register its path explicitly. |
| Request panic logging included the connection ID, request ID and panic payload | With `panic = "unwind"`, the report includes both IDs, the panic location, thread and backtrace | Before headers are committed the client receives 500. Afterward the server terminates the stream. Builds using `panic = "abort"` set `X-AspRs-Panic: abort` and retain process-abort behavior. `LOGGER::is_running()` exposes the logger state. |

### 0.2.12 Breaking Change

| Old | New | Note |
| --- | --- | --- |
| `FromBody<T>` and `FromForm<T>` read whole payloads | They now read a single named field | Use `Body<T>` or `Form<T>` for the whole payload. `Body<String>` and `Form<String>` read raw text. |
| `FromOptionQuery<T>` was a tuple struct and `bind` returned `Self` | It is an alias for `FromQuery<Option<T>>` and `bind` returns `Result<Self, BindError>` | Handle 422 for an invalid supplied value. Use `FromQuery(value)` as a constructor or pattern. `.0`, dereferencing and `into_inner()` still work. |
| Custom types could implement both `FromStr` and `FromParamValue` | The blanket `FromParamValue` implementation covers `FromStr + 'static` | Remove overlapping implementations. Query/form binders use `FromParamField<Marker>` for required and optional fields. |
| `ResolvedRoute::path_params` and `query_params` held `String` values | Both maps hold `Result<String, String>` | `Ok` holds decoded text and `Err` holds the original invalid input. Route binding failures return 404 and query binding failures return 422. |
| Matches on `BindError` covered the old variants | `MissingField`, `ParseField` and `Validation` also map to 422 | Add arms to exhaustive matches. |
| `PresetResponse`, raw byte results and the previous `ActionResult` methods | Use `controller::results` and `ActionResult: Send + Sync` | Wrap bytes in `Raw` to preserve a binary response. For custom results implement `status_code`, `write_header` and `write_body_async` as needed. |
| Manual dispatch used `Routing`, and `Middleware` included construction | `Controller` dispatch returns `Result<Box<dyn ActionResult + '_>, RoutingError>`. Middleware construction uses `DependencyInjectableMiddleware::inject`. | Migrate manual implementations. `Routing` remains a deprecated alias for `Controller`. |
| `RoutingServiceBuilder::add_controller_route` accepted separate route components and an invoker. `build()` returned `RoutingService`. | `add_controller_route` accepts a root and an `ActionRoute`. `build()` returns `(RoutingService, OpenApiService)`. | Update manual registrations and callers that build the routing service directly. |
| Controllers could use `ShareMutPtr` directly, and the request ID and request service provider were mutable | `ShareMutPtr` is private. `request_id` and the request's `service_provider` use `ReadOnly<T>`. | Use `http_context::HttpContextRef` for the controller context. Import `ArcAny` from the crate root. |
| `HttpRequest::max_body_size` exposed the body limit | The field was removed | Use `#[max_body_size(bytes)]` to configure a route. A manual `max_body_size` attribute of the wrong type now falls back to `HyperConfig::max_request_body_size`. |

See [model binding](docs/model_binding.md), [routing](docs/routing.md), [dependency injection](docs/dependency_injection.md), [middleware](docs/middleware.md), [Swagger](docs/swagger.md) and the [companion macro changelog](asp_dot_rust_macros/changelog.md).

## 0.2.11

### 0.2.11 New

- Basic and JWT Bearer authorization middleware: `use_authorize_basic()` parses Basic credentials and `use_authorize_bearer()` validates JWTs using `JwtAuthConfiguration`; both store the resulting `AuthUser` under `"user"`.
- Automatic Swagger / OpenAPI: call `use_swagger()` after `add_controllers()` to serve Swagger UI at `/swagger` and an OpenAPI 3.0 document at `/swagger/v1/swagger.json`. The document is generated from registered routes. See `docs/swagger.md`.

### 0.2.11 Fix

- Fix configuration registration and startup loading order so configured values are available when middleware and services are constructed.

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
