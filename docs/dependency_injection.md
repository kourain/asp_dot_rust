# Dependency Injection

ASP.Rust ships a small, `TypeId`-based dependency injection (DI) container
inspired by ASP.NET Core's `IServiceCollection`. Services are registered on
an `ApplicationBuilder` and resolved through a `ServiceProviderScope`, either
directly or as constructor parameters of other injectable services and
controllers.

## Core concepts

| Concept | Type | Purpose |
| ------- | ---- | ------- |
| Container | `ServiceProviderScope` | Holds every registered service and resolves them by type. Exposed as the public `service` field on `ApplicationBuilder`. |
| Injectable service | `#[inject_require]` on an `impl` block | Marks a type as constructible by the container and records its dependency edges for cycle detection. |
| Injectable controller | `#[controller_inject_require]` on an `impl` block | Same as above, but for controllers, which additionally may receive an `HttpContextRef`. |
| Lifetime | `ServiceType::{Singleton, Scope, Transient}` | Controls how long a resolved instance is reused. |

## Registering a service

```rust
use std::sync::Arc;
use asp_dot_rust::macros::inject_require;

pub struct ExService {
    prefix: String,
}

#[inject_require]
impl ExService {
    pub fn new() -> Self {
        Self { prefix: "Hello".into() }
    }

    pub fn greet(&self, name: &str) -> String {
        format!("{}, {}!", self.prefix, name)
    }
}
```

```rust
let mut builder = ApplicationBuilder::new("MyApp");
builder.service.add_singleton::<ExService>();
builder.service.add_scope::<ExService>();
builder.service.add_transient::<ExService>();
```

`add_singleton`, `add_scope`, and `add_transient` only register the type;
construction is lazy and happens the first time the service is resolved via
`get_service::<T>()`.

## Service lifetimes

- **Singleton** — one instance for the entire application. Built once, on
  first use, and reused by every subsequent `get_service::<T>()` call and by
  every scope, including scopes created with `create_scope()`.
- **Scope** — one instance per `ServiceProviderScope`. Reused for every
  `get_service::<T>()` call within the same scope, but `create_scope()`
  produces a fresh, uninitialized instance for the new scope. This is the
  lifetime typically used for per-request state (a new scope is created for
  each incoming HTTP request).
- **Transient** — a brand-new instance is constructed on every single
  `get_service::<T>()` call, even within the same scope. A transient service
  cannot be registered through `add_instance` (attempting to do so panics),
  since `add_instance` is meant for pre-built, shared instances.

```rust
builder.service.add_singleton::<ExService>(); // one for the whole app
builder.service.add_scope::<RequestIdService>();      // one per request
builder.service.add_transient::<RandomIdGenerator>(); // new value every call
```

## Injecting dependencies into another service

Any parameter of `fn new` typed as `Arc<T>` is resolved as a service
dependency; the container calls `get_service::<T>()` for you:

```rust
use std::sync::Arc;
use asp_dot_rust::macros::inject_require;

pub struct Ex2Service {
    ex_service: Arc<ExService>,
}

#[inject_require]
impl Ex2Service {
    pub fn new(ex_service: Arc<ExService>) -> Self {
        Ex2Service { ex_service }
    }
}
```

Both `ExService` and `Ex2Service` must be registered
(`add_singleton`/`add_scope`/`add_transient`) before `Ex2Service` is
resolved, or `get_service` panics with `Service <name> not found in scope`.

## Injecting configuration

A parameter typed as `Option<Arc<T>>` is treated as configuration rather than
a service, and is resolved from the application's `ConfigurationService`:

```rust
#[inject_require]
impl Ex2Service {
    pub fn new(config: Option<Arc<AuditConfiguration>>) -> Self {
        Ex2Service { enabled: config.map(|c| c.enabled).unwrap_or(false) }
    }
}
```

## Injecting into a controller

Controllers use `#[controller_inject_require]` instead of `#[inject_require]`
and must take an `HttpContextRef` as one of their parameters, in addition to
any `Arc<T>` service or `Option<Arc<T>>` configuration parameters:

```rust
use asp_dot_rust::prelude::*;

#[controller_route("home")]
impl HomeController {
    fn new(ctx: HttpContextRef, ex_service: Arc<ExService>) -> Self {
        HomeController { ctx, ex_service }
    }

    #[get("/")]
    pub async fn index(&mut self) -> impl ActionResult {
        self.ex_service.greet("world")
    }
}
```

## Circular dependency detection

Every `#[inject_require]`/`#[controller_inject_require]` impl registers its
dependency edges globally via `inventory::submit!`. `ApplicationBuilder::build()`
walks this graph — restricted to the types actually registered in that
builder's scope — and panics with the offending path if it finds a cycle,
for example:

```rs
thread 'main' panicked at 'Found circular dependency: CycleServiceX -> CycleServiceY -> CycleServiceX'
```

This check runs on every build profile (debug and release), not only in
debug builds, so a circular dependency is caught immediately on startup no
matter how the application is compiled.

If a registered service depends on a type that was never registered, the
container does not panic — it logs a warning instead, since this is
sometimes only reached through a code path that never runs. Prefer fixing
the registration, since resolving the missing service will still panic at
runtime.

## Known limitations (What not work)

- `fn new` cannot be `async`. Services that require asynchronous
  initialization (opening a database connection pool, calling an external
  API, etc.) must perform that work lazily inside a method other than `new`.
- `get_service::<T>()` panics rather than returning `Option`/`Result` when
  `T` was never registered; there is currently no `try_get_service`.
- Only a single implementation per concrete type is supported. Registering
  multiple implementations of the same trait behind `Arc<dyn Trait>` is not
  built in and requires a manual wrapper service.

## API summary

| Method | On | Description |
| ------ | -- | ----------- |
| `add_singleton::<T>()` | `ServiceProviderScope` | Register `T` with `Singleton` lifetime. |
| `add_scope::<T>()` | `ServiceProviderScope` | Register `T` with `Scope` lifetime. |
| `add_transient::<T>()` | `ServiceProviderScope` | Register `T` with `Transient` lifetime. |
| `add_instance::<T>(Arc<T>, ServiceType)` | `ServiceProviderScope` | Register a pre-built instance. Panics if `ServiceType::Transient` is used. |
| `get_service::<T>()` | `ServiceProviderScope` | Resolve `T`. Panics if `T` was never registered. |
| `contains_service::<T>()` | `ServiceProviderScope` | Check whether `T` is registered. |
| `create_scope()` | `ServiceProviderScope` | Create a new scope that shares Singletons but gets fresh Scope/Transient instances. |
| `check_dependency_cycles()` | `ServiceProviderScope` | Panic if the registered services form a circular dependency. Called automatically by `ApplicationBuilder::build()`. |
