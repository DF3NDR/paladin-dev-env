//! Parley — Human-in-the-Loop Pause/Resume Value Types (HITL-01, D-01)
//!
//! A [`ParleyRequest`] is one piece of external input a `WarGraph` run
//! (`paladin-battalion`) is waiting on: raised by a node's
//! `NextStep::Parley` (`paladin_core::platform::container::directive`),
//! suspending the run until a matching [`ParleyResponse`] is delivered
//! through `WarEngine::resume_with`. Every type here is a pure value type —
//! `paladin-core` adds no new dependency: `chrono`, `uuid` and `serde_json`
//! are already crate dependencies (ADR-0015), and core owns these port
//! value types per ADR-0016.
//!
//! [`ParleyId`] mirrors [`crate::platform::container::waypoint::WaypointId`]
//! exactly: a `#[serde(transparent)]` UUIDv7 newtype, so a fresh id sorts
//! chronologically alongside every other id in this codebase.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::platform::container::waypoint::NodeId;

/// Engine-generated identity of one outstanding [`ParleyRequest`], a UUIDv7
/// (time-ordered) value — mirrors [`crate::platform::container::waypoint::WaypointId`]'s
/// shape and constructor exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParleyId(Uuid);

impl ParleyId {
    /// Generate a fresh, time-ordered `ParleyId`.
    ///
    /// ```
    /// use paladin_core::platform::container::parley::ParleyId;
    ///
    /// let first = ParleyId::new();
    /// let second = ParleyId::new();
    /// assert_ne!(first, second);
    /// ```
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Borrow the underlying `Uuid`.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ParleyId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ParleyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The shape of input a [`ParleyRequest`] awaits (HITL-01, HITL-FR-04, D-01).
///
/// `#[non_exhaustive]`: a future kind can be added without breaking an
/// existing match, mirroring every other persisted enum in this module tree
/// (`WaypointStatus`, `NodeOutcomeKind`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ParleyKind {
    /// A yes/no decision, normalised to JSON `true`/`false` on delivery
    /// (HITL-FR-05).
    Approval,
    /// A choice among the request's `choices` list.
    Choice,
    /// Free-form text input.
    FreeText,
    /// A structured state edit, validated against the graph schema at
    /// resume time.
    StateEdit,
}

/// What happens to a [`ParleyRequest`] whose `expires_at` passes before it
/// is answered (HITL-FR-06, D-12). Evaluated lazily, at resume time,
/// against `Utc::now()` — no timer, no clock abstraction (D-13).
///
/// `#[non_exhaustive]`, matching [`ParleyKind`]'s open-ended shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[non_exhaustive]
pub enum OnExpire {
    /// Fail the run with a structured reason naming the parley (default).
    #[default]
    FailRun,
    /// Substitute this value as the response, `responded_by: None`, and the
    /// substitution recorded on the resulting [`ParleyResponse::defaulted`].
    ResumeWithDefault(serde_json::Value),
}

/// A pause point raised by a node's `NextStep::Parley` (HITL-01, HITL-FR-03,
/// D-01): the full description of one piece of outstanding external input,
/// persisted verbatim on the suspending `Waypoint`'s
/// `WaypointStatus::AwaitingInput.parleys` list.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), serde_json::Error> {
/// use paladin_core::platform::container::parley::{OnExpire, ParleyId, ParleyKind, ParleyRequest};
/// use paladin_core::platform::container::waypoint::NodeId;
/// use chrono::Utc;
///
/// let request = ParleyRequest {
///     parley_id: ParleyId::new(),
///     node_id: NodeId::new("approve"),
///     kind: ParleyKind::Approval,
///     prompt: "Proceed with the deploy?".to_string(),
///     payload: serde_json::json!({}),
///     choices: None,
///     expires_at: None,
///     created_at: Utc::now(),
///     on_expire: OnExpire::FailRun,
/// };
///
/// // A `ParleyRequest` round-trips through serde with every field intact —
/// // this is a stored contract (D-01): the exact shape a persisted
/// // `Waypoint` payload carries.
/// let json = serde_json::to_string(&request)?;
/// let restored: ParleyRequest = serde_json::from_str(&json)?;
/// assert_eq!(restored.parley_id, request.parley_id);
/// assert_eq!(restored.node_id, request.node_id);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParleyRequest {
    /// This request's identity, matched against a submitted
    /// [`ParleyResponse::parley_id`] on resume.
    pub parley_id: ParleyId,
    /// The node that raised this request.
    pub node_id: NodeId,
    /// The shape of input awaited.
    pub kind: ParleyKind,
    /// Free-form prompt describing what input is being awaited.
    pub prompt: String,
    /// Author-supplied context rendered alongside `prompt`. Never contains
    /// an API key, secret, or provider credential — the raise path copies
    /// only author-supplied template output (T-24-02).
    pub payload: serde_json::Value,
    /// Valid choices, populated for [`ParleyKind::Choice`]; `None`
    /// otherwise.
    pub choices: Option<Vec<String>>,
    /// When this request expires, if ever (D-12, D-13).
    pub expires_at: Option<DateTime<Utc>>,
    /// When this request was raised.
    pub created_at: DateTime<Utc>,
    /// What happens if this request expires unanswered.
    pub on_expire: OnExpire,
}

/// One answer to an outstanding [`ParleyRequest`] (HITL-02, HITL-FR-04,
/// D-01), submitted through `WarEngine::resume_with`.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), serde_json::Error> {
/// use paladin_core::platform::container::parley::{ParleyId, ParleyKind, ParleyResponse};
/// use chrono::Utc;
///
/// let response = ParleyResponse {
///     parley_id: ParleyId::new(),
///     kind: ParleyKind::Approval,
///     prompt: "Proceed with the deploy?".to_string(),
///     value: serde_json::json!(true),
///     responded_by: Some("ops@example.com".to_string()),
///     responded_at: Utc::now(),
///     defaulted: false,
/// };
///
/// let json = serde_json::to_string(&response)?;
/// let restored: ParleyResponse = serde_json::from_str(&json)?;
/// assert_eq!(restored.parley_id, response.parley_id);
/// assert!(!restored.defaulted);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParleyResponse {
    /// The request this response answers.
    pub parley_id: ParleyId,
    /// The originating [`ParleyRequest::kind`] this response answers,
    /// stamped by `WarEngine::resume_with` from the matching request
    /// regardless of what value an external caller supplied when
    /// submitting this response (mirrors [`ParleyRequest::node_id`]'s own
    /// engine-stamped-regardless contract, 24-01). This lets the
    /// `parley.kind` `InputMapping` placeholder (24-03, D-07) resolve
    /// entirely from this one type, with no `NodeContext`-only side
    /// channel duplicating what is already recorded on the request.
    pub kind: ParleyKind,
    /// The originating [`ParleyRequest::prompt`] this response answers,
    /// stamped the same way [`Self::kind`] is (see its own rustdoc) — never
    /// meaningfully supplied by an external caller.
    pub prompt: String,
    /// The submitted value, validated per [`ParleyKind`] at `resume_with`
    /// (the richer validation matrix lands in a later plan; this phase's
    /// `resume_with` accepts any value shape on its happy path).
    pub value: serde_json::Value,
    /// Who submitted this response, if known. `None` for a
    /// [`OnExpire::ResumeWithDefault`] substitution.
    pub responded_by: Option<String>,
    /// When this response was recorded.
    pub responded_at: DateTime<Utc>,
    /// Whether this response was substituted by
    /// [`OnExpire::ResumeWithDefault`] rather than submitted by a caller
    /// (D-12). `#[serde(default)]` so a response recorded before this field
    /// existed still loads, as `false`.
    #[serde(default)]
    pub defaulted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parley_id_new_generates_distinct_time_ordered_values() {
        let a = ParleyId::new();
        let b = ParleyId::new();
        assert_ne!(a, b);
        // UUIDv7 is time-ordered: a later id is greater when compared as a
        // raw UUID, mirroring `WaypointId`'s own ordering guarantee.
        assert!(b.as_uuid() >= a.as_uuid());
    }

    #[test]
    fn parley_id_default_generates_a_fresh_value() {
        assert_ne!(ParleyId::default(), ParleyId::default());
    }

    #[test]
    fn parley_id_display_matches_uuid_string() {
        let id = ParleyId::new();
        assert_eq!(id.to_string(), id.as_uuid().to_string());
    }

    fn sample_request() -> ParleyRequest {
        ParleyRequest {
            parley_id: ParleyId::new(),
            node_id: NodeId::new("approve"),
            kind: ParleyKind::Approval,
            prompt: "confirm?".to_string(),
            payload: serde_json::json!({"amount": 100}),
            choices: None,
            expires_at: None,
            created_at: Utc::now(),
            on_expire: OnExpire::FailRun,
        }
    }

    #[test]
    fn parley_request_round_trips_through_serde() {
        let request = sample_request();
        let json = serde_json::to_string(&request).unwrap();
        let restored: ParleyRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request, restored);
    }

    #[test]
    fn parley_request_with_choice_kind_round_trips_choices() {
        let mut request = sample_request();
        request.kind = ParleyKind::Choice;
        request.choices = Some(vec!["yes".to_string(), "no".to_string()]);
        let json = serde_json::to_string(&request).unwrap();
        let restored: ParleyRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.choices, request.choices);
    }

    #[test]
    fn on_expire_resume_with_default_round_trips_its_value() {
        let mut request = sample_request();
        request.on_expire = OnExpire::ResumeWithDefault(serde_json::json!("fallback"));
        let json = serde_json::to_string(&request).unwrap();
        let restored: ParleyRequest = serde_json::from_str(&json).unwrap();
        match restored.on_expire {
            OnExpire::ResumeWithDefault(v) => assert_eq!(v, serde_json::json!("fallback")),
            other => panic!("expected ResumeWithDefault, got {other:?}"),
        }
    }

    #[test]
    fn on_expire_defaults_to_fail_run() {
        assert_eq!(OnExpire::default(), OnExpire::FailRun);
    }

    #[test]
    fn parley_response_round_trips_through_serde() {
        let response = ParleyResponse {
            parley_id: ParleyId::new(),
            kind: ParleyKind::Approval,
            prompt: "confirm?".to_string(),
            value: serde_json::json!(true),
            responded_by: Some("alice".to_string()),
            responded_at: Utc::now(),
            defaulted: false,
        };
        let json = serde_json::to_string(&response).unwrap();
        let restored: ParleyResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(response, restored);
    }

    #[test]
    fn parley_response_without_defaulted_key_deserializes_as_false() {
        // D-12: `defaulted` is `#[serde(default)]` -- a payload written
        // before this field existed (or a hand-built envelope that omits
        // it) must still deserialize, defaulting to `false`. `kind`/`prompt`
        // are NOT `#[serde(default)]` (added same-phase, pre-release, 24-03
        // -- no released Waypoint predates them) so this fixture supplies
        // both explicitly.
        let mut value = serde_json::json!({
            "parley_id": ParleyId::new(),
            "kind": "Approval",
            "prompt": "confirm?",
            "value": true,
            "responded_by": null,
            "responded_at": Utc::now(),
        });
        assert!(value.get("defaulted").is_none());
        let restored: ParleyResponse = serde_json::from_value(value.clone()).unwrap();
        assert!(!restored.defaulted);

        value["defaulted"] = serde_json::json!(true);
        let restored_defaulted: ParleyResponse = serde_json::from_value(value).unwrap();
        assert!(restored_defaulted.defaulted);
    }
}
