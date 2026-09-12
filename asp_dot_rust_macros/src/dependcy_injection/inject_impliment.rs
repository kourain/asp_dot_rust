use crate::{
    dependcy_injection::flags::{HttpInjectType, InjectFlags},
    utils::compiler_error::{create_compiler_error, token_type_to_string},
    utils::extract_type::get_exact_type,
};
use proc_macro::TokenStream;
use quote::quote;
use syn::{FnArg, ImplItem, ItemImpl, PatType, Type, parse_macro_input};

pub(crate) fn inject(_args: TokenStream, item: TokenStream, flag: InjectFlags) -> TokenStream {
    let main_crate_path = crate::utils::find_crate::asp_dot_rust_crate_path();
    let input = parse_macro_input!(item as ItemImpl);
    let self_ty = &input.self_ty; // MyService | &MyService | &mut MyService | MyService<'a>
    let real_self_ty = get_exact_type(self_ty);
    let self_ty_str = quote!(#real_self_ty).to_string();

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
    let mut http_context_inject = quote! {};
    let mut configuration_service = quote! {};
    let mut httpcontext_inject_state = HttpInjectType::None;
    for arg in &new_fn.sig.inputs {
        if let FnArg::Typed(PatType { ty, .. }) = arg {
            if flag.contains(InjectFlags::SERVICE)
                && let Some(inner) = extract_wrapper_inner(ty, "Serv")
            {
                let arg = quote! { #main_crate_path::dependcy_injection::Serv(service_scope.get_service::<#inner>()) };
                call_args.push(arg);
                inner_types.push(inner.clone());
            } else if flag.contains(InjectFlags::CONFIG)
                && let Some(inner) = extract_wrapper_inner(ty, "CfgRequire")
            {
                let arg = quote! { #main_crate_path::dependcy_injection::CfgRequire(configuration_service.require::<#inner>()) };
                call_args.push(arg);
                inner_types.push(inner.clone());
                configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
            } else if flag.contains(InjectFlags::CONFIG)
                && let Some(inner) = extract_wrapper_inner(ty, "CfgReload")
            {
                let missing_msg = format!("CfgReload<{}> was never registered via configure_reload::<{}>(...)", quote!(#inner), quote!(#inner));
                let arg = quote! { #main_crate_path::dependcy_injection::CfgReload(configuration_service.get_reload::<#inner>().expect(#missing_msg)) };
                call_args.push(arg);
                inner_types.push(inner.clone());
                configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
            } else if flag.contains(InjectFlags::CONFIG)
                && let Some(inner) = extract_wrapper_inner(ty, "Cfg")
            {
                let arg = quote! { #main_crate_path::dependcy_injection::Cfg(configuration_service.get::<#inner>()) };
                call_args.push(arg);
                inner_types.push(inner.clone());
                configuration_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };
            } else if flag.contains(InjectFlags::INJECT_CONTROLLER) {
                let http_inject_type = get_http_inject_type(ty);
                if http_inject_type != HttpInjectType::None {
                    if httpcontext_inject_state != HttpInjectType::None {
                        return create_compiler_error(new_fn, "Can't inject HttpContextRef more than once");
                    }
                    httpcontext_inject_state = http_inject_type;
                    match httpcontext_inject_state {
                        HttpInjectType::ShareMutPtr => {
                            http_context_inject = quote! { let http_ctx_ref = #main_crate_path::utils::ShareMutPtr::new_with_state(http_context, is_valid); };
                            call_args.push(quote! { http_ctx_ref });
                        }
                        HttpInjectType::BorrowHttpContext | HttpInjectType::MoveHttpContext | _ => return create_compiler_error(new_fn, "Can't inject HttpContext, use HttpContextRef instead"),
                    }
                }
            } else {
                return syn::Error::new_spanned(
                    ty,
                    format!(
                        "Field must be Serv<T> (Service), Cfg<T> (optional config), CfgRequire<T> (required config), or CfgReload<T> (reloadable config), found: {}",
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
        if httpcontext_inject_state == HttpInjectType::None {
            return create_compiler_error(new_fn, "`new` fn args must contain HttpContextRef");
        }
        expanded = quote! {
            #input
            impl #main_crate_path::dependcy_injection::DependcyInjectableController for #self_ty {
                fn inject(
                    http_context: &mut #main_crate_path::http_context::HttpContext,
                    is_valid: std::sync::Arc<std::sync::atomic::AtomicBool>
                ) -> Self {
                    let service_scope: &#main_crate_path::services::service_provider::service_provider_scope::ServiceProviderScope = &http_context.service_provider;
                    #http_context_inject
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
                    #http_context_inject
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

/// Extract inner type T from a single-generic-arg wrapper like `Serv<T>`,
/// `Cfg<T>`, `CfgRequire<T>`, `CfgReload<T>` — matched by ident name only,
/// so it works regardless of which crate path the wrapper was imported from.
fn extract_wrapper_inner(ty: &Type, wrapper_name: &str) -> Option<Type> {
    if let Type::Path(p) = ty {
        let seg = p.path.segments.last()?;
        if seg.ident == wrapper_name {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner.clone());
                }
            }
        }
    }
    None
}

fn get_http_inject_type(ty: &Type) -> HttpInjectType {
    if let Type::Path(p) = ty {
        let seg = p.path.segments.last().unwrap();
        if seg.ident == "HttpContextRef" {
            return HttpInjectType::ShareMutPtr;
        } else if seg.ident == "HttpContext" {
            return HttpInjectType::MoveHttpContext;
        }
    } else if let Type::Reference(r) = ty {
        let child_type = get_http_inject_type(&r.elem);
        if child_type == HttpInjectType::MoveHttpContext {
            return HttpInjectType::BorrowHttpContext;
        }
    }
    HttpInjectType::None
}
