mod controller;
mod middleware;
use proc_macro::TokenStream;
#[proc_macro_attribute]
pub fn controller_route(args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::controller_route(args, item)
}
#[proc_macro_attribute]
pub fn route(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "")
}

/// registers a function as a GET route handler
#[proc_macro_attribute]
pub fn get(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "GET")
}

#[proc_macro_attribute]
pub fn post(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "POST")
}

#[proc_macro_attribute]
pub fn put(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "PUT")
}

#[proc_macro_attribute]
pub fn delete(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "DELETE")
}

#[proc_macro_attribute]
pub fn patch(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "PATCH")
}

#[proc_macro_attribute]
pub fn options(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "OPTIONS")
}

#[proc_macro_attribute]
pub fn head(_args: TokenStream, item: TokenStream) -> TokenStream {
    controller::routing::http_action(item, "HEAD")
}


/// #[middleware]
/// put it on the impl block of the Middleware trait
/// auto Service wrapper + impl MiddlewareService
///
/// REQUIRE:
///   async fn invoke_async(&self, ctx: &mut HttpContext, next: &impl MiddlewareService)
#[proc_macro_attribute]
pub fn middleware(_attr: TokenStream, item: TokenStream) -> TokenStream {
    middleware::middleware::middleware(_attr, item)
}