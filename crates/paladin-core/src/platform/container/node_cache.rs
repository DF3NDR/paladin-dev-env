//! `CachedDelta` (Doc 04 FT-FR-18…20, D-27): the persisted record a
//! `paladin_ports::output::node_cache_port::NodeCachePort` backend stores
//! and returns.
//!
//! Carries its own `schema_version` (X-04), following the
//! `Waypoint`/`StateDelta` precedent for a top-level persisted record: a
//! `CachedDelta` serialized and later read back by a different build remains
//! self-describing independent of whichever `StateDelta` schema version it
//! wraps.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::platform::container::battlefield::StateDelta;

/// Schema version this module's [`CachedDelta`] records are authored under
/// (X-04). Distinct from `BATTLEFIELD_SCHEMA_VERSION`: a `CachedDelta`'s own
/// envelope versions independently of the `StateDelta` payload it wraps,
/// which already carries its own `schema_version`.
pub const NODE_CACHE_SCHEMA_VERSION: &str = "1.0.0";

fn default_node_cache_schema_version() -> String {
    NODE_CACHE_SCHEMA_VERSION.to_string()
}

/// A cached node result (Doc 04 FT-FR-18, D-27): the [`StateDelta`] a node
/// produced on a successful attempt, stamped with when it was stored and
/// when it expires, so a `NodeCachePort` backend can serve a hit without
/// re-executing the node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CachedDelta {
    /// Schema version this record was authored under (X-04).
    #[serde(default = "default_node_cache_schema_version")]
    pub schema_version: String,
    /// The delta the node produced on the attempt that was cached.
    pub delta: StateDelta,
    /// When this entry was written.
    pub stored_at: DateTime<Utc>,
    /// When this entry expires. The TTL boundary is closed at `expires_at`
    /// (FT-06 edge assumption): a read AT OR AFTER this instant is a miss,
    /// never a hit.
    pub expires_at: DateTime<Utc>,
}

impl CachedDelta {
    /// Construct a new `CachedDelta`, stamping the current schema version.
    pub fn new(delta: StateDelta, stored_at: DateTime<Utc>, expires_at: DateTime<Utc>) -> Self {
        Self {
            schema_version: NODE_CACHE_SCHEMA_VERSION.to_string(),
            delta,
            stored_at,
            expires_at,
        }
    }

    /// Whether this entry is expired as of `now`.
    ///
    /// The boundary is CLOSED at `expires_at`: `now >= expires_at` is
    /// expired -- an entry read at exactly its `expires_at` instant is a
    /// miss, never a hit (FT-06 edge assumption, pinned by
    /// `paladin_storage::node_cache::contract_tests::entry_at_exactly_expires_at_is_expired`).
    pub fn is_expired_at(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::container::battlefield::FieldName;

    /// Test 10: `CachedDelta` serialises with its `schema_version` present
    /// and deserialises equal.
    #[test]
    fn cached_delta_round_trips_through_serde_with_schema_version() {
        let mut delta = StateDelta::new();
        delta.set(FieldName::new("count").unwrap(), 42_i64).unwrap();
        let stored_at = Utc::now();
        let expires_at = stored_at + chrono::Duration::seconds(60);
        let cached = CachedDelta::new(delta, stored_at, expires_at);

        let json = serde_json::to_string(&cached).expect("serialize");
        assert!(json.contains(NODE_CACHE_SCHEMA_VERSION));
        let restored: CachedDelta = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(restored, cached);
        assert_eq!(restored.schema_version, NODE_CACHE_SCHEMA_VERSION);
    }

    #[test]
    fn cached_delta_payload_without_schema_version_deserializes_with_default() {
        let stored_at = Utc::now();
        let expires_at = stored_at + chrono::Duration::seconds(60);
        let cached = CachedDelta::new(StateDelta::new(), stored_at, expires_at);

        let mut value = serde_json::to_value(&cached).unwrap();
        let obj = value
            .as_object_mut()
            .expect("CachedDelta serializes to a JSON object");
        obj.remove("schema_version");
        // `delta` (a `StateDelta`) carries its OWN nested `schema_version`
        // field, so a whole-string substring check would false-positive on
        // it -- assert the TOP-LEVEL key is genuinely absent instead.
        assert!(!obj.contains_key("schema_version"));

        let restored: CachedDelta = serde_json::from_value(value).unwrap();
        assert_eq!(restored.schema_version, NODE_CACHE_SCHEMA_VERSION);
    }

    #[test]
    fn is_expired_at_boundary_is_closed() {
        let stored_at = Utc::now();
        let expires_at = stored_at + chrono::Duration::seconds(10);
        let cached = CachedDelta::new(StateDelta::new(), stored_at, expires_at);

        assert!(!cached.is_expired_at(expires_at - chrono::Duration::milliseconds(1)));
        assert!(cached.is_expired_at(expires_at));
        assert!(cached.is_expired_at(expires_at + chrono::Duration::milliseconds(1)));
    }
}
