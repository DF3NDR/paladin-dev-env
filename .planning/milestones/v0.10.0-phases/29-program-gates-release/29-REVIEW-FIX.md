---
phase: 29-program-gates-release
fixed_at: 2026-09-10T12:55:00Z
review_path: .planning/phases/29-program-gates-release/29-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 29: Code Review Fix Report

**Fixed at:** 2026-09-10T12:55:00Z
**Source review:** .planning/phases/29-program-gates-release/29-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (fix_scope = critical_warning; WR-01, WR-02, WR-03. IN-01 was out of
  scope and not attempted.)
- Fixed: 3
- Skipped: 0

## Fixed Issues

### WR-01: Ten of eleven per-crate CHANGELOG.md files have an empty `[0.10.0]` section despite substantial in-crate changes

**Files modified:** `crates/paladin-core/CHANGELOG.md`, `crates/paladin-ports/CHANGELOG.md`,
`crates/paladin-battalion/CHANGELOG.md`, `crates/paladin-llm/CHANGELOG.md`,
`crates/paladin-memory/CHANGELOG.md`, `crates/paladin-web/CHANGELOG.md`,
`crates/paladin-storage/CHANGELOG.md`, `crates/paladin-content/CHANGELOG.md`,
`crates/paladin-herald/CHANGELOG.md`, `crates/paladin-notifications/CHANGELOG.md`
**Commit:** a30e1ce1
**Applied fix:** Populated each crate's `## [0.10.0]` section (which previously contained only
the dated heading, per the review's own `sed` reproduction). For the seven crates with
substantial changes attributed to them in `MIGRATION.md` §9.2/§9.4/§9.5 (`paladin-core`,
`paladin-ports`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-web`,
`paladin-storage`), wrote Keep-a-Changelog `### Added`/`### Changed`/`### Fixed` entries citing
the specific type/method/table changes, their mitigation, and requirement IDs, cross-checked
against the §9.2 register rows and (for `paladin-storage`, `paladin-memory`, `paladin-llm`,
which have no dedicated §9.2 rows) `git log --oneline v0.9.0..HEAD -- crates/<crate>/`. For the
three crates with no functional change since `v0.9.0` (`paladin-content`, `paladin-herald`,
`paladin-notifications` — confirmed via the same `git log` command, whose only hits are the
workspace-wide MSRV-bump commits touching no `src/` file), wrote the single
"No functional changes; lockstep version bump" line the review's own fix guidance specifies.
Re-ran `./scripts/check-release-consistency.sh --tag v0.10.0` afterward:
`✅ OK: 12 publishable package(s) checked, all match tag version '0.10.0' with a changelog
section for it.` No `.rs`/`Cargo.toml`/`MIGRATION.md`/allowlist file was touched (X-03/D-12
boundary respected).

### WR-02: `docs/src/appendix/release-checklist.md` §6 "Publish" list omits `paladin-eval`

**Files modified:** `docs/src/appendix/release-checklist.md`
**Commit:** 6891b77c
**Applied fix:** Added `paladin-eval` as item 5 in §6's ordered publish list, immediately before
`paladin-ai` (renumbering `paladin-ai` to item 6), mirroring §5's already-correct list six lines
above and `scripts/publish-crates.sh`'s actual `CRATES=(...)` array (confirmed by grep: `paladin-eval`
sits immediately before `paladin-ai` in that array). Ran `mdbook-mermaid install docs/` (no
generated assets left in `git status`) then `mdbook build docs/`: `mdbook_linkcheck` reported
"No broken links found."

### WR-03: `v0_9_config_boot`'s new "fewer than 9" CI guard step doesn't defend against its own zero-match edge case

**File:** `.github/workflows/ci.yml`
**Commit:** 59e4e5f8
**Applied fix:** Wrapped the `grep -oP ... | tail -1` pipeline feeding `ACTUAL=` in the same
`{ grep ... || test $? -eq 1; }` idiom the file's own §9.2/allowlist set-equality step already
uses (tolerates exit 1 = no match, still fails on exit 2 = a real error), so a zero-match grep no
longer aborts the script under GitHub Actions' default `bash -eo pipefail` before the step's own
`::error::` diagnostic prints. Scope was limited to the step named in the finding's **File**
line — the step "added in this phase's diff" — per the X-03/D-12 boundary; the pre-existing
sibling `e2e-platform-api` "Fail if the run selected zero tests" step the finding notes has the
identical gap predates this phase's diff and was left untouched, matching this run's scoped
instructions. Verified: `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/ci.yml"))'`
parses clean; the guard's shell body was extracted and run standalone against two log fixtures
— a 9-test `test result: ok. 9 passed...` fixture (`Passed: 9`, `GUARD PASSED`, exit 0) and a
zero-match fixture with no `test result:` line (`Passed: 0`, `::error::...` printed, exit 1) —
proving both branches now behave as intended.

## Skipped Issues

None — all in-scope findings were fixed.

---

_Fixed: 2026-09-10T12:55:00Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
