use syn::Type;

/// Returns the exact type, unwrapping references if necessary.
pub fn get_exact_type(ty: &Type) -> &Type {
    match ty {
        Type::Path(_) => {
            return ty;
        }
        Type::Reference(r) => {
            return get_exact_type(&r.elem);
        }
        _ => return ty
    }
}

/// Extract inner type T from a single-generic-arg wrapper like `Serv<T>`,
/// `Cfg<T>`, `CfgRequire<T>`, `CfgReload<T>` — matched by ident name only,
/// so it works regardless of which crate path the wrapper was imported from.
/// Shared between `#[inject]`/`#[controller_inject]` and
/// `#[derive(DependcyInjectableService)]`.
pub fn extract_wrapper_inner(ty: &Type, wrapper_name: &str) -> Option<Type> {
    if let Type::Path(p) = ty {
        let seg = p.path.segments.last()?;
        if seg.ident == wrapper_name {
            if let syn::PathArguments::AngleBracketed(args) = &seg.arguments {
                if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                    return Some(inner.clone());
                }
            }
        }
    }
    None
}
