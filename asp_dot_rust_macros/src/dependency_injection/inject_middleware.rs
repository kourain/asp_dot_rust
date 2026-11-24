use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{FnArg, ImplItemFn, ItemImpl, parse_macro_input};

use super::{
    flags::ConstructorInjectOptions,
    inject::{Constructor, ExtraArgument, InjectedArguments, expand_constructor, find_fn},
};
use crate::utils::{compiler_error::create_compiler_error, find_crate::asp_dot_rust_crate_path};

pub(crate) fn expand(args: TokenStream, item: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return create_compiler_error(&TokenStream2::from(args), "`#[middleware]` does not take any arguments");
    }
    let input = parse_macro_input!(item as ItemImpl);
    expand_middleware(&input).map_or_else(|error| error, Into::into)
}

fn expand_middleware(input: &ItemImpl) -> Result<TokenStream2, TokenStream> {
    let self_ty = &input.self_ty;
    if input.trait_.is_some() {
        return Err(create_compiler_error(self_ty, "`#[middleware]` requires an inherent impl"));
    }
    let Some(invoke) = find_fn(input, "invoke_async") else {
        return Err(create_compiler_error(self_ty, "impl block must have fn `invoke_async`"));
    };
    let main_crate = asp_dot_rust_crate_path();
    let invoke = middleware_invoke(invoke, &main_crate)?;
    let scope = quote! { #main_crate::services::service_provider::service_provider_scope::ServiceProviderScope };
    let constructor = Constructor {
        trait_name: format_ident!("DependencyInjectableMiddleware"),
        parameters: quote! { service_scope: &#scope, next: #main_crate::middleware::MiddlewareNext },
        scope_binding: TokenStream2::new(),
        extra_argument: Some(ExtraArgument {
            name: "MiddlewareNext",
            value: quote! { next },
            rejected_type: None,
        }),
        options: ConstructorInjectOptions::OPTIONAL,
    };
    let inject = expand_constructor(input, constructor, &main_crate)?;
    let (impl_generics, _, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        #input
        #inject
        #[#main_crate::third_party::async_trait::async_trait]
        impl #impl_generics #main_crate::middleware::Middleware for #self_ty #where_clause {
            #invoke
        }
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
