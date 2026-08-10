use syn::Type;

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
