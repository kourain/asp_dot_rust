#![feature(prelude_import)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2024::*;
mod controllers {
    pub mod homecontroller {
        use asp_dot_rust::{
            api_controller, controller::{ActionResult, get, post, put, route},
            controller_route, logging::LOGGER,
        };
        pub struct HomeController {
            temp: String,
            temp2: String,
            http_context: ::asp_dot_rust::controller::HttpContextRef,
        }
        impl ::asp_dot_rust::controller::WithHttpContext for HomeController {
            fn str_name() -> &'static str {
                "HomeController".strip_suffix("Controller").unwrap_or("HomeController")
            }
            fn new_with_http_context(
                http_context: &mut ::asp_dot_rust::http_context::HttpContext,
            ) -> Self {
                let controller_type_name = "HomeController";
                if !controller_type_name.ends_with("Controller") {
                    {
                        ::core::panicking::panic_fmt(
                            format_args!("Controller name must end with \'Controller\'"),
                        );
                    };
                }
                Self {
                    temp: if let Some(service) = http_context.try_get_service::<String>()
                    {
                        (*service).clone()
                    } else {
                        Default::default()
                    },
                    temp2: if let Some(service) = http_context
                        .try_get_service::<String>()
                    {
                        (*service).clone()
                    } else {
                        Default::default()
                    },
                    http_context: ::asp_dot_rust::controller::HttpContextRef::new(
                        http_context,
                    ),
                }
            }
        }
        #[allow(dead_code)]
        impl HomeController {
            pub async fn index(&mut self) -> impl ActionResult {
                LOGGER::info("Handling index action".to_string());
                self.http_context.response.headers.add("Content-Type", "text/html");
                self.http_context.response.status_code = http::StatusCode::OK;
                "<html><body><h1>Hello, World!</h1></body></html>"
            }
            pub fn update(&mut self) -> impl ActionResult {
                self.temp.clone()
            }
            pub async fn replace(&mut self) -> impl ActionResult {
                self.temp.clone()
            }
            pub async fn health(&self) -> impl ActionResult {
                "ok".to_string()
            }
        }
        /// impl by #[controller_route] macro
        impl ::asp_dot_rust::controller::Routing for HomeController {
            #[allow(
                elided_named_lifetimes,
                clippy::async_yields_async,
                clippy::diverging_sub_expression,
                clippy::let_unit_value,
                clippy::needless_arbitrary_self_type,
                clippy::no_effect_underscore_binding,
                clippy::shadow_same,
                clippy::type_complexity,
                clippy::type_repetition_in_bounds,
                clippy::used_underscore_binding
            )]
            fn routing<'life0, 'async_trait>(
                &'life0 mut self,
                method_name: String,
            ) -> ::core::pin::Pin<
                Box<
                    dyn ::core::future::Future<
                        Output = (),
                    > + ::core::marker::Send + 'async_trait,
                >,
            >
            where
                'life0: 'async_trait,
                Self: 'async_trait,
            {
                Box::pin(async move {
                    let mut __self = self;
                    let method_name = method_name;
                    let _: () = {
                        match method_name.as_str() {
                            "index" => {
                                let (body, content_type, status_code) = {
                                    let action_result = __self.index().await;
                                    let body = action_result.get_body_async().await;
                                    let content_type = action_result.content_type();
                                    let status_code = action_result.status_code();
                                    (body, content_type.to_string(), status_code)
                                };
                                __self.http_context.response.body = body;
                                __self.http_context.response.status_code = status_code;
                                if (!content_type.is_empty()) {
                                    __self
                                        .http_context
                                        .response
                                        .headers
                                        .set_content_type(&content_type);
                                }
                            }
                            "update" => {
                                let (body, content_type, status_code) = {
                                    let action_result = __self.update();
                                    let body = action_result.get_body_async().await;
                                    let content_type = action_result.content_type();
                                    let status_code = action_result.status_code();
                                    (body, content_type.to_string(), status_code)
                                };
                                __self.http_context.response.body = body;
                                __self.http_context.response.status_code = status_code;
                                if (!content_type.is_empty()) {
                                    __self
                                        .http_context
                                        .response
                                        .headers
                                        .set_content_type(&content_type);
                                }
                            }
                            "replace" => {
                                let (body, content_type, status_code) = {
                                    let action_result = __self.replace().await;
                                    let body = action_result.get_body_async().await;
                                    let content_type = action_result.content_type();
                                    let status_code = action_result.status_code();
                                    (body, content_type.to_string(), status_code)
                                };
                                __self.http_context.response.body = body;
                                __self.http_context.response.status_code = status_code;
                                if (!content_type.is_empty()) {
                                    __self
                                        .http_context
                                        .response
                                        .headers
                                        .set_content_type(&content_type);
                                }
                            }
                            "health" => {
                                let (body, content_type, status_code) = {
                                    let action_result = __self.health().await;
                                    let body = action_result.get_body_async().await;
                                    let content_type = action_result.content_type();
                                    let status_code = action_result.status_code();
                                    (body, content_type.to_string(), status_code)
                                };
                                __self.http_context.response.body = body;
                                __self.http_context.response.status_code = status_code;
                                if (!content_type.is_empty()) {
                                    __self
                                        .http_context
                                        .response
                                        .headers
                                        .set_content_type(&content_type);
                                }
                            }
                            _ => {
                                __self.http_context.response.status_code = http::StatusCode::NOT_FOUND;
                                __self.http_context.response.body = http::StatusCode::NOT_FOUND
                                    .canonical_reason()
                                    .unwrap_or("Not Found")
                                    .as_bytes()
                                    .to_vec();
                            }
                        };
                        let body_len = __self.http_context.response.body.len();
                        __self
                            .http_context
                            .response
                            .headers
                            .set_content_length(body_len);
                    };
                })
            }
        }
        #[allow(nonstandard_style)]
        /// This function is generated by the #[controller_route] macro and is used to register the controller and its routes with the routing service at application startup.
        const __asp_register_HomeController: () = {
            /// This function is called at application startup to register the controller and its routes with the routing service.
            fn __asp_register_HomeController_routes() -> ::asp_dot_rust::services::routing::ControllerCollect {
                ::asp_dot_rust::services::routing::register_controller::<
                    HomeController,
                >(
                    "",
                    <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            ::asp_dot_rust::controller::ActionRoute::new(
                                "index",
                                <[_]>::into_vec(::alloc::boxed::box_new(["GET"])),
                                "/",
                            ),
                            ::asp_dot_rust::controller::ActionRoute::new(
                                "update",
                                <[_]>::into_vec(::alloc::boxed::box_new(["POST"])),
                                "/update",
                            ),
                            ::asp_dot_rust::controller::ActionRoute::new(
                                "replace",
                                <[_]>::into_vec(::alloc::boxed::box_new(["PUT"])),
                                "/replace",
                            ),
                            ::asp_dot_rust::controller::ActionRoute::new(
                                "health",
                                <[_]>::into_vec(::alloc::boxed::box_new(["GET", "POST"])),
                                "/health",
                            ),
                        ]),
                    ),
                )
            }
            #[allow(non_upper_case_globals)]
            const _: () = {
                static __INVENTORY: ::inventory::Node = ::inventory::Node {
                    value: &{
                        ::asp_dot_rust::services::routing::ControllerBootstrapRegistration {
                            bootstrap: __asp_register_HomeController_routes,
                        }
                    },
                    next: ::inventory::__private::UnsafeCell::new(
                        ::inventory::__private::Option::None,
                    ),
                };
                unsafe extern "C" fn __ctor() {
                    unsafe {
                        ::inventory::ErasedNode::submit(__INVENTORY.value, &__INVENTORY)
                    }
                }
                #[used]
                #[link_section = ".CRT$XCU"]
                static __CTOR: unsafe extern "C" fn() = __ctor;
            };
        };
    }
}
use asp_dot_rust::{
    ApplicationBuilder, configuration::{CorsConfiguration, RateLimitConfiguration},
    logging::LOGGER,
};
extern crate test;
#[rustc_test_marker = "test_application"]
#[doc(hidden)]
pub const test_application: test::TestDescAndFn = test::TestDescAndFn {
    desc: test::TestDesc {
        name: test::StaticTestName("test_application"),
        ignore: false,
        ignore_message: ::core::option::Option::None,
        source_file: "tests\\test_application.rs",
        start_line: 9usize,
        start_col: 10usize,
        end_line: 9usize,
        end_col: 26usize,
        compile_fail: false,
        no_run: false,
        should_panic: test::ShouldPanic::No,
        test_type: test::TestType::IntegrationTest,
    },
    testfn: test::StaticTestFn(
        #[coverage(off)]
        || test::assert_test_result(test_application()),
    ),
};
fn test_application() {
    let body = async {
        LOGGER::with_color_output(true);
        LOGGER::with_level(asp_dot_rust::logging::LogLevel::None);
        let mut app_builder = ApplicationBuilder::new("TestApp");
        app_builder.with_any_ip().with_http_port(8080);
        app_builder
            .add_custom_configuration(|config: &mut CorsConfiguration| {
                config.allowed_origins = ["*"].into();
                config.allowed_methods = [http::Method::GET, http::Method::POST].into();
                config.allowed_headers = ["Content-Type"].into();
            })
            .add_custom_configuration::<
                RateLimitConfiguration,
            >(|cfg| {
                cfg.max_requests = 5000000000;
                cfg.limit_seconds = 1;
                cfg.block_duration_seconds = 60;
            });
        app_builder.add_controllers();
        let mut app = app_builder.build();
        let _ = app.run().await;
    };
    let mut body = body;
    #[allow(unused_mut)]
    let mut body = unsafe { ::tokio::macros::support::Pin::new_unchecked(&mut body) };
    let body: ::core::pin::Pin<&mut dyn ::core::future::Future<Output = ()>> = body;
    #[allow(
        clippy::expect_used,
        clippy::diverging_sub_expression,
        clippy::needless_return,
        clippy::unwrap_in_result
    )]
    {
        use tokio::runtime::Builder;
        return Builder::new_multi_thread()
            .worker_threads(16usize)
            .enable_all()
            .build()
            .expect("Failed building the Runtime")
            .block_on(body);
    }
}
#[rustc_main]
#[coverage(off)]
#[doc(hidden)]
pub fn main() -> () {
    extern crate test;
    test::test_main_static(&[&test_application])
}
