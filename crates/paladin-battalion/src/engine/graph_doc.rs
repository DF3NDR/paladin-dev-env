//! `WarGraphDoc` -- the serde/schemars document form of a [`crate::engine::graph::WarGraph`]
//! (PLAT-FR-12, D-31, D-33, D-34): the JSON body an admin authors over HTTP
//! (Phase 27's `POST /assistants`), an assistant version stores, and this
//! module's [`WarGraphDoc::compile`] turns into an executable, `validate`d
//! graph the worker can run.
//!
//! # Compile is validation (D-31)
//!
//! [`WarGraphDoc::compile`] is the ONLY way a document becomes a
//! [`crate::engine::graph::WarGraph`]: it resolves every named reference --
//! `EdgeCondition::Custom`, `RetryPredicate::Custom`, `ErrorHandlerSpec::Custom`
//! and `SchemaRef::Registered` -- through the caller's [`EngineRegistries`],
//! then calls [`WarGraph::validate`] on the fully-built graph before
//! returning it. A version that compiles is therefore a version that is
//! guaranteed to run: nothing partially built is ever returned, and every
//! failure is a typed [`CompileError`] variant naming the offending node,
//! edge, or name -- never a silent drop and never a bare string.
//!
//! # The v0.10 node-kind boundary (D-33 scope correction)
//!
//! A document may declare exactly three node kinds: `paladin`, `gate`, and
//! `workflow` (a nested `WarGraphDoc`, compiled recursively into
//! [`NodeSpec::Battalion`]). [`NodeSpec::Function`] -- arbitrary Rust
//! behaviour -- is **not** expressible in a document and never will be
//! resolved by name from one (T-27-05-02): only names already registered in
//! the process's [`EngineRegistries`] resolve, and only these three kinds
//! compile. Any other `kind` string -- including `"function"` -- fails
//! compilation with [`CompileError::UnsupportedNodeKind`], naming the
//! rejected string. Code-registered assistants remain the only path for
//! custom Rust logic.
//!
//! # Example
//!
//! The smallest compilable document: one Gate node, entered directly.
//!
//! ```
//! use paladin_battalion::engine::graph_doc::{
//!     EdgeDoc, GateNodeDoc, LimitsDoc, NodeDoc, OnExpireDoc, ParleyKindDoc, SchemaDoc,
//!     WarGraphDoc, WARGRAPH_DOC_SCHEMA_VERSION,
//! };
//! use paladin_battalion::engine::EngineRegistries;
//!
//! let doc = WarGraphDoc {
//!     schema_version: WARGRAPH_DOC_SCHEMA_VERSION.to_string(),
//!     entry: vec!["review".to_string()],
//!     nodes: vec![NodeDoc {
//!         id: "review".to_string(),
//!         kind: "gate".to_string(),
//!         paladin: None,
//!         gate: Some(GateNodeDoc {
//!             parley: ParleyKindDoc::Approval,
//!             prompt_template: "Approve?".to_string(),
//!             payload_template: None,
//!             choices: None,
//!             expires_in_secs: None,
//!             on_expire: OnExpireDoc::FailRun,
//!             output_field: Some("approved".to_string()),
//!         }),
//!         workflow: None,
//!         aegis: None,
//!         defer: false,
//!     }],
//!     edges: vec![EdgeDoc {
//!         from: "review".to_string(),
//!         to: "review".to_string(),
//!         condition: None,
//!     }],
//!     schema: SchemaDoc { fields: vec![] },
//!     limits: LimitsDoc::default(),
//!     default_aegis: None,
//! };
//!
//! // A real document would populate `schema.fields` for `approved`; this
//! // doc test only proves the smallest gate-only shape parses and the
//! // `compile` entry point is callable.
//! let _ = doc.compile(&EngineRegistries::new());
//! ```

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::aegis::{
    Aegis, CacheKeySpec, CachePolicy, ErrorHandlerSpec, RetryPolicy, RetryPredicate, TimeoutPolicy,
};
use paladin_core::platform::container::battalion::campaign::EdgeCondition;
use paladin_core::platform::container::battlefield::{
    BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_core::platform::container::parley::{OnExpire, ParleyKind};
use paladin_core::platform::container::structured::SchemaRef;
use paladin_core::platform::container::waypoint::NodeId;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::engine::EngineError;
use crate::engine::graph::{EdgeSpec, EngineLimits, GateRequestTemplate, StateMap, WarGraph};
use crate::engine::input_mapping::InputMapping;
use crate::engine::registries::EngineRegistries;
use crate::engine::{DispatchRegistry, NodeSpec};

/// The only `schema_version` [`WarGraphDoc::compile`] accepts (X-04). A
/// document persisted under a future version bumps this constant alongside
/// a reader shim -- there is no such shim yet, so any other value is a
/// typed [`CompileError::UnknownSchemaVersion`], never silently accepted.
pub const WARGRAPH_DOC_SCHEMA_VERSION: &str = "1";

/// The deepest a `workflow` node's nested [`WarGraphDoc`] may recurse before
/// [`CompileError::NestingTooDeep`] (T-27-05-03): bounds a pathological
/// document's compile-time and runtime cost without rejecting the
/// legitimate shallow nesting `NodeSpec::Battalion` composition is for.
const MAX_NESTING_DEPTH: u32 = 8;

/// The serde/schemars document form of a [`WarGraph`] (D-33, D-34): every
/// field here is either a plain, `schemars`-describable scalar or another
/// doc type in this module -- never a `NodeSpec`, `Aegis`, or other
/// `paladin-core`/`paladin-battalion` runtime type directly, so this type
/// alone carries the `JsonSchema` derive `paladin-core` may not take
/// (ADR-0015).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WarGraphDoc {
    /// Must equal [`WARGRAPH_DOC_SCHEMA_VERSION`] (X-04).
    pub schema_version: String,
    /// The graph's entry-point node ids (order-preserving).
    pub entry: Vec<String>,
    /// Every node this graph declares, in document order (compiled
    /// deterministically in this same order).
    pub nodes: Vec<NodeDoc>,
    /// Every static edge this graph declares.
    pub edges: Vec<EdgeDoc>,
    /// The graph's Battlefield schema.
    pub schema: SchemaDoc,
    /// Run bounds; absent fields fall back to [`EngineLimits::default`]'s
    /// values via [`LimitsDoc::default`].
    #[serde(default)]
    pub limits: LimitsDoc,
    /// The graph-wide fallback [`Aegis`], if any.
    #[serde(default)]
    pub default_aegis: Option<AegisDoc>,
}

impl WarGraphDoc {
    /// Resolve every named reference this document makes through
    /// `registries`, build the corresponding [`WarGraph`], and `validate`
    /// it before returning -- compile IS validation (D-31): a
    /// `WarGraphDoc` that compiles is a `WarGraphDoc` that is guaranteed to
    /// run. Every failure is a typed [`CompileError`] naming the offending
    /// node, edge, or name; nothing partially built is ever returned.
    pub fn compile(&self, registries: &EngineRegistries) -> Result<WarGraph, CompileError> {
        self.compile_at_depth(registries, 1)
    }

    /// The recursive worker behind [`WarGraphDoc::compile`] (private: the
    /// public surface is the single `compile` entry point). `depth` counts
    /// this document itself as level 1, incrementing for every `workflow`
    /// node's nested document (T-27-05-03's `MAX_NESTING_DEPTH` bound).
    fn compile_at_depth(
        &self,
        registries: &EngineRegistries,
        depth: u32,
    ) -> Result<WarGraph, CompileError> {
        if depth > MAX_NESTING_DEPTH {
            return Err(CompileError::NestingTooDeep {
                max: MAX_NESTING_DEPTH,
            });
        }
        if self.schema_version != WARGRAPH_DOC_SCHEMA_VERSION {
            return Err(CompileError::UnknownSchemaVersion {
                found: self.schema_version.clone(),
            });
        }

        let mut seen_ids: HashSet<&str> = HashSet::new();
        for node in &self.nodes {
            if !seen_ids.insert(node.id.as_str()) {
                return Err(CompileError::DuplicateNode {
                    id: node.id.clone(),
                });
            }
        }
        let known_ids: HashSet<&str> = self.nodes.iter().map(|n| n.id.as_str()).collect();

        for entry in &self.entry {
            if !known_ids.contains(entry.as_str()) {
                return Err(CompileError::UnknownNode { id: entry.clone() });
            }
        }
        for edge in &self.edges {
            if !known_ids.contains(edge.from.as_str()) {
                return Err(CompileError::UnknownNode {
                    id: edge.from.clone(),
                });
            }
            if !known_ids.contains(edge.to.as_str()) {
                return Err(CompileError::UnknownNode {
                    id: edge.to.clone(),
                });
            }
        }

        let schema = compile_schema(&self.schema)?;
        let limits = EngineLimits::from(&self.limits);
        let mut graph = WarGraph::new(schema, limits);

        for entry in &self.entry {
            graph.add_entry(NodeId::new(entry.clone()));
        }

        if let Some(default_aegis) = &self.default_aegis {
            graph.with_default_aegis(compile_aegis(default_aegis, "<default_aegis>", registries)?);
        }

        for node in &self.nodes {
            let spec = compile_node(node, registries, depth)?;
            if node.defer {
                graph.add_deferred_node(NodeId::new(node.id.clone()), spec);
            } else {
                graph.add_node(NodeId::new(node.id.clone()), spec);
            }
            if let Some(aegis_doc) = &node.aegis {
                let aegis = compile_aegis(aegis_doc, node.id.as_str(), registries)?;
                graph.set_aegis(NodeId::new(node.id.clone()), aegis);
            }
        }

        for edge in &self.edges {
            let condition = compile_edge_condition(edge, registries)?;
            graph.add_edge(EdgeSpec {
                from: NodeId::new(edge.from.clone()),
                to: NodeId::new(edge.to.clone()),
                condition,
            });
        }

        // Only the OUTERMOST call validates: `WarGraph::validate`'s own
        // recursion (`validate_battalion_children`) walks every nested
        // `NodeSpec::Battalion` child's structural rules exactly once,
        // exactly as it does for a Rust-constructed graph -- calling
        // `validate` again here at every nesting level would make a deep
        // `workflow` chain's compile cost exponential in depth for no
        // additional coverage (mirroring `WarGraph::validate`'s own
        // documented reason for avoiding recursive `child.validate()`
        // calls, WR-01, 23-REVIEW.md).
        if depth == 1 {
            graph
                .validate(DispatchRegistry::default().resolver(), registries)
                .map_err(|source| CompileError::Invalid { source })?;
        }

        Ok(graph)
    }
}

/// One node in a [`WarGraphDoc`] (D-33). `kind` names which of `paladin`,
/// `gate`, or `workflow` is populated; deliberately NOT a
/// `#[serde(tag = "kind")]`-dispatched enum sharing the `kind` key, because
/// that combination cannot both (a) accept and round-trip an unrecognised
/// `kind` string for [`CompileError::UnsupportedNodeKind`] to report, and
/// (b) combine with `#[serde(flatten)]` and `#[serde(deny_unknown_fields)]`
/// on the SAME struct (a serde restriction, not a choice) -- so this shape
/// keeps `kind` as its own always-present field and gates on it explicitly
/// in [`NodeDoc::kind_doc`], the one place a `"function"` (or any other
/// unrecognised) kind is turned into a typed, string-preserving error
/// rather than a raw `serde_json` deserialize failure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct NodeDoc {
    /// This node's id, unique within the document.
    pub id: String,
    /// Which of `paladin` / `gate` / `workflow` this node is; any other
    /// value fails [`WarGraphDoc::compile`] with
    /// [`CompileError::UnsupportedNodeKind`].
    pub kind: String,
    /// Populated when `kind == "paladin"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paladin: Option<PaladinNodeDoc>,
    /// Populated when `kind == "gate"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateNodeDoc>,
    /// Populated when `kind == "workflow"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<WorkflowNodeDoc>,
    /// This node's own `Aegis` override, if any.
    #[serde(default)]
    pub aegis: Option<AegisDoc>,
    /// Whether this node is registered deferred (`WarGraph::add_deferred_node`).
    #[serde(default)]
    pub defer: bool,
}

impl NodeDoc {
    /// Resolve this node's `kind` string to its typed body, or a typed
    /// [`CompileError`] naming the unsupported kind or the missing body
    /// (a `kind` naming a variant whose matching field was left absent).
    pub fn kind_doc(&self) -> Result<NodeKindDoc, CompileError> {
        match self.kind.as_str() {
            "paladin" => self
                .paladin
                .clone()
                .map(NodeKindDoc::Paladin)
                .ok_or_else(|| CompileError::MissingNodeBody {
                    node: self.id.clone(),
                    kind: "paladin".to_string(),
                }),
            "gate" => self.gate.clone().map(NodeKindDoc::Gate).ok_or_else(|| {
                CompileError::MissingNodeBody {
                    node: self.id.clone(),
                    kind: "gate".to_string(),
                }
            }),
            "workflow" => self
                .workflow
                .clone()
                .map(NodeKindDoc::Workflow)
                .ok_or_else(|| CompileError::MissingNodeBody {
                    node: self.id.clone(),
                    kind: "workflow".to_string(),
                }),
            other => Err(CompileError::UnsupportedNodeKind {
                kind: other.to_string(),
            }),
        }
    }
}

/// The typed result of [`NodeDoc::kind_doc`] -- the v0.10 node-kind
/// boundary made explicit in Rust's type system (D-33 scope correction):
/// exactly `paladin`, `gate`, `workflow`. Never itself part of the wire
/// format (see [`NodeDoc`]'s rustdoc for why `kind` stays a plain field
/// rather than this enum's own serde tag).
#[derive(Debug, Clone, PartialEq)]
pub enum NodeKindDoc {
    /// A `NodeSpec::Paladin` node.
    Paladin(PaladinNodeDoc),
    /// A `NodeSpec::Gate` node.
    Gate(GateNodeDoc),
    /// A `NodeSpec::Battalion` node wrapping a nested [`WarGraphDoc`].
    Workflow(WorkflowNodeDoc),
}

/// A `kind: "paladin"` node's document form, compiling to
/// [`NodeSpec::Paladin`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct PaladinNodeDoc {
    /// The Paladin's display name.
    pub name: String,
    /// The LLM model identifier.
    pub model: String,
    /// The Paladin's system prompt.
    pub system_prompt: String,
    /// Response randomness; defaults to `0.7` when absent.
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Maximum reasoning loops (`MaxLoops::Fixed`); defaults to `3` when
    /// absent.
    #[serde(default)]
    pub max_loops: Option<u32>,
    /// Tokens that signal the Paladin should stop.
    #[serde(default)]
    pub stop_words: Vec<String>,
    /// Renders the Paladin's string input from the Battlefield.
    pub input_template: String,
    /// The field this node's output is written to.
    pub output_field: String,
    /// When `Some`, this node dispatches through the structured executor
    /// and the parsed JSON value is written to `output_field` (RT-05).
    #[serde(default)]
    pub output_schema: Option<SchemaRefDoc>,
}

/// A `kind: "gate"` node's document form, compiling to [`NodeSpec::Gate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct GateNodeDoc {
    /// The shape of input this Gate awaits.
    pub parley: ParleyKindDoc,
    /// Renders the raised request's prompt from the Battlefield.
    pub prompt_template: String,
    /// Renders the raised request's payload from the Battlefield, if
    /// declared.
    #[serde(default)]
    pub payload_template: Option<String>,
    /// Valid choices, for [`ParleyKindDoc::Choice`].
    #[serde(default)]
    pub choices: Option<Vec<String>>,
    /// How long after raising this request expires, if ever.
    #[serde(default)]
    pub expires_in_secs: Option<u64>,
    /// What happens if this request expires unanswered.
    #[serde(default)]
    pub on_expire: OnExpireDoc,
    /// The field the delivered value is written to; required for every
    /// [`ParleyKindDoc`] except `StateEdit`, which must leave this `None`
    /// (`WarGraph::validate` enforces the pairing, D-05).
    #[serde(default)]
    pub output_field: Option<String>,
}

/// A `kind: "workflow"` node's document form, compiling recursively to
/// [`NodeSpec::Battalion`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WorkflowNodeDoc {
    /// The nested document, compiled with the SAME [`EngineRegistries`]
    /// bundle the parent uses (Phase 23 D-21).
    pub graph: Box<WarGraphDoc>,
    /// The declared parent/child state channel (CF-FR-14).
    #[serde(default)]
    pub state_map: StateMapDoc,
    /// Whether a resumed run restarts this child from scratch.
    #[serde(default)]
    pub restart_on_resume: bool,
}

/// Mirrors [`crate::engine::graph::StateMap`]: `(parent, child)` input pairs
/// and `(child, parent)` output pairs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StateMapDoc {
    /// `(parent field, child field)` pairs.
    #[serde(default)]
    pub inputs: Vec<(String, String)>,
    /// `(child field, parent field)` pairs.
    #[serde(default)]
    pub outputs: Vec<(String, String)>,
}

/// Mirrors [`ParleyKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ParleyKindDoc {
    /// A yes/no decision.
    Approval,
    /// A choice among `choices`.
    Choice,
    /// Free-form text input.
    FreeText,
    /// A structured state edit.
    StateEdit,
}

/// Mirrors [`OnExpire`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OnExpireDoc {
    /// Fail the run with a structured reason naming the parley (default).
    #[default]
    FailRun,
    /// Substitute this value as the response.
    ResumeWithDefault {
        /// The substituted value.
        value: serde_json::Value,
    },
}

/// One declared edge in a [`WarGraphDoc`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct EdgeDoc {
    /// The source node id.
    pub from: String,
    /// The target node id.
    pub to: String,
    /// The edge's traversal condition; `None` behaves like `Always`.
    #[serde(default)]
    pub condition: Option<EdgeConditionDoc>,
}

/// Mirrors [`EdgeCondition`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EdgeConditionDoc {
    /// Always traverse this edge.
    Always,
    /// Traverse if the output contains this string.
    Contains {
        /// The substring to search for.
        value: String,
    },
    /// Traverse if the output matches this regex.
    Regex {
        /// The regex pattern.
        pattern: String,
    },
    /// Traverse according to a registered evaluator named here; an
    /// unregistered name is [`CompileError::UnregisteredEdgeEvaluator`].
    Custom {
        /// The registered evaluator name.
        name: String,
    },
}

/// A reference to a JSON Schema, either carried inline or resolved by name
/// through [`EngineRegistries::output_schemas`]. Mirrors
/// [`SchemaRef`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SchemaRefDoc {
    /// A JSON Schema value carried inline.
    Inline {
        /// The inline schema value.
        schema: serde_json::Value,
    },
    /// The name of a schema registered elsewhere; an unregistered name is
    /// [`CompileError::UnregisteredOutputSchema`].
    Registered {
        /// The registered schema name.
        name: String,
    },
}

/// Mirrors [`Aegis`]: every field independently optional.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct AegisDoc {
    /// Retry policy, if this node retries a failed attempt.
    #[serde(default)]
    pub retry: Option<RetryPolicyDoc>,
    /// Timeout policy, if this node is time-bounded.
    #[serde(default)]
    pub timeout: Option<TimeoutPolicyDoc>,
    /// Typed error handler, if a failure should be compensated instead of
    /// failing the run.
    #[serde(default)]
    pub on_error: Option<ErrorHandlerSpecDoc>,
    /// Cache policy, if this node's result may be served from a cache.
    #[serde(default)]
    pub cache: Option<CachePolicyDoc>,
}

/// Mirrors [`RetryPolicy`]. Every field falls back to
/// [`RetryPolicy::default`]'s value when absent (the same defaults, via
/// `#[serde(default)]` at the container level backed by this type's own
/// `Default` impl below).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case", default)]
pub struct RetryPolicyDoc {
    /// Maximum number of attempts, including the first. `0` is invalid.
    pub max_attempts: u32,
    /// The delay, in milliseconds, before attempt 2.
    pub initial_interval_ms: u64,
    /// The multiplier applied per additional attempt.
    pub backoff_factor: f64,
    /// The ceiling every computed delay is capped at, in milliseconds.
    pub max_interval_ms: u64,
    /// Whether to add random jitter to the computed delay.
    pub jitter: bool,
    /// Which classified errors this policy retries.
    pub retry_on: RetryPredicateDoc,
}

impl Default for RetryPolicyDoc {
    /// Matches [`RetryPolicy::default`] exactly: `max_attempts: 3`,
    /// `initial_interval_ms: 500`, `backoff_factor: 2.0`,
    /// `max_interval_ms: 60_000`, `jitter: true`, `retry_on: TransientOnly`.
    fn default() -> Self {
        let d = RetryPolicy::default();
        Self {
            max_attempts: d.max_attempts,
            initial_interval_ms: d.initial_interval.as_millis() as u64,
            backoff_factor: d.backoff_factor,
            max_interval_ms: d.max_interval.as_millis() as u64,
            jitter: d.jitter,
            retry_on: RetryPredicateDoc::TransientOnly,
        }
    }
}

/// Mirrors [`RetryPredicate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RetryPredicateDoc {
    /// Retry only transient errors (the default).
    TransientOnly,
    /// Retry transient and unknown errors.
    TransientAndUnknown,
    /// Retry according to a registered predicate evaluator named here; an
    /// unregistered name is [`CompileError::UnregisteredRetryPredicate`].
    Custom {
        /// The registered predicate name.
        name: String,
    },
}

/// Mirrors [`TimeoutPolicy`], in whole seconds (sub-second per-attempt
/// timeouts are not a documented use case this phase).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct TimeoutPolicyDoc {
    /// A hard wall-clock cap on each attempt, in seconds.
    #[serde(default)]
    pub run_timeout_secs: Option<u64>,
    /// The longest an attempt may go without progress, in seconds.
    #[serde(default)]
    pub idle_timeout_secs: Option<u64>,
}

/// Mirrors [`ErrorHandlerSpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ErrorHandlerSpecDoc {
    /// Route the error to another node.
    Route {
        /// The node to route to.
        to: String,
        /// The field the error is written into.
        error_field: String,
    },
    /// Absorb the error by merging a fallback delta.
    Absorb {
        /// The delta merged in place of the failed node's own delta,
        /// keyed by field name.
        fallback_delta: HashMap<String, serde_json::Value>,
    },
    /// Handle according to a registered handler named here; an
    /// unregistered name is [`CompileError::UnregisteredErrorHandler`].
    Custom {
        /// The registered handler name.
        name: String,
    },
}

/// Mirrors [`CachePolicy`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CachePolicyDoc {
    /// How long a cached result stays valid, in seconds.
    pub ttl_secs: u64,
    /// How the cache key is composed.
    #[serde(default)]
    pub key: CacheKeySpecDoc,
}

/// Mirrors [`CacheKeySpec`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CacheKeySpecDoc {
    /// The engine's own default key composition.
    #[default]
    Default,
    /// Key on only these declared fields' values.
    Fields {
        /// The field names to key on.
        fields: Vec<String>,
    },
}

/// One field of a [`WarGraphDoc`]'s Battlefield schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct FieldDoc {
    /// The field's name.
    pub name: String,
    /// The field's expected JSON value shape -- informational only this
    /// phase: `compile` does not cross-check it against `reducer` or
    /// `default`.
    pub kind: FieldKindDoc,
    /// The merge strategy applied to deltas targeting this field.
    pub reducer: ReducerDoc,
    /// Default value used when the field is absent.
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    /// Whether the engine must refuse to start a run that cannot resolve
    /// this field.
    #[serde(default)]
    pub required: bool,
}

/// The declared JSON value shape of a [`FieldDoc`] -- documentation only
/// (see [`FieldDoc::kind`]'s own rustdoc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FieldKindDoc {
    /// A JSON string.
    String,
    /// A JSON number.
    Number,
    /// A JSON boolean.
    Boolean,
    /// A JSON array.
    Array,
    /// A JSON object.
    Object,
    /// Any JSON value.
    Any,
}

/// Mirrors [`DispatchRule`] -- excludes `DispatchRule::Custom`: a document
/// has no way to name a `Battlefield`-level custom dispatch resolver
/// (`WarGraphDoc::compile` always validates against an empty
/// `CustomDispatchResolver`), so declaring one here would always fail
/// `WarGraph::validate` and never resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReducerDoc {
    /// Last write wins.
    LastWrite,
    /// Value must be a JSON array; deltas append.
    Append,
    /// Value must be a JSON object; deltas shallow-merge keys.
    MergeObject,
    /// Numeric accumulation.
    Sum,
}

/// A [`WarGraphDoc`]'s Battlefield schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct SchemaDoc {
    /// This schema's declared fields.
    #[serde(default)]
    pub fields: Vec<FieldDoc>,
}

/// Mirrors [`EngineLimits`]. Every field falls back to
/// [`EngineLimits::default`]'s value when absent, via this type's own
/// `Default` impl and `#[serde(default)]` on [`WarGraphDoc::limits`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields, rename_all = "snake_case", default)]
pub struct LimitsDoc {
    /// Maximum number of supersteps before the run fails.
    pub max_supersteps: u64,
    /// Maximum number of times any single node may execute within one run.
    pub max_node_visits: u32,
    /// Optional wall-clock budget for the whole run, in seconds.
    pub run_timeout_secs: Option<u64>,
    /// Maximum number of tasks a single Muster directive may request.
    pub max_muster_tasks: u32,
}

impl Default for LimitsDoc {
    fn default() -> Self {
        let d = EngineLimits::default();
        Self {
            max_supersteps: d.max_supersteps,
            max_node_visits: d.max_node_visits,
            run_timeout_secs: d.run_timeout.map(|t| t.as_secs()),
            max_muster_tasks: d.max_muster_tasks,
        }
    }
}

impl From<&LimitsDoc> for EngineLimits {
    fn from(doc: &LimitsDoc) -> Self {
        EngineLimits {
            max_supersteps: doc.max_supersteps,
            max_node_visits: doc.max_node_visits,
            run_timeout: doc.run_timeout_secs.map(Duration::from_secs),
            max_muster_tasks: doc.max_muster_tasks,
        }
    }
}

/// Every way [`WarGraphDoc::compile`] can reject a document (X-06):
/// structured variants naming the offending node, edge, or name, never a
/// bare string. `#[non_exhaustive]`: a future document field may need its
/// own resolution failure without that being a breaking change.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CompileError {
    /// `schema_version` was not [`WARGRAPH_DOC_SCHEMA_VERSION`] (X-04).
    #[error("unknown WarGraphDoc schema version: {found} (expected {WARGRAPH_DOC_SCHEMA_VERSION})")]
    UnknownSchemaVersion {
        /// The version string found on the document.
        found: String,
    },
    /// Two nodes declared the same id.
    #[error("duplicate node id: {id}")]
    DuplicateNode {
        /// The offending, repeated node id.
        id: String,
    },
    /// An entry, edge endpoint, or state-map reference named a node id
    /// absent from `nodes`.
    #[error("unknown node referenced in document: {id}")]
    UnknownNode {
        /// The referenced, undeclared node id.
        id: String,
    },
    /// A node's `kind` was not one of `paladin`, `gate`, `workflow`
    /// (D-33 scope correction).
    #[error(
        "unsupported node kind '{kind}': v0.10 documents support only paladin, gate, and workflow; \
         a custom Rust node (`NodeSpec::Function`) is not expressible from a document -- register \
         a code-defined assistant instead"
    )]
    UnsupportedNodeKind {
        /// The rejected kind string.
        kind: String,
    },
    /// A node declared `kind` but left the matching body field absent.
    #[error("node '{node}' declares kind '{kind}' but its matching body is absent")]
    MissingNodeBody {
        /// The offending node's id.
        node: String,
        /// The declared kind whose body is missing.
        kind: String,
    },
    /// An edge's `Custom` condition named an evaluator absent from
    /// [`EngineRegistries::edge_evaluators`].
    #[error("edge {from} -> {to}: unregistered custom edge evaluator '{name}'")]
    UnregisteredEdgeEvaluator {
        /// The edge's source node.
        from: String,
        /// The edge's target node.
        to: String,
        /// The unregistered evaluator name.
        name: String,
    },
    /// An `Aegis.retry.retry_on`'s `Custom` predicate named an evaluator
    /// absent from [`EngineRegistries::retry_predicates`].
    #[error("node '{node}': unregistered custom retry predicate '{name}'")]
    UnregisteredRetryPredicate {
        /// The offending node's id (or `<default_aegis>` for the
        /// graph-wide default).
        node: String,
        /// The unregistered predicate name.
        name: String,
    },
    /// An `Aegis.on_error`'s `Custom` handler named a handler absent from
    /// [`EngineRegistries::error_handlers`].
    #[error("node '{node}': unregistered custom error handler '{name}'")]
    UnregisteredErrorHandler {
        /// The offending node's id (or `<default_aegis>` for the
        /// graph-wide default).
        node: String,
        /// The unregistered handler name.
        name: String,
    },
    /// A `SchemaRefDoc::Registered` name absent from
    /// [`EngineRegistries::output_schemas`].
    #[error("node '{node}': unregistered output schema '{name}'")]
    UnregisteredOutputSchema {
        /// The offending node's id.
        node: String,
        /// The unregistered schema name.
        name: String,
    },
    /// A field name string failed [`FieldName::new`] (e.g. empty).
    #[error("node '{node}': invalid field name '{field}'")]
    InvalidFieldName {
        /// The offending node's id (or context marker).
        node: String,
        /// The rejected field name string.
        field: String,
    },
    /// A `workflow` node's nested documents recursed past
    /// `MAX_NESTING_DEPTH` (T-27-05-03).
    #[error("workflow nesting too deep: exceeded max depth of {max}")]
    NestingTooDeep {
        /// The configured maximum nesting depth.
        max: u32,
    },
    /// The fully-built graph failed [`WarGraph::validate`] -- every
    /// structural rule `compile`'s own document-level checks do not
    /// already cover (limits, unreachable nodes, aegis-on-undeclared-node,
    /// and so on).
    #[error("graph validation failed: {source}")]
    Invalid {
        /// The underlying validation failure.
        #[source]
        source: EngineError,
    },
}

fn parse_field_name(raw: &str, node: &str) -> Result<FieldName, CompileError> {
    FieldName::new(raw).map_err(|_| CompileError::InvalidFieldName {
        node: node.to_string(),
        field: raw.to_string(),
    })
}

fn compile_schema(doc: &SchemaDoc) -> Result<BattlefieldSchema, CompileError> {
    let mut fields = Vec::with_capacity(doc.fields.len());
    for field in &doc.fields {
        let name = parse_field_name(&field.name, "<schema>")?;
        let dispatch = match field.reducer {
            ReducerDoc::LastWrite => DispatchRule::LastWrite,
            ReducerDoc::Append => DispatchRule::Append,
            ReducerDoc::MergeObject => DispatchRule::MergeObject,
            ReducerDoc::Sum => DispatchRule::Sum,
        };
        fields.push(FieldSpec::new(
            name,
            dispatch,
            field.default.clone(),
            field.required,
        ));
    }
    Ok(BattlefieldSchema::new(fields))
}

fn compile_edge_condition(
    edge: &EdgeDoc,
    registries: &EngineRegistries,
) -> Result<Option<EdgeCondition>, CompileError> {
    match &edge.condition {
        None => Ok(None),
        Some(EdgeConditionDoc::Always) => Ok(Some(EdgeCondition::Always)),
        Some(EdgeConditionDoc::Contains { value }) => {
            Ok(Some(EdgeCondition::Contains(value.clone())))
        }
        Some(EdgeConditionDoc::Regex { pattern }) => {
            Ok(Some(EdgeCondition::Regex(pattern.clone())))
        }
        Some(EdgeConditionDoc::Custom { name }) => {
            if registries.edge_evaluators.contains(name) {
                Ok(Some(EdgeCondition::Custom(name.clone())))
            } else {
                Err(CompileError::UnregisteredEdgeEvaluator {
                    from: edge.from.clone(),
                    to: edge.to.clone(),
                    name: name.clone(),
                })
            }
        }
    }
}

fn compile_aegis(
    doc: &AegisDoc,
    node: &str,
    registries: &EngineRegistries,
) -> Result<Aegis, CompileError> {
    let retry = doc
        .retry
        .as_ref()
        .map(|r| {
            let retry_on = match &r.retry_on {
                RetryPredicateDoc::TransientOnly => RetryPredicate::TransientOnly,
                RetryPredicateDoc::TransientAndUnknown => RetryPredicate::TransientAndUnknown,
                RetryPredicateDoc::Custom { name } => {
                    if registries.retry_predicates.contains(name) {
                        RetryPredicate::Custom(name.clone())
                    } else {
                        return Err(CompileError::UnregisteredRetryPredicate {
                            node: node.to_string(),
                            name: name.clone(),
                        });
                    }
                }
            };
            Ok(RetryPolicy {
                max_attempts: r.max_attempts,
                initial_interval: Duration::from_millis(r.initial_interval_ms),
                backoff_factor: r.backoff_factor,
                max_interval: Duration::from_millis(r.max_interval_ms),
                jitter: r.jitter,
                retry_on,
            })
        })
        .transpose()?;

    let timeout = doc.timeout.as_ref().map(|t| TimeoutPolicy {
        run_timeout: t.run_timeout_secs.map(Duration::from_secs),
        idle_timeout: t.idle_timeout_secs.map(Duration::from_secs),
    });

    let on_error = doc
        .on_error
        .as_ref()
        .map(|oe| compile_error_handler(oe, node, registries))
        .transpose()?;

    let cache = doc
        .cache
        .as_ref()
        .map(|c| compile_cache_policy(c, node))
        .transpose()?;

    Ok(Aegis {
        retry,
        timeout,
        on_error,
        cache,
    })
}

fn compile_error_handler(
    doc: &ErrorHandlerSpecDoc,
    node: &str,
    registries: &EngineRegistries,
) -> Result<ErrorHandlerSpec, CompileError> {
    match doc {
        ErrorHandlerSpecDoc::Route { to, error_field } => {
            let field = parse_field_name(error_field, node)?;
            Ok(ErrorHandlerSpec::Route {
                to: NodeId::new(to.clone()),
                error_field: field,
            })
        }
        ErrorHandlerSpecDoc::Absorb { fallback_delta } => {
            let mut delta = StateDelta::new();
            for (raw_field, value) in fallback_delta {
                let field = parse_field_name(raw_field, node)?;
                delta.set_raw(field, value.clone());
            }
            Ok(ErrorHandlerSpec::Absorb {
                fallback_delta: delta,
            })
        }
        ErrorHandlerSpecDoc::Custom { name } => {
            if registries.error_handlers.contains(name) {
                Ok(ErrorHandlerSpec::Custom(name.clone()))
            } else {
                Err(CompileError::UnregisteredErrorHandler {
                    node: node.to_string(),
                    name: name.clone(),
                })
            }
        }
    }
}

fn compile_cache_policy(doc: &CachePolicyDoc, node: &str) -> Result<CachePolicy, CompileError> {
    let key = match &doc.key {
        CacheKeySpecDoc::Default => CacheKeySpec::Default,
        CacheKeySpecDoc::Fields { fields } => {
            let parsed: Result<Vec<FieldName>, CompileError> =
                fields.iter().map(|f| parse_field_name(f, node)).collect();
            CacheKeySpec::Fields(parsed?)
        }
    };
    Ok(CachePolicy {
        ttl: Duration::from_secs(doc.ttl_secs),
        key,
    })
}

fn compile_state_map(doc: &StateMapDoc) -> Result<StateMap, CompileError> {
    let mut state_map = StateMap::new();
    for (parent, child) in &doc.inputs {
        let parent = parse_field_name(parent, "<workflow.state_map.inputs>")?;
        let child = parse_field_name(child, "<workflow.state_map.inputs>")?;
        state_map = state_map.with_input(parent, child);
    }
    for (child, parent) in &doc.outputs {
        let child = parse_field_name(child, "<workflow.state_map.outputs>")?;
        let parent = parse_field_name(parent, "<workflow.state_map.outputs>")?;
        state_map = state_map.with_output(child, parent);
    }
    Ok(state_map)
}

fn compile_node(
    node: &NodeDoc,
    registries: &EngineRegistries,
    depth: u32,
) -> Result<NodeSpec, CompileError> {
    match node.kind_doc()? {
        NodeKindDoc::Paladin(p) => {
            let data = PaladinData {
                system_prompt: p.system_prompt.clone(),
                name: p.name.clone(),
                user_name: String::new(),
                model: p.model.clone(),
                temperature: p.temperature.unwrap_or(0.7),
                max_loops: MaxLoops::Fixed(p.max_loops.unwrap_or(3)),
                stop_words: p.stop_words.clone(),
                status: PaladinStatus::Idle,
                vision_enabled: false,
                autonomous_planning: false,
                autonomous_prompts: false,
                agent_description: String::new(),
                dynamic_temperature: false,
            };
            let paladin: Paladin = Node::new(data, Some(p.name.clone()));
            let input_template = InputMapping::new(p.input_template.clone());
            let output_field = parse_field_name(&p.output_field, node.id.as_str())?;

            let output_schema = match &p.output_schema {
                None => None,
                Some(SchemaRefDoc::Inline { schema }) => Some(SchemaRef::Inline(schema.clone())),
                Some(SchemaRefDoc::Registered { name }) => {
                    if registries.output_schemas.contains_key(name) {
                        Some(SchemaRef::Registered(name.clone()))
                    } else {
                        return Err(CompileError::UnregisteredOutputSchema {
                            node: node.id.clone(),
                            name: name.clone(),
                        });
                    }
                }
            };

            Ok(NodeSpec::Paladin {
                paladin: Box::new(paladin),
                input_template,
                output_field,
                directive_parser: crate::engine::DirectiveParser::PlainOutput,
                output_schema,
            })
        }
        NodeKindDoc::Gate(g) => {
            let kind = match g.parley {
                ParleyKindDoc::Approval => ParleyKind::Approval,
                ParleyKindDoc::Choice => ParleyKind::Choice,
                ParleyKindDoc::FreeText => ParleyKind::FreeText,
                ParleyKindDoc::StateEdit => ParleyKind::StateEdit,
            };
            let prompt_template = InputMapping::new(g.prompt_template.clone());
            let mut request = GateRequestTemplate::new(kind, prompt_template);
            if let Some(payload_template) = &g.payload_template {
                request =
                    request.with_payload_template(InputMapping::new(payload_template.clone()));
            }
            if let Some(choices) = &g.choices {
                request = request.with_choices(choices.clone());
            }
            if let Some(secs) = g.expires_in_secs {
                request = request.with_expires_in(Duration::from_secs(secs));
            }
            let on_expire = match &g.on_expire {
                OnExpireDoc::FailRun => OnExpire::FailRun,
                OnExpireDoc::ResumeWithDefault { value } => {
                    OnExpire::ResumeWithDefault(value.clone())
                }
            };
            request = request.with_on_expire(on_expire);

            let output_field = g
                .output_field
                .as_ref()
                .map(|f| parse_field_name(f, node.id.as_str()))
                .transpose()?;

            Ok(NodeSpec::Gate {
                request,
                output_field,
            })
        }
        NodeKindDoc::Workflow(w) => {
            let child = w.graph.compile_at_depth(registries, depth + 1)?;
            let state_map = compile_state_map(&w.state_map)?;
            Ok(NodeSpec::Battalion {
                graph: Arc::new(child),
                state_map,
                restart_on_resume: w.restart_on_resume,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge_evaluator::{EdgeConditionEvaluator, EdgeContext, EdgeEvaluatorError};
    use crate::error_handler::ErrorHandler;
    use crate::retry_predicate::RetryPredicateEvaluator;
    use async_trait::async_trait;
    use paladin_core::platform::container::battlefield::Battlefield;
    use paladin_core::platform::container::directive::Directive;
    use paladin_core::platform::container::node_error::NodeError;

    fn gate_node(id: &str, output_field: Option<&str>) -> NodeDoc {
        NodeDoc {
            id: id.to_string(),
            kind: "gate".to_string(),
            paladin: None,
            gate: Some(GateNodeDoc {
                parley: ParleyKindDoc::Approval,
                prompt_template: "Approve?".to_string(),
                payload_template: None,
                choices: None,
                expires_in_secs: None,
                on_expire: OnExpireDoc::FailRun,
                output_field: output_field.map(str::to_string),
            }),
            workflow: None,
            aegis: None,
            defer: false,
        }
    }

    fn paladin_node(id: &str, output_field: &str) -> NodeDoc {
        NodeDoc {
            id: id.to_string(),
            kind: "paladin".to_string(),
            paladin: Some(PaladinNodeDoc {
                name: "Writer".to_string(),
                model: "gpt-4".to_string(),
                system_prompt: "Write".to_string(),
                temperature: None,
                max_loops: None,
                stop_words: vec![],
                input_template: "{approved}".to_string(),
                output_field: output_field.to_string(),
                output_schema: None,
            }),
            gate: None,
            workflow: None,
            aegis: None,
            defer: false,
        }
    }

    fn approved_field() -> FieldDoc {
        FieldDoc {
            name: "approved".to_string(),
            kind: FieldKindDoc::Boolean,
            reducer: ReducerDoc::LastWrite,
            // A Gate::Approval field's type is inferred from its schema
            // default (`WarGraph::validate`'s `GateOutputFieldTypeIncompatible`
            // check) -- `false` declares this field boolean-typed, matching
            // an Approval Gate's normalized true/false delivery.
            default: Some(serde_json::json!(false)),
            required: false,
        }
    }

    /// `WarGraph` deliberately carries no `Debug` derive (its `NodeSpec`
    /// map holds `Arc<dyn StateNode>`, D-19's own note), so
    /// `Result::unwrap_err` -- which bounds `T: Debug` -- cannot be used on
    /// a `Result<WarGraph, CompileError>` directly; this helper extracts
    /// the error without that bound.
    fn expect_err(result: Result<WarGraph, CompileError>) -> CompileError {
        match result {
            Ok(_) => panic!("expected CompileError, got a compiled WarGraph"),
            Err(e) => e,
        }
    }

    fn minimal_doc() -> WarGraphDoc {
        WarGraphDoc {
            schema_version: WARGRAPH_DOC_SCHEMA_VERSION.to_string(),
            entry: vec!["review".to_string()],
            nodes: vec![gate_node("review", Some("approved"))],
            edges: vec![EdgeDoc {
                from: "review".to_string(),
                to: "review".to_string(),
                condition: Some(EdgeConditionDoc::Contains {
                    value: "true".to_string(),
                }),
            }],
            schema: SchemaDoc {
                fields: vec![approved_field()],
            },
            limits: LimitsDoc::default(),
            default_aegis: None,
        }
    }

    #[test]
    fn minimal_gate_document_compiles_and_validates() {
        let doc = minimal_doc();
        let graph = doc.compile(&EngineRegistries::new()).unwrap();
        let mut node_ids: Vec<&str> = graph.node_order().iter().map(|id| id.as_str()).collect();
        node_ids.sort();
        assert_eq!(node_ids, vec!["review"]);
    }

    #[test]
    fn unknown_schema_version_is_typed() {
        let mut doc = minimal_doc();
        doc.schema_version = "999".to_string();
        let err = expect_err(doc.compile(&EngineRegistries::new()));
        assert!(matches!(
            err,
            CompileError::UnknownSchemaVersion { found } if found == "999"
        ));
    }

    #[test]
    fn duplicate_node_id_is_typed() {
        let mut doc = minimal_doc();
        doc.nodes.push(gate_node("review", Some("approved")));
        let err = expect_err(doc.compile(&EngineRegistries::new()));
        assert!(matches!(err, CompileError::DuplicateNode { id } if id == "review"));
    }

    #[test]
    fn unknown_entry_node_is_typed() {
        let mut doc = minimal_doc();
        doc.entry = vec!["missing".to_string()];
        let err = expect_err(doc.compile(&EngineRegistries::new()));
        assert!(matches!(err, CompileError::UnknownNode { id } if id == "missing"));
    }

    #[test]
    fn unknown_edge_endpoint_is_typed() {
        let mut doc = minimal_doc();
        doc.edges.push(EdgeDoc {
            from: "review".to_string(),
            to: "ghost".to_string(),
            condition: None,
        });
        let err = expect_err(doc.compile(&EngineRegistries::new()));
        assert!(matches!(err, CompileError::UnknownNode { id } if id == "ghost"));
    }

    /// `wargraph_doc_unsupported_node_kind` (named for 27-VALIDATION.md).
    #[test]
    fn wargraph_doc_unsupported_node_kind() {
        let mut doc = minimal_doc();
        doc.nodes[0].kind = "function".to_string();
        doc.nodes[0].gate = None;
        let err = expect_err(doc.compile(&EngineRegistries::new()));
        assert!(matches!(
            err,
            CompileError::UnsupportedNodeKind { kind } if kind == "function"
        ));
    }

    #[test]
    fn missing_node_body_is_typed() {
        let mut doc = minimal_doc();
        doc.nodes[0].gate = None;
        let err = expect_err(doc.compile(&EngineRegistries::new()));
        assert!(matches!(
            err,
            CompileError::MissingNodeBody { node, kind } if node == "review" && kind == "gate"
        ));
    }

    #[test]
    fn unregistered_edge_evaluator_is_typed() {
        let mut doc = minimal_doc();
        doc.edges[0].condition = Some(EdgeConditionDoc::Custom {
            name: "my_eval".to_string(),
        });
        let err = expect_err(doc.compile(&EngineRegistries::new()));
        assert!(matches!(
            err,
            CompileError::UnregisteredEdgeEvaluator { from, to, name }
                if from == "review" && to == "review" && name == "my_eval"
        ));
    }

    struct AlwaysTrue;
    #[async_trait]
    impl EdgeConditionEvaluator for AlwaysTrue {
        async fn evaluate(
            &self,
            _output: &str,
            _ctx: &EdgeContext<'_>,
        ) -> Result<bool, EdgeEvaluatorError> {
            Ok(true)
        }
    }

    #[test]
    fn registered_edge_evaluator_compiles() {
        let mut doc = minimal_doc();
        doc.edges[0].condition = Some(EdgeConditionDoc::Custom {
            name: "my_eval".to_string(),
        });
        let mut registries = EngineRegistries::new();
        registries
            .edge_evaluators
            .register("my_eval", Arc::new(AlwaysTrue));
        assert!(doc.compile(&registries).is_ok());
    }

    #[test]
    fn unregistered_retry_predicate_is_typed() {
        let mut doc = minimal_doc();
        doc.nodes[0].aegis = Some(AegisDoc {
            retry: Some(RetryPolicyDoc {
                retry_on: RetryPredicateDoc::Custom {
                    name: "my_pred".to_string(),
                },
                ..RetryPolicyDoc::default()
            }),
            ..AegisDoc::default()
        });
        let err = expect_err(doc.compile(&EngineRegistries::new()));
        assert!(matches!(
            err,
            CompileError::UnregisteredRetryPredicate { node, name }
                if node == "review" && name == "my_pred"
        ));
    }

    struct AlwaysRetry;
    #[async_trait]
    impl RetryPredicateEvaluator for AlwaysRetry {
        async fn allows(
            &self,
            _err: &NodeError,
            _attempt: u32,
        ) -> Result<bool, crate::retry_predicate::RetryPredicateError> {
            Ok(true)
        }
    }

    #[test]
    fn registered_retry_predicate_compiles() {
        let mut doc = minimal_doc();
        // Aegis is unsupported on a Gate node (`WarGraph::validate`); attach
        // it to a Paladin node instead.
        let mut writer = paladin_node("writer", "written");
        writer.aegis = Some(AegisDoc {
            retry: Some(RetryPolicyDoc {
                retry_on: RetryPredicateDoc::Custom {
                    name: "my_pred".to_string(),
                },
                ..RetryPolicyDoc::default()
            }),
            ..AegisDoc::default()
        });
        doc.nodes.push(writer);
        doc.schema.fields.push(FieldDoc {
            name: "written".to_string(),
            kind: FieldKindDoc::String,
            reducer: ReducerDoc::LastWrite,
            default: None,
            required: false,
        });
        doc.edges.push(EdgeDoc {
            from: "review".to_string(),
            to: "writer".to_string(),
            condition: Some(EdgeConditionDoc::Always),
        });
        let mut registries = EngineRegistries::new();
        registries
            .retry_predicates
            .register("my_pred", Arc::new(AlwaysRetry));
        if let Err(e) = doc.compile(&registries) {
            panic!("expected Ok, got {e}");
        }
    }

    #[test]
    fn unregistered_error_handler_is_typed() {
        let mut doc = minimal_doc();
        doc.nodes[0].aegis = Some(AegisDoc {
            on_error: Some(ErrorHandlerSpecDoc::Custom {
                name: "my_handler".to_string(),
            }),
            ..AegisDoc::default()
        });
        let err = expect_err(doc.compile(&EngineRegistries::new()));
        assert!(matches!(
            err,
            CompileError::UnregisteredErrorHandler { node, name }
                if node == "review" && name == "my_handler"
        ));
    }

    struct AlwaysAbsorb;
    #[async_trait]
    impl ErrorHandler for AlwaysAbsorb {
        async fn handle(
            &self,
            _err: &NodeError,
            _state: &Battlefield,
        ) -> Result<Directive, NodeError> {
            Ok(Directive {
                delta: paladin_core::platform::container::battlefield::StateDelta::new(),
                next: paladin_core::platform::container::directive::NextStep::Edges,
            })
        }
    }

    #[test]
    fn registered_error_handler_compiles() {
        let mut doc = minimal_doc();
        // Aegis is unsupported on a Gate node (`WarGraph::validate`); attach
        // it to a Paladin node instead.
        let mut writer = paladin_node("writer", "written");
        writer.aegis = Some(AegisDoc {
            on_error: Some(ErrorHandlerSpecDoc::Custom {
                name: "my_handler".to_string(),
            }),
            ..AegisDoc::default()
        });
        doc.nodes.push(writer);
        doc.schema.fields.push(FieldDoc {
            name: "written".to_string(),
            kind: FieldKindDoc::String,
            reducer: ReducerDoc::LastWrite,
            default: None,
            required: false,
        });
        doc.edges.push(EdgeDoc {
            from: "review".to_string(),
            to: "writer".to_string(),
            condition: Some(EdgeConditionDoc::Always),
        });
        let mut registries = EngineRegistries::new();
        registries
            .error_handlers
            .register("my_handler", Arc::new(AlwaysAbsorb));
        if let Err(e) = doc.compile(&registries) {
            panic!("expected Ok, got {e}");
        }
    }

    #[test]
    fn unregistered_output_schema_is_typed() {
        let mut doc = minimal_doc();
        doc.nodes.push(NodeDoc {
            id: "answer".to_string(),
            kind: "paladin".to_string(),
            paladin: Some(PaladinNodeDoc {
                name: "Answerer".to_string(),
                model: "gpt-4".to_string(),
                system_prompt: "Answer".to_string(),
                temperature: None,
                max_loops: None,
                stop_words: vec![],
                input_template: "{approved}".to_string(),
                output_field: "answered".to_string(),
                output_schema: Some(SchemaRefDoc::Registered {
                    name: "answer_schema".to_string(),
                }),
            }),
            gate: None,
            workflow: None,
            aegis: None,
            defer: false,
        });
        doc.schema.fields.push(FieldDoc {
            name: "answered".to_string(),
            kind: FieldKindDoc::String,
            reducer: ReducerDoc::LastWrite,
            default: None,
            required: false,
        });
        doc.edges.push(EdgeDoc {
            from: "review".to_string(),
            to: "answer".to_string(),
            condition: Some(EdgeConditionDoc::Always),
        });
        let err = expect_err(doc.compile(&EngineRegistries::new()));
        assert!(matches!(
            err,
            CompileError::UnregisteredOutputSchema { node, name }
                if node == "answer" && name == "answer_schema"
        ));
    }

    #[test]
    fn nested_workflow_compiles_to_battalion_node() {
        let inner = minimal_doc();
        let outer_nodes = vec![NodeDoc {
            id: "child".to_string(),
            kind: "workflow".to_string(),
            paladin: None,
            gate: None,
            workflow: Some(WorkflowNodeDoc {
                graph: Box::new(inner),
                state_map: StateMapDoc::default(),
                restart_on_resume: false,
            }),
            aegis: None,
            defer: false,
        }];
        let doc = WarGraphDoc {
            schema_version: WARGRAPH_DOC_SCHEMA_VERSION.to_string(),
            entry: vec!["child".to_string()],
            nodes: outer_nodes,
            edges: vec![],
            schema: SchemaDoc { fields: vec![] },
            limits: LimitsDoc::default(),
            default_aegis: None,
        };
        let graph = doc.compile(&EngineRegistries::new()).unwrap();
        assert_eq!(graph.node_order(), &[NodeId::new("child")]);
    }

    #[test]
    fn nesting_too_deep_is_typed() {
        // Build a chain of MAX_NESTING_DEPTH + 1 nested workflow documents.
        let mut innermost = minimal_doc();
        for i in 0..=MAX_NESTING_DEPTH {
            let wrapper = WarGraphDoc {
                schema_version: WARGRAPH_DOC_SCHEMA_VERSION.to_string(),
                entry: vec!["child".to_string()],
                nodes: vec![NodeDoc {
                    id: "child".to_string(),
                    kind: "workflow".to_string(),
                    paladin: None,
                    gate: None,
                    workflow: Some(WorkflowNodeDoc {
                        graph: Box::new(innermost),
                        state_map: StateMapDoc::default(),
                        restart_on_resume: false,
                    }),
                    aegis: None,
                    defer: false,
                }],
                edges: vec![],
                schema: SchemaDoc { fields: vec![] },
                limits: LimitsDoc::default(),
                default_aegis: None,
            };
            innermost = wrapper;
            let _ = i;
        }
        let err = expect_err(innermost.compile(&EngineRegistries::new()));
        assert!(matches!(err, CompileError::NestingTooDeep { max } if max == MAX_NESTING_DEPTH));
    }

    #[test]
    fn round_trip_preserves_json_value() {
        let doc = minimal_doc();
        let value = serde_json::to_value(&doc).unwrap();
        let parsed: WarGraphDoc = serde_json::from_value(value.clone()).unwrap();
        let value2 = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value, value2);
    }

    #[test]
    fn deny_unknown_fields_rejects_typo() {
        let mut value = serde_json::to_value(minimal_doc()).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("schmea_version_typo".to_string(), serde_json::json!("1"));
        let result: Result<WarGraphDoc, _> = serde_json::from_value(value);
        assert!(result.is_err());
    }
}
