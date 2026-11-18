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

## Limits

- A middleware is a singleton, take request scoped services in the parameters of `invoke_async`, not in `new`.
- Middlewares are created when the application runs, after all services are registered.
- `add_middleware_instance` is replaced by `add_middleware_factory(|next| ..)`.
- The macro `middleware!(pub Name, |ctx, next| { next.next_async(ctx).await; })` defines a middleware with only the field `next`.
- Per request trace logging of each middleware was removed from the pipeline.
