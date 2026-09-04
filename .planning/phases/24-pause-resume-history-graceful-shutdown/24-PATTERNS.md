# Phase 24: Pause/Resume, History & Graceful Shutdown - Pattern Map

**Mapped:** 2026-09-04
**Files analyzed:** ~24 new/modified files (RESEARCH.md's Recommended Project Structure)
**Analogs found:** 22 / 24 (nearly everything is an in-place extension of existing files, not a
greenfield file — RESEARCH.md already did most of the analog identification; this document adds
role/data-flow classification and pulls concrete excerpts for planner consumption)

**Note on this phase:** Unlike a typical phase, most "files to create" are *extensions of existing
files* (`waypoint.rs`, `superstep.rs`, `mod.rs`, `graph.rs`, `engine.rs`, `agent_controller.rs`'s
sibling). The analog for an extended file is usually itself (its own established internal
conventions) plus one true sibling for the handful of genuinely new files (`shutdown.rs`,
`parley.rs`, `chronicle.rs`, `thread_controller.rs`, `parley_port.rs`, `waypoint_store.rs`).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `crates/paladin-core/.../parley.rs` (NEW) | model (value types) | transform | `crates/paladin-core/.../waypoint.rs` (`WaypointId`, `ThreadId` newtypes) | exact |
| `crates/paladin-core/.../waypoint.rs` (extend: `WaypointStatus::AwaitingInput`, `NodeOutcomeKind::Parleyed`, `fork_of`) | model | CRUD (persisted aggregate) | itself (existing `#[non_exhaustive]` + `#[serde(default)]` precedents) | exact |
| `crates/paladin-battalion/.../superstep.rs` (extend: Parley suspension arm, grace-deadline race, Gate dispatch) | service (orchestration engine) | event-driven | itself (existing dispatch/join loop, existing child-`AwaitingInput` arm) | exact |
| `crates/paladin-battalion/.../mod.rs` (extend: `resume_with`, `replay`, `fork`, `with_shutdown_grace`) | service (facade over engine) | request-response | itself (existing `resume`/`resume_with_options`/`start`) | exact |
| `crates/paladin-battalion/.../graph.rs` (extend: `NodeSpec::Gate`, `validate()`, `fingerprint() v4`) | model + validation | transform | itself (`NodeSpec::Paladin`/`Battalion` variants, `fingerprint()`'s existing sections) | exact |
| `crates/paladin-battalion/.../node.rs` (extend: `NodeContext.parley_response`) | model | transform | itself (`NodeContext.muster` field, same shape) | exact |
| `crates/paladin-battalion/.../input_mapping.rs` (extend: `parley.` namespace) | utility (template resolver) | transform | itself (`muster.` namespace resolution, lines ~143-202) | exact |
| `crates/paladin-battalion/.../directive_parser.rs` (extend: envelope `parley` key) | utility (parser) | transform | itself (existing `goto`/`muster`/`end` envelope handling) | exact |
| `crates/paladin-battalion/.../shutdown.rs` (NEW) | service (coordinator) | event-driven | `crates/paladin-battalion/.../hooks.rs` (`CancellationToken` usage) — role-match only, this is genuinely new logic | role-match |
| `crates/paladin-ports/src/input/parley_port.rs` (NEW) | port (input trait) | request-response | `crates/paladin-ports/src/output/paladin_executor_port.rs` (injection-only trait shape) | role-match |
| `crates/paladin-ports/.../waypoint_port.rs` (extend: `WaypointSummary.fork_of`) | port (output trait) | CRUD | itself | exact |
| `crates/paladin-storage/.../contract_tests.rs` (extend) | test | CRUD | itself (existing 30-case suite) | exact |
| `src/application/services/chronicle.rs` (NEW) | service (read facade) | request-response | `src/application/services/waypoint_retention.rs` (application-service-over-`WaypointPort` shape) | exact |
| `src/application/services/parley/` (NEW, `ParleyPort` facade adapter + `GraphRegistry`) | service (port adapter) | request-response | `src/application/services/waypoint_retention.rs` for structure; `agent_controller.rs`'s `enqueue_job`/`JobStore` for the background-spawn pattern | role-match |
| `src/config/engine.rs` (extend: `shutdown_grace_secs`, `graceful_shutdown`) | config | transform | itself (existing `EngineConfig` fields/`EnvOverridable`) | exact |
| `src/config/waypoint_store.rs` (NEW) | config | transform | `src/config/waypoint_retention.rs` (second X-09 config-struct template) | exact |
| `src/bin/paladin-server.rs` (extend: `shutdown_signal`) | config/bootstrap | event-driven | itself | exact |
| `src/config/setup/service_runner.rs` (extend: `wait_for_shutdown`) | service (process lifecycle) | event-driven | itself | exact |
| `crates/paladin-web/src/thread_controller.rs` (NEW) | controller | request-response | `crates/paladin-web/src/agent_controller.rs` | exact |
| `crates/paladin-web/src/openapi.rs` (extend) | config (spec builder) | transform | itself | exact |
| `tests/integration/e2e_approval_gate_test.rs` (NEW) | test (integration) | event-driven | `tests/integration/e2e_crash_resume_test.rs` | exact |
| `tests/integration/*_stress_test.rs` (NEW) | test (stress) | event-driven | none dedicated — pattern is X-05 (multi_thread flavor, exact counts, timeout guard); no existing stress test file found as a direct analog | no analog (see below) |
| `docs/src/user-guides/parley-and-chronicle.md` (NEW) | docs | — | `docs/src/user-guides/*.md` sibling pages (per `SUMMARY.md`) | role-match |

## Pattern Assignments

### `crates/paladin-core/src/platform/container/parley.rs` (model, transform)

**Analog:** `crates/paladin-core/src/platform/container/waypoint.rs` — the file being extended is
its own best analog; `ParleyId` should mirror `WaypointId` exactly.

**Existing stub to replace** (verified, `waypoint.rs:447-455`):
```rust
/// Stub type for a paused run's outstanding input request.
///
/// Fully defined by Doc 03 (parley/resume-with-payload); this phase only
/// lands the stub so `WaypointStatus::AwaitingInput` has somewhere to point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ParleyRequest {
    /// Free-form prompt describing what input is being awaited.
    pub prompt: String,
}
```

**Current `WaypointStatus::AwaitingInput`/`NodeOutcomeKind` shape to extend** (verified,
`waypoint.rs:401-479`):
```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum NodeOutcomeKind {
    Succeeded,
    Failed,
    Skipped { reason: String },   // D-19 reuses this EXACT variant, reason: "shutdown"
    Ended,
    // D-03 adds: Parleyed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WaypointStatus {
    Running,
    Completed,
    Failed { error: String, failed_node: NodeId },
    AwaitingInput { parley: ParleyRequest },   // D-02 replaces this field:
    // -> AwaitingInput { parleys: Vec<ParleyRequest>, responses: Vec<ParleyResponse> }
    Halted,
}
```

**`WaypointId` as the copy target for `ParleyId(Uuid)`** — grep `WaypointId::new()` at
`waypoint.rs:171` (`Uuid::now_v7()`); mirror that exact constructor + serde-transparent newtype
shape for `ParleyId`.

**Additive-field test pattern to replicate** (verified, `waypoint.rs:1191-1235`):
```rust
#[test]
fn waypoint_payload_without_checkpoint_ns_deserializes_as_none() {
    let waypoint = /* construct with checkpoint_ns: Some(...) */;
    let mut value = serde_json::to_value(&waypoint).unwrap();
    value.as_object_mut().unwrap().remove("checkpoint_ns"); // simulate a pre-field payload
    let restored: Waypoint = serde_json::from_value(value).unwrap();
    assert_eq!(restored.checkpoint_ns, None);
}
```
Apply identically for `fork_of: Option<WaypointId>` on `Waypoint` and `WaypointSummary`.

**Error handling pattern:** none in this file (pure value types); errors belong to `EngineError`/
`ParleyError` in `mod.rs`/`parley_port.rs`, `thiserror`, `#[non_exhaustive]`.

---

### `crates/paladin-battalion/src/engine/superstep.rs` (service, event-driven)

**Analog:** itself — extend the existing dispatch/join loop and the child-Battalion
`AwaitingInput` arm.

**The typed-failure arm to replace** (per RESEARCH.md, `superstep.rs:1321`, `ParleyNotSupported`)
and the child arm to replace with `ParleyInChildUnsupported` (`superstep.rs:475`, currently a
stringly `NodeError`).

**Core dispatch-loop pattern — sequential await, MUST be restructured, not naively wrapped**
(verified shape at `superstep.rs:1258-1263`, batch-race replacement per RESEARCH.md Pitfall 1 and
Code Examples):
```rust
// Current (sequential, no shared deadline):
for (entry, handle) in dispatch_entries.iter().zip(handles) {
    let (...) = handle.await...;
}

// Required shape (batch race against ONE shared deadline, FuturesUnordered + index):
let deadline = cancel_observed_at.map(|t| t + shutdown_grace);
let mut remaining: FuturesUnordered<_> = handles.into_iter().enumerate().collect();
let mut results = vec![None; dispatch_entries.len()];
loop {
    match deadline {
        Some(dl) => tokio::select! {
            biased;
            Some((i, res)) = remaining.next() => { results[i] = Some(res); }
            _ = tokio::time::sleep_until(dl.into()), if !remaining.is_empty() => {
                // abort every still-outstanding handle; record Skipped { reason: "shutdown" }
                break;
            }
        },
        None => match remaining.next().await {
            Some((i, res)) => results[i] = Some(res),
            None => break,
        },
    }
}
```

**Edge evaluation pattern — Gate treated like Paladin** (verified, `superstep.rs:2328-2334`):
```rust
let output = match graph.node(source) {
    Some(NodeSpec::Paladin { output_field, .. }) => battlefield
        .get::<String>(output_field)
        .ok()
        .flatten()
        .unwrap_or_default(),
    // D-06: add `| Some(NodeSpec::Gate { output_field, .. })` to this arm
    _ => serde_json::to_string(battlefield).unwrap_or_default(),
};
```

**Error handling pattern:** typed `EngineError` variants via `thiserror`, no stringly errors (X-06)
— see `mod.rs`'s existing `EngineError` enum for the convention to extend.

**The inverted-assertion test this phase's suspension test must produce** (verified,
`superstep.rs:3376-3406`):
```rust
next: NextStep::Parley(ParleyRequest { /* ... */ }),
// ...
error: EngineError::ParleyNotSupported { node },
// ...
assert!(
    waypoints.iter().all(|w| !matches!(w.status, WaypointStatus::AwaitingInput { .. })),
    "no AwaitingInput waypoint may be written for an unsupported Parley"
);
// New test asserts the OPPOSITE: exactly one AwaitingInput waypoint IS written.
```

---

### `crates/paladin-battalion/src/engine/mod.rs` (service, request-response)

**Analog:** itself — `resume`/`resume_with_options`/`start` (lines ~600-970) are the direct
pattern for `resume_with`/`replay`/`fork`.

**`RunOutcome` reshape target** (verified, `mod.rs:102-141`):
```rust
pub enum RunOutcome {
    Completed { final_state: Battlefield, waypoint: WaypointId },
    AwaitingInput { parley: ParleyRequest, waypoint: WaypointId },
    // D-02 reshapes to: AwaitingInput { parleys: Vec<ParleyRequest>, waypoint: WaypointId }
    Halted { waypoint: WaypointId },
    Failed { error: EngineError, waypoint: Option<WaypointId> },
}
```

**Pitfall 2 — the `Completed`-only short-circuit must gain an explicit `AwaitingInput` arm before
the generic fallthrough** (mod.rs:890-900 today special-cases only `Completed`; `Halted` is
harmless via fallthrough, `AwaitingInput` is NOT — add an explicit early-return
`EngineError::ThreadAwaitingInput { thread, parleys }` arm ahead of the generic vanguard-restore
path). This is the single highest-risk correctness detail for `resume` vs `resume_with`.

**Error handling pattern:** `EngineError` is `#[non_exhaustive]` (line ~146); new variants land
alongside `ThreadNotFound`/`GraphMismatch`/`ParleyNotSupported` (line ~269) — `thiserror`-derived,
never stringly.

---

### `crates/paladin-battalion/src/engine/graph.rs` (model + validation, transform)

**Analog:** itself — `NodeSpec::Paladin`/`Battalion` variants for `Gate`'s shape; existing
fingerprint sections for the new `;gates:` section.

**Fingerprint section pattern to copy exactly** (verified, `graph.rs:1172-1211`):
```rust
buf.extend_from_slice(b";directive_parsers:");
for id in &self.node_order {
    let Some(NodeSpec::Paladin { directive_parser, .. }) = self.nodes.get(id) else { continue };
    push_field(&mut buf, id.as_str().as_bytes());
    let parser_json = serde_json::to_string(directive_parser).unwrap_or_default();
    push_field(&mut buf, parser_json.as_bytes());
}
// D-09: append a ";gates:" section here, same shape, then bump
// GRAPH_FINGERPRINT_VERSION from "v3" to "v4" and re-pin the golden hex test.
```
`push_field` is length-prefixed (8-byte LE prefix), never delimiter-joined — this is the exact
mechanism that fixed the Phase 22.1 CR-01 collision class; copy it verbatim, do not invent a new
join strategy.

---

### `crates/paladin-battalion/src/engine/input_mapping.rs` (utility, transform)

**Analog:** itself — the `muster.` namespace resolver is the direct template for `parley.`.

**Pattern to mirror** (verified, `input_mapping.rs:143-202`):
```rust
fn resolve(placeholder: &str, state: &Battlefield, muster: Option<&MusterContext>)
    -> Result<String, InputMappingError> {
    if let Some(name) = placeholder.strip_prefix("muster.") {
        return Self::resolve_muster(name, placeholder, muster);
    }
    // ... falls through to Battlefield lookup only for non-namespaced placeholders
}
```
`render`'s signature grows a third parameter (`parley: Option<&ParleyResponse>`); grep
`InputMapping::render(` across the whole `paladin-battalion` crate (not just `superstep.rs`)
before starting — Pitfall 3 warns every call site must be individually audited, not blanket-passed
`None`.

---

### `crates/paladin-battalion/src/engine/shutdown.rs` (NEW — service, event-driven)

**Analog:** role-match only — `crates/paladin-battalion/src/engine/hooks.rs` (existing
`tokio_util::sync::CancellationToken` import/usage) for the token pattern; no direct sibling for
the counter+`Notify` coordinator shape since this is genuinely new logic (RESEARCH.md's Pitfall 1
territory). Follow D-21's locked contract exactly: root `CancellationToken`, in-flight counter,
`Notify`; `register()` → child token + RAII `RunGuard`; `cancel_and_wait(grace)` cancels root and
waits until idle or deadline.

**Do NOT introduce `tokio-graceful-shutdown` or any third-party crate** — RESEARCH.md's
Alternatives Considered table explicitly rejects this; `tokio-util` (`0.7`, already a
`paladin-battalion` dependency) and `tokio::sync::Notify` (already available via existing `tokio`
feature set) suffice.

---

### `crates/paladin-ports/src/input/parley_port.rs` (NEW — port, request-response)

**Analog:** `crates/paladin-ports/src/output/paladin_executor_port.rs` — the injection-only trait
shape `paladin-web` already consumes via `Option<Arc<dyn T>>` fields.

**Trait shape** (per D-25, mirrors existing port conventions — `async_trait`, `Send + Sync`):
```rust
#[async_trait]
pub trait ParleyPort: Send + Sync {
    async fn resume_with(
        &self,
        thread: &ThreadId,
        responses: Vec<ParleyResponse>,
    ) -> Result<ResumeAccepted, ParleyError>;
}
```
`ParleyError` mirrors `EngineError`'s D-10 validation variants plus `ThreadNotFound`/
`GraphNotRegistered` — `thiserror`, `#[non_exhaustive]`.

---

### `src/application/services/chronicle.rs` (NEW — service, request-response)

**Analog:** `src/application/services/waypoint_retention.rs` — application-service-over-
`Arc<dyn WaypointPort>` shape, no engine dependency.

**Imports pattern** (mirror `waypoint_retention.rs`'s existing header — `Arc<dyn WaypointPort>`
injected, `paladin_ports::output::waypoint_port::{WaypointPort, WaypointSummary}`).

**Auth pattern:** none at this layer — auth is enforced at the HTTP adapter (`thread_router`'s
`route_layer`), not the application service.

**Core pattern:** `history`/`inspect`/`latest_on_branch` are thin reads over `WaypointPort`,
reused verbatim by `paladin-web` through the same port (no `paladin-battalion` dependency, per
ADR-0031).

---

### `src/application/services/parley/` (NEW — service, request-response)

**Analog (structure):** `src/application/services/waypoint_retention.rs`.
**Analog (background-spawn + 202 pattern):** `crates/paladin-web/src/agent_controller.rs`'s
`enqueue_job` (verified lines 609-647 per RESEARCH.md):
```rust
pub async fn enqueue_job(/* ... */) -> Result<(StatusCode, JsonValue), ApiError> {
    // ... synchronous validation (404/400 here) ...
    let job_id = state.jobs.create();
    tokio::spawn(async move { /* ... run, record outcome in job store ... */ });
    Ok((StatusCode::ACCEPTED, ok_body(&json!({ "job_id": job_id }))))
}
```
The facade `ParleyPort` adapter follows the identical shape: validate synchronously via
`WarEngine::resume_with`'s validation path (D-10), then `tokio::spawn` the continuation registered
with `ShutdownCoordinator::register()`, return immediately.

---

### `src/config/engine.rs` (config, transform)

**Analog:** itself — the file being extended is its own template.

**Pattern to extend** (verified, `engine.rs:40-141`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    pub max_supersteps: u64,
    pub max_node_visits: u32,
    pub run_timeout_secs: Option<u64>,
    #[serde(with = "waypoint_durability_serde")]
    pub waypoint_durability: WaypointDurability,
    pub max_muster_tasks: u32,
    // D-20 additions:
    // pub shutdown_grace_secs: u64,   // default 30, env APP_ENGINE_SHUTDOWN_GRACE_SECS
    // pub graceful_shutdown: bool,    // default true,  env APP_ENGINE_GRACEFUL_SHUTDOWN
}
```
`EnvOverridable::apply_env_overrides` gains two more `if let Some(v) = read_env::<T>(...)` blocks
(follow the four existing ones verbatim, `engine.rs:117-140`); `impl Default` gains the two new
fields at their documented defaults.

**Pitfall 4 (hard constraint):** do NOT add `shutdown_grace_secs`/`graceful_shutdown` to
`impl From<EngineConfig> for EngineLimits` (`engine.rs:143-169`) — this would make shutdown grace
graph-fingerprint-relevant, contradicting D-20. The existing test
`default_engine_config_matches_todays_engine_defaults` (`engine.rs:227-234`) must keep passing
unchanged — a diff touching `impl From<EngineConfig> for EngineLimits` alongside the new fields is
a signal to re-check this boundary.

---

### `src/config/waypoint_store.rs` (NEW — config, transform)

**Analog:** `src/config/waypoint_retention.rs` — the second X-09 config-struct template
(`Default` + `validate()` + `EnvOverridable`, never touching `Settings`).

---

### `crates/paladin-web/src/thread_controller.rs` (NEW — controller, request-response)

**Analog:** `crates/paladin-web/src/agent_controller.rs` — the direct, near-total template for
state shape, router composition, auth layering, and the `202` pattern.

**State shape to mirror** (verified, `agent_controller.rs:58-69`):
```rust
#[derive(Clone)]
pub struct AgentApiState {
    pub registry: Arc<AgentRegistry>,
    pub provisioner: Option<Arc<dyn AgentProvisioner>>,  // injection-only trait object
    pub timeouts: TimeoutPolicy,
    pub jobs: Arc<JobStore>,
    pub auth: crate::agent_auth::AgentAuthConfig,
}
// ThreadApiState mirrors this exactly:
// pub struct ThreadApiState {
//     pub waypoints: Option<Arc<dyn WaypointPort>>,
//     pub parley: Option<Arc<dyn ParleyPort>>,
//     pub auth: crate::agent_auth::AgentAuthConfig,
// }
```

**Router composition + auth layering pattern to mirror exactly** (verified,
`agent_controller.rs:700-745`):
```rust
pub fn agent_openapi_router(state: AgentApiState) -> OpenApiRouter {
    OpenApiRouter::new()
        .routes(routes!(list_agents, register_agent))
        .routes(routes!(describe_agent, deregister_agent))
        .routes(routes!(execute_agent))
        .routes(routes!(execute_agent_stream))
        .routes(routes!(enqueue_job))
        .routes(routes!(get_job))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            crate::agent_auth::require_authentication,
        ))
        .with_state(state)
}

pub const API_V1_PREFIX: &str = "/v1";

pub(crate) fn versioned_agent_parts(state: AgentApiState) -> (Router, utoipa::openapi::OpenApi) {
    OpenApiRouter::new()
        .nest(API_V1_PREFIX, agent_openapi_router(state))
        .split_for_parts()
}

pub fn agent_router(state: AgentApiState) -> Router {
    let (routes, _api) = versioned_agent_parts(state.clone());
    routes.merge(crate::health::health_routes(state))
}
```
`thread_router(state)` follows this shape 1:1: `thread_openapi_router` with
`routes!(get_thread_state, resume_thread, get_thread_history)`, same `route_layer` call (reusing
`crate::agent_auth::require_authentication` — D-24: "same auth middleware as `/v1/agents/*`"),
nested under the same `API_V1_PREFIX`, merged by `paladin-server` alongside `agent_router`'s
output (not inside it — `AgentApiState` stays untouched, D-24).

**202 background-job pattern** — see `parley/` section above (same `enqueue_job` source).

**Error handling / envelope pattern:** `crates/paladin-web/src/error.rs`'s `ApiError::{not_found,
conflict, bad_request, not_implemented}` and `ApiErrorBody` — D-25's status mapping (404/409/400/
501) maps directly onto these existing constructors; no new envelope shape needed.

**Test pattern:** `agent_controller.rs`'s `#[cfg(test)] mod tests` (verified starting line 747) —
`tower::util::oneshot`, an admin `Principal` extension for direct handler calls bypassing HTTP auth
plumbing in unit tests. `thread_controller.rs`'s own test module should mirror this structure.

---

## Shared Patterns

### `#[non_exhaustive]` + typed `thiserror` errors (X-06)
**Source:** `crates/paladin-battalion/src/engine/mod.rs`'s `EngineError` enum;
`crates/paladin-core/.../waypoint.rs`'s `WaypointStatus`/`NodeOutcomeKind`.
**Apply to:** every new enum this phase adds (`ParleyKind`, `OnExpire`, `ParleyError`, new
`EngineError` variants) — never a stringly `NodeError`/`format!`-built error for a case this
phase's decisions already named (D-04's `ParleyInChildUnsupported`, D-10's validation matrix).

### Additive `#[serde(default)]` field, no migration
**Source:** `waypoint.rs`'s `checkpoint_ns`/`visit_counts`/`frontier`/`muster_progress` precedent
(test pattern at lines 1191-1235).
**Apply to:** `Waypoint.fork_of`, `WaypointSummary.fork_of`, and the `AwaitingInput` payload
reshape (D-02) — write the strip-key-then-deserialize test for every new field, not just a
round-trip test.

### Config structs standalone under `src/config/`, never on `Settings`
**Source:** `src/config/engine.rs` (`EngineConfig`), `src/config/waypoint_retention.rs`
(`WaypointRetentionConfig`).
**Apply to:** `EngineConfig`'s two new fields, the new `WaypointStoreConfig` — `Default` +
`validate()` + `EnvOverridable` (`APP_*` env vars), composed by the binary, never a field on
`Settings` (`src/config/settings.rs:27`, all-pub, not `#[non_exhaustive]`, untouched this phase).

### Injection-only ports for `paladin-web`
**Source:** `AgentApiState.provisioner: Option<Arc<dyn AgentProvisioner>>`
(`agent_controller.rs:58-69`).
**Apply to:** `ThreadApiState.waypoints: Option<Arc<dyn WaypointPort>>`,
`ThreadApiState.parley: Option<Arc<dyn ParleyPort>>` — both `paladin-ports` trait objects,
constructed by `src/bin/paladin-server.rs`, never inside `paladin-web` (ADR-0031: no
`paladin-battalion` dependency in `paladin-web`'s default build).

### `202 Accepted` background-job pattern
**Source:** `agent_controller.rs:609-647` (`enqueue_job`).
**Apply to:** `POST /v1/threads/{id}/resume` — validate synchronously, `tokio::spawn` (registered
with `ShutdownCoordinator`), return `202 { thread_id, state_url }` immediately, never hold the
connection for the run's duration.

### Namespaced `InputMapping` placeholder resolved from `NodeContext`, never Battlefield
**Source:** `input_mapping.rs:143-202` (`muster.` resolution).
**Apply to:** the new `parley.` namespace (D-07) — `strip_prefix("parley.")` dispatch, graph
validation independently rejects a schema field with that prefix (mirrors the existing `muster.`
validation rule).

### Length-prefixed, sorted, version-bumped fingerprint sections
**Source:** `graph.rs:1172-1211` (`;directive_parsers:` section, `push_field`).
**Apply to:** the new `;gates:` section (D-09), `GRAPH_FINGERPRINT_VERSION` `"v3"` → `"v4"`,
golden-hex re-pin, one difference test per hashed property.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `tests/integration/*_stress_test.rs` (X-05, acceptance 7 — 10 concurrent resumes) | test | event-driven | No existing stress-test file in the tree to copy structurally; RESEARCH.md's Don't-Hand-Roll and Pitfall sections supply the shape (`flavor = "multi_thread"`, `FuturesUnordered`/`tokio::select!` pattern, exact-count assertions, timeout guard) but there is no sibling fixture — use `tests/integration/e2e_crash_resume_test.rs` only for the cross-process/SQLite-file technique, not for the concurrency-stress shape itself. |
| `crates/paladin-battalion/src/engine/shutdown.rs` (`ShutdownCoordinator`/`RunGuard`) | service | event-driven | Genuinely new primitive per RESEARCH.md ("the one genuine new primitive is the grace-deadline task race"); `hooks.rs`'s `CancellationToken` import is the closest existing usage but not a structural analog for the counter+`Notify` coordinator shape. Follow D-21's locked contract directly rather than an in-tree analog. |

## Metadata

**Analog search scope:** `crates/paladin-core/src/platform/container/`,
`crates/paladin-battalion/src/engine/`, `crates/paladin-ports/src/{input,output}/`,
`crates/paladin-storage/src/waypoint/`, `src/application/services/`, `src/config/`,
`crates/paladin-web/src/`, `tests/integration/` — all directly informed by RESEARCH.md's own
exhaustive direct-tree-read sourcing (Primary Sources list, 2026-09-04), cross-checked with two
direct reads in this pass (`waypoint.rs:395-495`, `agent_controller.rs:700-760`).
**Files scanned:** RESEARCH.md's Primary Sources (16 files, several read in full) plus 2
verification reads in this pass.
**Pattern extraction date:** 2026-09-04
