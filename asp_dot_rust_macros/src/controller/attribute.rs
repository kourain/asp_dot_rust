use proc_macro::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::{Attribute, Expr, ItemImpl, LitStr, Token, punctuated::Punctuated};

use crate::utils::compiler_error::create_compiler_error;

/// `#[attr(...)]` and `#[max_body_size(...)]` are read by `#[controller_route]`, which removes the ones on the impl. One that reaches this macro
/// on an impl was written above `#[controller_route]`, which would have expanded first and dropped it, so it is an error.
pub(crate) fn attr(item: TokenStream, name: &str) -> TokenStream {
    if let Ok(input_impl) = syn::parse::<ItemImpl>(item.clone()) {
        return create_compiler_error(
            &input_impl.self_ty,
            format!("#[{}(...)] on a controller impl must be written below #[controller_route], not above it", name),
        );
    }
    item
}

/// Attributes that `#[controller_route]` turns into route attributes: `#[attr(key = value, ...)]` and `#[max_body_size(bytes)]`.
pub(crate) fn is_attr_attr(attr: &Attribute) -> bool {
    attr.path().is_ident("attr")
}

pub(crate) struct AttrEntry {
    key: syn::Ident,
    value: Expr,
}

/// The `key = value` pairs of the `#[attr(...)]` attributes in `attrs`
/// A key can be set once.
pub(crate) fn parse_attr_entries(attrs: &[Attribute], owner: &str) -> Result<Vec<AttrEntry>, TokenStream> {
    let mut entries: Vec<AttrEntry> = Vec::new();
    let mut push = |entry: AttrEntry| -> Result<(), TokenStream> {
        if entries.iter().any(|existing| existing.key.unraw() == entry.key.unraw()) {
            return Err(create_compiler_error(&entry.key, format!("Attribute `{}` is set twice on {}", entry.key.unraw(), owner)));
        }
        entries.push(entry);
        Ok(())
    };
    for attr in attrs.iter().filter(|attr| is_attr_attr(attr)) {
        let pairs = attr
            .parse_args_with(Punctuated::<syn::MetaNameValue, Token![,]>::parse_terminated)
            .map_err(|_| create_compiler_error(attr, format!("Invalid #[attr(...)] on {}, expected #[attr(key = value, ...)]", owner)))?;
        if pairs.is_empty() {
            return Err(create_compiler_error(attr, format!("Empty #[attr()] on {}, expected #[attr(key = value, ...)]", owner)));
        }
        for pair in pairs {
            let Some(key) = pair.path.get_ident().cloned() else {
                return Err(create_compiler_error(&pair.path, "The key of an attribute must be a single identifier"));
            };
            push(AttrEntry { key, value: pair.value })?;
        }
    }
    Ok(entries)
}

/// The attributes of an action: the ones of the impl that the action does not set again, then the ones of the action.
pub(crate) fn merged_attribute_calls(impl_entries: &[AttrEntry], method_entries: &[AttrEntry]) -> Vec<proc_macro2::TokenStream> {
    let inherited = impl_entries.iter().filter(|entry| !method_entries.iter().any(|own| own.key.unraw() == entry.key.unraw()));
    inherited
        .chain(method_entries.iter())
        .map(|entry| {
            let key_name = entry.key.unraw().to_string();
            let key = LitStr::new(&key_name, entry.key.span());
            let value = &entry.value;
            // the value is stored as its own type, read it back with exactly that type
            quote! { .with_attribute(#key, #value) }
        })
        .collect()
}
