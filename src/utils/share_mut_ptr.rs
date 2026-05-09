use std::ops::{Deref, DerefMut};


pub struct ShareMutPtr<T>(*mut T);

unsafe impl<T> Send for ShareMutPtr<T> {}
unsafe impl<T> Sync for ShareMutPtr<T> {}

impl<T> ShareMutPtr<T> {
    pub fn new(write_stream: &mut T) -> Self {
        Self(write_stream as *mut T)
    }
}

impl<T> Deref for ShareMutPtr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<T> DerefMut for ShareMutPtr<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}