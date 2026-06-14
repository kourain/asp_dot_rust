use dashmap::DashMap;

use crate::{MutexAsync, services::Service};
use std::{
    any::{Any, TypeId},
    collections::VecDeque,
};

#[derive(Default)]
pub struct AppQueueService {
    _queue: DashMap<(TypeId, TypeId), MutexAsync<VecDeque<Box<dyn Any + Send + Sync>>>>,
}
impl Service for AppQueueService {
    fn name(&self) -> &'static str {
        "App Queue"
    }
}
impl AppQueueService {
    pub async fn add_queue_async<S, V>(self, value: V)
    where
        S: 'static,
        V: Any + Send + Sync,
    {
        let type_s = TypeId::of::<S>();
        let type_v = TypeId::of::<V>();
        let queue_key = (type_s, type_v);
        if !self._queue.contains_key(&queue_key) {
            let mut new = VecDeque::new();
            new.push_back(value);
            self._queue.insert(queue_key, MutexAsync::new(VecDeque::new()));
        } else {
            self._queue.get(&queue_key).unwrap().lock().await.push_back(Box::new(value));
        }
    }
    pub async fn de_queue_async<S, V>(&mut self, count: usize) -> Vec<V>
    where
        S: 'static,
        V: 'static,
    {
        let type_s = TypeId::of::<S>();
        let type_v = TypeId::of::<V>();
        let queue_key = (type_s, type_v);

        let mut result: Vec<V> = Vec::new();
        if let Some(queue) = self._queue.get(&queue_key) {
            let mut queue_lock = queue.lock().await;
            while result.len() < count {
                match queue_lock.pop_front() {
                    Some(value) => match value.downcast::<V>() {
                        Ok(boxed) => {
                            let item: V = *boxed;
                            result.push(item);
                        }
                        Err(_) => {
                            panic!("Queue: cast failed {} {}", std::any::type_name::<S>(), std::any::type_name::<V>());
                        }
                    },
                    None => break, // queue empty
                }
            }
        }
        return result;
    }
}
