---
phase: 24-pause-resume-history-graceful-shutdown
plan: 13
subsystem: docs
tags: [documentation, openapi, authorization, changelog, mdbook, gap-closure]

# Dependency graph
requires:
  - phase: 24-12
    provides: "CR-01/CR-02 code review fixes, including the require_admin gate on resume_thread"
provides:
  - "MIGRATION.md §9.6 resume row lists 403 with a note naming require_admin and PLAT-06"
  - "CHANGELOG.md Unreleased HITL-05 entry states the admin-gated resume posture"
  - "docs/src/user-guides/parley-and-chronicle.md HTTP-surface section describes the shipped posture"
  - "openapi::tests::openapi_thread_paths_document_every_status asserts 403 on the resume path"
affects: [24-VERIFICATION, phase-24-close]

# Tech tracking
tech-stack:
  added: []
  patterns: ["doc-drift closed via test-backed status-code assertion (mechanical self-guard)"]

key-files:
  created: []
  modified:
    - MIGRATION.md
    - CHANGELOG.md
    - docs/src/user-guides/parley-and-chronicle.md
    - crates/paladin-web/src/openapi.rs

key-decisions:
  - "No new CHANGELOG Security subsection opened — these routes never appeared in a tagged release, so this is the initial admin-gated shape of a new endpoint, not a security regression to shipped behavior"
  - "Only the test module in openapi.rs was touched (one status literal + rustdoc sentence); no production code path changed"

patterns-established: []

requirements-completed: [HITL-05]

coverage:
  - id: D1
    description: "MIGRATION.md §9.6 resume status-code row lists 403 in ascending order, with a note naming require_admin and PLAT-06"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "grep '^| `POST` | `/v1/threads/{id}/resume` |' MIGRATION.md | grep -c '`403`' -> 1"
        status: pass
      - kind: unit
        ref: "cargo test -p paladin-web --all-features --lib openapi::tests::openapi_thread_paths_document_every_status"
        status: pass
    human_judgment: false
  - id: D2
    description: "CHANGELOG.md Unreleased HITL-05 bullet states the admin requirement, the 403, the any-role reads, and PLAT-06, with no new Security subsection"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "sed -n '/^- \\*\\*Threads over HTTP (HITL-05)/,/^- \\|^## /p' CHANGELOG.md | grep -c '403' -> 1"
        status: pass
      - kind: unit
        ref: "git diff --stat CHANGELOG.md (confined to [Unreleased] section, no new ### Security)"
        status: pass
    human_judgment: false
  - id: D3
    description: "mdBook user guide's HTTP-surface intro, route table, and Interim authorization posture blockquote describe the shipped admin-gated posture; secret-in-payload blockquote and SUMMARY.md untouched"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "mdbook build docs/ -> exit 0, 'No broken links found'"
        status: pass
      - kind: unit
        ref: "! grep -q 'no admin/writer scope distinction' docs/src/user-guides/parley-and-chronicle.md -> exit 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "openapi_thread_paths_document_every_status asserts 403 on the resume path; shipped require_admin gate and the three role tests remain unchanged"
    requirement: "HITL-05"
    verification:
      - kind: unit
        ref: "cargo test -p paladin-web --all-features --lib thread_controller::tests::post_resume_with_non_admin_role_is_403 (+2 more) -> 3 passed"
        status: pass
      - kind: unit
        ref: "cargo clippy -p paladin-web --all-targets --all-features -- -D warnings -> clean"
        status: pass
    human_judgment: false

duration: ~25min
completed: 2026-09-05
status: complete
---

# Phase 24 Plan 13: Post-CR-01 Documentation Drift Gap Closure Summary

**Closed 24-VERIFICATION.md's single gap by making MIGRATION.md, CHANGELOG.md, and the mdBook user guide state the shipped admin-gated `POST /v1/threads/{id}/resume` posture (403 for non-admin callers), and adding a test-backed guard so the status set cannot silently drift again.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-09-05T11:27:00Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- `MIGRATION.md` §9.6's resume status-code row now lists `403` between `401` and `404`, plus a new paragraph naming `crate::agent_auth::require_admin`, CR-01/commit `00b1e552`, and PLAT-06 (Phase 27) as the successor scope work
- `CHANGELOG.md`'s `[Unreleased]` → `### Added` → HITL-05 bullet now states the admin-role requirement for resume (`403` for non-admin), that the two read routes stay any-authenticated-role, and cross-references `MIGRATION.md` §9.6 — no new `### Security` subsection opened since these routes have never shipped in a tagged release
- `docs/src/user-guides/parley-and-chronicle.md`'s "The HTTP Surface" section (intro sentence, resume route-table row, and the "Interim authorization posture" blockquote) now describes the shipped posture: admin-gated resume with `403`, any-authenticated-role reads, PLAT-06 named as successor
- `crates/paladin-web/src/openapi.rs`'s `openapi_thread_paths_document_every_status` test now asserts `403` on the resume path's status set, so a future `UPDATE_OPENAPI=1` regeneration that drops the admin gate's documented status fails a test instead of drifting silently

## Task Commits

Each task was committed atomically:

1. **Task 1: MIGRATION.md §9.6 registers the 403, and the openapi drift guard asserts it** - `e462fcb0` (docs)
2. **Task 2: CHANGELOG.md's Unreleased HITL-05 entry states the shipped posture** - `8838795a` (docs)
3. **Task 3: the mdBook user guide describes the as-shipped authorization posture** - `7a1b1f24` (docs)

_Note: no code behavior changed in this plan — every commit is a documentation or test-assertion edit (Rust convention: `docs` type used throughout, matching the plan's "no production code path changes" scope note)._

## Files Created/Modified

- `MIGRATION.md` - §9.6 resume row gains `403`; new paragraph names `require_admin`, CR-01/`00b1e552`, PLAT-06
- `CHANGELOG.md` - `[Unreleased]` HITL-05 bullet amended with the admin-gate statement and a §9.6 cross-reference
- `docs/src/user-guides/parley-and-chronicle.md` - HTTP-surface intro, resume route row, and "Interim authorization posture" blockquote rewritten to match the shipped posture
- `crates/paladin-web/src/openapi.rs` - test-only: `403` added to the resume status assertion array in `openapi_thread_paths_document_every_status`, plus its rustdoc sentence updated

## Decisions Made

- Kept the CHANGELOG amendment inside the existing HITL-05 bullet rather than opening a `### Security` subsection: these routes have never appeared in a tagged release, so relative to every released version this is the initial admin-gated shape of a new endpoint, not a security change to shipped behavior.
- Confined the `openapi.rs` edit strictly to the `#[cfg(test)] mod tests` module (one status literal plus one doc-comment sentence) — no production code path in `paladin-web` changes, verified by `git diff --stat` showing only the test module touched.

## Deviations from Plan

None - plan executed exactly as written. All four edits (three docs + one test assertion) matched the plan's `<action>` blocks precisely; no Rule 1-4 auto-fixes were needed.

One environment note (not a deviation from the plan's content, but worth recording): `mdbook build docs/` initially failed with "Unable to copy `mermaid.min.js`" because that generated asset (gitignored, per `.gitignore:21-22`) was not yet present in this fresh worktree. Ran `mdbook-mermaid install docs/` (the project's own documented setup step for that preprocessor) to regenerate it, then re-ran the build successfully. No tracked file changed as a result — `git status --short` confirms only `docs/src/user-guides/parley-and-chronicle.md` is modified.

## Issues Encountered

None beyond the mdbook-mermaid asset regeneration noted above, which is standard local setup and not a code or content issue.

## Gate Evidence (all commands actually run this session)

1. `grep '^| `POST` | `/v1/threads/{id}/resume` |' MIGRATION.md | grep -c '`403`'` → `1`
2. `sed -n '/^- \*\*Threads over HTTP (HITL-05)/,/^- \|^## /p' CHANGELOG.md | grep -c '403'` → `1`
3. `! grep -q 'no admin/writer scope distinction' docs/src/user-guides/parley-and-chronicle.md` → exit `0`
4. `mdbook build docs/` → exit `0`, ended with `[INFO] (mdbook_linkcheck) No broken links found`
5. `cargo test -p paladin-web --all-features --lib openapi::tests` → `8 passed; 0 failed` (includes `openapi_lists_the_three_thread_paths`, `openapi_pre_existing_agent_paths_are_unchanged`, `openapi_matches_committed_baseline`, `openapi_thread_paths_document_every_status`)
6. Control-unchanged guard: `grep -n 'require_admin' crates/paladin-web/src/thread_controller.rs` shows the gate still applied at `resume_thread` (line 554, module rustdoc line 40); `cargo test -p paladin-web --all-features --lib -- thread_controller::tests::post_resume_with_non_admin_role_is_403 thread_controller::tests::post_resume_with_admin_role_is_202 thread_controller::tests::get_thread_state_with_non_admin_role_is_200_not_403` → `3 passed; 0 failed`
7. `cargo fmt --all --check` → clean, no output; `cargo clippy --workspace --all-targets --features web-server -- -D warnings` → clean (`Finished` with no warnings)
8. `git diff --stat 6af45000e952194ce7729da6b0683b0010314ad2 HEAD` → exactly the four `files_modified`: `MIGRATION.md`, `CHANGELOG.md`, `docs/src/user-guides/parley-and-chronicle.md`, `crates/paladin-web/src/openapi.rs`

Additionally ran, as part of Task 1's acceptance criteria: `cargo clippy -p paladin-web --all-targets --all-features -- -D warnings` → clean, and `git diff --stat crates/paladin-web/src/openapi.rs` → confirmed only the test module touched (1 file, 6 insertions, 5 deletions).

## Known Stubs

None.

## Threat Flags

None. This plan introduces no new network surface, auth path, file access pattern, or schema change — it restates the already-shipped `require_admin` posture in documentation and adds a test-only assertion.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

24-VERIFICATION.md's single gap is now closed: all three named documentation artifacts (MIGRATION.md §9.6, CHANGELOG.md, the mdBook guide) state the as-shipped authorization posture for `POST /v1/threads/{id}/resume`, and the openapi drift-guard test mechanically enforces agreement between the MIGRATION §9.6 registry and `openapi.json`'s resume response set. The shipped admin gate and its three role tests are provably unchanged (T-24-13-02 threat mitigation confirmed). No blockers for phase 24 close.

---
*Phase: 24-pause-resume-history-graceful-shutdown*
*Completed: 2026-09-05*
