---
phase: 54-first-run-usability
plan: 09
subsystem: evidence
tags: [evidence, prove-then-lock, boot-refusal, api-surface, windows-ledger, credential-review, first-run]
status: complete

requires:
  - phase: 54-first-run-usability
    provides: "54-01..54-08: the placeholder refusal, the doc and manifest sweeps, the version gate and the release-comment fixes this plan measures"
provides:
  - "54-FIRST-EVIDENCE.md: prove-then-lock rows for FIRST-01..09 and both scout facts, the process-level boot refusal, the gate red run, the API-surface result, the Phase 58 carry-forward, the phase-head gate sweep, the manual credential review and the known residuals"
  - "FIRST-08 wording reconciled (dated notes in REQUIREMENTS, ROADMAP SC5 and inventory C10) without touching the snapshot or src/lib.rs"
  - "WINDOWS row 62 fixed on re-run tests (open 16 to 15, fixed 32 to 33)"
  - "Two STATE.md blocker lines: the Phase 58 changelog and gate carry-forward, and the pre-existing run_api_wiring failures"
affects: [phase 57 review sweep, phase 58 pre-tag audit and lockstep bump]

tech-stack:
  added: []
  patterns:
    - "Evidence file in the 53-TENANT-EVIDENCE shape: dated header, per-section tables of command, before, head output and disposition"
    - "A requirement whose wording a tool cannot satisfy is reconciled with a dated, italic note that keeps the ID, the checkbox and the original text"

key-files:
  created:
    - .planning/phases/54-first-run-usability/54-FIRST-EVIDENCE.md
  modified:
    - .planning/WINDOWS.md
    - .planning/REQUIREMENTS.md
    - .planning/ROADMAP.md
    - .project/v0.11.1/01-carried-gaps-inventory.md
    - .planning/STATE.md

key-decisions:
  - "PD-09-1: WINDOWS row 62 closed here, because every piece of its closing evidence is produced here"
  - "PD-09-3 applied: FIRST-08 reconciled by note, not by regenerating or padding the snapshot (D-18); PD-09-2 did not fire because make api-surface reported the snapshot unchanged"
  - "The WINDOWS reason cell was written in both the table row and the ledger's JSON mirror, as the 53-07 row 63 precedent has it"

requirements-completed: [FIRST-08, FIRST-01, FIRST-02, FIRST-03, FIRST-04, FIRST-05, FIRST-06, FIRST-07, FIRST-09]

duration: ~20min
completed: 2026-11-01
---

# Phase 54 Plan 09: Phase evidence, FIRST-08 reconciliation and WINDOWS row 62 Summary

**The real `paladin-server` binary refuses both a `${...}` API key and the unfilled k8s Secret template (exit 1, named entry, no listener); the version gate goes red on a README pin of `0.5` and green again on restore; `make api-surface` is unchanged, so FIRST-08's wording is reconciled instead of the snapshot padded; WINDOWS row 62 is closed on re-run tests; and Phase 58 inherits a measured carry-forward.**

## Facts

- **Plan-start SHA:** `027891467d03cae84529e6ec61ae4d0bdf23ea49`
- **Head SHA the gates ran on:** `4cfeb724bee98603d400929a307e978a57e5821b` (the Task 2 commit, working tree clean)
- **Tasks:** 3 (1 tracer, 2 auto); 1 file created, 5 modified; docs and planning records only, no code or test changed
- **WINDOWS counters:** before `open 16, waived 39, fixed 32, total 87`; after `open 15, waived 39, fixed 33, total 87`

## Task Commits

1. **Task 1 (tracer): prove-then-lock, boot refusal (SC1), gate red run (SC3)** - `f6994ec05`
2. **Task 2: FIRST-08 API surface and reconciliation, WINDOWS row 62, Phase 58 carry-forward** - `4cfeb724b`
3. **Task 3: phase-head gates, credential review, residuals** - `e077d9e56`

Tracer feedback gate (auto mode): the tracer's `<verify>` ran end to end and passed before Tasks 2 and 3 began.

## Accomplishments

- **SC1, process level.** After `cargo build --features web-server --bin paladin-server`: (a) a config whose only key is the placeholder for `PALADIN_API_KEY_CI` exits `1` with `paladin-server failed to start: http.auth.api_keys[ci-runner]: 'key' is an unexpanded '${…}' placeholder -- ...`, with the variable name absent and no listening line; (b) the Secret template's own `config.yml` (port changed to 18081, `OPENAI_API_KEY=sk-dummy-not-a-real-key`) exits `1` with `http.auth.api_keys[#0]: 'key' must not be empty`, after the agent service was built, so the failure is the auth step (RESEARCH A2 held; no field had to be adjusted).
- **SC3, real tree.** README line 108 set to `"0.5"`: the gate printed `README.md:108: found '0.5', want '0.11.0'` (script exit 1; `make` wraps it as exit 2); after `git checkout -- README.md` it printed `OK: 128 file(s) scanned ... class 1 Cargo pins: 42; class 2 Current version lines: 1; class 3 k8s labels and image tags: 8; class 4 openapi.json info.version: 1; exempt files: 1` and exited 0. `git status --porcelain README.md` was empty after both.
- **Prove-then-lock table.** One row per FIRST-01..09 plus the two scout facts, each with command, before output (from the SUMMARYs), head output and disposition (`changed in 54-NN` or `already closed`); the line drift (CHANGELOG footer link at 3413, `ci.yml` comment at 325) and the manual-only k8s quick-start read are recorded.
- **FIRST-08.** `make api-surface` on `nightly-2026-09-20`: exit 0, `API surface unchanged`, 4325 items. `grep -c 'UnsupportedParameter\|check_reasoning' .project/current-exports.txt` prints `0`; both items live in `paladin-ports`, which the facade-only snapshot does not re-export (`src/lib.rs:175`). `sha256sum` of the snapshot and `src/lib.rs` is identical before and after, and `git diff 027891467 -- src/lib.rs .project/current-exports.txt` is empty. REQUIREMENTS FIRST-08, ROADMAP Phase 54 SC5 and inventory C10 carry a dated reconciliation note citing WINDOWS row 59 and `make check-semver`.
- **WINDOWS row 62.** The three closing tests were re-run on the head (`validate_refuses_an_unexpanded_placeholder_key` 1 passed; `shipped_example` 2 passed; `build_auth_config_refuses_an_unexpanded_placeholder_key` 1 passed), then `windows fixed 62` and the reason cell. `windows status` loads the ledger with no count mismatch; only the frontmatter counters, `last_updated` and row 62 changed.
- **Phase 58 carry-forward.** Seven of the eleven crate changelogs lack `## [Unreleased]` and `## [0.11.1]` (battalion, eval, herald, llm, notifications, ports, storage), as RESEARCH F6 expected. Recorded with finalize rule 3, the 52 strings the version gate will list at the bump (42 + 1 + 8 + 1), and the unverified dirty-tree behaviour of `cargo release version --execute` (RESEARCH A3). STATE.md has the blocker line.
- **Phase-head gates (all exit 0 on `4cfeb724b`):** `cargo test` (3132 passed, 0 failed, 43 ignored across 35 result lines), `cargo test --features web-server --bin paladin-server` (21 passed), `cargo fmt --check`, `cargo clippy -- -D warnings`, `make clean-code`, `make check-gates`, `make test-shell-guards` (85 assertions), `make api-surface`, `make security` (cargo-audit with 9 allowed warnings, cargo-deny `advisories ok, bans ok, licenses ok, sources ok`), `make check-migration-allowlist`, `make check-doc-config` (161 blocks, 0 failed).
- **Manual credential-handling review.** Eight rows (the refusal message, the `ApiKeyConfig` rustdoc, the `validate` rustdoc, and the key comments in `config.example.yml`, the Secret template, the compose file, `Dockerfile.server` and `http-service-host.md`), each checked for no key or variable name in a message, no new `Debug` or serialization of a key-carrying type, literal keys only in a Secret volume or a read-only mount, and no redirect or response-body handling added. CodeQL is stated as advisory-only, so the review is the control.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Reconciliation notes carry the real date, 2026-11-01, not the plan's 2026-10-31**
- **Found during:** Task 2
- **Issue:** The plan's note text and Task 2 `<verify>` grep for `Reconciled 2026-10-31`, but the plan ran on 2026-11-01 (every 54-0N SUMMARY and the ledger timestamps say so). A note dated a day before it was written would be false.
- **Fix:** The three notes read `Reconciled 2026-11-01, Phase 54, D-18`; the Task 2 verify was run with that date substituted and passed.
- **Files modified:** `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.project/v0.11.1/01-carried-gaps-inventory.md`
- **Commit:** `4cfeb724b`

**2. [Rule 3 - Blocking] The ledger keeps a JSON mirror of every row**
- **Found during:** Task 2 step 3
- **Issue:** `.planning/WINDOWS.md` ends with a JSON block of all entries; `windows fixed 62` updated the table row and the JSON status but left `reason` empty in both.
- **Fix:** Wrote the reason (no `|`) into the table cell and the JSON `reason`, the same shape as row 63. Counts and every other row are unchanged.
- **Commit:** `4cfeb724b`

**3. [Rule 3 - Blocking] `gsd-tools state add-blocker` removes the word "none" from unrelated STATE.md lines**
- **Found during:** Task 2 and Task 3 (each `add-blocker` call)
- **Issue:** The tool's placeholder cleanup stripped `none` from the existing line `Policy::none()` (it became `Policy::()`), twice.
- **Fix:** Restored the line by hand each time and confirmed the STATE.md diff holds only the intended change. Worth a tool fix; later `state` calls in this plan were checked the same way.
- **Commits:** `4cfeb724b`, `e077d9e56`

**4. [Rule 3 - Blocking] Disk full during the first `cargo test`**
- **Found during:** Task 3 step 1
- **Issue:** The gate script's `cargo test` aborted with `rustc-LLVM ERROR: IO failure on output stream: No space left on device` (about 4 GB free at the start). Not a test failure.
- **Fix:** Deleted `target/debug/incremental` (the build cache the orchestrator named, nothing else) and re-ran `cargo test` with `CARGO_INCREMENTAL=0`; it finished `exit=0`. Both runs are recorded in the evidence file's gate table, and the failed attempt is not counted as a pass.

### Notes

- `make check-version-strings` exits 2 when the script exits 1 (make's own recipe-failure status); the evidence records both, as the plan's "exit 1" refers to the script.
- No PD-09-2 contingency (the snapshot was unchanged) and no RESEARCH A2 deviation (neither boot run needed an adjusted field).
- The plan lists seven known residuals; the evidence file records ten (the extra three are the pre-existing `run_api_wiring` failures, the no-cluster/no-coverage note, and a pre-existing `Dockerfile.server` line-56 comment that is now imprecise).

## Known residuals

Carried to Phase 57 (full list with pointers in `54-FIRST-EVIDENCE.md` `## Known residuals`):

- **Pre-existing, environment-specific test failure.** `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `...build_run_api_persists_run_traces_when_trace_persist_is_set` fail here at `src/infrastructure/web/run_api_wiring.rs:3036:13` (`the run never reached a terminal status`, 10 s deadline); I reproduced both in isolation on this head (`2 failed ... 1479 filtered out`). No Phase 54 change touches that file (`git diff --stat 8b23cb4c5 HEAD` on it is empty; the phase's only Rust changes are `src/config/agents.rs` and `src/bin/paladin-server.rs`). The orchestrator reports the same failures on base `8b23cb4c5` and green CI at the PR #62 merge. Cause undiagnosed; STATE.md Blockers/Concerns line added.
- `defaultMode: 0440` with `fsGroup: 65532` untested on a cluster (RESEARCH A1); fallback `defaultMode: 0444` on both Deployments' `config` volumes, as 54-06 records.
- The 82 % coverage floor (CI `Coverage` job; `cargo-llvm-cov` and Docker services absent here) and `actionlint` (CI job; not installed here) were not run locally and are not recorded as passes. `make check-semver` builds against crates.io and is left to CI and Phase 58.
- `ghcr.io/your-org` in two deployment guides, `paladin:latest` in two other pages, the frozen v0.9.0 fixture placeholder, the unrefused treasurer webhook `secret` placeholder, `ApiKeyConfig`'s pre-existing `Debug` derive, no boot warning for a node cache on the server (PACE-07), and the stale `Dockerfile.server:56` comment.

## Known Stubs

None. `REPLACE_ME` and empty `key: ""` entries in the Secret template are the intentional unfilled-template placeholders, refused at boot (SC1 (b)).

## Threat Flags

None. This plan adds no endpoint, auth path or trust-boundary surface. T-54-27 (row 62 closed without proof) is mitigated by the three re-run tests before `windows fixed`; T-54-28 (snapshot padded) by the empty `git diff` of `src/lib.rs` and the snapshot and the matching checksums; T-54-29 (a real key captured) by using only the placeholder text and `sk-dummy-not-a-real-key`, with zero matches for `sk-[A-Za-z0-9]{20,}` in the evidence and the scratch files; T-54-30 (a gate recorded as passing that was not run) by recording each gate's own result line with the head SHA and labelling the CI-only jobs and the failed first `cargo test` attempt.

## Self-Check: PASSED

- FOUND: `.planning/phases/54-first-run-usability/54-FIRST-EVIDENCE.md` with the eight sections the plan lists (plus `## WINDOWS row 62 closing evidence`)
- FOUND commits: `f6994ec05`, `4cfeb724b`, `e077d9e56` in `git log`
- Task 1, Task 2 (date substituted) and Task 3 `<verify>` commands all exited 0
- `git diff 027891467 -- src/lib.rs .project/current-exports.txt` is empty; `git status --porcelain README.md` is empty
