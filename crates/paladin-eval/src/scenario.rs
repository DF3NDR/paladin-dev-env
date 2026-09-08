//! The `.eval.yaml` scenario file format (OBS-04, D-28).
//!
//! A scenario file is hand-written, structured input describing one deterministic
//! evaluation run: which graph to compile, what the mocked LLM says, and what to assert
//! once the run finishes. [`Scenario::from_path`] parses either YAML (`.yaml`/`.yml`) or
//! JSON, validating [`EVAL_SCHEMA_VERSION`] before returning so an unrecognised newer
//! format is a typed error, never a silent misparse (X-04).
//!
//! # Safety (T-28-05-01)
//!
//! The format carries only structured, deserialized assertion and script parameters --
//! no shell command, no path that is executed, no dynamically-loaded code. A
//! `custom(fn)` assertion (the Rust-API-only escape hatch a later plan's assertion
//! library adds) is never deserialized from a file; this module has no variant for it.
//!
//! # A note on hand-rolled (de)serialization for three types (Rule 1 deviation)
//!
//! [`Times`], [`ScriptEntry`] and [`Assertion`] implement [`Serialize`]/[`Deserialize`]
//! by hand instead of `#[derive(Serialize, Deserialize)]`. `serde_yaml` 0.9 (this
//! workspace's already-pinned, deprecated-upstream version -- the same limitation
//! `src/application/cli/config/paladin_config.rs:847` documents for a narrower case)
//! cannot deserialize ANY externally-tagged enum variant that carries data through its
//! derived `Deserializer::deserialize_enum` path: a data-carrying variant's
//! conventional single-key-map wire form (`exact: 3`, `text: hello`,
//! `final_state_field_equals: {field: ..., value: ...}`) fails with `"invalid type:
//! map, expected a YAML tag starting with '!'"` for EVERY variant shape (newtype of a
//! primitive, newtype of a string, struct variant) -- confirmed with an isolated
//! reproduction against this exact dependency version before writing this workaround.
//! Unit-only enums ([`StoreKind`], [`LlmErrorKind`], [`RunStatusValue`]) are unaffected
//! (a bare-string variant never reaches the broken code path) and keep the ordinary
//! derive. The workaround: implement `Deserialize` by hand using
//! `Deserializer::deserialize_any` + a `Visitor` that reads the single-key map (or, for
//! [`Assertion::FinalStateSnapshot`], a bare string) via `MapAccess` directly --
//! `deserialize_any` never invokes `serde_yaml`'s broken `deserialize_enum`, so this
//! reads correctly under BOTH `serde_yaml` and `serde_json`. `Serialize` is
//! hand-written alongside it (via `Serializer::serialize_map`) so writing one of these
//! types back out produces the SAME conventional map form under both formats, rather
//! than `serde_yaml`'s own derived `Serialize` YAML-tag output (`!exact 3`) that its own
//! derived `Deserialize` would have required -- internally consistent with itself, but
//! not what D-28's hand-authored file format or the `schemars`-derived JSON Schema
//! describe. The `#[serde(...)]` container/variant attributes stay on each type
//! purely for `schemars::JsonSchema` to read (it parses them independently of whether
//! `Serialize`/`Deserialize` are actually derived) so the golden schema continues to
//! describe exactly the shape these manual impls produce and consume.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use paladin_ports::output::llm_port::LlmError;

/// The scenario schema version this build of `paladin-eval` understands.
///
/// A scenario document whose own `schema_version` does not equal this string fails to
/// parse with [`ScenarioError::UnsupportedSchemaVersion`] naming the version found,
/// rather than being silently misinterpreted under the current shape (X-04).
pub const EVAL_SCHEMA_VERSION: &str = "1";

/// A parsed, validated `.eval.yaml` (or `.eval.json`) scenario document (D-28).
///
/// # Example
///
/// ```rust
/// use paladin_eval::scenario::{Scenario, StoreKind};
///
/// let yaml = r#"
/// schema_version: "1"
/// target:
///   graph_doc: "fixtures/linear.json"
/// cases:
///   - name: happy_path
///     assertions:
///       - run_status: completed
/// "#;
///
/// let scenario: Scenario = serde_yaml::from_str(yaml).unwrap();
/// assert_eq!(scenario.store, StoreKind::InMemory);
/// assert_eq!(scenario.cases.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// The scenario schema version this document was authored against. Checked against
    /// [`EVAL_SCHEMA_VERSION`] by [`Scenario::from_path`] and [`Scenario::validate_schema_version`].
    pub schema_version: String,
    /// What this scenario runs: a `WarGraphDoc` file or a Rust-registered constructor
    /// (D-31).
    pub target: ScenarioTarget,
    /// Which `WaypointPort`/state store backs the run. Defaults to
    /// [`StoreKind::InMemory`] when absent.
    #[serde(default)]
    pub store: StoreKind,
    /// The scenario-wide scripted LLM behaviour (global sequence, per-node sequences,
    /// prompt-match rules). Defaults to an empty script.
    #[serde(default)]
    pub llm: LlmScript,
    /// One or more cases run against [`Scenario::target`], each with its own
    /// assertions.
    pub cases: Vec<Case>,
    /// Live-mode options (D-35): whether content-bearing assertions are allowed to run
    /// against a real provider instead of being skipped.
    #[serde(default)]
    pub live: LiveOptions,
    /// The name of a host-registered `EngineRegistries` constructor
    /// (`ScenarioRunner::register_registries`) this scenario's [`ScenarioTarget::GraphDoc`]
    /// target compiles through, if it needs one beyond the harness default.
    #[serde(default)]
    pub registries: Option<String>,
}

impl Scenario {
    /// Parse a [`Scenario`] from a file, dispatching on its extension: `.yaml`/`.yml`
    /// parse as YAML, every other extension (including `.json`) parses as JSON.
    ///
    /// Validates [`Scenario::schema_version`] against [`EVAL_SCHEMA_VERSION`] before
    /// returning -- a document from a newer, incompatible format version is a typed
    /// [`ScenarioError::UnsupportedSchemaVersion`], never a value silently misread under
    /// today's shape (X-04).
    ///
    /// # Errors
    ///
    /// - [`ScenarioError::Io`] if the file cannot be read.
    /// - [`ScenarioError::Parse`] if the contents do not deserialize.
    /// - [`ScenarioError::UnsupportedSchemaVersion`] if `schema_version` is not
    ///   [`EVAL_SCHEMA_VERSION`].
    pub fn from_path(path: &Path) -> Result<Self, ScenarioError> {
        let contents = std::fs::read_to_string(path).map_err(|source| ScenarioError::Io {
            path: path.to_path_buf(),
            source,
        })?;

        let is_yaml = matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("yaml") | Some("yml")
        );

        let scenario: Scenario = if is_yaml {
            serde_yaml::from_str(&contents).map_err(|source| ScenarioError::Parse {
                path: path.to_path_buf(),
                source: ParseSource::Yaml(source),
            })?
        } else {
            serde_json::from_str(&contents).map_err(|source| ScenarioError::Parse {
                path: path.to_path_buf(),
                source: ParseSource::Json(source),
            })?
        };

        scenario.validate_schema_version()?;
        Ok(scenario)
    }

    /// Confirm [`Scenario::schema_version`] equals [`EVAL_SCHEMA_VERSION`], or return a
    /// typed [`ScenarioError::UnsupportedSchemaVersion`] naming the version found.
    ///
    /// Called by [`Scenario::from_path`]; exposed separately so a caller holding an
    /// already-deserialized [`Scenario`] (e.g. from an embedded fixture) can run the
    /// same check without a round trip through the filesystem.
    ///
    /// # Errors
    ///
    /// [`ScenarioError::UnsupportedSchemaVersion`] if [`Scenario::schema_version`] is
    /// not [`EVAL_SCHEMA_VERSION`].
    pub fn validate_schema_version(&self) -> Result<(), ScenarioError> {
        if self.schema_version != EVAL_SCHEMA_VERSION {
            return Err(ScenarioError::UnsupportedSchemaVersion {
                found: self.schema_version.clone(),
            });
        }
        Ok(())
    }
}

/// What a [`Scenario`] runs (D-31): a `WarGraphDoc` file on disk, or a Rust closure a
/// host test binary registered by name.
///
/// Untagged so both documented spellings parse directly with no wrapping "type" key --
/// `{ graph_doc: <path> }` or `{ registered: <name> }` -- unambiguous because the two
/// variants' field names never overlap.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(untagged)]
pub enum ScenarioTarget {
    /// Compile the named `WarGraphDoc` file (JSON or YAML) through the scenario's
    /// (optional) named registries.
    GraphDoc {
        /// Path to the `WarGraphDoc` file, resolved relative to the scenario file's own
        /// directory by the runner.
        graph_doc: PathBuf,
    },
    /// Build the graph from a Rust closure a host test binary registered under this
    /// name (`ScenarioRunner::register_graph`).
    Registered {
        /// The name a host test binary passed to `ScenarioRunner::register_graph`.
        registered: String,
    },
}

/// Which state store backs a scenario run (D-28). Defaults to [`StoreKind::InMemory`].
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StoreKind {
    /// An in-memory `WaypointPort`, discarded when the run ends. The default.
    #[default]
    InMemory,
    /// A temp-file SQLite `WaypointPort`, needed by any case that simulates a crash
    /// (`interrupt_after_superstep`) and resumes over the same store.
    SqliteTemp,
}

/// The scenario-wide (or per-case) scripted LLM behaviour (D-28, D-30).
///
/// Resolution order, applied per call by `ScenarioLlm`: [`LlmScript::match_rules`]
/// first (first match wins, does not advance a sequence), then
/// [`LlmScript::per_node`] for the node the call is attributed to, then
/// [`LlmScript::global`].
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LlmScript {
    /// The default sequence consulted when no per-node sequence exists for a call's
    /// node, consumed one entry per call (cycling is `ScenarioLlm`'s decision, not the
    /// format's).
    #[serde(default)]
    pub global: Vec<ScriptEntry>,
    /// Per-node sequences, keyed by the Paladin node id. A call routed to a node with
    /// an entry here consumes THIS sequence and leaves [`LlmScript::global`] untouched.
    #[serde(default)]
    pub per_node: BTreeMap<String, Vec<ScriptEntry>>,
    /// Prompt-substring match rules, checked in declaration order before either
    /// sequence; the wire key is `match` (a Rust keyword, hence the field rename).
    #[serde(default, rename = "match")]
    pub match_rules: Vec<MatchRule>,
}

/// One scripted response to an incoming LLM call (D-28, D-30).
///
/// Externally tagged so the three documented spellings are `{ text: "..." }`,
/// `{ tool_call: { name, arguments } }` and `{ error: <kind> }` -- exactly D-28's `{
/// text | tool_call: {name, arguments} | error: <LlmError kind> }`.
///
/// `Serialize`/`Deserialize` are hand-written -- see this module's top-level "A note on
/// hand-rolled (de)serialization" doc section for why.
#[derive(Debug, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScriptEntry {
    /// `text`: a plain success completion.
    Text(String),
    /// `tool_call`: a success completion carrying a [`FunctionCall`](paladin_ports::output::llm_port::FunctionCall)-shaped
    /// tool invocation.
    ToolCall {
        /// The tool/function name the mocked model "calls".
        name: String,
        /// The JSON-encoded call arguments.
        arguments: serde_json::Value,
    },
    /// `error`: the call returns this [`LlmErrorKind`] instead of a completion.
    Error(LlmErrorKind),
}

/// The wire shape of [`ScriptEntry::ToolCall`]'s content, as its own plain struct so it
/// can be handed to `MapAccess::next_value` (deserializing) and
/// `SerializeMap::serialize_entry` (serializing) directly. Plain structs are unaffected
/// by the `serde_yaml` enum-deserialization bug this module's manual impls work around.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ToolCallFields {
    name: String,
    arguments: serde_json::Value,
}

const SCRIPT_ENTRY_VARIANTS: &[&str] = &["text", "tool_call", "error"];

impl Serialize for ScriptEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            ScriptEntry::Text(text) => map.serialize_entry("text", text)?,
            ScriptEntry::ToolCall { name, arguments } => map.serialize_entry(
                "tool_call",
                &ToolCallFields {
                    name: name.clone(),
                    arguments: arguments.clone(),
                },
            )?,
            ScriptEntry::Error(kind) => map.serialize_entry("error", kind)?,
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for ScriptEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct ScriptEntryVisitor;

        impl<'de> Visitor<'de> for ScriptEntryVisitor {
            type Value = ScriptEntry;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "a single-key map: `text`, `tool_call` or `error`")
            }

            fn visit_map<A>(self, mut map: A) -> Result<ScriptEntry, A::Error>
            where
                A: MapAccess<'de>,
            {
                let key: String = map.next_key()?.ok_or_else(|| {
                    de::Error::custom("expected exactly one key (`text`, `tool_call` or `error`)")
                })?;
                let entry = match key.as_str() {
                    "text" => ScriptEntry::Text(map.next_value()?),
                    "tool_call" => {
                        let fields: ToolCallFields = map.next_value()?;
                        ScriptEntry::ToolCall {
                            name: fields.name,
                            arguments: fields.arguments,
                        }
                    }
                    "error" => ScriptEntry::Error(map.next_value()?),
                    other => {
                        return Err(de::Error::unknown_variant(other, SCRIPT_ENTRY_VARIANTS));
                    }
                };
                Ok(entry)
            }
        }

        deserializer.deserialize_any(ScriptEntryVisitor)
    }
}

/// A closed vocabulary of [`paladin_ports::output::llm_port::LlmError`] shapes a
/// scenario file can script (D-28, D-30).
///
/// [`LlmError`] itself does not implement [`Deserialize`], so this mirrors its
/// vocabulary for the file format the same way `WarGraphDoc`'s `NodeKindDoc` mirrors
/// `NodeSpec` -- [`LlmErrorKind::to_llm_error`] is the one place a scripted kind becomes
/// a real [`LlmError`].
///
/// `#[non_exhaustive]`: a future `LlmError` variant may need its own scriptable kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum LlmErrorKind {
    /// Maps to [`LlmError::NetworkError`].
    Network,
    /// Maps to [`LlmError::AuthenticationError`].
    Authentication,
    /// Maps to [`LlmError::InvalidPrompt`].
    InvalidPrompt,
    /// Maps to [`LlmError::RateLimitExceeded`].
    RateLimit,
    /// Maps to [`LlmError::UsageLimitExceeded`].
    UsageLimit,
    /// Maps to [`LlmError::ModelNotAvailable`].
    ModelNotAvailable,
    /// Maps to [`LlmError::TokenLimitExceeded`].
    TokenLimit,
    /// Maps to [`LlmError::EmptyCompletion`].
    EmptyCompletion,
    /// Maps to [`LlmError::ProcessingError`].
    Processing,
    /// Maps to [`LlmError::Timeout`].
    Timeout,
    /// A generic transient failure (maps to [`LlmError::NetworkError`]) for a scenario
    /// that only cares about the retry/fallback behaviour a transient error drives, not
    /// which specific variant produced it.
    Transient,
}

impl LlmErrorKind {
    /// Build the concrete [`LlmError`] this scripted kind represents.
    ///
    /// `node` and `call_index` are folded into the error's own message so a failure
    /// rendered from a captured request log can point at exactly which scripted call
    /// produced it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use paladin_eval::scenario::LlmErrorKind;
    ///
    /// let error = LlmErrorKind::RateLimit.to_llm_error("planner", 2);
    /// assert!(error.to_string().contains("Rate limit"));
    /// ```
    pub fn to_llm_error(
        &self,
        node: &str,
        call_index: usize,
    ) -> paladin_ports::output::llm_port::LlmError {
        match self {
            LlmErrorKind::Network => LlmError::NetworkError(format!(
                "scripted network error (node {node}, call {call_index})"
            )),
            LlmErrorKind::Authentication => LlmError::AuthenticationError(format!(
                "scripted authentication error (node {node}, call {call_index})"
            )),
            LlmErrorKind::InvalidPrompt => LlmError::InvalidPrompt(format!(
                "scripted invalid prompt (node {node}, call {call_index})"
            )),
            LlmErrorKind::RateLimit => LlmError::RateLimitExceeded,
            LlmErrorKind::UsageLimit => LlmError::UsageLimitExceeded {
                provider: "scenario".to_string(),
                regain_hint: None,
            },
            LlmErrorKind::ModelNotAvailable => LlmError::ModelNotAvailable(format!(
                "scripted model-not-available (node {node}, call {call_index})"
            )),
            LlmErrorKind::TokenLimit => LlmError::TokenLimitExceeded,
            LlmErrorKind::EmptyCompletion => LlmError::EmptyCompletion(format!(
                "scripted empty completion (node {node}, call {call_index})"
            )),
            LlmErrorKind::Processing => LlmError::ProcessingError(format!(
                "scripted processing error (node {node}, call {call_index})"
            )),
            LlmErrorKind::Timeout => {
                LlmError::Timeout(format!("scripted timeout (node {node}, call {call_index})"))
            }
            LlmErrorKind::Transient => LlmError::NetworkError(format!(
                "scripted transient error (node {node}, call {call_index})"
            )),
        }
    }
}

/// A prompt-substring match rule (D-28), checked before either script sequence.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct MatchRule {
    /// If the incoming prompt contains this substring, [`MatchRule::response`] is
    /// returned and neither sequence is advanced.
    pub prompt_contains: String,
    /// The response returned when [`MatchRule::prompt_contains`] matches.
    pub response: ScriptEntry,
}

/// One test case within a [`Scenario`] (D-28, D-34).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Case {
    /// This case's name, used in `libtest-mimic` trial naming and failure output.
    pub name: String,
    /// Initial input fields, by name, handed to the run.
    #[serde(default)]
    pub input: BTreeMap<String, serde_json::Value>,
    /// Simulate a crash after this superstep completes: the runner drops the engine
    /// and builds a fresh one over the same [`StoreKind::SqliteTemp`] store to resume
    /// (D-34's E2E-1 technique). Requires `store: sqlite_temp`.
    #[serde(default)]
    pub interrupt_after_superstep: Option<u64>,
    /// Scripted Parley responses, consumed in order whenever the run raises a Parley
    /// (D-34's E2E-2 technique).
    #[serde(default)]
    pub parley_responses: Vec<ParleyScript>,
    /// A case-specific LLM script overriding [`Scenario::llm`] for this case only.
    #[serde(default)]
    pub llm: Option<LlmScript>,
    /// The assertions evaluated against the captured trace records and the final
    /// `Battlefield` once this case's run finishes.
    pub assertions: Vec<Assertion>,
}

/// One scripted response to a Parley the run raises (D-28, D-34).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ParleyScript {
    /// The Parley kind this response answers (mirrors
    /// `paladin_core::platform::container::parley::ParleyKind`'s wire vocabulary).
    pub kind: String,
    /// The value handed back as the Parley's resolution.
    pub value: serde_json::Value,
}

/// Live-mode options (D-35): whether a scenario opts in to running content-bearing
/// assertions against a real provider instead of skipping them.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LiveOptions {
    /// When `true` and the runner is invoked with `--live`, content-bearing assertions
    /// (`final_state_field_equals`, `final_state_field_matches`, `field_json_path_equals`,
    /// `final_state_snapshot`) run instead of being skipped. Defaults to `false`.
    #[serde(default)]
    pub allow_content_assertions: bool,
}

/// A bound on how many times [`Assertion::NodeExecuted`] expects its node to have run.
///
/// `Serialize`/`Deserialize` are hand-written -- see this module's top-level "A note on
/// hand-rolled (de)serialization" doc section for why.
#[derive(Debug, Clone, Copy, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Times {
    /// The node ran exactly this many times.
    Exact(u32),
    /// The node ran at least this many times.
    Min(u32),
    /// The node ran at most this many times.
    Max(u32),
}

const TIMES_VARIANTS: &[&str] = &["exact", "min", "max"];

impl Serialize for Times {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Times::Exact(v) => map.serialize_entry("exact", v)?,
            Times::Min(v) => map.serialize_entry("min", v)?,
            Times::Max(v) => map.serialize_entry("max", v)?,
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Times {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct TimesVisitor;

        impl<'de> Visitor<'de> for TimesVisitor {
            type Value = Times;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "a single-key map: `exact`, `min` or `max`")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Times, A::Error>
            where
                A: MapAccess<'de>,
            {
                let key: String = map.next_key()?.ok_or_else(|| {
                    de::Error::custom("expected exactly one key (`exact`, `min` or `max`)")
                })?;
                let times = match key.as_str() {
                    "exact" => Times::Exact(map.next_value()?),
                    "min" => Times::Min(map.next_value()?),
                    "max" => Times::Max(map.next_value()?),
                    other => return Err(de::Error::unknown_variant(other, TIMES_VARIANTS)),
                };
                Ok(times)
            }
        }

        deserializer.deserialize_any(TimesVisitor)
    }
}

/// The vocabulary [`Assertion::RunStatus`] compares against.
///
/// Mirrors `paladin_core::platform::container::trace::RunFinishStatus` field-for-field.
/// That core type carries no `schemars::JsonSchema` derive (ADR-0015: `paladin-core`
/// does not take the `schemars` dependency), so this crate mirrors the vocabulary
/// locally -- exactly the pattern `WarGraphDoc`'s `NodeKindDoc` uses for `NodeSpec`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatusValue {
    /// The run completed normally.
    Completed,
    /// The run failed.
    Failed,
    /// The run was halted.
    Halted,
    /// The run suspended awaiting Parley input.
    AwaitingInput,
}

/// One assertion evaluated against a case's captured trace records and final
/// `Battlefield` (D-28, D-29, PRD 07 OBS-FR-12).
///
/// A closed, externally tagged enum -- an unrecognised tag is a deserialize error
/// naming the tag, never a silently-ignored no-op (X-04's "closed, not silently
/// permissive" rule). `custom(fn)` (a later plan's Rust-API-only escape hatch) has no
/// variant here and can never be deserialized from a file (T-28-05-01).
///
/// Every variant name below is PRD 07 OBS-FR-12's verbatim, snake-cased wire name.
///
/// `Serialize`/`Deserialize` are hand-written -- see this module's top-level "A note on
/// hand-rolled (de)serialization" doc section for why.
#[derive(Debug, Clone, JsonSchema, PartialEq)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Assertion {
    /// `final_state_field_equals`: the named final-`Battlefield` field equals `value`
    /// exactly.
    FinalStateFieldEquals {
        /// The `Battlefield` field name.
        field: String,
        /// The expected value.
        value: serde_json::Value,
    },
    /// `final_state_field_matches`: the named final-`Battlefield` field, rendered as a
    /// string, matches the `pattern` regular expression.
    FinalStateFieldMatches {
        /// The `Battlefield` field name.
        field: String,
        /// The regular expression the field's rendered value must match.
        pattern: String,
    },
    /// `field_json_path_equals`: the value at `path` (a JSON Pointer / JSONPath-style
    /// path into the final `Battlefield`) equals `value`.
    FieldJsonPathEquals {
        /// The path into the final `Battlefield`'s JSON representation.
        path: String,
        /// The expected value at `path`.
        value: serde_json::Value,
    },
    /// `node_executed`: the named node ran the number of times `times` bounds.
    NodeExecuted {
        /// The node id.
        node: String,
        /// The execution-count bound.
        times: Times,
    },
    /// `node_not_executed`: the named node never ran.
    NodeNotExecuted {
        /// The node id.
        node: String,
    },
    /// `edge_fired`: the edge `from -> to` fired at least once (needs `EdgeEvaluated`
    /// records, D-04).
    EdgeFired {
        /// The source node id.
        from: String,
        /// The destination node id.
        to: String,
    },
    /// `route_taken`: the sequence of node ids in `NodeStarted` order contains this
    /// list as a subsequence.
    RouteTaken(Vec<String>),
    /// `run_status`: the run's terminal status equals this value.
    RunStatus(RunStatusValue),
    /// `total_tokens_max`: the run's total token usage does not exceed this bound.
    TotalTokensMax(u64),
    /// `supersteps_max`: the run's total superstep count does not exceed this bound.
    SuperstepsMax(u64),
    /// `parley_raised`: a Parley of `kind` was raised by `node`.
    ParleyRaised {
        /// The Parley kind (mirrors `ParleyKind`'s wire vocabulary).
        kind: String,
        /// The node that raised it.
        node: String,
    },
    /// `final_state_snapshot`: the final `Battlefield` matches a blessed snapshot file
    /// (`--bless` regenerates it, D-33).
    FinalStateSnapshot,
}

// ---------------------------------------------------------------------------
// Assertion: hand-rolled Serialize/Deserialize (see this module's top-level doc note).
// ---------------------------------------------------------------------------

/// Wire shape of [`Assertion::FinalStateFieldEquals`]'s content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldEqualsFields {
    field: String,
    value: serde_json::Value,
}

/// Wire shape of [`Assertion::FinalStateFieldMatches`]'s content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldMatchesFields {
    field: String,
    pattern: String,
}

/// Wire shape of [`Assertion::FieldJsonPathEquals`]'s content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct JsonPathFields {
    path: String,
    value: serde_json::Value,
}

/// Wire shape of [`Assertion::NodeExecuted`]'s content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeExecutedFields {
    node: String,
    times: Times,
}

/// Wire shape of [`Assertion::NodeNotExecuted`]'s content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct NodeOnlyFields {
    node: String,
}

/// Wire shape of [`Assertion::EdgeFired`]'s content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EdgeFields {
    from: String,
    to: String,
}

/// Wire shape of [`Assertion::ParleyRaised`]'s content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ParleyRaisedFields {
    kind: String,
    node: String,
}

const ASSERTION_VARIANTS: &[&str] = &[
    "final_state_field_equals",
    "final_state_field_matches",
    "field_json_path_equals",
    "node_executed",
    "node_not_executed",
    "edge_fired",
    "route_taken",
    "run_status",
    "total_tokens_max",
    "supersteps_max",
    "parley_raised",
    "final_state_snapshot",
];

impl Serialize for Assertion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // `final_state_snapshot` is the one unit variant -- a bare string, not a
        // single-key map, matching the default external-tagging representation a
        // derived impl would also have produced for a unit variant.
        if let Assertion::FinalStateSnapshot = self {
            return serializer.serialize_str("final_state_snapshot");
        }

        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Assertion::FinalStateFieldEquals { field, value } => map.serialize_entry(
                "final_state_field_equals",
                &FieldEqualsFields {
                    field: field.clone(),
                    value: value.clone(),
                },
            )?,
            Assertion::FinalStateFieldMatches { field, pattern } => map.serialize_entry(
                "final_state_field_matches",
                &FieldMatchesFields {
                    field: field.clone(),
                    pattern: pattern.clone(),
                },
            )?,
            Assertion::FieldJsonPathEquals { path, value } => map.serialize_entry(
                "field_json_path_equals",
                &JsonPathFields {
                    path: path.clone(),
                    value: value.clone(),
                },
            )?,
            Assertion::NodeExecuted { node, times } => map.serialize_entry(
                "node_executed",
                &NodeExecutedFields {
                    node: node.clone(),
                    times: *times,
                },
            )?,
            Assertion::NodeNotExecuted { node } => {
                map.serialize_entry("node_not_executed", &NodeOnlyFields { node: node.clone() })?
            }
            Assertion::EdgeFired { from, to } => map.serialize_entry(
                "edge_fired",
                &EdgeFields {
                    from: from.clone(),
                    to: to.clone(),
                },
            )?,
            Assertion::RouteTaken(nodes) => map.serialize_entry("route_taken", nodes)?,
            Assertion::RunStatus(status) => map.serialize_entry("run_status", status)?,
            Assertion::TotalTokensMax(v) => map.serialize_entry("total_tokens_max", v)?,
            Assertion::SuperstepsMax(v) => map.serialize_entry("supersteps_max", v)?,
            Assertion::ParleyRaised { kind, node } => map.serialize_entry(
                "parley_raised",
                &ParleyRaisedFields {
                    kind: kind.clone(),
                    node: node.clone(),
                },
            )?,
            Assertion::FinalStateSnapshot => unreachable!("handled by the early return above"),
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for Assertion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AssertionVisitor;

        impl<'de> Visitor<'de> for AssertionVisitor {
            type Value = Assertion;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(
                    f,
                    "`final_state_snapshot` or a single-key map naming one of the other eleven assertion kinds"
                )
            }

            fn visit_str<E>(self, v: &str) -> Result<Assertion, E>
            where
                E: de::Error,
            {
                match v {
                    "final_state_snapshot" => Ok(Assertion::FinalStateSnapshot),
                    other => Err(de::Error::unknown_variant(other, ASSERTION_VARIANTS)),
                }
            }

            fn visit_map<A>(self, mut map: A) -> Result<Assertion, A::Error>
            where
                A: MapAccess<'de>,
            {
                let key: String = map.next_key()?.ok_or_else(|| {
                    de::Error::custom("expected exactly one key naming the assertion kind")
                })?;
                let assertion = match key.as_str() {
                    "final_state_field_equals" => {
                        let fields: FieldEqualsFields = map.next_value()?;
                        Assertion::FinalStateFieldEquals {
                            field: fields.field,
                            value: fields.value,
                        }
                    }
                    "final_state_field_matches" => {
                        let fields: FieldMatchesFields = map.next_value()?;
                        Assertion::FinalStateFieldMatches {
                            field: fields.field,
                            pattern: fields.pattern,
                        }
                    }
                    "field_json_path_equals" => {
                        let fields: JsonPathFields = map.next_value()?;
                        Assertion::FieldJsonPathEquals {
                            path: fields.path,
                            value: fields.value,
                        }
                    }
                    "node_executed" => {
                        let fields: NodeExecutedFields = map.next_value()?;
                        Assertion::NodeExecuted {
                            node: fields.node,
                            times: fields.times,
                        }
                    }
                    "node_not_executed" => {
                        let fields: NodeOnlyFields = map.next_value()?;
                        Assertion::NodeNotExecuted { node: fields.node }
                    }
                    "edge_fired" => {
                        let fields: EdgeFields = map.next_value()?;
                        Assertion::EdgeFired {
                            from: fields.from,
                            to: fields.to,
                        }
                    }
                    "route_taken" => Assertion::RouteTaken(map.next_value()?),
                    "run_status" => Assertion::RunStatus(map.next_value()?),
                    "total_tokens_max" => Assertion::TotalTokensMax(map.next_value()?),
                    "supersteps_max" => Assertion::SuperstepsMax(map.next_value()?),
                    "parley_raised" => {
                        let fields: ParleyRaisedFields = map.next_value()?;
                        Assertion::ParleyRaised {
                            kind: fields.kind,
                            node: fields.node,
                        }
                    }
                    "final_state_snapshot" => {
                        // Tolerate `{ final_state_snapshot: null }` / `{
                        // final_state_snapshot: {} }` alongside the bare-string form
                        // `visit_str` handles -- either spelling of the unit variant
                        // parses.
                        let _: serde::de::IgnoredAny = map.next_value()?;
                        Assertion::FinalStateSnapshot
                    }
                    other => return Err(de::Error::unknown_variant(other, ASSERTION_VARIANTS)),
                };
                Ok(assertion)
            }
        }

        deserializer.deserialize_any(AssertionVisitor)
    }
}

/// The underlying parse failure wrapped by [`ScenarioError::Parse`] -- either a YAML or
/// a JSON deserialize error, depending on which format [`Scenario::from_path`]
/// dispatched to.
#[derive(Debug, Error)]
pub enum ParseSource {
    /// A YAML deserialize failure (`.yaml`/`.yml` files).
    #[error(transparent)]
    Yaml(#[from] serde_yaml::Error),
    /// A JSON deserialize failure (every other extension).
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Errors [`Scenario::from_path`] and [`Scenario::validate_schema_version`] can return.
///
/// `#[non_exhaustive]`: a future variant (e.g. a semantic validation failure beyond
/// `schema_version`) can be added without a semver-major bump.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ScenarioError {
    /// The document's `schema_version` is not [`EVAL_SCHEMA_VERSION`].
    #[error(
        "unsupported scenario schema_version {found:?}; this build of paladin-eval understands {EVAL_SCHEMA_VERSION:?}"
    )]
    UnsupportedSchemaVersion {
        /// The `schema_version` string found in the document.
        found: String,
    },
    /// The document's contents did not deserialize.
    #[error("failed to parse scenario file {path}: {source}", path = path.display())]
    Parse {
        /// The file that failed to parse.
        path: PathBuf,
        /// The underlying YAML or JSON deserialize error.
        #[source]
        source: ParseSource,
    },
    /// The scenario file could not be read from disk.
    #[error("failed to read scenario file {path}: {source}", path = path.display())]
    Io {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_yaml() -> &'static str {
        r#"
schema_version: "1"
target:
  graph_doc: "fixtures/linear.json"
cases:
  - name: happy_path
    input:
      foo: "bar"
    assertions:
      - final_state_field_equals:
          field: "status"
          value: "done"
"#
    }

    #[test]
    fn minimal_scenario_parses() {
        let scenario: Scenario = serde_yaml::from_str(minimal_yaml()).expect("parses");
        assert_eq!(scenario.schema_version, "1");
        assert_eq!(
            scenario.target,
            ScenarioTarget::GraphDoc {
                graph_doc: PathBuf::from("fixtures/linear.json"),
            }
        );
        assert_eq!(scenario.store, StoreKind::InMemory);
        assert_eq!(scenario.llm, LlmScript::default());
        assert_eq!(scenario.cases.len(), 1);
        assert_eq!(scenario.cases[0].name, "happy_path");
        assert_eq!(scenario.cases[0].assertions.len(), 1);
    }

    #[test]
    fn json_and_yaml_agree() {
        let from_yaml: Scenario = serde_yaml::from_str(minimal_yaml()).expect("parses yaml");

        let json = serde_json::json!({
            "schema_version": "1",
            "target": { "graph_doc": "fixtures/linear.json" },
            "cases": [
                {
                    "name": "happy_path",
                    "input": { "foo": "bar" },
                    "assertions": [
                        { "final_state_field_equals": { "field": "status", "value": "done" } }
                    ]
                }
            ]
        });
        let from_json: Scenario =
            serde_json::from_value(json).expect("parses the equivalent json document");

        assert_eq!(from_yaml, from_json);
    }

    #[test]
    fn registered_target_parses() {
        let yaml = r#"
schema_version: "1"
target:
  registered: "e2e1"
cases: []
"#;
        let scenario: Scenario = serde_yaml::from_str(yaml).expect("parses");
        assert_eq!(
            scenario.target,
            ScenarioTarget::Registered {
                registered: "e2e1".to_string(),
            }
        );
    }

    #[test]
    fn unknown_schema_version_is_typed() {
        let yaml = r#"
schema_version: "2"
target:
  graph_doc: "fixtures/linear.json"
cases: []
"#;
        let scenario: Scenario = serde_yaml::from_str(yaml).expect("still deserializes");
        let result = scenario.validate_schema_version();
        match result {
            Err(ScenarioError::UnsupportedSchemaVersion { found }) => {
                assert_eq!(found, "2");
            }
            other => panic!("expected UnsupportedSchemaVersion, got {other:?}"),
        }
    }

    #[test]
    fn unknown_assertion_kind_is_an_error() {
        let result: Result<Assertion, _> = serde_yaml::from_str("made_up_kind: {}");
        let err = result.expect_err("an unrecognised assertion tag must fail to deserialize");
        let message = err.to_string();
        assert!(
            message.contains("made_up_kind"),
            "error message {message:?} does not name the unrecognised tag"
        );
    }

    #[test]
    fn times_variants_round_trip() {
        for (yaml, expected) in [
            ("exact: 3", Times::Exact(3)),
            ("min: 1", Times::Min(1)),
            ("max: 5", Times::Max(5)),
        ] {
            let parsed: Times = serde_yaml::from_str(yaml).expect("parses");
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn script_entry_spellings_round_trip() {
        let text: ScriptEntry = serde_yaml::from_str("text: hello").unwrap();
        assert_eq!(text, ScriptEntry::Text("hello".to_string()));

        let tool_call: ScriptEntry =
            serde_yaml::from_str("tool_call:\n  name: search\n  arguments: {}").unwrap();
        assert_eq!(
            tool_call,
            ScriptEntry::ToolCall {
                name: "search".to_string(),
                arguments: serde_json::json!({}),
            }
        );

        let error: ScriptEntry = serde_yaml::from_str("error: transient").unwrap();
        assert_eq!(error, ScriptEntry::Error(LlmErrorKind::Transient));
    }
}
