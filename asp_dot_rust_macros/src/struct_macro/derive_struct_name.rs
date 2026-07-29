use proc_macro::TokenStream;

use crate::utils::find_crate::asp_dot_rust_crate_path;

pub(crate) fn struct_name(input: TokenStream) -> TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let struct_name = &ast.ident;
    let main_crate_path = asp_dot_rust_crate_path();
    quote::quote! {
        impl #main_crate_path::utils::StructName for #struct_name {
            fn str_name() -> &'static str
            {
                stringify!(#struct_name)
            }
        }
    }.into()
}
