use crate::{dependcy_injection::DependcyInjectableService, logging::LOGGER};
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
    pub fn contains<T: 'static + Send + Sync>(&self) -> bool {
        self._inner.contains_key(&TypeId::of::<T>())
    }
    pub fn insert<T: 'static + Send + Sync>(&mut self, config: T)
    where
        T: Clone + Send + Sync + 'static,
    {
        if self._inner.contains_key(&TypeId::of::<T>()) {
            LOGGER::warn(format!("Replace configuration of type {} already exists", std::any::type_name::<T>()));
        }
        self._inner.insert(TypeId::of::<T>(), Arc::new(config));
    }
    pub fn get<T: 'static + Send + Sync>(&self) -> Option<Arc<T>>
    where
        T: Clone + Send + Sync + 'static,
    {
        match self._inner.get(&TypeId::of::<T>()) {
            Some(config) => config.clone().downcast::<T>().ok(),
            None => None,
        }
    }
    pub fn load_main_toml(&mut self) {
        self.add_toml_cfg(".\\appsettings.toml");
    }
    pub fn add_toml_cfg(&mut self, path: impl AsRef<str>) -> &mut Self {
        match std::fs::read_to_string(path.as_ref()) {
            Ok(data) => match data.parse() {
                Ok(toml_table) => self._toml_tables.insert(path.as_ref().to_string(), toml_table),
                Err(e) => panic!("Failed to parse {} file {}", path.as_ref(), e),
            },
            Err(_) => {
                LOGGER::warn(format!("Failed to read configuration file {}", path.as_ref()));
                None
            }
        };
        self
    }
    pub fn configure<T>(&mut self, section: impl AsRef<str>) -> &mut Self
    where
        T: DeserializeOwned + Default + Debug + Clone + Send + Sync + 'static,
    {
        let key = section.as_ref();
        let mut config = T::default();
        for (path, table) in self._toml_tables.iter() {
            let Some(value) = table.get(key) else {
                continue;
            };
            let value_cfg: Result<T, toml::de::Error> = value.clone().try_into();
            match value_cfg {
                Ok(val) => config = val,
                Err(_) => {
                    panic!("Failed to parse [{}] of toml {} as {}", key, path, std::any::type_name::<T>());
                }
            }
        }
        self.insert(config);
        self
    }
}
