use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{FnArg, ImplItem, ImplItemFn, ItemImpl, Pat, PatType, ReturnType, Type, parse_macro_input, parse_quote};

pub fn middleware(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);

    // get name from self type
    // eg: AuthorizeMiddleware
    let self_ty = &input.self_ty;
    let struct_name = match self_ty.as_ref() {
        Type::Path(p) => p.path.segments.last().unwrap().ident.clone(),
        _ => panic!("#[middleware] only usable on struct impl blocks"),
    };

    // name Service wrapper
    // eg: AuthorizeMiddleware → AuthorizeMiddlewareService
    let service_name = format_ident!("{}Service", struct_name);

    // find invoke_async in impl block
    let invoke_fn = input
        .items
        .iter()
        .find_map(|item| {
            if let ImplItem::Fn(f) = item {
                if f.sig.ident == "invoke_async" {
                    return Some(f.clone());
                }
            }
            None
        })
        .expect("#[middleware] requires the `invoke_async` function");

    let body = &invoke_fn.block;

    // Keep other functions (with_application, type_name,...)
    let other_items: Vec<&ImplItem> = input
        .items
        .iter()
        .filter(|item| if let ImplItem::Fn(f) = item { f.sig.ident != "invoke_async" } else { true })
        .collect();

    let generics = &input.generics;
    let where_clause = &generics.where_clause;

    quote! {
        // Keep other functions (with_application, type_name,...)
        impl #generics #self_ty #where_clause {
            #(#other_items)*
        }

        // Service wrapper được sinh tự động
        pub struct #service_name<S> {
            middleware: #self_ty,
            inner: S,
        }

        // impl MiddlewareService cho wrapper
        impl<S> crate::middleware::MiddlewareService for #service_name<S>
        where
            S: crate::middleware::MiddlewareService + Send + Sync,
        {
            fn invoke_async<'__a>(
                &'__a self,
                http_context: &'__a mut crate::http_context::HttpContext,
            ) -> impl std::future::Future<Output = ()> + Send + '__a {
                // this = &self.middleware to invoke_async invoke_async
                let this = &self.middleware;
                let next = &self.inner;
                async move #body
            }
        }

        // impl Middleware<S> to use .with(...)
        impl<S> crate::middleware::Middleware<S> for #self_ty
        where
            S: crate::middleware::MiddlewareService + Send + Sync + 'static,
        {
            type Service = #service_name<S>;

            fn wrap(self, inner: S) -> Self::Service {
                #service_name {
                    middleware: self,
                    inner,
                }
            }
        }
    }
    .into()
}
