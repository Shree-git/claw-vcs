use std::collections::{HashMap, HashSet};

use claw_core::id::ObjectId;
use claw_store::ClawStore;

/// Walk the revision DAG from `heads` to find all reachable objects.
pub fn find_reachable_objects(store: &ClawStore, heads: &[ObjectId]) -> HashSet<ObjectId> {
    let mut visited = HashSet::new();
    let mut queue: Vec<ObjectId> = heads.to_vec();

    while let Some(id) = queue.pop() {
        if !visited.insert(id) {
            continue;
        }
        let obj = match store.load_object(&id) {
            Ok(obj) => obj,
            Err(e) => {
                tracing::warn!("missing object in DAG traversal: {} ({})", id, e);
                continue;
            }
        };
        queue.extend(obj.dependencies());
    }

    visited
}

/// Walk the dependency graph from `heads` up to an optional dependency depth.
///
/// Depth is measured in edges from each head. A head object is depth 0, its
/// direct dependencies are depth 1, and so on.
pub fn find_reachable_objects_with_depth(
    store: &ClawStore,
    heads: &[ObjectId],
    max_depth: Option<u32>,
) -> HashSet<ObjectId> {
    let mut visited_depth: HashMap<ObjectId, u32> = HashMap::new();
    let mut queue: Vec<(ObjectId, u32)> = heads.iter().copied().map(|id| (id, 0)).collect();

    while let Some((id, depth)) = queue.pop() {
        if let Some(prev) = visited_depth.get(&id) {
            if *prev <= depth {
                continue;
            }
        }
        visited_depth.insert(id, depth);

        if max_depth.is_some_and(|limit| depth >= limit) {
            continue;
        }

        let obj = match store.load_object(&id) {
            Ok(obj) => obj,
            Err(e) => {
                tracing::warn!("missing object in DAG traversal: {} ({})", id, e);
                continue;
            }
        };

        for dep in obj.dependencies() {
            queue.push((dep, depth + 1));
        }
    }

    visited_depth.into_keys().collect()
}

fn visit_ordered(
    store: &ClawStore,
    id: ObjectId,
    visiting: &mut HashSet<ObjectId>,
    visited: &mut HashSet<ObjectId>,
    out: &mut Vec<ObjectId>,
) {
    if visited.contains(&id) {
        return;
    }
    if !visiting.insert(id) {
        // Defensive cycle guard; object graphs should be acyclic for dependency links.
        tracing::warn!("cycle detected in object dependency traversal at {}", id);
        return;
    }

    let obj = match store.load_object(&id) {
        Ok(obj) => obj,
        Err(e) => {
            tracing::warn!("missing object in dependency traversal: {} ({})", id, e);
            visiting.remove(&id);
            return;
        }
    };

    for dep in obj.dependencies() {
        visit_ordered(store, dep, visiting, visited, out);
    }

    visiting.remove(&id);
    if visited.insert(id) {
        out.push(id);
    }
}

/// Walk the dependency graph from `heads` and return object ids in dependency-first order.
///
/// This ordering is required by transports that validate referenced objects on each insert
/// (for example, ClawLab HTTP object upload), where children must not be sent before parents.
pub fn ordered_reachable_objects(store: &ClawStore, heads: &[ObjectId]) -> Vec<ObjectId> {
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    let mut out = Vec::new();

    for id in heads {
        visit_ordered(store, *id, &mut visiting, &mut visited, &mut out);
    }

    out
}

/// Compute the objects we need to send (have but remote doesn't).
pub fn compute_want_have(
    local_objects: &HashSet<ObjectId>,
    remote_objects: &HashSet<ObjectId>,
) -> (Vec<ObjectId>, Vec<ObjectId>) {
    let want: Vec<ObjectId> = remote_objects.difference(local_objects).copied().collect();
    let have: Vec<ObjectId> = local_objects
        .intersection(remote_objects)
        .copied()
        .collect();
    (want, have)
}
