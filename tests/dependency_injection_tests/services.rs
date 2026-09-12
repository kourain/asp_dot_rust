use std::sync::atomic::{AtomicU32, Ordering};

use asp_dot_rust::dependcy_injection::Serv;
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
    pub inner: Serv<CounterService>,
}

#[inject_require]
impl WrapperService {
    pub fn new(inner: Serv<CounterService>) -> Self {
        WrapperService { inner: inner }
    }
}

pub struct CycleServiceX {}

#[inject_require]
impl CycleServiceX {
    pub fn new(_: Serv<CycleServiceY>) -> Self {
        CycleServiceX {}
    }
}
pub struct CycleServiceY {}

#[inject_require]
impl CycleServiceY {
    pub fn new(_: Serv<CycleServiceX>) -> Self {
        CycleServiceY {}
    }
}
