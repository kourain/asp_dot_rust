use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{FnArg, ImplItem, ImplItemFn, ItemImpl, Signature, parse_macro_input};

use super::dependencies::Dependencies;
use crate::utils::{
    compiler_error::{create_compiler_error, token_type_to_string},
    extract_type::{get_exact_type, type_name},
    find_crate::asp_dot_rust_crate_path,
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum InjectTarget {
    Service,
    Controller,
    Middleware,
}

#[derive(Default)]
struct InjectedArguments {
    dependencies: Dependencies,
    bindings: Vec<TokenStream2>,
    values: Vec<TokenStream2>,
}

impl InjectedArguments {
    fn resolve<'a>(signature: &Signature, inputs: impl Iterator<Item = &'a FnArg>, extra: Option<(&str, TokenStream2)>) -> Result<Self, TokenStream> {
        let mut arguments = Self::default();
        let mut has_extra = false;
        for (index, input) in inputs.enumerate() {
            let FnArg::Typed(parameter) = input else {
                return Err(create_compiler_error(input, "fn `new` must be an associated function"));
            };
            let ty = &parameter.ty;
            if let Some((name, value)) = &extra
                && type_name(ty).is_some_and(|ident| ident == *name)
            {
                if has_extra {
                    return Err(create_compiler_error(ty, format!("`{name}` can only be injected once")));
                }
                has_extra = true;
                arguments.values.push(value.clone());
            } else if let Some(expression) = arguments.dependencies.resolve(ty) {
                let name = format_ident!("__di_arg{index}", span = Span::mixed_site());
                arguments.bindings.push(quote! { let #name = #expression; });
                arguments.values.push(quote! { #name });
            } else {
                if extra.as_ref().is_some_and(|(name, _)| *name == "HttpContextRef") && type_name(get_exact_type(ty)).is_some_and(|ident| ident == "HttpContext") {
                    return Err(create_compiler_error(ty, "Can't inject HttpContext, use HttpContextRef instead"));
                }
                let allowed_extra = extra.as_ref().map_or(String::new(), |(name, _)| format!("{name}, "));
                let found = token_type_to_string(ty);
                return Err(create_compiler_error(
                    ty,
                    format!(
                        "Parameter of `{}` must be {allowed_extra}Serv<T>, CfgOption<T>, CfgRequire<T> or CfgReload<T>, found: {found}",
                        signature.ident
                    ),
                ));
            }
        }
        if let Some((name, _)) = extra
            && !has_extra
        {
            return Err(create_compiler_error(signature, format!("fn `{}` must take `{name}`", signature.ident)));
        }
        Ok(arguments)
    }
}

pub(crate) fn inject(args: TokenStream, item: TokenStream, target: InjectTarget) -> TokenStream {
    if target == InjectTarget::Middleware && !args.is_empty() {
        return create_compiler_error(&TokenStream2::from(args), "`#[middleware]` does not take any arguments");
    }
    let input = parse_macro_input!(item as ItemImpl);
    expand(&input, target).map_or_else(|error| error, Into::into)
}

fn expand(input: &ItemImpl, target: InjectTarget) -> Result<TokenStream2, TokenStream> {
    let main_crate = asp_dot_rust_crate_path();
    let self_ty = &input.self_ty;
    if target == InjectTarget::Middleware && input.trait_.is_some() {
        return Err(create_compiler_error(self_ty, "`#[middleware]` requires an inherent impl"));
    }

    let find_fn = |name: &str| {
        input.items.iter().find_map(|item| match item {
            ImplItem::Fn(function) if function.sig.ident == name => Some(function),
            _ => None,
        })
    };
    let extra = match target {
        InjectTarget::Service => None,
        InjectTarget::Controller => Some(("HttpContextRef", quote! { http_context })),
        InjectTarget::Middleware => Some(("MiddlewareNext", quote! { next })),
    };
    let (constructor, dependencies) = if let Some(new) = find_fn("new") {
        let await_call = if new.sig.asyncness.is_some() {
            if target != InjectTarget::Service {
                return Err(create_compiler_error(&new.sig, "fn `new` cannot be async"));
            }
            quote! { .await }
        } else {
            TokenStream2::new()
        };
        let InjectedArguments { dependencies, bindings, values } = InjectedArguments::resolve(&new.sig, new.sig.inputs.iter(), extra)?;
        let configuration = dependencies.configuration_binding(&main_crate);
        let registration_check = if target == InjectTarget::Controller {
            dependencies.check_registrations(self_ty)
        } else {
            TokenStream2::new()
        };
        // Finish borrowing the scope before moving a controller's HttpContextRef.
        let constructor = quote! {
            #configuration
            #registration_check
            #(#bindings)*
            Self::new(#(#values),*) #await_call
        };
        (constructor, dependencies)
    } else if target == InjectTarget::Middleware {
        (quote! { Self { next } }, Dependencies::default())
    } else {
        return Err(create_compiler_error(self_ty, "impl block must have fn `new`"));
    };

    let scope = quote! { #main_crate::services::service_provider::service_provider_scope::ServiceProviderScope };
    let (trait_path, methods, registration) = match target {
        InjectTarget::Service => (
            quote! { #main_crate::dependency_injection::DependencyInjectableService },
            quote! {
                async fn inject(service_scope: &#scope) -> Self {
                    #constructor
                }
            },
            dependencies.register(&main_crate, self_ty),
        ),
        InjectTarget::Controller => (
            quote! { #main_crate::dependency_injection::DependencyInjectableController },
            quote! {
                async fn inject(http_context: #main_crate::http_context::HttpContextRef) -> Self {
                    let service_scope = &http_context.service_provider;
                    #constructor
                }
            },
            TokenStream2::new(),
        ),
        InjectTarget::Middleware => {
            let Some(invoke) = find_fn("invoke_async") else {
                return Err(create_compiler_error(self_ty, "impl block must have fn `invoke_async`"));
            };
            let invoke = middleware_invoke(invoke, &main_crate)?;
            (
                quote! { #main_crate::middleware::Middleware },
                quote! {
                    async fn inject(service_scope: &#scope, next: #main_crate::middleware::MiddlewareNext) -> Self {
                        #constructor
                    }
                    #invoke
                },
                TokenStream2::new(),
            )
        }
    };

    Ok(quote! {
        #input
        #[#main_crate::third_party::async_trait::async_trait]
        impl #trait_path for #self_ty {
            #methods
        }
        #registration
    })
}

fn middleware_invoke(invoke: &ImplItemFn, main_crate: &TokenStream2) -> Result<TokenStream2, TokenStream> {
    let mut inputs = invoke.sig.inputs.iter();
    if !matches!(inputs.next(), Some(FnArg::Receiver(_))) || !matches!(inputs.next(), Some(FnArg::Typed(_))) {
        return Err(create_compiler_error(&invoke.sig, "fn `invoke_async` must take `&self` and `http_context: &mut HttpContext`"));
    }
    let InjectedArguments { dependencies, bindings, values } = InjectedArguments::resolve(&invoke.sig, inputs, None)?;
    let configuration = dependencies.configuration_binding(main_crate);
    let registration_check = dependencies.check_registrations(&quote! { Self });
    Ok(quote! {
        async fn inject_invoke_async(&self, http_context: &mut #main_crate::http_context::HttpContext) {
            let service_scope = &http_context.service_provider;
            #configuration
            #registration_check
            #(#bindings)*
            self.invoke_async(http_context #(, #values)*).await;
        }
    })
}
