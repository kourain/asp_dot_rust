use std::{env, path::{Path, PathBuf}};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestCfg {
    test_field: u32,
    test_field2: String,
}

#[derive(Default)]
pub struct ReloadFileConfig {
    pub list_files: Vec<&'static str>,
}

pub fn make_absolute(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();

    if path.is_absolute() {
        // Đã absolute rồi → giữ nguyên
        path.to_path_buf()
    } else {
        // Lấy thư mục hiện tại + nối với path tương đối
        env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join(path)
    }
}
