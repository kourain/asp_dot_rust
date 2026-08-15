use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, atomic::AtomicBool},
};

pub struct ShareMutPtr<T>(*mut T, Arc<AtomicBool>);

unsafe impl<T> Send for ShareMutPtr<T> {}
unsafe impl<T> Sync for ShareMutPtr<T> {}

impl<T> ShareMutPtr<T> {
    pub fn new(value: &mut T) -> Self {
        Self(value as *mut T, Arc::new(AtomicBool::new(false)))
    }
    pub fn new_with_state(value: &mut T, state: Arc<AtomicBool>) -> Self {
        Self(value as *mut T, state)
    }
}

impl<T> Clone for ShareMutPtr<T> {
    fn clone(&self) -> Self {
        Self(self.0, self.1.clone())
    }
}

impl<T> Deref for ShareMutPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        if !self.1.load(std::sync::atomic::Ordering::Relaxed) {
            let type_name = std::any::type_name::<T>();
            panic!("{} of ShareMutPtr<T>: Target already drop", type_name);
        }
        unsafe { &*self.0 }
    }
}

impl<T> DerefMut for ShareMutPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if !self.1.load(std::sync::atomic::Ordering::Relaxed) {
            let type_name = std::any::type_name::<T>();
            panic!("{} of ShareMutPtr<T>: Target already drop", type_name);
        }
        unsafe { &mut *self.0 }
    }
}
