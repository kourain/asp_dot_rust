# Routing

ASP.Rust matches incoming requests to controller actions using
[`matchit`](https://crates.io/crates/matchit), the same radix-tree router
used by `axum` and `hyper`'s own examples. Routes are collected at startup
into a `RoutingServiceBuilder`, then compiled once into an immutable
`RoutingService` that resolves every request.

## Core concepts

| Concept | Type | Purpose |
| ------- | ---- | ------- |
| Route collector | `RoutingServiceBuilder` | Accumulates every `#[get]`/`#[post]`/... action declared on every controller. Lives behind a process-wide registry until `add_controllers()` is called. |
| Compiled router | `RoutingService` | Built once from `RoutingServiceBuilder::build()`, registered as a singleton service, and used to resolve every incoming request. Read-only after startup. |
| Resolution result | `ResolvedRoute` | Carries the matched `RoutingResult`, extracted `path_params`, and parsed `query_params` for one request. Reachable at `http_context.request.routing_info`. |
| Match outcome | `RoutingResult` | `Found(info)`, `NotFound`, or `MethodNotAllowed(allowed_methods)`. |

## Declaring routes on a controller

```rust
use asp_dot_rust::{
    api_controller,
    controller::{ActionResult, get, post, route},
    http_context::HttpContextRef,
    macros::{controller_inject, controller_route},
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
        "Hello, World!"
    }

    #[get("/{id}")] // -> GET /home/{id}, a dynamic path parameter
    pub async fn show(&mut self) -> impl ActionResult {
        let id = self.http_context.request.routing_info.path_params.get("id").cloned().unwrap_or_default();
        format!("home #{id}")
    }

    #[post("/update")] // -> POST /home/update
    pub fn update(&mut self) -> impl ActionResult {
        self.temp.clone()
    }

    #[route(["get", "post"], "/health")] // multiple methods on the same path
    pub async fn health(&self) -> impl ActionResult {
        "ok".to_string()
    }
}
```

- The root prefix on `#[controller_route]` and the action's own route are
  joined with `/`, and surrounding slashes are trimmed, so
  `#[controller_route("/home/")]` + `#[get("/index/")]` and
  `#[controller_route("home")]` + `#[get("index")]` produce the same route.
- Path parameters use `matchit`'s `{name}` syntax (e.g. `/{id}`,
  `/{category}/{id}`). They are resolved per-request and exposed as
  `http_context.request.routing_info.path_params: HashMap<String, String>`.
- Every action is registered under exactly one literal route string per
  HTTP method; registering the same controller twice, or two different
  controllers on the same route *and* method, panics at startup
  (`register_controller` / `add_controller_route`).

## Path parameter conflicts are caught at startup

Two different controllers (or two actions) cannot register conflicting
dynamic segments on the same path, even under different HTTP methods —
for example `/{user_id}` and `/{id}` conflict regardless of the methods
each is registered under, because `matchit` cannot know which name to bind
at match time. `RoutingServiceBuilder::build()` panics immediately with the
conflicting route and the underlying `matchit` error, so this is caught
when the application starts, not silently at request time.

## Query string

`RoutingService::resolve` parses the request URI's query string into
`http_context.request.routing_info.query_params: HashMap<String, String>`:

- Both keys and values are percent-decoded (`urlencoding::decode`).
- A flag-style parameter with no `=` (e.g. `?debug`) is kept with an empty
  string value (`"debug" -> ""`), it is not dropped.
- Duplicate keys: the last occurrence wins, since parsing folds into a
  `HashMap`.

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

`#[attr(key = value, ...)]` attaches `key = value` pairs to a route. The value is any expression of a type that is `Send + Sync + 'static` (a number, a bool, a `&'static str`). It is stored as its own type and read back as exactly that type: `5` is an `i32`, `5u8` a `u8`, `0.5` an `f64`, `0.5f32` an `f32`.

```rust
#[controller_route("files")]
#[attr(rate_limit = 100, zone = "files")] // below #[controller_route]: the default of every route of the controller
#[controller_inject]
impl FilesController {
    #[get("/list")]               // rate_limit = 100 (i32), zone = "files"
    pub async fn list(&self) -> impl ActionResult { "list" }

    #[post("/upload")]
    #[attr(rate_limit = 5u8, audit = true)]   // rate_limit replaced (now a u8), audit added, zone kept
    pub async fn upload(&self) -> impl ActionResult { "upload" }
}
```

- Several `#[attr]` can be put on an impl or an action. A key can be set once on each of them, a second time is a compile error.
- An attribute on an action replaces the attribute with the same key of the impl, the value can be of another type. It applies to every route of the action.
- `#[attr]` on the impl has to be below `#[controller_route]`; above it, it is a compile error.
- Read them in an action or a middleware: `request.route_attribute::<u8>("rate_limit")`, `request.route_attributes()` (`None` when the request resolved to no route) or `route.attributes()` on a resolved `Route`.
- `get::<T>` returns `None` when the key is missing or the value was stored as another type: a number is never converted (`5` is not found as a `u8`).
- `#[max_body_size(..)]` is a built-in attribute macro: `#[name(value)]` is `#[attr(name = value)]` with the value typed as the macro declares, and it needs no import of `attr`. They are all declared in `attr_aliases!` in `asp_dot_rust_macros/src/lib.rs`, a new one is one more entry there (`name: type`, with its doc comment) and, if you want it in `asp_dot_rust::controller`, a line in the `pub use` of `src/controller/mod.rs`.
- The key `max_body_size` is the request body limit of the route, see below.

## Request body limit per route

`max_request_body_size` of `HyperConfig` is the default limit. `#[max_body_size(bytes)]` gives a route its own limit, it is the same as `#[attr(max_body_size = bytes)]`, so it follows the rules of route attributes: on an action it applies to all its routes, below `#[controller_route]` it is the default of all the routes of the controller and an action can override it.

```rust
#[controller_route("files")]
#[max_body_size(1024 * 1024)]                     // default of the controller
#[controller_inject]
impl FilesController {
    #[post("/note")]
    pub async fn note(&mut self) -> impl ActionResult { /* ... */ }      // 1 MiB

    #[post("/upload")]
    #[max_body_size(10 * 1024 * 1024)]            // this action: 10 MiB
    pub async fn upload(&mut self) -> impl ActionResult { /* ... */ }
}
```

- `#[max_body_size(5)]` reads `5` as a `usize`, a negative value or another type is a compile error. The long form `#[attr(max_body_size = ..)]` keeps the type you write (`5` is an `i32`), so write `5usize`: any other type makes the application panic at startup (so does an `ActionRoute` built by hand with `with_attribute("max_body_size", ..)` of another type).
- The limit is chosen when the request is created. `http_context.request.max_body_size` tells the one in use.
- A declared `Content-Length` over the limit is refused with 413, the body is not read.
- `http_context.request.body` is a `Limited<Incoming>`: reading more than the limit (chunked body, or no declared length) fails with `http_body_util::LengthLimitError`. The action decides what to answer.
- An unknown path or method uses the default limit.

## Known limitations

- There is no automatic request-body or form binding yet (no
  `FromBody`/`FromForm`/`FromQuery` helpers) — read
  `http_context.request.body` directly in the action for now.
- No route groups/areas beyond a single controller-level prefix.
