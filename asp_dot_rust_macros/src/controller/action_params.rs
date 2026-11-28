use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::{FnArg, Pat, PatType, Type, ext::IdentExt, spanned::Spanned};

use super::{attribute::attr_named, bind::BindOptions, validation::ControllerActionValidation};
use crate::utils::compiler_error::create_compiler_error;
use crate::utils::extract_type::type_name;
use crate::utils::find_crate::asp_dot_rust_crate_path;

#[derive(Clone, Copy, PartialEq, Eq)]
enum BinderKind {
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
    name: syn::Ident,
    kind: BinderKind,
    wrapper: Type,
    bind: BindOptions,
    validation: Option<ControllerActionValidation>,
}

fn binder_kind(ty: &Type) -> Option<BinderKind> {
    match type_name(ty)?.to_string().as_str() {
        "FromRoute" => Some(BinderKind::FromRoute),
        "FromQuery" | "FromOptionQuery" => Some(BinderKind::FromQuery),
        "FromBody" | "FromOptionBody" => Some(BinderKind::FromBody),
        "FromForm" | "FromOptionForm" => Some(BinderKind::FromForm),
        "Body" => Some(BinderKind::Body),
        "Form" => Some(BinderKind::Form),
        _ => None,
    }
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

                let Some(kind) = binder_kind(ty) else {
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
                let mut validation = None;
                for attr in attrs.iter().filter(|attr| attr_named(attr, "validate")) {
                    if matches!(kind, BinderKind::Body | BinderKind::Form) {
                        return Err(create_compiler_error(attr, "#[validate(...)] requires a route, query, form or body field binder"));
                    }
                    if validation.is_some() {
                        return Err(create_compiler_error(attr, "duplicate #[validate(...)] on this action parameter"));
                    }
                    validation = Some(ControllerActionValidation::parse(attr).map_err(|e| proc_macro::TokenStream::from(e.into_compile_error()))?);
                }
                let mut bind = None;
                for attr in attrs.iter().filter(|attr| attr_named(attr, "bind")) {
                    if matches!(kind, BinderKind::Body | BinderKind::Form) {
                        return Err(create_compiler_error(attr, "#[bind(...)] requires a route, query, form or body field binder"));
                    }
                    if bind.is_some() {
                        return Err(create_compiler_error(attr, "duplicate #[bind(...)] on this action parameter"));
                    }
                    bind = Some(BindOptions::parse(attr).map_err(|e| proc_macro::TokenStream::from(e.into_compile_error()))?);
                }
                attrs.retain(|attr| !attr_named(attr, "validate") && !attr_named(attr, "bind"));
                params.push(ActionParam { name, kind, wrapper: (**ty).clone(), bind: bind.unwrap_or_default(), validation });
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
        let name_lit = param.bind.name.as_ref().map_or_else(|| name.unraw().to_string(), syn::LitStr::value);
        let wrapper = &param.wrapper;
        // Put the generated binder call on the parameter type, so a rejected type is reported there and not on `#[controller_route]`.
        let span = wrapper.span();
        let defaulted = syn::Ident::new("__asp_used_default", proc_macro2::Span::mixed_site());
        let (binding, call, default_arg) = match param.bind.default_value() {
            Some(default) => (
                quote!((#name, #defaulted)),
                quote_spanned!(span=> bind_with_default),
                quote!(, ::core::option::Option::Some(|| #default)),
            ),
            None => (quote!(#name), quote_spanned!(span=> bind), TokenStream2::new()),
        };
        let bind_expr = match param.kind {
            BinderKind::FromRoute | BinderKind::FromQuery => {
                quote_spanned! { span=> <#wrapper>::#call(#name_lit, &self.http_context #default_arg)? }
            }
            BinderKind::FromBody | BinderKind::FromForm => {
                quote_spanned! { span=> <#wrapper>::#call(#name_lit, &mut self.http_context #default_arg).await? }
            }
            BinderKind::Body | BinderKind::Form => {
                quote_spanned! { span=> {
                    #[allow(unused_imports)]
                    use #main::controller::{action_params::BindSerdeBody as _, validation::SkipValidation as _};
                    let bound = <#wrapper>::bind(&mut self.http_context).await?;
                    #main::controller::validation::ValidationCheck(&bound.0)
                        .check()
                        .map_err(#main::controller::BindError::Validation)?;
                    bound
                }}
            }
        };

        lets.push(quote! { let #binding = #bind_expr; });
        if let Some(rule) = &param.validation {
            let check = rule.check(name, &name_lit);
            if param.bind.default.is_some() && param.bind.validate_default.as_ref().is_some_and(|flag| !flag.value) {
                lets.push(quote! { if !#defaulted { #check } });
            } else {
                lets.push(check);
            }
        }
        args.push(quote! { #name });
    }

    (quote! { #(#lets)* }, args)
}
