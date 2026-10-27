---
phase: 48-base-entity-rename
plan: 09
subsystem: closeout
tags: [closeout, gates, adr-0060, conformance, requirements-wording, guard-fix, d-00a]

requires:
  - phase: 48-07
    provides: "Register, allowlist block and the D-27 semver numbers"
  - phase: 48-08
    provides: "tests/entity_rename_guard.rs and the freeze base"
provides:
  - "Every project gate run on the finished tree with its exit code, the freeze and Cargo.lock re-proven against the phase base"
  - "ADR-0060 Code Conformance `conforms` with one evidence bullet per decision group; PROJECT.md row Implemented"
  - "REQUIREMENTS.md and ROADMAP Phase 48 wording matches the tree (ENTITY-01 storage list, ENTITY-04 type name and outcome, ENTITY-05 baseline file), IDs untouched"
  - "A fix to the entity rename guard that had turned the Docker Integration Tests CI job red"
affects: [phase-46, phase-47]

tech-stack:
  added: []
  patterns:
    - "A repository-scan guard that reads a docs/ file must tolerate a build context with no docs/ directory (the Docker test image)"

key-files:
  created: []
  modified:
    - tests/entity_rename_guard.rs
    - .planning/decisions/0060-base-entity-rename.md
    - .planning/PROJECT.md
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md

key-decisions:
  - "Code Conformance is `conforms` because every decision group has a passing named test or gate on the finished tree; the two gates that could not be completed in the sandbox (make security, the coverage percentage) are recorded as gaps, not as passes"
  - "The coverage floor is evidenced by CI's Coverage job passing on branch head dcbcb63 (the job runs cargo llvm-cov --fail-under-lines 82); no percentage is quoted because none was retrievable"
  - "The Docker-context failure was fixed in the guard (skip the schema only when the tree has no docs/ directory), not by editing .dockerignore or the Dockerfile"

patterns-established: []

requirements-completed: [ENTITY-01, ENTITY-02, ENTITY-03, ENTITY-04, ENTITY-05]

duration: about 1 hour
completed: 2026-10-14
status: complete
---

# Phase 48 Plan 09: Closeout Summary

**Every project gate was run on the finished tree (all green except the two named sandbox-baseline tests), a real regression in the 48-08 guard that broke CI's Docker Integration Tests was found and fixed, ADR-0060 now reads `conforms` with per-group evidence, and the three inherited wording errors in REQUIREMENTS.md and the ROADMAP Phase 48 section are corrected without touching a requirement ID. Two gates could not be completed in this sandbox and are recorded as gaps: `make security` (tools absent; CI jobs red before Phase 48 too) and the numeric coverage figure (CI gate passed, percentage not retrievable).**

## Phase base

`BASE=3cac94f` (`docs(phase-48): begin phase execution`), computed by the plan's formula: the parent of `f053585`, the first commit whose subject contains `(48-0`. Branch `claude/laughing-dirac-e0h2ax`.

## Gate results (D-00a), one line per gate

All run with `CARGO_INCREMENTAL=0` on the finished tree. Free disk was 6.4 GB before the heavy gates and 3.3 GB at the end; nothing was deleted.

| # | Step | Gate | Exit | Key counts |
|---|------|------|------|------------|
| 1 | 1 | `cargo fmt --all --check` | 0 | clean (re-run after the guard fix: 0) |
| 2 | 1 | `make lint` (`clippy --workspace --all-targets --all-features -D warnings`) | 0 | re-run after the guard fix: 0 |
| 3 | 1 | `make check` | 0 | |
| 4 | 1 | `cargo check --workspace --all-targets --all-features` | 0 | |
| 5 | 1 | `make doc-check` (default-features zero warnings, all-features `-D warnings`, doctests) | 0 | 0 `warning:` lines; 603 doctests passed, 0 failed, 210 ignored |
| 6 | 1 | `make check-api-examples` | 0 | 105 of 105 entry points carry `# Examples` |
| 7 | 1 | no shell script changed: `git diff --name-only 3cac94f -- '*.sh'` | prints nothing | `shellcheck` is not installed locally; CI `Code Quality` job passed on head |
| 8 | 2 | `cargo test --workspace --no-fail-fast` | 101 | 6,907 passed, 2 failed, 229 ignored over 61 result lines; the only failures are `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `..._persists_run_traces_when_trace_persist_is_set` (48-RESEARCH.md Pitfall 10, confirmed by name, neither skipped nor edited) |
| 9 | 2 | `cargo test --workspace --doc` | 0 | run inside `make doc-check` (row 5) and again inside row 8 |
| 10 | 3 | `cargo test --test entity_rename_guard` | 0 | 5 passed after the fix (4 passed before it) |
| 11 | 3 | `cargo test --test legacy_removal_guard` (D-00e) | 0 | 3 passed |
| 12 | 3 | freeze: `git diff --exit-code 3cac94f --` the schema and the ten frozen graph sources | 0 | byte-identical |
| 13 | 4 | `make check-gates` (includes `check-migration-allowlist`, `check-changelogs`) | 0 | allowlist set-equal to the 9.2 register |
| 14 | 4 | `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | 0 | surface unchanged, 4,274 items; `.project/current-exports.txt` numstat 15/15 (14 lines plus the generated header) |
| 15 | 4 | `make check-doc-examples` | 0 | 0 checked, 617 skipped, 0 failed |
| 16 | 4 | `make check-examples` | 0 | 62 of 62 example binaries |
| 17 | 5 | `git diff --exit-code 3cac94f -- Cargo.lock` (D-00d) | 0 | unchanged; the only manifest change is the two allow lines in `crates/paladin-core/Cargo.toml` |
| 18 | 5 | `make security` | not run | GAP, see below |
| 19 | 6 | credential-handling review | done | see below |
| 20 | 7 | `make coverage` | 2 (service probe) | GAP for the percentage, see below |
| 21 | 8 | `git branch --show-current` | `claude/laughing-dirac-e0h2ax` | no separate PR branch exists (`git branch -a` lists only this branch and its `origin` ref); no `/gsd-pr-branch` run (D-06) |

Row 8 detail: the command exits 101 only because of the two named tests. Every other test target passed.

### Gap: `make security` (D-00a)

`cargo-audit` and `cargo-deny` are not installed here, and plan flagged assumption A2-48-09 says not to install them (they are not in 48-RESEARCH.md's Package Legitimacy Audit). The fallback is CI's security jobs on the branch head, and they are **not green**:

- On head `dcbcb63` (run 38487208284): `Security Audit` fails at the step `Run cargo-audit (exceptions from .cargo/audit.toml)` and `License & Dependency Policy` fails at `Run cargo-deny check`.
- The same two jobs fail on every earlier push of this branch, including docs-only commits made before any Phase 48 code existed: `License & Dependency Policy` is red from at least 2026-10-12 14:03Z (head `e9a20e6`) and `Security Audit` from 2026-10-12 18:04Z (head `d61d5c1`, a Phase 44 commit). `Cargo.lock` is byte-identical to the phase base (row 17), so Phase 48 cannot have introduced an advisory. I could not read the job logs (the proxy denied the log download), so I do not know which advisory or licence rule fires.

This acceptance criterion ("`make security` exits 0, or CI's security job passed") is therefore **not met**. It is a pre-existing, branch-wide failure outside this phase's scope. Recorded in `.planning/WINDOWS.md` (kind `unrun-verify`).

### Gap: the measured coverage percentage (D-00a)

`make coverage` exits 2 immediately: `scripts/coverage.sh` hard-fails with `ERROR: Redis unreachable ... Start services with 'make services-up'`. The sandbox has the `docker` client but no daemon (`failed to connect to the docker API at unix:///var/run/docker.sock`), so Redis and the object store cannot be started, and `cargo-llvm-cov` is not installed either. With 3.3 GB free an instrumented workspace build would not fit regardless.

What was obtained instead, from CI on branch head `dcbcb63` (run 38487208284, job 115604145094): the `Coverage` job **passed**. That job runs `bash scripts/coverage.sh`, which ends in `cargo llvm-cov --workspace --features integration-tests,llm-all --fail-under-lines 82`, so a pass means the measured workspace line coverage is at least 82 %. The `Coverage` job also passed on the 48-02, 48-03, 48-04 and 48-06 heads. The exact percentage lives in the `coverage-summary` artifact (id on the run, 929,290 bytes), but downloading it returned 403 from the egress proxy's blob-storage policy; I did not retry (organization policy denial). **No percentage is quoted here because none was measured by me.** The plan's criterion "records a workspace coverage figure of at least 82 %" is met only in the form "the CI gate at 82 % passed on the head"; someone with artifact access should copy the figure from that artifact. Recorded in `.planning/WINDOWS.md`.

Head `dcbcb63` carries the same Rust code as the final tree except the guard fix below, which only adds a test.

### Manual credential-handling review (step 6)

`git diff 3cac94f -- '*.rs' | grep -nE '^[+-].*(api_key|ApiKey|secret|token|Authorization|redact|bearer)'` printed 11 lines. Each was inspected:

- 6 are the `.node.` to `.data.` payload rename on token-count accounting (`result.usage.total_tokens`, `per_paladin_tokens`, `token_count`): usage counters, not credentials.
- 5 are the 48-05 compat test reading `max_tokens` from the `prompt.json` fixture (a model parameter).

No hit touches an API key, an `Authorization` header, a redaction path or a secret. A second grep for `reqwest`, `Client::builder`, `redirect`, `impl .* Debug for`, `webhook` and `ssrf` over the same diff printed nothing, and the whole `paladin-llm` and `paladin-web` diff is 32 insertions and 32 deletions of the same mechanical rename. Conclusion: no credential-bearing code path, log statement, `Debug` impl or HTTP client changed.

## ENTITY-05 reconciliation

Plan 48-05 marked ENTITY-05 complete before 48-06..48-08 had delivered the docs, guard and register legs. Each clause checked against the finished tree:

| Clause | Evidence | Met |
|--------|----------|-----|
| graph vocabulary untouched | row 12 (byte-identical) and guard rule 5 | yes |
| an ADR reserves "node" for the graph and "Entity" for the base wrapper | ADR-0060 decision (i) | yes |
| CLAUDE.md/copilot naming table and mdBook domain-model, design-patterns, crate-map agree | `Entity<T>` in CLAUDE.md (2 lines), the `Entity` row in `.github/copilot-instructions.md`, 4 / 3 / 1 mentions in the three pages | yes |
| repository-scan test in the legacy guard shape, positive controls included | `tests/entity_rename_guard.rs`, 5 tests | yes |
| every rename has a 9.2 row with a set-equal allowlist entry | row 13 | yes |
| baseline refreshed with a CHANGELOG entry | row 14, root and `paladin-core` CHANGELOG blocks (48-03, 48-07) | yes |
| `cargo doc`, `make check-api-examples`, `make clean-code`, 82 % floor stay green | rows 2-6 and 7 (shellcheck not runnable locally, no `.sh` changed, CI `Code Quality` passed), and the CI `Coverage` pass | yes, with the coverage percentage caveat above |

All five requirements stay `Complete`; no clause is unmet. ENTITY-01..04 were also re-checked: fixtures and `FileCitadel` tracer (ENTITY-01), ADR-0060 with 9.7 empty (ENTITY-02), exactly one `Paladin` alias and one `NodeError` under guard rule 4 plus `EntityVersion*` (ENTITY-03), `# Revisioning contract` rustdoc and the `# Retained pending review` rustdoc (ENTITY-04).

## Task Commits

1. **Task 1: gates** - no commit was demanded by a gate except the fix below.
   - `2ad9fa6` fix(48-09): let the entity rename guard run in a build context without docs/
2. **Task 2: ADR-0060 conforms and the wording corrections** - `ddd0c9d` (docs)

**Plan metadata:** the commit carrying this SUMMARY, STATE.md, ROADMAP.md progress and the WINDOWS.md entries (docs: complete plan).

## Task 2 acceptance counts

- Code Conformance contains `conforms`, no `must change`; `entity_surfaces_are_renamed_in_the_tree` appears in the ADR 1 time and `v0_10_citadel_statuses_load_into_the_unified_status` 2 times.
- Five `**ENTITY-0n**` requirement lines before and after; traceability rows `| ENTITY-0[1-5] | Phase 48 |`: 5 before, 5 after; no checkbox or ID changed.
- `retained pending review` in REQUIREMENTS.md: 1; `api_surface_current.txt` in REQUIREMENTS.md: 0 (the dated notes do not name it).
- ROADMAP Phase 48 section: `CollectionVersionService` 1, `SQLite/Postgres rows` 0. The section's "Ground truth" survey paragraph still says `CollectionVersioningService` because it records the 2026-10-10 survey and the plan limits the fix to criteria 1, 4 and 5.
- `Corrected ... (plan 48-09)` in REQUIREMENTS.md: 2 (under the ENTITY intro and under the footer); one in ROADMAP.
- `git diff HEAD~1 --stat` for `ddd0c9d` lists exactly the four planned files.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The 48-08 guard failed in the Docker test image**
- **Found during:** Task 1, while reading CI on the branch head (not by a local gate: the local run was green).
- **Issue:** the `Docker Integration Tests` CI job went from success (48-06 head) to failure at the 48-08 head. The Docker `test` stage copies only `Cargo.toml`, `Cargo.lock`, `src`, `crates`, `benches`, `tests` and two config files, and `.dockerignore` excludes `docs/`, so `entity_surfaces_are_renamed_in_the_tree` reported `docs/schemas/wargraph-doc.schema.json: rule 5: target file is missing` under `cargo test --features integration-tests`. I reproduced it by copying that context to a scratch directory and compiling the guard against it with `CARGO_MANIFEST_DIR` pointed there: the schema was the only violation, and adding the schema to the copy made all 4 tests pass. I could not read the CI log, so the CI cause is established by that reproduction, not by the log.
- **Fix:** test first (`scanner_accepts_a_build_context_without_docs`, red), then `scan_declared` skips the schema only when the tree has no `docs/` directory. A tree with `docs/` but without the schema is still reported (`scanner_reports_missing_scoped_targets`, unchanged), and the byte-identical freeze is proved by row 12. The same build against the scratch Docker context now passes all 5 tests.
- **Files modified:** `tests/entity_rename_guard.rs`
- **Commit:** `2ad9fa6`
- **To confirm:** the fix has not been pushed from here, so CI's `Docker Integration Tests` has not yet been seen green on it.

## Deferred Issues (out of scope, not fixed)

- **`pre-commit` CI workflow red since 2026-10-12 18:59Z** (head `2485b2a`, a docs-only commit before any Phase 48 code; last green 18:37Z at the Phase 44 seal). I could not read the log. A by-hand check of trailing whitespace and end-of-file newlines over every non-`.rs` file changed in Phase 48 found nothing, and `cargo fmt`, clippy, the API surface and the doc gates all pass locally, so the failing hook is not one of those. Needs someone with log access.
- **`Security Audit` and `License & Dependency Policy` CI jobs red** (see the `make security` gap).
- Open `.planning/WINDOWS.md` entries: the two `unrun-verify` rows above; they block `/gsd-ship` until closed, which is intended.

## Authentication Gates

None. The CI artifact and log downloads were denied by the egress proxy's policy (403); I did not retry or work around them.

## Known Stubs

None.

## Threat Flags

None. The only source change is a test-file guard tweak; no network endpoint, credential path or trust-boundary change.

## Self-Check: PASSED

- Files found: `tests/entity_rename_guard.rs`, `.planning/decisions/0060-base-entity-rename.md`, `.planning/PROJECT.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`
- Commits found: `2ad9fa6`, `ddd0c9d`
- Claims that are NOT self-verified: the `make security` result and the coverage percentage (see the two gaps above).
