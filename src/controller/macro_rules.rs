#[macro_export]
macro_rules! api_controller {
    ( $vis:vis $name:ident { $( $field_name:ident : $field_type:ty ),* $(,)? } ) => {
        $vis struct $name {
            $(
                $field_name : $field_type,
            )*
            http_context: $crate::controller::HttpContextRef,
        }

        impl $crate::controller::WithHttpContext for $name
        {
            fn str_name() -> &'static str {
                stringify!($name)
                .strip_suffix("Controller")
                        .unwrap_or(stringify!($name))
            }
            fn new_with_http_context(
                http_context: &mut $crate::http_context::HttpContext,
            ) -> Self {
                let controller_type_name = stringify!($name);
                if !controller_type_name.ends_with("Controller") {
                    panic!("Controller name must end with 'Controller'");
                }

                Self {
                    $(
                        $field_name : if let Some(service) = http_context.try_get_service::<$field_type>() {
                            (*service).clone()
                        } else {
                            Default::default()
                        },
                    )*
                    http_context: $crate::controller::HttpContextRef::new(http_context),
                }
            }
        }
    };
}
