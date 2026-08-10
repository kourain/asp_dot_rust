use proc_macro::TokenStream;
use quote::ToTokens;
pub fn create_compiler_error(ty: &impl ToTokens, message: impl Into<String>) -> TokenStream {
    return syn::Error::new_spanned(ty, message.into()).into_compile_error().into();
}
pub fn token_type_to_string(ty: &impl ToTokens) -> String {
    token_to_string(ty).replace(" ", "")
}
pub fn token_to_string(token: &impl  ToTokens) -> String {
    quote::quote!(#token).to_string()
}