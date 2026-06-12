use proc_macro::TokenStream;

use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ExprLit, ImplItem, ItemImpl, Lit, LitStr, Token, parse_macro_input, punctuated::Punctuated};

pub(crate) fn http_action(item: TokenStream, _method: &str) -> TokenStream {
    item
}

fn is_route_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("get")
        || attr.path().is_ident("post")
        || attr.path().is_ident("put")
        || attr.path().is_ident("delete")
        || attr.path().is_ident("patch")
        || attr.path().is_ident("options")
        || attr.path().is_ident("head")
        || attr.path().is_ident("route")
}

fn lit_str_from_expr(expr: &Expr) -> Option<LitStr> {
    match expr {
        Expr::Lit(ExprLit { lit: Lit::Str(value), .. }) => Some(value.clone()),
        _ => None,
    }
}
fn list_lit_str_from_expr(expr: &Expr) -> Option<Vec<LitStr>> {
    if let Expr::Array(array) = expr {
        let mut result = Vec::new();
        for elem in &array.elems {
            if let Expr::Lit(ExprLit { lit: Lit::Str(value), .. }) = elem {
                result.push(value.clone());
            } else {
                return None;
            }
        }
        Some(result)
    } else {
        None
    }
}

fn push_route_registration(registrations: &mut Vec<proc_macro2::TokenStream>, method_name_lit: &LitStr, http_method: Vec<LitStr>, path_lit: &LitStr) {
    registrations.push(quote! {
        ::asp_dot_rust::controller::ActionRoute::new(
            #method_name_lit,
            vec![#(#http_method),*],
            #path_lit,
        )
    });
}

fn push_match_route(routes: &mut Vec<proc_macro2::TokenStream>, method_name_lit: &LitStr, method_name: &syn::Ident, is_async: bool) {
    let invoke = if is_async {
        quote! { self.#method_name().await }
    } else {
        quote! { self.#method_name() }
    };

    routes.push(quote! {
        #method_name_lit =>
        {
            let (body, content_type, status_code) = {
                let action_result = #invoke;
                let body = action_result.get_body_async().await;
                let content_type = action_result.content_type();
                let status_code = action_result.status_code();
                (body, content_type.to_string(), status_code)
            };

            self.http_context.response.body = body;
            self.http_context.response.status_code = status_code;
            if(!content_type.is_empty()) {
                self.http_context.response.headers.set_content_type(&content_type);
            }
        }
    });
}

pub(crate) fn controller_route(args: TokenStream, item: TokenStream) -> TokenStream {
    let root_route = parse_macro_input!(args as LitStr);
    let input_impl = parse_macro_input!(item as ItemImpl);
    let self_ty = input_impl.self_ty.as_ref();
    let controller_bootstrap_name = syn::Ident::new(&format!("__asp_register_{}", quote! {#self_ty}), input_impl.span());
    let controller_bootstrap_fn_name = syn::Ident::new(&format!("__asp_register_{}_routes", quote! {#self_ty}), input_impl.span());
    let mut action_route_registrations = Vec::new();
    let mut match_routes = Vec::new();
    for impl_item in &input_impl.items {
        let ImplItem::Fn(impl_method) = impl_item else {
            continue;
        };

        let method_name = &impl_method.sig.ident;
        let is_async = impl_method.sig.asyncness.is_some();

        for attr in impl_method.attrs.iter().filter(|attr| is_route_attr(attr)) {
            let route_ident = attr.path().segments.last().unwrap().ident.to_string();

            let method_name_lit = LitStr::new(&method_name.to_string(), method_name.span());

            if route_ident == "route" {
                let values: Punctuated<Expr, Token![,]> = attr
                    .parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)
                    .unwrap_or_else(|_| panic!("Invalid #[route(...)] on {}", method_name));
                let mut route_iter = values.iter();
                let method_expr = route_iter.next().expect("Missing HTTP method in #[route]");
                let path_expr = route_iter.next().expect("Missing route path in #[route]");
                let method_lit = list_lit_str_from_expr(method_expr).unwrap_or_else(|| panic!("HTTP method in #[route] must be a string literal on {}", method_name));
                let path_lit = lit_str_from_expr(path_expr).unwrap_or_else(|| panic!("Route path in #[route] must be a string literal on {}", method_name));
                push_route_registration(&mut action_route_registrations, &method_name_lit, method_lit, &path_lit);
                push_match_route(&mut match_routes, &method_name_lit, method_name, is_async);
            } else {
                let path_lit: LitStr = attr.parse_args().unwrap_or_else(|_| panic!("Invalid #[{}(...)] on {}", route_ident, method_name));
                let http_method_string = route_ident.to_uppercase();
                let http_method = LitStr::new(&http_method_string, attr.span());
                push_route_registration(&mut action_route_registrations, &method_name_lit, vec![http_method], &path_lit);
                push_match_route(&mut match_routes, &method_name_lit, method_name, is_async);
            };
        }
    }

    let expanded = quote! {
        #[allow(dead_code)]
        #input_impl
        /// impl by #[controller_route] macro
        #[async_trait::async_trait]
        impl ::asp_dot_rust::controller::Routing for #self_ty {
            async fn routing(&mut self, method_name: String){
                match method_name.as_str() {
                    #(#match_routes)*
                    _ => {
                        self.http_context.response.status_code = http::StatusCode::NOT_FOUND;
                        self.http_context.response.body = http::StatusCode::NOT_FOUND.canonical_reason().unwrap_or("Not Found").as_bytes().to_vec();
                    }
                };
                let body_len = self.http_context.response.body.len();
                self.http_context.response.headers.set_content_length(body_len);
            }
        }
        #[allow(nonstandard_style)]
        /// This function is generated by the #[controller_route] macro and is used to register the controller and its routes with the routing service at application startup.
        const #controller_bootstrap_name: () = {
            /// This function is called at application startup to register the controller and its routes with the routing service.
            fn #controller_bootstrap_fn_name() -> ::asp_dot_rust::services::routing::ControllerCollect {
                ::asp_dot_rust::services::routing::register_controller::<#self_ty>(
                    #root_route,
                    vec![
                        #(#action_route_registrations,)*
                    ],
                )
            }
            /// inventory submit to register the bootstrap function
            ::asp_dot_rust::inventory::submit! {
                ::asp_dot_rust::services::routing::ControllerBootstrapRegistration {
                    bootstrap: #controller_bootstrap_fn_name,
                }
            }
        };
    };

    expanded.into()
}
