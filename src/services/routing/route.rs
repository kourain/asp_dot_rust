use std::{any::Any, collections::HashMap, pin::Pin};

use crate::http_context::HttpContext;

pub(crate) type ControllerInvoke = for<'a> fn(&'a mut HttpContext, &'static str) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
pub type RouteAttributeMap = HashMap<&'static str, Box<dyn Any + Send + Sync + 'static>>;

#[derive(Debug)]
pub struct Route {
    pub controller_name: &'static str,
    pub controller_type_name: &'static str,
    pub action_name: &'static str,
    pub(crate) route_attr: RouteAttribute,
    pub(crate) invoke_async: ControllerInvoke,
}
#[derive(Debug)]
pub struct RouteAttribute(RouteAttributeMap);
impl RouteAttribute {
    pub(crate) fn new(val: RouteAttributeMap) -> Self {
        Self(val)
    }
    pub fn get<T: Send + Sync + Clone + 'static>(&self, key: &'static str) -> Option<T> {
        if let Some(val) = self.0.get(key) {
            return val.downcast_ref::<T>().cloned();
        }
        None
    }
}
