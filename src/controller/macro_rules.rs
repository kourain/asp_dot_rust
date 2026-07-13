#[macro_export]
macro_rules! api_controller {
    ( $vis:vis $name:ident { $( $field_name:ident : $field_type:ty ),* $(,)? } ) => {
        $vis struct $name {
            $(
                $field_name : $field_type,
            )*
            http_context: $crate::controller::HttpContextRef,
        }
    };
}
