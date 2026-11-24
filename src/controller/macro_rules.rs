/// Defines a controller with an `HttpContextRef` field.
#[macro_export]
macro_rules! api_controller {
    ( $(#[$attr:meta])* $vis:vis $name:ident { $( $(#[$field_attr:meta])* $field_name:ident : $field_type:ty ),* $(,)? } ) => {
        $(#[$attr])*
        $vis struct $name {
            $(
                $(#[$field_attr])*
                $field_name : $field_type,
            )*
            http_context: $crate::http_context::HttpContextRef,
        }
    };
}
