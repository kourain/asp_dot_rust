//! Several applications built at the same time in one process (the way the test runner builds them) must not break the controller registry.

use std::sync::{Arc, Barrier};

use asp_dot_rust::{
    ApplicationBuilder, api_controller,
    controller::{ActionResult, get},
    http_context::HttpContextRef,
    logging::LOGGER,
    macros::{controller_inject, controller_route},
};

api_controller!(pub ParallelController {});

#[controller_route]
#[controller_inject]
impl ParallelController {
    fn new(http_context: HttpContextRef) -> Self {
        Self { http_context }
    }

    #[get("/parallel")]
    pub async fn index(&mut self) -> impl ActionResult {
        "ok"
    }
}

#[test]
fn add_controllers_can_run_on_many_threads_at_the_same_time() {
    LOGGER::disable();
    const THREADS: usize = 16;
    for _round in 0..3000 {
        let barrier = Arc::new(Barrier::new(THREADS));
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let mut builder = ApplicationBuilder::new("ParallelRegistration");
                    barrier.wait();
                    builder.add_controllers();
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("add_controllers panicked when it ran at the same time as another one");
        }
    }
}
