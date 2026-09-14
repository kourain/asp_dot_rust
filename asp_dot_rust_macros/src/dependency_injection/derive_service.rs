use crate::utils::{compiler_error::create_compiler_error, extract_type::extract_wrapper_inner};
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// `#[derive(DependencyInjectableService)]` — for the common case where every
/// dependency a service needs is already a struct field typed `Serv<T>`,
/// `Cfg<T>`, `CfgRequire<T>`, or `CfgReload<T>`. It builds `Self { .. }`
/// directly from those fields, so no `fn new` / `#[inject]` is
/// needed at all.
///
/// Any field that is NOT one of the four wrapper types falls back to
/// `Default::default()` (a hard compile error if the field's type does not
/// implement `Default`), and emits a `deprecated`-style compiler **warning**
/// at the field's call site so the fallback is never silent. Prefer
/// `#[inject]` on a hand-written `fn new` instead if the service needs real
/// custom construction logic (computed fields, a dependency the constructor
/// needs but doesn't store, etc).
pub(crate) fn derive_injectable_service(item: TokenStream) -> TokenStream {
    let main_crate_path = crate::utils::find_crate::asp_dot_rust_crate_path();
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let ident_str = ident.to_string();

    let data = match &input.data {
        Data::Struct(s) => s,
        _ => {
            return create_compiler_error(&input, "#[derive(DependencyInjectableService)] only supports structs")
        }
    };

    let named = match &data.fields {
        Fields::Named(f) => &f.named,
        Fields::Unit => {
            // no fields at all -- nothing to resolve, no dependency edges
            return quote! {
                impl #main_crate_path::dependency_injection::DependencyInjectableService for #ident {
                    fn inject(_service_scope: &#main_crate_path::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
                        Self
                    }
                }
                #main_crate_path::dependency_injection::inventory::submit! {
                    #main_crate_path::dependency_injection::DependencyEdge {
                        owner: std::any::TypeId::of::<#ident>,
                        owner_name: #ident_str,
                        dependencies: || vec![],
                    }
                }
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
    let mut inner_types = Vec::new();
    let mut configuration_service = quote! {};

    for (field_index, field) in named.iter().enumerate() {
        let field_ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        if let Some(inner) = extract_wrapper_inner(ty, "Serv") {
            field_inits.push(quote! { #field_ident: #main_crate_path::dependency_injection::Serv(service_scope.get_service::<#inner>()) });
            inner_types.push(inner);
        } else if let Some(inner) = extract_wrapper_inner(ty, "CfgRequire") {
            field_inits.push(quote! { #field_ident: #main_crate_path::dependency_injection::CfgRequire(configuration_service.require::<#inner>()) });
            inner_types.push(inner.clone());
            configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
        } else if let Some(inner) = extract_wrapper_inner(ty, "CfgReload") {
            let missing_msg = format!("CfgReload<{}> was never registered via configure_reload::<{}>(...)", quote!(#inner), quote!(#inner));
            field_inits.push(quote! { #field_ident: #main_crate_path::dependency_injection::CfgReload(configuration_service.get_reload::<#inner>().expect(#missing_msg)) });
            inner_types.push(inner.clone());
            configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
        } else if let Some(inner) = extract_wrapper_inner(ty, "Cfg") {
            field_inits.push(quote! { #field_ident: #main_crate_path::dependency_injection::Cfg(configuration_service.get::<#inner>()) });
            inner_types.push(inner.clone());
            configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
        } else {
            field_inits.push(quote! { #field_ident: Default::default() });

            // Not a compile error: emit a `deprecated`-style compiler
            // *warning* instead, so silently defaulting a field is never
            // truly silent. The nested fn only exists to be `#[deprecated]`;
            // calling it does nothing at runtime.
            let marker_name = syn::Ident::new(&format!("__di_default_field_marker_{}_{}", ident_str, field_index), field_ident.span());
            let warning_msg = format!(
                "field `{}` on `{}` is not Serv<T>, Cfg<T>, CfgRequire<T>, or CfgReload<T>; #[derive(DependencyInjectableService)] defaulted it via `Default::default()`. \
                 Use #[inject] on a hand-written `fn new` instead if this field needs real construction.",
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

    quote! {
        impl #main_crate_path::dependency_injection::DependencyInjectableService for #ident {
            fn inject(service_scope: &#main_crate_path::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
                #configuration_service
                #(#field_warnings)*
                Self { #(#field_inits),* }
            }
        }

        #main_crate_path::dependency_injection::inventory::submit! {
            #main_crate_path::dependency_injection::DependencyEdge {
                owner: std::any::TypeId::of::<#ident>,
                owner_name: #ident_str,
                dependencies: || vec![ #( (std::any::TypeId::of::<#inner_types>(), std::any::type_name::<#inner_types>()) ),* ],
            }
        }
    }
    .into()
}
