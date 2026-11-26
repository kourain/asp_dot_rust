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
    /// `FromQuery<T>`, also `FromOptionQuery<T>` which is `FromQuery<Option<T>>`.
    FromQuery,
    /// One field of the JSON / XML body, also `FromOptionBody<T>`.
    FromBody,
    /// One field of the form body, also `FromOptionForm<T>`.
    FromForm,
    /// The whole body.
    Body,
    /// The whole form body.
    Form,
}

pub struct ActionParam {
    pub name: syn::Ident,
    pub kind: BinderKind,
    /// The type given to the binder. An optional field is `Option<T>`, also when it was written `FromOptionQuery<T>`.
    pub inner: Type,
    /// The type of the value itself: `T` for an optional field, `inner` otherwise. Query rules are checked on it.
    pub value: Type,
    /// True when the field is optional (`Option<T>`).
    pub optional: bool,
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

/// `T` of `Option<T>`, `std::option::Option<T>` or `core::option::Option<T>`.
fn option_inner(ty: &Type) -> Option<Type> {
    let Type::Path(path) = ty else { return None };
    if path.qself.is_some() {
        return None;
    }
    let names: Vec<String> = path.path.segments.iter().map(|segment| segment.ident.to_string()).collect();
    let is_option = matches!(names.iter().map(String::as_str).collect::<Vec<_>>().as_slice(), ["Option"] | ["std" | "core", "option", "Option"]);
    if !is_option {
        return None;
    }
    let syn::PathArguments::AngleBracketed(arguments) = &path.path.segments.last()?.arguments else {
        return None;
    };
    match arguments.args.first()? {
        syn::GenericArgument::Type(inner) if arguments.args.len() == 1 => Some(inner.clone()),
        _ => None,
    }
}

/// The binder of a parameter type, the type it is given and whether the alias form (`FromOptionX<T>`) was written.
fn binder_kind(ty: &Type) -> Option<(BinderKind, Type)> {
    for (name, kind, alias_of_option) in [
        ("FromRoute", BinderKind::FromRoute, false),
        ("FromQuery", BinderKind::FromQuery, false),
        ("FromOptionQuery", BinderKind::FromQuery, true),
        ("FromBody", BinderKind::FromBody, false),
        ("FromOptionBody", BinderKind::FromBody, true),
        ("FromForm", BinderKind::FromForm, false),
        ("FromOptionForm", BinderKind::FromForm, true),
        ("Body", BinderKind::Body, false),
        ("Form", BinderKind::Form, false),
    ] {
        if let Some(inner) = extract_wrapper_inner(ty, name) {
            // `FromOptionQuery<T>` is `FromQuery<Option<T>>`, so both spellings reach the binder as `Option<T>`.
            let inner = if alias_of_option { syn::parse_quote!(::core::option::Option<#inner>) } else { inner };
            return Some((kind, inner));
        }
    }
    None
}

/// Parse action method parameters after `&self` / `&mut self`.
/// Returns compiler error TokenStream on invalid params, or the list of binders.
pub fn parse_action_params(inputs: &mut syn::punctuated::Punctuated<FnArg, syn::token::Comma>, method_name: &syn::Ident) -> Result<Vec<ActionParam>, proc_macro::TokenStream> {
    let mut params = Vec::new();
    let mut has_form_binder = false;
    let mut has_body_binder = false;

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
                            "action parameter `{}` on `{}` must be FromRoute<T>, FromQuery<T>, FromOptionQuery<T>, FromBody<T>, FromOptionBody<T>, FromForm<T>, FromOptionForm<T>, Body<T> or Form<T>",
                            name, method_name
                        ),
                    ));
                };

                // A request has one body, so its binders must read the same kind of body.
                match kind {
                    BinderKind::FromForm | BinderKind::Form => has_form_binder = true,
                    BinderKind::FromBody | BinderKind::Body => has_body_binder = true,
                    _ => {}
                }
                if has_form_binder && has_body_binder {
                    return Err(create_compiler_error(
                        ty,
                        format!("action `{}` cannot mix form binders (FromForm, Form) with body binders (FromBody, Body)", method_name),
                    ));
                }
                let option_value = if matches!(kind, BinderKind::Body | BinderKind::Form) { None } else { option_inner(&inner) };
                if kind == BinderKind::FromRoute && option_value.is_some() {
                    return Err(create_compiler_error(
                        ty,
                        format!("route parameter `{}` on `{}` cannot be optional: a route parameter is always present, use FromRoute<T>", name, method_name),
                    ));
                }
                let optional = option_value.is_some();
                let value = option_value.unwrap_or_else(|| inner.clone());
                let is_string = is_string_type(&inner);
                let mut validation = None;
                for attr in attrs.iter().filter(|attr| attr.path().is_ident("validate")) {
                    if kind != BinderKind::FromQuery {
                        return Err(create_compiler_error(attr, "#[validate(...)] requires FromQuery or FromOptionQuery"));
                    }
                    if validation.is_some() {
                        return Err(create_compiler_error(attr, "duplicate #[validate(...)] on this query parameter"));
                    }
                    validation = Some(QueryValidation::parse(attr).map_err(|e| proc_macro::TokenStream::from(e.into_compile_error()))?);
                }
                attrs.retain(|attr| !attr.path().is_ident("validate"));
                params.push(ActionParam { name, kind, inner, value, optional, is_string, validation });
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
            BinderKind::FromBody => quote! { #main::controller::FromBody::<#inner> },
            BinderKind::FromForm => quote! { #main::controller::FromForm::<#inner> },
            BinderKind::Body => quote! { #main::controller::Body::<#inner> },
            BinderKind::Form => quote! { #main::controller::Form::<#inner> },
        };

        let bind_expr = match param.kind {
            BinderKind::FromRoute | BinderKind::FromQuery => {
                quote! { #wrapper::bind(#name_lit, &self.http_context)? }
            }
            BinderKind::FromBody | BinderKind::FromForm => {
                quote! { #wrapper::bind(#name_lit, &mut self.http_context).await? }
            }
            BinderKind::Body | BinderKind::Form if param.is_string => {
                quote! { #wrapper::bind(&mut self.http_context).await? }
            }
            BinderKind::Body | BinderKind::Form => {
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
            let check = rule.check(&value, &name_lit, &param.value);
            // A missing optional parameter skips the rule.
            lets.push(if param.optional {
                quote! { if let ::core::option::Option::Some(#value) = &#name.0 { #check } }
            } else {
                quote! { { let #value = &#name.0; #check } }
            });
        }
        args.push(quote! { #name });
    }

    (quote! { #(#lets)* }, args)
}
