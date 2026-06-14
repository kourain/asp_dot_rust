mod controller;
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

/// registers a function as a route handler for the specified HTTP method(s)
///
/// ```no_run
/// #[route(["GET", "POST"], "/health")]
/// ```
#[proc_macro_attribute]
pub fn route(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "")
}

/// registers a function as a GET route handler
///
/// ```no_run
/// #[get("/health")]
/// ```
#[proc_macro_attribute]
pub fn get(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "GET")
}

/// registers a function as a POST route handler
///
/// ```no_run
/// #[post("/health")]
/// ```
#[proc_macro_attribute]
pub fn post(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "POST")
}
    
/// registers a function as a PUT route handler
///
/// ```no_run
/// #[put("/health")]
/// ```
#[proc_macro_attribute]
pub fn put(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "PUT")
}

/// registers a function as a DELETE route handler
///
/// ```no_run
/// #[delete("/health")]
/// ```
#[proc_macro_attribute]
pub fn delete(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "DELETE")
}

/// registers a function as a PATCH route handler
///
/// ```no_run
/// #[patch("/health")]
/// ```
#[proc_macro_attribute]
pub fn patch(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "PATCH")
}

/// registers a function as an OPTIONS route handler
///
/// ```no_run
/// #[options("/health")]
/// ```
#[proc_macro_attribute]
pub fn options(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "OPTIONS")
}

/// registers a function as a HEAD route handler
///
/// ```no_run
/// #[head("/health")]
/// ```
#[proc_macro_attribute]
pub fn head(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "HEAD")
}
