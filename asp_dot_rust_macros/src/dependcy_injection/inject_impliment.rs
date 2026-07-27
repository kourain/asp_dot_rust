use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ImplItem, ItemImpl, PatType, Type, parse_macro_input};

use crate::utils::compiler_error::{create_compiler_error, token_type_to_string};

pub(crate) fn injectable_service(_args: TokenStream, item: TokenStream) -> TokenStream {
    let main_crate_path = crate::utils::find_crate::asp_dot_rust_crate_path();
    let input = parse_macro_input!(item as ItemImpl);
    let self_ty = &input.self_ty; // MyService

    // find the `new` function in the impl block
    let new_fn = input
        .items
        .iter()
        .find_map(|item| {
            if let ImplItem::Fn(f) = item {
                if f.sig.ident == "new" {
                    return Some(f);
                }
            }
            None
        })
        .expect("impl block must have fn `new`");

    //check new is async or not
    if new_fn.sig.asyncness.is_some() {
        return syn::Error::new_spanned(new_fn, "fn `new` cannot be async").to_compile_error().into();
    }

    //check new is return type is Self or not
    if let syn::ReturnType::Type(_, ty) = &new_fn.sig.output {
        if let Type::Path(type_path) = &**ty {
            if type_path.path.segments.last().unwrap().ident != "Self" {
                return create_compiler_error(new_fn, "fn `new` must return Self");
            }
        } else {
            return create_compiler_error(new_fn, "fn `new` must return Self");
        }
    } else {
        return create_compiler_error(new_fn, "fn `new` must return Self");
    }

    // extract the inner type from each Arc<T> parameter
    let mut inner_types = Vec::new();
    let mut call_args = Vec::new();
    let mut configuration_service = quote! {};
    for arg in &new_fn.sig.inputs {
        if let FnArg::Typed(PatType { ty, .. }) = arg {
            if let Some(inner) = extract_arc_inner(ty) {
                call_args.push(quote! { service_scope.get_service::<#inner>() });
                inner_types.push(inner.clone());
            } else if let Some(inner) = extract_option_arc_inner(ty) {
                call_args.push(quote! { configuration_service.get::<#inner>() });
                inner_types.push(inner.clone());
                configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
            } else {
                return syn::Error::new_spanned(
                    ty,
                    format!(
                        "Field must be std::sync::Arc<T> (Service) or Option<std::sync::Arc<T>> (Configuration), found: {}",
                        token_type_to_string(ty)
                    ),
                )
                .into_compile_error()
                .into();
            }
        }
    }

    let ty_name_str = quote!(#self_ty).to_string();
    let expanded = quote! {
        #input // keep the original impl block

        impl #main_crate_path::dependcy_injection::DependcyInjectableService for #self_ty {
            fn inject_service(
                service_scope: &#main_crate_path::services::service_provider::service_provider_scope::ServiceProviderScope
            ) -> Self {
                #configuration_service
                Self::new( #(#call_args),* )
            }
        }

        // Register dependency edges to check for cycles at build time
        #main_crate_path::dependcy_injection::inventory::submit! {
            #main_crate_path::dependcy_injection::DependencyEdge {
                owner: std::any::TypeId::of::<#self_ty>,
                owner_name: #ty_name_str,
                dependencies: || vec![ #( (std::any::TypeId::of::<#inner_types>(), std::any::type_name::<#inner_types>()) ),* ],
            }
        }
    };

    expanded.into()
}

/// Extract the inner type from Arc<T>
fn extract_arc_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(p) = ty {
        let seg = p.path.segments.last()?;
        if seg.ident == "Arc" {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner.clone());
                }
            }
        }
    }
    None
}

/// Extract the inner type from Option<Arc<T>>
fn extract_option_arc_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(p) = ty {
        let seg = p.path.segments.last()?;
        if seg.ident == "Option" {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return extract_arc_inner(inner);
                }
            }
        }
    }
    None
}
