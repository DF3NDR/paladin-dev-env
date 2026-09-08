//! Assistant identity and the append-only immutable version aggregate (PLAT-04).
//!
//! This module defines the core types the Platform API's assistant surface
//! submits, persists and publishes over HTTP: [`AssistantId`] addresses one
//! assistant, [`Assistant`] is the persisted `assistants` row shape (plan
//! 27-09) tracking the assistant's `latest` published version, and
//! [`AssistantVersion`] is one immutable, append-only entry in that
//! assistant's version history.
//!
//! # One-way door (D-28, D-29)
//!
//! [`AssistantDefinition`] is simultaneously a persisted column pair
//! (`kind`, `body`) and the request/response body of every assistant HTTP
//! route — changing its shape after v0.10.0 means a data migration for every
//! stored version and a wire break for generated SDK clients. It is
//! deliberately an OPAQUE tagged envelope over `serde_json::Value` rather
//! than a typed `Agent(PaladinConfigDoc) | Workflow(WarGraphDoc)` pair: core
//! has no engine, so it must not name engine-shaped types (X-01, ADR-0031).
//! Only the facade (`paladin-ai`) knows how to interpret `body` for a given
//! `kind`.
//!
//! Immutability itself is enforced by the schema and the router, not by
//! handler discipline (D-29): `assistant_versions` has primary key
//! `(assistant_id, version)`, [`crate`]'s ports expose `append_version` but
//! deliberately no method that mutates an existing version, and no `PUT`
//! route is ever registered for a version. "No update, ever" holds because
//! the code to violate it does not exist — see
//! `paladin_ports::output::assistant_repository_port` for the port itself.
//!
//! # Schema versioning (X-04)
//!
//! Every persisted [`Assistant`] and [`AssistantVersion`] carries
//! [`ASSISTANT_SCHEMA_VERSION`] in its own `schema_version` field, mirroring
//! the `Run`/`Waypoint` precedent.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Schema version stamped on every persisted [`Assistant`] and
/// [`AssistantVersion`] (X-04).
pub const ASSISTANT_SCHEMA_VERSION: &str = "v1";

fn default_assistant_schema_version() -> String {
    ASSISTANT_SCHEMA_VERSION.to_string()
}

/// Maximum length, in bytes, of an [`AssistantId`] slug.
pub const ASSISTANT_ID_MAX_LEN: usize = 64;

/// Caller-supplied, URL-safe identity of an assistant: a slug matching
/// `[a-z0-9][a-z0-9_-]{0,63}` — starts with a lowercase letter or digit,
/// followed by up to 63 more lowercase letters, digits, underscores or
/// hyphens. Validated so it is safe to use as a storage key and an HTTP path
/// segment without further sanitization, mirroring
/// [`crate::platform::container::waypoint::ThreadId`]'s validation
/// discipline.
///
/// ```
/// use paladin_core::platform::container::assistant::{AssistantId, AssistantIdError};
///
/// assert!(AssistantId::new("triage-agent").is_ok());
/// assert!(AssistantId::new("v2").is_ok());
/// assert_eq!(AssistantId::new(""), Err(AssistantIdError::Empty));
/// assert!(matches!(
///     AssistantId::new("Triage-Agent"),
///     Err(AssistantIdError::InvalidFormat { .. })
/// ));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssistantId(String);

/// Error returned by [`AssistantId::new`] when the supplied string is not a
/// valid slug.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AssistantIdError {
    /// The supplied assistant id was empty.
    #[error("assistant id must not be empty")]
    Empty,
    /// The supplied assistant id exceeded [`ASSISTANT_ID_MAX_LEN`] bytes.
    #[error("assistant id must be at most {ASSISTANT_ID_MAX_LEN} characters, got {len}")]
    TooLong {
        /// The length of the rejected assistant id.
        len: usize,
    },
    /// The supplied assistant id did not match `[a-z0-9][a-z0-9_-]*`.
    #[error(
        "assistant id {value:?} must start with a lowercase letter or digit and contain only \
         lowercase letters, digits, underscores and hyphens"
    )]
    InvalidFormat {
        /// The rejected value.
        value: String,
    },
}

impl AssistantId {
    /// Construct an `AssistantId`, validating non-empty, length, and slug
    /// format (`[a-z0-9][a-z0-9_-]{0,63}`).
    pub fn new(id: impl Into<String>) -> Result<Self, AssistantIdError> {
        let id = id.into();
        if id.is_empty() {
            return Err(AssistantIdError::Empty);
        }
        if id.len() > ASSISTANT_ID_MAX_LEN {
            return Err(AssistantIdError::TooLong { len: id.len() });
        }
        let mut chars = id.chars();
        let first_ok = chars
            .next()
            .map(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            .unwrap_or(false);
        let rest_ok =
            chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
        if !first_ok || !rest_ok {
            return Err(AssistantIdError::InvalidFormat { value: id });
        }
        Ok(Self(id))
    }

    /// Borrow the assistant id as a `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AssistantId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The two shapes an [`AssistantDefinition`] can carry (D-28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistantKind {
    /// A single-Paladin (LLM + prompt) assistant.
    Agent,
    /// A `WarGraphDoc`-defined multi-node workflow assistant.
    Workflow,
}

/// An assistant's definition: a tagged envelope over opaque JSON (D-28).
///
/// `body`'s shape depends on `kind` and is validated and interpreted only by
/// the facade (`paladin-ai`) — this crate, the port crate, the storage crate
/// and the web crate carry, persist and echo it without depending on the
/// workflow engine crate (X-01, ADR-0031).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssistantDefinition {
    /// Whether `body` is an `Agent` config or a `Workflow` graph document.
    pub kind: AssistantKind,
    /// The opaque definition payload.
    pub body: serde_json::Value,
}

/// Where an [`Assistant`] came from: persisted through the API, or a
/// read-only synthetic entry for a code-registered agent (D-32).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssistantSource {
    /// Created and versioned through the assistant repository.
    Stored,
    /// A read-only synthetic entry for a code-registered agent
    /// (`assistants.expose_code_registry`, D-32); rejected by every
    /// mutating route with `code_registered_immutable`.
    Code,
}

/// One immutable, append-only version of an assistant's definition.
///
/// Construct through [`AssistantVersion::new`] plus the `with_*` builder
/// methods; `#[non_exhaustive]` keeps future field additions non-breaking
/// (X-10.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AssistantVersion {
    /// The assistant this version belongs to.
    pub assistant_id: AssistantId,
    /// This version's 1-based sequence number, unique per assistant.
    pub version: u32,
    /// The definition this version carries.
    pub definition: AssistantDefinition,
    /// When this version was created.
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// Who published this version, if known (PLAT-FR-10 audit trail).
    #[serde(default)]
    pub created_by: Option<String>,
    /// An optional publish-time note (PLAT-FR-10 audit trail).
    #[serde(default)]
    pub note: Option<String>,
    /// Schema version this row was persisted under (X-04).
    #[serde(default = "default_assistant_schema_version")]
    pub schema_version: String,
}

impl AssistantVersion {
    /// Construct a fresh `AssistantVersion`, created now, with no
    /// `created_by`/`note`.
    pub fn new(assistant_id: AssistantId, version: u32, definition: AssistantDefinition) -> Self {
        Self {
            assistant_id,
            version,
            definition,
            created_at: Utc::now(),
            created_by: None,
            note: None,
            schema_version: ASSISTANT_SCHEMA_VERSION.to_string(),
        }
    }

    /// Record who published this version.
    pub fn with_created_by(mut self, created_by: impl Into<String>) -> Self {
        self.created_by = Some(created_by.into());
        self
    }

    /// Record a publish-time note.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// The fields a caller supplies to create or append a new
/// [`AssistantVersion`] (the repository assigns `version` and
/// `created_at`).
#[derive(Debug, Clone)]
pub struct NewAssistantVersion {
    /// The definition to publish.
    pub definition: AssistantDefinition,
    /// Who is publishing this version, if known.
    pub created_by: Option<String>,
    /// An optional publish-time note.
    pub note: Option<String>,
}

/// An assistant: its identity, its `latest` published version number, and
/// bookkeeping. The version history itself lives in the repository's
/// `AssistantVersion` rows, addressed by `(assistant_id, version)`.
///
/// Construct through [`Assistant::new`] plus the `with_*` builder methods;
/// `#[non_exhaustive]` keeps future field additions non-breaking (X-10.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Assistant {
    /// This assistant's identity.
    pub assistant_id: AssistantId,
    /// The most recently published version number (1-based).
    pub latest: u32,
    /// Where this assistant came from.
    pub source: AssistantSource,
    /// When this assistant was first created.
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// When this assistant was soft-deleted, if it has been. A
    /// soft-deleted assistant's versions stay readable by `get_version` so
    /// historical runs remain reconstructable (PLAT-FR-10).
    #[serde(default)]
    pub deleted_at: Option<DateTime<Utc>>,
    /// Schema version this row was persisted under (X-04).
    #[serde(default = "default_assistant_schema_version")]
    pub schema_version: String,
}

impl Assistant {
    /// Construct a fresh `Assistant`, created now, not deleted.
    pub fn new(assistant_id: AssistantId, latest: u32, source: AssistantSource) -> Self {
        Self {
            assistant_id,
            latest,
            source,
            created_at: Utc::now(),
            deleted_at: None,
            schema_version: ASSISTANT_SCHEMA_VERSION.to_string(),
        }
    }

    /// Mark this assistant soft-deleted at `at`.
    pub fn with_deleted_at(mut self, at: DateTime<Utc>) -> Self {
        self.deleted_at = Some(at);
        self
    }

    /// Whether this assistant is soft-deleted.
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assistant_id_accepts_valid_slugs() {
        assert!(AssistantId::new("a").is_ok());
        assert!(AssistantId::new("triage-agent").is_ok());
        assert!(AssistantId::new("agent_v2").is_ok());
        assert!(AssistantId::new("9lives").is_ok());
    }

    #[test]
    fn assistant_id_rejects_empty() {
        assert_eq!(AssistantId::new(""), Err(AssistantIdError::Empty));
    }

    #[test]
    fn assistant_id_rejects_too_long() {
        let long = "a".repeat(ASSISTANT_ID_MAX_LEN + 1);
        assert_eq!(
            AssistantId::new(long.clone()),
            Err(AssistantIdError::TooLong { len: long.len() })
        );
    }

    #[test]
    fn assistant_id_accepts_exactly_max_len() {
        let max = "a".repeat(ASSISTANT_ID_MAX_LEN);
        assert!(AssistantId::new(max).is_ok());
    }

    #[test]
    fn assistant_id_rejects_uppercase() {
        assert!(matches!(
            AssistantId::new("Triage"),
            Err(AssistantIdError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn assistant_id_rejects_whitespace() {
        assert!(matches!(
            AssistantId::new("triage agent"),
            Err(AssistantIdError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn assistant_id_rejects_leading_hyphen_or_underscore() {
        assert!(matches!(
            AssistantId::new("-triage"),
            Err(AssistantIdError::InvalidFormat { .. })
        ));
        assert!(matches!(
            AssistantId::new("_triage"),
            Err(AssistantIdError::InvalidFormat { .. })
        ));
    }

    #[test]
    fn assistant_id_display_matches_as_str() {
        let id = AssistantId::new("triage-agent").unwrap();
        assert_eq!(id.to_string(), id.as_str());
    }

    #[test]
    fn assistant_kind_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&AssistantKind::Agent).unwrap(),
            "\"agent\""
        );
        assert_eq!(
            serde_json::to_string(&AssistantKind::Workflow).unwrap(),
            "\"workflow\""
        );
    }

    #[test]
    fn assistant_definition_round_trips_through_serde() {
        let def = AssistantDefinition {
            kind: AssistantKind::Workflow,
            body: serde_json::json!({ "nodes": [] }),
        };
        let json = serde_json::to_string(&def).unwrap();
        let restored: AssistantDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, def);
        assert!(json.contains("\"kind\":\"workflow\""));
    }

    #[test]
    fn assistant_source_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&AssistantSource::Stored).unwrap(),
            "\"stored\""
        );
        assert_eq!(
            serde_json::to_string(&AssistantSource::Code).unwrap(),
            "\"code\""
        );
    }

    #[test]
    fn assistant_version_builder_sets_fields() {
        let id = AssistantId::new("a1").unwrap();
        let version = AssistantVersion::new(
            id.clone(),
            1,
            AssistantDefinition {
                kind: AssistantKind::Agent,
                body: serde_json::json!({}),
            },
        )
        .with_created_by("alice")
        .with_note("initial publish");

        assert_eq!(version.assistant_id, id);
        assert_eq!(version.version, 1);
        assert_eq!(version.created_by.as_deref(), Some("alice"));
        assert_eq!(version.note.as_deref(), Some("initial publish"));
        assert_eq!(version.schema_version, ASSISTANT_SCHEMA_VERSION);
    }

    #[test]
    fn assistant_version_round_trips_through_serde_json() {
        let id = AssistantId::new("a1").unwrap();
        let version = AssistantVersion::new(
            id,
            1,
            AssistantDefinition {
                kind: AssistantKind::Agent,
                body: serde_json::json!({ "prompt": "hi" }),
            },
        );
        let json = serde_json::to_string(&version).unwrap();
        let restored: AssistantVersion = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.assistant_id, version.assistant_id);
        assert_eq!(restored.version, version.version);
        assert_eq!(restored.definition, version.definition);
        assert_eq!(restored.schema_version, ASSISTANT_SCHEMA_VERSION);
    }

    #[test]
    fn assistant_version_deserializes_with_only_required_fields_present() {
        // X-10.3: a JSON document missing every field added after the
        // initial set still deserializes, defaulting created_by/note to
        // None and schema_version to the current constant.
        let minimal = serde_json::json!({
            "assistant_id": "a1",
            "version": 1,
            "definition": { "kind": "agent", "body": {} },
        });
        let version: AssistantVersion = serde_json::from_str(&minimal.to_string()).unwrap();
        assert!(version.created_by.is_none());
        assert!(version.note.is_none());
        assert_eq!(version.schema_version, ASSISTANT_SCHEMA_VERSION);
    }

    #[test]
    fn assistant_builder_sets_deleted_at() {
        let id = AssistantId::new("a1").unwrap();
        let at = Utc::now();
        let assistant = Assistant::new(id, 1, AssistantSource::Stored).with_deleted_at(at);
        assert_eq!(assistant.deleted_at, Some(at));
        assert!(assistant.is_deleted());
    }

    #[test]
    fn assistant_new_is_not_deleted_by_default() {
        let id = AssistantId::new("a1").unwrap();
        let assistant = Assistant::new(id, 1, AssistantSource::Stored);
        assert!(!assistant.is_deleted());
        assert_eq!(assistant.schema_version, ASSISTANT_SCHEMA_VERSION);
    }

    #[test]
    fn assistant_round_trips_through_serde_json() {
        let id = AssistantId::new("a1").unwrap();
        let assistant = Assistant::new(id.clone(), 3, AssistantSource::Code);
        let json = serde_json::to_string(&assistant).unwrap();
        let restored: Assistant = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.assistant_id, id);
        assert_eq!(restored.latest, 3);
        assert_eq!(restored.source, assistant.source);
    }

    #[test]
    fn assistant_deserializes_with_only_required_fields_present() {
        let minimal = serde_json::json!({
            "assistant_id": "a1",
            "latest": 1,
            "source": "stored",
        });
        let assistant: Assistant = serde_json::from_str(&minimal.to_string()).unwrap();
        assert!(assistant.deleted_at.is_none());
        assert_eq!(assistant.schema_version, ASSISTANT_SCHEMA_VERSION);
    }
}
