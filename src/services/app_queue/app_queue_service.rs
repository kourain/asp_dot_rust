use crate::{MutexAsync, services::Service};
use std::{
    any::{Any, TypeId},
    collections::{HashMap, VecDeque}
};

#[derive(Default)]
pub struct AppQueueService {
    _queue: MutexAsync<HashMap<(TypeId, TypeId), VecDeque<Box<dyn Any + Send + Sync>>>>,
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
        let mut lock = self._queue.lock().await;
        if !lock.contains_key(&queue_key) {
            let mut new = VecDeque::new();
            new.push_back(value);
            lock.insert(queue_key, VecDeque::new());
        } else {
            lock.get_mut(&queue_key).unwrap().push_back(Box::new(value));
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
        let mut lock = self._queue.lock().await;
        if let Some(queue) = lock.get_mut(&queue_key) {
            while result.len() < count {
                match queue.pop_front() {
                    Some(value) => {
                        match value.downcast::<V>() {
                            Ok(boxed) => {
                                let item: V = *boxed;
                                result.push(item);
                            }
                            Err(_) => {
                                panic!("Queue: cast failed {} {}", std::any::type_name::<S>(), std::any::type_name::<V>());
                            }
                        }
                    }
                    None => break, // queue empty
                }
            }
        }
        return result;
    }
}
