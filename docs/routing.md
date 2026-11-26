# Routing

ASP.RS matches incoming requests to controller actions using
[`matchit`](https://crates.io/crates/matchit), the same radix-tree router
used by `axum` and `hyper`'s own examples. `add_controllers()` invokes the registered controller callbacks and inserts their routes into the application builder's router. `Application::run()` installs the finished router before it builds the middleware pipeline.

## Core concepts

| Concept | Type | Purpose |
| ------- | ---- | ------- |
| Route collector | `RoutingServiceBuilder` | Each `ApplicationBuilder` has its own router. The process-wide `inventory` contains controller registration callbacks. |
| Compiled router | `RoutingService` | Registered as a singleton by `ApplicationBuilder::build()`. `Application::run()` stores the router prepared by `RoutingServiceBuilder` before initializing middleware. Read-only after startup. |
| Resolution result | `ResolvedRoute` | Carries the matched `RoutingResult`, extracted `path_params`, and parsed `query_params` for one request. Reachable at `http_context.request.routing_info`. |
| Match outcome | `RoutingResult` | `Found(info)`, `NotFound`, or `MethodNotAllowed(allowed_methods)`. |

## Declaring routes on a controller

```rust
use asp_dot_rust::{
    api_controller,
    controller::{ActionResult, get, post, results, route},
    http_context::HttpContextRef,
    macros::{controller_inject, controller_route},
    third_party::async_trait,
};

api_controller!(pub HomeController {
    temp: String,
});

#[controller_route("home")] // root prefix; omit the argument (`#[controller_route]`) for no prefix
#[controller_inject]
impl HomeController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { temp: Default::default(), http_context }
    }

    #[get("/")] // -> GET /home
    pub async fn index(&mut self) -> impl ActionResult {
        results::ok("Hello, World!")
    }

    #[get("/{id}")] // -> GET /home/{id}, a dynamic path parameter
    pub async fn show(&mut self) -> impl ActionResult {
        let id = self.http_context.request.routing_info.path_params.get("id").and_then(|value| value.as_deref().ok()).unwrap_or_default();
        results::ok(format!("home #{id}"))
    }

    #[post("/update")] // -> POST /home/update
    pub fn update(&mut self) -> impl ActionResult {
        results::ok(self.temp.clone())
    }

    #[route(["get", "post"], "/health")] // multiple methods on the same path
    pub async fn health(&self) -> impl ActionResult {
        results::ok("ok")
    }
}
```

- The root prefix on `#[controller_route]` and the action's own route are
  joined with `/`, and surrounding slashes are trimmed, so
  `#[controller_route("/home/")]` + `#[get("/index/")]` and
  `#[controller_route("home")]` + `#[get("index")]` produce the same route.
- Path parameters use `matchit`'s `{name}` syntax (e.g. `/{id}`,
  `/{category}/{id}`). They are resolved per-request and exposed as
  `http_context.request.routing_info.path_params: HashMap<String, Result<String, String>>`.
  The values are percent-decoded after the route matched (an encoded `%2F` does not
  split a segment). A value that is not valid UTF-8 after decoding is stored as `Err(raw)`.
- An action may declare several route attributes, including different paths for the same HTTP method. Registering the same controller twice or repeating a path and method panics during registration.

## HTTP QUERY

Use `#[query("/search")]` or `#[route(["QUERY"], "/search")]` to register the `http::Method::QUERY` method. QUERY is safe and idempotent and describes the operation in its request body. Clients must send `Content-Type`. Use `Body<T>` to bind a JSON or XML query (or `FromBody<T>` for one field of it), or read the body in the action for a custom query format. [RFC 10008](https://www.rfc-editor.org/rfc/rfc10008.html#section-2)

Generated OpenAPI 3.2 documents describe QUERY with the `query` field on the Path Item Object. [OpenAPI 3.2 Path Item Object](https://spec.openapis.org/oas/v3.2.1.html#path-item-object)

## Action results

The status helpers in `asp_dot_rust::controller::results` return a `RouteResult<T>`. For example, `ok(model)` returns 200 and `created(model)` returns 201. Adding a `Location` with `.with_location(path)` returns `Result<RouteResult<T>, InvalidHeaderValue>`. Handle the header validation result before returning the action result.

Each status function also has a PascalCase alias. For example, `results::BadRequest("invalid id")` and `results::bad_request("invalid id")` call the same function.

Use `result.headers_mut().insert(...)` to set a response header. For headers with multiple values, such as `Set-Cookie`, call `result.headers_mut().append(...)` for each value.

```rust
use asp_dot_rust::controller::results::{self, RouteResult};
use serde::Serialize;

#[derive(Serialize)]
struct Message {
    text: &'static str,
}

fn look_up(found: bool) -> RouteResult<Message> {
    if found {
        results::ok(Message { text: "found" })
    } else {
        results::not_found(Message { text: "missing" })
    }
}
```

The router serializes the model according to the request's `Accept` header. It offers `application/json` and `application/xml`, honors weights and wildcards, and prefers JSON when both match equally or the header is absent. XML uses a `response` root with a `value` field so sequences and scalars can also form XML documents. Negotiated responses include `Vary: Accept`. An unsupported selection returns 406 with an empty body.

Pass a `String` or `&str` directly for UTF-8 text. For example, `ok("default")` sends the seven bytes `default` with a default content type of `text/plain`. A model with `#[derive(Serialize)]` can be returned directly or in a collection. Vectors and slices, including `Vec<u8>`, use JSON/XML negotiation. Serde supports fixed arrays directly up to 32 elements. For longer arrays, use `ok(&array[..])` when the data outlives the result, or `ok(Vec::from(array))` to return owned data.

Use `ok(Raw(bytes))` to send a byte vector, byte array, byte slice or `Bytes` with `application/octet-stream`. Use `created(Json(model))` or `ok(Xml(model))` to fix the format for any serializable value. The wrapper becomes part of the return type, such as `RouteResult<Json<Item>>`. Use `Box<dyn ActionResult>` for branches with different body types.

Migrate a former `Ok(bytes)` call to `ok(Raw(bytes))` to preserve the response body. Use `no_content()` for 204 or `not_modified()` for 304. For a 302 with a known literal location, `redirect("/items").expect("valid redirect location")` produces the action result. Handle the possible header error for a dynamic location.

If the response has yet to start, the router applies the status from `status_code()` and the headers from `write_header` first. In `write_body_async(&self, request_headers: &HeaderMap, response: &mut HttpResponse)`, fill `response.body` to buffer bytes or call `response.write_async(chunk).await?` to stream each chunk.

```rust
use asp_dot_rust::controller::{ActionResult, results};

fn look_up(id: u32) -> Box<dyn ActionResult> {
    if id == 0 {
        return results::bad_request("id must be positive").into();
    }
    results::ok(format!("item {id}")).into()
}
```

## Path parameter conflicts are caught at startup

Two different controllers (or two actions) cannot register conflicting
dynamic segments on the same path, even under different HTTP methods —
for example `/{user_id}` and `/{id}` conflict regardless of the methods
each is registered under, because `matchit` cannot know which name to bind
at match time. Insertion into `matchit` fails in `add_controller_route`, so `add_controllers()` can panic with the conflicting route and insertion error. `RoutingServiceBuilder::build()` moves the populated router into `RoutingService`.

## Query string

`RoutingService::resolve` parses the request URI's query string into
`http_context.request.routing_info.query_params: HashMap<String, Result<String, String>>`:

- Keys and values are decoded as form data: `%XX` is decoded and `+` is a space
  (`%2B` is a plus).
- A flag-style parameter with no `=` (e.g. `?debug`) is kept with an empty
  string value (`"debug" -> ""`), it is not dropped.
- Duplicate keys: the last occurrence wins, whether it is valid or not, so
  `?id=1&id=2` gives `"2"`. The earlier values are discarded without an error.
- Empty pairs (`a=1&&b=2`) and pairs with an empty key (`=v`) are skipped.
- Invalid UTF-8 values retain the encoded text as `Err(raw)` for binder errors (HTTP 422).
  A key with invalid UTF-8 cannot match a
  parameter name and is dropped.

## Resolving a request: `Found` / `NotFound` / `MethodNotAllowed`

`RoutingService::resolve(uri, method) -> ResolvedRoute` distinguishes two
failure cases instead of collapsing them into one:

- `RoutingResult::NotFound` — no route matches the path at all. The
  built-in `auto_route` middleware replies `404 Not Found`.
- `RoutingResult::MethodNotAllowed(Vec<http::Method>)` — the path matches a
  registered route, but not with this HTTP method. `auto_route` replies
  `405 Method Not Allowed` and sets the `Allow` header to the methods that
  *are* registered for that path, per RFC 7231.
- `RoutingResult::Found(Arc<Route>)` — the path and method both
  matched; `auto_route` invokes the matched controller action.

`RoutingService::get_allowed_methods(path) -> HashSet<http::Method>` answers
the same "what methods exist at this path" question independently of a
specific request's method; it's used by the CORS middleware to compute
`Access-Control-Allow-Methods`.

## Route attributes

`#[attr(key = value, ...)]` attaches `key = value` pairs to a route. The value is any expression of a type that is `Send + Sync + 'static` (a number, a bool, a `&'static str`). The getter clones the stored value, so reading it requires the exact type to implement `Clone`. For example, `5` is an `i32`, `5u8` a `u8`, `0.5` an `f64`, `0.5f32` an `f32`.

```rust
#[controller_route("files")]
#[attr(rate_limit = 100, zone = "files")] // below #[controller_route]: the default of every route of the controller
#[controller_inject]
impl FilesController {
    #[get("/list")]               // rate_limit = 100 (i32), zone = "files"
    pub async fn list(&self) -> impl ActionResult { results::ok("list") }

    #[post("/upload")]
    #[attr(rate_limit = 5u8, audit = true)]   // rate_limit replaced (now a u8), audit added, zone kept
    pub async fn upload(&self) -> impl ActionResult { results::ok("upload") }
}
```

- Several `#[attr]` can be put on an impl or an action. A key can be set once on each of them, a second time is a compile error.
- An attribute on an action replaces the attribute with the same key of the impl, the value can be of another type. It applies to every route of the action.
- `#[attr]` on the impl has to be below `#[controller_route]`; above it, it is a compile error.
- Read them in an action or a middleware: `request.route_attribute::<u8>("rate_limit")`, `request.route_attributes()` (`None` when the request resolved to no route) or `route.attributes()` on a resolved `Route`.
- `get::<T>` returns `None` when the key is missing or the value was stored as another type: a number is never converted (`5` is not found as a `u8`).
- `#[max_body_size(..)]` is a built-in attribute macro: `#[name(value)]` is `#[attr(name = value)]` with the value typed as the macro declares, and it needs no import of `attr`. They are all declared in `for_each_attr_alias!` in `asp_dot_rust_macros/src/controller/extra_attribute.rs`: a new one is one more entry there (`name: type`, with its doc comment) and nothing else. If you want it in `asp_dot_rust::controller`, add its name to the `pub use` of `src/controller/mod.rs`.
- The key `max_body_size` is the request body limit of the route, see below.

## Request body limit per route

`max_request_body_size` of `HyperConfig` is the default limit. `#[max_body_size(bytes)]` gives a route its own limit, it is the same as `#[attr(max_body_size = bytes)]`, so it follows the rules of route attributes: on an action it applies to all its routes, below `#[controller_route]` it is the default of all the routes of the controller and an action can override it.

```rust
#[controller_route("files")]
#[max_body_size(1024 * 1024)]                     // default of the controller
#[controller_inject]
impl FilesController {
    #[post("/note")]
    pub async fn note(&mut self) -> impl ActionResult { results::ok("note") } // 1 MiB

    #[post("/upload")]
    #[max_body_size(10 * 1024 * 1024)]            // this action: 10 MiB
    pub async fn upload(&mut self) -> impl ActionResult { results::ok("upload") }
}
```

- `#[max_body_size(5)]` reads `5` as a `usize`, a negative value or another type is a compile error. The long form `#[attr(max_body_size = ..)]` keeps the type you write (`5` is an `i32`), so write `5usize`: any other type makes the application panic at startup (so does an `ActionRoute` built by hand with `with_attribute("max_body_size", ..)` of another type).
- The listener selects the effective limit before constructing `HttpRequest` and passes it to the body stream. On a matched route, `request.route_attribute::<usize>("max_body_size")` exposes an explicit override. The effective limit is not stored as a public request field.
- A declared `Content-Length` over the limit is refused with 413, the body is not read.
- `http_context.request.body` is a `RequestBody` (`Stream(Limited<Incoming>)` or `Buffered`). Reading more than the limit from the stream fails with `LengthLimitError`. Call `enable_buffering` then `seek_body(0)` to read the body more than once (see `docs/model_binding.md`).
- An unknown path or method uses the default limit.

## Known limitations

- No route groups/areas beyond a single controller-level prefix.

Model binding (`FromRoute` / `FromQuery` / `FromBody` / …) is documented in `docs/model_binding.md`.
