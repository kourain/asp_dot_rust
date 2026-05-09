#![allow(dead_code)]

use std::{any::{Any, TypeId}, collections::HashMap, sync::Arc};

use crate::Application;

type ControllerFactory = Box<dyn Fn() -> Arc<dyn Any + Send + Sync> + Send + Sync>;
pub struct ControllerRegistry {
    controllers: HashMap<TypeId, ControllerFactory>,
}
impl ControllerRegistry {
    pub fn new() -> Self {
        Self {
            controllers: HashMap::new(),
        }
    }

    pub fn add_controller<T, F>(&mut self, factory: F)
    where
        T: Send + Sync + 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        self.controllers
            .insert(TypeId::of::<T>(), Box::new(move || Arc::new(factory())));
    }

    pub fn get_controller<T>(&self) -> Option<Arc<T>>
    where
        T: Send + Sync + 'static,
    {
        let type_id = TypeId::of::<T>();

        let factory = self.controllers.get(&type_id)?;
        Arc::downcast::<T>((factory)()).ok()
    }
}
impl Application{
    fn add_controllers(&mut self) -> &mut Self {
        // Here you can add your controllers to the application
        // For example:
        // self.controllers.push(Box::new(YourController::new()));
        self
    }
}