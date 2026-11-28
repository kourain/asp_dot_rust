use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{FnArg, ImplItem, ImplItemFn, ItemImpl, Signature};

use super::{dependencies::Dependencies, flags::ConstructorInjectOptions};
use crate::utils::{
    compiler_error::{create_compiler_error, token_type_to_string},
    extract_type::{get_exact_type, type_name},
};

pub(super) struct Constructor {
    pub(super) trait_name: syn::Ident,
    pub(super) parameters: TokenStream2,
    pub(super) scope_binding: TokenStream2,
    pub(super) extra_argument: Option<ExtraArgument>,
    pub(super) options: ConstructorInjectOptions,
}

pub(super) struct ExtraArgument {
    pub(super) name: &'static str,
    pub(super) value: TokenStream2,
    pub(super) rejected_type: Option<&'static str>,
}

#[derive(Default)]
pub(super) struct InjectedArguments {
    pub(super) dependencies: Dependencies,
    pub(super) bindings: Vec<TokenStream2>,
    pub(super) values: Vec<TokenStream2>,
}

impl InjectedArguments {
    pub(super) fn resolve<'a>(signature: &Signature, inputs: impl Iterator<Item = &'a FnArg>, extra: Option<ExtraArgument>) -> Result<Self, TokenStream> {
        let mut arguments = Self::default();
        let mut has_extra = false;
        for (index, input) in inputs.enumerate() {
            let FnArg::Typed(parameter) = input else {
                return Err(create_compiler_error(input, "fn `new` must be an associated function"));
            };
            let ty = &parameter.ty;
            if let Some(ExtraArgument { name, value, .. }) = &extra
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
                if let Some(ExtraArgument {
                    name, rejected_type: Some(rejected), ..
                }) = &extra
                    && type_name(get_exact_type(ty)).is_some_and(|ident| ident == *rejected)
                {
                    return Err(create_compiler_error(ty, format!("Can't inject {rejected}, use {name} instead")));
                }
                let allowed_extra = extra.as_ref().map_or(String::new(), |argument| format!("{}, ", argument.name));
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
        if let Some(ExtraArgument { name, .. }) = extra
            && !has_extra
        {
            return Err(create_compiler_error(signature, format!("fn `{}` must take `{name}`", signature.ident)));
        }
        Ok(arguments)
    }
}

pub(super) fn find_fn<'a>(input: &'a ItemImpl, name: &str) -> Option<&'a ImplItemFn> {
    input.items.iter().find_map(|item| match item {
        ImplItem::Fn(function) if function.sig.ident == name => Some(function),
        _ => None,
    })
}

pub(super) fn expand_constructor(input: &ItemImpl, constructor: Constructor, main_crate: &TokenStream2) -> Result<TokenStream2, TokenStream> {
    let Constructor {
        trait_name,
        parameters,
        scope_binding,
        extra_argument,
        options,
    } = constructor;
    let self_ty = &input.self_ty;
    let Some(new) = find_fn(input, "new") else {
        if options.contains(ConstructorInjectOptions::OPTIONAL) {
            return Ok(TokenStream2::new());
        }
        return Err(create_compiler_error(self_ty, "impl block must have fn `new`"));
    };

    let await_call = if new.sig.asyncness.is_some() {
        if !options.contains(ConstructorInjectOptions::ALLOW_ASYNC) {
            return Err(create_compiler_error(&new.sig, "fn `new` cannot be async"));
        }
        quote! { .await }
    } else {
        TokenStream2::new()
    };
    let InjectedArguments { dependencies, bindings, values } = InjectedArguments::resolve(&new.sig, new.sig.inputs.iter(), extra_argument)?;
    let configuration = dependencies.configuration_binding(main_crate);
    let registration_check = if options.contains(ConstructorInjectOptions::CHECK_REGISTRATIONS) {
        dependencies.check_registrations(self_ty)
    } else {
        TokenStream2::new()
    };
    let registration = if options.contains(ConstructorInjectOptions::REGISTER_DEPENDENCIES) {
        dependencies.register(main_crate, self_ty)
    } else {
        TokenStream2::new()
    };
    let (impl_generics, _, where_clause) = input.generics.split_for_impl();
    // Finish borrowing the scope before moving any constructor arguments.
    Ok(quote! {
        #[#main_crate::third_party::async_trait::async_trait]
        impl #impl_generics #main_crate::dependency_injection::#trait_name for #self_ty #where_clause {
            async fn inject(#parameters) -> Self {
                #scope_binding
                #configuration
                #registration_check
                #(#bindings)*
                Self::new(#(#values),*) #await_call
            }
        }
        #registration
    })
}
