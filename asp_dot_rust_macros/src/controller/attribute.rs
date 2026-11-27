use proc_macro::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{Attribute, Expr, ItemImpl, LitStr, Token, punctuated::Punctuated};

use crate::utils::compiler_error::{create_compiler_error, create_compiler_error_from_token_stream};

/// A built-in attribute macro: `#[name(value)]` is `#[attr(name = value)]`, with the value typed as `ty`.
/// They are all declared in one place, `for_each_attr_alias!` in `extra_attribute.rs`.
pub(crate) struct AttrAlias {
    pub name: &'static str,
    pub ty: &'static str,
}

/// The name of an attribute macro of this crate: the last segment of its path.
pub(crate) fn attr_name(attr: &Attribute) -> Option<&syn::Ident> {
    attr.path().segments.last().map(|segment| &segment.ident)
}

/// Whether the path starts at this crate: `asp_dot_rust::...`, `::asp_dot_rust::...`, the name the crate was renamed to in `Cargo.toml`,
/// or `crate::...` when the crate is compiling itself. A path of another crate (`other::get`) is not.
fn is_own_path(path: &syn::Path) -> bool {
    let Some(first) = path.segments.first() else {
        return false;
    };
    if first.ident == "asp_dot_rust" || first.ident == "asp_dot_rust_macros" {
        return true;
    }
    match proc_macro_crate::crate_name("asp_dot_rust") {
        Ok(proc_macro_crate::FoundCrate::Itself) => first.ident == "crate",
        Ok(proc_macro_crate::FoundCrate::Name(name)) => first.ident == name.as_str(),
        Err(_) => false,
    }
}

/// Whether the attribute is the macro `name` of this crate. A path of one segment (`#[get]`) is taken as ours, a macro can not see what a name
/// was imported from. A longer path has to start at this crate, so `#[other_crate::get(...)]` is left to the other crate.
pub(crate) fn attr_named(attr: &Attribute, name: &str) -> bool {
    let path = attr.path();
    attr_name(attr).is_some_and(|ident| ident == name) && (path.segments.len() == 1 || is_own_path(path))
}

fn alias_of(attr: &Attribute) -> Option<&'static AttrAlias> {
    crate::controller::extra_attribute::ATTR_ALIASES.iter().find(|alias| attr_named(attr, alias.name))
}

/// `name = { let value: ty = <value>; value }`: the value is of the type of the alias, a value of another type is a compile error.
fn alias_pair(alias: &AttrAlias, name_span: proc_macro2::Span, value: &proc_macro2::TokenStream) -> proc_macro2::TokenStream {
    let name = syn::Ident::new(alias.name, name_span);
    let ty: syn::Type = syn::parse_str(alias.ty).expect("the type of an attribute alias is a valid type");
    quote! { #name = { let value: #ty = #value; value } }
}

/// Body of the built-in attribute macros: puts `#[attr(name = value)]` on the item.
/// `#[controller_route]` expands first and reads the alias itself with `alias_to_attr` (it can not see what this macro generates),
/// the result is the same attribute.
pub(crate) fn create_attr(args: TokenStream, item: TokenStream, name: &str) -> TokenStream {
    let item = attr(&args, item, name);
    let Some(alias) = crate::controller::extra_attribute::ATTR_ALIASES.iter().find(|alias| alias.name == name) else {
        return create_compiler_error(&proc_macro2::TokenStream::new(), format!("#[{}] is not declared in attr_aliases!", name));
    };
    let args = proc_macro2::TokenStream::from(args);
    if syn::parse2::<Expr>(args.clone()).is_err() {
        return create_compiler_error(&args, format!("Invalid #[{}(...)], expected a value of type {}", name, alias.ty));
    }
    let pair = alias_pair(alias, proc_macro2::Span::call_site(), &args);
    let item = proc_macro2::TokenStream::from(item);
    // the full path, so the user does not need to import `attr` to use a built-in attribute macro
    let main_crate_path = crate::utils::find_crate::asp_dot_rust_crate_path();
    quote! { #[#main_crate_path::controller::attr(#pair)] #item }.into()
}

/// `#[name(value)]` as the attribute `#[attr(name = value)]`.
fn alias_to_attr(attr: &Attribute, alias: &AttrAlias, owner: &str) -> Result<Attribute, TokenStream> {
    let value: Expr = attr
        .parse_args()
        .map_err(|_| create_compiler_error(attr, format!("Invalid #[{}(...)] on {}, expected a value of type {}", alias.name, owner, alias.ty)))?;
    let pair = alias_pair(alias, attr.span(), &quote! { #value });
    Ok(syn::parse_quote!(#[attr(#pair)]))
}

/// `#[attr(...)]` and `#[max_body_size(...)]` are read by `#[controller_route]`, which removes the ones on the impl. One that reaches this macro
/// on an impl was written above `#[controller_route]`, which would have expanded first and dropped it, so it is an error.
pub(crate) fn attr(args: &TokenStream, item: TokenStream, name: &str) -> TokenStream {
    if let Ok(_) = syn::parse::<ItemImpl>(item.clone()) {
        return create_compiler_error_from_token_stream(args, format!("#[{}(...)] on a controller impl must be written below #[controller_route], not above it", name));
    }
    item
}

/// Attributes that `#[controller_route]` turns into route attributes: `#[attr(key = value, ...)]` and the built-in ones (`#[max_body_size(bytes)]`, ...).
pub(crate) fn is_attr_attr(attr: &Attribute) -> bool {
    attr_named(attr, "attr") || alias_of(attr).is_some()
}

pub(crate) struct AttrEntry {
    key: syn::Ident,
    value: Expr,
}

/// The `key = value` pairs of the `#[attr(...)]` attributes in `attrs`, the built-in attribute macros count as `#[attr(...)]`.
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
        let converted;
        let attr = match alias_of(attr) {
            Some(alias) => {
                converted = alias_to_attr(attr, alias, owner)?;
                &converted
            }
            None => attr,
        };
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
            quote! { .with_attribute(#key, #value) }
        })
        .collect()
}
