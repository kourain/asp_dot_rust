/// create Controller with impl StructName
#[macro_export]
macro_rules! api_controller {
    ( $vis:vis $name:ident { $( $field_name:ident : $field_type:ty ),* $(,)? } ) => {
        /// pre-impl 
        /// ```no_run
        /// impl asp_dot_rust::utils::StructName for 
        /// {
        ///     fn str_name() -> &'static str {...}
        /// }
        /// ```
        $vis struct $name {
            $(
                $field_name : $field_type,
            )*
            http_context: $crate::http_context::HttpContextRef,
        }
    };
}
