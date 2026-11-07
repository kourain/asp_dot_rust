use std::{collections::HashMap, ops::Deref, sync::Arc};

use arc_swap::ArcSwap;

pub type BoxAny = Box<dyn std::any::Any + Send + Sync + 'static>;
pub type EntryValue<T> = Arc<Option<ArcSwap<T>>>;
pub enum CfgEntryAllowReloadType {
    NOTALLOWED,
    ALLOWED(Vec<&'static str>),
    Any,
}

pub struct CfgReload<T>(pub(crate) EntryValue<T>);
impl<T> CfgReload<T> {
    pub fn capture(&self) -> Cfg<T> {
        match self.0.as_ref() {
            Some(arc_swap) => Cfg(Some(arc_swap.load_full())),
            None => Cfg(None),
        }
    }
}
pub struct Cfg<T>(pub(crate) Option<Arc<T>>);
impl<T: Default> Cfg<T> {
    pub fn unwrap_or_default(&self) -> CfgRequired<T> {
        CfgRequired(self.0.as_ref().map(|arc| arc.clone()).unwrap_or_else(|| Arc::new(T::default())))
    }
}
impl<T> Deref for Cfg<T> {
    type Target = Option<Arc<T>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub struct CfgRequired<T>(pub(crate) Arc<T>);
impl<T> Deref for CfgRequired<T> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
pub struct CfgEntry {
    value: EntryValue<BoxAny>,
    updatetor: Option<Box<dyn Fn(&toml::Value) -> Option<BoxAny> + Send + Sync>>,
    allow_file: CfgEntryAllowReloadType,
}
impl CfgEntry {
    pub fn new<T>(allow_update: bool, update_key: &'static str, allow_file: CfgEntryAllowReloadType) -> Self
    where
        T: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
    {
        let value = Arc::new(Some(ArcSwap::from_pointee(Box::new(T::default()) as BoxAny)));
        let updatetor = if allow_update {
            Some(Box::new(move |toml_value: &toml::Value| {
                if let Some(new_value) = toml_value.get(update_key).and_then(|v| v.clone().try_into::<T>().ok()) {
                    Some(Box::new(new_value) as BoxAny)
                } else {
                    None
                }
            }) as Box<dyn Fn(&toml::Value) -> Option<BoxAny> + Send + Sync>)
        } else {
            None
        };
        Self { value, updatetor, allow_file }
    }
    pub fn downcast<T>(&self) -> CfgReload<T>
    where
        T: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
    {
        CfgReload(self.value.clone().map(|arc_swap| {
            let arc = arc_swap.load_full();
            let downcasted_arc = arc.downcast::<T>().unwrap_or_else(|_| Arc::new(T::default()));
            ArcSwap::from(Arc::new(downcasted_arc))
        }))
    }
}
#[derive(Default)]
pub struct TomlEntry {
    file_path: &'static str,
    watch_file_change: bool,
}

#[derive(Default)]
pub struct CfgService {
    _inner: HashMap<&'static str, CfgEntry>,
    _toml_file_path: TomlEntry,
}
impl CfgService {
    pub fn configure<T>(&mut self, key: &'static str)
    where
        T: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
    {
        let entry = CfgEntry::new::<T>(true, key, CfgEntryAllowReloadType::Any);
        let type_name = std::any::type_name::<T>();
        self._inner.insert(type_name, entry);
    }
    pub fn add_toml_file(&mut self, file_path: &'static str, watch_file_change: bool) {
        self._toml_file_path = TomlEntry { file_path, watch_file_change };
    }
    pub(crate) fn add_cfg_entry<T>(&mut self, allow_update: bool, update_key: &'static str, allow_file: CfgEntryAllowReloadType)
    where
        T: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
    {
        let entry = CfgEntry::new::<T>(allow_update, update_key, allow_file);
        let type_name = std::any::type_name::<T>();
        self._inner.insert(type_name, entry);
    }
    pub(crate) fn get<T>(&self) -> CfgReload<T>
    where
        T: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();
        match self._inner.get(type_name) {
            Some(entry) => entry.downcast::<T>(),
            None => CfgReload(Arc::new(None)),
        }
    }
}
