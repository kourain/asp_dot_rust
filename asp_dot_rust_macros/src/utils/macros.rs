macro_rules! try_get_or_error {
    (expr) => {
        let path_expr;
        if let Some(val) = route_iter.next() {
            path_expr = val;
        } else {
            return create_compiler_error(attr, "Missing route method or path in #[route]");
        }
    };
}