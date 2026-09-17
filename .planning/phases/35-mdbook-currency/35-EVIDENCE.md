# Phase 35 mdBook Currency — Evidence

## Measurement Header

- **Date:** 2026-09-17
- **HEAD SHA (this run):** `8d9f74b79f13ea910ae37b816ffb58a85c9b2793`
- **Tool versions vs `docs.yml` pins:**

| Tool | Installed | `docs.yml` pin | Match |
|---|---|---|---|
| `mdbook` | `mdbook v0.4.40` | `0.4.40` | yes |
| `mdbook-mermaid` | `mdbook-mermaid 0.13.0` | `0.13.0` | yes |
| `mdbook-linkcheck` | `mdbook-linkcheck 0.7.7` | `0.7.7` | yes |

## D-00b baseline re-run check

Per D-00b, `git diff --stat` against the Phase 34 baseline commit was run scoped to everything
outside `.planning/`, `docs/`, and `CHANGELOG.md`:

```
$ git diff --stat ee1fb160f8e743e638b32beb6c4e32be4ede9325..HEAD -- . ':!.planning' ':!docs' ':!CHANGELOG.md'
 Cargo.lock                                       |   1 +
 crates/doc-examples/Cargo.toml                   |   4 +
 crates/doc-examples/src/arsenal_tools.rs         |  97 ++++++++++++
 crates/doc-examples/src/battalion_patterns.rs    |  42 ++++++
 crates/doc-examples/src/herald_output.rs         |  57 ++++++++
 crates/doc-examples/src/lib.rs                   |   6 +
 crates/doc-examples/src/paladin_agents.rs        |  70 +++++++++
 crates/doc-examples/src/sanctum_vector_memory.rs |  89 ++++++++++++
 crates/doc-examples/src/superstep_engine.rs      | 178 +++++++++++++++++++++++
 9 files changed, 544 insertions(+)
```

**Disposition:** every changed path is under `crates/doc-examples/` (`publish = false`, additive
`pub mod` registrations only — D-26) plus the expected `Cargo.lock` line for the new
`paladin-memory` path dependency (35-02's Rule 3 auto-fix). No source under `src/` or any other
`crates/*` changed. This is exactly the anticipated Phase 35 shape, not an unrelated branch move —
so this run's measurements below are taken directly on the current tree (not re-derived from an
inherited Phase 34 baseline; they were always going to be, since Phase 35 never trusted the Phase
34 build-baseline for anything beyond tool-version confirmation).

## Gate sequence — `docs.yml`, run in order

### 1. `mdbook-mermaid install docs/`

```
[2026-09-17T14:32:41Z INFO  mdbook_mermaid] Reading configuration file docs/book.toml
[2026-09-17T14:32:41Z INFO  mdbook_mermaid] Writing additional files to project directory at docs/
[2026-09-17T14:32:41Z INFO  mdbook_mermaid] Files & configuration for mdbook-mermaid are installed. You can start using it in your book.
[2026-09-17T14:32:41Z INFO  mdbook_mermaid] Add a code block like:
    ```mermaid
    graph TD;
        A-->B;
        A-->C;
        B-->D;
        C-->D;
    ```
```

### 2. `git status --porcelain -- docs` (immediately after mermaid install)

```
 M docs/src/appendix/cli-setup-check.md
 M docs/src/appendix/cli-testing.md
 M docs/src/appendix/cli-usage.md
 M docs/src/appendix/minio-file-repository-setup.md
 M docs/src/appendix/redis-queue-adapter-setup.md
```

**Disposition:** the mermaid asset files (`docs/mermaid.min.js`, `docs/mermaid-init.js`) show no
drift — the five modified paths above are this plan's own Task 1 residual-defect fixes (the D-21
pre-flight MSRV/fabricated-CI-name follow-ups authorized by the orchestrator, see "Task 1
residual-defect fixes" below), made before this gate run, not mermaid-install output. No
`git checkout -- docs/` restoration was needed.

### 3. `mdbook build docs/`

Full output tail (linkcheck backend):

```
...
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "distributed-tracing" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "performance-baselines" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "benchmarking" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "llm-optimization" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "memory-optimization" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "concurrency-tuning" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "database-optimization" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "network-optimization" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "resource-allocation" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "diagnostic-tools" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "common-issues" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "performance-issues" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "configuration-issues" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "deployment-issues" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "integration-issues" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation] Not checking "getting-help" in the current file because fragment resolution isn't implemented
[2026-09-17T14:32:48Z WARN  linkcheck::validation::filesystem] Not checking that the "distributed-tracing" section exists in ".../docs/src/operations/monitoring.md" because fragment resolution isn't implemented
[2026-09-17T14:32:48Z INFO  mdbook_linkcheck] No broken links found
```

All `WARN` lines are the linkcheck backend's known "fragment resolution isn't implemented"
in-page anchor limitation (pre-existing, not a Phase 35 regression) — the terminal line is
**`No broken links found`**.

### 4. `./scripts/check-doc-examples.sh`

```
Compiling documentation examples (paladin-doc-examples crate)...
All included examples compile.

Checking README Quick Example matches crates/doc-examples/src/readme.rs ...
README Quick Example is in sync.

Checking doc code examples in .../docs/src ...

Results: 0 checked, 622 skipped, 0 failed
All doc code examples pass validation.
```

### 5. `./scripts/check-doc-config.sh`

```
Validating fenced YAML blocks in .../docs/src ...

Results: 151 YAML block(s) checked, 0 failed
```

### 6. `make api-surface`

```
Checking public API surface...
🔍 Checking API surface for changes...
Extracting public API surface using cargo-public-api...
✅ API surface extracted to /tmp/tmp.GDWFyYGzM0 (3959 items)
✅ API surface unchanged
```

Per D-26, `make api-surface` reporting **no change** is the expected, correct result: the
`doc-examples` crate is `publish = false` and has zero matches in
`.project/current-exports.txt`, so five new `doc-examples` modules and their `pub mod`
registrations in `crates/doc-examples/src/lib.rs` do not move the public surface any publishable
crate exposes.

## Task 1 residual-defect fixes (D-21 pre-flight follow-ups)

The orchestrator's pre-flight D-21 grep pass on the merged wave-1/wave-2 tree found five residual
defects outside any single wave plan's own MB-nn scope, explicitly authorized for this plan to fix
as Rule-1 deviations (follow-ups to MB-53, MB-50, MB-46, MB-44 and MB-45):

| File | Before | After | Follow-up to |
|---|---|---|---|
| `docs/src/appendix/redis-queue-adapter-setup.md:11` | "Rust 1.75 or later" | "Rust 1.88 or later" | MB-53 |
| `docs/src/appendix/minio-file-repository-setup.md:22` | "Rust 1.75 or later" | "Rust 1.88 or later" | MB-50 |
| `docs/src/appendix/cli-usage.md:227` | sample output `✓ Rust Toolchain: 1.75.0` | `✓ Rust Toolchain: 1.88.0` | MB-46 |
| `docs/src/appendix/cli-setup-check.md:58,72,218,258` | sample output `1.75.0` / `rustc 1.75.0 (82e1608df 2023-12-21)` | `1.88.0` / `rustc 1.88.0 (6b00bc388 2025-06-23)` | MB-44 |
| `docs/src/appendix/cli-testing.md:426-439` | fabricated `# .github/workflows/test.yml` CI/CD Integration sample (`actions-rs`-era `cargo insta test --check` block that does not exist in `ci.yml`) | pointer to `deployment/cicd.md`'s job table plus a verbatim captioned excerpt of the real `cli-tests` job (`.github/workflows/ci.yml`) | MB-45 |

All five are one-line-or-block corrections, not architectural changes. Verified by the exit-grep
re-run below (the D-21 MSRV-drift grep is now fully empty, and the D-21 fabricated-CI-name grep's
only remaining hits are real, existing filenames — see the allowlist table).

## D-21 exit greps

All seven checks, run on the tree after the Task 1 residual-defect fixes above.

### 1. Version pins — `grep -rnE '"0\.[5-9]\.[0-9]+"' docs/src`

```
(no output)
```
Empty.

### 2. MSRV drift — `grep -rnE '\b1\.(70|75|85)(\.[0-9]+)?\b' docs/src`

```
(no output)
```
Empty (the four Task 1 fixes above closed the only residual hits).

### 3. `paladin::paladin_ports::` double-nesting — `grep -rn 'paladin::paladin_ports::' docs/src`

```
(no output)
```
Empty (D-13, closed book-wide by plan 35-09).

### 4. Relocated LLM-adapter import — `grep -rn 'paladin::infrastructure::adapters::llm::' docs/src`

```
docs/src/contributing/contributing-providers.md:272:use paladin::infrastructure::adapters::llm::myprovider_adapter::*;
docs/src/contributing/contributing-providers.md:367:/// use paladin::infrastructure::adapters::llm::myprovider_adapter::*;
```
Not empty — both allowlisted (see table below).

### 5. Vocabulary — `grep -rniE '\bQuartermaster\b' docs/src`

```
(no output)
```
Empty (D-17; `commissary.md` fixed by plan 35-04's `24b0e5b0`, and `adr-index.md`'s residual
occurrence — created after that plan's own scope was frozen — was independently closed by the
orchestrator's cross-plan integration commit `64a44c51` after wave 2).

### 6. Fabricated CI names — `grep -rn 'test\.yml\|build-release' docs/src`

```
docs/src/contributing/testing-guide.md:96:    ├── config.test.yml
docs/src/appendix/redis-queue-adapter-setup.md:53:docker-compose -f docker/docker-compose.test.yml up --build test-runner
docs/src/appendix/integration-tests.md:133:docker compose -f docker/docker-compose.test.yml up -d redis-test minio-test minio-test-init
docs/src/appendix/integration-tests.md:146:docker compose -f docker/docker-compose.test.yml down -v
docs/src/appendix/integration-tests.md:224:(`test` stage) and runs tests inside the container using `docker/docker-compose.test.yml`.
docs/src/appendix/integration-tests.md:244:- `config.test.yml` (required by `test_load_from_file_regression`)
docs/src/appendix/minio-file-repository-setup.md:69:docker-compose -f docker/docker-compose.test.yml up --build test-runner
docs/src/appendix/minio-file-repository-setup.md:518:docker-compose -f docker/docker-compose.test.yml up --build test-runner
```
Not empty — all eight allowlisted (see table below). The one genuine fabrication in this pattern
class, `docs/src/appendix/cli-testing.md:431`'s invented `.github/workflows/test.yml`, was fixed
in the Task 1 residual-defect pass above and no longer matches.

### 7. Phase 31 D-29 `token_count`-beside-`TokenUsage` re-check (ten-page hit list)

| Page | `token_count` hits | `TokenUsage` hits | Disposition |
|---|---|---|---|
| `docs/src/getting-started/quickstart.md` | none | line 118 | clean |
| `docs/src/operations/observability.md` | none | lines 38, 43 | clean |
| `docs/src/user-guides/battalion-patterns.md` | none | lines 307-308 | clean |
| `docs/src/user-guides/output-formatting.md` | none | line 170 | clean |
| `docs/src/user-guides/agent-orchestrator-bridge.md` | none | line 111 | clean |
| `docs/src/appendix/conclave-pattern.md` | none | line 726 | clean |
| `docs/src/architecture/domain-model.md` | line 105: `pub token_count: Option<u32>,` (the corrected `GarrisonEntry` field) | line 35 (prose, vocabulary-rule context) | **clean — was offending at `34-AUDIT.md` time (`token_count: usize`); corrected by plan 35-04's `e2a2e81f` (MB-03) to the live `Option<u32>` shape** |
| `docs/src/user-guides/herald-output.md` | none | line 55 | clean |
| `docs/src/user-guides/memory-management.md` | many, all `GarrisonEntry.token_count` (fields, SQL column, builder calls) | none | clean — distinct Garrison field, not an ACCT-01…05 carrier (34-AUDIT.md's own disposition, unchanged) |
| `docs/src/user-guides/paladin-agents.md` | none | line 161 | clean |

All ten pages clean. `domain-model.md` is the one page that changed disposition since
`34-AUDIT.md` — plan 35-04's MB-03/MB-11 fix directly resolved this D-29 finding as a byproduct of
correcting the `GarrisonEntry` struct fence to its live field set.

### Allowlist table

| File:line | Matched text | Reason |
|---|---|---|
| `docs/src/contributing/contributing-providers.md:272` | `paladin::infrastructure::adapters::llm::myprovider_adapter::*` | Template/example page for third-party contributors; not one of `34-AUDIT.md` §5's 60 `MB-nn` rows. Recorded as a deferred observation in `deferred-items.md` (plan 35-01, D-27) for a future pass to correct. |
| `docs/src/contributing/contributing-providers.md:367` | same pattern, inside a doc-comment example | Same reason — same page, same out-of-§5-scope status. |
| `docs/src/contributing/testing-guide.md:96` | `config.test.yml` (ASCII directory-tree entry) | Real, existing fixture file at the repo root (`config.test.yml`); not a fabricated CI workflow name. The tree's claimed location (`tests/fixtures/config.test.yml`) is a separate, pre-existing minor inaccuracy recorded as a deferred observation (plan 35-06). |
| `docs/src/appendix/redis-queue-adapter-setup.md:53` | `docker/docker-compose.test.yml` | Real, existing compose file (confirmed on disk at `docker/docker-compose.test.yml`), not an invented CI artifact. |
| `docs/src/appendix/integration-tests.md:133` | `docker/docker-compose.test.yml` | Same real file, same page's Docker-based integration-test instructions. |
| `docs/src/appendix/integration-tests.md:146` | `docker/docker-compose.test.yml` | Same. |
| `docs/src/appendix/integration-tests.md:224` | `docker/docker-compose.test.yml` | Same. |
| `docs/src/appendix/integration-tests.md:244` | `config.test.yml` | Real fixture file, required by `test_load_from_file_regression` per the page's own citation. |
| `docs/src/appendix/minio-file-repository-setup.md:69` | `docker/docker-compose.test.yml` | Same real compose file. |
| `docs/src/appendix/minio-file-repository-setup.md:518` | `docker/docker-compose.test.yml` | Same. |

No allowlist row lacks a historical/real-file reason; none is a miss papered over as an exemption.

## Closure map self-check — `git log --oneline --grep 'MB-'`

```
$ for n in $(seq -w 1 60); do git log --oneline --grep "MB-$n" | wc -l | grep -q '^0$' && echo "MISSING: MB-$n"; done
(no output — ALL 60 PRESENT)
```

Every one of the sixty `MB-nn` identifiers in `34-AUDIT.md` §5 appears in at least one commit
subject on this branch, reproducing the closure map mechanically (D-00a).

## Consolidated closure table (D-23) — all sixty rows, `34-AUDIT.md` §5 order

| MB-nn | Page | Disposition | Commit | How the §5 finding was addressed |
|---|---|---|---|---|
| MB-30 | `user-guides/superstep-engine.md` | new | `7cca347d`, `25f0e600` | The missing Phase 22 superstep-engine page written end to end: `WarGraph`/`Battlefield` merge semantics, `Waypoint` checkpointing, the three `WaypointPort` backends, `EngineConfig`/`EngineLimits`, `WaypointRetentionService`, the graph-fingerprint scheme; nav-inserted before Control Flow; four compile-verified `doc-examples` anchors |
| MB-05 | `introduction.md` | corrected | `6fcd779e` | Nav index gained the 9 previously-missing links (superstep-engine, control-flow, parley-and-chronicle, fault-tolerance, agent-runtime, eval-harness, commissary, deployment-topologies, platform-api, observability) |
| MB-09 | `architecture/overview.md` | corrected | `ecde2121` | Five new subsections (WarEngine, Aegis, Commissary, Observability/TraceRecord, Platform API) added, each linking its guide |
| MB-11 | `architecture/domain-model.md` | corrected | `e2a2e81f` | `GarrisonEntry` struct corrected to the live 7-field shape (`Option<u32> token_count`, `ConversationRole`, `is_summary`); Battlefield/Waypoint/Aegis/TraceRecord added to Core Domain Entities |
| MB-22 | `user-guides/control-flow.md` | corrected | `778bc53d` | Deferred-engine-guide sentence replaced with a link to `superstep-engine.md` (D-10); `NextStep::Parley` prose (and its code-sample comment) rewritten to describe the mechanism as shipped since Phase 24; `APP_ENGINE_MAX_MUSTER_TASKS` named |
| MB-27 | `user-guides/paladin-agents.md` | corrected | `7207e894` | New `paladin_agents.rs` module (`build_agent`, `attach_garrison`); Garrison/Handoffs sections `{{#include}}` the compile-verified anchors; version pin `0.5.0`→`0.10.0` |
| MB-19 | `user-guides/arsenal-tools.md` | corrected | `2cc4bdd3` | New `arsenal_tools.rs` module (`custom_armament`, `handoffs`); Custom Armaments/Handoff Tool sections compile-verified |
| MB-24 | `user-guides/herald-output.md` | corrected | `83d44878` | New `herald_output.rs` module; full 7-method `Herald` trait (`format_paladin_result`, `format_battalion_result`, `format_stream_chunk`, `finalize_stream`, `format_error`, `name`, `mime_type`) documented and compile-verified |
| MB-20 | `user-guides/battalion-patterns.md` | corrected | `dc771569` | New `battalion_patterns.rs` module (`commander` anchor via `CommanderBuilder`, live single-argument `execute`); version pin `0.5.0`→`0.10.0` |
| MB-28 | `user-guides/sanctum-vector-memory.md` | corrected | `185cacf5` | New `sanctum_vector_memory.rs` module (`rag_retrieve`, `rag_format`); service name corrected to camelCase `RagRetrievalService`; RAG section extended to the full Phase 33 surface (`RagRetrievalResult`, `ShedItem`, `RagRetrievalError`, `retrieve_context_with_timeout`, `with_token_counter`, the omission marker) |
| MB-06 | `getting-started/installation.md` | corrected | `aa2e1883` | MSRV `1.85`→`1.88` (toolchain table, heading, verification line); every dependency pin `0.5.0`→`0.10.0`; Feature Flag Profiles table regenerated from the facade `Cargo.toml` `[features]` (5→28 flags across two tables) |
| MB-07 | `getting-started/quickstart.md` | corrected | `700c2588` | Three dependency pins `0.7.0`→`0.10.0` |
| MB-18 | `user-guides/agent-orchestrator-bridge.md` | corrected | `926fd5f1` | "current **v0.5.0**" → "current **v0.10.0**" |
| MB-21 | `user-guides/content-processing.md` | corrected | `d75783a5` | Header sentence and "not yet implemented in v0.5.0" line → v0.10.0 |
| MB-25 | `user-guides/maneuver-flow-dsl.md` | corrected | `ddd79c33` | Crate pin and closing Version line `0.8.0`→`0.10.0` |
| MB-26 | `user-guides/orchestration.md` | corrected | `3bdc862d` | "current **v0.8.0**" → "current **v0.10.0**" |
| MB-23 | `user-guides/fault-tolerance.md` | corrected | `d2156c0e` | Graph fingerprint version claim `v5`→`v6`, with a note on Phase 26's `output_schema` bump; links the engine guide's fingerprint section |
| MB-29 | `user-guides/tool-integration.md` | corrected | `35630a85` | Reachability note extended with the shipped `ToolCallProtocolMiddleware`/`FinishOnPlainAnswerMiddleware` pair and `reasoning_agent` preset, linked to `agent-runtime.md` |
| MB-04 | `introduction.md` | corrected | `6fcd779e` | Medieval Military Theme table cut from 12 to a labelled 8-term excerpt, spelled identically to `domain-model.md`'s table, linking it as the full list |
| MB-02 | `architecture/commissary.md` | corrected | `24b0e5b0` | Line 7 reworded off the pre-rename literal term; points at ADR-0049 for the rename rationale and rejected-name list |
| MB-03 | `architecture/domain-model.md` | corrected | `e2a2e81f` | `GarrisonEntry` fence replaced with the live 7-field struct, fenced `rust,ignore` with the source path noted |
| MB-08 | `architecture/overview.md` | corrected | `ecde2121` | Opening line and crate table corrected "nine focused crates" → "eleven library crates plus a facade"; `paladin-herald`/`paladin-eval` rows added |
| MB-10 | `architecture/hexagonal-design.md` | corrected | `aef92924` | `LlmPort` trait excerpt corrected to the single-`LlmRequest`-parameter `generate` signature; the directly adjacent `OpenAIAdapter` sample updated for internal consistency |
| MB-12 | `architecture/design-patterns.md` | corrected | `7add15fe` | Pattern-5 `PaladinExecutionService::new` sample's fourth parameter corrected from `Option<Arc<dyn Herald>>` to `Option<Arc<dyn ArsenalPort>>` |
| MB-13 | `architecture/crate-map.md`, `api-reference/crate-map.md` | corrected | `bc2a50d1` | Both pages corrected to eleven library crates plus the facade (`paladin-eval`, `paladin-herald` named); every version claim reads 0.10.0; `architecture/crate-map.md`'s `paladin-llm` feature table gains the six Phase 17 provider flags |
| MB-14 | `api-reference/crate-map.md` | corrected | `bc2a50d1` | `mem --> llm` mermaid edge added so diagram and prose agree (Phase 33 D-03) |
| MB-15 | `api-reference/feature-flags.md` | corrected | `10dba3e2` | Regenerated from the facade and per-crate `Cargo.toml` `[features]`: `otel`, `dev-ui`, `redis-cache`, `storage-postgres` added; storage aggregate corrected; Dockerfile excerpt fixed to `rust:1.93-slim-bookworm`; relocated LLM-adapter import fixed |
| MB-35 | `contributing/architecture-decisions.md`, `contributing/adr-index.md` | retitled | `745e8a64` | Nav entry for the existing adapter-development page retitled "Adapter Development Guide"; new `contributing/adr-index.md` page added directly after it, indexing nine consumer/operator-visible ADRs via GitHub blob URLs |
| MB-16 | `api-reference/migration-guide.md` | corrected | `cb824ee2` | Opening sentence and Timeline table now name v0.10.0 as the current release |
| MB-17 | `api-reference/stable-api.md` | corrected | `f2b25fd3` | Catalogue paths rerooted from pre-workspace `paladin::core::…` onto live `paladin_core::platform::container::…`/`paladin_ports::output::…` paths; Version/footer corrected to 0.10.0; Public crates list extended with `paladin-eval`/`paladin-herald` |
| MB-31 | `deployment/cicd.md` | corrected | `ea34a45c` | Fabricated 3-job `ci.yml` sample replaced with a 27-row real job table (job/display name/what it gates/required-or-advisory, sourced from `protect-main-branch.json`); fabricated `release.yml` sample replaced with a 9-row real job table; `codeql.yml` added, advisory-only disposition stated verbatim |
| MB-36 | `contributing/testing-guide.md` | corrected | `2bb7c833` | Fabricated `test.yml`/`actions-rs/toolchain` sample replaced with a pointer to `cicd.md`'s job table plus a verbatim `coverage` job excerpt; coverage command corrected to `scripts/coverage.sh`'s real `--features integration-tests,llm-all --fail-under-lines 82` invocation |
| MB-32 | `operations/monitoring.md` | corrected | `69f3336f` | Overview premise corrected: `opentelemetry` is now a real optional dependency behind `otel`; fabricated Jaeger sample replaced with the shipped `OtelTraceSink` section |
| MB-33 | `operations/performance-tuning.md` | corrected | `e5d16af8` | Benchmark Results callout names both real benchmark files (`config_benchmarks.rs`, `engine_benchmarks.rs`); links the new Superstep Engine guide |
| MB-34 | `operations/troubleshooting.md` | corrected | `8c31b557` | Same repeated tracing-dependency premise corrected; other confirmed-accurate conclusions kept |
| MB-37 | `appendix/battalion-benchmarks.md` | corrected | `9dbecc08` | Toolchain line corrected `1.85+`→`1.88+`, matching `Cargo.toml` `rust-version` |
| MB-38 | `appendix/battalion-patterns-guide.md` | corrected | `1b887f18` | All four opening `use paladin::battalion::*;` imports replaced with the compiling `paladin::core::platform::container::battalion::*;` path |
| MB-39 | `appendix/build-baselines.md` | archived | `2ac3cac0` | ADR-0047 banner naming it a dated Milestone 7 build-time snapshot, pointing at `appendix/performance-baseline.md`; crate-count table header reworded as the snapshot's own figure |
| MB-40 | `appendix/cli-configuration.md` | corrected | `9f0c4873` | Scheduler troubleshooting entry rewritten to point at the live `/v1/schedules*` route family (Phase 27) instead of a stale "no TODO at line 297" claim |
| MB-41 | `appendix/cli-council.md` | corrected | `48a7cc6f` | Command Syntax block replaced with a live `council --help` capture; every fabricated flag removed; Discussion Modes/Output Options rewritten around the real `--participants`/`--max-rounds` knobs |
| MB-42 | `appendix/cli-muster.md` | corrected | `a3794c62` | Command Syntax block replaced with a live `muster --help` capture; fabricated "Validation Phase" rewritten to the real accept/edit/cancel review prompt |
| MB-43 | `appendix/cli-onboarding.md` | corrected | `0c3d6ea1` | Live `onboarding --help` capture added; fabricated Environment Variables section (`PALADIN_ENV_FILE`, `PALADIN_SKIP_VALIDATION`) deleted outright |
| MB-44 | `appendix/cli-setup-check.md` | corrected | `520b8420` | Command Options block replaced with a live `setup-check --help` capture; fabricated `-v`/`-q`/`--json` flags removed; rebuild command corrected to `--features cli`; sample toolchain output corrected `1.75.0`→`1.88.0` (Task 1 residual fix, this plan) |
| MB-45 | `appendix/cli-testing.md` | corrected | `c7bcb9d5` | Tier 4 count corrected from claimed 12 to live-counted 13; fabricated `.github/workflows/test.yml` CI/CD Integration sample replaced with a pointer to `cicd.md`'s job table plus a verbatim `cli-tests` job excerpt (Task 1 residual fix, this plan) |
| MB-46 | `appendix/cli-usage.md` | corrected | `d1603402` | Build command corrected to `--features cli`; live top-level `paladin-cli --help` capture added; every invented short alias removed; battalion-run's fabricated `-i/--input` replaced with the real required `-t/--type`; sample toolchain output corrected `1.75.0`→`1.88.0` (Task 1 residual fix, this plan) |
| MB-47 | `appendix/contributing-legacy.md` | archived | `3af85fc0` | ADR-0047 banner pointing at `contributing/development-setup.md`; MSRV corrected to 1.88; `crates/` workspace line added; placeholder clone URL replaced with the real repository URL |
| MB-48 | `appendix/council.md` | corrected | `41d58304` | `CouncilExecutionService::new` corrected to its live 3-argument form (both call sites); `CouncilResult`/`TerminationCondition`/`CouncilConfig` field and variant shapes corrected throughout the page |
| MB-49 | `appendix/integration-tests.md` | corrected | `fe8386c6` | Main test files inventory rebuilt from `ls tests/integration/*.rs` — all 59 live test files (26 previously missing) now present with Crate Scope, Services Required and Feature Gate columns |
| MB-50 | `appendix/minio-file-repository-setup.md` | corrected | `f65f2db2` | All six `paladin::paladin_ports::` occurrences corrected to the bare `paladin_ports::output::file_storage_port` crate path; page fenced `rust,ignore` wholesale; scratch-compile proved; "Rust 1.75" corrected to "Rust 1.88" (Task 1 residual fix, this plan) |
| MB-51 | `appendix/port-trait-template.md` | corrected | `5b08ce79` | All four template placeholder imports corrected (two previously uncited); probe substitutes `file_storage_port` for the placeholder to prove the shape |
| MB-52 | `appendix/provider-expansion.md` | corrected | `b9596926` | Adapter/port imports corrected; provider-comparison table expanded to all nine shipped providers with measured `ProviderCapabilities` values; version footer corrected to 0.10.0; fabricated `OpenAILlmAdapter` three-arg constructor rewritten at all four call sites |
| MB-53 | `appendix/redis-queue-adapter-setup.md` | corrected | `626a6019` | Queue-port import corrected; a second, previously-uncited `QueueError` import pointing at a nonexistent module corrected to `paladin_ports::output::queue_port::QueueError`; "Rust 1.75" corrected to "Rust 1.88" (Task 1 residual fix, this plan) |
| MB-54 | `appendix/release-automation.md` | corrected | `189208f0` | Operational caveat corrected to name all three `publish-crates` dependencies (`test`, `create-release`, `check-release-consistency`) and what the third gate enforces |
| MB-55 | `appendix/sanctum-benchmarks.md` | corrected | `59720e17` | Qdrant adapter framing corrected from future/unimplemented to shipped (module + `qdrant` feature both exist), benchmark numbers recorded as not yet captured |
| MB-56 | `appendix/sanctum-migration.md` | corrected | `c6021fed` | All three `paladin::paladin_ports::` occurrences corrected (one previously uncited); the `output::{SanctumPort, EmbeddingPort}` glob split into two module-qualified imports |
| MB-57 | `appendix/security-scanning.md` | corrected | `5afcc542` | Snyk section rewritten to the evaluated-and-removed/zero-Rust-coverage disposition; new Known Gap: No Rust SAST section states CodeQL's advisory-only disposition; tracked-exceptions list expanded to all five `.cargo/audit.toml` advisories |
| MB-58 | `appendix/sentinel.md` | corrected | `d5855c4f`, `9bb7362a` | Three adapter imports corrected (module path + `OpenAIAdapter` casing); fabricated `OpenAiConfig{..Default::default()}` literals rewritten to the real 5-field `OpenAIConfig` shape; a sixth, previously-uncited `paladin_ports::input::document_port` import corrected in a follow-up commit |
| MB-01 | `appendix/doc-coverage-report.md` | archived | `59b3dcb1` | ADR-0047 banner naming ADR-0033 and Phase 36 as the live measure; the false "no warnings" claim and the 9-crate list reframed as the snapshot's own historical figures |
| MB-59 | `appendix/user-rest-api.md` | archived | `47ca479c` | ADR-0047 banner stating the `paladin user` CLI does not exist in the shipped binary while the user service/repository layers do; malformed raw-source tail fenced as inert text so the page renders |
| MB-60 | `appendix/user-system.md` | archived | `47ca479c` | Shared banner with MB-59; clap pin left untouched (audit confirmed it matches the manifest) |

**Reconciliation against `34-AUDIT.md` §5:** all sixty IDs are present above, each with exactly
one disposition and at least one commit. No ID's disposition disagrees with its plan SUMMARY's own
closure-table entry (MB-13/MB-14 share one commit as a tightly-coupled page pair per D-24; MB-30
and MB-58 each carry two commits — the tracer commit plus a same-plan follow-up — both cited).

## Summary

- All sixty `MB-nn` rows closed and reconciled against `34-AUDIT.md` §5.
- Full `docs.yml` gate sequence green on `8d9f74b79f13ea910ae37b816ffb58a85c9b2793` before this
  plan's own commits (Task 1 residual fixes still pending commit at capture time; re-verified in
  the final-run section plan 35-10's Task 3 appends below).
- `make api-surface` reports no change (D-26).
- All seven D-21 checks empty or fully allowlisted, with every allowlist row naming a historical or
  real-file reason.
- `git log --oneline --grep 'MB-'` reproduces the sixty-ID closure map mechanically (D-00a).
