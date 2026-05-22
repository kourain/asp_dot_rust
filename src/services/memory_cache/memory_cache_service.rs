#[derive(Default)]
pub struct MemoryCacheService {
    _inner: dashmap::DashMap<std::any::TypeId, dashmap::DashMap<String, Box<dyn std::any::Any + Send + Sync>>>,
}
impl MemoryCacheService {
    pub fn set<T: 'static + Send + Sync>(&self, key: &str, value: T)
    where
        T: 'static + Send + Sync + Clone,
    {
        let type_id = std::any::TypeId::of::<T>();
        let type_map = self._inner.entry(type_id).or_insert_with(dashmap::DashMap::new);
        type_map.insert(key.to_string(), Box::new(value));
    }
    pub fn get<T: 'static + Send + Sync>(&self, key: &str) -> Option<T>
    where
        T: Clone,
    {
        let type_id = std::any::TypeId::of::<T>();
        if let Some(type_map) = self._inner.get(&type_id) {
            if let Some(value) = type_map.get(key) {
                return value.downcast_ref::<T>().cloned();
            }
        }
        None
    }
    pub fn remove<T: 'static + Send + Sync>(&self, key: &str) {
        let type_id = std::any::TypeId::of::<T>();
        if let Some(type_map) = self._inner.get(&type_id) {
            type_map.remove(key);
        }
    }
    pub fn clear<T: 'static + Send + Sync>(&self) {
        let type_id = std::any::TypeId::of::<T>();
        if let Some(type_map) = self._inner.get(&type_id) {
            type_map.clear();
        }
    }
    pub fn get_or_update<T: 'static + Send + Sync, F: FnOnce() -> T>(&self, key: &str, value_factory: F) -> T
    where
        T: Clone,
    {
        let type_id = std::any::TypeId::of::<T>();
        let type_map = self._inner.entry(type_id).or_insert_with(dashmap::DashMap::new);
        let entry = type_map.entry(key.to_string()).or_insert_with(|| Box::new(value_factory()));
        entry.downcast_ref::<T>().cloned().unwrap()
    }
    pub async fn get_or_update_async<T: 'static + Send + Sync, F: AsyncFnOnce() -> T>(&self, key: &str, value_factory: F) -> T
    where
        T: Clone,
    {
        let type_id = std::any::TypeId::of::<T>();
        let type_map = self._inner.entry(type_id).or_insert_with(dashmap::DashMap::new);
        if !type_map.contains_key(key) {
            let value = value_factory().await;
            type_map.insert(key.to_string(), Box::new(value.clone()));
            return value;
        }
        type_map.get(key).unwrap().downcast_ref::<T>().cloned().unwrap()
    }
}
