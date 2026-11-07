use crate::{
    dependency_injection::{Cfg, CfgReload, CfgRequire, DependencyInjectableService},
    logging::LOGGER,
    services::service_provider::service_provider_scope::ServiceProviderScope,
    utils::path::get_real_path,
};
use arc_swap::ArcSwap;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, RwLock},
};

/// Reload closure for one `configure_reload::<T>()` section: given the
/// freshly re-read tables, re-parses the section and `.store()`s the result
/// into the matching `ArcSwap<T>` already sitting in `_inner`.
pub(super) type ReloadFn = Box<dyn Fn(&HashMap<String, toml::Table>, &HashMap<TypeId, Arc<dyn Any + Send + Sync>>) + Send + Sync>;

pub(super) struct ReloadEntry {
    #[allow(dead_code)] // kept for future diagnostics / logging
    pub(super) section: String,
    pub(super) reload_fn: ReloadFn,
}

/// Read-only view of the application configuration.
///
/// It is created by `ConfigurationServiceBuilder::build()` when
/// `ApplicationBuilder::build()` runs, so values can no longer be added or
/// replaced after the application is built. The only state that still moves
/// is a section bound with `configure_reload::<T>()`, refreshed by `reload_all()`.
pub struct ConfigurationService {
    pub(super) _inner: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    // RwLock so `reload_all()` can run through `&self`, e.g. from a background
    // task holding `Arc<ConfigurationService>` after the app has started.
    pub(super) _toml_tables: RwLock<HashMap<String, toml::Table>>,
    pub(super) _reload_sections: HashMap<TypeId, ReloadEntry>,
}

// The container never builds this service: `ApplicationBuilder::build()` registers the
// finished instance, so resolving it from a scope where it was not registered is a bug.
#[async_trait::async_trait]
impl DependencyInjectableService for ConfigurationService {
    async fn inject(_service_scope: &ServiceProviderScope) -> Self {
        panic!("ConfigurationService is created by ApplicationBuilder::build and can not be constructed by the service container")
    }
}

impl ConfigurationService {
    /// check if the configuration of type T exists
    pub fn contains<T>(&self) -> bool
    where
        T: 'static + Send + Sync,
    {
        self._inner.contains_key(&TypeId::of::<T>())
    }

    /// get a configuration of type T, if the configuration of type T does not exist, return `Cfg(None)`
    pub fn get<T>(&self) -> Cfg<T>
    where
        T: Send + Sync + 'static,
    {
        match self._inner.get(&TypeId::of::<T>()) {
            Some(config) => Cfg(config.clone().downcast::<T>().ok()),
            None => Cfg(None),
        }
    }

    /// get a configuration of type T, panic if it was never registered
    /// (via `insert`, `configure`, or `configure_optional` on the builder)
    pub fn require<T>(&self) -> CfgRequire<T>
    where
        T: Send + Sync + 'static,
    {
        match self.get::<T>().0 {
            Some(val) => CfgRequire(val),
            None => panic!("Configuration of type {} is required but was never registered", std::any::type_name::<T>()),
        }
    }

    /// retrieve a section registered via `configure_reload::<T>()` on the builder,
    /// `None` if it was never registered
    pub fn get_reload<T>(&self) -> Option<CfgReload<T>>
    where
        T: Send + Sync + 'static,
    {
        let config = self._inner.get(&TypeId::of::<ArcSwap<T>>())?;
        CfgReload::<T>::downcast(config).ok()
    }

    /// Re-read every file loaded via `add_toml_cfg`/`add_optional_toml_cfg`
    /// from disk, and push fresh values into every section registered via
    /// `configure_reload::<T>()`. Safe to call through `&self` (e.g. from a
    /// polling/file-watch hosted service holding `Arc<ConfigurationService>`).
    /// Sections bound with plain `configure`/`configure_optional` are NOT
    /// affected — only `configure_reload::<T>()` sections refresh.
    pub fn reload_all(&self) {
        let paths: Vec<String> = self._toml_tables.read().unwrap().keys().cloned().collect();
        {
            let mut tables = self._toml_tables.write().unwrap();
            for path in paths {
                match get_real_path(&path) {
                    Some(real_path) => match std::fs::read_to_string(&real_path) {
                        Ok(data) => match data.parse::<toml::Table>() {
                            Ok(table) => {
                                tables.insert(path, table);
                            }
                            Err(e) => LOGGER::warn(format!("Skipped reload of {}: failed to parse ({})", path, e)),
                        },
                        Err(_) => LOGGER::warn(format!("Skipped reload of {}: failed to read", path)),
                    },
                    None => LOGGER::warn(format!("Skipped reload of {}: failed to find file", path)),
                }
            }
        }
        let tables = self._toml_tables.read().unwrap();
        for entry in self._reload_sections.values() {
            (entry.reload_fn)(&tables, &self._inner);
        }
    }
}
