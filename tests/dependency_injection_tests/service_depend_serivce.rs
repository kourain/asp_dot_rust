// --- Circular dependency detection -----------------------------------------
//
// `CycleServiceX` depends on `CycleServiceY` and vice-versa. `#[inject]`
// records this edge via `inventory::submit!`. Startup dependency validation
// in `Application::run` must detect and panic on it.
// Since the fix in commit `a37438f`, this check runs in every build profile,
// not just debug builds.

use crate::dependency_injection_tests::services::*;
use asp_dot_rust::ApplicationBuilder;

#[tokio::test]
#[should_panic(expected = "Found circular dependency")]
async fn run_panics_on_circular_dependency() {
    let mut builder = ApplicationBuilder::new("TestCircularDependency");
    builder.service.add_singleton::<CycleServiceX>();
    builder.service.add_singleton::<CycleServiceY>();

    let app = builder.build();
    tokio::time::timeout(std::time::Duration::from_secs(5), app.run()).await.expect("startup should reject the cycle");
}

#[tokio::test]
async fn build_succeeds_when_dependency_graph_has_no_cycle() {
    let mut builder = ApplicationBuilder::new("TestAcyclicDependency");
    builder.service.add_singleton::<CounterService>();
    builder.service.add_singleton::<WrapperService>();

    // Must not panic: CounterService -> (no deps), WrapperService -> CounterService.
    let _ = builder.build();
}
