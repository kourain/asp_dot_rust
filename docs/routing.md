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
- `RoutingResult::Found(Arc<RoutingInfo>)` — the path and method both
  matched; `auto_route` invokes the matched controller action.

`RoutingService::get_allowed_methods(path) -> HashSet<http::Method>` answers
the same "what methods exist at this path" question independently of a
specific request's method; it's used by the CORS middleware to compute
`Access-Control-Allow-Methods`.

## Known limitations

- There is no automatic request-body or form binding yet (no
  `FromBody`/`FromForm`/`FromQuery` helpers) — read
  `http_context.request.body` directly in the action for now.
- No route groups/areas beyond a single controller-level prefix.
