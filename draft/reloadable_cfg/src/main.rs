use std::sync::Arc;

use crate::{code::ReloadFileConfig, sv::Serv};

mod bg_sv;
mod code;
mod sv;

#[tokio::main]
async fn main() {
    let regex = regex::Regex::new(r"{cwd}appsettings(dev|prod|stag)\.toml$").unwrap();
    let mut sv = sv::cfg::CfgService::default();
    sv.insert::<ReloadFileConfig>(|cfg| {
        cfg.list_files = vec!["appsettings.toml", "appsettings.dev.toml", "appsettings.prod.toml", "appsettings.stag.toml"];
    });
    sv.add_toml_file("appsettings.toml", true);
    sv.configure_entry::<code::TestCfg>("data", sv::cfg::CfgEntryAllowReloadFile::AllowedRegexPattern(regex));
    let serv = Serv(Arc::new(sv));
    let file_system_watcher = bg_sv::file_system_watcher::FileSystemWatcher::new(serv.clone());
    file_system_watcher.invoke_async().await;
}
