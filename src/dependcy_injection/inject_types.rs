use arc_swap::ArcSwap;
use std::{ops::Deref, sync::Arc};

/// Injected service: resolved via `ServiceProviderScope::get_service::<T>()`.
///
/// The tuple field is `pub` only so the `#[inject_require]` macro can
/// construct `Serv(...)` from generated code in the caller's crate; prefer
/// `.unwrap()` (or the `Deref` to `Arc<T>`) over reaching into `.0` directly.
pub struct Serv<T: ?Sized>(pub Arc<T>);

impl<T: ?Sized> Deref for Serv<T> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T: ?Sized> Clone for Serv<T> {
    fn clone(&self) -> Self {
        Serv(self.0.clone())
    }
}

/// Optional configuration: resolves to `None` if the type was never
/// registered in `ConfigurationService`.
///
/// The tuple field is `pub` only so the `#[inject_require]` macro can
/// construct `Cfg(...)` from generated code in the caller's crate; prefer
/// `.unwrap()` over reaching into `.0` directly.
pub struct Cfg<T>(pub Option<Arc<T>>);
impl<T> Cfg<T> {
    /// Consume the wrapper and return the inner `Arc<T>`, panicking if the
    /// configuration was never registered — mirrors `Option::unwrap()`.
    pub fn unwrap(self) -> Arc<T> {
        self.0.unwrap()
    }

    /// Consume the wrapper, falling back to `Arc::new(T::default())` if the
    /// configuration was never registered — mirrors `Option::unwrap_or_default()`.
    pub fn unwrap_or_default(self) -> Arc<T>
    where
        T: Default,
    {
        self.0.unwrap_or_default()
    }
}
impl<T> Deref for Cfg<T> {
    type Target = Option<Arc<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> Clone for Cfg<T> {
    fn clone(&self) -> Self {
        Cfg(self.0.clone())
    }
}

/// Required configuration: panics at construction time if the type was never
/// registered in `ConfigurationService`.
///
/// The tuple field is `pub` only so the `#[inject_require]` macro can
/// construct `CfgRequire(...)` from generated code in the caller's crate;
/// prefer `.unwrap()` over reaching into `.0` directly.
pub struct CfgRequire<T>(pub Arc<T>);
impl<T> CfgRequire<T> {
    /// Consume the wrapper and return the inner `Arc<T>`.
    pub fn unwrap(self) -> Arc<T> {
        self.0
    }
}
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

/// Hot-reloadable configuration: the inner `ArcSwap<T>` is updated in place
/// whenever `ConfigurationService::reload_all()` re-reads the TOML file(s)
/// this section was bound from via `configure_reload::<T>()`.
///
/// The tuple field is `pub` only so the `#[inject_require]` macro can
/// construct `CfgReload(...)` from generated code in the caller's crate;
/// prefer `.unwrap()` over reaching into `.0` directly.
pub struct CfgReload<T>(pub Arc<ArcSwap<T>>);
impl<T> CfgReload<T> {
    /// Consume the wrapper and return the inner `Arc<ArcSwap<T>>`.
    pub fn unwrap(self) -> Arc<ArcSwap<T>> {
        self.0
    }
}
impl<T> Deref for CfgReload<T> {
    type Target = Arc<ArcSwap<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> Clone for CfgReload<T> {
    fn clone(&self) -> Self {
        CfgReload(self.0.clone())
    }
}
