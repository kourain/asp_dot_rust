# Middleware

A middleware is created once when the application runs and holds the next middleware (`next: MiddlewareNext`), like `RequestDelegate` in ASP.NET.

## Write a middleware

```rust
use asp_dot_rust::{macros::middleware, middleware::MiddlewareNext, http_context::HttpContext};

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
        self.next.call_async(http_context).await;
        // after the handler
    }
}
```

## Dependency injection

- Parameters of `new` after `next` are resolved once when the pipeline is built: `Serv<T>`, `Cfg<T>`, `CfgRequire<T>`, `CfgReload<T>`.
- Parameters of `invoke_async` after `http_context` are resolved on every request from the request scope, with the same types.
- Without `fn new` the struct must have only the field `next`.

## What `#[middleware]` generates

`Middleware` (from `new`), `Middleware` (from `invoke_async`) and `Middleware` for the type.

## Limits

- A middleware is a singleton, take request scoped services in the parameters of `invoke_async`, not in `new`.
- Middlewares are created when the application runs, after all services are registered.
- `add_middleware_instance` is replaced by `add_middleware_factory(|next| ..)`.
- The macro `middleware!(pub Name, |ctx, next| { next.call_async(ctx).await; })` defines a middleware with only the field `next`.
- Per request trace logging of each middleware was removed from the pipeline.
