pub(crate) mod action_params;
pub(crate) mod attribute;
pub(crate) mod extra_attribute;
pub(crate) mod route;
pub(crate) mod routing;

pub(crate) fn http_action(item: proc_macro::TokenStream, _method: &str) -> proc_macro::TokenStream {
    item
}
