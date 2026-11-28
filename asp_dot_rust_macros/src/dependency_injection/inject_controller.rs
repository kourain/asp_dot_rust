use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::ItemImpl;

use super::{
    flags::ConstructorInjectOptions,
    inject::{Constructor, ExtraArgument, expand_constructor},
};
use crate::utils::find_crate::asp_dot_rust_crate_path;

pub(crate) fn expand(item: TokenStream) -> TokenStream {
    let input = match syn::parse::<ItemImpl>(item.clone()) {
        Ok(input) => input,
        Err(error) => return error.to_compile_error().into(),
    };
    let main_crate = asp_dot_rust_crate_path();
    let constructor = Constructor {
        trait_name: format_ident!("DependencyInjectableController"),
        parameters: quote! { http_context: #main_crate::http_context::HttpContextRef },
        scope_binding: quote! { let service_scope = &http_context.service_provider; },
        extra_argument: Some(ExtraArgument {
            name: "HttpContextRef",
            value: quote! { http_context },
            rejected_type: Some("HttpContext"),
        }),
        options: ConstructorInjectOptions::OPTIONAL | ConstructorInjectOptions::CHECK_REGISTRATIONS,
    };
    expand_constructor(&input, constructor, &main_crate).map_or_else(
        |error| error,
        |inject| {
            let mut output = item;
            output.extend(TokenStream::from(inject));
            output
        },
    )
}
