---
phase: 29-program-gates-release
plan: 09
subsystem: infra
tags: [release, cargo-release, changelog, semver, dry-run-publish, ship-04, d-18, d-19, d-20, d-21, d-23]

# Dependency graph
requires:
  - phase: 29-01
    provides: "MIGRATION.md's TBD gate, D-06/D-07/D-08 SHIP-02 proofs (v0_9_config_boot, openapi_golden_v0_9) this plan re-runs post-bump"
  - phase: 29-04
    provides: "Audit sections 1-5 (per-FR table, X-rule spot-checks, E2E re-runs, BUG-01/02 re-verification)"
  - phase: 29-05
    provides: "MIGRATION.md closed (zero TBD markers), the semver allowlist/section 9.2 row-level CI gate"
  - phase: 29-07
    provides: "Audit sections 6-9 filled, the D-16 accepted-deviation record (artefacts 1-2 of 4), the proposed-but-unfiled 72-warning cargo-doc WINDOWS.md row"
  - phase: 29-08
    provides: "WINDOWS.md fully triaged (open_count: 0) — this plan's release-readiness verdict cites a closed register"
provides:
  - "Every publishable crate (twelve) plus the doc-examples workspace member at 0.10.0; Cargo.lock committed; crates/paladin-web/openapi.json's info.version regenerated to 0.10.0"
  - "All twelve publishable packages' changelogs carry a dated ## [0.10.0] section; root CHANGELOG.md curated (Behavioral changes sub-list first, grouped Added/Changed/Fixed preserved, new Known limitations section carrying the D-16 deviation — artefact 3 of 4)"
  - ".planning/phases/29-program-gates-release/29-CI-EVIDENCE.md: a 16-row local sweep against the actual bumped tree plus the CI-run table for the newest available feature/phase-26 run"
  - ".project/v0.10.0/09-program-acceptance-audit.md section 10 filled (Verdict: PASS) and the whole ten-section audit closed with zero pending verdicts; 29-ACCEPTANCE-AUDIT.md's overall verdict updated to PASS with findings"
affects: ["/gsd-ship (this branch is release-ready pending push + PR + real CI run)", "the milestone close-out that cuts the v0.10.0 tag from main after merge"]

# Tech tracking
tech-stack:
  added: []
  patterns: ["lockstep version bump via cargo-release, changelog stamped then hand-curated (D-19's script-stamps/human-curates split)", "measured-vs-planning-time discrepancies recorded as precision findings rather than forced to match a stale plan-text number (continuing 29-04/29-07's house pattern)"]

key-files:
  created:
    - .planning/phases/29-program-gates-release/29-CI-EVIDENCE.md
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/paladin-core/Cargo.toml
    - crates/paladin-ports/Cargo.toml
    - crates/paladin-herald/Cargo.toml
    - crates/paladin-battalion/Cargo.toml
    - crates/paladin-llm/Cargo.toml
    - crates/paladin-memory/Cargo.toml
    - crates/paladin-storage/Cargo.toml
    - crates/paladin-notifications/Cargo.toml
    - crates/paladin-content/Cargo.toml
    - crates/paladin-web/Cargo.toml
    - crates/paladin-eval/Cargo.toml
    - crates/doc-examples/Cargo.toml
    - crates/paladin-web/openapi.json
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-herald/CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - crates/paladin-memory/CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - crates/paladin-notifications/CHANGELOG.md
    - crates/paladin-content/CHANGELOG.md
    - crates/paladin-web/CHANGELOG.md
    - crates/paladin-eval/CHANGELOG.md
    - .project/v0.10.0/09-program-acceptance-audit.md
    - .planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md

key-decisions:
  - "Two additional comment-only 0.9.0 string occurrences (crates/paladin-battalion/Cargo.toml:44, crates/paladin-web/Cargo.toml:70) survive the bump beyond the single root-manifest schemars comment the plan's own <interfaces> text names -- both read in full context and confirmed non-pin (a mirrored schemars-transitive-version note and a struct_marked_non_exhaustive suppression comment citing v0.9.0 historically) before being left untouched. Recorded as a measured-vs-planning-time precision finding in audit section 10 rather than forced to match the plan's stated count; the actual acceptance test applied was 'zero non-comment 0.9.0 lines in any Cargo.toml', which holds."
  - "ci.yml's literal 'baseline-version 0.9.0' substring count measured 3 at HEAD (lines 276, 334, 357), not the plan's stated 5 -- the two additional line numbers it names (304, 339) carry 'v0.9.0' in adjacent job-name/comment prose without the exact substring. Recorded as a precision finding; all five referenced line numbers are present, unedited, and the file is untouched by every commit in this plan regardless of which literal count applies."
  - "The finalize-crate-changelogs.sh insertion point (immediately after the '## [Unreleased]' anchor) already accomplishes D-19's 'move the entries under the new heading' instruction as a side effect for every changelog whose Unreleased content sat directly below the anchor with no intervening content -- confirmed for all twelve files before doing any further manual editing. Task 2's actual manual work was root CHANGELOG.md's curation (Behavioral changes sub-list, Known limitations section) on top of that already-correct placement, not a from-scratch content move."
  - "A full cargo test --workspace run hit 3 transient 30s-budget timeouts in pre-existing, unrelated-to-this-plan async tests (paladin-ai's cancel_tests/stream_tests) under this devcontainer's concurrent-build load; re-run individually with --test-threads=1 (all 3 passed) and a clean full-workspace re-run (0 failed) confirmed devcontainer-load flakiness, not a regression -- this plan's own commits touch zero .rs files. Recorded honestly in 29-CI-EVIDENCE.md row 6 rather than silently re-running until green with no note."
  - "This branch (feature/phase-26) has never been pushed past commit 77912ac8, a Phase 28 post-merge-evidence commit predating every one of Phase 29's nine plans -- confirmed via git fetch + gh run list, not assumed. The CI-run table in 29-CI-EVIDENCE.md cites the newest available run on that stale SHA honestly (proving the pre-Phase-29 base was green) rather than fabricating or omitting a table for commits that have no CI run at all; the docs.yml workflow has additionally never run for this branch at all (PR-gated, no PR open) -- both gaps are named as owed to the orchestrator's eventual push/PR, per this plan's own instruction not to push or run gh run watch."
  - "The D-16 tracing-overhead deviation's four-artefact cross-reference (audit, published docs, CHANGELOG.md, WINDOWS.md) reaches 3 of 4 in this plan's own scope -- the root CHANGELOG.md's new Known limitations section is artefact 3. WINDOWS.md (artefact 4) is explicitly NOT touched by this plan per the orchestrator's file-scope prohibition; it was already closed by plan 29-08 (row id 35, waived, citing the same maintainer sign-off), so the cross-reference is complete across the phase even though this single plan does not write to that file."

patterns-established:
  - "When a finalize/stamp script's insertion point happens to already satisfy a 'move content under the new heading' instruction as a side effect of where the anchor sits, verify that placement explicitly (diff review) before doing redundant manual work, and note the finding as a decision rather than silently claiming manual authorship of an already-correct result."

requirements-completed: [SHIP-04, SHIP-03]

coverage:
  - id: D1
    description: "Every publishable crate (twelve) plus doc-examples bumped to 0.10.0 via cargo-release; Cargo.lock committed; openapi.json's info.version regenerated (1-line diff only); the two intentionally-pinned occurrences (ci.yml's five baseline-version 0.9.0 lines by line-number presence, root Cargo.toml's schemars comment) left untouched; both SHIP-02 proofs (v0_9_config_boot, openapi_golden_v0_9) re-run green post-bump; no tag created"
    requirement: SHIP-04
    verification:
      - kind: integration
        ref: "cargo test --features web-server --test v0_9_config_boot -> 9 passed; cargo test -p paladin-web --test openapi_golden_v0_9 -> 6 passed; cargo build --workspace -> Finished"
        status: pass
      - kind: other
        ref: "grep-based reach verification of every version field and every 0.9.0 occurrence across all Cargo.toml files, .github/workflows/ci.yml, and crates/paladin-web/openapi.json's info.version -- all pass per 83d219f1's commit message"
        status: pass
    human_judgment: false
  - id: D2
    description: "All twelve publishable packages' changelogs carry a dated ## [0.10.0] section with [Unreleased] preserved and empty; root CHANGELOG.md curated with a Behavioral changes sub-list (M-B-01..04, each linking MIGRATION.md Sec9.1), the pre-existing Added/Changed/Fixed content preserved, and a new Known limitations section carrying the D-16 deviation (+22.18%/+18.46%); check-release-consistency.sh --tag v0.10.0 exits 0; zero .rs files touched"
    requirement: SHIP-04
    verification:
      - kind: other
        ref: "./scripts/check-release-consistency.sh --tag v0.10.0 -> OK, 12 packages checked; awk-scoped grep counts on the [0.10.0]..[0.9.0] range: behavioral-change >=1, all four M-B IDs present, 22.18 and 18.46 present, MIGRATION.md link count 7, range line count 310; git diff --name-only | grep -c '\\.rs$' == 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "29-CI-EVIDENCE.md written with a 16-row local sweep (fmt, four clippy feature variants, full test suite, evals, both SHIP-02 proofs, 11/11 post-bump semver-checks reporting 0.9.0->0.10.0, MSRV 1.88 check, make security, check-release-consistency, cargo publish --workspace --dry-run across twelve crates in dependency order, mdbook build, and the carried 72-warning cargo-doc condition) plus the CI-run table for the newest available branch run; audit section 10 filled (Verdict: PASS) closing all ten sections with zero pending; 29-ACCEPTANCE-AUDIT.md's overall verdict updated"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "cargo publish --workspace --dry-run -> exit 0, 12 crates verified in dependency order (paladin-ai-core..paladin-ai), paladin-doc-examples correctly absent from output; mdbook build docs/ -> exit 0, No broken links found; grep -c 'Verdict: pending' .project/v0.10.0/09-program-acceptance-audit.md == 0; grep -c '^- \\[x\\]' same file == 0; git tag -l v0.10.0 | wc -l == 0"
        status: pass
    human_judgment: false

# Metrics
duration: ~10.5h (wall clock, dominated by repeated cargo build/test/clippy/semver-checks/publish-dry-run compilation in this devcontainer)
completed: 2026-09-10
status: complete
---

# Phase 29 Plan 09: Cut the v0.10.0 Release Commit Summary

**Bumped all twelve publishable crates plus the doc-examples workspace member to 0.10.0 via `cargo-release`, regenerated the OpenAPI baseline, stamped and curated all twelve changelogs, ran a 16-row local release-readiness sweep including a clean twelve-crate `cargo publish --workspace --dry-run` in dependency order, and closed the program acceptance audit's tenth and final section — with no tag cut and zero `.rs` files touched.**

## Performance

- **Duration:** ~10.5h wall clock (three task commits spanning 02:03–12:30 UTC; dominated by repeated multi-minute `cargo build`/`cargo test --workspace`/four clippy feature-variant runs/eleven-crate `cargo semver-checks`/`cargo publish --workspace --dry-run`'s from-scratch compile of every crate twice/`RUSTUP_TOOLCHAIN=1.88 cargo check --all-features --all-targets` in this devcontainer, not idle time)
- **Tasks:** 3
- **Files modified:** 30 (15 manifests/lockfile/openapi in Task 1, 12 changelogs in Task 2, 3 docs artefacts — 1 new, 2 modified — in Task 3)

## Accomplishments

- **Task 1 — version bump:** `cargo release version 0.10.0 --execute --no-confirm --workspace` moved `workspace.package.version`, all twelve publishable crates' own `version` fields, `paladin-doc-examples`, and every intra-workspace path-dependency pin to `0.10.0`; `UPDATE_OPENAPI=1 cargo test -p paladin-web openapi_matches_committed_baseline --quiet` regenerated `crates/paladin-web/openapi.json`'s `info.version` with a 1-line diff. Verified the bump's reach directly rather than trusting it: every publishable crate's `version = "0.10.0"` confirmed by grep, zero non-comment `0.9.0` lines remain in any `Cargo.toml`, `.github/workflows/ci.yml`'s pinned `baseline-version 0.9.0` occurrences untouched, root `Cargo.toml`'s schemars comment untouched, `cargo build --workspace` clean, both SHIP-02 proofs re-run green (`v0_9_config_boot` 9/9, `openapi_golden_v0_9` 6/6), no tag created.
- **Task 2 — changelog stamping and curation:** `make finalize-crate-changelogs VERSION=0.10.0` stamped a dated `## [0.10.0] - 2026-09-10` section into all twelve publishable packages' changelogs with `[Unreleased]` preserved and empty. Discovered the script's insertion point (directly after the `[Unreleased]` anchor) already relocated every crate's pre-existing Unreleased content under the new heading as a structural side effect — confirmed by diff before doing further manual work. Curated the root `CHANGELOG.md`'s 270-line moved body for a consumer: added a "Behavioral changes" sub-list leading the section (M-B-01 through M-B-04, each linking `MIGRATION.md` §9.1), preserved the existing grouped Added/Changed/Fixed content (reorganised, not rewritten), and added a new "Known limitations" section carrying the D-16 tracing-overhead deviation (+22.18% log sink / +18.46% composite, ACCEPTED per Phase 28 maintainer sign-off) — completing the third of the D-16 record's four cross-referenced artefacts. `crates/paladin-eval/CHANGELOG.md`'s "Initial release" content now sits under its own `[0.10.0]` section. `./scripts/check-release-consistency.sh --tag v0.10.0` confirmed green.
- **Task 3 — release-readiness evidence and audit close-out:** Ran the full D-23 local sweep (16 numbered rows: `cargo fmt --all --check`; four `cargo clippy --workspace --all-targets -- -D warnings` variants — default, `otel`, `dev-ui`, `web-server`; `cargo test --workspace`; `cargo test --test evals`; both SHIP-02 proofs; `cargo semver-checks check-release` for all eleven baseline crates against the *actual bumped tree*, each now reporting `v0.9.0 -> v0.10.0` with `Summary no semver update required`; `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets --locked`; `make security`; `check-release-consistency.sh`; `cargo publish --workspace --dry-run`; `mdbook build docs/`; and the re-measured 72-warning `cargo doc --workspace --no-deps` carried condition), recorded in a new `29-CI-EVIDENCE.md` with a CI-run table for the newest available `feature/phase-26` workflow run (honestly noting this branch has never been pushed past a pre-Phase-29 commit). Filled audit section 10 (all crates at `0.10.0`, changelogs updated, dry-run publish green across a non-empty twelve-crate set in the real dependency order, `MIGRATION.md` carries zero `TBD`, post-bump semver re-run recorded explicitly) — `Verdict: PASS`, closing all ten sections with zero pending verdicts. Updated `29-ACCEPTANCE-AUDIT.md`'s overall verdict to `PASS with findings`, maintainer sign-off boxes left unticked.

## Task Commits

1. **Task 1: Bump every crate to 0.10.0, regenerate the OpenAPI baseline, and prove nothing else moved** — `83d219f1` (chore)
2. **Task 2: Stamp every changelog and curate the root release notes for a consumer** — `3019ed8e` (docs)
3. **Task 3: Run the dry-run publish, record the evidence, and close audit section 10** — `3f6eed2a` (docs)

**Plan metadata:** committed as part of this SUMMARY (see final commit).

## Files Created/Modified

- `Cargo.toml`, `Cargo.lock`, twelve crate `Cargo.toml` files (`paladin-core`, `paladin-ports`, `paladin-herald`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-storage`, `paladin-notifications`, `paladin-content`, `paladin-web`, `paladin-eval`, `doc-examples`) — version fields bumped to `0.10.0`, every intra-workspace pin moved.
- `crates/paladin-web/openapi.json` — `info.version` regenerated to `0.10.0` (1-line diff).
- `CHANGELOG.md` plus eleven per-crate `CHANGELOG.md` files — dated `## [0.10.0]` sections stamped; root curated with Behavioral changes and Known limitations sections.
- `.planning/phases/29-program-gates-release/29-CI-EVIDENCE.md` (new) — 16-row local sweep plus the CI-run table.
- `.project/v0.10.0/09-program-acceptance-audit.md` — section 10 filled, `Verdict: PASS`.
- `.planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md` — overall verdict updated to `PASS with findings`.

## Decisions Made

See `key-decisions` in the frontmatter above — summarized: (1) two additional comment-only `0.9.0` occurrences beyond the plan's named exception, confirmed non-pin and recorded as a precision finding; (2) `ci.yml`'s literal `baseline-version 0.9.0` substring count measured 3, not the plan's stated 5, also recorded as a precision finding with no functional impact (the file is untouched by this plan either way); (3) the changelog-stamping script's insertion point already accomplished the "move entries" instruction as a structural side effect, verified rather than redundantly re-done; (4) a `cargo test --workspace` run's 3 transient timeouts (pre-existing async tests, devcontainer load) recorded honestly with the clean re-run's result rather than silently retried with no note; (5) this branch's stale push state (no CI run for any Phase 29 commit) recorded honestly in the CI-run table rather than fabricated or omitted; (6) the D-16 deviation's four-artefact cross-reference reaches 3 of 4 in this plan's own file scope, with the fourth (WINDOWS.md) already closed by plan 29-08 under the orchestrator's file-scope prohibition for this plan.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — plan-text precision] Two additional comment-only `0.9.0` occurrences exist beyond the plan's named exception**
- **Found during:** Task 1's reach-verification step
- **Issue:** The plan's `<interfaces>` text names only root `Cargo.toml:150`'s schemars comment as a sanctioned non-pin `0.9.0` exception. `crates/paladin-battalion/Cargo.toml:44` (a mirrored schemars-transitive-version note) and `crates/paladin-web/Cargo.toml:70` (a `struct_marked_non_exhaustive` suppression comment citing "the v0.9.0 baseline" historically) also survive the bump.
- **Fix:** Read both in full surrounding context, confirmed neither is a version pin (`cargo-release` correctly left them alone — it only rewrites `version =` and dependency-version fields), and recorded both as a precision finding in audit section 10 rather than editing comment prose that was never in scope.
- **Files modified:** none (finding recorded in `.project/v0.10.0/09-program-acceptance-audit.md` only)
- **Verification:** `grep -rn '0\.9\.0' --include=Cargo.toml . | grep -v '#'` (non-comment lines) returns zero results across the entire workspace.
- **Committed in:** `83d219f1` (bump commit, precision note recorded in `3f6eed2a`'s audit section 10 edit)

**2. [Rule 1 — plan-text precision] `ci.yml`'s literal `baseline-version 0.9.0` substring count measured 3, not the plan's stated 5**
- **Found during:** Task 1's reach-verification step
- **Issue:** The plan's acceptance criteria assert `grep -c 'baseline-version 0.9.0' .github/workflows/ci.yml` equals 5 (citing lines 276, 304, 334, 339, 357). Direct measurement returns 3 (lines 276, 334, 357) — lines 304 and 339 carry `v0.9.0` in adjacent job-name/comment prose (`Semver Checks (vs v0.9.0)`, `Check semver against the published v0.9.0 baseline`) without the exact substring `baseline-version 0.9.0`.
- **Fix:** Recorded the measured count and the exact discrepancy in audit section 10's Findings list, following the same house pattern `29-04-SUMMARY.md`/`29-07-SUMMARY.md` established for their own measured-vs-planning-time counts. The functional claim (this file is untouched by any commit in this plan) holds regardless of which literal count applies — confirmed via `git diff --stat` across all three task commits.
- **Files modified:** none (finding recorded in audit section 10 only)
- **Verification:** `grep -c 'baseline-version 0.9.0' .github/workflows/ci.yml` → 3; `git diff --stat 5e0c979a..HEAD -- .github/workflows/ci.yml` → empty.
- **Committed in:** `3f6eed2a` (audit section 10)

**3. [Rule 1 — environment flakiness, not a bug] Three pre-existing async tests hit transient 30s timeouts under devcontainer load**
- **Found during:** Task 3's local sweep, first `cargo test --workspace` run
- **Issue:** `application::services::run::cancel_tests::cross_instance_cancel_probe`, `cancel_tests::local_cancel_signals_token`, and `stream_tests::degraded_stream_follows_remote_progress` each panicked with a 30s-budget `Elapsed(())` — timing-sensitive async tests racing under this devcontainer's concurrent-build/test load (immediately following a multi-minute `RUSTUP_TOOLCHAIN=1.88 cargo check` run). None of these tests, or any file they touch, is modified by this plan (zero `.rs` files changed across all three task commits).
- **Fix:** Re-ran the three failing tests individually with `--test-threads=1` — all three passed. Re-ran the full `cargo test --workspace` suite cleanly — 0 failed across every crate (paladin-ai lib: 974 passed). Recorded both the transient failure and the clean re-run honestly in `29-CI-EVIDENCE.md` row 6 rather than omitting the first run or silently retrying with no note.
- **Files modified:** none
- **Verification:** `cargo test -p paladin-ai --lib cancel_tests:: -- --test-threads=1` → 3 passed; `cargo test -p paladin-ai --lib stream_tests::degraded_stream_follows_remote_progress -- --test-threads=1` → 1 passed; full `cargo test --workspace` re-run → 0 failed workspace-wide.
- **Committed in:** N/A (no file changed; recorded as evidence in `3f6eed2a`)

---

**Total deviations:** 3 (all Rule 1 — plan-text precision findings and an honestly-recorded environment-flakiness observation, zero production or planning-artefact edits beyond the recorded findings text itself).
**Impact on plan:** None on the deliverable's correctness. All three are documented so a future reader understands why measured counts differ from plan text and why one test run showed transient failures that a clean re-run did not reproduce.

## Threat Flags

None. This plan adds no new attack surface — it is release mechanics (a version bump, changelog stamping, a dry-run publish that never uploads, and documentation) over already-shipped, already-security-reviewed code. Every command run was either read-only/dry-run (`cargo publish --dry-run` never uploads; `cargo semver-checks`, `cargo audit`, `cargo deny check`, `mdbook build` are all read-only analyses) or a scoped, verified-reach version-string edit (`cargo-release`, confirmed to touch only version fields and pins).

## Known Stubs

None. The one carried, honestly-recorded condition (the pre-existing 72-warning `cargo doc --workspace --no-deps` red `lint`-job step) is not a stub introduced by this plan — it predates Phase 29 entirely, is explicitly out of SHIP-04's and D-25's scope, and is recorded with its exact re-measured count in `29-CI-EVIDENCE.md` row 16 and audit section 8 (filled by plan 29-07) rather than silently passed over.

## User Setup Required

None — no external service configuration required. This plan does not push, does not open a PR, and does not run `gh run watch`; those steps belong to the orchestrator.

## Next Phase Readiness

- The release commit is cut and release-ready on `feature/phase-26`: all twelve publishable crates at `0.10.0`, changelogs finalized, the dry-run publish green across a non-empty twelve-crate set in dependency order, `MIGRATION.md` carries zero `TBD`, and the program acceptance audit's ten-step protocol is closed (`Verdict: PASS` on section 10, `PASS with findings` overall) with only the seven maintainer sign-off checkboxes outstanding for a human at the phase's UAT step.
- `.planning/WINDOWS.md`'s `open_count` is 0 (closed by plan 29-08); this plan's own audit findings (two precision notes, both non-blocking, no functional impact) do not require a new WINDOWS.md row — neither is a defect, both are plan-text-vs-measured-reality corrections already fully resolved by direct measurement.
- **This branch has never been pushed past a pre-Phase-29 commit (`77912ac8`)** — no CI run exists for any of Phase 29's nine plans, and `docs.yml`'s "Build MDBook" required status check has never run for this branch (PR-gated, no PR open). The orchestrator must push this branch (or open the PR) to get the real pre-merge CI evidence that D-21 requires before `/gsd-ship` can proceed to a merge decision; `29-CI-EVIDENCE.md` names this gap explicitly rather than presenting a stale or fabricated run as current evidence.
- No tag was created (`git tag -l v0.10.0` is empty) — per D-18/ADR-0043/ADR-0044, the tag is cut from `main` after merge via `make release VERSION=0.10.0`.
- `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md` checkboxes, and `.planning/WINDOWS.md` were left untouched, per the orchestrator's explicit instruction — those transitions belong to the orchestrator after this plan completes.

## Self-Check: PASSED

- `Cargo.toml`, all twelve crate `Cargo.toml` files, `Cargo.lock`, `crates/paladin-web/openapi.json` — FOUND, modified, version `0.10.0` confirmed
- `CHANGELOG.md` and eleven per-crate `CHANGELOG.md` files — FOUND, modified, dated `## [0.10.0]` sections confirmed
- `.planning/phases/29-program-gates-release/29-CI-EVIDENCE.md` — FOUND, created
- `.project/v0.10.0/09-program-acceptance-audit.md` — FOUND, modified, section 10 `Verdict: PASS`
- `.planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md` — FOUND, modified, overall verdict updated
- Commit `83d219f1` (Task 1) — FOUND in `git log --oneline`
- Commit `3019ed8e` (Task 2) — FOUND in `git log --oneline`
- Commit `3f6eed2a` (Task 3) — FOUND in `git log --oneline`
- `cargo publish --workspace --dry-run` → exit 0, 12 crates verified in dependency order, `paladin-doc-examples` correctly absent — CONFIRMED (re-run at end of Task 3)
- `./scripts/check-release-consistency.sh --tag v0.10.0` → exit 0 — CONFIRMED (re-run at end of Task 3)
- `mdbook build docs/` → exit 0, "No broken links found" — CONFIRMED (re-run at end of Task 3)
- `grep -c 'Verdict: pending' .project/v0.10.0/09-program-acceptance-audit.md` → 0 — CONFIRMED
- `grep -c '^- \[x\]' .project/v0.10.0/09-program-acceptance-audit.md` → 0 — CONFIRMED
- `git tag -l v0.10.0 | wc -l` → 0 — CONFIRMED
- `git diff --name-only 5e0c979a..HEAD | grep -c '\.rs$'` → 0 — CONFIRMED
- `git status --short` → clean working tree — CONFIRMED

---
*Phase: 29-program-gates-release*
*Completed: 2026-09-10*
