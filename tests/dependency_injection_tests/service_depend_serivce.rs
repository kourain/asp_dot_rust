// --- Circular dependency detection -----------------------------------------
//
// `CycleServiceX` depends on `CycleServiceY` and vice-versa. `#[inject]`
// records this edge via `inventory::submit!`, and `ServiceProviderScope::check_dependency_cycles`
// (invoked from `ApplicationBuilder::build`) must detect and panic on it.
// Since the fix in commit `a37438f`, this check runs in every build profile,
// not just debug builds.

use asp_dot_rust::ApplicationBuilder;
use crate::{dependency_injection_tests::services::*};


#[test]
#[should_panic(expected = "Found circular dependency")]
fn build_panics_on_circular_dependency() {
    let mut builder = ApplicationBuilder::new("TestCircularDependency");
    builder.service.add_singleton::<CycleServiceX>();
    builder.service.add_singleton::<CycleServiceY>();

    // `build()` runs `check_dependency_cycles()` internally and must panic
    // before the application is ever returned.
    let _ = builder.build();
}

#[test]
fn build_succeeds_when_dependency_graph_has_no_cycle() {
    let mut builder = ApplicationBuilder::new("TestAcyclicDependency");
    builder.service.add_singleton::<CounterService>();
    builder.service.add_singleton::<WrapperService>();

    // Must not panic: CounterService -> (no deps), WrapperService -> CounterService.
    let _ = builder.build();
}
