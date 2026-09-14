# Phase 28 Observability & Tooling — CI Evidence Record (plan 28-17)

**Phase:** 28-observability-tooling
**Branch:** `feature/phase-26`
**Base SHA at dispatch:** `08dc002b77c7c6f5f6ef6aa6882d9fc3b7ec3c73` — pushed by the orchestrator at
2026-09-09T05:44Z to start CI ahead of this plan's own work. This run **predates every commit
this plan (28-17) makes** — it proves the state of the sixteen implementation plans (28-01…28-16)
merged, not this plan's own docs/bookkeeping/registration work. The orchestrator will append a
second evidence block for the post-merge run on the wave's final SHA once this plan's commits are
folded in.
**Written:** 2026-09-09

This record has three parts: a **Local sweep** (everything this devcontainer can prove without
Docker — all run and recorded below, all green), a **Pre-close-out CI evidence** table for the
four workflow runs at `08dc002b` (mixed: several genuine failures, three already fixed upstream
by the orchestrator with commits not present in this worktree's base, one fixed by this plan's own
Task 3 work), and a note on what remains for the post-merge run to supply.

---

## Local sweep

Every command below was run in this worktree, against this plan's own Task 1/2/3 commits on top
of the `08dc002b` base — no `.rs` file was modified by Tasks 1/2 (`git diff --stat 08dc002b HEAD
-- '*.rs'` is empty), so every Rust-level result below is unchanged from what shipped in the
sixteen prior plans, re-verified here as this plan's own close-out gate pass.

| # | Command | Result (verbatim/summarized) | Verdict |
|---|---------|-------------------------------|---------|
| 1 | `./scripts/extract-public-api.sh .project/current-exports.txt` | `✅ API surface extracted to .project/current-exports.txt (3936 items)` | ✅ PASS |
| 2 | `./scripts/check-api-surface.sh .project/current-exports.txt` | `✅ API surface unchanged` (against the just-regenerated baseline) | ✅ PASS |
| 3 | `git diff --stat .project/current-exports.txt` | `1 file changed, 397 insertions(+), 13 deletions(-)` — the baseline genuinely changed vs the prior committed version (the carried Phase 25/26 concern is not left red) | ✅ PASS |
| 4 | `cargo build --workspace --all-targets` | `Finished dev profile [unoptimized + debuginfo] target(s) in 6m 48s` | ✅ PASS |
| 5 | `cargo test --workspace` | Every crate's `test result: ok`, aggregate **0 failed** across all lib/bin/doc/integration targets (paladin-ai 970, paladin-battalion 733+14 ignored, paladin-web 310, paladin-eval 585+doc, and every other member) | ✅ PASS |
| 6 | `cargo test --test evals` | `e2e-2-approval-gate.eval::approve ... ok` / `::deny ... ok` / `e2e-3-map-reduce-fault-tolerance.eval::recovering_worker ... ok` / `e2e-1-crash-resume.eval::crash_after_superstep_3 ... ok` — `test result: ok. 4 passed; 0 failed` | ✅ PASS |
| 7 | `cargo fmt --all --check` | Exit 0, no output | ✅ PASS |
| 8 | `cargo clippy --workspace --all-targets -- -D warnings` | `Finished` — exit 0, zero warnings | ✅ PASS |
| 9 | `cargo clippy --workspace --all-targets --features otel -- -D warnings` | `Finished` — exit 0, zero warnings | ✅ PASS |
| 10 | `cargo clippy --workspace --all-targets --features dev-ui -- -D warnings` | `Finished` — exit 0, zero warnings | ✅ PASS |
| 11 | `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` for all eleven pre-existing publishable crates (`paladin-ai`, `paladin-ai-core`, `paladin-ports`, `paladin-battalion`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-storage`, `paladin-notifications`, `paladin-content`, `paladin-web`) | Every crate: `Summary no semver update required` (193–196 checks each, 0 failures) | ✅ PASS (11/11) |
| 12 | Semver allowlist / MIGRATION.md §9.2 set-equality check (the exact `ci.yml` `semver` job script, reproduced locally) | `MIGRATION.md §9.2 deliberate-breaking crates: paladin-ai, paladin-ai-core, paladin-ports, paladin-web` / `Allowlist crates:` — identical sets, `diff` exits 0 | ✅ PASS |
| 13 | `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets --locked` | `Finished dev profile [unoptimized + debuginfo] target(s) in 4m 32s` | ✅ PASS |
| 14 | `cargo audit` | `warning: 10 allowed warnings found` (RustSec advisories on `rustls-pemfile`, `smartstring`, `event-listener`, `scc`, `chacha20`, `spin` — all pre-existing, none introduced by this phase, all documented in `.github/instructions/security.instructions.md`) — exit 0 | ✅ PASS |
| 15 | `cargo deny check` | `advisories ok, bans ok, licenses ok, sources ok` (pre-existing `axum`/`axum-core` duplicate-version and `chacha20`/`spin` yanked warnings, unchanged by this phase) — exit 0 | ✅ PASS |
| 16 | `./scripts/check-changelogs.sh` | `✅ 11 publishable crate(s) checked, all have a CHANGELOG.md.` (after adding `crates/paladin-eval/CHANGELOG.md` this task — see Deviations) | ✅ PASS |
| 17 | `./scripts/check-crate-names.sh` | `✅ 12 publishable crate(s) checked, all match the allow-list exactly.` (after adding `paladin-eval` to `.crate-names.txt` this task, verified NOT registered on crates.io first — `404`/`NoSuchKey` from `index.crates.io/pa/la/paladin-eval` — see Deviations) | ✅ PASS |
| 18 | `./scripts/check-advisory-register.sh` | `✅ 11 register row(s) checked against 11 deny.toml and 5 .cargo/audit.toml ignore entries; all clauses satisfied.` | ✅ PASS |
| 19 | `./scripts/check-workflow-suppressions.sh` | `✅ 7 workflow file(s) scanned, 157 run step(s) examined ... no inline advisory-ignore suppression detected.` | ✅ PASS |
| 20 | `./scripts/check-workflow-triggers.sh` | `✅ 7 workflow file(s) scanned, 7 policy-table row(s) read; coverage, drift, context and reachability clauses all pass.` | ✅ PASS |
| 21 | `./scripts/check-codeql-dismissals.sh` | `✅ 1 governed dismissal(s) checked ... all pass.` | ✅ PASS |
| 22 | `UPDATE_OPENAPI=1 cargo test -p paladin-web --features dev-ui --lib openapi_matches_committed_baseline` then `git diff --stat crates/paladin-web/openapi.json` | Test passes; diff is **empty** — confirms 28-11's own finding that `trace_seq`/`RunStreamMode::Replay` are unreachable from the OpenAPI schema-derivation sources, recorded as a known gap in the observability docs, not assumed | ✅ PASS (empty diff, as expected) |
| 23 | `cargo tree -e normal,build -p paladin-ai \| grep -c "opentelemetry\|libtest-mimic\|paladin-eval"` | `0` | ✅ PASS |
| 24 | `cargo doc --workspace --no-deps` | 64 total warnings across 8 crates (`paladin-battalion` 36, `paladin-ai-core` 14, `paladin-ai` 5, `paladin-web` 3, `paladin-llm` 3, `paladin-ports` 1, `paladin-storage` 1, `paladin-memory` 1) — `paladin-eval`, `paladin-content`, `paladin-notifications`, `paladin-herald` and `paladin-doc-examples` carry **zero**. Every warning is inherited from the merged base `08dc002b` (plans 28-01…28-16); this plan's own Task 1/2 commits touch zero `.rs` files (`git diff --stat 08dc002b HEAD -- '*.rs'` is empty), so this plan adds no new broken intra-doc link. See Deviations for the full reasoning. | ✅ PASS (no new links from this plan) |
| 25 | `mdbook-mermaid install docs && mdbook build docs` | `[INFO] mdbook_linkcheck] No broken links found` — exit 0, three new pages built | ✅ PASS |

**Local sweep verdict: 25/25 green.**

**Not run locally, by design (Docker/Java unavailable in this devcontainer, 24-CONTEXT D-28):** the
Postgres `run_traces` Tier 2 contract suite, live Redis contract suites, the `coverage` job's
`cargo llvm-cov --workspace --features integration-tests,llm-all --fail-under-lines 82` invocation
(hard-fails without Redis/MinIO reachable), and the real `openapi-generator-cli` run inside
`sdk-clients` (needs Docker). These map to the CI evidence table below.

---

## Pre-close-out CI evidence (base `08dc002b`, before this plan's own commits)

Four workflows ran on push at `08dc002b`. **This is evidence for the sixteen prior implementation
plans, not for this plan's own Tasks 1–3** — none of this plan's commits are in this SHA. Three
genuine failures below were already fixed by the orchestrator directly on `feature/phase-26` with
commits **not present in this worktree's base or history** (`c81a5e7a`, `59c33c19`, `b7d0fb52`);
one (the per-crate changelog / crate-name-allowlist gap) was fixed by this plan's own Task 3 work
(see Deviations). The post-merge run on the wave's final SHA — after this plan merges and those
three orchestrator commits are folded in — is what actually re-proves all four green together;
this table is the honest, dated snapshot of what `08dc002b` alone showed.

| Workflow | Run ID (URL) | Conclusion | Notes |
|---|---|---|---|
| `pre-commit` | `34344074340` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/34344074340 | **failure** | `end-of-file-fixer` on `evals/e2e-1-crash-resume.eval.crash_after_superstep_3.snap.json` — the `--bless` writer didn't append a trailing newline. **Fixed on `feature/phase-26` by the orchestrator: `c81a5e7a`** (not in this worktree's base). |
| `.github/workflows/feature-flags.yml` | `34344074315` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/34344074315 | **failure** (3 of 19 jobs) | `Build & Test (all-features)`, `Build & Test (full)`, `Build & Test (llm-all)` failed: `config::trace::tests::validate_typed_rejects_each_invalid_case_distinctly` assumed no `otel` feature compiled in, and `paladin-eval::runner::tests::live_mode_requires_flag_and_env_and_keys` assumed no compiled-in LLM provider — both wrong once workspace feature unification pulls Ollama in under `full`/`llm-all`. **Fixed on `feature/phase-26` by the orchestrator: `59c33c19`** (feature-aware assertions; `llm-all` leg reproduced locally by the orchestrator). **`OTel Feature (otel)` and `Build & Test (dev-ui)` — this plan's own two named legs — both PASSED**: `otel` ran `test result: ok. 2 passed; 0 failed` (the transport integration test) plus the span-tree shape tests (all green, 0 selected per crate but non-zero total across the workspace filter); `dev-ui` ran `test result: ok. 238 passed; 0 failed` (`--lib`, workspace). |
| `.github/workflows/codeql.yml` | `34344074336` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/34344074336 | **success** | `CodeQL Analysis (Rust)` — success, advisory-only per `security.instructions.md`. |
| `.github/workflows/ci.yml` | `34344074367` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/34344074367 | **failure** (4 of ~30 jobs) | See per-job table below. |

### `ci.yml` (`34344074367`) per-job detail

| Job | Conclusion | Notes |
|---|---|---|
| `License & Dependency Policy` | **failure → now fixed** | Failed at `Check per-crate changelogs`: `crates/paladin-eval` had no `CHANGELOG.md` (it is `publish = true`). **Fixed by THIS plan's Task 3**: `crates/paladin-eval/CHANGELOG.md` added (Keep-a-Changelog shape, `## [Unreleased]` section covering the phase's own additions) — the check-changelogs.sh/check-crate-names.sh sequence in that same job ALSO surfaced `paladin-eval` missing from `.crate-names.txt`, fixed in the same commit after confirming the name is not registered on crates.io (`404`/`NoSuchKey` from a direct `index.crates.io` query). Both re-run clean locally (evidence rows 16–17 above). `cargo-deny check` itself never reached in this CI run (job failed at an earlier step) but passes locally (row 15). |
| `API Surface Tracking` | **failure → now fixed** | Expected: the pre-Task-3 `.project/current-exports.txt` had not yet been regenerated for the twelfth crate's new public surface. This plan's Task 3 regenerates it (evidence rows 1–3 above); `check-api-surface.sh` now passes against the committed baseline. |
| `Docker Build` | **failure** | `Build Docker image` step: `src/application/cli/commands/eval.rs` includes `tests/helpers/e2e_fixtures.rs` via `#[path]`, but the Docker build context had no `tests/` directory copied in. **Fixed on `feature/phase-26` by the orchestrator: `b7d0fb52`** (`COPY tests ./tests` added to `Dockerfile` and `Dockerfile.chef`) — not in this worktree's base. |
| `Coverage` | **failure** | `Measure coverage` step: `cargo test --workspace --features integration-tests,llm-all` hit the SAME `paladin-eval::runner::tests::live_mode_requires_flag_and_env_and_keys` assertion bug the `feature-flags.yml` failures above hit (feature unification compiles Ollama in under `llm-all`). **No coverage-threshold percentage was produced by this run** — the failure occurred before `cargo llvm-cov` reported a number. **Fixed on `feature/phase-26` by the orchestrator: `59c33c19`** (same commit as the `feature-flags.yml` fix, not in this worktree's base). **This record therefore cannot cite an 82% coverage figure from `08dc002b` — none exists to cite.** The post-merge run on the final SHA is the source of that number; it is not fabricated or estimated here. |
| `Semver Checks (vs v0.9.0)` | **success** | `Summary no semver update required` for all 11 packages, matching this plan's own local re-run (row 11 above) verbatim. |
| `MSRV (Rust 1.88)` | **success** | `Finished dev profile [unoptimized + debuginfo] target(s) in 43.30s`, matching row 13 above. |
| `Postgres Storage Contract Suites (live server)` (`ci.yml`'s job key: `postgres-integration`) | **success** | `test result: ok. 97 passed; 0 failed`. All ten `run_trace::postgres::tests::*` cases (28-04's own Tier 2 suite — `append_is_idempotent_on_same_seq`, `append_then_read_round_trips`, `connection_error_redacts_password_from_database_url`, `prune_thread_removes_only_older_supersteps`, `read_of_unknown_thread_is_empty_not_error`, `read_paginates_by_after_seq`, `record_written_as_jsonb_reads_back_as_equal_record`, `records_are_scoped_by_thread`, `run_all_contract_functions_smoke_aggregate`, `unsupported_schema_version_is_typed`) ran and passed against a REAL Postgres server via the `postgres-integration` job — read green from this named CI run, never claimed locally (Docker is unavailable in this devcontainer). |
| `Redis Run Queue Contract Suite (live server)` | **success** | Unaffected by this phase. |
| `Redis Node Cache Contract Suite (live server)` | **success** | Unaffected by this phase. |
| `Ollama Integration Tests (live server)` | **success** | Unaffected by this phase. |
| `E2E Platform API (PRD 06 acceptance-1 lifecycle)` | **success** | Unaffected by this phase. |
| `Generated SDK Clients (Python + TypeScript) smoke` | **success** | The `sdk-clients` job — the real `openapi-generator-cli` run this devcontainer cannot reproduce locally — passed. |
| `Integration Tests` | **success** | |
| `Unit Tests (stable)` / `Unit Tests (beta)` | **success** / **success** | |
| `Crate Isolation (paladin-ai-core / -ports / -battalion / -herald / -llm / -memory / -storage / -notifications / -content / -web / -ai)` | **success** (all eleven) | |
| `CLI Snapshot Tests` | **success** | |
| `Code Quality`, `Workflow Lint`, `Security Audit`, `OSV Scanner` | **success** (all four) | |
| `Docker Integration Tests`, `Example Muster (Feature Matrix)`, `Benchmark Compile Check` | **success** (all three) | |
| `Benchmark Regression Signal (Non-Blocking)`, `Publish Dry Run`, `Kubernetes Smoke Test`, `End-to-End Tests` | **skipped** | Gated on the failing jobs above (standard `needs:` chain behavior on a red run), not evidence of anything themselves. |

---

## Summary and what remains

**What this record proves:** every gate this devcontainer CAN run locally is green (25/25, Local
sweep), including all eleven pre-existing crates' semver checks, the MSRV floor, security scans,
the regenerated API surface baseline, and the two feature-flags legs this plan's own scope names
(`otel`, `dev-ui`) — both read green from the pre-close-out CI run. The Postgres `run_traces` Tier
2 suite is read green from a named CI run with the exact ten test names, never claimed passed
locally. The per-crate-changelog and crate-name-allowlist gaps the pre-close-out CI run surfaced
were real, are fixed in this plan's own commits, and are re-verified locally as green.

**What this record does NOT claim:** an 82% (or any) workspace coverage percentage from
`08dc002b` — that run's `Coverage` job failed before producing one, for a reason already fixed
upstream but not present in this worktree. A green `Coverage` job with a real percentage, and a
fully green four-workflow run with none of the pre-close-out failures above, are both expected
from the **post-merge run on the wave's final SHA**, which the orchestrator appends after this
plan's commits (including the `CHANGELOG.md`/`.crate-names.txt`/`.project/current-exports.txt`
fixes) merge alongside the three orchestrator-authored fix commits (`c81a5e7a`, `59c33c19`,
`b7d0fb52`). This plan does not push and does not have access to that merged SHA's own CI run.

---

## Post-merge CI evidence (appended by the orchestrator at phase close-out, 2026-09-09)

Two definitive pushes of `feature/phase-26` followed the pre-close-out run above. Both were plain
branch pushes (no PR); the second superseded the first while it was still in flight.

### Push 1 — `b5ba9e07` (all 17 plans merged, pre code-review-fix)

| Workflow | Run | Conclusion | Notes |
|---|---|---|---|
| `pre-commit` | 34350681931 | **success** | |
| `.github/workflows/feature-flags.yml` | 34350682069 | **success** | all legs incl. `otel`, `dev-ui`, `llm-all` unification (fix `59c33c19` confirmed) |
| `.github/workflows/codeql.yml` | 34350682016 | **success** | advisory-only |
| `.github/workflows/ci.yml` | 34350682084 | cancelled (superseded) | 32 jobs **success**, 3 skipped by `needs:`; only `Docker Build` and `Kubernetes Smoke Test` were **cancelled** by the concurrency group when push 2 landed — every other job, including `Coverage`, `API Surface Tracking`, `License & Dependency Policy`, `Semver Checks`, `MSRV`, Postgres/Redis/Ollama live suites, `sdk-clients`, and all eleven crate-isolation legs, completed green |

`Coverage` (run 34350682084, `Coverage summary` step): `Lines: 107930/119558 = 90.27%` — above the
82% floor (ADR-0006).

### Push 2 — `ff78a6b5` (definitive: + code-review fixes `6c0c8b69`, `4b3a223a`, `8325accc`, `7f0dcb92`, review/fix reports)

| Workflow | Run | Conclusion | Notes |
|---|---|---|---|
| `pre-commit` | 34356304897 | **success** | |
| `.github/workflows/feature-flags.yml` | 34356304900 | **success** | |
| `.github/workflows/codeql.yml` | 34356305138 | **success** | advisory-only |
| `.github/workflows/ci.yml` | 34356304863 | **success** | 34 jobs **success**, 3 skipped (`Benchmark Regression Signal (Non-Blocking)`, `Publish Dry Run`, `End-to-End Tests` — gated on tag/manual triggers, not failures). `Docker Build` **success** (13:27→14:32 UTC; confirms fix `b7d0fb52`), `Kubernetes Smoke Test` **success** (14:32→14:37 UTC) |

`Coverage` (run 34356304863, job 102482098437, `Coverage summary` step):
`Lines: 108172/119822 = 90.28%` — above the 82% floor (ADR-0006).

Local close-out gates on `ff78a6b5` (warm checkout): `make test` 3592 passed / 0 failed;
`cargo test -p paladin-web --features dev-ui` 239 + 5 passed; `cargo test --features otel --lib
infrastructure::telemetry` 19 passed; `cargo test --features cli --test evals` 4 passed;
`scripts/check-api-surface.sh .project/current-exports.txt` → unchanged (3936 items); `make security`
→ `advisories ok, bans ok, licenses ok, sources ok` (two `warning[yanked]` notices on transitive
`chacha20`/`spin`, non-fatal); `cargo tree -p paladin-battalion --no-default-features -e normal |
grep -c paladin-llm` = 0 (ADR-0031).

**What this appendix proves:** the fully green four-workflow run with a real coverage percentage
that the section above said was "expected from the post-merge run on the wave's final SHA" exists —
run 34356304863 on `ff78a6b5` — and it includes the code-review fixes.
