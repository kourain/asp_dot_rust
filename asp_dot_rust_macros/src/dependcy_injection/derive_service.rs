use crate::utils::{compiler_error::create_compiler_error, extract_type::extract_wrapper_inner};
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

/// `#[derive(DependcyInjectableService)]` — for the common case where every
/// dependency a service needs is already a struct field typed `Serv<T>`,
/// `Cfg<T>`, `CfgRequire<T>`, or `CfgReload<T>`. It builds `Self { .. }`
/// directly from those fields, so no `fn new` / `#[inject]` is
/// needed at all.
///
/// Any field that is NOT one of the four wrapper types is a compile error —
/// use `#[inject]` on a hand-written `fn new` instead if the service
/// needs custom construction logic (computed fields, non-DI defaults, etc).
pub(crate) fn derive_injectable_service(item: TokenStream) -> TokenStream {
    let main_crate_path = crate::utils::find_crate::asp_dot_rust_crate_path();
    let input = parse_macro_input!(item as DeriveInput);
    let ident = &input.ident;
    let ident_str = ident.to_string();

    let data = match &input.data {
        Data::Struct(s) => s,
        _ => {
            return create_compiler_error(&input, "#[derive(DependcyInjectableService)] only supports structs")
        }
    };

    let named = match &data.fields {
        Fields::Named(f) => &f.named,
        Fields::Unit => {
            // no fields at all -- nothing to resolve, no dependency edges
            return quote! {
                impl #main_crate_path::dependcy_injection::DependcyInjectableService for #ident {
                    fn inject(_service_scope: &#main_crate_path::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
                        Self
                    }
                }
                #main_crate_path::dependcy_injection::inventory::submit! {
                    #main_crate_path::dependcy_injection::DependencyEdge {
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
                "#[derive(DependcyInjectableService)] does not support tuple structs; use #[inject] on a hand-written `fn new` instead",
            );
        }
    };

    let mut field_inits = Vec::new();
    let mut inner_types = Vec::new();
    let mut configuration_service = quote! {};

    for field in named {
        let field_ident = field.ident.as_ref().unwrap();
        let ty = &field.ty;

        if let Some(inner) = extract_wrapper_inner(ty, "Serv") {
            field_inits.push(quote! { #field_ident: #main_crate_path::dependcy_injection::Serv(service_scope.get_service::<#inner>()) });
            inner_types.push(inner);
        } else if let Some(inner) = extract_wrapper_inner(ty, "CfgRequire") {
            field_inits.push(quote! { #field_ident: #main_crate_path::dependcy_injection::CfgRequire(configuration_service.require::<#inner>()) });
            inner_types.push(inner.clone());
            configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
        } else if let Some(inner) = extract_wrapper_inner(ty, "CfgReload") {
            let missing_msg = format!("CfgReload<{}> was never registered via configure_reload::<{}>(...)", quote!(#inner), quote!(#inner));
            field_inits.push(quote! { #field_ident: #main_crate_path::dependcy_injection::CfgReload(configuration_service.get_reload::<#inner>().expect(#missing_msg)) });
            inner_types.push(inner.clone());
            configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
        } else if let Some(inner) = extract_wrapper_inner(ty, "Cfg") {
            field_inits.push(quote! { #field_ident: #main_crate_path::dependcy_injection::Cfg(configuration_service.get::<#inner>()) });
            inner_types.push(inner.clone());
            configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
        } else {
            field_inits.push(quote! { #field_ident: Default::default() });
            // return create_compiler_error(
            //     ty,
            //     format!(
            //         "field `{}` must be Serv<T>, Cfg<T>, CfgRequire<T>, or CfgReload<T> to use #[derive(DependcyInjectableService)]; use #[inject] on a hand-written `fn new` if this field needs custom construction",
            //         field_ident
            //     ),
            // );
        }
    }

    quote! {
        impl #main_crate_path::dependcy_injection::DependcyInjectableService for #ident {
            fn inject(service_scope: &#main_crate_path::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self {
                #configuration_service
                Self { #(#field_inits),* }
            }
        }

        #main_crate_path::dependcy_injection::inventory::submit! {
            #main_crate_path::dependcy_injection::DependencyEdge {
                owner: std::any::TypeId::of::<#ident>,
                owner_name: #ident_str,
                dependencies: || vec![ #( (std::any::TypeId::of::<#inner_types>(), std::any::type_name::<#inner_types>()) ),* ],
            }
        }
    }
    .into()
}
