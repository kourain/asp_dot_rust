# Middleware

A middleware is created once when the application runs and holds the next middleware (`next: MiddlewareNext`), like `RequestDelegate` in ASP.NET.

## Write a middleware

### Inject struct fields with `#[derive(DependencyInjectableMiddleware)]`

```rust
use asp_dot_rust::{
    http_context::HttpContext,
    macros::{DependencyInjectableMiddleware, middleware},
    middleware::MiddlewareNext,
};

#[derive(DependencyInjectableMiddleware)]
pub struct LogMiddleware {
    next: MiddlewareNext,
}
#[middleware]
impl LogMiddleware {
    pub async fn invoke_async(&self, http_context: &mut HttpContext) {
        // before the handler
        self.next.next_async(http_context).await;
        // after the handler
    }
}
```

### Inject constructor parameters with `fn new`

```rust
use asp_dot_rust::{http_context::HttpContext, macros::middleware, middleware::MiddlewareNext};

pub struct LogMiddleware {
    next: MiddlewareNext,
}
#[middleware]
impl LogMiddleware {
    pub fn new(next: MiddlewareNext) -> Self {
        Self { next }
    }
    pub async fn invoke_async(&self, http_context: &mut HttpContext) {
        // before the handler
        self.next.next_async(http_context).await;
        // after the handler
    }
}
```

## Dependency injection

- `#[derive(DependencyInjectableMiddleware)]` fills one `MiddlewareNext` field and resolves fields typed `Serv<T>`, `CfgOption<T>`, `CfgRequire<T>`, and `CfgReload<T>` when the pipeline is built. Mark fields that use `Default::default()` with `#[di(default)]`.
- For custom construction, define `fn new` inside the `#[middleware]` impl. Its parameters can use the same dependency types and must include `MiddlewareNext`.
- Parameters of `invoke_async` after `http_context` are resolved on every request from the request scope, with the same types.

## What `#[middleware]` generates

`#[middleware]` implements `Middleware` using `invoke_async`. When the block contains `fn new`, it also implements `DependencyInjectableMiddleware` using that constructor. When the block contains only `invoke_async`, provide `DependencyInjectableMiddleware` via the derive or a manual implementation.

`DependencyInjectableMiddleware::inject` takes ownership of `MiddlewareNext` and passes it directly to the constructor or stores it in the derived field.

## Built-in middleware

Register middleware on the `Application` returned by `build()`. The first one registered is the outermost. The router runs at the end of the chain.

| Method on `Application` | Behavior | Configuration at pipeline startup |
| --- | --- | --- |
| `use_compression()` | Compresses eligible buffered responses after the inner middleware returns. | Optional `CompressionConfiguration`. [Compression](./compression.md) |
| `use_web_sockets()` | Lets downstream middleware and controllers accept HTTP/1.1 and HTTP/2 WebSockets. | Customize defaults with `use_web_sockets_with_options(\|options\| { ... })`. [WebSocket controller example](./web_sockets.md) |
| `use_cors()` | Applies the origin policy and answers allowed preflight requests. | Register `CorsConfiguration`. An empty entry uses its default. |
| `use_rate_limit()` | Counts requests per client socket IP and returns 429 when the limit is exceeded. | Requires a populated `RateLimitConfiguration`. |
| `use_request_timeout()` | Limits the time spent in the inner pipeline. On timeout it returns 408 or aborts a response that has already started. | Requires a populated `RequestTimeoutConfiguration`. |
| `use_static_files()` | Copies matching files into the response buffer. Middleware must also set a body-permitting status such as 200 for those bytes to be sent if the response still has the initial 204 status. | Register `StaticFileConfiguration`. The configured directory must exist. |
| `use_authorize_basic()` | Parses Basic credentials into an `AuthUser` under `USER_CONTEXT_KEY`. The application is responsible for verifying the username and password. | No configuration required. |
| `use_authorize_bearer()` | Validates a bearer JWT using HS256 and the configured issuer and audience, then stores the user in the request context. | Requires a populated `JwtAuthConfiguration`. |
| `use_swagger()` | Serves the UI and generated API document. | Uses the routing service. [Swagger endpoints](./swagger.md#endpoints) |

## Limits

- A middleware is a singleton, take request scoped services in the parameters of `invoke_async`, not in `new`.
- Middlewares are created when the application runs, after all services are registered.
- `add_middleware_instance` is replaced by `add_middleware_factory(|next| ..)`.
- The macro `middleware!(pub Name, |ctx, next| { next.next_async(ctx).await; })` defines a middleware with only the field `next`.
- Per request trace logging of each middleware was removed from the pipeline.
