use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, atomic::AtomicBool},
};

pub(crate) struct ShareMutPtr<T>(*mut T, Arc<AtomicBool>)
where
    T: Send + Sync + 'static;

unsafe impl<T: Send + Sync + 'static> Send for ShareMutPtr<T> {}
unsafe impl<T: Send + Sync + 'static> Sync for ShareMutPtr<T> {}

impl<T: Send + Sync + 'static> ShareMutPtr<T> {
    pub fn new(value: &mut T, state: Arc<AtomicBool>) -> Self {
        Self(value as *mut T, state)
    }
}

impl<T: Send + Sync + 'static> Deref for ShareMutPtr<T> {
    type Target = T;

    #[track_caller]
    fn deref(&self) -> &Self::Target {
        if !self.1.load(std::sync::atomic::Ordering::Relaxed) {
            let type_name = std::any::type_name::<T>();
            panic!("{} of ShareMutPtr<T>: Target already drop", type_name);
        }
        unsafe { &*self.0 }
    }
}

impl<T: Send + Sync + 'static> DerefMut for ShareMutPtr<T> {
    #[track_caller]
    fn deref_mut(&mut self) -> &mut Self::Target {
        if !self.1.load(std::sync::atomic::Ordering::Relaxed) {
            let type_name = std::any::type_name::<T>();
            panic!("{} of ShareMutPtr<T>: Target already drop", type_name);
        }
        unsafe { &mut *self.0 }
    }
}
