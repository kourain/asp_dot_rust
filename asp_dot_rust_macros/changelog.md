# Changelog

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
