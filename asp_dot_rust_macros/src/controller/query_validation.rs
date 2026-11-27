use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, Expr, Ident, MetaList, Token, punctuated::Punctuated};

use crate::utils::find_crate::asp_dot_rust_crate_path;

pub(super) struct QueryValidation {
    attribute: Ident,
    method: Ident,
    arguments: TokenStream,
}

impl QueryValidation {
    pub(super) fn parse(attr: &Attribute) -> syn::Result<Self> {
        let list: MetaList = attr.parse_args()?;
        let method = list.path.require_ident()?.clone();
        let args = list.parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)?;
        let arguments = if args.iter().any(|expr| matches!(expr, Expr::Assign(_))) {
            named_bounds(&args)?
        } else {
            // One expression stays a scalar. Multiple expressions form a tuple.
            let args = args.iter();
            quote!((#(#args),*))
        };
        Ok(Self { attribute: attr.path().require_ident()?.clone(), method, arguments })
    }

    pub(super) fn check(&self, binder: &Ident, field: &str) -> TokenStream {
        let main = asp_dot_rust_crate_path();
        let Self { attribute, method, arguments } = self;
        let value = Ident::new("__asp_query_value", proc_macro2::Span::mixed_site());
        quote! {
            // One import per annotated parameter: hovering `validate` in
            // #[validate(length(2))] resolves to the attribute macro's docs.
            #[allow(unused_imports)]
            use #main::macros::#attribute as _;
            #[allow(unused_imports)]
            use #main::controller::validation::RequiredFieldValue as _;
            if let ::core::option::Option::Some(#value) = #main::controller::validation::FieldValue(&#binder.0).present() {
                #main::controller::validation::validate(
                    // Hovering the inner rule name resolves to its method, including extension-trait methods.
                    #main::controller::ControllerActionFieldValidate::#method(#value, #field, #arguments)
                )?;
            }
        }
    }
}

fn named_bounds(args: &Punctuated<Expr, Token![,]>) -> syn::Result<TokenStream> {
    let mut min = None;
    let mut max = None;
    for arg in args {
        let Expr::Assign(pair) = arg else {
            return Err(syn::Error::new_spanned(arg, "cannot mix positional arguments and named bounds"));
        };
        let bound = match pair.left.as_ref() {
            Expr::Path(key) if key.path.is_ident("min") => &mut min,
            Expr::Path(key) if key.path.is_ident("max") => &mut max,
            key => return Err(syn::Error::new_spanned(key, "expected min or max")),
        };
        if bound.replace(&pair.right).is_some() {
            return Err(syn::Error::new_spanned(arg, "duplicate validation bound"));
        }
    }
    let main = asp_dot_rust_crate_path();
    let bounds = quote!(#main::controller::validation::ValidateBounds);
    Ok(match (min, max) {
        (Some(min), Some(max)) => quote!(const {
            let (min, max) = (#min, #max);
            assert!(min <= max, "validation min exceeds max");
            #bounds::Between(min, max)
        }),
        (Some(min), None) => quote!(#bounds::Min(#min)),
        (None, Some(max)) => quote!(#bounds::Max(#max)),
        (None, None) => unreachable!("a named bound was supplied"),
    })
}
