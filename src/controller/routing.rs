use std::collections::HashMap;

use crate::controller::{ActionResult, BindError};
use crate::services::routing::RouteAttributeMap;

#[async_trait::async_trait]
pub trait Routing: Send + 'static {
    async fn routing(&mut self, method_name: &'static str) -> Result<Box<dyn ActionResult + '_>, RoutingError>;
}

/// A failure to dispatch an action or bind its arguments.
#[derive(Debug)]
pub enum RoutingError {
    ActionNotFound,
    Bind(BindError),
}

impl From<BindError> for RoutingError {
    fn from(error: BindError) -> Self {
        Self::Bind(error)
    }
}

#[derive(Debug)]
pub struct ActionRoute {
    pub action_name: &'static str,
    pub method: Vec<&'static str>,
    pub route: &'static str,
    pub route_attribute_map: RouteAttributeMap,
}

impl ActionRoute {
    pub fn new(action_name: &'static str, method: Vec<&'static str>, route: &'static str) -> Self {
        Self {
            action_name,
            method,
            route,
            route_attribute_map: HashMap::default(),
        }
    }

    /// Sets an attribute of this route, an attribute with the same key is replaced.
    /// The value is stored as its own type, `RouteAttribute::get::<T>` reads it back with exactly that type.
    pub fn with_attribute<V: std::any::Any + Send + Sync + 'static>(mut self, key: &'static str, value: V) -> Self {
        self.route_attribute_map.insert(key, Box::new(value));
        self
    }
}
