# Dependency Injection

ASP.RS ships a small, `TypeId`-based dependency injection (DI) container
inspired by ASP.NET Core's `IServiceCollection`. Services are registered on
an `ApplicationBuilder` and resolved through a `ServiceProviderScope`, either
directly or as constructor parameters of other injectable services and
controllers.

## Core concepts

| Concept | Type | Purpose |
| ------- | ---- | ------- |
| Container | `ServiceProviderScope` | Holds every registered service and resolves them by type. Exposed as the public `service` field on `ApplicationBuilder`. |
| Injectable service | `#[inject]` on an `impl` block | Marks a type as constructible by the container and records its dependency edges for cycle detection. |
| Injectable controller | `#[controller_inject]` on an `impl` block or `#[derive(DependencyInjectableController)]` on a struct | Injects request dependencies and can provide an `HttpContextRef`. |
| Lifetime | `ServiceType::{Singleton, Scope, Transient}` | Controls how long a resolved instance is reused. |

## Registering a service

```rust
use std::sync::Arc;
use asp_dot_rust::macros::inject;

pub struct ExService {
    prefix: String,
}

#[inject]
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

Any parameter of `fn new` typed as `Serv<T>` is resolved as a service
dependency; the container calls `get_service::<T>()` for you and hands it
back wrapped as `Serv<T>` (derefs to `Arc<T>`):

```rust
use asp_dot_rust::dependency_injection::Serv;
use asp_dot_rust::macros::inject;

pub struct Ex2Service {
    ex_service: Serv<ExService>,
}

#[inject]
impl Ex2Service {
    pub fn new(ex_service: Serv<ExService>) -> Self {
        Ex2Service { ex_service }
    }
}
```

Both `ExService` and `Ex2Service` must be registered
(`add_singleton`/`add_scope`/`add_transient`) before `Ex2Service` is
resolved, or `get_service` panics with `Service <name> not found in scope`.

### Shortcut: `#[derive(DependencyInjectableService)]`

When every dependency a service needs is already a plain struct field typed
`Serv<T>`, `CfgOption<T>`, `CfgRequire<T>`, or `CfgReload<T>`, `fn new` can be
skipped entirely:

```rust
use asp_dot_rust::dependency_injection::Serv;
use asp_dot_rust_macros::DependencyInjectableService;

#[derive(DependencyInjectableService)]
pub struct Ex2Service {
    ex_service: Serv<ExService>,
}
```

This expands to the same `impl DependencyInjectableService` (and dependency
edge registration) that `#[inject]` would generate for a `fn new`
that just does `Self { ex_service }`. It only supports named-field structs
(or unit structs). A field needing custom construction (a computed value, a
non-DI default, a dependency the constructor needs but doesn't store) is not
one of the four wrapper types, so it is built with `Default::default()`
instead — a compile error if the field's type doesn't implement `Default`.

This fallback is never silent: every unmarked field defaulted this way emits
a compiler **warning** (via the `deprecated` lint) pointing at the field:

```text
warning: use of deprecated function `Ex2Service::__di_default_field_marker_Ex2Service_1`: field `note` on `Ex2Service` is not Serv<T>, CfgOption<T>, CfgRequire<T>, or CfgReload<T>; #[derive(DependencyInjectableService)] defaulted it via `Default::default()`. Mark it `#[di(default)]` if that is intended, or use #[inject] on a hand-written `fn new` if this field needs real construction.
```

Treat that warning as a prompt to double-check the field is intentionally
non-DI (and not, say, a typo'd `Serve<T>`).

### Opting in with `#[di(default)]`

When a field is *meant* to be built with `Default::default()`, mark it
`#[di(default)]`. The field is still defaulted, but the warning is silenced
for that field only — unlike `#[allow(deprecated)]`, which would have to be
applied at module or crate level and would also hide genuine deprecation
warnings:

```rust
#[derive(DependencyInjectableService)]
pub struct Ex3Service {
    ex_service: Serv<ExService>,
    #[di(default)]
    call_count: u32,     // intentionally not injected, no warning
}
```

Misuse of the attribute is a compile error rather than a silent no-op:

| Written | Result |
| --- | --- |
| `#[di(default)]` on a non-wrapper field | defaulted, no warning |
| `#[di(default)]` on `Serv<T>` / `CfgOption<T>` / `CfgRequire<T>` / `CfgReload<T>` | **error** — it would replace a real dependency with `Default::default()` |
| `#[di]` or `#[di()]` | **error** — the option is missing |
| `#[di(defualt)]` and other unknown options | **error** — unknown `di` option |

A `#[di(default)]` field registers no dependency edge, so it is ignored by
the startup cycle check.

If the field needs real construction logic rather than just a default, use
`#[inject]` on a hand-written `fn new` instead.

## Injecting configuration

All three wrappers require `T` to be registered in the application's
`ConfigurationService`, for example with `insert::<T>()` or
`configure::<T>()`. The wrapper determines how a registered entry is read.

| Wrapper | Resolves via | Registered entry without a value |
| ------- | ------------- | -------------------- |
| `CfgOption<T>` (`Cfg<T>`, deprecated) | `ConfigurationService::get::<T>().capture()` | no value (`has_value()` is `false`) |
| `CfgRequire<T>` | `ConfigurationService::get::<T>().capture().unwrap()` | panics |
| `CfgReload<T>` | `ConfigurationService::get::<T>()` | a handle without a value |

```rust
use asp_dot_rust::dependency_injection::{CfgOption, CfgRequire};

#[inject]
impl Ex2Service {
    pub fn new(config: CfgOption<AuditConfiguration>) -> Self {
        Ex2Service { enabled: config.has_value() && config.unwrap().enabled }
    }

    // or, if the service cannot function without this configuration:
    pub fn new_strict(config: CfgRequire<AuditConfiguration>) -> Self {
        Ex2Service { enabled: config.enabled }
    }
}
```

`CfgReload<T>` is a handle that follows the configuration while the
application is running, `capture()` gives the value now. `CfgOption<T>` and
`CfgRequire<T>` are values captured when the service is built (see
[`appsettings.md`](./appsettings.md) for `configure::<T>()` and
`reload_all_async()`).

## Injecting into a controller

Controllers use `#[controller_inject]` instead of `#[inject]`
and must take an `HttpContextRef` as one of their parameters, in addition to
any `Serv<T>` service or `CfgOption<T>`/`CfgRequire<T>`/`CfgReload<T>` configuration
parameters:

```rust
use asp_dot_rust::prelude::*;

#[controller_route("home")]
#[controller_inject]
impl HomeController {
    fn new(http_context: HttpContextRef, ex_service: Serv<ExService>) -> Self {
        HomeController { http_context, ex_service }
    }

    #[get("/")]
    pub async fn index(&mut self) -> impl ActionResult {
        results::Ok(self.ex_service.greet("world"))
    }
}
```

`#[controller("home")]` combines `#[controller_route("home")]` and
`#[controller_inject]`. Use `#[controller]` to omit the route prefix.
Without `fn new`, `#[controller_inject]` leaves construction to a derive
or a manual `DependencyInjectableController` implementation.

Use the derive to inject fields directly from the request scope:

```rust
use asp_dot_rust::{
    api_controller,
    dependency_injection::Serv,
    macros::DependencyInjectableController,
};

api_controller!(
    #[derive(DependencyInjectableController)]
    HomeController {
        ex_service: Serv<ExService>,
    }
);
```

`api_controller!` supplies the `http_context: HttpContextRef` field used by
`#[controller_route]`. When deriving on a struct that intentionally omits an
`HttpContextRef` field, add `#[di(ignore_missing_http_context)]` below the
derive to suppress the warning. Use `#[di(default)]` on fields initialized
with `Default::default()`.

## Startup dependency validation

Service injection macros submit non-empty constructor dependencies as
`ServiceDependencies` metadata to `inventory`.
The dependency graph includes only services registered in the current provider.
`Application::run()` loads configuration, builds and stores the routing singleton,
then checks service cycles and missing registrations before initializing middleware
or hosted services and opening a port. Missing registrations report the dependency path.

```rs
thread 'main' panicked at 'Found circular dependency: CycleServiceX -> CycleServiceY -> CycleServiceX'
```

Middleware and hosted service constructors resolve their dependencies during
`run()`. Controller injection and middleware `inject_invoke_async` check all direct
dependency registrations before resolving their arguments for the current request.
They use checks generated by the macros. Controller, middleware, and hosted-service
registrations add no nodes to the service dependency graph.
All configuration wrappers require a registration. `CfgRequire<T>` also requires
a value when resolved, while `CfgOption<T>` and `CfgReload<T>` allow an empty entry.

## Known limitations (What not work)

- `fn new` cannot be `async`. Services that require asynchronous
  initialization (opening a database connection pool, calling an external
  API, etc.) must perform that work lazily inside a method other than `new`.
- `get_service::<T>()` panics rather than returning `Option`/`Result` when
  `T` was never registered; there is currently no `try_get_service`.
- Only a single implementation per concrete type is supported. Registering
  multiple implementations of the same trait behind `Arc<dyn Trait>` is not
  built in and requires a manual wrapper service.
- `fn new` parameters must be one of `Serv<T>`, `CfgOption<T>`, `CfgRequire<T>`,
  `CfgReload<T>` (or `HttpContextRef` for controllers). Bare `Arc<T>` /
  `Option<Arc<T>>` parameters are **no longer recognized** as of 0.2.1 —
  use `Serv<T>` / `CfgOption<T>` instead.

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
| `check_dependency_cycles()` | `ServiceProviderScope` | Check registered service cycles and service registrations. Startup validation runs in `Application::run()`. |
