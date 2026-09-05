//! Graph Registry — Fingerprint-Keyed `WarGraph` Lookup (HITL-05, D-26)
//!
//! A thread's own latest Waypoint carries the [`GraphFingerprint`] the
//! [`ParleyPort`](crate::application::services::parley::adapter) facade
//! adapter resolves a runnable `WarGraph` by. [`GraphRegistry`] is the
//! simplest thing that can hold that mapping today: a code-registered,
//! in-process `HashMap`. Phase 27's `WarGraphDoc`/assistant registry
//! replaces this lookup behind the SAME [`ParleyPort`](crate::application::services::parley::adapter)
//! — so this type's surface stays deliberately minimal (register, resolve,
//! nothing else) to keep that seam narrow.
//!
//! An unregistered fingerprint is never resolved to a default or "nearest"
//! graph (D-26): [`GraphRegistry::resolve`] returns `None`, and the facade
//! adapter turns that into `ParleyError::GraphNotRegistered` — a thread
//! whose own graph has not been registered in THIS process can never be
//! silently run against the wrong one.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use paladin_battalion::engine::WarGraph;
use paladin_core::platform::container::waypoint::GraphFingerprint;

/// A code-registered, fingerprint-keyed lookup from a [`GraphFingerprint`]
/// to the [`WarGraph`] that produced it.
///
/// # Examples
///
/// ```
/// use paladin::application::services::parley::registry::GraphRegistry;
/// use paladin_battalion::engine::{EngineLimits, WarGraph};
/// use paladin_core::platform::container::battlefield::BattlefieldSchema;
/// use paladin_core::platform::container::waypoint::GraphFingerprint;
///
/// let registry = GraphRegistry::new();
/// let graph = WarGraph::new(BattlefieldSchema::new(Vec::new()), EngineLimits::default());
/// let fingerprint = graph.fingerprint();
/// registry.register(graph);
///
/// assert!(registry.resolve(&fingerprint).is_some());
/// let unregistered = GraphFingerprint::from_canonical_bytes(b"never-registered");
/// assert!(registry.resolve(&unregistered).is_none());
/// ```
#[derive(Default)]
pub struct GraphRegistry {
    graphs: RwLock<HashMap<GraphFingerprint, Arc<WarGraph>>>,
}

impl GraphRegistry {
    /// Construct an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register `graph` under its own [`WarGraph::fingerprint`], returning
    /// that fingerprint. Registering a graph whose fingerprint is already
    /// present replaces the prior entry — re-registering the identical
    /// graph is an idempotent no-op in effect, and this is never treated as
    /// an error (a process restart that re-runs its own registration code
    /// must not need special-casing).
    pub fn register(&self, graph: WarGraph) -> GraphFingerprint {
        let fingerprint = graph.fingerprint();
        // --- WR-01 (24-REVIEW.md): recover the guard on poison rather than
        // `.unwrap()`ing it -- a panic elsewhere while some OTHER writer
        // held this lock must not cascade into every subsequent
        // register/resolve call across this `Arc`-shared registry, unlike a
        // normal single-owner panic. Safe here: the map itself cannot be
        // left torn by a panic mid-`insert`/`get` (no partial-write
        // invariant to violate), so the recovered guard's contents are
        // trusted exactly as an unpoisoned one's would be.
        self.graphs
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(fingerprint.clone(), Arc::new(graph));
        fingerprint
    }

    /// Resolve `fingerprint` to its registered `WarGraph`, or `None` if
    /// nothing has been registered under it in this process (D-26) — never
    /// a default or "nearest" graph.
    pub fn resolve(&self, fingerprint: &GraphFingerprint) -> Option<Arc<WarGraph>> {
        self.graphs
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(fingerprint)
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_battalion::engine::EngineLimits;
    use paladin_core::platform::container::battlefield::BattlefieldSchema;

    fn empty_graph() -> WarGraph {
        WarGraph::new(BattlefieldSchema::new(Vec::new()), EngineLimits::default())
    }

    /// Test 7 (Task 2): a registered graph is found by the fingerprint on
    /// the thread's latest Waypoint.
    #[test]
    fn registry_resolves_by_fingerprint() {
        let registry = GraphRegistry::new();
        let graph = empty_graph();
        let fingerprint = graph.fingerprint();

        let returned = registry.register(graph);
        assert_eq!(returned, fingerprint);

        let resolved = registry
            .resolve(&fingerprint)
            .expect("a registered fingerprint must resolve");
        assert_eq!(resolved.fingerprint(), fingerprint);
    }

    #[test]
    fn unregistered_fingerprint_resolves_to_none() {
        let registry = GraphRegistry::new();
        let unregistered = GraphFingerprint::from_canonical_bytes(b"never-registered");
        assert!(registry.resolve(&unregistered).is_none());
    }

    #[test]
    fn re_registering_the_same_fingerprint_replaces_the_entry() {
        let registry = GraphRegistry::new();
        let graph_a = empty_graph();
        let fingerprint = registry.register(graph_a);

        // A second empty graph has the identical shape, so it fingerprints
        // identically -- registering it again must not error, and the
        // fingerprint must still resolve afterward.
        let graph_b = empty_graph();
        assert_eq!(graph_b.fingerprint(), fingerprint);
        registry.register(graph_b);

        assert!(registry.resolve(&fingerprint).is_some());
    }

    /// WR-01 (24-REVIEW.md): a panic elsewhere while a writer holds the
    /// lock must not cascade into every subsequent `register`/`resolve`
    /// call -- both recover the poisoned guard instead of propagating the
    /// poison as a fresh panic.
    #[test]
    fn register_and_resolve_survive_a_poisoned_lock() {
        let registry = Arc::new(GraphRegistry::new());
        let graph = empty_graph();
        let fingerprint = registry.register(graph);

        // Poison the lock: a writer panics while holding the write guard.
        let poisoner = Arc::clone(&registry);
        let panicked = std::thread::spawn(move || {
            let _guard = poisoner.graphs.write().unwrap();
            panic!("simulated writer panic while holding the lock");
        })
        .join();
        assert!(panicked.is_err(), "the poisoning thread must have panicked");

        // Both operations must recover the poisoned guard rather than
        // panicking themselves, and see the state the poisoned writer left
        // behind (a plain `HashMap` cannot be left torn by a panic
        // mid-`insert`/`get`).
        assert!(
            registry.resolve(&fingerprint).is_some(),
            "resolve must recover a poisoned lock instead of panicking"
        );
        let graph_b = empty_graph();
        registry.register(graph_b);
        assert!(
            registry.resolve(&fingerprint).is_some(),
            "register must recover a poisoned lock instead of panicking"
        );
    }
}
