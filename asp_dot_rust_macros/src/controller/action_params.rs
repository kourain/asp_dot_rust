use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{FnArg, Pat, PatType, Type};

use crate::utils::compiler_error::create_compiler_error;
use crate::utils::extract_type::extract_wrapper_inner;
use crate::utils::find_crate::asp_dot_rust_crate_path;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BinderKind {
    FromRoute,
    FromQuery,
    FromOptionQuery,
    FromBody,
    FromForm,
}

pub struct ActionParam {
    pub name: syn::Ident,
    pub kind: BinderKind,
    pub inner: Type,
    /// True when inner type is exactly `String` (path of one segment `String`).
    pub is_string: bool,
}

fn is_string_type(ty: &Type) -> bool {
    if let Type::Path(p) = ty {
        if p.path.segments.len() == 1 {
            return p.path.segments[0].ident == "String";
        }
    }
    false
}

fn binder_kind(ty: &Type) -> Option<(BinderKind, Type)> {
    for (name, kind) in [
        ("FromRoute", BinderKind::FromRoute),
        ("FromQuery", BinderKind::FromQuery),
        ("FromOptionQuery", BinderKind::FromOptionQuery),
        ("FromBody", BinderKind::FromBody),
        ("FromForm", BinderKind::FromForm),
    ] {
        if let Some(inner) = extract_wrapper_inner(ty, name) {
            return Some((kind, inner));
        }
    }
    None
}

/// Parse action method parameters after `&self` / `&mut self`.
/// Returns compiler error TokenStream on invalid params, or the list of binders.
pub fn parse_action_params(inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>, method_name: &syn::Ident) -> Result<Vec<ActionParam>, proc_macro::TokenStream> {
    let mut params = Vec::new();
    let mut body_or_form_count = 0usize;

    for arg in inputs {
        match arg {
            FnArg::Receiver(_) => continue,
            FnArg::Typed(PatType { pat, ty, .. }) => {
                let name = match pat.as_ref() {
                    Pat::Ident(pat_ident) => pat_ident.ident.clone(),
                    other => {
                        return Err(create_compiler_error(other, format!("action parameter on `{}` must be a plain identifier", method_name)));
                    }
                };

                let Some((kind, inner)) = binder_kind(ty) else {
                    return Err(create_compiler_error(
                        ty,
                        format!(
                            "action parameter `{}` on `{}` must be FromRoute<T>, FromQuery<T>, FromOptionQuery<T>, FromBody<T> or FromForm<T>",
                            name, method_name
                        ),
                    ));
                };

                if matches!(kind, BinderKind::FromBody | BinderKind::FromForm) {
                    body_or_form_count += 1;
                    if body_or_form_count > 1 {
                        return Err(create_compiler_error(ty, format!("action `{}` may have at most one FromBody or FromForm parameter", method_name)));
                    }
                }
                let is_string = is_string_type(&inner);
                params.push(ActionParam { name, kind, inner, is_string });
            }
        }
    }

    Ok(params)
}

/// Generate let-bindings and argument list for the action invoke.
/// On bind failure sets status/body and returns from the match arm.
pub fn generate_bind_code(params: &[ActionParam]) -> (TokenStream2, Vec<TokenStream2>) {
    let main = asp_dot_rust_crate_path();
    let mut lets = Vec::new();
    let mut args = Vec::new();

    for param in params {
        let name = &param.name;
        let name_lit = name.to_string();
        let inner = &param.inner;
        let wrapper = match param.kind {
            BinderKind::FromRoute => quote! { #main::controller::FromRoute<#inner> },
            BinderKind::FromQuery => quote! { #main::controller::FromQuery<#inner> },
            BinderKind::FromOptionQuery => quote! { #main::controller::FromOptionQuery<#inner> },
            BinderKind::FromBody => quote! { #main::controller::FromBody<#inner> },
            BinderKind::FromForm => quote! { #main::controller::FromForm<#inner> },
        };

        let bind_expr = match param.kind {
            BinderKind::FromRoute => {
                quote! { #wrapper::bind(#name_lit, &self.http_context) }
            }
            BinderKind::FromQuery => {
                quote! { #wrapper::bind(#name_lit, &self.http_context) }
            }
            BinderKind::FromOptionQuery => {
                quote! { Ok::<_, #main::controller::BindError>(#wrapper::bind(#name_lit, &self.http_context)) }
            }
            BinderKind::FromBody if param.is_string => {
                quote! { #wrapper::bind(&mut self.http_context).await }
            }
            BinderKind::FromBody => {
                quote! { #wrapper::bind_serde(&mut self.http_context).await }
            }
            BinderKind::FromForm if param.is_string => {
                quote! { #wrapper::bind(&mut self.http_context).await }
            }
            BinderKind::FromForm => {
                quote! { #wrapper::bind_serde(&mut self.http_context).await }
            }
        };

        lets.push(quote! {
            let #name = match #bind_expr {
                Ok(value) => value,
                Err(err) => {
                    self.http_context.response.status_code = err.status_code();
                    self.http_context.response.body = err.to_string().into_bytes();
                    self.http_context.response.headers.set_content_type("text/plain");
                    let body_len = self.http_context.response.body.len();
                    self.http_context.response.headers.set_content_length(body_len);
                    return;
                }
            };
        });
        args.push(quote! { #name });
    }

    (quote! { #(#lets)* }, args)
}
