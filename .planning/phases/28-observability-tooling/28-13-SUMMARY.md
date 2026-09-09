---
phase: 28-observability-tooling
plan: 13
subsystem: infra
tags: [rust, cli, mermaid, dot, observability, clap]

# Dependency graph
requires:
  - phase: 28-observability-tooling (plan 07)
    provides: "GraphShape/ShapeNode/ShapeEdge/ShapeKind, to_mermaid, to_dot -- the pure exporters this plan's graph.rs calls in process"
  - phase: 28-observability-tooling (plan 10)
    provides: "ExecutionOverlay/OverlaySource/Visit, to_mermaid_overlay, GraphShape::observed -- the overlay layer this plan's run.rs calls in process"
  - phase: 28-observability-tooling (plan 12)
    provides: "eval.rs's clap-derive/CliError/OutputFormatter conventions this plan's graph.rs/run.rs mirror"
provides:
  - "src/application/cli/commands/graph.rs -- run_graph_export/render_graph_export: paladin-cli graph export --format mermaid|dot (<FILE> | --assistant <id>[@version]) [--out <path>], resolving --assistant through AssistantRepositoryPort over the RunStoreConfig-named store (ADR-0023)"
  - "src/application/cli/commands/run.rs -- run_run_export/render_run_export: paladin-cli run export --thread <id> [--waypoint <id>] [--run <run_id>] [--graph <FILE>] [--out <path>], implementing D-21's overlay-source preference (trace over Waypoints) and D-22's locked graph-resolution order"
  - "Graph/Run subcommand groups registered in src/bin/paladin-cli.rs"
  - "tests/cli/graph_export_test.rs, tests/cli/run_export_test.rs -- all 14 <behavior> clauses (7 + 7) against real Sqlite-backed stores and the 28-07 goldens"
affects: [28-14-run-inspector-service-and-view, 28-15-graph-inspector, 28-17-migration-doc]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "render_graph_export/render_run_export: a testable, port-injection-free core returning the rendered String/report, mirroring 28-12's run_eval/run_eval_report split -- the outer run_graph_export/run_run_export (the plan's own required exact signatures) only adds stdout/--out writing"
    - "Store construction lives in graph.rs/run.rs themselves (build_assistant_repository, build_run_repository, try_build_run_trace_store, build_waypoint_store), each Default + apply_env_overrides() + validate()'d from RunStoreConfig/WaypointStoreConfig -- the exact X-09 pattern paladin-server.rs and run_api_wiring.rs use, but scoped to only the repositories this plan's two commands need (no worker pool, no queue, no webhook service)"
    - "write_output uses print! (not println!) because to_mermaid/to_mermaid_overlay's own output already ends with a trailing newline -- println! would add a second one, breaking the 'prints the same string and nothing else' byte-exact contract the manual smoke test and graph_export_test.rs's golden comparisons depend on"
    - "WaypointId has no public parse-from-string constructor (core type, ADR-0016); --waypoint's clap value_parser round-trips through serde_json::from_value(Value::String(..)), mirroring paladin-web's thread_controller.rs::parse_waypoint_id precedent verbatim"

key-files:
  created:
    - src/application/cli/commands/graph.rs
    - src/application/cli/commands/run.rs
    - tests/cli/graph_export_test.rs
    - tests/cli/run_export_test.rs
  modified:
    - src/application/cli/commands/mod.rs
    - src/bin/paladin-cli.rs
    - tests/cli/mod.rs

key-decisions:
  - "render_graph_export/render_run_export are named distinctly from run_graph_export/run_run_export (not e.g. run_graph_export_report) so a plain grep -c 'pub async fn run_graph_export' file.rs (Task 1/2's own acceptance criteria) reads exactly 1 -- 28-12's eval.rs hit this same substring-match trap with run_eval/run_eval_report and documented it as an accepted deviation rather than avoiding it; this plan avoids it outright by choosing a non-overlapping verb."
  - "D-22's graph resolution is computed BEFORE the overlay is built (not after), because ExecutionOverlay::from_waypoints's fired-edge derivation needs a GraphShape up front to validate which completed->vanguard candidate pairs are genuine declared edges. When no real graph resolves (--graph absent, no run, or an Agent-kind assistant), an empty placeholder GraphShape is passed to from_waypoints, and GraphShape::observed(&overlay) is built AFTER from the overlay's own visits/edges -- see the 'Known Stubs' section below for the resulting narrow gap this produces for the Waypoints-source + no-real-graph combination."
  - "The overlay-source preference (trace over Waypoints) is decided by ACTUALLY READING trace rows and checking non-emptiness, not by checking whether a trace store is merely configured -- a configured-but-empty trace store (the common case before any run persists a trace) correctly falls through to Waypoint history rather than rendering nothing."
  - "run_run_export's stdout is annotated with the resolved overlay source and graph resolution (two lines) ahead of the diagram, printed via the SAME print!/--out path graph_export uses -- D-23's 'both write to stdout by default, pipe-friendly, no colour' holds for run_run_export's OWN diagram text (to_mermaid_overlay's output is still exactly what's on the later lines), while giving a human the source/resolution context the plan's action text explicitly asks for ('Print the resolved source and the resolution outcome alongside the diagram')."

requirements-completed: [OBS-03]

coverage:
  - id: D1
    description: "paladin-cli graph export renders a WarGraphDoc file to Mermaid or DOT on stdout, byte-matching the 28-07 goldens, with --out writing to a file plus a short confirmation, an unrecognised --format rejected by clap, and a missing/malformed document producing distinct errors"
    requirement: "OBS-03"
    verification:
      - kind: integration
        ref: "tests/cli/graph_export_test.rs#graph_export_file_to_mermaid"
        status: pass
      - kind: integration
        ref: "tests/cli/graph_export_test.rs#graph_export_file_to_dot"
        status: pass
      - kind: integration
        ref: "tests/cli/graph_export_test.rs#graph_export_to_out_file"
        status: pass
      - kind: unit
        ref: "tests/cli/graph_export_test.rs#graph_export_unknown_format_is_an_error"
        status: pass
      - kind: integration
        ref: "tests/cli/graph_export_test.rs#graph_export_unreadable_document_is_an_error"
        status: pass
    human_judgment: false
  - id: D2
    description: "paladin-cli graph export --assistant <id>[@version] resolves a stored Workflow assistant's graph document through AssistantRepositoryPort over a real SqliteAssistantRepository, rendering identically to the file-resolved path; an unknown assistant id is a distinct, naming error"
    requirement: "OBS-03"
    verification:
      - kind: integration
        ref: "tests/cli/graph_export_test.rs#graph_export_assistant_resolves_through_the_store"
        status: pass
    human_judgment: false
  - id: D3
    description: "paladin-cli run export prefers persisted trace rows (exact fired AND evaluated-but-not-fired edges, rendered dotted) over Waypoint history (derived fired edges only) when both sources are non-empty for a thread"
    requirement: "OBS-03"
    verification:
      - kind: integration
        ref: "tests/cli/run_export_test.rs#run_export_thread_with_waypoints"
        status: pass
      - kind: integration
        ref: "tests/cli/run_export_test.rs#run_export_prefers_trace_rows_when_present"
        status: pass
    human_judgment: false
  - id: D4
    description: "Graph resolution follows D-22's locked order: --graph file first, then the run's assistant version's document, then GraphShape::observed with the locked observed-only title when neither resolves; --run alone derives both the thread and the graph from the run row"
    requirement: "OBS-03"
    verification:
      - kind: integration
        ref: "tests/cli/run_export_test.rs#run_export_graph_resolution_order"
        status: pass
      - kind: integration
        ref: "tests/cli/run_export_test.rs#run_export_run_flag_resolves_thread_and_graph"
        status: pass
    human_judgment: false
  - id: D5
    description: "An unknown thread and a thread with no Waypoint history and no persisted trace each produce a clear, naming error rather than an empty diagram; --waypoint caps the overlay to that Waypoint's own history, excluding later visits"
    requirement: "OBS-03"
    verification:
      - kind: integration
        ref: "tests/cli/run_export_test.rs#run_export_unknown_thread_is_an_error"
        status: pass
      - kind: integration
        ref: "tests/cli/run_export_test.rs#run_export_thread_with_no_history_is_a_clear_message"
        status: pass
      - kind: integration
        ref: "tests/cli/run_export_test.rs#run_export_waypoint_flag_limits_history"
        status: pass
    human_judgment: false
  - id: D6
    description: "cargo fmt clean; cargo clippy clean with and without the cli feature; both commands reach storage through ports only (zero reqwest occurrences); the manual paladin-cli graph export smoke test byte-matches the linear.mermaid golden; cargo test --test cli_isolation passes without the cli feature"
    requirement: "OBS-03"
    verification:
      - kind: other
        ref: "cargo fmt --all --check (exit 0); cargo clippy -p paladin-ai --all-targets --features cli -- -D warnings (0 warnings); cargo clippy -p paladin-ai --all-targets -- -D warnings (0 warnings); cargo check --workspace --all-targets --all-features (exit 0); cargo test -p paladin-ai --features cli --test cli (130 passed); cargo test -p paladin-ai --test cli_isolation (9 passed); cargo run --features cli --bin paladin-cli -- graph export --format mermaid crates/paladin-battalion/tests/fixtures/graph_docs/linear.json | diff - crates/paladin-battalion/tests/golden/export/linear.mermaid (exit 0)"
        status: pass
    human_judgment: false

# Metrics
duration: ~95min
completed: 2026-09-09
status: complete
---

# Phase 28 Plan 13: CLI Graph/Run Export Summary

**`paladin-cli graph export --format mermaid|dot` and `paladin-cli run export` put the 28-07/28-10 exporters in a human's hands: a graph document or a stored assistant's `Workflow` renders on stdout in one command, and a thread's execution overlay renders with its resolved source (trace vs. Waypoints) and graph (explicit file, run-derived assistant version, or observed-only) annotated alongside it -- both reaching storage exclusively through ports over the same `RunStoreConfig`/`WaypointStoreConfig`-named stores the server uses (ADR-0023), never HTTP.**

## Performance

- **Duration:** ~95 min
- **Started:** 2026-09-09 (worktree base `483dd79a`)
- **Completed:** 2026-09-09T04:55:48Z
- **Tasks:** 2 (Task 1 tracer, Task 2 auto/tdd)
- **Files modified:** 7 distinct files across 2 commits (4 created, 3 modified)

## Accomplishments
- `src/application/cli/commands/graph.rs`: `run_graph_export`/`render_graph_export` resolve exactly one of `<FILE>` (JSON/YAML by extension, mirroring `paladin_eval::Scenario::from_path`'s dispatch) or `--assistant <id>[@version]` into a `WarGraphDoc`, render it with the existing `GraphShape::from_doc` + `to_mermaid`/`to_dot` (28-07), and write the result to stdout (no colour, no extra text) or `--out` with a short confirmation. `--assistant` resolves through `AssistantRepositoryPort`, built from `RunStoreConfig` exactly like `run_api_wiring.rs`'s `build_sqlite_quartet`/`build_postgres_quartet`, including the `storage-postgres` feature-gate fallback error.
- `src/application/cli/commands/run.rs`: `run_run_export`/`render_run_export` implement D-21's overlay-source preference (trace rows, when non-empty, over Waypoint history) and D-22's locked graph-resolution order (`--graph` file, then the run's assistant version's document, then `GraphShape::observed` with the locked `(observed nodes only — no graph document available)` title), printing a two-line source/resolution annotation ahead of the exact `to_mermaid_overlay` diagram. `--run <run_id>` derives both the thread and the graph from the run row; `--waypoint <id>` caps the rendered overlay to that Waypoint's own (older-or-equal) history.
- `Graph`/`Run` subcommand groups registered additively in `src/bin/paladin-cli.rs`, alongside 28-12's `Eval` group, with no changes to any pre-existing command.
- `tests/cli/graph_export_test.rs` (6 tests) and `tests/cli/run_export_test.rs` (7 tests): all 14 required `<behavior>` cases, exercised against REAL `Sqlite{Assistant,Run,RunTrace,Waypoint}` stores (never mocks) pointed at temp files through the SAME `APP_RUN_STORE_*`/`APP_WAYPOINT_STORE_*` env vars `paladin-server` reads, serialized under one shared `serial_test` key across both files since they mutate the identical process-global env vars.
- Fixed a byte-exactness bug caught only by the manual smoke test (not by `render_graph_export`'s own string-equality assertions, which never touch real stdout): `write_output` used `println!`, adding a second trailing newline on top of `to_mermaid`/`to_mermaid_overlay`'s own already-newline-terminated output. Switched to `print!` + explicit flush.

## Task Commits

Each task was committed atomically:

1. **Task 1 (tracer): `paladin-cli graph export`** - `0136f374` (feat)
2. **Task 2 (auto): `paladin-cli run export`** - `d535af92` (feat)

**Tracer feedback gate:** Task 1's own `<verify>` (`cargo test -p paladin-ai --features cli --test cli` -> `test result: ok`) was re-run after the commit and passed before Task 2 began; the manual byte-exactness smoke test was ALSO re-run after fixing the `println!`/`print!` deviation (found during Task 2's own verification pass, before the final commit) and passed.

**Plan metadata:** (this commit, following this SUMMARY)

## Files Created/Modified
- `src/application/cli/commands/graph.rs` - `ExportFormat`, `GraphCommands`, `GraphExportArgs`, `run_graph_export`, `render_graph_export`, `build_assistant_repository`, `load_graph_doc_file`/`write_output` (both `pub(crate)`, reused by `run.rs`)
- `src/application/cli/commands/run.rs` - `RunCommands`, `RunExportArgs`, `GraphResolution`, `RunExportReport`, `run_run_export`, `render_run_export`, and the four store builders (`build_run_repository`, `try_build_run_trace_store`, `build_waypoint_store`, reusing `graph.rs`'s `build_assistant_repository`)
- `src/application/cli/commands/mod.rs` - `pub mod graph;` / `pub mod run;`
- `src/bin/paladin-cli.rs` - `Graph`/`Run` subcommand groups and their dispatch arms
- `tests/cli/graph_export_test.rs` - 6 tests covering all 7 named `<behavior>` clauses (one, `graph_export_unknown_format_is_an_error`, is a pure clap-parse assertion needing no I/O)
- `tests/cli/run_export_test.rs` - 7 tests, one per named `<behavior>` clause
- `tests/cli/mod.rs` - `mod graph_export_test;` / `mod run_export_test;`
- `tests/cli/snapshots/cli__graph_export_test__*.snap`, `tests/cli/snapshots/cli__run_export_test__*.snap` - 4 committed `insta` snapshots

## Decisions Made
See `key-decisions` in the frontmatter. Most consequential: computing D-22's graph resolution BEFORE building the overlay (not after), because `ExecutionOverlay::from_waypoints`'s fired-edge derivation needs a `GraphShape` up front — this ordering is what makes the Waypoints-source-with-a-real-graph and Trace-source-with-no-graph combinations both correct, at the cost of a narrow, documented gap for the Waypoints-source-with-no-graph combination (see Known Stubs below).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `write_output` printed a second trailing newline, breaking byte-exact stdout output**
- **Found during:** Task 2, running the project execution rules' own manual smoke-test command (`paladin-cli graph export ... | diff - linear.mermaid`) as part of final verification
- **Issue:** `write_output`'s stdout branch used `println!("{rendered}")`. `to_mermaid`/`to_dot`/`to_mermaid_overlay` already terminate their own output with a trailing `\n` (confirmed against the 28-07 golden files, which each end with exactly one `\n`); `println!` appended a SECOND newline, so real CLI stdout output diverged from the golden by one blank trailing line. `render_graph_export`'s own test assertions (`rendered == expected`) never caught this because they compare the returned `String` directly, never actual process stdout.
- **Fix:** Changed to `print!("{rendered}")` followed by an explicit `std::io::stdout().flush()`, with the reasoning documented inline so a future reader does not "fix" it back to `println!`.
- **Files modified:** `src/application/cli/commands/graph.rs`
- **Verification:** `cargo run --features cli --bin paladin-cli -- graph export --format mermaid crates/paladin-battalion/tests/fixtures/graph_docs/linear.json | diff - crates/paladin-battalion/tests/golden/export/linear.mermaid` — exit 0 (previously exited 1 with a one-line diff).
- **Committed in:** `0136f374` (Task 1 commit — fixed before the commit landed, since `write_output` is a Task-1-owned function `run.rs` also calls)

---

**Total deviations:** 1 auto-fixed (1 bug). **Impact on plan:** Necessary for correctness — the plan's own `<behavior>` clause ("the command prints the same string `to_mermaid` produces ... and nothing else") is a byte-exactness requirement the pre-fix code violated. No scope creep; the fix stayed inside `write_output`, a function already in this plan's declared `files_modified` list.

## Issues Encountered
None beyond the one auto-fixed deviation above.

## Known Stubs

- **`render_run_export`'s Waypoints-source + no-real-graph combination derives NO fired edges (visits only).** When no real `WarGraphDoc` resolves (no `--graph`, no `--run`, or the run's assistant is `Agent`-kind) AND the overlay's source is Waypoint history (not persisted trace), `build_overlay` passes an EMPTY placeholder `GraphShape` to `ExecutionOverlay::from_waypoints`, because a real shape is not yet known at that point in the resolution order (see Decisions above). `from_waypoints`'s fired-edge derivation only counts a completed→vanguard candidate pair as fired when the shape declares that exact edge — against an empty shape, every candidate is rejected, so `overlay.fired_edges` stays empty and the resulting `GraphShape::observed` diagram shows visited nodes with NO edges between them, even though transitions genuinely happened. This is a narrow, silent-by-omission gap (not a crash, not a wrong answer, just an incomplete one) for exactly this one combination out of the 2×3 (source × resolution) matrix; none of this plan's 7 required `run export` `<behavior>` clauses exercise it directly (`run_export_thread_with_waypoints` is Waypoints-source but doesn't assert on edges; `run_export_graph_resolution_order`'s observed-only bucket uses `NodeOutcomeKind::Succeeded` visits with no edge assertions either). The Trace source has no such gap (its edges are read directly from `TraceEvent::EdgeEvaluated`, needing no shape at all). A future plan wanting exact Waypoints-derived edges in the fully-observed case would need `ExecutionOverlay::from_waypoints` to accept an "unconstrained" mode, or `28-14`'s `RunInspectorPort` (which shares this exact D-21/D-22 resolution logic per the plan's own scope note) to resolve this differently.
  - File: `src/application/cli/commands/run.rs`, `build_overlay` (~line 268) and `render_run_export`'s shape/overlay ordering (~line 178).
  - Reason: documented, deliberate ordering trade-off (see key-decisions); not fixed in this plan because doing so would require either reordering D-22's own resolution steps (against the plan's locked order) or a new `ExecutionOverlay` constructor variant, both out of this plan's declared scope.

## Threat Flags

None beyond the plan's own `<threat_model>` register (T-28-13-01..04), all of which are addressed as specified: zero `reqwest` occurrences in both files (T-28-13-01), `ExecutionOverlay`/`Visit` never carry a `Battlefield` field value (T-28-13-02, inherited unchanged from 28-10), `--out` writes with the operator's own shell authority (T-28-13-03, accepted per the threat model), and the source/resolution annotation lines address T-28-13-04 directly.

## User Setup Required
None - no external service configuration required. (`--assistant`/`--run` require a configured `RunStoreConfig`/`WaypointStoreConfig` backend to resolve at all, but that is the FEATURE's own documented prerequisite — both commands fail with a clear, actionable `CliError::configuration` naming the exact env var to set, not something this plan's completion depends on.)

## Next Phase Readiness
- 28-14's `RunInspectorPort` implementation (`src/application/services/run/inspector.rs`) shares the same D-21/D-22 resolution logic this plan implements in `run.rs` — per this plan's own scope boundary, that file and `crates/paladin-ports/src/input/*` were NOT touched here. A follow-up may unify the two D-21/D-22 implementations (this CLI's `build_overlay`/`resolve_real_graph_document` and 28-14's own equivalent) behind one shared helper, but nothing in this plan blocks 28-14 from proceeding independently.
- 28-15's graph inspector page can reuse `graph.rs`'s `load_graph_doc_file`/`build_assistant_repository` pattern (both `pub(crate)`, currently scoped to `commands/`) if a similar file/assistant resolution is needed there — promoting them to a shared, non-CLI-scoped location would be that plan's call.
- The Known Stubs gap above (Waypoints-source + no-real-graph fired edges) is a real, bounded limitation worth a WINDOWS.md entry, not a blocker for any currently-planned Phase 28 work.
- No blockers.

## Self-Check: PASSED

- FOUND: src/application/cli/commands/graph.rs
- FOUND: src/application/cli/commands/run.rs
- FOUND: tests/cli/graph_export_test.rs
- FOUND: tests/cli/run_export_test.rs
- FOUND: tests/cli/snapshots/cli__graph_export_test__graph_export_file_to_dot.snap
- FOUND: tests/cli/snapshots/cli__graph_export_test__graph_export_file_to_mermaid.snap
- FOUND: tests/cli/snapshots/cli__run_export_test__run_export_prefers_trace_rows_when_present.snap
- FOUND: tests/cli/snapshots/cli__run_export_test__run_export_thread_with_waypoints.snap
- FOUND commit: 0136f374
- FOUND commit: d535af92

## Verification Commands Run (all green)

- `cargo check --features cli --bin paladin-cli` -- exit 0
- `cargo check -p paladin-ai --features cli --test cli` -- exit 0
- `cargo clippy -p paladin-ai --all-targets --features cli -- -D warnings` -- exit 0, 0 warnings
- `cargo clippy -p paladin-ai --all-targets -- -D warnings` -- exit 0, 0 warnings (CLI code compiles out cleanly without `cli`)
- `cargo fmt --all --check` -- exit 0
- `cargo check --workspace --all-targets --all-features` -- exit 0 (~4m12s cold)
- `cargo test -p paladin-ai --features cli --test cli` -- 130 passed, 0 failed (includes both new files' 13 tests)
- `cargo test -p paladin-ai --test cli_isolation` -- 9 passed (asserts the `cli` feature is off)
- `cargo run --features cli --bin paladin-cli -- graph export --format mermaid crates/paladin-battalion/tests/fixtures/graph_docs/linear.json | diff - crates/paladin-battalion/tests/golden/export/linear.mermaid` -- exit 0, no diff

---
*Phase: 28-observability-tooling*
*Completed: 2026-09-09*
