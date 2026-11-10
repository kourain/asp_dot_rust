use std::{
    collections::{BTreeSet, HashSet},
    path::PathBuf,
    time::Duration,
};

use async_trait::async_trait;
use notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, new_debouncer};
use tokio::sync::mpsc;

use crate::{
    dependency_injection::{DependencyInjectableService, Serv},
    hosted_service::BackGroundService,
    logging::LOGGER,
    services::{configuration::ConfigurationService, service_provider::service_provider_scope::ServiceProviderScope},
};

/// Watches the TOML files of `ConfigurationService` and reloads the configuration when they change.
pub struct ConfigurationWatcherBackgroundService {
    configuration: Serv<ConfigurationService>,
}

#[async_trait]
impl DependencyInjectableService for ConfigurationWatcherBackgroundService {
    async fn inject(service_scope: &ServiceProviderScope) -> Self {
        Self {
            configuration: service_scope.get_service::<ConfigurationService>().await,
        }
    }
}

#[async_trait]
impl BackGroundService for ConfigurationWatcherBackgroundService {
    async fn invoke_async(&mut self) {
        let (tx, mut rx) = mpsc::channel(32);

        // batch event in 2s -> callback
        // tick_rate = None -> 1/4 timeout -> 500ms
        let mut debouncer = match new_debouncer(Duration::from_secs(2), None, move |result: DebounceEventResult| {
            let _ = tx.blocking_send(result);
        }) {
            Ok(debouncer) => debouncer,
            Err(e) => {
                LOGGER::warn(format!("Configuration is not reloaded, failed to start the file watcher: {}", e));
                return;
            }
        };

        // Watch the folder of each file, not the file: a file replaced by an editor
        // (temp file + rename) or created later is still seen.
        let folders: BTreeSet<PathBuf> = self
            .configuration
            .watched_files()
            .iter()
            .filter_map(|file| file.parent().map(|folder| folder.to_path_buf()))
            .collect();
        for folder in folders {
            if let Err(e) = debouncer.watch(&folder, RecursiveMode::NonRecursive) {
                LOGGER::warn(format!("Failed to watch configuration folder {}: {:?}", folder.display(), e));
            }
        }

        while let Some(result) = rx.recv().await {
            match result {
                Ok(events) => {
                    let mut changed_file = HashSet::new();
                    for event in events {
                        match &event.kind {
                            EventKind::Create(_) | EventKind::Modify(_) => {
                                for path in &event.paths {
                                    changed_file.insert(path.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                    self.configuration.reload_toml_file_paths(changed_file).await;
                }
                Err(errors) => {
                    for e in errors {
                        LOGGER::warn(format!("Configuration file watcher error: {:?}", e));
                    }
                }
            }
        }
    }
}
