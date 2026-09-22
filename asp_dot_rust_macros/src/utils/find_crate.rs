use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::Span;
use quote::quote;
use syn::Ident;

/// Returns a `TokenStream` that resolves to the path of the `asp_dot_rust` crate, whether it's the current crate or an external dependency.
pub(crate) fn asp_dot_rust_crate_path() -> proc_macro2::TokenStream {
    match crate_name("asp_dot_rust") {
        Ok(FoundCrate::Itself) => quote!(crate),
        Ok(FoundCrate::Name(name)) => {
            let ident = Ident::new(&name, Span::call_site());
            quote!(::#ident)
        }
        Err(_) => quote!(::asp_dot_rust), // fallback
    }
}
