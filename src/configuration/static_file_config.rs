#[derive(Clone)]
pub struct StaticFileConfiguration {
    pub static_files_directory: String,
}
impl Default for StaticFileConfiguration {
    fn default() -> Self {
        Self {
            static_files_directory: "wwwroot".to_string(),
        }
    }
}