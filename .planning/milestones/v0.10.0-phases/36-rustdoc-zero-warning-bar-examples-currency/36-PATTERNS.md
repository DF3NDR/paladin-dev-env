# Phase 36: Rustdoc Zero-Warning Bar & Examples Currency - Pattern Map

**Mapped:** 2026-09-17
**Files analyzed:** ~14 new example programs, 5 edited config/doc files, ~75 rustdoc-comment edit
locations across 8 crates (no new source files for the rustdoc-fix work — see note below)
**Analogs found:** all categories matched

**Note on scope:** Most of this phase's "files" are one-line rustdoc-comment edits inside ~75
existing locations, not new files — CONTEXT.md D-09 already names the technique per warning kind
(D-05..D-08) with worked examples in `36-RESEARCH.md`'s Architecture Patterns section, so this map
focuses its per-file table on the genuinely new artifacts: new `examples/*.rs` programs, the two
`http_service_host.rs` edits, the gate-wiring edits (`Makefile`, `.pre-commit-config.yaml`,
`ci.yml`, `scripts/check-all-examples.sh`), and `examples/README.md`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `examples/war_engine_configuration.rs` (WarEngine cluster, EX-62..66,80) | example binary | request-response (in-process graph run) | `crates/doc-examples/src/superstep_engine.rs` | exact |
| `examples/control_flow_dynamic_routing.rs` (EX-67..70) | example binary | event-driven (graph edge/node routing) | `crates/doc-examples/src/superstep_engine.rs` + `examples/maneuver_basic.rs` | role-match |
| `examples/human_in_the_loop_gate.rs` (EX-71..73) | example binary | event-driven (gate/resume/replay) | `examples/battalion_checkpoint_recovery.rs` | role-match |
| `examples/graceful_shutdown.rs` (EX-74..76) | example binary | event-driven | `examples/battalion_checkpoint_recovery.rs` | role-match |
| `examples/platform_api_client.rs` (EX-77..79,91..99,104) | example binary | request-response over HTTP (in-process axum + reqwest) | `examples/http_service_host.rs` | exact |
| `examples/node_result_cache.rs` (EX-81..82) | example binary | CRUD (cache read/write) | `examples/war_engine_memory_baseline.rs` | role-match |
| `examples/agent_runtime_middleware.rs` (EX-83..90) | example binary | request-response (middleware pipeline) | `examples/basic_paladin.rs` + `examples/paladin_with_rag.rs` | role-match |
| `examples/observability_tracing.rs` (EX-100..104,108) | example binary | event-driven (trace emission) | `examples/war_engine_memory_baseline.rs` | role-match |
| `examples/eval_scenarios_demo.rs` (EX-105..107) | example binary | batch (`eval_scenarios!` runner) | `examples/basic_paladin.rs` | partial-match (no existing eval example) |
| `examples/token_economy_commissary.rs` (EX-109..115) | example binary | CRUD (Commissary window accounting) | `examples/basic_paladin.rs` | role-match |
| `examples/paladin_with_rag.rs` (extended, EX-116..120) | example binary (edit) | request-response (RAG retrieval) | itself (existing file) | exact |
| `examples/http_service_host.rs` (EX-33 fix) | example binary (edit) | request-response (HTTP host) | `src/bin/paladin-server.rs` (router-merge order) | exact |
| `crates/doc-examples/src/http_service_host.rs` (EX-55 fix) | doc-example module (edit) | request-response (HTTP host) | `src/bin/paladin-server.rs` (router-merge order) | exact |
| `examples/README.md` (EX-01,121,122) | doc/config | — | itself (existing file, house section shape) | exact |
| `Makefile` (`doc-check`, `check-examples` targets) | config | — | existing `api-surface`/`lint` targets | exact |
| `.pre-commit-config.yaml` (`doc-check` hook) | config | — | existing `check-api-surface` hook block | exact |
| `.github/workflows/ci.yml` (lint job step + Example Muster invocations) | config | — | existing "Check documentation" step + existing 4-invocation block | exact |
| `scripts/check-all-examples.sh` (D-14 rewrite) | utility/script | batch | `.github/workflows/ci.yml`'s own 4-invocation Example Muster block | exact |
| `Cargo.toml` (`[[example]]` declarations for gated clusters) | config | — | existing `http_service_host`/`document_processing` `[[example]]` blocks | exact |
| ~75 rustdoc-comment edit sites across 8 crates (RD-nn) | doc-comment edit | — | `crates/paladin-battalion/src/engine/mod.rs:59-61` (working link) vs `:20` (failing link) | exact |

## Pattern Assignments

### New example binaries (all D-15 clusters)

**Analog:** `examples/basic_paladin.rs` (simplest shape) and `examples/http_service_host.rs`
(HTTP-serving shape)

**Header-comment pattern** (`examples/http_service_host.rs` lines 1-9):
```rust
//! Runnable example: boot the Paladin HTTP API in-process and call an agent.
//!
//! Hermetic — backed by [`MockLlmAdapter`], so it needs no network or provider keys and runs
//! with `cargo run --example http_service_host --features web-server`.
//!
//! It assembles the app exactly as the `paladin-server` binary does (agent router under
//! `/v1`, OpenAPI docs, cross-cutting layers, auth enabled with a sample key), serves it on an
//! ephemeral port, then drives it over real HTTP: lists agents, runs one buffered and one
//! streamed execution, and reads the OpenAPI title — printing each result.
```
Every new program's header comment must follow this shape per CONTEXT `<specifics>`: what it
demonstrates (naming the capability the README section names), the run command, whether it needs
a key/service (D-16, D-29).

**Imports pattern** (`examples/http_service_host.rs` lines 11-27):
```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use paladin::MockLlmAdapter;
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::infrastructure::web::openapi::{build_openapi, docs_router};
use paladin::infrastructure::web::{
    AgentApiState, AgentAuthConfig, AgentRegistry, HttpLayersConfig, Principal, agent_router,
    with_http_layers,
};
use paladin_core::platform::container::user::UserRole;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
```
Convention: `paladin::` facade re-exports first, then `paladin_core`/`paladin_ports` direct
crate paths, `std` at top. Simpler programs (`examples/basic_paladin.rs` lines 16-23) use a
shorter subset of the same ordering.

**Offline-first mock-LLM pattern** (research `36-RESEARCH.md` Architecture Patterns Pattern 2,
verified `src/lib.rs:192`):
```rust
use paladin::MockLlmAdapter;
let llm = std::sync::Arc::new(MockLlmAdapter::new(/* canned response(s) */));
```
`examples/basic_paladin.rs` lines 35-39 shows the builder variant with `.with_response(...)`.

**Error handling pattern** (`examples/http_service_host.rs`, `examples/basic_paladin.rs`): both
use `async fn main() -> Result<(), Box<dyn std::error::Error>>` with `#[tokio::main]` and `?`
propagation — no `unwrap()`/`expect()` in example bodies, matching `CLAUDE.md`'s library rule
extended informally to examples.

**Cyclic-graph / WarEngine seed pattern** (for `war_engine_configuration.rs` and
`control_flow_dynamic_routing.rs`): use `crates/doc-examples/src/superstep_engine.rs` as the seed
— it already builds a small cyclic `WarGraph`, sets `EngineLimits`, runs it, and reads Waypoint
history (per CONTEXT `<specifics>` and `36-RESEARCH.md` "Reusable Assets"). Read that file directly
when drafting these two programs; it is the closest in-tree engine-configuration example and is
short enough for a single non-overlapping read.

---

### `examples/platform_api_client.rs` (Platform API cluster, EX-77..79,91..99,104)

**Analog:** `examples/http_service_host.rs` (full file) + `src/bin/paladin-server.rs:228-235`
(router-merge order)

**Router-parity pattern — the exact fix for EX-33/EX-55 and the seed for this new program**
(`src/bin/paladin-server.rs:228-235`, verified in-tree):
```rust
let routes = agent_router(state.clone())
    .merge(thread_router(thread_state))
    .merge(run_router(run_handles.run_state));
let app = if let Some(spec) = openapi_spec {
    routes.merge(paladin::infrastructure::web::openapi::docs_router(spec))
} else {
    routes
};
```
`examples/http_service_host.rs` and `crates/doc-examples/src/http_service_host.rs` currently stop
at `agent_router(state).merge(docs_router(spec))` — the EX-33/EX-55 fix is to insert the
`thread_router`/`run_router` merges in the same order shown above, in both files.

**In-process serve + drive pattern**: `examples/http_service_host.rs`'s full body (binds an
ephemeral `tokio::net::TcpListener`, calls `axum::serve`, then drives the app with real HTTP calls
in the same process) is the exact shape `platform_api_client.rs` extends with the ordered sequence
CONTEXT `<specifics>` describes: submit run → stream → (cancel) → assistants → schedules, then a
separate small webhook-receiver handler.

**`[[example]]` declaration for the gated new target** (`Cargo.toml:453-456`, the closest
existing analog — same feature gate):
```toml
[[example]]
name = "http_service_host"
path = "examples/http_service_host.rs"
required-features = ["web-server"]
```
New entry follows this exact shape with `name = "platform_api_client"`.

**Webhook-receiver HMAC pattern (security-sensitive, D-29)** — do not hand-roll; read
`crates/paladin-core/src/platform/container/webhook.rs:1-40,175-190` for the shipped
`X-Paladin-Signature: sha256=<hex>` invariant (verify over the raw captured request body bytes,
never re-serialise, never log the secret) before writing this part of the example.

---

### `Makefile` — `doc-check` and `check-examples` targets (D-11, D-14)

**Analog:** existing `api-surface` and `lint` targets (`Makefile:328-331`, `:377-380`)

**Style to copy** (`Makefile:328-331`, `:377-380` — verified in-tree):
```makefile
.PHONY: lint
lint: ## Run linter
	@echo "$(CYAN)Running linter...$(NC)"
	@$(CARGO) clippy --workspace --all-targets --all-features -- -D warnings

.PHONY: api-surface
api-surface: ## Check the public API surface against the committed baseline
	@echo "$(CYAN)Checking public API surface...$(NC)"
	@./scripts/check-api-surface.sh .project/current-exports.txt
```

**Drafted `doc-check` target** (`36-RESEARCH.md` Code Examples, matches the above style exactly):
```makefile
.PHONY: doc-check
doc-check: ## Zero-warning rustdoc bar (default + all-features) plus doctests (ADR-0033, D-00a)
	@echo "$(CYAN)Checking documentation (default features, zero warnings)...$(NC)"
	@$(CARGO) doc --workspace --no-deps 2>&1 | tee /tmp/doc-output.txt
	@! grep -q "warning:" /tmp/doc-output.txt
	@echo "$(CYAN)Checking documentation (all features, -D warnings)...$(NC)"
	@RUSTDOCFLAGS="-D warnings" $(CARGO) doc --workspace --all-features --no-deps
	@echo "$(CYAN)Running documentation tests...$(NC)"
	@$(CARGO) test --workspace --doc
```

**`clean-code` dependency chain to extend** (`Makefile:415-416` region — current):
```makefile
.PHONY: clean-code
clean-code: fmt lint lint-shell check ## Format, lint (Rust + shell), and check code
```
becomes `clean-code: fmt lint lint-shell check doc-check` per D-11.

---

### `.pre-commit-config.yaml` — new `doc-check` pre-push hook (D-11)

**Analog:** the existing `check-api-surface` block (`.pre-commit-config.yaml:126-132`, verified
in-tree):
```yaml
      - id: check-api-surface
        name: check public API surface vs baseline
        entry: ./scripts/check-api-surface.sh .project/current-exports.txt
        language: system
        stages: [pre-push]
        pass_filenames: false
        files: ^(src|crates)/.*\.rs$|^Cargo\.toml$
```
New entry copies this exact shape (`id: doc-check`, `entry: make doc-check`, same `files:` filter,
same `stages: [pre-push]`), per `36-RESEARCH.md` Code Examples.

---

### `.github/workflows/ci.yml` — lint-job step (D-12) and Example Muster additions (D-17)

**Analog:** the existing "Check documentation" step (`ci.yml:62-63`, verified in-tree):
```yaml
      - name: Check documentation
        run: cargo doc --workspace --no-deps 2>&1 | tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt
```
New step inserted directly after it:
```yaml
      - name: Check documentation (all features, -D warnings)
        run: RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps
```

**Analog for each new gated example's CI invocation** — the existing 4-invocation Example Muster
block (`ci.yml:548-558`, verified in-tree, full text captured above in this map's context):
```yaml
      - name: Build examples (web-server — http_service_host)
        run: cargo build --example http_service_host --features "web-server" --offline
```
Every new `required-features` example (Platform API cluster on `web-server`, node-result-cache on
`redis-cache`, any `otel`-gated observability sub-target) needs its own line in this exact shape,
plus a matching line in the rewritten `scripts/check-all-examples.sh` (D-14) — the bulk
`cargo build --examples` step silently skips `required-features` targets, per the comment already
at `ci.yml:538-546`.

**Binary-count assertion to update** (`ci.yml:565-579`, verified in-tree — uses
`find examples -name '*.rs' | wc -l` as `EXPECTED`, so it self-corrects at runtime; only the
human-readable "examples/ holds 47 .rs files" comment text needs updating per Pitfall 4 in
`36-RESEARCH.md`).

---

### `scripts/check-all-examples.sh` rewrite (D-14)

**Analog:** the CI Example Muster block itself (`ci.yml:548-558`) is the source of truth to mirror
verbatim — do not invent a different check. The old script's `cargo check --example <name>
--all-features` per-file loop is the anti-pattern to remove (it silently satisfies every
`required-features` gate, defeating the point of the split — `36-RESEARCH.md` Pitfall 3).

---

### `examples/README.md` (EX-01, EX-121, EX-122)

**Analog:** the existing section shape, e.g. `### [basic_paladin.rs]` (`examples/README.md`
lines 51-73, verified in-tree):
```markdown
### [basic_paladin.rs](basic_paladin.rs)
**Demonstrates:** Creating and executing a simple Paladin agent

The most basic Paladin usage - create an agent with a system prompt and execute a query.

```bash
cargo run --example basic_paladin
```

**Key concepts:**
- PaladinBuilder fluent API
- System prompt configuration
- Simple execution
```
Every new program's section follows this exact shape (heading, **Demonstrates:**, one sentence,
fenced `cargo run --example … [--features …]` command, **Key concepts:** list) per D-20. D-21
forbids a **Code snippet:** block in *new* sections (existing ones, like this file's own
`response.content` / `response.token_usage.total_tokens` / `response.execution_time` snippet at
lines 65-72, are corrected in place to the real `PaladinResult` field names `output`,
`usage.total_tokens`, `execution_time_ms` — never deleted).

**TOC pattern** (`examples/README.md` lines 5-17, verified in-tree) — new TOC entries are added in
this same flat bullet-list-of-anchors style, one new `##`-level section per D-15 cluster plus the
five sections D-20 names (Vision, Document Processing, HTTP Service Host, RAG & Retrieval,
Commander Strategies).

**"Getting Started" currency fix (EX-01, D-22)** — line 23 currently reads `Rust 1.70 or later`;
correct to `Rust 1.88` per the workspace `rust-version`.

---

### Rustdoc-comment fixes (D-05..D-09, the 75 location groups / 143 rows)

**Analog:** `crates/paladin-battalion/src/engine/mod.rs` — contains both the working pattern
(line ~59-61) and the failing pattern (line ~20) in the same file, the clearest in-tree contrast
for training the fix technique.

**Working intra-doc link (path syntax)** — `crates/paladin-battalion/src/engine/mod.rs:59-61`:
```rust
/// a [`graph::WarGraph`] or [`graph_doc::WarGraphDoc`] as a Mermaid
```

**Failing bare-shorthand sibling-module link** — same file, ~line 20:
```rust
//! - [`bridges`] — `WarGraph::from_formation`/`from_phalanx`/`from_campaign`
```

**Fix (D-06/D-09, `mod@` disambiguator or explicit path)**:
```rust
//! - [`bridges`](mod@bridges) — `WarGraph::from_formation`/`from_phalanx`/`from_campaign`
// or:
//! - [`self::bridges`] — `WarGraph::from_formation`/`from_phalanx`/`from_campaign`
```
This exact pattern closes the `engine/mod.rs` "Submodules:" list cluster (~14 of the 143 rows).

**Private intra-doc link fix (D-05, 57 rows)** — replace the link with plain code font or reword
to the public entry point:
```rust
// before: [`Commander::analyze_and_select`]   (private method, unresolved/private-link warning)
// after:  `Commander::analyze_and_select`      (plain code font, no link)
```

**Redundant explicit link (D-08, 6 rows):**
```rust
// before: [`Foo`](Foo)
// after:  [`Foo`]
```

**Unclosed HTML tag (D-08, 4 rows):**
```rust
// before: Vec<T> or Arc<dyn Port> written bare in a doc comment
// after:  `Vec<T>` / `Arc<dyn Port>` — wrapped in backticks, never escaped with &lt;
```

All fix techniques above are fully specified by CONTEXT D-05..D-09; this map's job is only to
name the in-tree contrast pair (`engine/mod.rs`) that grounds the one technique CONTEXT left
partially open (bare sibling-module shorthand).

## Shared Patterns

### Offline-first `MockLlmAdapter`
**Source:** `paladin::MockLlmAdapter`, unconditional facade re-export (`src/lib.rs:192`,
root `Cargo.toml:104-115` confirms `mock` feature is in `[dependencies]`, not
`[dev-dependencies]`)
**Apply to:** every new example program in every D-15 cluster except EX-111 (which demonstrates
the mock adapter's usage shape plus a comment, since no live provider key runs in CI)

### `#[tokio::main] async fn main() -> Result<(), Box<dyn std::error::Error>>` + `?`
**Source:** `examples/http_service_host.rs`, `examples/basic_paladin.rs` (both files, full extent)
**Apply to:** every new example program — no `unwrap()`/`expect()` in example bodies

### House `[[example]]` + gated-CI-invocation + `check-all-examples.sh` triple
**Source:** `Cargo.toml:453-456`, `ci.yml:548-558`, and the rewritten `scripts/check-all-examples.sh`
(D-14)
**Apply to:** the Platform API client, node-result-cache, and any `otel`-gated example — every
gated target needs all three edits in the same commit or the local/CI checks disagree (Pitfall 3)

### Security: never log/print a credential, verify HMAC over raw bytes
**Source:** `crates/paladin-core/src/platform/container/webhook.rs:1-40,175-190`,
`.github/instructions/security.instructions.md`
**Apply to:** `platform_api_client.rs`'s webhook-receiver portion (EX-96/EX-97) and any example
reading a key from the environment (state it in the header comment per D-29, never print the value)

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `examples/eval_scenarios_demo.rs` (EX-105..107) | example binary | batch (`eval_scenarios!` macro runner) | No existing `examples/*.rs` program uses `paladin-eval`; nearest is `crates/paladin-eval/src/runner.rs:682`'s own macro definition and its own crate-internal tests, not a gallery example. Planner/executor should read `crates/paladin-eval/src/runner.rs` directly for the macro's calling convention when drafting this file; for EX-107 specifically, CONTEXT leaves "shell out to the CLI vs. README-only" as Claude's Discretion (Open Question 2 in `36-RESEARCH.md`). |

## Metadata

**Analog search scope:** `examples/`, `crates/doc-examples/src/`, `src/bin/paladin-server.rs`,
`Makefile`, `.pre-commit-config.yaml`, `.github/workflows/ci.yml`, `Cargo.toml`,
`crates/paladin-battalion/src/engine/mod.rs`, `crates/paladin-core/src/platform/container/webhook.rs`
**Files scanned:** ~20 read directly this session (in addition to the exhaustive command-level
re-measurement already captured in `36-RESEARCH.md`, which this map treats as authoritative for
the 75 rustdoc fix-site locations rather than re-deriving)
**Pattern extraction date:** 2026-09-17
