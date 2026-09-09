# Eval Harness

**Since:** v0.10.0 (Phase 28, PRD 07)

`paladin-eval` is a published, composition-tier crate (classification recorded in
`.planning/decisions/0048-paladin-eval-composition-crate.md`, ADR-0048 — not linked here as a
clickable URL, since it lives outside `docs/src` and mdBook's linkcheck runs in strict
`warning-policy = "error"` mode) for writing deterministic, scripted-LLM test scenarios against a
real `WarEngine` run, evaluating them with a purpose-built assertion library, and running them
either through `cargo test` or `paladin-cli eval run`.

## The scenario file format

A scenario is a `.eval.yaml` (JSON also accepted) file carrying `schema_version: "1"`, one
**target**, and one or more **cases**:

```yaml
schema_version: "1"
target:
  graph_doc: crates/paladin-battalion/tests/fixtures/graph_docs/approval_gate.json
store: in_memory
llm:
  writer:
    - text: "This looks great, ship it."
cases:
  - name: approve
    input:
      topic: "the quarterly report"
    parley_responses:
      - kind: approval
        value: true
    assertions:
      - kind: route_taken
        nodes: [writer, review]
      - kind: run_status
        status: completed
```

`target` is either `{ graph_doc: <path> }` — a `WarGraphDoc` compiled through a named,
host-registered `EngineRegistries` — or `{ registered: <name> }` — a Rust closure the host test
binary registered via `ScenarioRunner::register_graph`, for graphs that need Function nodes,
worker templates, or custom edge evaluators no document can express. `store` defaults to
`in_memory`; `sqlite_temp` is available for scenarios that need real persistence (crash/resume
cases). `llm` scripts a `ScenarioLlm` — global and/or per-node sequences of `text`, `tool_call`,
or `error` entries, plus prompt-substring `match` rules checked before the sequence, consumed one
entry per call. Every case may set `interrupt_after_superstep` (drops the engine after that
superstep, resumes over the same store — the simulated-crash technique the integration tests also
use) and `parley_responses` (answers a raised Parley in order).

### The JSON Schema

The scenario format's JSON Schema is `schemars`-derived from the Rust types, never hand-written,
so it can never drift from what the runner actually accepts. The generated schema is checked in
as a golden file at `docs/schemas/eval-scenario.schema.json` (repo-root-relative). Regenerate it
after changing any scenario type:

```bash
UPDATE_EVAL_SCHEMA=1 cargo test -p paladin-eval --test schema_golden
```

## The twelve assertions

Every assertion evaluates against exactly three inputs: the captured `TraceRecord` stream, the
final `Battlefield`, and the `RunOutcome` — proving the trace model is sufficient without reaching
into engine internals. Every failure renders an actionable message (expected vs. observed, the
relevant `seq` range, and — for `node_executed` — a full visit table).

| Assertion | Checks | Example |
|---|---|---|
| `final_state_field_equals` | a `Battlefield` field equals a value | `{ kind: final_state_field_equals, field: status, value: "done" }` |
| `final_state_field_matches` | a `Battlefield` field matches a regex (linear-time, never backtracking) | `{ kind: final_state_field_matches, field: summary, pattern: "^Report:" }` |
| `field_json_path_equals` | a JSONPath expression into a field's value equals a value | `{ kind: field_json_path_equals, field: results, path: "$[0].status", value: "ok" }` |
| `node_executed` | a node's attempt count meets an exact/min/max bound (a retry counts once per attempt) | `{ kind: node_executed, node: worker, times: { min: 3 } }` |
| `node_not_executed` | a node never ran | `{ kind: node_not_executed, node: fallback_handler }` |
| `edge_fired` | a specific edge fired at least once, distinguishing "evaluated but did not fire" from "never evaluated" | `{ kind: edge_fired, from: review, to: writer }` |
| `route_taken` | a node-id list is a subsequence (not necessarily contiguous) of the executed route | `{ kind: route_taken, nodes: [writer, review, writer] }` |
| `run_status` | the run's terminal status | `{ kind: run_status, status: completed }` |
| `total_tokens_max` | total tokens consumed does not exceed a bound | `{ kind: total_tokens_max, max: 5000 }` |
| `supersteps_max` | total supersteps executed does not exceed a bound | `{ kind: supersteps_max, max: 10 }` |
| `parley_raised` | a Parley of a given kind was raised by a given node | `{ kind: parley_raised, node: review, parley_kind: approval }` |
| `final_state_snapshot` | the final `Battlefield` matches a blessed snapshot file | `{ kind: final_state_snapshot }` |

`run_status`/`total_tokens_max`/`supersteps_max` fail explicitly — never silently default — when
the trace carries no `RunFinished` record at all. A `custom(fn)` assertion also exists, but only
in the Rust API: it has no `serde` representation and can never be written into a scenario file.

## Running scenarios

### `cargo test` (the `evals` harness)

Scenarios are discovered and run through a `libtest-mimic` custom test harness — no proc macro,
one runtime-discovered `Trial` per `(file, case)` pair:

```rust,ignore
// tests/evals.rs
paladin_eval::eval_scenarios!("evals/**/*.eval.yaml", |runner: &mut ScenarioRunner| {
    runner.register_graph("approval-gate", build_approval_gate_graph);
});
```

```bash
cargo test --test evals                    # every scenario
cargo test --test evals e2e-2::approve      # one case, by <file-stem>::<case>
```

### `paladin-cli eval run`

The same `ScenarioRunner` reachable from the CLI, behind the `cli` feature:

```bash
paladin-cli eval run "evals/*.eval.yaml"
```

reports one line per case plus a summary, exiting non-zero on any failure.

```bash
paladin-cli eval run "evals/*.eval.yaml" --repeat 20
```

runs every case 20 times and reports a pass rate, exiting non-zero on **any divergence across
repeats** — not merely a failure count. Nondeterminism under fully scripted mocks is treated as a
bug to surface, not averaged away; the diverging `seq` range is named in the output.

```bash
paladin-cli eval run "evals/*.eval.yaml" --bless
```

(re)writes each `final_state_snapshot` case's blessed file from its case's own final
`Battlefield` — the `UPDATE_*=1` env-var bless idiom's CLI cousin.

## Live mode: the promotion path

Every scenario above runs against `ScenarioLlm`, a scripted, deterministic `LlmPort`
implementation — no network call, no real provider, safe for every CI run. `--live` promotes a
scenario to a real provider:

```bash
PALADIN_EVAL_LIVE=1 paladin-cli eval run "evals/*.eval.yaml" --live
```

Live mode requires **all three** of `--live`, the `PALADIN_EVAL_LIVE` environment variable, and a
configured provider credential (the same live-API-key handling every live-API test in this
project follows) — missing any one refuses with a typed error naming which is missing, never
silently falling back to scripted mocks. Content-bearing assertions
(`final_state_field_equals`/`_matches`, `field_json_path_equals`, `final_state_snapshot`) are
**skipped** under live mode — a real provider's non-deterministic output cannot be asserted
byte-exact — unless the scenario opts in via `live: { allow_content_assertions: true }`;
structural assertions (`node_executed`, `route_taken`, `run_status`, and the rest) always run.

This is the intended promotion path: every scenario is scripted-mocked in CI by default, and
`--live` is reserved for a pre-release live smoke pass against a real provider, never entered by
accident in ordinary CI.
