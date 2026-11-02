use proc_macro::Ident;

pub(crate) struct Route {
    method_name: Ident,
    is_async: bool
}