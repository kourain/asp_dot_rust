use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ImplItem, ItemImpl, PatType, Type, parse_macro_input};

pub(crate) fn injectable_service(_args: TokenStream, item: TokenStream) -> TokenStream {
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
        .expect("#[injectable_service] impl block must have fn `new`");

    //check new is async or not
    if new_fn.sig.asyncness.is_some() {
        panic!("#[injectable_service] fn `new` cannot be async");
    }

    //check new is return type is Self or not
    if let syn::ReturnType::Type(_, ty) = &new_fn.sig.output {
        if let Type::Path(type_path) = &**ty {
            if type_path.path.segments.last().unwrap().ident != "Self" {
                panic!("#[injectable_service] fn `new` must return Self");
            }
        } else {
            panic!("#[injectable_service] fn `new` must return Self");
        }
    } else {
        panic!("#[injectable_service] fn `new` must return Self");
    }

    // extract the inner type from each Arc<T> parameter
    let mut inner_types = Vec::new();
    let mut call_args = Vec::new();

    for arg in &new_fn.sig.inputs {
        if let FnArg::Typed(PatType { ty, .. }) = arg {
            if let Some(inner) = extract_arc_inner(ty) {
                call_args.push(quote! { service_scope.get_service::<#inner>() });
                inner_types.push(inner.clone());
            } else if let Some(inner) = extract_option_arc_inner(ty) {
                call_args.push(quote! { configuration_service.get::<#inner>() });
                inner_types.push(inner.clone());
            } else {
                panic!("#[injectable_service] constructor arg must be Arc<T> for Service or Option<Arc<T>> for Configuration");
            }
        }
    }

    let ty_name_str = quote!(#self_ty).to_string();
    let crate_path = crate::utils::find_crate::asp_dot_rust_path();
    let expanded = quote! {
        #input // keep the original impl block

        impl #crate_path::dependcy_injection::DependcyInjectableService for #self_ty {
            fn inject_service(
                service_scope: &#crate_path::services::service_provider::service_provider_scope::ServiceProviderScope
            ) -> Self {
                let configuration_service = service_scope.get_service::<#crate_path::services::configuration::ConfigurationService>();
                Self::new( #(#call_args),* )
            }
        }

        // Register dependency edges to check for cycles at build time
        #[cfg(debug_assertions)]
        #crate_path::dependcy_injection::inventory::submit! {
            #crate_path::dependcy_injection::DependencyEdge {
                owner: std::any::TypeId::of::<#self_ty>,
                owner_name: #ty_name_str,
                dependencies: || vec![ #( (std::any::TypeId::of::<#inner_types>(), std::any::type_name::<#inner_types>()) ),* ],
            }
        }
    };

    expanded.into()
}

/// Extract the inner type from Arc<T> ; support adding Box<T>, Option<Arc<T>> if needed
fn extract_arc_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(p) = ty {
        let seg = p.path.segments.last()?;
        if seg.ident == "Arc" {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner.clone());
                }
            }
        } else {
            eprintln!("Warning: constructor type: {:?}", p.path.segments.last().unwrap().ident);
        }
    }
    None
}
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
