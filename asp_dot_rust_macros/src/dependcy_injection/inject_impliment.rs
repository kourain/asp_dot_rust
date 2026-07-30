use crate::{
    dependcy_injection::flags::InjectFlags,
    utils::compiler_error::{create_compiler_error, token_to_string, token_type_to_string},
};
use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ImplItem, ItemImpl, PatType, Type, parse_macro_input};

pub(crate) fn inject(_args: TokenStream, item: TokenStream, flag: InjectFlags) -> TokenStream {
    let main_crate_path = crate::utils::find_crate::asp_dot_rust_crate_path();
    let input = parse_macro_input!(item as ItemImpl);
    let self_ty = &input.self_ty; // MyService
    let self_ty_str = quote!(#self_ty).to_string();

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
    if let syn::ReturnType::Type(_, ty) = &new_fn.sig.output
        && let Type::Path(type_path) = &**ty
        && (type_path.path.segments.last().unwrap().ident == "Self" || type_path.path.segments.last().unwrap().ident == self_ty_str)
    {
        //ok
    } else {
        return create_compiler_error(new_fn, format!("fn `new` must return Self or {}", self_ty_str));
    }

    // extract the inner type from each Arc<T> parameter
    let mut inner_types = Vec::new();
    let mut call_args = Vec::new();
    let mut configuration_service = quote! {};
    let mut is_http_context_ref_injected = false;
    for arg in &new_fn.sig.inputs {
        if let FnArg::Typed(PatType { ty, .. }) = arg {
            if flag.contains(InjectFlags::SERVICE)
                && let Some(inner) = extract_arc_inner(ty)
            {
                let arg = quote! { service_scope.get_service::<#inner>() };
                call_args.push(arg);
                inner_types.push(inner.clone());
            } else if flag.contains(InjectFlags::CONFIG)
                && let Some(inner) = extract_option_arc_inner(ty)
            {
                let arg = quote! { configuration_service.get::<#inner>() };
                call_args.push(arg);
                inner_types.push(inner.clone());
                configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
            } else if flag.contains(InjectFlags::INJECT_CONTROLLER) {
                if is_http_context_ref(ty) {
                    if is_http_context_ref_injected {
                        return create_compiler_error(new_fn, "Can't inject HttpContextRef more than once");
                    } else {
                        call_args.push(quote! { http_context });
                        is_http_context_ref_injected = true;
                    }
                }
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

    let expanded;
    if flag.contains(InjectFlags::INJECT_CONTROLLER) {
        expanded = quote! {
            #input
            impl #main_crate_path::dependcy_injection::DependcyInjectableController for #self_ty {
                fn inject(
                    http_context: #main_crate_path::http_context::HttpContextRef,
                ) -> Self {
                    let service_scope: &#main_crate_path::services::service_provider::service_provider_scope::ServiceProviderScope = &http_context.service_provider;
                    #configuration_service
                    Self::new( #(#call_args),* )
                }
            }
        }
    } else {
        expanded = quote! {
            #input
            impl #main_crate_path::dependcy_injection::DependcyInjectableService for #self_ty {
                fn inject(
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
                    owner_name: #self_ty_str,
                    dependencies: || vec![ #( (std::any::TypeId::of::<#inner_types>(), std::any::type_name::<#inner_types>()) ),* ],
                }
            }
        };
    }
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

fn is_http_context_ref(ty: &Type) -> bool {
    if let Type::Path(p) = ty {
        let seg = p.path.segments.last().unwrap();
        eprint!("{:?}", token_to_string(&p));
        if seg.ident == "HttpContextRef" {
            return true;
        }
        // else if seg.ident == "HttpContext" {
        // HttpContextStruct is private inside main crate
        // }
    }
    false
}
