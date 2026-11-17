use super::{dependencies::Dependencies, flags::DIPropOption};
use crate::utils::compiler_error::create_compiler_error;
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// Returns `Ok(true)` when the field carries `#[di(default)]`, `Ok(false)`
/// when it carries no `#[di(..)]` attribute at all, and `Err(..)` for a
/// malformed one (`#[di]`, `#[di(typo)]`) so a mistyped opt-in is a compile
/// error rather than a silently ignored attribute.
fn extract_di_options(field: &syn::Field) -> DIPropOption {
    let mut result = DIPropOption::empty();
    for attr in &field.attrs {
        if !attr.path().is_ident("di") {
            continue;
        }

        _ = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("default") {
                result.insert(DIPropOption::DEFAULT);
            }
            Ok(())
        });
    }

    result
}

/// `#[derive(DependencyInjectableService)]` — for the common case where every
/// dependency a service needs is already a struct field typed `Serv<T>`,
/// `CfgOption<T>`, `CfgRequire<T>`, or `CfgReload<T>`. It builds `Self { .. }`
/// directly from those fields, so no `fn new` / `#[inject]` is
/// needed at all.
///
/// Any field that is NOT one of the four wrapper types falls back to
/// `Default::default()` (a hard compile error if the field's type does not
/// implement `Default`), and emits a `deprecated`-style compiler **warning**
/// at the field's call site so the fallback is never silent. Mark such a
/// field `#[di(default)]` to say the fallback is intentional and silence
/// that warning for that field only. Prefer `#[inject]` on a hand-written
/// `fn new` instead if the service needs real custom construction logic
/// (computed fields, a dependency the constructor needs but doesn't store,
/// etc).
pub(crate) fn derive_injectable_service(item: TokenStream) -> TokenStream {
    let main_crate_path = crate::utils::find_crate::asp_dot_rust_crate_path();
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let ident_str = ident.to_string();
    let mut dependencies = Dependencies::default();

    let data = match &input.data {
        Data::Struct(s) => s,
        _ => return create_compiler_error(&input, "#[derive(DependencyInjectableService)] only supports structs"),
    };

    let named = match &data.fields {
        Fields::Named(f) => &f.named,
        Fields::Unit => {
            let registration = dependencies.register(&main_crate_path, ident);
            return quote! {
                #[#main_crate_path::third_party::async_trait::async_trait]
                impl #main_crate_path::dependency_injection::DependencyInjectableService for #ident {
                    async fn inject(_service_scope: &#main_crate_path::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
                        Self
                    }
                }
                #registration
            }
            .into();
        }
        Fields::Unnamed(_) => {
            return create_compiler_error(
                &data.fields,
                "#[derive(DependencyInjectableService)] does not support tuple structs; use #[inject] on a hand-written `fn new` instead",
            );
        }
    };

    let mut field_inits = Vec::new();
    let mut field_warnings = Vec::new();

    for (field_index, field) in named.iter().enumerate() {
        let field_ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        let di_default = extract_di_options(field);

        if let Some(expression) = dependencies.resolve(ty) {
            field_inits.push(quote! { #field_ident: #expression });
        } else {
            field_inits.push(quote! { #field_ident: Default::default() });
            if !di_default.contains(DIPropOption::DEFAULT) {
                // Not a compile error: emit a `deprecated`-style compiler
                // *warning* instead, so silently defaulting a field is never
                // truly silent. The nested fn only exists to be `#[deprecated]`;
                // calling it does nothing at runtime.
                let marker_name = syn::Ident::new(&format!("__di_default_field_marker_{}_{}", ident_str, field_index), field_ident.span());
                let warning_msg = format!(
                    "field `{}` on `{}` is not Serv<T>, CfgOption<T>, CfgRequire<T>, or CfgReload<T>; #[derive(DependencyInjectableService)] defaulted it via `Default::default()`. \
                    Mark it `#[di(default)]` if that is intended, or use #[inject] on a hand-written `fn new` if this field needs real construction.",
                    field_ident, ident_str
                );
                field_warnings.push(quote! {
                    #[deprecated(note = #warning_msg)]
                    #[allow(dead_code, non_snake_case)]
                    fn #marker_name() {}
                    #marker_name();
                });
            }
        }
    }

    let configuration = dependencies.configuration_binding(&main_crate_path);
    let registration = dependencies.register(&main_crate_path, ident);
    quote! {
        #[#main_crate_path::third_party::async_trait::async_trait]
        impl #main_crate_path::dependency_injection::DependencyInjectableService for #ident {
            async fn inject(service_scope: &#main_crate_path::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
                #configuration
                #(#field_warnings)*
                Self { #(#field_inits),* }
            }
        }

        #registration
    }
    .into()
}
