#![allow(deprecated)]
use std::{
    any::Any,
    collections::HashSet,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
};

use arc_swap::ArcSwapOption;
use asp_dot_rust_macros::inject;
use regex::Regex;
use serde::de::DeserializeOwned;
use std::collections::HashMap;

use crate::{configuration::HyperConfig, dependency_injection::CfgReload, logging::LOGGER, utils::path::get_executable_folder};

pub type ArcAny = Arc<dyn Any + Send + Sync + 'static>;
/// Value of an entry: `None` until the entry is loaded. Every `CfgReload` of the entry shares it,
/// so a reload is seen by all of them.
pub type EntryValue<T> = Arc<ArcSwapOption<T>>;

/// Which changed TOML files may update an entry.
pub enum CfgEntryAllowReloadFile {
    /// The entry is never updated from a file (after Load once at app.run().await).
    NotAllowed,
    /// The entry is updated only from these files, a file matches when its path ends with the given path, e.g. `appsettings.toml`.
    AllowedFileNames(Vec<&'static str>),
    /// the entry is updated only from these file_full_paths
    AllowedFullPaths(Vec<PathBuf>),
    /// The entry is updated only from files whose full path matches the pattern.
    AllowedRegexPattern(Regex),
    /// The entry is updated from any file.
    AllowedAll,
}

impl CfgEntryAllowReloadFile {
    pub fn is_allow(&self, full_path_file: &str) -> bool {
        match self {
            CfgEntryAllowReloadFile::NotAllowed => false,
            CfgEntryAllowReloadFile::AllowedFileNames(allowed_files) => allowed_files.iter().any(|file| Path::new(full_path_file).ends_with(file)),
            CfgEntryAllowReloadFile::AllowedFullPaths(allowed_files) => allowed_files.iter().any(|file| full_path_file == file),
            CfgEntryAllowReloadFile::AllowedRegexPattern(pattern) => pattern.is_match(full_path_file),
            CfgEntryAllowReloadFile::AllowedAll => true,
        }
    }
    pub fn is_not_allowed(&self) -> bool {
        match self {
            CfgEntryAllowReloadFile::NotAllowed => true,
            _ => false,
        }
    }
}

pub struct CfgEntry {
    /// an `EntryValue<T>` of the type the entry was made for
    value: ArcAny,
    type_name: &'static str,
    updator: Option<Box<dyn Fn(&toml::Value) + Send + Sync>>,
    allow_file: CfgEntryAllowReloadFile,
}

impl CfgEntry {
    /// An entry bound to the section `toml_key`, its value is `None` until a file loads it.
    pub(crate) fn create_new<T>(toml_key: &'static str, allow_file: CfgEntryAllowReloadFile, default_value: EntryValue<T>) -> Self
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let target = default_value.clone();
        let updator = Box::new(move |toml_value: &toml::Value| {
            // a section that is not in this file keeps the current value
            let Some(section) = toml_value.get(toml_key) else {
                return;
            };
            match section.clone().try_into::<T>() {
                Ok(new_value) => target.store(Some(Arc::new(new_value))),
                Err(e) => LOGGER::warn(format!("Failed to parse [{}] as {}: {}", toml_key, std::any::type_name::<T>(), e)),
            }
        }) as Box<dyn Fn(&toml::Value) + Send + Sync>;
        Self {
            value: default_value as ArcAny,
            type_name: std::any::type_name::<T>(),
            updator: Some(updator),
            allow_file,
        }
    }
    pub fn new<T>(toml_key: &'static str, allow_file: CfgEntryAllowReloadFile) -> Self
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let value: EntryValue<T> = Arc::new(ArcSwapOption::empty());
        Self::create_new(toml_key, allow_file, value)
    }
    pub fn new_with_default_init<T>(toml_key: &'static str, allow_file: CfgEntryAllowReloadFile) -> Self
    where
        T: Default + DeserializeOwned + Send + Sync + 'static,
    {
        let value: EntryValue<T> = Arc::new(ArcSwapOption::new(Some(Arc::new(T::default()))));
        Self::create_new(toml_key, allow_file, value)
    }
    /// An entry with a value that no file changes.
    pub fn from_static_entry<T>(value: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        let value: EntryValue<T> = Arc::new(ArcSwapOption::from_pointee(value));
        Self {
            value: value as ArcAny,
            type_name: std::any::type_name::<T>(),
            updator: None,
            allow_file: CfgEntryAllowReloadFile::NotAllowed,
        }
    }

    pub(crate) fn update_from_toml(&self, toml_value: &toml::Value, full_path_file: &str, is_first_time: bool) {
        if let Some(updatetor) = &self.updator {
            if self.allow_file.is_allow(full_path_file) || (is_first_time && self.allow_file.is_not_allowed()) {
                updatetor(toml_value);
            }
        }
    }

    #[track_caller]
    pub fn downcast<T>(&self) -> CfgReload<T>
    where
        T: Send + Sync + 'static,
    {
        match self.value.clone().downcast::<ArcSwapOption<T>>() {
            Ok(value) => CfgReload(value),
            Err(_) => panic!("Configuration entry of type {} was asked as {}", self.type_name, std::any::type_name::<T>()),
        }
    }
}

#[derive(Default)]
pub struct ConfigurationService {
    _inner: HashMap<&'static str, CfgEntry>,
    /// files in the order they were added, the flag tells if a change of the file reloads the configuration
    _toml_file_path: Vec<(PathBuf, bool)>,
    _is_loaded: AtomicBool,
}

#[inject]
impl ConfigurationService {
    pub fn new() -> Self {
        let mut _self = Self::default();
        _self.add_toml_file(r".\appsetting.toml", true);
        _self.insert::<crate::configuration::StartupArgsConfiguration>(|_| {});
        _self.configure_default_entry::<HyperConfig>(
            "hyper_server",
            CfgEntryAllowReloadFile::AllowedFullPaths(vec![format!("{}\\appsettings.toml", get_executable_folder()).into()]),
        );
        _self
    }
    /// Bind the configuration of type T to the section `key`, it is updated from any file.
    /// Nothing is read here, the value is `None` until `reload_all_async` runs.
    pub fn configure<T>(&mut self, key: &'static str) -> &mut Self
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        self.configure_entry::<T>(key, CfgEntryAllowReloadFile::AllowedAll)
    }

    /// Same as `configure`, `allow_file` tells which files may update the configuration.
    pub fn configure_entry<T>(&mut self, toml_key: &'static str, allow_file: CfgEntryAllowReloadFile) -> &mut Self
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        let entry = CfgEntry::new::<T>(toml_key, allow_file);
        self._inner.insert(std::any::type_name::<T>(), entry);
        self
    }

    pub fn configure_default_entry<T>(&mut self, toml_key: &'static str, allow_file: CfgEntryAllowReloadFile) -> &mut Self
    where
        T: Default + DeserializeOwned + Send + Sync + 'static,
    {
        let entry = CfgEntry::new_with_default_init::<T>(toml_key, allow_file);
        self._inner.insert(std::any::type_name::<T>(), entry);
        self
    }

    /// Add a TOML file, `reload_on_change` tells if a change of the file reloads the configuration.
    /// A file that does not exist is skipped, it is read when it is created later.
    pub fn add_toml_file(&mut self, file_path: impl AsRef<Path>, reload_on_change: bool) -> &mut Self {
        let file_path = std::path::absolute(file_path.as_ref()).unwrap_or_else(|_| file_path.as_ref().to_path_buf());
        match self._toml_file_path.iter_mut().find(|(path, _)| *path == file_path) {
            Some((_, watch)) => *watch = reload_on_change,
            None => self._toml_file_path.push((file_path, reload_on_change)),
        }
        self
    }

    /// Insert a configuration of type T made from `T::default()` and `custom`, no file changes it.
    /// A configuration of type T that already exists is replaced.
    pub fn insert<T>(&mut self, custom: impl Fn(&mut T)) -> &mut Self
    where
        T: Default + Send + Sync + 'static,
    {
        let mut value: T = Default::default();
        custom(&mut value);
        self._inner.insert(std::any::type_name::<T>(), CfgEntry::from_static_entry(value));
        self
    }

    /// update a exist cfg (default if not loaded yet) with a custom function, if not exist insert a new one
    pub fn update<T>(&mut self, custom: impl Fn(&mut T)) -> &mut Self
    where
        T: Default + Clone + Send + Sync + 'static,
    {
        if let Some(instance) = self._inner.get(std::any::type_name::<T>()) {
            let reloader = instance.downcast::<T>();
            let mut new_cfg = (**reloader.capture().unwrap_or_default()).clone();
            custom(&mut new_cfg);
            reloader.0.store(Some(Arc::new(new_cfg)));
        } else {
            self.insert::<T>(custom);
        }
        self
    }

    /// check if a configuration of type T is registered, it may still have no value
    pub fn contains<T>(&self) -> bool
    where
        T: 'static,
    {
        self._inner.contains_key(std::any::type_name::<T>())
    }

    /// The handle of the configuration of type T, the only way to get a configuration.
    /// A configuration that is not registered gives a handle without a value.
    #[track_caller]
    pub fn get<T>(&self) -> CfgReload<T>
    where
        T: Send + Sync + 'static,
    {
        match self._inner.get(std::any::type_name::<T>()) {
            Some(entry) => entry.downcast::<T>(),
            None => panic!(
                "Configuration of type {} was not registered, use `ConfigurationService::configure::<T>(key)`",
                std::any::type_name::<T>()
            ),
        }
    }

    /// paths of the files that reload the configuration when they change
    pub(crate) fn watched_files(&self) -> Vec<PathBuf> {
        self._toml_file_path.iter().filter(|(_, watch)| *watch).map(|(path, _)| path.clone()).collect()
    }

    /// Read every TOML file (watched or not) and load the configurations from them, a file added later wins.
    /// `Application::run` calls it before it builds the middleware pipeline and opens a port.
    pub async fn load_all_async(&self) {
        let is_firsttime = !self._is_loaded.load(std::sync::atomic::Ordering::Relaxed);
        for (file_path, _) in &self._toml_file_path {
            self.load_toml_file(file_path, is_firsttime).await;
        }
        self._is_loaded.store(true, std::sync::atomic::Ordering::Release);
    }

    pub(crate) async fn reload_toml_file_paths(&self, file_paths: HashSet<PathBuf>) {
        for file_path in file_paths {
            self.reload_toml_file_path(&file_path).await;
        }
    }

    pub(crate) async fn reload_toml_file_path(&self, file_path: &PathBuf) {
        if let Some((_, true)) = self._toml_file_path.iter().find(|(path, _)| path == file_path) {
            self.load_toml_file(file_path, false).await;
        }
    }

    /// a file that cannot be read or parsed is skipped, the current values are kept
    async fn load_toml_file(&self, file_path: &Path, is_firsttime: bool) {
        match tokio::fs::read_to_string(file_path).await {
            Ok(toml_content) => match toml::from_str::<toml::Value>(&toml_content) {
                Ok(toml_value) => {
                    for entry in self._inner.values() {
                        entry.update_from_toml(&toml_value, file_path.to_str().unwrap_or_default(), is_firsttime);
                    }
                }
                Err(e) => LOGGER::error(format!("Skipped {}: failed to parse ({})", file_path.display(), e)),
            },
            Err(e) if e.kind() == ErrorKind::NotFound => LOGGER::error(format!("Configuration file {} does not exist, skipped", file_path.display())),
            Err(e) => LOGGER::error(format!("Skipped {}: failed to read ({})", file_path.display(), e)),
        }
    }
}
