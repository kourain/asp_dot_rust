use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ExprLit, ImplItem, ItemImpl, Lit, LitStr, Token, parse_macro_input, punctuated::Punctuated};

use crate::utils::compiler_error::create_compiler_error;
use crate::utils::find_crate::asp_dot_rust_crate_path;

const HTTP_METHOD: [&str; 7] = ["get", "post", "put", "delete", "patch", "options", "head"];

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

fn is_http_method(method: impl AsRef<str>) -> bool {
    let method = method.as_ref();
    HTTP_METHOD.iter().any(|m| m.eq_ignore_ascii_case(method))
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
    let main_crate_path = asp_dot_rust_crate_path();
    registrations.push(quote! {
        #main_crate_path::controller::ActionRoute::new(
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
    let main_crate_path = asp_dot_rust_crate_path();
    let root_route = parse_macro_input!(args as LitStr);
    let original_input = item.clone();
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
                let values;
                if let Ok(val) = attr.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated) {
                    values = val;
                } else {
                    return create_compiler_error(attr, format!("Invalid #[route(...)] on `{}`", method_name));
                }

                let mut route_iter = values.iter();
                let method_expr;
                if let Some(val) = route_iter.next() {
                    method_expr = val;
                } else {
                    return create_compiler_error(attr, "Missing HTTP method or path in #[route]");
                }

                let path_expr;
                if let Some(val) = route_iter.next() {
                    path_expr = val;
                } else {
                    return create_compiler_error(attr, "Missing route method or path in #[route]");
                }

                let method_lit;
                if let Some(val) = list_lit_str_from_expr(method_expr) {
                    method_lit = val;
                    for method in method_lit.iter() {
                        let method_str = method.token().to_string();
                        if is_http_method(method_str.trim_matches('"').to_string()) == false {
                            return create_compiler_error(attr, format!("Invalid HTTP method `{}` in #[route] on `{}`", method_str, method_name));
                        }
                    }
                } else {
                    return create_compiler_error(attr, format!("HTTP method in #[route] must be a string literal on `{}`", method_name));
                }

                let path_lit;
                if let Some(val) = lit_str_from_expr(path_expr) {
                    path_lit = val;
                } else {
                    return create_compiler_error(attr, format!("Route path in #[route] must be a string literal on `{}`", method_name));
                }

                push_route_registration(&mut action_route_registrations, &method_name_lit, method_lit, &path_lit);
                push_match_route(&mut match_routes, &method_name_lit, method_name, is_async);
            } else {
                let path_lit: LitStr;
                if let Ok(val) = attr.parse_args() {
                    path_lit = val;
                } else {
                    return create_compiler_error(attr, format!("Invalid #[{}(...)] on `{}`", route_ident, method_name));
                }
                let http_method_string = route_ident.to_uppercase();
                let http_method = LitStr::new(&http_method_string, attr.span());
                push_route_registration(&mut action_route_registrations, &method_name_lit, vec![http_method], &path_lit);
                push_match_route(&mut match_routes, &method_name_lit, method_name, is_async);
            };
        }
    }

    let input_impl_tokens = proc_macro2::TokenStream::from(original_input);

    let expanded = quote! {
        use #main_crate_path::http_context::AspDotRustHttpHeader;
        #input_impl_tokens
        /// impl by #[controller_route] macro
        #[async_trait::async_trait]
        impl #main_crate_path::controller::Routing for #self_ty {
            async fn routing(&mut self, method_name: &'static str) {
                match method_name {
                    #(#match_routes)*
                    _ => {
                        self.http_context.response.status_code = http::StatusCode::NOT_FOUND;
                        self.http_context.response.body = http::StatusCode::NOT_FOUND.canonical_reason().unwrap().as_bytes().to_vec();
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
            fn #controller_bootstrap_fn_name() -> #main_crate_path::services::routing::ControllerCollect {
                #main_crate_path::services::routing::register_controller::<#self_ty>(
                    #root_route,
                    vec![
                        #(#action_route_registrations,)*
                    ],
                )
            }
            /// inventory submit to register the bootstrap function
            #main_crate_path::inventory::submit! {
                #main_crate_path::services::routing::ControllerBootstrapRegistration {
                    bootstrap: #controller_bootstrap_fn_name,
                }
            }
        };
    };

    expanded.into()
}
