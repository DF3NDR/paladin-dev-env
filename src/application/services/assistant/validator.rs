//! Compile-is-validation for assistant definitions (D-31, PLAT-FR-09).
//!
//! [`AssistantValidator::validate`] is the ONLY gate a definition body crosses before it is
//! persisted: an `Agent` body validates structurally against [`AgentDefinition`] (the
//! facade's own web-independent twin of `paladin-web`'s `AgentSpec` JSON shape, D-33/27-
//! RESEARCH Open Question 1); a `Workflow` body validates by deserialising to a
//! `WarGraphDoc` and calling `compile()` against the caller's live `EngineRegistries` --
//! compile IS validation (D-31, 27-05). Every failure is a non-empty
//! `Vec<ValidationViolation>`; nothing partially built is ever returned.

use std::sync::Arc;

use paladin_battalion::engine::WarGraph;
use paladin_battalion::engine::graph_doc::{CompileError, WarGraphDoc};
use paladin_battalion::engine::registries::EngineRegistries;
use paladin_core::base::entity::node::Node;
use paladin_core::platform::container::assistant::{AssistantDefinition, AssistantKind};
use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData, PaladinStatus};
use paladin_core::platform::container::user::UserRole;
use paladin_ports::input::assistant_admin_port::ValidationViolation;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Field names an assistant definition body must never carry, at any depth (prohibition
/// P1, threat T-27-12-04): `model` is a provider-agnostic NAME resolved server-side, never
/// a place to smuggle a credential.
const FORBIDDEN_CREDENTIAL_KEYS: [&str; 4] = ["api_key", "token", "secret", "authorization"];

/// The facade's web-independent twin of `paladin-web`'s `AgentSpec` JSON shape (27-
/// RESEARCH.md Open Question 1 resolution) -- deliberately its own type rather than an
/// extension of `AgentSpec`, so `src/application/services/assistant/*` never names
/// `paladin-web` (grep-gated, X-01/ADR-0031: `paladin-web` is an optional dependency this
/// module must not require).
///
/// `tools` and `middleware` are accepted (so the wire schema is forward-compatible with a
/// later phase that wires them) but REJECTED with `code: "unsupported_in_v0_10"` if
/// non-empty -- v0.10 has no tool-execution or middleware pipeline behind a stored
/// assistant yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Human-friendly display name.
    pub name: String,
    /// LLM model identifier (e.g. `"gpt-4"`).
    pub model: String,
    /// System prompt defining the agent's behavior.
    pub system_prompt: String,
    /// Response randomness (`0.0`-`1.0`).
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Stop words that terminate execution.
    #[serde(default)]
    pub stop_words: Vec<String>,
    /// Per-agent execution timeout, in seconds. Must be `> 0` when present.
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    /// Roles permitted to invoke this assistant; empty means any authenticated caller
    /// (D-46).
    #[serde(default)]
    pub allowed_roles: Vec<UserRole>,
    /// Accepted but rejected if non-empty (`unsupported_in_v0_10`) -- forward-compatible
    /// placeholder for a later phase's tool wiring.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Accepted but rejected if non-empty (`unsupported_in_v0_10`) -- forward-compatible
    /// placeholder for a later phase's middleware wiring.
    #[serde(default)]
    pub middleware: Value,
}

/// The typed result of a successful [`AssistantValidator::validate`] call.
pub enum Validated {
    /// A valid Agent definition, already built into a runnable [`Paladin`], plus its
    /// `allowed_roles`.
    Agent(Arc<Paladin>, Vec<UserRole>),
    /// A valid Workflow definition, already compiled into a runnable [`WarGraph`].
    Workflow(Arc<WarGraph>),
}

// `WarGraph` (`paladin-battalion`) does not implement `Debug`, so this is hand-written
// rather than derived -- deliberately shallow, mirroring
// `services::run::resolver::Runnable`'s own precedent.
impl std::fmt::Debug for Validated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Validated::Agent(..) => f.write_str("Validated::Agent(..)"),
            Validated::Workflow(..) => f.write_str("Validated::Workflow(..)"),
        }
    }
}

/// Validates [`AssistantDefinition`] bodies against the caller's live engine registries
/// (D-31).
pub struct AssistantValidator {
    registries: Arc<EngineRegistries>,
}

impl AssistantValidator {
    /// Construct a validator over `registries` -- the SAME bundle the process's live
    /// `WarEngine`/`WarGraph::validate` calls resolve `Custom`/`Registered` names against,
    /// so a Workflow body that compiles here is guaranteed to run for real (D-31).
    pub fn new(registries: Arc<EngineRegistries>) -> Self {
        Self { registries }
    }

    /// Validate `definition`, returning the runnable artifact on success or a non-empty
    /// list of every violation found on failure.
    pub fn validate(
        &self,
        definition: &AssistantDefinition,
    ) -> Result<Validated, Vec<ValidationViolation>> {
        match definition.kind {
            AssistantKind::Agent => self.validate_agent(&definition.body),
            AssistantKind::Workflow => self.validate_workflow(&definition.body),
        }
    }

    fn validate_agent(&self, body: &Value) -> Result<Validated, Vec<ValidationViolation>> {
        let mut violations = Vec::new();
        scan_for_credential_fields(body, "", &mut violations);

        if is_empty_body(body) {
            violations.push(ValidationViolation::new(
                "/",
                "missing_field",
                "assistant definition body must not be empty",
            ));
            return Err(violations);
        }

        let def: AgentDefinition = match serde_json::from_value(body.clone()) {
            Ok(def) => def,
            Err(err) => {
                violations.push(ValidationViolation::new(
                    "/",
                    "invalid_shape",
                    format!("body does not match the Agent definition shape: {err}"),
                ));
                return Err(violations);
            }
        };

        if def.name.trim().is_empty() {
            violations.push(ValidationViolation::new(
                "/name",
                "empty",
                "name must not be empty",
            ));
        }
        if def.model.trim().is_empty() {
            violations.push(ValidationViolation::new(
                "/model",
                "empty",
                "model must not be empty",
            ));
        }
        if def.system_prompt.trim().is_empty() {
            violations.push(ValidationViolation::new(
                "/system_prompt",
                "empty",
                "system_prompt must not be empty",
            ));
        }
        if let Some(temperature) = def.temperature
            && !(0.0..=1.0).contains(&temperature)
        {
            violations.push(ValidationViolation::new(
                "/temperature",
                "out_of_range",
                format!("temperature must be between 0.0 and 1.0, got {temperature}"),
            ));
        }
        if let Some(timeout) = def.timeout_seconds
            && timeout == 0
        {
            violations.push(ValidationViolation::new(
                "/timeout_seconds",
                "out_of_range",
                "timeout_seconds must be greater than 0",
            ));
        }
        if !def.tools.is_empty() {
            violations.push(ValidationViolation::new(
                "/tools",
                "unsupported_in_v0_10",
                "tools are accepted for forward-compatibility but not yet supported",
            ));
        }
        if !is_empty_middleware(&def.middleware) {
            violations.push(ValidationViolation::new(
                "/middleware",
                "unsupported_in_v0_10",
                "middleware is accepted for forward-compatibility but not yet supported",
            ));
        }

        if !violations.is_empty() {
            return Err(violations);
        }

        // Mirrors `WarGraphDoc::compile`'s own `PaladinNodeDoc` -> `PaladinData`
        // construction (`crates/paladin-battalion/src/engine/graph_doc.rs`) --
        // constructed directly, not through `PaladinBuilder`, since that builder needs a
        // live `Arc<dyn LlmPort>` purely to read a provider's declared temperature range
        // (ADR-0004); this validator's own [0.0, 1.0] range check above is deliberately
        // provider-agnostic (a stored assistant definition outlives any one provider
        // wiring), so no `LlmPort` dependency is needed here at all.
        let data = PaladinData {
            system_prompt: def.system_prompt.clone(),
            name: def.name.clone(),
            user_name: String::new(),
            model: def.model.clone(),
            temperature: def.temperature.unwrap_or(0.7),
            max_loops: MaxLoops::Fixed(3),
            stop_words: def.stop_words.clone(),
            status: PaladinStatus::Idle,
            vision_enabled: false,
            autonomous_planning: false,
            autonomous_prompts: false,
            agent_description: String::new(),
            dynamic_temperature: false,
        };
        let paladin: Paladin = Node::new(data, Some(def.name.clone()));
        Ok(Validated::Agent(Arc::new(paladin), def.allowed_roles))
    }

    fn validate_workflow(&self, body: &Value) -> Result<Validated, Vec<ValidationViolation>> {
        if is_empty_body(body) {
            return Err(vec![ValidationViolation::new(
                "/",
                "missing_field",
                "assistant definition body must not be empty",
            )]);
        }

        let mut violations = Vec::new();
        scan_for_credential_fields(body, "", &mut violations);
        if !violations.is_empty() {
            return Err(violations);
        }

        let doc: WarGraphDoc = match serde_json::from_value(body.clone()) {
            Ok(doc) => doc,
            Err(err) => {
                return Err(vec![ValidationViolation::new(
                    "/",
                    "invalid_shape",
                    format!("body does not match the WarGraphDoc shape: {err}"),
                )]);
            }
        };

        match doc.compile(&self.registries) {
            Ok(graph) => Ok(Validated::Workflow(Arc::new(graph))),
            Err(err) => Err(vec![map_compile_error(&err)]),
        }
    }
}

fn is_empty_body(body: &Value) -> bool {
    body.is_null() || matches!(body, Value::Object(map) if map.is_empty())
}

fn is_empty_middleware(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Object(map) => map.is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

/// Recursively scan `value` for any object key matching (case-insensitively)
/// [`FORBIDDEN_CREDENTIAL_KEYS`], appending a `credential_field_forbidden` violation for
/// every one found (prohibition P1) -- `model` stays a name resolved server-side; a
/// caller-supplied credential has no legitimate reason to appear anywhere in a definition
/// body.
fn scan_for_credential_fields(
    value: &Value,
    path: &str,
    violations: &mut Vec<ValidationViolation>,
) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = format!("{path}/{key}");
                if FORBIDDEN_CREDENTIAL_KEYS
                    .iter()
                    .any(|k| k.eq_ignore_ascii_case(key))
                {
                    violations.push(ValidationViolation::new(
                        child_path.clone(),
                        "credential_field_forbidden",
                        format!("field '{key}' may not appear in an assistant definition body"),
                    ));
                }
                scan_for_credential_fields(child, &child_path, violations);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                scan_for_credential_fields(child, &format!("{path}/{index}"), violations);
            }
        }
        _ => {}
    }
}

/// Map every [`CompileError`] variant to its own distinct [`ValidationViolation`] `code`
/// (D-31's own "every `CompileError` variant maps to a distinct `code`" requirement).
/// `#[non_exhaustive]` on `CompileError` means this match needs a catch-all; it still
/// carries the real `Display` message rather than a generic string.
fn map_compile_error(err: &CompileError) -> ValidationViolation {
    match err {
        CompileError::UnknownSchemaVersion { found } => ValidationViolation::new(
            "/schema_version",
            "unknown_schema_version",
            format!("unknown WarGraphDoc schema version: {found}"),
        ),
        CompileError::DuplicateNode { id } => ValidationViolation::new(
            format!("/nodes/{id}"),
            "duplicate_node",
            format!("duplicate node id: {id}"),
        ),
        CompileError::UnknownNode { id } => ValidationViolation::new(
            format!("/nodes/{id}"),
            "unknown_node",
            format!("unknown node referenced in document: {id}"),
        ),
        CompileError::UnsupportedNodeKind { kind } => ValidationViolation::new(
            "/nodes",
            "unsupported_node_kind",
            format!(
                "unsupported node kind '{kind}': v0.10 documents support only paladin, gate, \
                 and workflow"
            ),
        ),
        CompileError::MissingNodeBody { node, kind } => ValidationViolation::new(
            format!("/nodes/{node}"),
            "missing_node_body",
            format!("node '{node}' declares kind '{kind}' but its matching body is absent"),
        ),
        CompileError::UnregisteredEdgeEvaluator { from, to, name } => ValidationViolation::new(
            format!("/edges/{from}->{to}/condition"),
            "unregistered_edge_evaluator",
            format!("unregistered custom edge evaluator '{name}'"),
        ),
        CompileError::UnregisteredRetryPredicate { node, name } => ValidationViolation::new(
            format!("/nodes/{node}/aegis/retry"),
            "unregistered_retry_predicate",
            format!("unregistered custom retry predicate '{name}'"),
        ),
        CompileError::UnregisteredErrorHandler { node, name } => ValidationViolation::new(
            format!("/nodes/{node}/aegis/on_error"),
            "unregistered_error_handler",
            format!("unregistered custom error handler '{name}'"),
        ),
        CompileError::UnregisteredOutputSchema { node, name } => ValidationViolation::new(
            format!("/nodes/{node}/output_schema"),
            "unregistered_output_schema",
            format!("unregistered output schema '{name}'"),
        ),
        CompileError::InvalidFieldName { node, field } => ValidationViolation::new(
            format!("/nodes/{node}"),
            "invalid_field_name",
            format!("invalid field name '{field}'"),
        ),
        CompileError::NestingTooDeep { max } => ValidationViolation::new(
            "/nodes",
            "nesting_too_deep",
            format!("workflow nesting exceeded max depth of {max}"),
        ),
        CompileError::Invalid { source } => ValidationViolation::new(
            "/",
            "graph_invalid",
            format!("graph validation failed: {source}"),
        ),
        other => ValidationViolation::new("/", "compile_error", other.to_string()),
    }
}
