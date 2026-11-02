use std::{any::Any, collections::HashMap, pin::Pin};

use crate::http_context::HttpContext;

pub(crate) type ControllerInvoke = for<'a> fn(&'a mut HttpContext, &'static str) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
/// Key of the attribute that is the request body limit of a route, in bytes (`#[max_body_size(..)]` sets it). Its value is a `usize`.
pub const MAX_BODY_SIZE_ATTRIBUTE: &str = "max_body_size";
pub type RouteAttributeMap = HashMap<&'static str, Box<dyn Any + Send + Sync + 'static>>;

#[derive(Debug)]
pub struct Route {
    pub controller_name: &'static str,
    pub controller_type_name: &'static str,
    pub action_name: &'static str,
    pub(crate) route_attr: RouteAttribute,
    pub(crate) invoke_async: ControllerInvoke,
}
impl Route {
    /// The attributes of the route, set with `#[attr(key = value)]` and `#[max_body_size(bytes)]`.
    pub fn attributes(&self) -> &RouteAttribute {
        &self.route_attr
    }
}
#[derive(Debug)]
pub struct RouteAttribute(RouteAttributeMap);
impl RouteAttribute {
    pub(crate) fn new(val: RouteAttributeMap) -> Self {
        Self(val)
    }
    /// The value of `key` when it was stored as exactly the type `T`, `None` when there is no such attribute or it is of another type.
    pub fn get<T: Send + Sync + Clone + 'static>(&self, key: &str) -> Option<T> {
        if let Some(val) = self.0.get(key) {
            return val.downcast_ref::<T>().cloned();
        }
        None
    }

    pub fn contains(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
