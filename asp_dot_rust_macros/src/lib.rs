mod controller;
mod dependency_injection;
mod utils;

use proc_macro::TokenStream;

use crate::controller::extra_attribute::{define_attr_alias_macros, for_each_attr_alias};

/// Registers routes and generates constructor injection when the impl contains `fn new`.
///
/// Use `#[controller]` for the root or `#[controller("prefix")]` for a route prefix.
/// Without `fn new`, the controller can derive `DependencyInjectableController`.
#[proc_macro_attribute]
pub fn controller(args: TokenStream, item: TokenStream) -> TokenStream {
    let main_crate = crate::utils::find_crate::asp_dot_rust_crate_path();
    let args = proc_macro2::TokenStream::from(args);
    let item = proc_macro2::TokenStream::from(item);
    quote::quote! {
        #[#main_crate::macros::controller_route(#args)]
        #[#main_crate::macros::controller_inject]
        #item
    }
    .into()
}

/// using the controller_route attribute to define a controller and its routes
///
///```no_run
///use asp_dot_rust::controller::results::ok;
///#[controller_route("")]
///impl HomeController {
///    #[get("/")]
///    pub async fn index(&mut self) -> impl ActionResult {
///       ok("Hello, World!")
///   }
/// }
/// ```
#[proc_macro_attribute]
pub fn controller_route(args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::controller_route(args, item)
}

/// Configures a route, query, form or body field on a routed controller action.
/// `name` selects the request key. `default` supplies a value when the field is absent.
/// Field validation runs on the default unless `validate_default = false` is set.
#[proc_macro_attribute]
pub fn bind(_args: TokenStream, item: TokenStream) -> TokenStream {
    crate::utils::compiler_error::create_compiler_error_from_token_stream(&item, "#[bind(...)] belongs on a field binder of a routed controller action")
}

/// Validates a route, query, form or body field before the controller action runs.
///
/// `#[validate(length(2))]` requires exactly two Unicode scalar values.
/// `#[validate(length(1, 80))]` accepts an inclusive interval
/// `#[validate(range(min = 1, max = 100))]` checks a numeric value.
/// User-defined rules dispatch to the corresponding method on `ControllerActionFieldValidate`.
/// Missing optional parameters skip the rule. Invalid values produce HTTP 422.
/// 
///```no_run
///use asp_dot_rust::controller::results::ok;
///#[controller_route("")]
///impl HomeController {
///    #[get("/")]
///    pub async fn index(&mut self, #[validate(range(max=32))] a : FromQuery<u32>) -> impl ActionResult {
///       ok("Hello, World!")
///   }
/// }
/// ```
#[proc_macro_attribute]
pub fn validate(_args: TokenStream, item: TokenStream) -> TokenStream {
    crate::utils::compiler_error::create_compiler_error_from_token_stream(&item, "#[validate(...)] belongs on a field binder of a routed controller action")
}

/// registers a function as a route handler for the specified HTTP method(s) <br>
/// Example usage:
///
/// ```no_run
/// #[route(["GET", "POST"], "/health")]
/// ```
/// this ignore case you can use any case for the HTTP methods, such as "get", "POST", "Put", etc.
/// ```no_run
/// #[route(["get", "post"], "/health")]
/// ```
#[proc_macro_attribute]
pub fn route(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// sets attributes of a route as `key = value` pairs. The value is any expression of a type that is `Send + Sync + 'static`
/// (a number, a bool, a `&'static str`), it is stored as that type and has to be read back as exactly that type: `5` is an `i32`, `5u8` a `u8`. <br>
/// On an action it sets them for all the routes of the action. On the controller impl, below `#[controller_route]`,
/// it sets them for all the routes of the controller, an attribute with the same key on an action overrides it.
///
/// ```no_run
/// use asp_dot_rust::controller::results::ok;
/// #[controller_route("files")]
/// #[attr(rate_limit = 100, zone = "files")]
/// impl FilesController {
///     #[get("/list")]
///     pub async fn list(&self) -> impl ActionResult { ok("list") }
///
///     #[post("/upload")]
///     #[attr(rate_limit = 5u8, audit = true)]
///     pub async fn upload(&self) -> impl ActionResult { ok("upload") }
/// }
/// ```
/// Read them with `http_context.request.route_attribute::<u8>("rate_limit")`.
#[proc_macro_attribute]
pub fn attr(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::attribute::attr(&_args, item, "attr")
}

for_each_attr_alias!(define_attr_alias_macros);

/// registers a function as a GET route handler <br>
/// Example usage:
///
/// ```no_run
/// #[get("/health")]
/// ```
#[proc_macro_attribute]
pub fn get(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// registers a function as a POST route handler <br>
/// Example usage:
///
/// ```no_run
/// #[post("/health")]
/// ```
#[proc_macro_attribute]
pub fn post(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// registers a function as a PUT route handler <br>
/// Example usage:
///
/// ```no_run
/// #[put("/health")]
/// ```
#[proc_macro_attribute]
pub fn put(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// registers a function as a DELETE route handler <br>
/// Example usage:
///
/// ```no_run
/// #[delete("/health")]
/// ```
#[proc_macro_attribute]
pub fn delete(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// registers a function as a PATCH route handler <br>
/// Example usage:
///
/// ```no_run
/// #[patch("/health")]
/// ```
#[proc_macro_attribute]
pub fn patch(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// registers a function as an OPTIONS route handler <br>
/// Example usage:
///
/// ```no_run
/// #[options("/health")]
/// ```
#[proc_macro_attribute]
pub fn options(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// registers a function as a HEAD route handler <br>
/// Example usage:
///
/// ```no_run
/// #[head("/health")]
/// ```
#[proc_macro_attribute]
pub fn head(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Registers a function as a QUERY route handler for requests with a query body.
///
/// ```no_run
/// #[query("/search")]
/// ```
#[proc_macro_attribute]
pub fn query(_args: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// inject service and config <br>
/// Example usage:
///
/// ```no_run
/// #[injectable_service]
/// impl ExampleService1 {
///     fn new(service2: Serv<Service2>, service3: Serv<Service3>, configuration1: CfgOption<Configuration1>) -> Self
///     {
///         //your logic to create ExampleService1
///     }
///     //other impl
/// }
/// ```
#[proc_macro_attribute]
pub fn inject(_args: TokenStream, item: TokenStream) -> TokenStream {
    dependency_injection::inject_service::expand(item)
}

/// middleware in the style of ASP.NET: the next middleware is given to the constructor <br>
/// Place it on `impl T { fn new(next: MiddlewareNext, ..); async fn invoke_async(&self, ctx, ..) }`. <br>
/// - parameters of `new` are resolved once when the pipeline is built (`MiddlewareNext`, `Serv<T>`, `CfgOption<T>`, `CfgRequire<T>`, `CfgReload<T>`). <br>
/// - parameters of `invoke_async` after `ctx` are resolved on every request from the request scope (`Serv<T>`, `CfgOption<T>`...). <br>
/// - it implements `Middleware` for the type and implements `DependencyInjectableMiddleware` when the block contains `fn new`. <br>
/// - for field injection, put `#[derive(DependencyInjectableMiddleware)]` on the struct and omit `fn new` from the annotated impl.
///
/// ```no_run
/// pub struct LogMiddleware {
///     next: MiddlewareNext,
/// }
/// #[middleware]
/// impl LogMiddleware {
///     pub fn new(next: MiddlewareNext) -> Self {
///         Self { next }
///     }
///     pub async fn invoke_async(&self, http_context: &mut HttpContext) {
///         self.next.next_async(http_context).await;
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn middleware(args: TokenStream, item: TokenStream) -> TokenStream {
    dependency_injection::inject_middleware::expand(args, item)
}

/// Implements `DependencyInjectableMiddleware` for a struct with one `MiddlewareNext` field.
///
/// Resolves `Serv<T>` and configuration fields when the pipeline is built.
/// Use `#[di(default)]` to initialize other fields with `Default::default()`.
#[proc_macro_derive(DependencyInjectableMiddleware, attributes(di))]
pub fn derive_dependency_injectable_middleware(item: TokenStream) -> TokenStream {
    dependency_injection::derive_middleware::derive_injectable_middleware(item)
}

/// inject httpcontext service and config <br>
/// Example usage:
///
/// ```no_run
/// #[controller_route("")]
/// #[controller_inject]
/// impl Controller1 {
///     fn new(http_context: HttpContextRef,service2: Serv<Service2>, service3: Serv<Service3>, configuration1: CfgOption<Configuration1>) -> Self
///     {
///         //your logic to create Controller1
///     }
///     //other impl
/// }
/// ```
#[proc_macro_attribute]
pub fn controller_inject(_args: TokenStream, item: TokenStream) -> TokenStream {
    dependency_injection::inject_controller::expand(item)
}

/// Injects a controller's service, configuration, and `HttpContextRef` fields from the request.
///
/// Use `#[di(default)]` on fields initialized with `Default::default()`.
/// Use `#[di(ignore_missing_http_context)]` on the struct to silence the warning
/// when it has no `HttpContextRef` field.
#[proc_macro_derive(DependencyInjectableController, attributes(di))]
pub fn derive_dependency_injectable_controller(item: TokenStream) -> TokenStream {
    dependency_injection::derive_controller::derive_injectable_controller(item)
}

/// Derive `DependencyInjectableService` directly from struct fields, when
/// every field is already `Serv<T>`, `CfgOption<T>`, `CfgRequire<T>`, or
/// `CfgReload<T>`. No `fn new` needed. <br>
/// Example usage:
///
/// ```no_run
/// #[derive(DependencyInjectableService)]
/// pub struct Ex2Service {
///     ex_service: Serv<ExService>,
///     audit_cfg: CfgOption<AuditConfiguration>,
/// }
/// ```
///
/// If a field needs custom construction (not a straight injected value),
/// use `#[inject]` on a hand-written `fn new` instead.
///
/// A field that is intentionally not injected can be marked `#[di(default)]`
/// to build it with `Default::default()` without the compiler warning that
/// an unmarked non-wrapper field would produce:
///
/// ```ignore
/// #[derive(DependencyInjectableService)]
/// pub struct Ex3Service {
///     ex_service: Serv<ExService>,
///     #[di(default)]
///     call_count: u32,
/// }
/// ```
#[proc_macro_derive(DependencyInjectableService, attributes(di))]
pub fn derive_dependency_injectable_service(item: TokenStream) -> TokenStream {
    dependency_injection::derive_service::derive_injectable_service(item)
}
