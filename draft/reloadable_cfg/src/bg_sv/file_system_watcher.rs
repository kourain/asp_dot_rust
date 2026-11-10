use std::{collections::HashSet, time::Duration};

use notify::RecursiveMode;
use notify_debouncer_full::{DebounceEventResult, new_debouncer};
use tokio::sync::mpsc;

use crate::{
    code::ReloadFileConfig,
    sv::{Serv, cfg::CfgService},
};

pub struct FileSystemWatcher {
    cfg: Serv<CfgService>,
}
impl FileSystemWatcher {
    pub fn new(cfg: Serv<CfgService>) -> Self {
        Self { cfg }
    }
    pub async fn invoke_async(&self) {
        let (tx, mut rx) = mpsc::channel(32);

        // batch event in 2s -> callback
        // tick_rate = None -> 1/4 timeout -> 500ms
        let mut debouncer = new_debouncer(Duration::from_secs(2), None, move |result: DebounceEventResult| {
            let _ = tx.blocking_send(result);
        })
        .expect("msg");

        // Watch current folder
        let reload_list = self.cfg.get::<ReloadFileConfig>().capture().unwrap_or_default();
        for reload_file in &reload_list.list_files {
            let absolute_path = crate::code::make_absolute(reload_file);
            if let Err(e) = debouncer.watch(&absolute_path, RecursiveMode::NonRecursive) {
                eprintln!("Error watching file {}: {:?}", absolute_path.display(), e);
            }
        }

        while let Some(result) = rx.recv().await {
            match result {
                Ok(events) => {
                    let mut changed_file = HashSet::new();
                    for event in events {
                        match &event.kind {
                            notify::EventKind::Create(_) => {
                                for path in &event.paths {
                                    changed_file.insert(path.clone());
                                }
                                println!("File created: {:?}", event.paths);
                            }
                            notify::EventKind::Modify(_) => {
                                for path in &event.paths {
                                    changed_file.insert(path.clone());
                                    println!("File {} changed, reloading config...", path.display());
                                }
                            }
                            _ => {}
                        }
                    }
                    self.cfg.reload_toml_file_paths(changed_file).await;
                }
                Err(errors) => {
                    for e in errors {
                        // replace with logger
                        eprintln!("Err: {:?}", e);
                    }
                }
            }
        }
    }
}
