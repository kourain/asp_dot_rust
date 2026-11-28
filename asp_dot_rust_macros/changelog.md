# Changelog

## 0.2.10

Companion macro release for `asp_dot_rust` 0.2.12.

### Controller routes and field binding

- `#[controller_route]` generates dispatch through `Controller` and returns the action result to routing middleware. Actions can return borrowed results. Each route is registered separately so startup errors point to the offending attribute. Qualified impls such as `impl inner::PathController` work.
- `#[query("/search")]` registers HTTP QUERY. `#[route(...)]` accepts exactly two arguments, the methods and the path. `#[cfg]` is carried into the generated registration and dispatch arm. Rust Analyzer keeps the inactive range within the disabled action.
- Route, metadata and field attributes accept qualified paths from this crate, including renamed dependencies. A bare route name such as `#[get]` is treated as a framework attribute. Use `#[other_crate::get(...)]` for a different macro of the same name. Generated code retains source locations for hover and reports a rejected binder type or validation rule at the action parameter.
- `FromBody<T>` and `FromForm<T>` bind a single field. `Body<T>` and `Form<T>` bind whole payloads. Multiple binders may read the same body format. Mixing form and body formats on one action produces a compile error. Query, form and body fields accept `Option<T>` and their `FromOption*` aliases. `FromRoute<Option<T>>` is rejected.
- `#[bind]` accepts `name`, `default` and `validate_default`. The default expression is evaluated when the request field is absent. `validate_default` defaults to `true`. A value of `false` skips the rule only for the fallback. A raw identifier such as `r#type` binds `type`, and an explicit `name` is also used in field diagnostics.
- `#[validate]` dispatches built-in and extension rules on `ControllerActionFieldValidate` after binding. Optional fields are checked when present. Whole DTOs use their `Validate` implementation. Duplicate field attributes, unsupported options and malformed rule bounds are reported during compilation.

### Dependency injection

- `#[derive(DependencyInjectableController)]` injects `HttpContextRef`, services and configuration into named fields. `#[di(default)]` initializes other fields with `Default`. `#[di(ignore_missing_http_context)]` suppresses the warning for controllers that intentionally omit the context field. `api_controller!` forwards both struct and field attributes, including the derive and per-field `#[di(default)]`.
- `#[derive(DependencyInjectableMiddleware)]` injects `MiddlewareNext`, services and configuration into named fields. `#[middleware]` can use this derive, a manual `DependencyInjectableMiddleware` implementation, or a `fn new` constructor. Additional parameters on `invoke_async` are injected from the current request scope.
- Service injection emits typed `ServiceDependencies` metadata for services and configuration, including whether a configuration value is required. Generated controller and request-time middleware code checks direct registrations before resolving arguments. Manual controller injectors now receive `HttpContextRef`, and manual middleware injectors implement `DependencyInjectableMiddleware` separately from `Middleware`.

See the [framework changelog](../changelog.md) for runtime behavior and migration examples.

## 0.2.9

- `#[inject]` now supports an `async fn new` constructor; the generated injection code awaits it.
- Generated controller and dependency-registration code uses the public `third_party` re-exports and the current `HttpHeaderExt` / `ShareMutPtr` APIs.
- move to new conversion and public third_party of core project

## 0.2.8

- `#[controller_route]` add model binding for action parameters: `FromRoute<T>`, `FromQuery<T>`, `FromOptionQuery<T>`, `FromBody<T>`, and `FromForm<T>`.
- Refactor route attribute parsing with reusable `some_or_error!` and `ok_or_error!` macros to reduce repeated parsing and error-handling branches.
- `#[derive(DependencyInjectableService)]`, `#[inject]` and `#[middleware]` generate `async fn inject` (with `async_trait` from `asp_dot_rust::third_party`) and resolve `Serv<T>` with `get_service::<T>().await`.
- The generated code injects `CfgReload<T>` with `get::<T>()`, `CfgOption<T>` with `get::<T>().capture()` and `CfgRequire<T>` with `get::<T>().capture().unwrap()`, it does not call a tuple constructor of the wrapper any more.
- `CfgOption<T>` is recognised as a parameter or field type besides the deprecated `Cfg<T>`.
- Needs `asp_dot_rust` 0.2.10.

## 0.2.7

- Built-in attribute macros such as `#[max_body_size(..)]` are `#[attr(name = value)]` with a typed value; they are declared in one place (`for_each_attr_alias!` in `controller/extra_attribute.rs` of the macros crate) and need no import of `attr`.
- `#[attr(key = value, ...)]` sets attributes of a route, on the controller impl as defaults for its routes or on an action to override a key. A value is stored as its own type and read back as exactly that type (`5` is an `i32`, `5u8` a `u8`) with `HttpRequest::route_attribute::<T>(key)`, `HttpRequest::route_attributes()` or `Route::attributes()`.

## 0.2.6

- Add #[middleware] at top impl middleware

## 0.2.5

- Allow controller_route auto asign empty default root route ("")
