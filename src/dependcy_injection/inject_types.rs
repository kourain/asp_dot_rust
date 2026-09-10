use arc_swap::ArcSwap;
use std::{ops::Deref, sync::Arc};

/// Injected service: resolved via `ServiceProviderScope::get_service::<T>()`.
/// Equivalent to a bare `Arc<T>` constructor parameter, but recognizable by
/// wrapper name instead of by raw `Arc<T>` shape.
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
/// registered in `ConfigurationService`. Equivalent to `Option<Arc<T>>`.
pub struct Cfg<T>(pub Option<Arc<T>>);
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
pub struct CfgRequire<T>(pub Arc<T>);
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
pub struct CfgReload<T>(pub Arc<ArcSwap<T>>);
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
