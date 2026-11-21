use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Data, DeriveInput, Fields, parse_macro_input};

use super::{dependencies::Dependencies, derive_service::extract_di_options, flags::DIPropOption};
use crate::utils::{compiler_error::create_compiler_error, extract_type::type_name, find_crate::asp_dot_rust_crate_path};

pub(crate) fn derive_injectable_controller(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let Data::Struct(data) = &input.data else {
        return create_compiler_error(&input, "#[derive(DependencyInjectableController)] only supports structs");
    };

    let mut ignore_missing_http_context = false;
    for attr in input.attrs.iter().filter(|attr| attr.path().is_ident("di")) {
        if let Err(error) = attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("ignore_missing_http_context") {
                ignore_missing_http_context = true;
                Ok(())
            } else {
                Err(meta.error("expected `ignore_missing_http_context`"))
            }
        }) {
            return error.into_compile_error().into();
        }
    }

    let mut dependencies = Dependencies::default();
    let mut has_http_context = false;
    let initializer = match &data.fields {
        Fields::Named(fields) => {
            let mut field_inits = Vec::new();
            for field in &fields.named {
                let name = &field.ident;
                let ty = &field.ty;
                let value = if type_name(ty).is_some_and(|ident| ident == "HttpContextRef") {
                    if has_http_context {
                        return create_compiler_error(ty, "`HttpContextRef` can only be injected once");
                    }
                    has_http_context = true;
                    quote! { http_context }
                } else if let Some(expression) = dependencies.resolve(ty) {
                    expression
                } else if extract_di_options(field).contains(DIPropOption::DEFAULT) {
                    quote! { ::core::default::Default::default() }
                } else {
                    return create_compiler_error(
                        ty,
                        "expected HttpContextRef, Serv<T>, CfgOption<T>, CfgRequire<T> or CfgReload<T>; use #[di(default)] or #[controller_inject] with a custom constructor",
                    );
                };
                field_inits.push(quote! { #name: #value });
            }
            quote! { Self { #(#field_inits),* } }
        }
        Fields::Unit => quote! { Self },
        Fields::Unnamed(_) => {
            return create_compiler_error(&data.fields, "#[derive(DependencyInjectableController)] does not support tuple structs");
        }
    };

    let name = &input.ident;
    let warning = if !has_http_context && !ignore_missing_http_context {
        let note = format!("controller `{name}` has no `HttpContextRef` field; add #[di(ignore_missing_http_context)] to the struct if intentional");
        quote_spanned! {name.span()=>
            #[deprecated(note = #note)]
            fn __di_missing_http_context() {}
            __di_missing_http_context();
        }
    } else {
        quote! {}
    };
    let main_crate = asp_dot_rust_crate_path();
    let configuration = dependencies.configuration_binding(&main_crate);
    let registration_check = dependencies.check_registrations(&quote! { Self });
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        #[#main_crate::third_party::async_trait::async_trait]
        impl #impl_generics #main_crate::dependency_injection::DependencyInjectableController for #name #ty_generics #where_clause {
            async fn inject(http_context: #main_crate::http_context::HttpContextRef) -> Self {
                #warning
                let service_scope = &http_context.service_provider;
                #configuration
                #registration_check
                #initializer
            }
        }
    }
    .into()
}
