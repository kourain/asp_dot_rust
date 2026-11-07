use std::{ops::Deref, sync::Arc};

mod cfg;

pub struct Serv<T>(Arc<T>);
impl<T> Deref for Serv<Arc<T>> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}