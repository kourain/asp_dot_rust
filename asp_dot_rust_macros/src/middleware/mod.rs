use crate::utils::{
    compiler_error::{create_compiler_error, token_type_to_string},
    extract_type::extract_wrapper_inner,
    find_crate::asp_dot_rust_crate_path,
};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{FnArg, ImplItem, ItemImpl, Pat, PatType, Type, parse_macro_input};

/// A service parameter: the expression that creates it, its inner type and whether it needs the configuration service.
struct Resolved {
    expr: TokenStream2,
    inner: Type,
    needs_configuration: bool,
}

fn resolve_param(ty: &Type, main_crate_path: &TokenStream2) -> Option<Resolved> {
    let di = quote!(#main_crate_path::dependency_injection);
    if let Some(inner) = extract_wrapper_inner(ty, "Serv") {
        return Some(Resolved {
            expr: quote! { #di::Serv(service_scope.get_service::<#inner>()) },
            inner: inner.clone(),
            needs_configuration: false,
        });
    }
    if let Some(inner) = extract_wrapper_inner(ty, "CfgRequire") {
        return Some(Resolved {
            expr: quote! { #di::CfgRequire(configuration_service.require::<#inner>()) },
            inner: inner.clone(),
            needs_configuration: true,
        });
    }
    if let Some(inner) = extract_wrapper_inner(ty, "CfgReload") {
        let missing_msg = format!("CfgReload<{}> was never registered via configure_reload::<{}>(...)", quote!(#inner), quote!(#inner));
        return Some(Resolved {
            expr: quote! { #di::CfgReload(configuration_service.get_reload::<#inner>().expect(#missing_msg)) },
            inner: inner.clone(),
            needs_configuration: true,
        });
    }
    if let Some(inner) = extract_wrapper_inner(ty, "Cfg") {
        return Some(Resolved {
            expr: quote! { #di::Cfg(configuration_service.get::<#inner>()) },
            inner: inner.clone(),
            needs_configuration: true,
        });
    }
    None
}

/// `MiddlewareNext`: the next middleware of the chain, it is not a service of the container.
fn is_middleware_next(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.path.segments.last().is_some_and(|segment| segment.ident == "MiddlewareNext"))
}

pub(crate) fn middleware(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return create_compiler_error(&TokenStream2::from(args), "`#[middleware]` does not take any arguments");
    }
    let main_crate_path = asp_dot_rust_crate_path();
    let input = parse_macro_input!(item as ItemImpl);
    if input.trait_.is_some() {
        return create_compiler_error(
            &input.self_ty,
            "`#[middleware]` must be placed on `impl T { fn new(..); async fn invoke_async(..) }`, not on a trait impl",
        );
    }
    let self_ty = &input.self_ty;
    let self_ty_str = quote!(#self_ty).to_string();
    let find_fn = |name: &str| {
        input.items.iter().find_map(|item| match item {
            ImplItem::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
    };
    let Some(invoke_fn) = find_fn("invoke_async") else {
        return create_compiler_error(self_ty, "impl block must have fn `invoke_async`");
    };
    let new_fn = find_fn("new");
    let mw = quote!(#main_crate_path::middleware);
    let scope_ty = quote!(#main_crate_path::services::service_provider::service_provider_scope::ServiceProviderScope);
    let config_service = quote! { let configuration_service = service_scope.get_service::<#main_crate_path::services::configuration::ConfigurationService>(); };

    // ---- startup: `new` parameters, resolved once when the pipeline is built
    let mut startup_args: Vec<TokenStream2> = Vec::new();
    let mut inner_types: Vec<Type> = Vec::new();
    let mut startup_config = quote! {};
    let construct = match new_fn {
        Some(new_fn) => {
            if new_fn.sig.asyncness.is_some() {
                return create_compiler_error(new_fn, "fn `new` cannot be async");
            }
            let mut has_next = false;
            for arg in &new_fn.sig.inputs {
                let FnArg::Typed(PatType { ty, .. }) = arg else { continue };
                if is_middleware_next(ty) {
                    startup_args.push(quote! { next });
                    has_next = true;
                } else if let Some(resolved) = resolve_param(ty, &main_crate_path) {
                    startup_args.push(resolved.expr);
                    inner_types.push(resolved.inner);
                    if resolved.needs_configuration {
                        startup_config = config_service.clone();
                    }
                } else {
                    return create_compiler_error(
                        ty,
                        format!("Parameter must be MiddlewareNext, Serv<T>, Cfg<T>, CfgRequire<T> or CfgReload<T>, found: {}", token_type_to_string(ty)),
                    );
                }
            }
            if !has_next {
                return create_compiler_error(new_fn, "fn `new` must take `next: MiddlewareNext`");
            }
            quote! { Self::new( #(#startup_args),* ) }
        }
        // no `new`: the struct has only the field `next`
        None => quote! { Self { next } },
    };

    // ---- per request: the parameters of `invoke_async` after `ctx`, resolved from the request scope
    let mut inputs = invoke_fn.sig.inputs.iter();
    if !matches!(inputs.next(), Some(FnArg::Receiver(_))) {
        return create_compiler_error(&invoke_fn.sig, "fn `invoke_async` must take `&self`");
    }
    let ctx = match inputs.next() {
        Some(FnArg::Typed(PatType { pat, .. })) => match &**pat {
            Pat::Ident(pat_ident) => pat_ident.ident.clone(),
            other => return create_compiler_error(other, "the parameter must be a plain identifier"),
        },
        _ => return create_compiler_error(&invoke_fn.sig, "fn `invoke_async` must take `http_context: &mut HttpContext`"),
    };
    let mut request_lets: Vec<TokenStream2> = Vec::new();
    let mut request_names: Vec<syn::Ident> = Vec::new();
    let mut request_config = quote! {};
    for (index, arg) in inputs.enumerate() {
        let FnArg::Typed(PatType { ty, .. }) = arg else { continue };
        let Some(resolved) = resolve_param(ty, &main_crate_path) else {
            return create_compiler_error(
                ty,
                format!(
                    "Parameter of `invoke_async` must be Serv<T>, Cfg<T>, CfgRequire<T> or CfgReload<T>, found: {}",
                    token_type_to_string(ty)
                ),
            );
        };
        let name = syn::Ident::new(&format!("__request_arg{index}"), proc_macro2::Span::call_site());
        let expr = resolved.expr;
        request_lets.push(quote! { let #name = #expr; });
        request_names.push(name);
        if resolved.needs_configuration {
            request_config = config_service.clone();
        }
    }

    quote! {
        #input
        #[#main_crate_path::__async_trait::async_trait]
        impl #mw::Middleware for #self_ty {
            fn inject(service_scope: &#scope_ty, next: #mw::MiddlewareNext) -> Self {
                #startup_config
                #construct
            }
            async fn inject_invoke_async(&self, #ctx: &mut #main_crate_path::http_context::HttpContext) {
                let service_scope = &#ctx.service_provider;
                #request_config
                #(#request_lets)*
                self.invoke_async(#ctx, #(#request_names),*).await;
            }
        }

        // Register dependency edges to check for cycles at build time
        #main_crate_path::dependency_injection::inventory::submit! {
            #main_crate_path::dependency_injection::DependencyEdge {
                owner: std::any::TypeId::of::<#self_ty>,
                owner_name: #self_ty_str,
                dependencies: || vec![ #( (std::any::TypeId::of::<#inner_types>(), std::any::type_name::<#inner_types>()) ),* ],
            }
        }
    }
    .into()
}
