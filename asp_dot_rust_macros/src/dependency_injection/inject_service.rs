use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{ItemImpl, parse_macro_input};

use super::{
    flags::ConstructorInjectOptions,
    inject::{Constructor, expand_constructor},
};
use crate::utils::find_crate::asp_dot_rust_crate_path;

pub(crate) fn expand(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemImpl);
    let main_crate = asp_dot_rust_crate_path();
    let constructor = Constructor {
        trait_name: format_ident!("DependencyInjectableService"),
        parameters: quote! { service_scope: &#main_crate::services::service_provider::service_provider_scope::ServiceProviderScope },
        scope_binding: TokenStream2::new(),
        extra_argument: None,
        options: ConstructorInjectOptions::ALLOW_ASYNC | ConstructorInjectOptions::REGISTER_DEPENDENCIES,
    };
    expand_constructor(&input, constructor, &main_crate).map_or_else(|error| error, |inject| quote! { #input #inject }.into())
}
