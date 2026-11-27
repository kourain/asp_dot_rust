use proc_macro::TokenStream;

/// Stand-in for the `get` macro of another web crate: keeps the function and adds `__other_get_path()`.
#[proc_macro_attribute]
pub fn get(args: TokenStream, item: TokenStream) -> TokenStream {
    let source = item.to_string();
    let name = source.split("fn ").nth(1).and_then(|rest| rest.split('(').next()).unwrap().trim().to_string();
    let extra = format!("pub fn __other_get_path_{name}() -> &'static str {{ {} }}", args);
    format!("{source} {extra}").parse().unwrap()
}
