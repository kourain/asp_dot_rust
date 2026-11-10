use std::{ops::Deref, sync::Arc};

use crate::services::configuration::EntryValue;

macro_rules! define_service_wrapper {
    () => {};

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
            /// the inner `Arc<T>`
            #[inline]
            pub fn unwrap(self) -> Arc<T> {
                self.0
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
);

/// Handle of a configuration, it follows every reload of the configuration.
/// Use `capture` to get the value at this moment.
#[derive(Default)]
pub struct CfgReload<T>(pub(crate) EntryValue<T>);

impl<T> CfgReload<T> {
    pub fn capture(&self) -> CfgOption<T> {
        CfgOption(self.0.load_full())
    }
}

impl<T> Clone for CfgReload<T> {
    fn clone(&self) -> Self {
        CfgReload(self.0.clone())
    }
}

/// Value of a configuration at the moment it was captured, it has no value if the configuration is not registered,
/// its section is missing or it is not loaded yet. It does not change when the configuration is reloaded.
///
/// `Cfg<T>` will be removed, use `CfgOption<T>`.
#[deprecated(note = "Cfg<T> will be removed, use CfgOption<T> instead")]
pub type Cfg<T> = CfgOption<T>;

#[derive(Default)]
pub struct CfgOption<T>(pub(crate) Option<Arc<T>>);

impl<T> CfgOption<T> {
    /// `true` if the configuration has a value
    pub fn has_value(&self) -> bool {
        self.0.is_some()
    }

    /// the value, **panics** if there is none (check `has_value` first if that is possible)
    pub fn unwrap(self) -> CfgRequire<T> {
        match self.0 {
            Some(arc) => CfgRequire(arc),
            None => panic!("Configuration of type {} has no value: its section is missing or it was not loaded yet", std::any::type_name::<T>()),
        }
    }
}

impl<T: Default> CfgOption<T> {
    pub fn unwrap_or_default(&self) -> CfgRequire<T> {
        CfgRequire(self.0.as_ref().map(|arc| arc.clone()).unwrap_or_else(|| Arc::new(T::default())))
    }
}

impl<T> Deref for CfgOption<T> {
    type Target = Option<Arc<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Clone for CfgOption<T> {
    fn clone(&self) -> Self {
        CfgOption(self.0.clone())
    }
}

/// Value of a configuration that must have one.
pub struct CfgRequire<T>(pub(crate) Arc<T>);

impl<T> Deref for CfgRequire<T> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Clone for CfgRequire<T> {
    fn clone(&self) -> Self {
        CfgRequire(self.0.clone())
    }
}
