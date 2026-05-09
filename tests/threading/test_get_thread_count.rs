
#[test]
fn test_get_thread_count() {
    let count = asp_dot_rust::threading::info::get_thread_count();
    eprintln!("Available threads: {}", count);
    assert!(count > 0);
}