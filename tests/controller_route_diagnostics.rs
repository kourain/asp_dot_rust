use asp_dot_rust::{
    ApplicationBuilder, api_controller,
    controller::{ActionResult, get},
    http_context::HttpContextRef,
    macros::{controller_inject, controller_route},
};
use std::{
    panic::AssertUnwindSafe,
    sync::{Arc, Mutex},
};

mod first {
    use asp_dot_rust::controller::results::ok;

    use super::*;
    api_controller!(FirstController {});

    pub const ROUTE_LINE: u32 = line!() + 8;
    #[controller_route("/conflict")]
    #[controller_inject]
    impl FirstController {
        fn new(http_context: HttpContextRef) -> Self {
            Self { http_context }
        }

        #[get("/{id}")]
        async fn get(&self) -> impl ActionResult {
            ok("first")
        }
    }
}

mod second {
    use asp_dot_rust::controller::results::ok;

    use super::*;
    api_controller!(SecondController {});

    pub const ROUTE_LINE: u32 = line!() + 8;
    #[controller_route("/conflict")]
    #[controller_inject]
    impl SecondController {
        fn new(http_context: HttpContextRef) -> Self {
            Self { http_context }
        }

        #[get("/{name}")]
        async fn get(&self) -> impl ActionResult {
            ok("second")
        }
    }
}

#[test]
fn route_conflicts_report_the_action_route_source() {
    // This integration-test binary has one test, so the temporary panic hook
    // cannot capture an unrelated test's panic.
    let location = Arc::new(Mutex::new(None));
    let captured = location.clone();
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(location) = info.location() {
            *captured.lock().unwrap() = Some((location.file().to_owned(), location.line()));
        }
    }));
    let error = std::panic::catch_unwind(AssertUnwindSafe(|| {
        ApplicationBuilder::new("route_diagnostics").add_controllers();
    }));
    std::panic::set_hook(previous_hook);

    let error = error.expect_err("conflicting routes should fail while registering controllers");
    let message = error.downcast_ref::<String>().expect("expected a route diagnostic");
    assert!(message.contains("Failed to insert route") && message.contains("Controller::get"), "{message}");
    let (file, line) = location.lock().unwrap().clone().expect("expected a panic location");
    assert_eq!(file, file!());
    assert!([first::ROUTE_LINE, second::ROUTE_LINE].contains(&line), "panic should point to a route attribute, got {file}:{line}");
}
