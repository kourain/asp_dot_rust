use arc_swap::ArcSwap;
use std::{any::Any, ops::Deref, sync::Arc};
macro_rules! define_service_wrapper {
    () => {};

    // Option<Arc>
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident ( Option<Arc> );
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        #[repr(transparent)]
        $vis struct $name<T: ?Sized>(pub(crate) ::std::option::Option<::std::sync::Arc<T>>);

        impl<T: ?Sized> ::std::ops::Deref for $name<T> {
            type Target = ::std::option::Option<::std::sync::Arc<T>>;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<T: ?Sized> ::std::clone::Clone for $name<T> {
            #[inline]
            fn clone(&self) -> Self {
                $name(self.0.clone())
            }
        }

        impl<T: ?Sized> ::std::default::Default for $name<T> {
            #[inline]
            fn default() -> Self {
                $name(::std::option::Option::None)
            }
        }

        impl<T> $name<T> {
            pub fn downcast(
                value: &$name<dyn ::std::any::Any + ::std::marker::Send + ::std::marker::Sync + 'static>,
            ) -> ::std::result::Result<
                $name<T>,
                ::std::option::Option<::std::sync::Arc<dyn ::std::any::Any + ::std::marker::Send + ::std::marker::Sync>>,
            >
            where
                T: ::std::marker::Send + ::std::marker::Sync + 'static,
            {
                match &value.0 {
                    ::std::option::Option::Some(arc) => {
                        arc.clone()
                            .downcast::<T>()
                            .map(|val| $name(::std::option::Option::Some(val)))
                            .map_err(::std::option::Option::Some)
                    }
                    ::std::option::Option::None => ::std::result::Result::Ok($name(::std::option::Option::None)),
                }
            }
            pub fn unwrap_or_default(self) -> Arc<T>
            where
                T: Default,
            {
                self.0.unwrap_or_default()
            }
        }

        define_service_wrapper!($($rest)*);
    };

    // Arc
    (
        $(#[$meta:meta])*
        $vis:vis struct $name:ident ( Arc );
        $($rest:tt)*
    ) => {
        $(#[$meta])*
        #[repr(transparent)]
        $vis struct $name<T: ?Sized>(pub(crate) ::std::sync::Arc<T>);

        impl<T: ?Sized> ::std::ops::Deref for $name<T> {
            type Target = ::std::sync::Arc<T>;

            #[inline]
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl<T: ?Sized> ::std::clone::Clone for $name<T> {
            #[inline]
            fn clone(&self) -> Self {
                $name(self.0.clone())
            }
        }

        impl<T: ::std::default::Default> ::std::default::Default for $name<T> {
            #[inline]
            fn default() -> Self {
                $name(::std::sync::Arc::new(::std::default::Default::default()))
            }
        }

        impl<T> $name<T> {
            #[doc = concat!("use \n```no_run\n", stringify!($name), "::<T>::downcast(&arc_any).expect(\"Type mismatch when downcasting service\");\n```")]
            pub fn downcast(
                value: &$name<dyn ::std::any::Any + ::std::marker::Send + ::std::marker::Sync + 'static>,
            ) -> ::std::result::Result<
                $name<T>,
                ::std::sync::Arc<dyn ::std::any::Any + ::std::marker::Send + ::std::marker::Sync>,
            >
            where
                T: ::std::marker::Send + ::std::marker::Sync + 'static,
            {
                value.0.clone().downcast::<T>().map($name)
            }
        }

        define_service_wrapper!($($rest)*);
    };
}
define_service_wrapper!(
    /// Injected service: resolved via `ServiceProviderScope::get_service::<T>()`.
    ///
    /// The tuple field is `pub` only so the `#[inject]` macro can
    /// construct `Serv(...)` from generated code in the caller's crate; prefer
    /// `.unwrap()` (or the `Deref` to `Arc<T>`) over reaching into `.0` directly.
    pub struct Serv(Arc);

    /// Optional configuration: resolves to `None` if the type was never
    /// registered in `ConfigurationService`.
    ///
    /// The tuple field is `pub` only so the `#[inject]` macro can
    /// construct `Cfg(...)` from generated code in the caller's crate; prefer
    /// `.unwrap()` over reaching into `.0` directly.
    pub struct Cfg(Option<Arc>);

    /// Required configuration: panics at construction time if the type was never
    /// registered in `ConfigurationService`.
    ///
    /// The tuple field is `pub` only so the `#[inject]` macro can
    /// construct `CfgRequire(...)` from generated code in the caller's crate;
    /// prefer `.unwrap()` over reaching into `.0` directly.
    pub struct CfgRequire(Arc);
);

/// Hot-reloadable configuration: the inner `ArcSwap<T>` is updated in place
/// whenever `ConfigurationService::reload_all()` re-reads the TOML file(s)
/// this section was bound from via `configure_reload::<T>()`.
///
/// The tuple field is `pub` only so the `#[inject]` macro can
/// construct `CfgReload(...)` from generated code in the caller's crate;
/// prefer `.unwrap()` over reaching into `.0` directly.
pub struct CfgReload<T>(pub(crate) Arc<ArcSwap<T>>);

impl<T> Deref for CfgReload<T> {
    type Target = ArcSwap<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Clone for CfgReload<T> {
    #[inline]
    fn clone(&self) -> Self {
        CfgReload(self.0.clone())
    }
}

impl<T: Default> Default for CfgReload<T> {
    #[inline]
    fn default() -> Self {
        CfgReload(Arc::new(ArcSwap::from_pointee(T::default())))
    }
}

impl<T> CfgReload<T> {
    pub fn downcast(value: &Arc<dyn Any + Send + Sync + 'static>) -> Result<CfgReload<T>, Arc<dyn Any + Send + Sync + 'static>>
    where
        T: Send + Sync + 'static,
    {
        value.clone().downcast::<ArcSwap<T>>().map(CfgReload)
    }
}
