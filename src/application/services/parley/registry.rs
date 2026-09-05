//! RED state (Phase 24 Plan 10, Task 2): `GraphRegistry` does not exist yet.
//! See the GREEN commit that follows for the full implementation and
//! rustdoc.

use paladin_battalion::engine::WarGraph;
use paladin_core::platform::container::waypoint::GraphFingerprint;

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
}
