# 0.2.4

## Routing

- Dynamic path parameters (`/{id}`) are now supported and extracted per
  request into `http_context.request.routing_info.path_params`.
- Route resolution now distinguishes `404 Not Found` from
  `405 Method Not Allowed` instead of collapsing both into one outcome.
  A `405` response now also sets the `Allow` header with the methods that
  are actually registered for that path.
- `RoutingServiceBuilder::build()` now panics immediately, with the
  conflicting route and the underlying error, if two routes conflict on
  their dynamic path segments (e.g. `/{id}` vs `/{name}` at the same
  position). Previously such a conflict was silently dropped and the
  affected route would 404 at request time with no indication why.
- Query string parsing now percent-decodes both the key and the value
  (previously only the value was decoded), and a flag-style parameter with
  no `=` (e.g. `?debug`) is now kept with an empty-string value instead of
  being silently dropped.
- `#[controller_route]` can now be used with no argument for an empty root
  prefix, instead of requiring `#[controller_route("")]`.

## Breaking changes

- `HttpRequest`: `path: String` and `version: http::Version` fields are
  replaced by `uri: http::Uri` and `http_version: http::Version`; the
  `method()`, `uri()`, `version()`, `headers()`, `headers_mut()`, and
  `body_mut()` getter methods are removed in favor of public fields and
  `path()`/`query_string()` helpers. `client_addr`/`local_addr` are renamed
  to `client_socket_addr`/`local_socket_addr`. A new `routing_info:
  ResolvedRoute` field carries the result of route resolution.
- `HttpContext`: the `routing_info: Option<ResolvedRoute>` field is removed;
  routing is now resolved before the request is constructed and lives on
  `HttpRequest::routing_info` instead.
- `HttpResponse`: the unused `version: http::Version` field is removed.

## Internal

- Added unit tests for `RoutingServiceBuilder` and `RoutingService`
  (duplicate route/controller panics, route conflict panics, path
  parameter extraction, query string parsing, `404`/`405` resolution,
  `get_allowed_methods`).
- Reduced per-request allocations in the routing path (`use less memory`).
- `HttpResponse` sends a `Server: ASP.RS` header by default.
