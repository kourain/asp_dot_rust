use proc_macro::TokenStream;
use quote::ToTokens;

/// Creates a compiler error `TokenStream` for the given type and message.
pub fn create_compiler_error(ty: &impl ToTokens, message: impl Into<String>) -> TokenStream {
    return syn::Error::new_spanned(ty, message.into()).into_compile_error().into();
}

/// Converts a type to a string representation, removing spaces.
pub fn token_type_to_string(ty: &impl ToTokens) -> String {
    token_to_string(ty).replace(" ", "")
}

/// Converts a token to a string representation.
pub fn token_to_string(token: &impl  ToTokens) -> String {
    quote::quote!(#token).to_string()
}