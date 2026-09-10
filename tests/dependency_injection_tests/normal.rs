use crate::dependency_injection_tests::services::*;
use asp_dot_rust::ApplicationBuilder;
use std::sync::Arc;

#[test]
fn singleton_returns_the_same_instance_on_every_call() {
    let mut builder = ApplicationBuilder::new("TestSingleton");
    builder.service.add_singleton::<CounterService>();

    let first = builder.service.get_service::<CounterService>();
    let second = builder.service.get_service::<CounterService>();

    assert!(Arc::ptr_eq(&first, &second), "Singleton must resolve to the same Arc on every call");
    assert_eq!(first.id, second.id, "Singleton must not be constructed more than once");
}

#[test]
fn transient_returns_a_new_instance_on_every_call() {
    let mut builder = ApplicationBuilder::new("TestTransient");
    builder.service.add_transient::<CounterService>();

    let first = builder.service.get_service::<CounterService>();
    let second = builder.service.get_service::<CounterService>();

    assert!(!Arc::ptr_eq(&first, &second), "Transient must resolve to a new Arc on every call");
    assert_ne!(first.id, second.id, "Transient must construct a fresh instance every time");
}

#[test]
fn scope_shares_the_instance_within_one_scope_but_not_across_scopes() {
    let mut builder = ApplicationBuilder::new("TestScope");
    builder.service.add_scope::<CounterService>();

    // Two calls within the same scope must return the same instance.
    let root_first = builder.service.get_service::<CounterService>();
    let root_second = builder.service.get_service::<CounterService>();
    assert!(Arc::ptr_eq(&root_first, &root_second), "Scope must share the instance within the same scope");

    // A fresh scope (e.g. a new incoming request) must get its own instance.
    let child_scope = builder.service.create_scope();
    let child_first = child_scope.get_service::<CounterService>();

    assert!(!Arc::ptr_eq(&root_first, &child_first), "A new scope must not reuse the instance created in a previous scope");
}

#[test]
fn nested_dependency_resolves_through_the_same_scope() {
    let mut builder = ApplicationBuilder::new("TestNestedDependency");
    builder.service.add_singleton::<CounterService>();
    builder.service.add_singleton::<WrapperService>();

    let wrapper = builder.service.get_service::<WrapperService>();
    let direct = builder.service.get_service::<CounterService>();

    assert!(
        Arc::ptr_eq(&wrapper.inner, &direct),
        "A service resolved as a constructor dependency must be the same Singleton instance as one resolved directly"
    );
}

#[tokio::test]
async fn contains_service_reflects_registration_state() {
    let mut builder = ApplicationBuilder::new("TestContainsService");

    assert!(!builder.service.contains_service::<CounterService>());
    builder.service.add_singleton::<CounterService>();
    assert!(builder.service.contains_service::<CounterService>());
}

#[test]
#[should_panic(expected = "Service")]
fn get_service_panics_when_the_type_was_never_registered() {
    let builder = ApplicationBuilder::new("TestUnregisteredService");
    // CounterService was never added to the scope, so this must panic rather
    // than silently returning a default value.
    let _ = builder.service.get_service::<CounterService>();
}
