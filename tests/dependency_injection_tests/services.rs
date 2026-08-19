use std::sync::{Arc, atomic::{AtomicU32, Ordering}};

use asp_dot_rust_macros::inject_require;


/// A simple service with no dependencies.
pub struct CounterService {
    pub id: u32,
}

#[inject_require]
impl CounterService {
    pub fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        CounterService {
            id: COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }
}

/// A service that depends on `CounterService`
pub struct WrapperService {
    pub inner: Arc<CounterService>,
}

#[inject_require]
impl WrapperService {
    pub fn new(inner: Arc<CounterService>) -> Self {
        WrapperService { inner }
    }
}

pub struct CycleServiceX {
    pub y: Arc<CycleServiceY>,
}

#[inject_require]
impl CycleServiceX {
    pub fn new(y: Arc<CycleServiceY>) -> Self {
        CycleServiceX { y }
    }
}
pub struct CycleServiceY {
    pub x: Arc<CycleServiceX>,
}

#[inject_require]
impl CycleServiceY {
    pub fn new(x: Arc<CycleServiceX>) -> Self {
        CycleServiceY { x }
    }
}
