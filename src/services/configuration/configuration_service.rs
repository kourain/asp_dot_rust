use crate::{dependcy_injection::DependcyInjectableService, logging::LOGGER, utils::get_real_path};
use serde::de::DeserializeOwned;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt::Debug,
    sync::Arc,
};

pub struct ConfigurationService {
    _inner: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
    _toml_tables: HashMap<String, toml::Table>,
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
            _toml_tables: HashMap::new(),
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

    /// load default toml configuration file, if the file does not exist, it will be ignored
    pub(crate) fn load_default_toml(&mut self) {
        self.add_optional_toml_cfg("./appsettings.toml");
    }

    /// add a toml configuration file, if the file does not exist, it will panic
    pub fn add_toml_cfg(&mut self, path: impl AsRef<str>) -> &mut Self {
        match std::fs::read_to_string(get_real_path(&path).unwrap()) {
            Ok(data) => match data.parse() {
                Ok(toml_table) => self._toml_tables.insert(path.as_ref().to_string(), toml_table),
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
        match get_real_path(&path) {
            Some(value) => {
                match std::fs::read_to_string(value) {
                    Ok(data) => match data.parse() {
                        Ok(toml_table) => self._toml_tables.insert(path.as_ref().to_string(), toml_table),
                        Err(e) => panic!("Failed to parse {} file {}", path.as_ref(), e),
                    },
                    Err(_) => {
                        LOGGER::warn(format!("Failed to read configuration file {}", path.as_ref()));
                        None
                    }
                };
            }
            None => {
                LOGGER::warn(format!("Failed to find configuration file {}", path.as_ref()));
            }
        }
        self
    }

    /// configure a configuration of type T from the toml configuration file, if the configuration of type T does not exist, it will panic
    pub fn configure<T>(&mut self, section: impl AsRef<str>) -> &mut Self
    where
        T: DeserializeOwned + Default + Debug + Clone + Send + Sync + 'static,
    {
        let key = section.as_ref();
        let mut config = T::default();
        let mut has_value = false;
        for (path, table) in self._toml_tables.iter() {
            let Some(value) = table.get(key) else {
                continue;
            };
            let value_cfg: Result<T, toml::de::Error> = value.clone().try_into();
            match value_cfg {
                Ok(val) => {
                    config = val;
                    has_value = true;
                }
                Err(_) => {
                    panic!("Failed to parse [{}] of toml {} as {}", key, path, std::any::type_name::<T>());
                }
            }
        }
        if has_value {
            LOGGER::trace(format!("Add TOML Config for: {}", std::any::type_name::<T>()));
            LOGGER::verbose(format!("{} = {:#?}", std::any::type_name::<T>(), config.clone()));
            self.insert(config);
        } else {
            panic!("Failed to find [{}] in any loaded toml configuration", key);
        }
        self
    }

    /// configure a configuration of type T from the toml configuration file, if the configuration of type T does not exist, it will be ignored
    pub fn configure_optional<T>(&mut self, section: impl AsRef<str>) -> &mut Self
    where
        T: DeserializeOwned + Default + Debug + Clone + Send + Sync + 'static,
    {
        let key = section.as_ref();
        let mut config = T::default();
        let mut has_value = false;
        for (path, table) in self._toml_tables.iter() {
            let Some(value) = table.get(key) else {
                continue;
            };
            let value_cfg: Result<T, toml::de::Error> = value.clone().try_into();
            match value_cfg {
                Ok(val) => {
                    config = val;
                    has_value = true;
                }
                Err(_) => {
                    panic!("Failed to parse [{}] of toml {} as {}", key, path, std::any::type_name::<T>());
                }
            }
        }
        if has_value {
            LOGGER::trace(format!("Add TOML Config for: {}", std::any::type_name::<T>()));
            LOGGER::verbose(format!("{} = {:#?}", std::any::type_name::<T>(), config.clone()));
            self.insert(config);
        } else {
            LOGGER::warn(format!("Failed to find [{}] in any loaded toml configuration", key));
        }
        self
    }
}
