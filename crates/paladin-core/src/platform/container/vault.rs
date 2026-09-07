//! The Vault — cross-thread namespaced key/value memory (Doc 05 RT-FR-13…16, D-18/D-19).
//!
//! Paladin ships three distinct "memory" concepts, and it is easy to reach
//! for the wrong one. This table is the canonical distinction; `VaultPort`'s
//! own rustdoc (`paladin_ports::output::vault_port`) repeats it for readers
//! who land there first:
//!
//! | | Vault | Garrison | Waypoint |
//! |---|---|---|---|
//! | **Scope** | cross-thread namespaced key/value | one conversation's transcript | one run's durable engine state |
//! | **Lifetime** | until explicitly deleted | the conversation | the retention policy |
//! | **Addressed by** | `(Namespace, key)` | a `Garrison` instance | `(ThreadId, WaypointId)` |
//! | **Who writes** | the host, or the agent through a confined tool | the execution service | the `WarEngine` |
//! | **Typical content** | durable facts about a user or a domain | conversation turns | Battlefield snapshots and Frontier state |
//!
//! This module adds **no new dependency** to `paladin-core` (ADR-0015): every
//! type here is built from `serde`, `serde_json`, `chrono` and `thiserror`,
//! all already present in this crate's `Cargo.toml`. The value types live
//! here and the port trait lives in `paladin_ports::output::vault_port`
//! (ADR-0016: core owns port value types), which re-exports them so a
//! consumer needs only one `use`.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The maximum size, in bytes, a [`VaultRecord`] value may serialize to
/// unless an adapter overrides it with its own bound (e.g.
/// `InMemoryVault::with_max_value_bytes`). 64 KiB.
pub const DEFAULT_MAX_VALUE_BYTES: usize = 64 * 1024;

const MIN_NAMESPACE_SEGMENTS: usize = 1;
const MAX_NAMESPACE_SEGMENTS: usize = 16;
const MIN_SEGMENT_CHARS: usize = 1;
const MAX_SEGMENT_CHARS: usize = 64;
const MIN_KEY_CHARS: usize = 1;
const MAX_KEY_CHARS: usize = 256;
const MAX_PAGE_LIMIT: u32 = 1000;
const DEFAULT_PAGE_LIMIT: u32 = 50;

/// A validated, ordered sequence of path segments addressing a region of the
/// Vault -- e.g. `["user", "alice", "prefs"]`, displayed as
/// `user/alice/prefs`.
///
/// # Invariants
///
/// - 1 to 16 segments (inclusive).
/// - Each segment is 1 to 64 **characters** (not bytes -- a multi-byte
///   segment is never rejected for being long in UTF-8 byte length).
/// - No segment contains `/`.
/// - No segment is exactly `.` or `..`.
/// - No segment contains a control character.
///
/// The inner `Vec<String>` is private so these invariants cannot be
/// bypassed by direct field construction -- every `Namespace` in existence
/// has already passed [`Namespace::new`] or [`Namespace::parse`].
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::vault::Namespace;
///
/// let ns = Namespace::parse("user/alice/prefs")?;
/// assert_eq!(ns.to_string(), "user/alice/prefs");
/// # Ok::<(), paladin_core::platform::container::vault::VaultError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "Vec<String>", into = "Vec<String>")]
pub struct Namespace(Vec<String>);

impl Namespace {
    /// Constructs a [`Namespace`] from an owned list of segments, validating
    /// every invariant documented on the type.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::InvalidNamespace`] naming the first violated
    /// rule if the segment list is empty, too long, contains an
    /// out-of-bounds segment, a `/`, a `.` or `..` segment, or a control
    /// character.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::vault::Namespace;
    ///
    /// let ns = Namespace::new(vec!["user", "alice"])?;
    /// assert_eq!(ns.segments(), &["user".to_string(), "alice".to_string()]);
    /// # Ok::<(), paladin_core::platform::container::vault::VaultError>(())
    /// ```
    pub fn new(segments: Vec<impl Into<String>>) -> Result<Self, VaultError> {
        let segments: Vec<String> = segments.into_iter().map(Into::into).collect();
        Self::validate_segments(&segments)?;
        Ok(Self(segments))
    }

    /// Parses a `/`-joined string into a [`Namespace`], validating every
    /// invariant documented on the type.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::InvalidNamespace`] under the same conditions as
    /// [`Namespace::new`].
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::vault::Namespace;
    ///
    /// let ns = Namespace::parse("user/alice")?;
    /// assert_eq!(ns.segments().len(), 2);
    /// # Ok::<(), paladin_core::platform::container::vault::VaultError>(())
    /// ```
    pub fn parse(joined: &str) -> Result<Self, VaultError> {
        let segments: Vec<String> = joined.split('/').map(str::to_string).collect();
        Self::validate_segments(&segments)?;
        Ok(Self(segments))
    }

    fn validate_segments(segments: &[String]) -> Result<(), VaultError> {
        if segments.len() < MIN_NAMESPACE_SEGMENTS {
            return Err(VaultError::InvalidNamespace {
                reason: "a namespace must have at least one segment".to_string(),
            });
        }
        if segments.len() > MAX_NAMESPACE_SEGMENTS {
            return Err(VaultError::InvalidNamespace {
                reason: format!(
                    "a namespace may have at most {MAX_NAMESPACE_SEGMENTS} segments, got {}",
                    segments.len()
                ),
            });
        }
        for segment in segments {
            let char_count = segment.chars().count();
            if char_count < MIN_SEGMENT_CHARS {
                return Err(VaultError::InvalidNamespace {
                    reason: "a namespace segment must not be empty".to_string(),
                });
            }
            if char_count > MAX_SEGMENT_CHARS {
                return Err(VaultError::InvalidNamespace {
                    reason: format!(
                        "a namespace segment must be at most {MAX_SEGMENT_CHARS} characters, \
                         got {char_count} in {segment:?}"
                    ),
                });
            }
            if segment.contains('/') {
                return Err(VaultError::InvalidNamespace {
                    reason: format!("a namespace segment must not contain '/': {segment:?}"),
                });
            }
            if segment == "." || segment == ".." {
                return Err(VaultError::InvalidNamespace {
                    reason: format!("a namespace segment must not be '.' or '..': {segment:?}"),
                });
            }
            if segment.chars().any(|c| c.is_control()) {
                return Err(VaultError::InvalidNamespace {
                    reason: format!(
                        "a namespace segment must not contain a control character: {segment:?}"
                    ),
                });
            }
        }
        Ok(())
    }

    /// Borrows this namespace's segments in order.
    pub fn segments(&self) -> &[String] {
        &self.0
    }

    /// Whether `self` is `other` or an ancestor of `other`.
    ///
    /// This is the Vault's sole confinement primitive
    /// (`ConfinedVault`, plan 26-13, is built entirely on this one method) and
    /// it is deliberately **segment-wise**, never a string-prefix comparison
    /// on the joined `Display` form. A string comparison would let
    /// `["user", "alice"]` wrongly match `["user", "alice2"]`, because the
    /// joined string `"user/alice2"` starts with `"user/alice"` even though
    /// `alice2` is a sibling namespace, not a descendant of `alice`. Comparing
    /// `Vec<String>` element by element makes that sibling case structurally
    /// impossible to confuse with a true descendant.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin_core::platform::container::vault::Namespace;
    ///
    /// let grant = Namespace::parse("user/alice")?;
    /// let descendant = Namespace::parse("user/alice/prefs")?;
    /// let sibling = Namespace::parse("user/alice2")?;
    ///
    /// assert!(grant.is_prefix_of(&descendant));
    /// assert!(!grant.is_prefix_of(&sibling));
    /// assert!(grant.is_prefix_of(&grant)); // a grant contains itself
    /// # Ok::<(), paladin_core::platform::container::vault::VaultError>(())
    /// ```
    pub fn is_prefix_of(&self, other: &Namespace) -> bool {
        if self.0.len() > other.0.len() {
            return false;
        }
        self.0.iter().zip(other.0.iter()).all(|(a, b)| a == b)
    }
}

impl fmt::Display for Namespace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join("/"))
    }
}

impl TryFrom<Vec<String>> for Namespace {
    type Error = VaultError;

    fn try_from(segments: Vec<String>) -> Result<Self, Self::Error> {
        Self::validate_segments(&segments)?;
        Ok(Self(segments))
    }
}

impl From<Namespace> for Vec<String> {
    fn from(ns: Namespace) -> Self {
        ns.0
    }
}

/// A single record stored in the Vault, addressed by `(namespace, key)`.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::vault::{Namespace, VaultRecord};
/// use serde_json::json;
///
/// let ns = Namespace::parse("user/alice")?;
/// let record = VaultRecord::new(ns, "favorite_color", json!("blue"))?;
/// assert_eq!(record.key(), "favorite_color");
/// assert_eq!(record.created_at(), record.updated_at());
/// # Ok::<(), paladin_core::platform::container::vault::VaultError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultRecord {
    namespace: Namespace,
    key: String,
    value: serde_json::Value,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl VaultRecord {
    /// Constructs a new [`VaultRecord`] with `created_at == updated_at ==
    /// Utc::now()`, validating the key's length and the value's serialized
    /// size against [`DEFAULT_MAX_VALUE_BYTES`].
    ///
    /// Adapters that support a configurable bound should validate against
    /// their own bound explicitly rather than relying solely on this
    /// constructor's default; see `InMemoryVault::with_max_value_bytes`.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::InvalidKey`] if `key` is empty or longer than
    /// 256 characters. Returns [`VaultError::ValueTooLarge`] if `value`
    /// serializes to more than [`DEFAULT_MAX_VALUE_BYTES`] bytes. Returns
    /// [`VaultError::Serialization`] if `value` cannot be serialized at all.
    pub fn new(
        namespace: Namespace,
        key: impl Into<String>,
        value: serde_json::Value,
    ) -> Result<Self, VaultError> {
        Self::new_with_bound(namespace, key, value, DEFAULT_MAX_VALUE_BYTES)
    }

    /// Like [`VaultRecord::new`], but validates the value's serialized size
    /// against an explicit `max_value_bytes` bound rather than the crate
    /// default -- the hook an adapter's configurable bound uses.
    ///
    /// # Errors
    ///
    /// Same as [`VaultRecord::new`], but the size check is against
    /// `max_value_bytes`.
    pub fn new_with_bound(
        namespace: Namespace,
        key: impl Into<String>,
        value: serde_json::Value,
        max_value_bytes: usize,
    ) -> Result<Self, VaultError> {
        let key = key.into();
        validate_key(&key)?;
        validate_value_size(&value, max_value_bytes)?;
        let now = Utc::now();
        Ok(Self {
            namespace,
            key,
            value,
            created_at: now,
            updated_at: now,
        })
    }

    /// This record's namespace.
    pub fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    /// This record's key.
    pub fn key(&self) -> &str {
        &self.key
    }

    /// This record's stored value.
    pub fn value(&self) -> &serde_json::Value {
        &self.value
    }

    /// When this record was first created.
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// When this record was last written.
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Replaces this record's value and bumps `updated_at` to now, leaving
    /// `created_at` unchanged -- the shape every `VaultPort::put` overwrite
    /// must reproduce.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::ValueTooLarge`] if `value` serializes to more
    /// than [`DEFAULT_MAX_VALUE_BYTES`] bytes.
    pub fn overwrite_value(&mut self, value: serde_json::Value) -> Result<(), VaultError> {
        self.overwrite_value_with_bound(value, DEFAULT_MAX_VALUE_BYTES)
    }

    /// Like [`VaultRecord::overwrite_value`], but validates against an
    /// explicit `max_value_bytes` bound.
    ///
    /// # Errors
    ///
    /// Returns [`VaultError::ValueTooLarge`] if `value` serializes to more
    /// than `max_value_bytes` bytes.
    pub fn overwrite_value_with_bound(
        &mut self,
        value: serde_json::Value,
        max_value_bytes: usize,
    ) -> Result<(), VaultError> {
        validate_value_size(&value, max_value_bytes)?;
        self.value = value;
        self.updated_at = Utc::now();
        Ok(())
    }
}

fn validate_key(key: &str) -> Result<(), VaultError> {
    let char_count = key.chars().count();
    if !(MIN_KEY_CHARS..=MAX_KEY_CHARS).contains(&char_count) {
        return Err(VaultError::InvalidKey {
            reason: format!(
                "a key must be between {MIN_KEY_CHARS} and {MAX_KEY_CHARS} characters, got \
                 {char_count}"
            ),
        });
    }
    Ok(())
}

fn validate_value_size(
    value: &serde_json::Value,
    max_value_bytes: usize,
) -> Result<(), VaultError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|e| VaultError::Serialization {
            message: e.to_string(),
        })?
        .len();
    if bytes > max_value_bytes {
        return Err(VaultError::ValueTooLarge {
            bytes,
            max: max_value_bytes,
        });
    }
    Ok(())
}

/// A [`VaultRecord`] paired with a relevance score, returned by
/// [`VaultPort::search`](../../../../paladin_ports/output/vault_port/trait.VaultPort.html#tymethod.search)
/// implementations that support it.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::vault::{Namespace, ScoredVaultRecord, VaultRecord};
/// use serde_json::json;
///
/// let ns = Namespace::parse("user/alice")?;
/// let record = VaultRecord::new(ns, "note", json!("likes tea"))?;
/// let scored = ScoredVaultRecord::new(record, 0.87);
/// assert!((scored.score() - 0.87).abs() < f32::EPSILON);
/// # Ok::<(), paladin_core::platform::container::vault::VaultError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoredVaultRecord {
    record: VaultRecord,
    score: f32,
}

impl ScoredVaultRecord {
    /// Pairs a [`VaultRecord`] with its relevance `score`.
    pub fn new(record: VaultRecord, score: f32) -> Self {
        Self { record, score }
    }

    /// The scored record.
    pub fn record(&self) -> &VaultRecord {
        &self.record
    }

    /// The relevance score, backend-defined (typically cosine similarity
    /// in `[0.0, 1.0]`, but not asserted by this type).
    pub fn score(&self) -> f32 {
        self.score
    }
}

/// A pagination request for [`VaultPort::list`](../../../../paladin_ports/output/vault_port/trait.VaultPort.html#tymethod.list).
///
/// `after` is the last key returned by the previous page and is opaque to
/// callers -- treat it as an unstructured cursor, never as a value to
/// construct or compare directly.
///
/// # Examples
///
/// ```
/// use paladin_core::platform::container::vault::Page;
///
/// let page = Page::default();
/// assert_eq!(page.limit(), 50);
/// assert_eq!(page.after(), None);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    limit: u32,
    after: Option<String>,
}

impl Page {
    /// Constructs a [`Page`], rejecting a `limit` above 1000.
    ///
    /// # Errors
    ///
    /// Returns `Err(VaultError::Storage { message })` naming the
    /// out-of-range value if `limit` exceeds 1000 -- pagination is a
    /// request-shaping concern, not a domain invariant on stored data,
    /// hence the adapter-facing variant rather than a dedicated one.
    pub fn new(limit: u32, after: Option<String>) -> Result<Self, VaultError> {
        if limit > MAX_PAGE_LIMIT {
            return Err(VaultError::Storage {
                message: format!("page limit must be at most {MAX_PAGE_LIMIT}, got {limit}"),
            });
        }
        Ok(Self { limit, after })
    }

    /// The maximum number of records to return.
    pub fn limit(&self) -> u32 {
        self.limit
    }

    /// The opaque cursor from a previous page's last returned key.
    pub fn after(&self) -> Option<&str> {
        self.after.as_deref()
    }
}

impl Default for Page {
    fn default() -> Self {
        Self {
            limit: DEFAULT_PAGE_LIMIT,
            after: None,
        }
    }
}

/// Errors a [`VaultPort`](../../../../paladin_ports/output/vault_port/trait.VaultPort.html)
/// implementation can report.
///
/// `#[non_exhaustive]`: this taxonomy may grow (a future capability might
/// need its own variant) without that being a semver break for downstream
/// matchers, per X-06.
///
/// `Storage` and `Serialization` are the two adapter-boundary variants where
/// a backend's own error text is summarised into a `String`. Any text
/// entering either variant MUST be redacted before it is bounded (D-34's
/// rule, applied here preemptively so no adapter has to rediscover it) --
/// see `paladin_llm::redaction`.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum VaultError {
    /// A call's namespace is not covered by the caller's granted namespace.
    #[error("namespace {requested} is not within the granted namespace {granted}")]
    NamespaceDenied {
        /// The namespace the call attempted to access.
        requested: Namespace,
        /// The namespace the caller was actually granted.
        granted: Namespace,
    },

    /// A namespace failed construction validation.
    #[error("invalid namespace: {reason}")]
    InvalidNamespace {
        /// Which invariant was violated, in prose.
        reason: String,
    },

    /// A key failed construction validation.
    #[error("invalid key: {reason}")]
    InvalidKey {
        /// Which invariant was violated, in prose.
        reason: String,
    },

    /// A value's serialized size exceeded the configured bound.
    #[error("value too large: {bytes} bytes exceeds the maximum of {max} bytes")]
    ValueTooLarge {
        /// The value's actual serialized size, in bytes.
        bytes: usize,
        /// The maximum allowed size, in bytes.
        max: usize,
    },

    /// The requested operation is not supported by this backend.
    #[error("unsupported operation: {operation}")]
    Unsupported {
        /// The name of the unsupported operation, e.g. `"search"`.
        operation: &'static str,
    },

    /// The underlying storage backend failed. Adapter-boundary variant --
    /// its `message` must be redacted before it is bounded (D-34).
    #[error("vault storage error: {message}")]
    Storage {
        /// A redacted, bounded summary of the backend's own error text.
        message: String,
    },

    /// A value could not be serialized or deserialized. Adapter-boundary
    /// variant -- its `message` must be redacted before it is bounded
    /// (D-34).
    #[error("vault serialization error: {message}")]
    Serialization {
        /// A redacted, bounded summary of the underlying (de)serialization
        /// error.
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- Test 1: namespace_accepts_valid_segment_lists ---

    #[test]
    fn namespace_accepts_valid_segment_lists() {
        assert!(Namespace::new(vec!["user"]).is_ok());
        assert!(Namespace::new(vec!["user", "alice", "prefs"]).is_ok());

        let sixteen: Vec<String> = (0..16).map(|i| format!("seg{i}")).collect();
        assert!(Namespace::new(sixteen).is_ok());

        let parsed = Namespace::parse("user/alice/prefs").unwrap();
        let constructed = Namespace::new(vec!["user", "alice", "prefs"]).unwrap();
        assert_eq!(parsed, constructed);
    }

    // --- Test 2: namespace_rejects_every_invalid_shape ---

    #[test]
    fn namespace_rejects_every_invalid_shape_zero_segments() {
        let empty: Vec<String> = vec![];
        let err = Namespace::new(empty).unwrap_err();
        assert!(matches!(err, VaultError::InvalidNamespace { .. }));
    }

    #[test]
    fn namespace_rejects_every_invalid_shape_seventeen_segments() {
        let seventeen: Vec<String> = (0..17).map(|i| format!("seg{i}")).collect();
        let err = Namespace::new(seventeen).unwrap_err();
        assert!(matches!(err, VaultError::InvalidNamespace { .. }));
    }

    #[test]
    fn namespace_rejects_every_invalid_shape_empty_segment() {
        let err = Namespace::new(vec!["user", ""]).unwrap_err();
        assert!(matches!(err, VaultError::InvalidNamespace { .. }));
    }

    #[test]
    fn namespace_rejects_every_invalid_shape_sixty_five_char_segment() {
        let too_long = "a".repeat(65);
        let err = Namespace::new(vec![too_long]).unwrap_err();
        assert!(matches!(err, VaultError::InvalidNamespace { .. }));
    }

    #[test]
    fn namespace_rejects_every_invalid_shape_segment_with_slash() {
        let err = Namespace::new(vec!["user/alice"]).unwrap_err();
        assert!(matches!(err, VaultError::InvalidNamespace { .. }));
    }

    #[test]
    fn namespace_rejects_every_invalid_shape_dot_segment() {
        let err = Namespace::new(vec!["user", "."]).unwrap_err();
        assert!(matches!(err, VaultError::InvalidNamespace { .. }));
    }

    #[test]
    fn namespace_rejects_every_invalid_shape_dotdot_segment() {
        let err = Namespace::new(vec!["user", ".."]).unwrap_err();
        assert!(matches!(err, VaultError::InvalidNamespace { .. }));
    }

    #[test]
    fn namespace_rejects_every_invalid_shape_control_character() {
        let err = Namespace::new(vec!["user\u{0007}"]).unwrap_err();
        assert!(matches!(err, VaultError::InvalidNamespace { .. }));
    }

    #[test]
    fn namespace_rejects_every_invalid_shape() {
        // Aggregate named test the plan's <verify> filters on; each case is
        // also asserted individually above so a failure names the exact
        // violated rule.
        namespace_rejects_every_invalid_shape_zero_segments();
        namespace_rejects_every_invalid_shape_seventeen_segments();
        namespace_rejects_every_invalid_shape_empty_segment();
        namespace_rejects_every_invalid_shape_sixty_five_char_segment();
        namespace_rejects_every_invalid_shape_segment_with_slash();
        namespace_rejects_every_invalid_shape_dot_segment();
        namespace_rejects_every_invalid_shape_dotdot_segment();
        namespace_rejects_every_invalid_shape_control_character();
    }

    // --- Test 3: is_prefix_of_is_segment_wise_not_string_wise ---

    #[test]
    fn is_prefix_of_is_segment_wise_not_string_wise() {
        let alice = Namespace::parse("user/alice").unwrap();
        let alice_prefs = Namespace::parse("user/alice/prefs").unwrap();
        let alice2 = Namespace::parse("user/alice2").unwrap();

        assert!(alice.is_prefix_of(&alice_prefs));
        assert!(
            !alice.is_prefix_of(&alice2),
            "sibling namespace user/alice2 must not match the grant user/alice, even though \
             the joined string starts with it"
        );
        assert!(alice.is_prefix_of(&alice), "a grant contains itself");
        assert!(!alice_prefs.is_prefix_of(&alice));
    }

    // --- Test 4: display_and_parse_round_trip ---

    #[test]
    fn display_and_parse_round_trip() {
        for segments in [
            vec!["user".to_string()],
            vec!["user".to_string(), "alice".to_string(), "prefs".to_string()],
            (0..16).map(|i| format!("seg{i}")).collect::<Vec<_>>(),
        ] {
            let ns = Namespace::new(segments).unwrap();
            let rendered = ns.to_string();
            let reparsed = Namespace::parse(&rendered).unwrap();
            assert_eq!(ns, reparsed);
        }
    }

    // --- Test 5: vault_record_and_page_round_trip_through_serde ---

    #[test]
    fn vault_record_and_page_round_trip_through_serde() {
        let ns = Namespace::parse("user/alice").unwrap();
        let record = VaultRecord::new(ns, "k", json!(1)).unwrap();

        let json = serde_json::to_string(&record).unwrap();
        let back: VaultRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, back);

        let page = Page::default();
        let json = serde_json::to_string(&page).unwrap();
        let back: Page = serde_json::from_str(&json).unwrap();
        assert_eq!(page, back);

        assert_eq!(Page::default().limit(), 50);
        assert_eq!(Page::default().after(), None);
        assert!(Page::new(1001, None).is_err());
        assert!(Page::new(1000, None).is_ok());
    }

    // --- Test 6: vault_error_variants_are_structured_and_non_exhaustive ---

    #[test]
    fn vault_error_variants_are_structured_and_non_exhaustive() {
        let ns_a = Namespace::parse("user/alice").unwrap();
        let ns_b = Namespace::parse("user/bob").unwrap();

        let errors: Vec<VaultError> = vec![
            VaultError::NamespaceDenied {
                requested: ns_a.clone(),
                granted: ns_b,
            },
            VaultError::InvalidNamespace {
                reason: "x".to_string(),
            },
            VaultError::InvalidKey {
                reason: "x".to_string(),
            },
            VaultError::ValueTooLarge { bytes: 10, max: 5 },
            VaultError::Unsupported {
                operation: "search",
            },
            VaultError::Storage {
                message: "x".to_string(),
            },
            VaultError::Serialization {
                message: "x".to_string(),
            },
        ];

        for err in &errors {
            // Display must never panic.
            let _ = err.to_string();
        }

        // Matching requires a `_` arm because the enum is #[non_exhaustive]
        // from outside the crate; within-crate matches are still exhaustive
        // over the seven current variants but this loop demonstrates named
        // fields (not a bare positional String) except the two documented
        // adapter-boundary variants.
        for err in &errors {
            match err {
                VaultError::NamespaceDenied { requested, granted } => {
                    let _ = (requested, granted);
                }
                VaultError::InvalidNamespace { reason } => {
                    let _ = reason;
                }
                VaultError::InvalidKey { reason } => {
                    let _ = reason;
                }
                VaultError::ValueTooLarge { bytes, max } => {
                    let _ = (bytes, max);
                }
                VaultError::Unsupported { operation } => {
                    let _ = operation;
                }
                VaultError::Storage { message } => {
                    let _ = message;
                }
                VaultError::Serialization { message } => {
                    let _ = message;
                }
            }
        }
    }

    // --- Test 7: key_and_value_bounds_are_typed ---

    #[test]
    fn key_and_value_bounds_are_typed() {
        let ns = Namespace::parse("user/alice").unwrap();

        let err = VaultRecord::new(ns.clone(), "", json!(1)).unwrap_err();
        assert!(matches!(err, VaultError::InvalidKey { .. }));

        let too_long_key = "k".repeat(257);
        let err = VaultRecord::new(ns.clone(), too_long_key, json!(1)).unwrap_err();
        assert!(matches!(err, VaultError::InvalidKey { .. }));

        // A value serializing to more than DEFAULT_MAX_VALUE_BYTES fails
        // with ValueTooLarge carrying both numbers.
        let huge_value = json!("x".repeat(DEFAULT_MAX_VALUE_BYTES + 1));
        let err = VaultRecord::new(ns, "k", huge_value).unwrap_err();
        match err {
            VaultError::ValueTooLarge { bytes, max } => {
                assert!(bytes > max);
                assert_eq!(max, DEFAULT_MAX_VALUE_BYTES);
            }
            other => panic!("expected ValueTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn overwrite_value_preserves_created_at_and_bumps_updated_at() {
        let ns = Namespace::parse("user/alice").unwrap();
        let mut record = VaultRecord::new(ns, "k", json!(1)).unwrap();
        let created_at = record.created_at();

        record.overwrite_value(json!(2)).unwrap();

        assert_eq!(record.created_at(), created_at);
        assert!(record.updated_at() >= created_at);
        assert_eq!(record.value(), &json!(2));
    }
}
