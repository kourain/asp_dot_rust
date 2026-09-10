use crate::{dependcy_injection::DependcyInjectableService, logging::LOGGER, utils::get_real_path};
use arc_swap::ArcSwap;
use serde::de::DeserializeOwned;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

/// Reload closure for one `configure_reload::<T>()` section: given the
/// freshly re-read tables, re-parses the section and `.store()`s the result
/// into the matching `ArcSwap<T>` already sitting in `_inner`.
type ReloadFn = Box<dyn Fn(&HashMap<String, toml::Table>, &HashMap<TypeId, Arc<dyn Any + Send + Sync>>) + Send + Sync>;

struct ReloadEntry {
    #[allow(dead_code)] // kept for future diagnostics / logging
    section: String,
    reload_fn: ReloadFn,
}

pub struct ConfigurationService {
    _inner: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    // RwLock so `reload_all()` can run through `&self`, e.g. from a background
    // task holding `Arc<ConfigurationService>` after the app has started.
    _toml_tables: RwLock<HashMap<String, toml::Table>>,
    _reload_sections: RwLock<HashMap<TypeId, ReloadEntry>>,
}
impl DependcyInjectableService for ConfigurationService {
    fn inject(_service_provider: &crate::services::service_provider::ServiceProviderScope) -> Self {
        Self::new()
    }
}
impl ConfigurationService {
    pub fn new() -> Self {
        Self {
            _inner: HashMap::new(),
            _toml_tables: RwLock::new(HashMap::new()),
            _reload_sections: RwLock::new(HashMap::new()),
        }
    }

    /// check if the configuration of type T exists
    pub fn contains<T: 'static + Send + Sync>(&self) -> bool {
        self._inner.contains_key(&TypeId::of::<T>())
    }

    /// insert a configuration of type T, if the configuration of type T already exists, it will be replaced
    pub fn insert<T: 'static + Send + Sync>(&mut self, config: T)
    where
        T: Clone + Send + Sync + 'static,
    {
        if self._inner.contains_key(&TypeId::of::<T>()) {
            LOGGER::warn(format!("Replace configuration of type {} already exists", std::any::type_name::<T>()));
        }
        self._inner.insert(TypeId::of::<T>(), Arc::new(config));
    }

    /// get a configuration of type T, if the configuration of type T does not exist, return None
    pub fn get<T: 'static + Send + Sync>(&self) -> Option<Arc<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        match self._inner.get(&TypeId::of::<T>()) {
            Some(config) => config.clone().downcast::<T>().ok(),
            None => None,
        }
    }

    /// get a configuration of type T, panic if it was never registered
    /// (via `insert`, `configure`, or `configure_optional`)
    pub fn require<T: 'static + Send + Sync + Clone>(&self) -> Arc<T> {
        self.get::<T>()
            .unwrap_or_else(|| panic!("Configuration of type {} is required but was never registered", std::any::type_name::<T>()))
    }

    /// load default toml configuration file, if the file does not exist, it will be ignored
    pub(crate) fn load_default_toml(&mut self) {
        self.add_optional_toml_cfg("./appsettings.toml");
    }

    /// add a toml configuration file, if the file does not exist, it will panic
    pub fn add_toml_cfg(&mut self, path: impl AsRef<str>) -> &mut Self {
        match std::fs::read_to_string(get_real_path(&path)) {
            Ok(data) => match data.parse() {
                Ok(toml_table) => {
                    self._toml_tables.write().unwrap().insert(path.as_ref().to_string(), toml_table);
                }
                Err(e) => panic!("Failed to parse {} file {}", path.as_ref(), e),
            },
            Err(_) => {
                panic!("Failed to read configuration file {}", path.as_ref());
            }
        };
        self
    }

    /// add a toml configuration file, if the file does not exist, it will be ignored
    pub fn add_optional_toml_cfg(&mut self, path: impl AsRef<str>) -> &mut Self {
        match std::fs::read_to_string(get_real_path(&path)) {
            Ok(data) => match data.parse() {
                Ok(toml_table) => {
                    self._toml_tables.write().unwrap().insert(path.as_ref().to_string(), toml_table);
                }
                Err(e) => panic!("Failed to parse {} file {}", path.as_ref(), e),
            },
            Err(_) => {
                LOGGER::warn(format!("Failed to read configuration file {}", path.as_ref()));
            }
        };
        self
    }

    /// parse `section` out of every loaded toml table into `T`, panicking if
    /// it is present but malformed. Returns `None` if the section was not
    /// found in any loaded file (caller decides whether that's a panic or a
    /// fallback to `T::default()`).
    fn try_build_section<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned + Default + Debug + Clone + Send + Sync + 'static,
    {
        let mut config = None;
        for (path, table) in self._toml_tables.read().unwrap().iter() {
            let Some(value) = table.get(key) else {
                continue;
            };
            match value.clone().try_into::<T>() {
                Ok(val) => config = Some(val),
                Err(_) => panic!("Failed to parse [{}] of toml {} as {}", key, path, std::any::type_name::<T>()),
            }
        }
        config
    }

    /// configure a configuration of type T from the toml configuration file, if the configuration of type T does not exist, it will panic
    pub fn configure<T>(&mut self, section: impl AsRef<str>) -> &mut Self
    where
        T: DeserializeOwned + Default + Debug + Clone + Send + Sync + 'static,
    {
        let key = section.as_ref();
        match self.try_build_section::<T>(key) {
            Some(config) => {
                LOGGER::trace(format!("Add TOML Config for: {}", std::any::type_name::<T>()));
                LOGGER::verbose(format!("{} = {:#?}", std::any::type_name::<T>(), config.clone()));
                self.insert(config);
            }
            None => panic!("Failed to find [{}] in any loaded toml configuration", key),
        }
        self
    }

    /// configure a configuration of type T from the toml configuration file, if the configuration of type T does not exist, it will be ignored
    pub fn configure_optional<T>(&mut self, section: impl AsRef<str>) -> &mut Self
    where
        T: DeserializeOwned + Default + Debug + Clone + Send + Sync + 'static,
    {
        let key = section.as_ref();
        match self.try_build_section::<T>(key) {
            Some(config) => {
                LOGGER::trace(format!("Add TOML Config for: {}", std::any::type_name::<T>()));
                LOGGER::verbose(format!("{} = {:#?}", std::any::type_name::<T>(), config.clone()));
                self.insert(config);
            }
            None => {
                LOGGER::warn(format!("Failed to find [{}] in any loaded toml configuration", key));
                self.insert(T::default());
            }
        }
        self
    }

    /// bind `section` into a hot-reloadable `ArcSwap<T>`. Panics if the
    /// section is not found in any loaded file (same as `configure::<T>()`).
    /// Retrieve it later via `CfgReload<T>` (macro-injected) or `get_reload::<T>()`.
    pub fn configure_reload<T>(&mut self, section: impl AsRef<str>) -> &mut Self
    where
        T: DeserializeOwned + Default + Debug + Clone + Send + Sync + 'static,
    {
        let key = section.as_ref().to_string();
        let Some(initial) = self.try_build_section::<T>(&key) else {
            panic!("Failed to find [{}] in any loaded toml configuration", key);
        };
        LOGGER::trace(format!("Add reloadable TOML Config for: {}", std::any::type_name::<T>()));
        let swap = Arc::new(ArcSwap::from_pointee(initial));
        self._inner.insert(TypeId::of::<ArcSwap<T>>(), swap as Arc<dyn Any + Send + Sync>);

        let key_for_closure = key.clone();
        self._reload_sections.write().unwrap().insert(
            TypeId::of::<ArcSwap<T>>(),
            ReloadEntry {
                section: key,
                reload_fn: Box::new(move |tables, inner| {
                    let Some(any_swap) = inner.get(&TypeId::of::<ArcSwap<T>>()) else {
                        return;
                    };
                    let Ok(swap) = any_swap.clone().downcast::<ArcSwap<T>>() else {
                        return;
                    };
                    for (path, table) in tables.iter() {
                        let Some(value) = table.get(&key_for_closure) else {
                            continue;
                        };
                        match value.clone().try_into::<T>() {
                            Ok(new_val) => {
                                swap.store(Arc::new(new_val));
                                LOGGER::trace(format!("Reloaded TOML Config for: {}", std::any::type_name::<T>()));
                            }
                            Err(e) => LOGGER::warn(format!(
                                "Skipped reload of [{}] from {}: failed to parse as {} ({})",
                                key_for_closure,
                                path,
                                std::any::type_name::<T>(),
                                e
                            )),
                        }
                    }
                }),
            },
        );
        self
    }

    /// retrieve a section registered via `configure_reload::<T>()`
    pub fn get_reload<T: 'static + Send + Sync>(&self) -> Option<Arc<ArcSwap<T>>> {
        self._inner.get(&TypeId::of::<ArcSwap<T>>())?.clone().downcast::<ArcSwap<T>>().ok()
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
                match std::fs::read_to_string(get_real_path(&path)) {
                    Ok(data) => match data.parse::<toml::Table>() {
                        Ok(table) => {
                            tables.insert(path, table);
                        }
                        Err(e) => LOGGER::warn(format!("Skipped reload of {}: failed to parse ({})", path, e)),
                    },
                    Err(_) => LOGGER::warn(format!("Skipped reload of {}: failed to read", path)),
                }
            }
        }
        let tables = self._toml_tables.read().unwrap();
        for entry in self._reload_sections.read().unwrap().values() {
            (entry.reload_fn)(&tables, &self._inner);
        }
    }
}
