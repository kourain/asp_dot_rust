use std::{ops::Deref, sync::Arc};

pub mod cfg;

pub struct Serv<T>(pub(crate) Arc<T>);

impl<T> Deref for Serv<T> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> Clone for Serv<T> {
    fn clone(&self) -> Self {
        Serv(self.0.clone())
    }
}
