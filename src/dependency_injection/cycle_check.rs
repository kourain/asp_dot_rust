use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

use super::{Dependency, ServiceDependencies};
use crate::services::{
    configuration::ConfigurationService,
    service_provider::{ServiceInstance, ServiceProviderScope, ServiceType},
};

impl ServiceProviderScope {
    /// Checks the declared dependencies of services registered in this provider.
    #[track_caller]
    pub fn check_dependency_cycles(&self) {
        let graph = DependencyGraph::new(self);
        Validator::new(&graph, self, None).registered_services();
    }

    pub(crate) fn validate_startup_dependencies(&self, configuration: &ConfigurationService) {
        let graph = DependencyGraph::new(self);
        Validator::new(&graph, self, Some(configuration)).registered_services();
    }
}

struct DependencyNode {
    name: &'static str,
    dependencies: Vec<Dependency>,
}

/// Only service registrations in this provider participate in cycle validation.
struct DependencyGraph {
    nodes: HashMap<TypeId, DependencyNode>,
}

impl DependencyGraph {
    fn new(scope: &ServiceProviderScope) -> Self {
        let mut graph = Self { nodes: HashMap::new() };
        for entry in inventory::iter::<ServiceDependencies> {
            let owner = (entry.owner)();
            if scope.contains_type_id(&owner) {
                graph.nodes.insert(
                    owner,
                    DependencyNode {
                        name: (entry.owner_name)(),
                        dependencies: (entry.dependencies)(),
                    },
                );
            }
        }
        graph
    }
}

struct Validator<'a> {
    graph: &'a DependencyGraph,
    scope: &'a ServiceProviderScope,
    configuration: Option<&'a ConfigurationService>,
    visited: HashSet<TypeId>,
    visiting: HashSet<TypeId>,
    path: Vec<&'static str>,
}

impl<'a> Validator<'a> {
    fn new(graph: &'a DependencyGraph, scope: &'a ServiceProviderScope, configuration: Option<&'a ConfigurationService>) -> Self {
        Self {
            graph,
            scope,
            configuration,
            visited: HashSet::new(),
            visiting: HashSet::new(),
            path: Vec::new(),
        }
    }

    fn registered_services(&mut self) {
        for &owner in self.graph.nodes.keys() {
            if let Some(registration) = self.scope.registration(&owner) {
                self.service(owner, registration);
            }
        }
    }

    fn service(&mut self, owner: TypeId, registration: &ServiceInstance) {
        if self.visited.contains(&owner) || (registration.instance.initialized() && registration.service_type != ServiceType::Scope) {
            return;
        }
        let Some(node) = self.graph.nodes.get(&owner) else {
            return;
        };
        if !self.visiting.insert(owner) {
            panic!("Found circular dependency: {} -> {}", self.path.join(" -> "), node.name);
        }
        self.path.push(node.name);
        for dependency in &node.dependencies {
            match *dependency {
                Dependency::Service(id, name) => match self.scope.registration(&id) {
                    Some(registration) => self.service(id, registration),
                    None => panic!("Invalid dependency graph: {} -> service {name} is not registered", self.path.join(" -> ")),
                },
                Dependency::Configuration { name, problem } => {
                    if let Some(configuration) = self.configuration
                        && let Some(reason) = problem(configuration)
                    {
                        panic!("Invalid dependency graph: {} -> configuration {name} {reason}", self.path.join(" -> "));
                    }
                }
            }
        }
        self.path.pop();
        self.visiting.remove(&owner);
        self.visited.insert(owner);
    }
}
