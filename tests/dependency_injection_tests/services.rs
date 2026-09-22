use std::sync::atomic::{AtomicU32, Ordering};

use asp_dot_rust::dependency_injection::Serv;
use asp_dot_rust_macros::{DependencyInjectableService, inject};

/// A simple service with no dependencies.
pub struct CounterService {
    pub id: u32,
}

#[inject]
impl CounterService {
    pub fn new() -> Self {
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        CounterService {
            id: COUNTER.fetch_add(1, Ordering::SeqCst),
        }
    }
}

/// A service that depends on `CounterService`. Every field is a wrapper
/// type, so `#[derive(DependencyInjectableService)]` builds `inject()` for us
/// -- no `fn new` needed.
#[derive(DependencyInjectableService)]
pub struct WrapperService {
    pub inner: Serv<CounterService>,
}

/// A service mixing an injected dependency with a field that is
/// intentionally not injected. `#[di(default)]` marks the opt-in so the
/// derive builds it with `Default::default()` without emitting the
/// "silently defaulted" warning.
#[derive(DependencyInjectableService)]
pub struct DiDefaultService {
    pub inner: Serv<CounterService>,
    #[di(default)]
    pub label: String,
    #[di(default)]
    pub call_count: u32,
}

pub struct CycleServiceX {}

#[inject]
impl CycleServiceX {
    pub fn new(_: Serv<CycleServiceY>) -> Self {
        CycleServiceX {}
    }
}
pub struct CycleServiceY {}

#[inject]
impl CycleServiceY {
    pub fn new(_: Serv<CycleServiceX>) -> Self {
        CycleServiceY {}
    }
}
