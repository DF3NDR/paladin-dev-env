//! Structured node-execution error family (Doc 04 FT-FR-02, D-07).
//!
//! [`NodeError`] is the sole error value the superstep engine's per-node
//! dispatch retry loop reasons about: a plain `serde` value carrying only
//! serializable summaries -- no live error object, no trait object, nothing
//! that would defeat `Clone`/`PartialEq`/`Serialize`/`Deserialize`. Every
//! variant is built by a `From` conversion at the boundary where the
//! ORIGINAL error (a live `StateNodeError`, an HTTP response, a provider SDK
//! error) is still in scope; by the time it reaches a [`NodeError`], only
//! its `Display` output survives (mirroring
//! [`crate::platform::container::waypoint::NodeExecutionRecord`]'s own
//! "no live error objects" precedent). `NodeError` carries no
//! `schema_version` of its own -- it nests inside the already-versioned
//! `Waypoint` (the `ParleyRequest` precedent).
//!
//! # Security: redact before you bound (D-34)
//!
//! Any text that crosses from a node implementation or an LLM provider into
//! a [`NodeErrorSource::Paladin`]/[`NodeErrorSource::Llm`] `message` is
//! attacker- or third-party-influenced. The ordering is load-bearing:
//! **redact, then bound.** Bounding a response body before redaction can
//! slice a leaked credential in half at the truncation boundary and leave
//! the surviving half in a persisted `Waypoint`. `paladin-core` has zero
//! dependencies (ADR-0015), so the actual redaction call lives in the
//! calling crate (`paladin-llm`'s `redaction::redact_credentials`, or the
//! engine boundary in `paladin-battalion`) -- this module's contract is
//! that every `message` passed into a constructor here has ALREADY been
//! redacted before it was bounded, never the reverse.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::platform::container::transience::Transience;
use crate::platform::container::waypoint::NodeId;

/// Which timeout fired (Doc 04 FT-FR-03; the policy that produces this is
/// plan 25-09's `TimeoutPolicy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TimeoutKind {
    /// The node's whole execution (across every attempt) exceeded
    /// `TimeoutPolicy::run_timeout`.
    Run,
    /// The node produced no heartbeat within `TimeoutPolicy::idle_timeout`.
    Idle,
    /// The whole engine run's own bound was exceeded.
    EngineRun,
}

impl std::fmt::Display for TimeoutKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TimeoutKind::Run => "run",
            TimeoutKind::Idle => "idle",
            TimeoutKind::EngineRun => "engine_run",
        };
        f.write_str(s)
    }
}

/// The structured detail of what failed a node's execution (D-07): every
/// variant carries only serializable data, never a live error object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NodeErrorSource {
    /// A `NodeSpec::Paladin` execution failed.
    Paladin {
        /// A short, machine-stable classification of the failure (e.g. the
        /// provider adapter's own error variant name).
        kind: String,
        /// The HTTP status code, if the failure came from an HTTP response.
        status: Option<u16>,
        /// The LLM provider name, if applicable.
        provider: Option<String>,
        /// A redacted, human-readable summary (see the module-level D-34
        /// note: redacted before this point, never bounded before that).
        message: String,
    },
    /// An LLM-provider-level failure surfaced independently of a specific
    /// Paladin execution (Doc 04 plan 25-08's fallback chain).
    Llm {
        /// See [`NodeErrorSource::Paladin`]'s `kind`.
        kind: String,
        /// See [`NodeErrorSource::Paladin`]'s `status`.
        status: Option<u16>,
        /// See [`NodeErrorSource::Paladin`]'s `provider`.
        provider: Option<String>,
        /// See [`NodeErrorSource::Paladin`]'s `message`.
        message: String,
    },
    /// A `NodeSpec::Function` node's `StateNode::run` returned an error.
    Function {
        /// The node's own error message (its `Display` output).
        message: String,
    },
    /// A timeout fired (plan 25-09).
    Timeout(TimeoutKind),
    /// The run was cancelled while this node was executing or waiting on a
    /// retry backoff (D-15).
    Cancelled,
}

impl std::fmt::Display for NodeErrorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeErrorSource::Paladin { message, .. } => write!(f, "paladin error: {message}"),
            NodeErrorSource::Llm { message, .. } => write!(f, "llm error: {message}"),
            NodeErrorSource::Function { message } => write!(f, "function error: {message}"),
            NodeErrorSource::Timeout(kind) => write!(f, "{kind} timeout"),
            NodeErrorSource::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// A node execution's structured failure (D-07, FT-FR-02): a plain `serde`
/// value persisted on a `Waypoint` (and, from plan 25-10, written into
/// Battlefield state) -- never a live error object.
///
/// Field order is part of this type's serde contract: `node_id`, `attempt`,
/// `transience`, `source`, in exactly this declaration order, so a JSON
/// comparison in a later plan (`node_error_round_trips_through_serde_with_stable_field_order`)
/// is deterministic.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::node_error::{NodeError, NodeErrorSource};
/// use paladin_core::platform::container::transience::Transience;
/// use paladin_core::platform::container::waypoint::NodeId;
///
/// let err = NodeError {
///     node_id: NodeId::new("summarize"),
///     attempt: 2,
///     transience: Transience::Transient,
///     source: NodeErrorSource::Function {
///         message: "connection reset".to_string(),
///     },
/// };
/// let json = serde_json::to_string(&err)?;
/// let back: NodeError = serde_json::from_str(&json)?;
/// assert_eq!(err, back);
/// # Ok::<(), serde_json::Error>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeError {
    /// The node whose execution produced this error.
    pub node_id: NodeId,
    /// Which attempt (1-indexed) produced this error.
    pub attempt: u32,
    /// Whether this failure is worth retrying.
    pub transience: Transience,
    /// The structured detail of what failed.
    pub source: NodeErrorSource,
}

// Deliberately NOT `#[derive(thiserror::Error)]`: thiserror treats a field
// literally named `source` as `std::error::Error::source()` unless the
// field type itself implements `std::error::Error` -- which
// `NodeErrorSource` (a plain serde value type, D-07) never will. A manual
// `Display` (and no `std::error::Error` impl at all -- this is a value
// type nested inside `Waypoint`/`AttemptRecord`, never propagated via `?`
// on its own) sidesteps that trap while keeping the field name the D-07
// contract requires.
impl std::fmt::Display for NodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "node {} attempt {} failed ({:?}): {}",
            self.node_id, self.attempt, self.transience, self.source
        )
    }
}

/// One retry attempt's outcome. Plan 25-07 wires this onto
/// `NodeExecutionRecord.attempts: Vec<AttemptRecord>`; declared here now so
/// that later plan only has to add the field, not design the type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttemptRecord {
    /// This attempt's 1-indexed number.
    pub attempt: u32,
    /// When this attempt started.
    pub started_at: DateTime<Utc>,
    /// How long this attempt ran, in milliseconds.
    pub duration_ms: u64,
    /// This attempt's error.
    pub error: NodeError,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_error() -> NodeError {
        NodeError {
            node_id: NodeId::new("summarize"),
            attempt: 2,
            transience: Transience::Transient,
            source: NodeErrorSource::Function {
                message: "boom".to_string(),
            },
        }
    }

    #[test]
    fn node_error_round_trips_through_serde_with_stable_field_order() {
        let err = sample_error();
        // `serde_json::to_string` walks the struct's own field-declaration
        // order (serde_json's `Serializer` writes struct fields in the
        // order `Serialize` visits them, with no reordering); going through
        // `serde_json::Value` instead would launder that order away, since
        // `Value::Object` is a `BTreeMap`-backed map (alphabetized) without
        // the `preserve_order` feature -- which this workspace does not
        // enable. Asserting on the raw string is what actually pins the
        // declaration order the D-07 contract promises.
        let json = serde_json::to_string(&err).expect("serialize");
        let positions = [
            json.find("\"node_id\"").expect("node_id present"),
            json.find("\"attempt\"").expect("attempt present"),
            json.find("\"transience\"").expect("transience present"),
            json.find("\"source\"").expect("source present"),
        ];
        assert!(
            positions.is_sorted(),
            "expected node_id, attempt, transience, source in declaration order, got {json}"
        );

        let round_tripped: NodeError =
            serde_json::from_str(&json).expect("deserialize back into NodeError");
        assert_eq!(err, round_tripped);
    }

    #[test]
    fn node_error_json_field_order_is_stable() {
        // Serialising the same value twice yields byte-identical JSON with
        // the keys in declaration order (`node_id`, `attempt`, `transience`,
        // `source`), so an `error_field` comparison (plan 25-10) or a
        // stored-Waypoint diff is deterministic. `serde_json::to_string`
        // (not `to_value`) is what pins the order -- see the sibling test.
        let err = sample_error();
        let first = serde_json::to_string(&err).expect("serialize once");
        let second = serde_json::to_string(&err).expect("serialize twice");
        assert_eq!(first, second, "byte-identical across serialisations");
        assert!(
            first.starts_with("{\"node_id\":"),
            "node_id is the first key: {first}"
        );
        let keys: Vec<&str> = ["\"node_id\"", "\"attempt\"", "\"transience\"", "\"source\""]
            .into_iter()
            .collect();
        let positions: Vec<usize> = keys
            .iter()
            .map(|k| first.find(k).expect("every declared key is present"))
            .collect();
        assert!(positions.is_sorted(), "declaration order, got {first}");
        // The nested `source` variant's own fields are likewise declaration-
        // ordered (a `Function` source has exactly one key).
        assert!(first.ends_with("\"source\":{\"Function\":{\"message\":\"boom\"}}}"));
    }

    #[test]
    fn display_includes_node_id_and_message() {
        let err = sample_error();
        let text = err.to_string();
        assert!(text.contains("summarize"));
        assert!(text.contains("boom"));
    }

    #[test]
    fn timeout_kind_display_is_stable() {
        assert_eq!(TimeoutKind::Run.to_string(), "run");
        assert_eq!(TimeoutKind::Idle.to_string(), "idle");
        assert_eq!(TimeoutKind::EngineRun.to_string(), "engine_run");
    }
}
