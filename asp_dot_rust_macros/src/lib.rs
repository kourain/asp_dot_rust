mod controller;
mod dependcy_injection;
mod struct_macro;
mod utils;

use proc_macro::TokenStream;

/// using the controller_route attribute to define a controller and its routes
///
///```no_run
///#[controller_route("")]
///impl HomeController {
///    #[get("/")]
///    pub async fn index(&mut self) -> impl ActionResult {
///       "Hello, World!"
///   }
/// }
/// ```
#[proc_macro_attribute]
pub fn controller_route(args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::controller_route(args, item)
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
    controller::routing::http_action(item, "")
}

/// registers a function as a GET route handler <br>
/// Example usage:
///
/// ```no_run
/// #[get("/health")]
/// ```
#[proc_macro_attribute]
pub fn get(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "GET")
}

/// registers a function as a POST route handler <br>
/// Example usage:
///
/// ```no_run
/// #[post("/health")]
/// ```
#[proc_macro_attribute]
pub fn post(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "POST")
}

/// registers a function as a PUT route handler <br>
/// Example usage:
///
/// ```no_run
/// #[put("/health")]
/// ```
#[proc_macro_attribute]
pub fn put(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "PUT")
}

/// registers a function as a DELETE route handler <br>
/// Example usage:
///
/// ```no_run
/// #[delete("/health")]
/// ```
#[proc_macro_attribute]
pub fn delete(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "DELETE")
}

/// registers a function as a PATCH route handler <br>
/// Example usage:
///
/// ```no_run
/// #[patch("/health")]
/// ```
#[proc_macro_attribute]
pub fn patch(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "PATCH")
}

/// registers a function as an OPTIONS route handler <br>
/// Example usage:
///
/// ```no_run
/// #[options("/health")]
/// ```
#[proc_macro_attribute]
pub fn options(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "OPTIONS")
}

/// registers a function as a HEAD route handler <br>
/// Example usage:
///
/// ```no_run
/// #[head("/health")]
/// ```
#[proc_macro_attribute]
pub fn head(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "HEAD")
}

///TODO: re-write
/// Auto Impl DependcyInjectableService for this Struct
// #[proc_macro_derive(DependcyInjectableService)]
// pub fn derive_di(input: TokenStream) -> TokenStream {
//     dependcy_injection::derive::derive_di(input)
// }

#[proc_macro_derive(StructName)]
pub fn struct_name(input: TokenStream) -> TokenStream {
    struct_macro::struct_name::struct_name(input)
}

/// inject service and config <br>
/// Example usage:
///
/// ```no_run
/// #[injectable_service]
/// impl ExampleService1 {
///     fn new(service2: Arc<Service2>, service3: Arc<Service3>, configuration1: Option<Arc<Configuration1>>) -> Self
///     {
///         //your logic to create ExampleService1
///     }
///     //other impl
/// }
/// ```
#[proc_macro_attribute]
pub fn injectable_service(args: TokenStream, item: TokenStream) -> TokenStream {
    dependcy_injection::inject_impliment::injectable_service(args, item)
}