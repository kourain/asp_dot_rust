use std::{any::Any, sync::Arc};

pub(crate) type ConfigurationService = std::collections::HashMap<String, Box<dyn Any + Send + Sync>>;
pub type ApplicationConfiguration = Arc<ConfigurationService>;
