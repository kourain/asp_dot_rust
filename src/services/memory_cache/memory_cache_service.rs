use crate::dependency_injection::DependencyInjectableService;

struct CacheItem {
    value: Box<dyn std::any::Any + Send + Sync>,
    expiration: Option<std::time::Instant>,
}

#[derive(Default)]
pub struct MemoryCacheService {
    _inner: dashmap::DashMap<std::any::TypeId, dashmap::DashMap<String, CacheItem>>,
}
impl MemoryCacheService {
    pub fn set<T>(&self, key: &str, value: T, exp_seconds: Option<u64>)
    where
        T: 'static + Send + Sync + Clone + Sized,
    {
        let type_id = std::any::TypeId::of::<T>();
        let type_map = self._inner.entry(type_id).or_default();
        let mut cacheval = CacheItem {
            value: Box::new(value.clone()),
            expiration: None,
        };
        if let Some(seconds) = exp_seconds {
            cacheval.expiration = Some(std::time::Instant::now() + std::time::Duration::from_secs(seconds));
        }
        type_map.insert(key.to_string(), cacheval);
    }
    pub fn get<T>(&self, key: &str) -> Option<T>
    where
        T: 'static + Send + Sync + Clone,
    {
        let type_id = std::any::TypeId::of::<T>();
        if let Some(type_map) = self._inner.get(&type_id)
            && let Some(value) = type_map.get(key)
        {
            return value.value.downcast_ref::<T>().cloned();
        }
        None
    }
    pub fn remove<T>(&self, key: &str)
    where
        T: 'static + Send + Sync,
    {
        let type_id = std::any::TypeId::of::<T>();
        if let Some(type_map) = self._inner.get(&type_id) {
            type_map.remove(key);
        }
    }
    pub fn clear<T>(&self)
    where T: 'static + Send + Sync {
        let type_id = std::any::TypeId::of::<T>();
        if let Some(type_map) = self._inner.get(&type_id) {
            type_map.clear();
        }
    }
    pub fn get_or_update<T, F>(&self, key: &str, value_factory: F, exp_seconds: Option<u64>) -> T
    where
        T: 'static + Send + Sync + Clone,
        F: FnOnce() -> T
    {
        let type_id = std::any::TypeId::of::<T>();
        let type_map = self._inner.entry(type_id).or_default();
        let entry = type_map.entry(key.to_string()).or_insert_with(|| CacheItem {
            value: Box::new(value_factory()),
            expiration: exp_seconds.map(|s| std::time::Instant::now() + std::time::Duration::from_secs(s)),
        });
        entry.value.downcast_ref::<T>().cloned().unwrap()
    }
    pub async fn get_or_update_async<T, F>(&self, key: &str, value_factory: F, exp_seconds: Option<u64>) -> Option<T>
    where
        T: 'static + Send + Sync + Clone,
        F: AsyncFnOnce() -> T,
    {
        let type_id = std::any::TypeId::of::<T>();
        let type_map = self._inner.entry(type_id).or_default();
        if !type_map.contains_key(key) {
            let value = value_factory().await;
            type_map.insert(
                key.to_string(),
                CacheItem {
                    value: Box::new(value.clone()),
                    expiration: exp_seconds.map(|s| std::time::Instant::now() + std::time::Duration::from_secs(s)),
                },
            );
            return Some(value);
        }
        type_map.get(key).unwrap().value.downcast_ref::<T>().cloned()
    }
    pub fn release_expired_cache(&self) {
        for type_map in self._inner.iter() {
            let type_map = type_map.value();
            let keys_to_remove: Vec<String> = type_map
                .iter()
                .filter_map(|entry| {
                    let cache_item = entry.value();
                    if let Some(expiration) = cache_item.expiration
                        && expiration <= std::time::Instant::now()
                    {
                        return Some(entry.key().clone());
                    }
                    None
                })
                .collect();
            for key in keys_to_remove {
                type_map.remove(&key);
            }
        }
    }
}
impl DependencyInjectableService for MemoryCacheService {
    fn inject(_service_scope: &crate::services::service_provider::service_provider_scope::ServiceProviderScope) -> Self
    where
        Self: Sized,
    {
        MemoryCacheService::default()
    }
}
