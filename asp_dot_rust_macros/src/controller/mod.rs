pub(crate) mod routing;
pub(crate) mod attribute;

pub(crate) fn http_action(item: proc_macro::TokenStream, _method: &str) -> proc_macro::TokenStream {
    item
}