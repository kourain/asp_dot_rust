use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    path::{Path, PathBuf},
    sync::Arc,
};

use arc_swap::ArcSwap;
use regex::Regex;

pub type BoxAny = Arc<dyn std::any::Any + Send + Sync + 'static>;
pub type EntryValue<T> = Arc<Option<ArcSwap<T>>>;
pub enum CfgEntryAllowReloadFile {
    NotAllowed,
    Allowed(Vec<&'static str>),
    AllowedRegexPattern(Regex),
    AllowedAll,
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
    type_name: &'static str,
    updatetor: Option<Box<dyn Fn(&toml::Value) -> Option<BoxAny> + Send + Sync>>,
    allow_file: CfgEntryAllowReloadFile,
}
impl CfgEntry {
    pub fn new<T>(toml_key: &'static str, allow_file: CfgEntryAllowReloadFile) -> Self
    where
        T: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
    {
        let value = T::default();
        Self::from_value(value, toml_key, allow_file)
    }
    pub fn from_static_entry<T>(value: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        let value = Arc::new(Some(ArcSwap::from_pointee(Arc::new(value) as BoxAny)));
        Self {
            value,
            type_name: std::any::type_name::<T>(),
            updatetor: None,
            allow_file: CfgEntryAllowReloadFile::NotAllowed,
        }
    }
    pub fn from_value<T>(value: T, toml_key: &'static str, allow_file: CfgEntryAllowReloadFile) -> Self
    where
        T: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
    {
        let value = Arc::new(Some(ArcSwap::from_pointee(Arc::new(value) as BoxAny)));
        let updatetor = Some(Box::new(move |toml_value: &toml::Value| {
            if let Some(new_value) = toml_value.get(toml_key).and_then(|v| v.clone().try_into::<T>().ok()) {
                return Some(Arc::new(new_value) as BoxAny);
            }
            None
        }) as Box<dyn Fn(&toml::Value) -> Option<BoxAny> + Send + Sync>);
        Self {
            value,
            type_name: std::any::type_name::<T>(),
            updatetor,
            allow_file,
        }
    }
    pub fn update_from_toml(&self, toml_value: &toml::Value, full_path_file: &str) {
        if let Some(updatetor) = &self.updatetor {
            match self.allow_file {
                CfgEntryAllowReloadFile::NotAllowed => return,
                CfgEntryAllowReloadFile::Allowed(ref allowed_files) => {
                    if !allowed_files.contains(&full_path_file) {
                        return;
                    }
                }
                CfgEntryAllowReloadFile::AllowedRegexPattern(ref pattern) => {
                    if !pattern.is_match(full_path_file) {
                        return;
                    }
                }
                CfgEntryAllowReloadFile::AllowedAll => {}
            }
            if let Some(new_value) = updatetor(toml_value) {
                if let Some(arc_swap) = self.value.as_ref() {
                    arc_swap.store(Arc::new(new_value));
                }
            }
        }
    }
    pub fn downcast<T>(&self) -> CfgReload<T>
    where
        T: Default + Send + Sync + 'static,
    {
        CfgReload(Arc::new(self.value.as_ref().as_ref().map(|arc_swap| {
            let arc = arc_swap.load_full();
            let any_arc = arc.as_ref().clone();
            let downcasted_arc = any_arc.downcast::<T>().unwrap_or_else(|_| Arc::new(T::default()));
            ArcSwap::from(downcasted_arc)
        })))
    }
}

#[derive(Default)]
pub struct CfgService {
    _inner: HashMap<&'static str, CfgEntry>,
    _toml_file_path: HashMap<PathBuf, bool>,
}
impl CfgService {
    pub fn configure<T>(&mut self, key: &'static str)
    where
        T: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
    {
        self.configure_entry::<T>(key, CfgEntryAllowReloadFile::NotAllowed);
    }
    pub fn add_toml_file(&mut self, file_path: impl AsRef<Path>, watch_file_change: bool) {
        self._toml_file_path.insert(crate::code::make_absolute(file_path), watch_file_change);
    }
    pub(crate) fn configure_entry<T>(&mut self, toml_key: &'static str, allow_file: CfgEntryAllowReloadFile)
    where
        T: serde::de::DeserializeOwned + Default + Send + Sync + 'static,
    {
        let entry = CfgEntry::new::<T>(toml_key, allow_file);
        let type_name = std::any::type_name::<T>();
        self._inner.insert(type_name, entry);
    }
    pub(crate) async fn reload_toml_file_paths(&self, file_paths: HashSet<PathBuf>) {
        for file_path in file_paths {
            self.reload_toml_file_path(&file_path).await;
        }
    }
    pub(crate) async fn reload_toml_file_path(&self, file_path: &PathBuf) {
        if let Some(watch_file_change) = self._toml_file_path.get(file_path) {
            if *watch_file_change {
                if let Ok(toml_content) = tokio::fs::read_to_string(file_path).await {
                    if let Ok(toml_value) = toml::from_str::<toml::Value>(&toml_content) {
                        for entry in self._inner.values() {
                            entry.update_from_toml(&toml_value, file_path.to_str().unwrap_or_default());
                            //debug code
                            print!("updated: {}", entry.type_name);
                        }
                    }
                }
            }
        }
    }
    pub(crate) fn get<T>(&self) -> CfgReload<T>
    where
        T: Default + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();
        match self._inner.get(type_name) {
            Some(entry) => entry.downcast::<T>(),
            None => panic!("CfgService: No entry found for type {}", type_name),
        }
    }
    pub(crate) fn insert<T>(&mut self, custom: impl Fn(&mut T))
    where
        T: Default + Send + Sync + 'static,
    {
        let type_name = std::any::type_name::<T>();
        let mut value: T = Default::default();
        custom(&mut value);
        let entry = CfgEntry::from_static_entry(value);
        self._inner.insert(type_name, entry);
    }
}
