//! The first `LOGGER` call used to allocate ~64 MB (a broadcast channel with 1_000_000
//! pre-allocated slots). Linux only: reads the resident set size from /proc.
//!
//! This file must keep a single test, see `logging_no_runtime.rs`.

#[cfg(target_os = "linux")]
fn rss_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .find(|l| l.starts_with("VmRSS"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse().ok())
        .expect("VmRSS not found")
}

// A runtime is used on purpose so this test only measures memory (the no-runtime case is
// covered by `logging_no_runtime.rs`).
#[tokio::test]
#[cfg(target_os = "linux")]
async fn first_log_call_does_not_allocate_tens_of_megabytes() {
    let before = rss_kb();
    asp_dot_rust::logging::LOGGER::info("logging_memory: first log call");
    let delta_mb = (rss_kb().saturating_sub(before)) / 1024;
    assert!(delta_mb < 16, "first LOGGER call grew RSS by {delta_mb} MB");
}
