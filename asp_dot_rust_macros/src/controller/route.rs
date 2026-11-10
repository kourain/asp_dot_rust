use proc_macro::Ident;

#[allow(unused)]
pub(crate) struct Route {
    method_name: Ident,
    is_async: bool
}