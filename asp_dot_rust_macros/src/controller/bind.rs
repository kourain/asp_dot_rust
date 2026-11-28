use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Expr, ExprLit, Lit, LitBool, LitStr};

#[derive(Default)]
pub(super) struct BindOptions {
    pub name: Option<LitStr>,
    pub default: Option<Expr>,
    pub validate_default: Option<LitBool>,
}

impl BindOptions {
    pub fn parse(attr: &Attribute) -> syn::Result<Self> {
        let mut options = Self::default();
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                if options.name.is_some() {
                    return Err(meta.error("duplicate bind name"));
                }
                options.name = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("default") {
                if options.default.is_some() {
                    return Err(meta.error("duplicate bind default"));
                }
                options.default = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("validate_default") {
                if options.validate_default.is_some() {
                    return Err(meta.error("duplicate validate_default"));
                }
                options.validate_default = Some(meta.value()?.parse()?);
            } else {
                return Err(meta.error("expected name, default or validate_default"));
            }
            Ok(())
        })?;
        Ok(options)
    }

    pub fn default_value(&self) -> Option<TokenStream> {
        self.default.as_ref().map(|expr| match expr {
            Expr::Lit(ExprLit { lit: Lit::Str(text), .. }) => quote!(::std::convert::Into::into(::std::string::String::from(#text))),
            _ => quote!(#expr),
        })
    }
}
