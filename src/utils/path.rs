/// get real path of a file(ignore case), if the file does not exist, return the original path
pub fn get_real_path(path: impl AsRef<str>) -> String {
    let path = path.as_ref();
    let path = std::path::Path::new(path);
    match std::fs::canonicalize(path) {
        Ok(p) => p.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

/// check if the path is in the folder
pub fn is_path_in_folder(path: impl AsRef<str>, folder: impl AsRef<str>) -> bool {
    let path = path.as_ref();
    let folder = folder.as_ref();
    let path = std::path::Path::new(path);
    let folder = std::path::Path::new(folder);
    match std::fs::canonicalize(path) {
        Ok(p) => {
            if p.starts_with(folder) {
                true
            } else {
                false
            }
        }
        Err(_) => {
            if path.starts_with(folder) {
                true
            } else {
                false
            }
        }
    }
}

/// get current executable folder path
pub fn get_executable_folder() -> String {
    let exe_path = std::env::current_exe().unwrap();
    let exe_folder = exe_path.parent().unwrap();
    exe_folder.to_string_lossy().to_string()
}
