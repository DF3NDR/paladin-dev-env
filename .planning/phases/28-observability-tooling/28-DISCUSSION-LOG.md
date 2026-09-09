# Phase 28: Observability & Tooling - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-08
**Phase:** 28-observability-tooling
**Mode:** `--auto` — every selection below is the recommended default chosen by Claude without a
human in the loop. No `AskUserQuestion` was issued. Auto-log lines follow the
`[auto] [Area] — Q → Selected` form required by `modes/auto.md`.
**Areas discussed:** Trace event model & seq authority, Sink non-interference & fan-out, Log &
OTel sinks, SSE collapse & trace persistence, Graph export & execution overlay, dev-ui inspector,
paladin-eval crate shape, Config docs & bookkeeping

**Prior context applied:** PROJECT.md, REQUIREMENTS.md (OBS-01…04), STATE.md, ROADMAP.md Phase 28
entry, 27-/26-/25-CONTEXT.md decision and deferred sections, `.project/v0.10.0/07-observability-
tooling.md` and `00-program-overview.md` §3/§6, codebase maps ARCHITECTURE.md + TESTING.md, and a
direct read of the trace seam, run-event bus, `WarGraphDoc`, CLI, mock adapter and E2E fixtures.
No SPEC.md, no `.continue-here.md`, no checkpoint, no plans, no todo matches ≥ 0.4, no
discuss:pre hooks.

---

## Trace event model & seq authority

| Option | Description | Selected |
|--------|-------------|----------|
| Move to `paladin-core`, re-export from `paladin-ports` | PRD 07 + ADR-0016; import paths preserved | ✓ |
| Leave in `paladin-ports` | Smaller diff; contradicts PRD "types only in core" and ADR-0016 | |

`[auto] Trace model — Q: "Where does the authoritative enum live?" → Selected: "Move to paladin-core, re-export from paladin-ports" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `TraceRecord` envelope wrapping `TraceEvent`, serde-flattened | One flat JSON object per line; header stamped once | ✓ |
| Header fields on every variant | Duplicated fields on twelve variants; every producer must fill them | |
| Rename enum to `TraceEventKind`, make `TraceEvent` a struct | Changes what the name means for every existing match | |

`[auto] Trace model — Q: "How do thread_id/seq/at attach?" → Selected: "TraceRecord envelope" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Engine dispatcher owns the per-run counter; producers get a `TraceEmitter` handle | Single seq authority; below-engine producers (fallback adapter, middleware) emit through the handle | ✓ |
| Shared `Arc<AtomicU64>` passed by value | Same guarantee, worse API | |
| Producer-local counters merged downstream | Breaks strictly-increasing per-run seq | |

`[auto] Trace model — Q: "Who assigns seq for below-engine producers?" → Selected: "Dispatcher + TraceEmitter handle" (recommended default)`

**Notes:** Producer sites for the four new variants were fixed (D-04) so research does not re-open
them; `MiddlewareEvent.action` is a closed enum per X-06.

---

## Sink non-interference & fan-out

| Option | Description | Selected |
|--------|-------------|----------|
| Keep `TraceSink` | Seven implementors and four CONTEXT files already use it; module is `trace_sink_port` | ✓ |
| Rename to `TraceSinkPort` per PRD text | Rename buys nothing; churn | |

`[auto] Non-interference — Q: "Rename TraceSink to TraceSinkPort?" → Selected: "Keep TraceSink" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `catch_unwind` in the dispatcher consumer AND per child in `CompositeSink` | One panicking child cannot starve siblings; consumer task survives | ✓ |
| Dispatcher only | A panicking composite child would abort the whole fan-out | |

`[auto] Non-interference — Q: "Where is panic isolation applied?" → Selected: "Both" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `CompositeSink` in `paladin-ports` beside the trait | Zero deps, ADR-0015 untouched, usable by any crate | ✓ |
| `CompositeSink` in facade `infrastructure/telemetry` | Forces every composer through the facade | |

`[auto] Non-interference — Q: "Where does CompositeSink live?" → Selected: "paladin-ports" (recommended default)`

**Notes:** `RunFinished` carries `trace_dropped_total` stamped at enqueue time; drop-oldest already
guarantees it is never the dropped event (D-07).

---

## Log & OTel sinks

| Option | Description | Selected |
|--------|-------------|----------|
| `log` crate, target `paladin::trace`, JSON line, default-on in facade composition | House stack (57 `log::` users, zero `tracing::`); engine stays sink-less | ✓ |
| `tracing` | First `tracing` consumer in the tree; a second logging stack | |
| Default-on inside `WarEngine` | Breaks Phase 22's zero-cost untraced path | |

`[auto] Sinks — Q: "Log sink on which stack and where is it default-on?" → Selected: "log crate, facade composition only" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| OTLP http/protobuf only; `InMemorySpanExporter` for tree shape + axum stub for transport | No tonic; deterministic shape test; hermetic transport test | ✓ |
| http + gRPC | Doubles the dependency graph for an unrequested transport | |
| Real collector container | Docker unavailable locally; adds nothing over stub | |

`[auto] Sinks — Q: "OTel transport and collector stub?" → Selected: "http/protobuf; in-memory exporter + axum stub" (recommended default)`

---

## SSE collapse & trace persistence

| Option | Description | Selected |
|--------|-------------|----------|
| Collapse to one producer; wire `seq` stays bus-owned; payload gains `trace_seq`; `RunStreamMode::Replay` | Published `seq` contract untouched; correlation added additively | ✓ |
| Adopt trace seq as the wire seq | Introduces gaps into a published dense field | |
| Keep two producers | Leaves 27-CONTEXT D-25's "single edit point" unclaimed | |

`[auto] SSE/persist — Q: "Collapse the two producers and how does seq map to the wire?" → Selected: "Collapse; bus seq + trace_seq" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| New `RunTracePort` + `in_memory/sqlite/postgres` adapters mirroring `waypoint/`; flush per superstep; prune with Waypoint retention | Separate lifetime/write pattern; one retention policy, two ports | ✓ |
| Extend `WaypointPort` with trace methods | Every Waypoint implementor carries trace storage | |

`[auto] SSE/persist — Q: "Persistence port shape?" → Selected: "RunTracePort" (recommended default)`

---

## Graph export & execution overlay

| Option | Description | Selected |
|--------|-------------|----------|
| `GraphShape` derivable from both `WarGraph` and `WarGraphDoc` | Renders Function nodes, worker templates and the three code-built E2E graphs; 27-CONTEXT D-33 scoped the doc to Paladin/Gate/Workflow | ✓ |
| `WarGraphDoc`-only exporter (PRD literal) | Cannot render the PRD's own muster fixture or any E2E graph | |
| Add a Function-node registry to `WarGraphDoc` | New capability deferred by Phase 27 | |

`[auto] Export — Q: "Exporter input type?" → Selected: "GraphShape" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Waypoint history baseline; persisted trace upgrades fired edges to exact | Works with `trace.persist` off (the default) | ✓ |
| Require persisted trace | Overlay unavailable on default installs | |

`[auto] Export — Q: "Overlay data source?" → Selected: "Waypoints baseline + trace upgrade" (recommended default)`

**Notes:** `flowchart TD`, sanitized ids, guillemet badges, dashed worker/deferred nodes;
`UPDATE_GOLDEN=1` bless idiom; `run export` falls back to observed-only mode with a labelled title
when no graph document is resolvable.

---

## dev-ui inspector

| Option | Description | Selected |
|--------|-------------|----------|
| New input port `RunInspectorPort`, facade implements it | ADR-0031; mirrors `RunEventStreamPort` (27-CONTEXT D-27) | ✓ |
| `paladin-web` depends on `paladin-battalion` behind `dev-ui` | Permitted by ADR-0031(iii) for a non-default feature, but leaks engine vocabulary into web and sets a precedent | |

`[auto] dev-ui — Q: "How does paladin-web get the diagram?" → Selected: "RunInspectorPort" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| CDN-configurable Mermaid URL, no vendoring; route outside `openapi.json`; admin-scoped; data embedded in page | Keeps crate small, keeps OpenAPI golden feature-independent, DOM-level smoke test on embedded payload | ✓ |
| Vendored Mermaid | Multi-MB asset in a published crate | |
| Page fetches `/history` + `/state` at runtime | Auth-header plumbing in JS; harder to smoke test | |

`[auto] dev-ui — Q: "Mermaid delivery, OpenAPI and data flow?" → Selected: "CDN-configurable, out of OpenAPI, embedded data" (recommended default)`

---

## paladin-eval crate shape

| Option | Description | Selected |
|--------|-------------|----------|
| Facade-tier composition crate (depends on core/ports/battalion/llm/storage), published, ADR-0047 records the classification | "Dev-dependency-oriented" needs publishing; `doc-examples` precedent for the dependency shape | ✓ |
| Depend on the facade `paladin-ai` | Dev-dep cycle with the facade's own dogfood tests | |
| Unpublished, in-tree only | Downstream teams cannot use it as a dev-dependency | |

`[auto] Eval — Q: "Crate dependency shape and publishing posture?" → Selected: "Composition crate, published, ADR" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `libtest-mimic` custom harness (`harness = false`), one runtime Trial per case | Filtering, `cargo test` output, no proc-macro crate | ✓ |
| Proc macro reading files at compile time | Second crate, fragile globbing, recompiles per scenario edit | |
| `macro_rules!` with explicit paths | One test per file, not per case — fails OBS-FR-13 | |

`[auto] Eval — Q: "Runner mechanism for one-test-per-case?" → Selected: "libtest-mimic" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Scenario target may name a registered graph constructor; E2E builders shared via `tests/helpers/e2e_fixtures.rs` | E2E graphs use Function nodes the doc cannot express; integration tests stay green (SHIP-03) | ✓ |
| Port E2E fixtures to `WarGraphDoc` | Impossible for Function nodes / worker templates | |

`[auto] Eval — Q: "How do E2E-1/2/3 become scenarios?" → Selected: "Registered constructors + shared builders" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `paladin-eval` owns `ScenarioLlm`; `MockLlmAdapter` untouched | Avoids an X-10 register event on a possibly pre-existing public type | ✓ |
| Extend `MockScriptEntry` with prompt matching | Public-surface change in `paladin-llm` for a harness-local need | |

`[auto] Eval — Q: "Prompt-substring matching: extend MockLlmAdapter?" → Selected: "No; ScenarioLlm in paladin-eval" (recommended default)`

**Notes:** `--repeat` exits non-zero on any divergence; `--bless` writes `<scenario>.<case>.snap.json`;
`--live` gated by `PALADIN_EVAL_LIVE=1` + ADR-0012 keys, structural assertions only by default.

---

## Config, docs & bookkeeping

| Option | Description | Selected |
|--------|-------------|----------|
| `src/config/trace.rs` `TraceConfig` + `OtelConfig`; `web_server.dev_ui.mermaid_url` | X-09 mirror of `RunStreamConfig`; typed `FeatureNotCompiled` error; redacted `Debug` on headers | ✓ |
| Fields on `EngineConfig` | Mixes engine limits with telemetry tunables | |

`[auto] Bookkeeping — Q: "Config home?" → Selected: "trace.rs" (recommended default)`

**Notes:** Bench (≤ 3 % overhead) and the X-05 stress test are deliverables; three mdBook pages;
`MIGRATION.md` §9.2–9.6 rows enumerated; gate order fixed including the `api-surface`
regeneration carried from Phases 25/26 and a `cargo tree -e features` default-build check.

---

## Claude's Discretion

File splits; whether `NodeContext` gains a trace accessor; heartbeat rate-limit mechanism; Mermaid
colours/badges; inspector layout beyond the four panels; `Trial` naming and `--repeat` tabulation;
`graph_doc` twins for the doc-expressible E2E scenario; `run export --format dot` for the static
shape.

## Deferred Ideas

LLM child spans; `FallbackHop.node_id` enrichment; live SSE attach in the inspector; DOT overlay;
`WarGraphDoc` Function-node registry; LLM-as-judge; `tracing` migration; values on the SSE wire;
per-`run_traces` retention tunables; vendored Mermaid; Phase 27 items (`27-SECURITY.md`,
WR-27-01, IN-27-01 — already closed; WINDOWS.md 31/32 still open); SHIP-01…04; carried review warnings. Reviewed-not-folded
todo: local coverage reproduction (no match ≥ 0.4).
