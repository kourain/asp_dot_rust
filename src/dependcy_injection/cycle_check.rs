use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
};

use crate::{dependcy_injection::edge::DependencyEdge, logging::LOGGER, services::service_provider::ServiceProviderScope};

pub fn check_dependency_cycles(service: &ServiceProviderScope) {
    let mut graph: HashMap<TypeId, (&'static str, Vec<TypeId>)> = HashMap::new();

    for edge in inventory::iter::<DependencyEdge> {
        let owner = (edge.owner)();
        if !service.contains_type_id(&owner) {
            continue;
        }

        let deps = (edge.dependencies)();
        let mut filtered_deps = Vec::new();

        for (dep_id, dep_name) in &deps {
            if service.contains_type_id(dep_id) {
                filtered_deps.push(*dep_id);
            } else {
                // owner registered but dependency of it has not been registered —
                // this is almost certainly a DI configuration error

                #[cfg(debug_assertions)]
                LOGGER::warn(format!("Service {} require {} but it has not been registered in ServiceProviderScope", edge.owner_name, dep_name));
            }
        }

        graph.entry(owner).or_insert((edge.owner_name, Vec::new())).1.extend(filtered_deps);
    }

    let mut visited = HashSet::new();
    let mut stack = HashSet::new();
    let mut path = Vec::new();

    for &start in graph.keys() {
        if !visited.contains(&start) {
            if let Some(cycle) = dfs_detect(start, &graph, &mut visited, &mut stack, &mut path) {
                let dependcy_path = cycle.iter().map(|id| graph.get(id).map(|(n, _)| *n).unwrap_or("?")).collect::<Vec<_>>().join(" -> ");
                panic!("Found circular dependency: {}", dependcy_path);
            }
        }
    }
}
fn dfs_detect(node: TypeId, graph: &HashMap<TypeId, (&'static str, Vec<TypeId>)>, visited: &mut HashSet<TypeId>, stack: &mut HashSet<TypeId>, path: &mut Vec<TypeId>) -> Option<Vec<TypeId>> {
    visited.insert(node);
    stack.insert(node);
    path.push(node);

    if let Some((_, deps)) = graph.get(&node) {
        for &dep in deps {
            if stack.contains(&dep) {
                // tìm thấy vòng lặp — cắt path từ điểm bắt đầu vòng
                let start_idx = path.iter().position(|&n| n == dep).unwrap();
                let mut cycle = path[start_idx..].to_vec();
                cycle.push(dep);
                return Some(cycle);
            }
            if !visited.contains(&dep) {
                if let Some(cycle) = dfs_detect(dep, graph, visited, stack, path) {
                    return Some(cycle);
                }
            }
        }
    }

    stack.remove(&node);
    path.pop();
    None
}
