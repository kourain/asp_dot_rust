use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

use super::{dependencies::Dependencies, derive_service::extract_di_options, flags::DIPropOption};
use crate::utils::{compiler_error::create_compiler_error, extract_type::type_name, find_crate::asp_dot_rust_crate_path};

pub(crate) fn derive_injectable_middleware(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let Data::Struct(data) = &input.data else {
        return create_compiler_error(&input, "#[derive(DependencyInjectableMiddleware)] only supports structs with named fields");
    };
    let Fields::Named(fields) = &data.fields else {
        return create_compiler_error(&input, "#[derive(DependencyInjectableMiddleware)] only supports structs with named fields");
    };

    let mut dependencies = Dependencies::default();
    let mut field_inits = Vec::new();
    let mut has_next = false;
    for field in &fields.named {
        let name = &field.ident;
        let ty = &field.ty;
        let value = if type_name(ty).is_some_and(|ident| ident == "MiddlewareNext") {
            if has_next {
                return create_compiler_error(ty, "`MiddlewareNext` can only be injected once");
            }
            has_next = true;
            quote! { next }
        } else if let Some(expression) = dependencies.resolve(ty) {
            expression
        } else if extract_di_options(field).contains(DIPropOption::DEFAULT) {
            quote! { ::core::default::Default::default() }
        } else {
            return create_compiler_error(
                ty,
                "expected MiddlewareNext, Serv<T>, CfgOption<T>, CfgRequire<T> or CfgReload<T>; use #[di(default)] for a default field or define `fn new` for custom construction",
            );
        };
        field_inits.push(quote! { #name: #value });
    }
    if !has_next {
        return create_compiler_error(&input.ident, "#[derive(DependencyInjectableMiddleware)] requires a `MiddlewareNext` field");
    }

    let main_crate = asp_dot_rust_crate_path();
    let configuration = dependencies.configuration_binding(&main_crate);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        #[#main_crate::third_party::async_trait::async_trait]
        impl #impl_generics #main_crate::dependency_injection::DependencyInjectableMiddleware for #name #ty_generics #where_clause {
            async fn inject(
                service_scope: &#main_crate::services::service_provider::ServiceProviderScope,
                next: #main_crate::middleware::MiddlewareNext,
            ) -> Self {
                #configuration
                Self { #(#field_inits),* }
            }
        }
    }
    .into()
}
