# Phase 51: Adjustable Reasoning Effort - Pattern Map

**Mapped:** 2026-10-21
**Files analyzed:** 22 (new/modified groups)
**Analogs found:** 21 / 22 (line numbers read at branch head 7b77ffef; re-locate if drifted)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `crates/paladin-core/src/platform/container/reasoning.rs` (NEW: `ReasoningEffort`, `ReasoningFallback`) | model (value type) | transform | `TokenUsage` / `Cost` in paladin-core; `ResponseFormat` in `llm_port.rs` | role-match |
| `crates/paladin-core/src/platform/container/paladin.rs` (`PaladinData` +2 fields) | model | CRUD | same file, lines 165-198 (`#[serde(default)]` knobs) | exact |
| `crates/paladin-core/src/platform/container/trace.rs` (`NodeFinished` +fields) | model (event) | event-driven | `cost` field on `NodeFinished` (:248) | exact |
| `crates/paladin-ports/src/output/model_profile.rs` (`EffortSupport`, `resolve`, `intersect`) | model | transform | `ParamSupport::admits/intersect` (:81, :89), `ModelProfile::intersect` (:466) | exact |
| `crates/paladin-ports/src/output/llm_port.rs` (`LlmRequest` fields, `LlmError::UnsupportedParameter`) | model / port | request-response | `response_format` field + `with_response_format` (:1092, :1218); `LlmError::Refusal` (:551, :946) | exact |
| `crates/paladin-llm/src/reasoning.rs` (NEW: keys, warn-once, map helpers) | utility | transform | `crates/paladin-llm/src/sampling.rs` (`RequestNotes` :371, `OmissionLog`) | role-match |
| `crates/paladin-llm/src/sampling.rs` (`RequestNotes` +2 fields) | utility | transform | same file :371-400 | exact |
| `crates/paladin-llm/src/compat/engine.rs` (`ReasoningWire`, `CompatRequestParameters.reasoning`, `with_default_reasoning`, `build_request`) | service | request-response | `CompatRequestParameters` (:133), `with_default_max_tokens` (:532), `build_request` (:595) | exact |
| `compat/types.rs` (`CompatRequest` fields skip-if-None) | model | request-response | existing optional fields in same file | exact |
| `{anthropic,openai,deepseek,gemini}/adapter.rs` (private `map_reasoning`) | adapter | request-response | each adapter's own `build_request` + `RequestNotes` use | role-match |
| `{kimi,qwen,grok,ollama,openai_compatible}/adapter.rs` | adapter (preset) | request-response | `kimi/adapter.rs:321`, `grok/adapter.rs:298` literals; `all()` users `qwen:407`, `ollama:240`, `openai_compatible:654` | exact |
| `crates/paladin-llm/src/registry/mod.rs` (`config_knob_builders!`, `ModelOverride`) | config | CRUD | macro at :182, `@common` arm | exact |
| `crates/paladin-llm/src/registry/seeded.rs` (effort data, after probe) | config (data) | batch | per-vendor `SeededProfile { model, verified_on, build }` rows | exact |
| `crates/paladin-llm/src/fallback.rs` (D-08 pre-flight, metadata keys) | service (decorator) | request-response | `model_profile` intersect (:870), `SERVED_*_METADATA_KEY` (:176, :184), `from_steps` (:451) | role-match |
| `crates/paladin-llm/src/conformance.rs` (wire-body cases, Kimi regression) | test | request-response | `send_capturing` (:1057) users (:1214, :1275, :1733) | exact |
| `crates/paladin-llm/examples/live_vendor_smoke.rs` (`--effort` mode) | utility (harness) | request-response | `--probe` (:569), `--usage` (:588) modes | exact |
| `crates/paladin-battalion/src/llm_failure.rs`, `llm_decision.rs` | service | transform | `Refusal` arms (llm_failure.rs :117, :374) | exact |
| `crates/paladin-battalion/src/engine/cache_key.rs` (`push_paladin_config`) | utility | transform | same fn :239-250 | exact |
| `src/application/services/paladin/{paladin_builder,paladin_execution_service}.rs` | service / builder | request-response | `response_format` threading (:3136-3142); `PaladinBuilder` setters | exact |
| `src/config/agents.rs` (`AgentDefinition`), `src/infrastructure/web/agent_host.rs` | config / service | CRUD | `AgentDefinition.model/provider` (:326, :335); `name_temperature_key` (agent_host.rs :237) | exact |
| `docs/src/user-guides/reasoning-effort.md` + drift test | docs/test | batch | `registry::switching_page_tests` (registry/mod.rs :2145) | exact |
| `.planning/decisions/0062-*.md`, `0004` amendment, `PROMOTION.md`, MIGRATION 9.2, CHANGELOG | docs | n/a | ADR-0061; MIGRATION 9.2 rows | role-match |

## Pattern Assignments

### `LlmError::UnsupportedParameter` + `LlmRequest` fields (`llm_port.rs`)

**Analog:** `LlmError::Refusal` and `response_format`.

Variant shape (llm_port.rs ~:551; copy doc-test style, `#[non_exhaustive]` on variant, struct fields documented):
```rust
#[error("Refused by provider '{provider}' (model '{model}'): {detail}")]
#[non_exhaustive]
Refusal { provider: String, model: String, detail: String, billed: Option<Box<BilledUsage>> },
```
New: `#[error("Unsupported parameter {param}={value} ...")] #[non_exhaustive] UnsupportedParameter { param, value, provider, model }` (error text names the `llm.<provider>.models.<model-id>` override key, D-03).

Transience arm (:946; the match has no wildcard, so the new variant is a compile error until added):
```rust
LlmError::EmptyCompletion { .. } => Transience::Permanent,
LlmError::Refusal { .. } => Transience::Permanent,
```
Add `LlmError::UnsupportedParameter { .. } => Transience::Permanent,`.

Request field + builder (:1085-1093, :1218):
```rust
#[serde(default)]
pub response_format: Option<ResponseFormat>,
...
pub fn with_response_format(mut self, response_format: ResponseFormat) -> Self {
    self.response_format = Some(response_format);
    self
}
```
Copy for `reasoning: Option<ReasoningEffort>` / `reasoning_fallback: Option<ReasoningFallback>` + `with_reasoning` / `with_reasoning_fallback`, each with a doc test. Re-export core types on `llm_port`.

### `llm_failure.rs` / `llm_decision.rs`

**Analog:** `Refusal` arms. `typed_origin` (:112-122):
```rust
LlmError::Refusal { provider, .. } => (None, Some(provider.clone())),
LlmError::AllProvidersFailed { last, .. } => typed_origin(last),
_ => (None, None),
```
Add an explicit `UnsupportedParameter { provider, .. }` arm (wildcard is compiler-required; do not rely on it). Add label row in `llm_decision` where `Refusal` is labelled (see :374/:400 comments). Also add explicit arm in compat `classify_fetch_failure`.

### `model_profile.rs` (`EffortSupport`, `resolve`, `intersect`)

**Analog:** `ParamSupport::admits` (:81-87) and `intersect` (:89), `ModelProfile::intersect` (:466-485).
```rust
pub fn admits(&self, value: f32) -> bool {
    match *self { Self::Range(lo, hi) => value >= lo && value <= hi, Self::Fixed(f) => value == f, Self::NotSent => false }
}
```
```rust
reasoning: self.reasoning || other.reasoning,
json_object: self.json_object && other.json_object,   // AND semantics: copy for effort levels
json_schema: self.json_schema && other.json_schema,
verified_on: None,
```
Conventions: `#[non_exhaustive]` enum, fail-closed `Default` (`Unmeasured`), `with_*` setters on `ModelProfile`, doc test per public item, commutative+associative intersect with property-style tests (see existing `tests` mod). New `ModelProfile` field must be added to `ModelProfile::new()` and `intersect`. Do NOT derive "does not reason" from `ModelProfile.reasoning` (RESEARCH Pitfall 1).

### `PaladinData` (paladin.rs :165-198)

**Analog:** same struct.
```rust
#[serde(default)]
pub autonomous_planning: bool,
```
New fields use `#[serde(default, skip_serializing_if = "Option::is_none")]` so Citadel snapshots unset stay byte-identical. `PaladinData` is NOT `#[non_exhaustive]`: registered break (MIGRATION 9.2, semver allowlist, update `tests/consumer_surface_muster.rs:206` and ~20 in-tree literals).

### `TraceEvent::NodeFinished` (trace.rs :248)

**Analog:** `cost` field (`#[serde(default, skip_serializing_if = "Option::is_none")]`). Add requested/effective optional string fields the same way; about 70 occurrences across about 23 files need updating (or a new variant, RESEARCH OQ5; planner decides).

### Compat engine (`compat/engine.rs`)

**Analog:** same file. `CompatRequestParameters` (:133-160): no `Default`, every preset states every field; `all()` (:170-) is a convenience that must be reshaped so reasoning is stated explicitly:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompatRequestParameters { pub temperature: bool, pub max_tokens: bool, pub top_p: bool, ... }
pub const fn all() -> Self { Self { temperature: true, max_tokens: true, ... } }
```
Provider-default precedent (:441-445, :532):
```rust
default_max_tokens: Option<u32>,
pub(crate) fn with_default_max_tokens(mut self, max_tokens: Option<u32>) -> Self { self.default_max_tokens = max_tokens; self }
```
and in `build_request` (:606-610):
```rust
let max_tokens = params.max_tokens.or(self.default_max_tokens).filter(|_| self.config.request_parameters.max_tokens);
sampling::check_max_tokens(self.provider_name, &request.model, &profile, max_tokens)?;
```
Copy for `effective = request.reasoning.or(self.default_reasoning)`; call shared `EffortSupport::resolve` in `build_request` before any HTTP and return `UnsupportedParameter` (never `sampling::apply`/`OmissionLog` omit-and-log path). Presets: Kimi `kimi/adapter.rs:321` and Grok `grok/adapter.rs:298` write literals; Qwen `:407`, Ollama `:240`, OpenAI-compatible `:654` call `all()`.

### `RequestNotes` (sampling.rs :371-400)

```rust
pub(crate) struct RequestNotes { pub params_omitted: Option<String>, pub format_adjusted: Option<String>, pub unprofiled_model: Option<String> }
impl RequestNotes { pub(crate) fn into_metadata(self) -> HashMap<String, String> { ... if let Some(m) = self.unprofiled_model { metadata.insert(UNPROFILED_MODEL_METADATA_KEY.to_string(), m); } ... } }
```
Add `reasoning_requested` / `reasoning_effective` the same way; stamp only when effort was set (unset stays byte-identical, D-00i). Key constants beside `SERVED_BY_METADATA_KEY` style (`pub const ... = "paladin.reasoning_requested"`). Warn-once log: copy `OmissionLog`/`FormatLog` (bounded; target `paladin::reasoning`).

### `config_knob_builders!` (registry/mod.rs :182)

Add `with_reasoning_effort` / `with_reasoning_fallback` in the `@common` arm (all nine types), setting `self.reasoning_effort = Some(..)`:
```rust
#[must_use]
pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self { self.base_url = base_url.into(); self }
```
`with_reasoning_wire` only on `OpenAiCompatibleConfig` and `OllamaConfig` (not in `@common`; hand-written or a new macro arm). Update each `*Config`'s hand-written `Debug` (a forgotten field is invisible, not a compile error). Config layering: `LlmProviderConfig` field + `config::layers::Layered`, `max_tokens` is the template.

### `FallbackLlmAdapter` (fallback.rs)

`from_steps` (:451) stores `chain` and `models: Vec<Option<String>>`; `model_profile` (:870) already iterates per hop with the hop model:
```rust
.filter_map(|(provider, step_model)| provider.model_profile(step_model.as_deref().unwrap_or(model)))
.reduce(|combined, next| combined.intersect(&next))
```
D-08 pre-flight: same zip loop, calling `resolve` per hop before the first HTTP call; error names the hop's provider and model. Metadata keys style (:176, :184):
```rust
pub const SERVED_BY_METADATA_KEY: &str = "paladin.served_by";
```
Ensure effort keys survive `served_by` stamping (Pitfall 11).

### `PaladinExecutionService` request sites

Analog `paladin_execution_service.rs:3136-3142`:
```rust
if let Some(response_format) = cx.response_format.clone() {
    request = request.with_response_format(response_format);
}
```
Replicate at all three `LlmRequest::new` sites (:3140, :3351, :3978): request wins, else agent (`PaladinData`) value; unset stays `None`.

### `AgentDefinition` / `agent_host`

`src/config/agents.rs:321-335` (`model`, `provider` optional fields); not `#[non_exhaustive]` (11 literals, registered break). `agent_host.rs:237` `name_temperature_key(error, key)` names the YAML key in build-time refusals: copy for naming `agents[].reasoning_effort` / `llm.<provider>.reasoning_effort` (D-09).

### `cache_key.rs` (:239-250)

```rust
fn push_paladin_config(buf: &mut Vec<u8>, paladin: &Paladin) {
    let data = &paladin.data;
    push_field(buf, data.model.as_bytes());
    ...
}
```
Append effort/fallback bytes only when `Some` (zero bytes when unset keeps existing keys valid); test changed effort changes the key.

### `conformance.rs` wire-body tests

`send_capturing::<F>(request, F::success_body()).await` returns `(wire_body, result)` (:1057, used :1214, :1275, :1733). Copy for per-adapter: unset golden body (committed before any adapter edit), effort level body, refusal-before-HTTP (no request captured), and Kimi regression (direction fixed only after probe, RESEARCH Pitfall 2). Red-first: land inert types, then `#[ignore = "fixed by 51-NN"]` tests plus `51-RED-EVIDENCE.md` (Phase 50 plan-01 pattern; Pitfall 14).

### `live_vendor_smoke.rs`

Third mode beside `--probe` (:569) and `--usage` (:588); Kimi already special-cased with `thinking` disabled. Output numbers only, scrub keys, `PROBE_TIMEOUT_SECS=120`, small output cap. Evidence file `51-EFFORT-PROBE.md` copies `50.1-SEED-PROBE.md` header; candidate list per `50.1-PROBE-CANDIDATES.md`.

### Docs drift test

`registry/mod.rs:2145 mod switching_page_tests` parses the mdBook page against seeded data; mirror for `reasoning-effort.md`.

## Shared Patterns

### One admission rule
**Source:** `ParamSupport::admits` pattern, `model_profile.rs`. **Apply to:** compat engine, 4 native adapters, `FallbackLlmAdapter` pre-flight, `PaladinBuilder::build`, `agent_host`, docs drift test. Single `EffortSupport::resolve(requested, fallback) -> Result<Resolved, EffortRefusal>`; wire folded into the returned `ModelProfile` (no new `LlmPort` method; decorators already forward `model_profile`).

### Layering
Types needed by `PaladinData` live in `paladin-core` (core cannot import ports); re-export on `llm_port`. Dependencies flow inward only.

### Error redaction
`crates/paladin-llm/src/redaction.rs` (`diagnostic_excerpt`: redact then bound). `UnsupportedParameter` is built pre-HTTP and carries no body; vendor 400s still go through the existing mapper.

### Serde spelling
`XHigh` must serialise as `xhigh` (explicit `#[serde(rename = "xhigh")]`); pin all nine spellings in a round-trip test; give `ReasoningEffort` a `Display`.

### Registered breaks
Non-`#[non_exhaustive]` touched types (`PaladinData`, `AgentDefinition`, `CompatRequestParameters`, `NodeFinished`): MIGRATION 9.2 row, `.cargo/semver-checks-allowlist.toml`, `tests/consumer_surface_muster.rs`, `make api-surface-update`, CHANGELOG.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `ReasoningWire` enum and per-vendor `map_reasoning` bodies | adapter | transform | No existing per-model wire-form switch; use RESEARCH vendor tables, gated by the live effort probe (D-04) |

## Metadata

**Analog search scope:** `crates/paladin-{core,ports,llm,battalion}`, `src/application`, `src/config`, `src/infrastructure/web`, `docs/src`
**Files scanned:** about 14 targeted reads and greps
**Caveat:** RESEARCH.md lines 478-705 were not read (output truncated); open questions there (OQ 5 trace shape, OQ 6 per-model default) are unaddressed here.
**Pattern extraction date:** 2026-10-21
