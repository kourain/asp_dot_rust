#![allow(deprecated)]

use std::{
    any::type_name,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use asp_dot_rust::{
    Application, ApplicationBuilder, api_controller,
    controller::{ActionResult, get, results::ok},
    dependency_injection::{Cfg, CfgOption, CfgReload, CfgRequire, DependencyInjectableService, Serv},
    hosted_service::BackGroundService,
    http_context::{HttpContext, HttpContextRef},
    macros::{DependencyInjectableService, controller_inject, controller_route, inject, middleware},
    middleware::{Middleware, MiddlewareNext},
    services::routing::RoutingService,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

static RESOLUTIONS: AtomicUsize = AtomicUsize::new(0);

#[derive(Default)]
struct OptionalRequestConfiguration;

#[derive(Default)]
struct RequiredRequestConfiguration;

#[derive(Default)]
struct ReloadableRequestConfiguration;

#[derive(Default)]
struct AliasRequestConfiguration;

struct ResolutionProbe;

#[inject]
impl ResolutionProbe {
    fn new() -> Self {
        RESOLUTIONS.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

mod request_controller {
    use asp_dot_rust::controller::results::ok;

    use super::*;

    api_controller!(RegistrationController {});

    #[controller_route]
    #[controller_inject]
    impl RegistrationController {
        fn new(
            http_context: HttpContextRef,
            _: Serv<ResolutionProbe>,
            _: Serv<MissingService>,
            _: CfgOption<OptionalRequestConfiguration>,
            _: CfgRequire<RequiredRequestConfiguration>,
            _: CfgReload<ReloadableRequestConfiguration>,
            _: Cfg<AliasRequestConfiguration>,
        ) -> Self {
            Self { http_context }
        }

        #[get("/registration-check")]
        async fn get(&self) -> impl ActionResult {
            ok("unexpected")
        }
    }
}

#[derive(DependencyInjectableService)]
struct MissingService;

#[derive(DependencyInjectableService)]
struct IntermediateService {
    _dependency: Serv<MissingService>,
}

api_controller!(DependencyController {});

#[controller_route]
#[controller_inject]
impl DependencyController {
    fn new(http_context: HttpContextRef, _: Serv<IntermediateService>) -> Self {
        Self { http_context }
    }

    #[get("/dependency")]
    async fn get(&mut self) -> impl ActionResult {
        ok("ok")
    }
}

#[derive(DependencyInjectableService)]
struct ConstructorService;

struct RequestMiddleware {
    next: MiddlewareNext,
}

#[middleware]
impl RequestMiddleware {
    fn new(next: MiddlewareNext, _: Serv<ConstructorService>) -> Self {
        Self { next }
    }

    async fn invoke_async(
        &self,
        ctx: &mut HttpContext,
        _: Serv<ResolutionProbe>,
        _: Serv<MissingService>,
        _: CfgOption<OptionalRequestConfiguration>,
        _: CfgRequire<RequiredRequestConfiguration>,
        _: CfgReload<ReloadableRequestConfiguration>,
        _: Cfg<AliasRequestConfiguration>,
    ) {
        self.next.next_async(ctx).await;
    }
}

#[derive(serde::Deserialize)]
struct StartupConfiguration {}

macro_rules! configuration_service {
    ($name:ident, $wrapper:ident) => {
        struct $name;

        #[inject]
        impl $name {
            fn new(_: $wrapper<StartupConfiguration>) -> Self {
                Self
            }
        }
    };
}

configuration_service!(OptionalConfigurationService, CfgOption);
configuration_service!(RequiredConfigurationService, CfgRequire);
configuration_service!(ReloadableConfigurationService, CfgReload);
configuration_service!(AliasConfigurationService, Cfg);

struct OptionalConfigurationMiddleware {
    next: MiddlewareNext,
}

#[middleware]
impl OptionalConfigurationMiddleware {
    fn new(next: MiddlewareNext, _: CfgOption<StartupConfiguration>) -> Self {
        Self { next }
    }

    async fn invoke_async(&self, ctx: &mut HttpContext) {
        self.next.next_async(ctx).await;
    }
}

struct ReloadableConfigurationMiddleware {
    next: MiddlewareNext,
}

#[middleware]
impl ReloadableConfigurationMiddleware {
    fn new(next: MiddlewareNext, _: CfgReload<StartupConfiguration>) -> Self {
        Self { next }
    }

    async fn invoke_async(&self, ctx: &mut HttpContext) {
        self.next.next_async(ctx).await;
    }
}

struct ConfigurationMiddleware {
    next: MiddlewareNext,
}

#[middleware]
impl ConfigurationMiddleware {
    fn new(next: MiddlewareNext, _: CfgRequire<StartupConfiguration>) -> Self {
        Self { next }
    }

    async fn invoke_async(&self, ctx: &mut HttpContext) {
        self.next.next_async(ctx).await;
    }
}

struct HostedWorker;

#[inject]
impl HostedWorker {
    fn new(_: Serv<MissingService>) -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl BackGroundService for HostedWorker {
    async fn invoke_async(&mut self) {
        unreachable!("invalid applications must fail before spawning hosted services");
    }
}

async fn startup_error(app: Application) -> String {
    let error = tokio::time::timeout(Duration::from_secs(5), tokio::spawn(app.run()))
        .await
        .expect("invalid dependencies should stop startup")
        .expect_err("startup should panic");
    *error.into_panic().downcast::<String>().expect("expected a startup diagnostic")
}

#[tokio::test]
async fn every_service_configuration_wrapper_requires_registration_at_startup() {
    async fn check<S: DependencyInjectableService>() {
        let mut builder = ApplicationBuilder::new("unregistered_service_configuration");
        builder.service.add_singleton::<S>();
        let error = startup_error(builder.build()).await;
        let expected = format!("configuration {} is not registered", type_name::<StartupConfiguration>());
        assert!(error.contains(&expected), "{error}");
    }

    check::<OptionalConfigurationService>().await;
    check::<RequiredConfigurationService>().await;
    check::<ReloadableConfigurationService>().await;
    check::<AliasConfigurationService>().await;
}

#[tokio::test]
async fn registered_service_reports_the_full_path_to_a_missing_service() {
    let mut builder = ApplicationBuilder::new("controller_dependency");
    builder.add_controllers();
    builder.service.add_singleton::<IntermediateService>();
    let error = startup_error(builder.build()).await;
    assert!(
        error.contains("IntermediateService -> service startup_dependency_validation::MissingService is not registered"),
        "{error}"
    );
}

#[tokio::test]
async fn middleware_constructor_dependencies_are_validated() {
    let mut app = ApplicationBuilder::new("middleware_constructor_dependency").build();
    app.add_middleware::<RequestMiddleware>();
    let error = startup_error(app).await;
    let expected = format!("Service {} not registered", type_name::<ConstructorService>());
    assert!(error.contains(&expected), "{error}");
}

#[tokio::test]
async fn swagger_without_registration_names_the_required_builder_call() {
    let mut app = ApplicationBuilder::new("unregistered_swagger").build();
    app.use_swagger();
    let error = startup_error(app).await;
    assert!(error.contains("OpenApiService"), "{error}");
    assert!(error.contains("builder.add_swagger_gen()"), "{error}");
}

#[tokio::test]
#[should_panic(expected = "middleware factory was called")]
async fn middleware_request_dependencies_do_not_block_construction() {
    let mut app = ApplicationBuilder::new("middleware_factory_dependency").build();
    app.add_middleware_factory(|_| -> RequestMiddleware { panic!("middleware factory was called") });
    tokio::time::timeout(Duration::from_secs(5), app.run()).await.expect("application should reach its middleware factory");
}

#[tokio::test]
async fn every_configuration_wrapper_requires_registration_at_startup() {
    async fn check<M: Middleware + 'static>() {
        let mut app = ApplicationBuilder::new("unregistered_configuration").build();
        app.add_middleware::<M>();

        let error = startup_error(app).await;
        let expected = format!("Configuration of type {} was not registered", type_name::<StartupConfiguration>());
        assert!(error.contains(&expected), "{error}");
    }

    check::<OptionalConfigurationMiddleware>().await;
    check::<ReloadableConfigurationMiddleware>().await;
    check::<ConfigurationMiddleware>().await;
}

#[tokio::test]
#[should_panic(expected = "StartupConfiguration has no value")]
async fn middleware_constructor_rejects_an_empty_required_configuration() {
    let mut builder = ApplicationBuilder::new("middleware_config_dependency");
    builder.configuration.configure::<StartupConfiguration>("missing_startup_validation_section");
    let mut app = builder.build();
    app.add_middleware::<ConfigurationMiddleware>();
    tokio::time::timeout(Duration::from_secs(5), app.run())
        .await
        .expect("application should resolve the required configuration");
}

#[tokio::test]
async fn hosted_service_is_validated_before_it_is_spawned() {
    let mut builder = ApplicationBuilder::new("hosted_dependency");
    builder.add_hosted_service::<HostedWorker>();
    let error = startup_error(builder.build()).await;
    let expected = format!("Service {} not registered", type_name::<MissingService>());
    assert!(error.contains(&expected), "{error}");
}

#[tokio::test]
#[should_panic(expected = "second application reached its middleware factory")]
async fn startup_dependencies_are_isolated_by_application() {
    let mut first = ApplicationBuilder::new("first_application");
    first.add_controllers().add_hosted_service::<HostedWorker>();
    let mut first = first.build();
    first.add_middleware::<RequestMiddleware>();

    let mut second = ApplicationBuilder::new("second_application");
    second.configuration.configure::<StartupConfiguration>("startup_configuration");
    let mut second = second.build();
    second.add_middleware_factory(|_| -> OptionalConfigurationMiddleware { panic!("second application reached its middleware factory") });
    tokio::time::timeout(Duration::from_secs(5), second.run())
        .await
        .expect("application should reach its middleware factory");
}

#[tokio::test]
async fn request_injection_checks_all_registrations_before_resolving_arguments() {
    for (with_middleware, missing) in [false, true]
        .into_iter()
        .flat_map(|with_middleware| ["service", "optional", "required", "reload", "alias"].map(|missing| (with_middleware, missing)))
    {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);

        let mut builder = ApplicationBuilder::new("request_registration_check");
        builder.add_controllers().with_loopback_ip().with_http_port(port);
        builder.service.add_transient::<ResolutionProbe>();
        builder.service.add_singleton::<ConstructorService>();
        if missing != "service" {
            builder.service.add_singleton::<MissingService>();
        }
        if missing != "optional" {
            builder.configuration.insert::<OptionalRequestConfiguration>(|_| {});
        }
        if missing != "required" {
            builder.configuration.insert::<RequiredRequestConfiguration>(|_| {});
        }
        if missing != "reload" {
            builder.configuration.insert::<ReloadableRequestConfiguration>(|_| {});
        }
        if missing != "alias" {
            builder.configuration.insert::<AliasRequestConfiguration>(|_| {});
        }
        let mut app = builder.build();
        assert!(app.service_provider.contains_service::<RoutingService>());
        // Startup must replace an already cached empty router with the compiled routes.
        let empty_routing = app.service_provider.get_service::<RoutingService>().await;
        assert!(empty_routing.get_allowed_methods("/registration-check").is_empty());
        if with_middleware {
            app.add_middleware::<RequestMiddleware>();
        }
        let task = tokio::spawn(app.run());
        let mut stream = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Result::Ok(stream) = TcpStream::connect(("127.0.0.1", port)).await {
                    break stream;
                }
                assert!(!task.is_finished(), "request dependencies should be checked when the request is injected");
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("application should start before checking request registrations");
        stream.write_all(b"GET /registration-check HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").await.unwrap();
        let mut response = String::new();
        let result = tokio::time::timeout(Duration::from_secs(5), stream.read_to_string(&mut response)).await;
        task.abort();
        let _ = task.await;
        result.expect("request should complete").unwrap();
        assert!(response.starts_with("HTTP/1.1 500"), "missing {missing}, middleware={with_middleware}: {response}");
        assert_eq!(
            RESOLUTIONS.load(Ordering::SeqCst),
            0,
            "missing {missing}, middleware={with_middleware}: all registrations must be checked before the first argument is resolved"
        );
    }
}
