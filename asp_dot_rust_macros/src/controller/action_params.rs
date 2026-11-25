use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{FnArg, Pat, PatType, Type};

use super::query_validation::QueryValidation;
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
    validation: Option<QueryValidation>,
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
pub fn parse_action_params(inputs: &mut syn::punctuated::Punctuated<FnArg, syn::token::Comma>, method_name: &syn::Ident) -> Result<Vec<ActionParam>, proc_macro::TokenStream> {
    let mut params = Vec::new();
    let mut body_or_form_count = 0usize;

    for arg in inputs {
        match arg {
            FnArg::Receiver(_) => continue,
            FnArg::Typed(PatType { pat, ty, attrs, .. }) => {
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
                let mut validation = None;
                for attr in attrs.iter().filter(|attr| attr.path().is_ident("validate")) {
                    if !matches!(kind, BinderKind::FromQuery | BinderKind::FromOptionQuery) {
                        return Err(create_compiler_error(attr, "#[validate(...)] requires FromQuery or FromOptionQuery"));
                    }
                    if validation.is_some() {
                        return Err(create_compiler_error(attr, "duplicate #[validate(...)] on this query parameter"));
                    }
                    validation = Some(QueryValidation::parse(attr).map_err(|e| proc_macro::TokenStream::from(e.into_compile_error()))?);
                }
                attrs.retain(|attr| !attr.path().is_ident("validate"));
                params.push(ActionParam { name, kind, inner, is_string, validation });
            }
        }
    }

    Ok(params)
}

/// Generate let-bindings and argument list for the action invoke.
/// Fallible binders return their error to the routing middleware.
pub fn generate_bind_code(params: &[ActionParam]) -> (TokenStream2, Vec<TokenStream2>) {
    let main = asp_dot_rust_crate_path();
    let mut lets = Vec::new();
    let mut args = Vec::new();

    for param in params {
        let name = &param.name;
        let name_lit = name.to_string();
        let inner = &param.inner;
        let wrapper = match param.kind {
            BinderKind::FromRoute => quote! { #main::controller::FromRoute::<#inner> },
            BinderKind::FromQuery => quote! { #main::controller::FromQuery::<#inner> },
            BinderKind::FromOptionQuery => quote! { #main::controller::FromOptionQuery::<#inner> },
            BinderKind::FromBody => quote! { #main::controller::FromBody::<#inner> },
            BinderKind::FromForm => quote! { #main::controller::FromForm::<#inner> },
        };

        let bind_expr = match param.kind {
            BinderKind::FromRoute | BinderKind::FromQuery => {
                quote! { #wrapper::bind(#name_lit, &self.http_context)? }
            }
            BinderKind::FromOptionQuery if param.validation.is_some() => {
                quote! { #wrapper::bind_if_present(#name_lit, &self.http_context)? }
            }
            BinderKind::FromOptionQuery => {
                quote! { #wrapper::bind(#name_lit, &self.http_context) }
            }
            BinderKind::FromBody | BinderKind::FromForm if param.is_string => {
                quote! { #wrapper::bind(&mut self.http_context).await? }
            }
            BinderKind::FromBody | BinderKind::FromForm => {
                quote! {{
                    #[allow(unused_imports)]
                    use #main::controller::validation::SkipValidation as _;
                    let bound = #wrapper::bind_serde(&mut self.http_context).await?;
                    #main::controller::validation::ValidationCheck(&bound.0)
                        .check()
                        .map_err(#main::controller::BindError::Validation)?;
                    bound
                }}
            }
        };

        lets.push(quote! { let #name = #bind_expr; });
        if let Some(rule) = &param.validation {
            let value = syn::Ident::new("__asp_query_value", proc_macro2::Span::mixed_site());
            let check = rule.check(&value, &name_lit, inner);
            lets.push(if param.kind == BinderKind::FromOptionQuery {
                quote! { if let ::core::option::Option::Some(#value) = &#name.0 { #check } }
            } else {
                quote! { { let #value = &#name.0; #check } }
            });
        }
        args.push(quote! { #name });
    }

    (quote! { #(#lets)* }, args)
}
