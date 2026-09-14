---
phase: 29-program-gates-release
plan: 07
subsystem: docs
tags: [audit, compatibility, msrv, semver, ship-03, deviation, sign-off]

# Dependency graph
requires:
  - phase: 29-04
    provides: "The audit document's ten-section skeleton and sections 1-5 (per-FR table, X-rule spot-checks, E2E re-runs, BUG-01/02 re-verification, orphan-behavior + ubiquitous-language) this plan continues sections 6-9 of"
  - phase: 29-05
    provides: "MIGRATION.md closed (zero placeholder markers), the v0_9_config_boot and openapi_golden_v0_9 CI gates this plan's section 9 cites by name and re-runs"
provides:
  - "Audit sections 6-9 filled: the compatibility audit (semver-checks + a stated-limitation manual API diff, 26/10/9 row reconciliation, 6 deliberate-zero notes confirmed), the behavioral-change audit (M-B-01..04 confirmed exhaustive, M-B-04's ENG-08 provenance cited), the toolchain audit (1.88 three-way agreement, MSRV job re-run, cargo deny/audit green, the 72-warning cargo doc condition recorded as carried), and config-compat test existence (both SHIP-02 proofs re-run green, their CI gates cited)"
  - "The Phase 28 tracing-overhead bar (PRD 07 acceptance 6) recorded as an ACCEPTED v0.10.0 deviation per D-16, citing the STATE.md D-37 maintainer sign-off already on record, cross-referencing the three other artefacts that carry the same record"
  - "A seven-item maintainer sign-off section (six judgment-tier prohibitions + the M-B-04 countersignature), every box left unticked per D-17"
  - "docs/src/operations/observability.md's Known limitations overhead bullet extended with the v0.10.0 disposition sentence"
affects: ["29-08 (WINDOWS.md triage — this plan proposes but does not write the cargo-doc-72-warnings deviation row)", "29-09 (release readiness section 10, and the phase's UAT step that ticks the seven sign-off boxes this plan leaves unticked)"]

# Tech tracking
tech-stack:
  added: []
  patterns: ["dual-evidence compatibility check (cargo semver-checks per-crate + a manual API-surface diff), each tool's own scope/limitation stated explicitly rather than implied", "cite-and-prove-current instead of re-running a multi-minute cold sweep (git diff --stat against the last measured commit proves nothing changed since)", "record-a-discrepancy-rather-than-force-a-number (continuing 29-04/29-05's house pattern for planning-time vs measured counts)"]

key-files:
  created: []
  modified:
    - .project/v0.10.0/09-program-acceptance-audit.md
    - docs/src/operations/observability.md

key-decisions:
  - "The public-API diff mechanism (.project/current-exports.txt) is generated from the root paladin facade crate only and does not expand enum variants of re-exported leaf-crate types -- confirmed directly (StopReason appears only as a bare `pub use` re-export in both v0.9.0 and HEAD, no variants listed either version). Stated as an explicit methodology limitation in section 6 rather than silently treating the manual diff as authoritative for enum-variant/field-level changes; cargo semver-checks (which builds and diffs each crate's own rustdoc JSON) is the tool that actually closes that direction, which is why D-05 requires both together."
  - "For the ten remaining baseline crates beyond paladin-ai-core (which this plan's own <verify> block requires running live), cargo semver-checks was NOT re-run cold -- the repo's own operating rules name the 30+ minute cold-build cost of an eleven-crate sweep against a published baseline. Instead, Phase 28's 11/11 CI evidence (28-CI-EVIDENCE.md row 11) is cited and proven CURRENT, not stale, by a direct git diff --stat showing zero non-test .rs files and only a single new [[test]] block changed between Phase 28's close commit (25a0eaaf) and this HEAD."
  - "Measured 6 'deliberate zero' note paragraphs in MIGRATION.md Sec9.2 (Plan 22-01, Phases 23/24/25/26/28), not the 'eight' this plan's own <read_first> text names. Recorded as a measured-vs-planning-time discrepancy in the same house style 29-04-SUMMARY.md already established for its own FR-count and orphan-behavior-count findings -- every named type in all six notes was independently grep-confirmed absent from the v0.9.0 export snapshot; no note is missing content."
  - "The pre-existing 72-warning `cargo doc --workspace --no-deps` red lint-job condition is recorded in section 8 with a proposed WINDOWS.md deviation-row disposition, but the row itself is NOT written here -- this plan is isolated from WINDOWS.md per the orchestrator's explicit instruction; plan 29-08 owns writing it."
  - "The D-16 tracing-overhead deviation is recorded as ACCEPTED citing STATE.md's own D-37 line ('accepted by maintainer sign-off at close-out UAT, 2026-09-09') rather than re-asking the question 28-VERIFICATION.md's human_verification item 1 originally posed -- the maintainer decision already happened once, at Phase 28 close, and this audit cites it rather than re-litigating it."
  - "The six judgment-tier prohibition boxes and the M-B-04 countersignature box (seven total) are authored unchecked, per D-17 -- an agent's code-level inspection is recorded as supporting evidence in 28-VERIFICATION.md but is explicitly non-authoritative; this executor does not tick them under any circumstance."

patterns-established:
  - "When a manual evidence-gathering mechanism (a facade-only API-surface snapshot) has a real scope gap relative to what a section's protocol text asks it to prove, state the gap explicitly and name which OTHER tool actually closes it, rather than letting the manual diff's presence imply completeness it does not have."

requirements-completed: [SHIP-03]

coverage:
  - id: D1
    description: "Section 6 reconciles §9.2's 26 data rows (10 Y-marked, 9 distinct pairs) against cargo semver-checks output and the v0.9.0 public-API diff in both directions, confirms all 6 deliberate-zero notes by absence, and states the trait required-method conclusion as its own sentence"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0 -> Summary no semver update required (live); awk section-6 range checks for current-exports.txt, ADR-0048, 'required method', zero 'Verdict: pending' all pass"
        status: pass
    human_judgment: false
  - id: D2
    description: "Section 7 confirms MIGRATION.md §9.1 holds exactly M-B-01..04 and cites the Phase 22 ENG-08 decision for M-B-04's provenance"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "grep -c '| M-B-0' MIGRATION.md == 4; awk section-7 range grep -cE 'M-B-0[1-4]' >= 4 and cites ENG-08"
        status: pass
    human_judgment: false
  - id: D3
    description: "Section 8 records the MSRV job's toolchain/result (re-run locally), the 1.88 three-way agreement, cargo tree showing no new heavyweight default dependency, cargo deny/cargo audit green, and the re-measured 72-warning cargo doc condition as a carried, out-of-scope, non-fixed-here deviation"
    requirement: SHIP-03
    verification:
      - kind: integration
        ref: "RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets --locked -> Finished, 0 errors; cargo deny check -> advisories ok, bans ok, licenses ok, sources ok; cargo audit -> warning: 10 allowed warnings found, exit 0; cargo doc --workspace --no-deps -> 72 warning: lines"
        status: pass
    human_judgment: false
  - id: D4
    description: "Section 9 re-runs both SHIP-02 proofs (v0_9_config_boot, openapi_golden_v0_9) live, cites their observed pass counts and the CI jobs that gate them"
    requirement: SHIP-03
    verification:
      - kind: integration
        ref: "cargo test --features web-server --test v0_9_config_boot -> 9 passed, 0 failed; cargo test -p paladin-web --test openapi_golden_v0_9 -> 6 passed, 0 failed"
        status: pass
    human_judgment: false
  - id: D5
    description: "The D-16 overhead deviation is recorded as ACCEPTED with the reason and follow-up, citing STATE.md's own D-37 sign-off; docs/src/operations/observability.md gains the v0.10.0 disposition sentence beside the existing figures; the seven-item maintainer sign-off section is authored fully unchecked"
    requirement: SHIP-03
    verification:
      - kind: other
        ref: "grep -c 22.18 / 18.46 on both files >= 1; grep -c '^- \\[ \\]' audit doc == 7; grep -c '^- \\[x\\]' audit doc == 0; mdbook build docs/ exits 0"
        status: pass
    human_judgment: false

# Metrics
duration: ~1.5h
completed: 2026-09-10
status: complete
---

# Phase 29 Plan 07: Fill Audit Sections 6-9, Record the D-16 Deviation and Sign-off Summary

**Filled the program acceptance audit's compatibility, behavioral-change, toolchain and config-compat sections with re-run evidence (a live `cargo semver-checks` run plus a stated-limitation manual API diff for section 6, a re-run MSRV/deny/audit sweep for section 8, both SHIP-02 proofs re-run green for section 9), recorded the Phase 28 tracing-overhead FAIL as an ACCEPTED v0.10.0 deviation citing the maintainer sign-off already on record, and authored a seven-item maintainer sign-off section fully unticked.**

## Performance

- **Duration:** ~1.5h
- **Tasks:** 2
- **Files modified:** 2 (both existing)

## Accomplishments

- **Section 6 (compatibility audit):** ran `cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0` live (`Summary no semver update required`, 193 checks); cited Phase 28's 11/11 CI evidence for the other ten crates and proved it current via `git diff --stat 25a0eaaf..HEAD -- '*.rs'` (empty) and the Cargo.toml manifest diff (one new `[[test]]` block only). Diffed `.project/current-exports.txt` (7901 lines, 3936 `pub `-prefixed items) against `git show v0.9.0:.project/current-exports.txt` (3550 lines) — 4353 lines added, 2 removed (both header/count comment lines, zero actual API-item removals). Stated an honest methodology limitation: the snapshot is facade-only and does not expand enum variants of re-exported leaf-crate types (confirmed: `StopReason` appears only as a bare `pub use` line in both versions, no variants listed). Reconciled §9.2's 26 data rows / 10 Y-marked rows / 9 distinct pairs against the allowlist's 9 entries and the four per-crate `Cargo.toml` lint suppressions, both directions. Confirmed the 6 (not the plan text's stated 8) "deliberate zero" note paragraphs by grepping every named type's absence from the v0.9.0 export. Stated, as its own sentence, that no pre-existing public trait gained a required method (`PaladinPort`'s two new methods are both default methods, `N`, not `Y`).
- **Section 7 (behavioral-change audit):** confirmed `MIGRATION.md` §9.1 holds exactly M-B-01 through M-B-04 and no fifth entry (`grep -c '| M-B-0'` == 4); cited Phase 22's `ENG-08` as M-B-04's provenance (MIGRATION.md's own scope note and M-B-04's own row text) and placed the countersignature in the maintainer sign-off section rather than declaring the clause closed unilaterally.
- **Section 8 (toolchain audit):** confirmed the 1.88 three-way agreement (`Cargo.toml`, README badge, `MIGRATION.md` §9.3); re-ran the MSRV job's exact command locally (`RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets --locked`, `Finished` in 4m18s, 0 errors); ran `cargo tree -e no-dev,features -p paladin-ai` and confirmed zero `opentelemetry`/`tonic`/gRPC entries in the default production tree; ran `cargo deny check` (`advisories ok, bans ok, licenses ok, sources ok`) and `cargo audit` (`warning: 10 allowed warnings found`, exit 0, same four pre-existing RustSec IDs as `28-CI-EVIDENCE.md`); re-measured `cargo doc --workspace --no-deps` live (**72** `warning:` lines, matching `29-RESEARCH.md`'s live figure) and recorded it as a carried, out-of-scope condition per SHIP-04's actual requirement text and Open Question 2's resolution, proposing (not writing) a WINDOWS.md `deviation` row for plan 29-08.
- **Section 9 (config-compat test existence):** re-ran `cargo test --features web-server --test v0_9_config_boot` (9 passed, matching `29-01-SUMMARY.md`) and `cargo test -p paladin-web --test openapi_golden_v0_9` (6 passed) live; cited both tests' CI gates (`e2e-platform-api` job's two dedicated steps for the boot test; the `crate-isolation (paladin-web)` matrix leg's ordinary auto-discovery for the golden-diff test).
- **The D-16 deviation record:** added a dedicated "Accepted deviation for v0.10.0" subsection stating the bar, the measurement, the ACCEPTED disposition (citing `STATE.md`'s own D-37 line rather than re-asking a question already answered at Phase 28 close-out UAT), the defensibility reasoning (opt-in sinks, `trace.state_values` off by default, an all-Function-node microbenchmark with no LLM latency to amortise against), the follow-up (re-scope to an I/O-bound superstep), and the four artefacts that carry the same record (this audit, the published docs, the root changelog [29-09's scope], the WINDOWS.md row [29-08's scope]).
- **Maintainer sign-off:** added a seven-item unchecked-checkbox section — the six judgment-tier prohibitions from `28-VERIFICATION.md`'s `human_verification` item 2 (labels quoted verbatim, evidence per item mapped from that file's own "expected" text) plus the M-B-04 provenance countersignature — every box `- [ ]`, none ticked.
- **`docs/src/operations/observability.md`:** extended the existing overhead bullet with one sentence stating the v0.10.0 disposition, without softening or restating the already-honest measured figures.
- Zero production Rust code touched (`git diff --name-only | grep -c '\.rs$'` is 0 across both task commits); `mdbook build docs/` exits 0 after the observability edit.

## Task Commits

1. **Task 1: Fill audit section 6** — `3137b683` (docs)
2. **Task 2: Fill audit sections 7-9, record the accepted overhead deviation, and add its disposition to the observability docs** — `c9502836` (docs)

**Plan metadata:** committed as part of this SUMMARY (see final commit).

## Files Created/Modified

- `.project/v0.10.0/09-program-acceptance-audit.md` — sections 6-9 filled with re-run evidence, the "Accepted deviation for v0.10.0" subsection, and the seven-item maintainer sign-off section (all `Verdict:` lines now non-`pending` through section 9; section 10 remains `pending`, owned by plan 29-09).
- `docs/src/operations/observability.md` — the Known limitations overhead bullet gained the v0.10.0 disposition sentence.

## Decisions Made

- **The public-API diff's facade-only scope is stated as an explicit methodology limitation, not silently treated as authoritative.** `.project/current-exports.txt` is generated from the root `paladin` crate only and its `cargo-public-api` extraction does not expand enum variants of re-exported leaf-crate types — confirmed directly (`StopReason` appears in both v0.9.0 and HEAD snapshots only as a bare `pub use paladin::prelude::StopReason` re-export line, with no variant enumeration in either). `cargo semver-checks`, which builds and diffs each crate's own rustdoc JSON, is the tool that actually closes the "every pre-existing signature change has a §9.2 row" reverse direction for enum-variant/field-level changes — the section names this explicitly so a reader does not overstate what the manual diff alone proves.
- **`cargo semver-checks` was run live for `paladin-ai-core` only** (as the plan's own `<verify>` block literally requires), not for all eleven baseline crates. The repository's own operating rules name the 30+ minute cold-build cost of a full eleven-crate sweep against a published `0.9.0` baseline in this worktree. For the remaining ten, Phase 28's `28-CI-EVIDENCE.md` row 11 (`11/11 PASS`, `Summary no semver update required` for all eleven) is cited and proven current — not merely assumed — via `git diff --stat 25a0eaaf..HEAD -- '*.rs' ':!tests/**' ':!crates/*/tests/**'` (empty: zero non-test Rust files changed) and the `Cargo.toml` manifest diff (a single new `[[test]]` block, no dependency/version/feature change).
- **Measured 6 "deliberate zero" note paragraphs, not the plan's own stated 8.** `grep -n '\*\*Note on' MIGRATION.md` scoped to §9.2 returns exactly six paragraphs (Plan 22-01, Phases 23/D-27, 24/D-29, 25/D-30, 26/D-37, 28/D-39). Recorded as a measured-vs-planning-time discrepancy, in the same house style `29-04-SUMMARY.md` already established twice for its own FR-count and orphan-behavior-count findings — every named type across all six notes was independently grep-confirmed absent from the v0.9.0 export snapshot (`StateNode`, `NodeSpec`, `EngineLimits`, `EngineError`, `WarGraph`, `RunOutcome`, `NodeContext`, `NodeOutcomeKind`, `WaypointSummary`, `NodeExecutionRecord`, `WaypointStatus`, `FieldSpec`, `TraceEvent` — all zero matches), so no note lacks a genuine absence-confirmation regardless of the count discrepancy.
- **The 72-warning `cargo doc` condition is recorded with a proposed disposition, not fixed and not written to WINDOWS.md here.** This plan is isolated from `.planning/WINDOWS.md` per the orchestrator's own instruction; section 8 names the exact re-measured count (72, matching `29-RESEARCH.md`'s live figure), states SHIP-04's actual requirement text does not require it to be zero, and proposes a `deviation`-kind WINDOWS.md row for plan 29-08 to actually write.
- **The D-16 deviation cites the existing STATE.md D-37 sign-off rather than re-posing 28-VERIFICATION.md's original human_verification question.** `.planning/STATE.md`'s Phase 28 close entry already records "accepted by maintainer sign-off at close-out UAT (2026-09-09)" — this audit treats that as the closing decision and cites it, rather than re-asking a question a human already answered once.
- **All seven maintainer sign-off boxes are authored unchecked, with zero exceptions.** Per D-17, this executor does not tick a judgment-tier box under any circumstance — the tier exists precisely because an agent's verdict on these items is non-authoritative.
- **Commits were split by reconstructing the intermediate Task-1-only document state** (rather than committing the fully-edited file once) to preserve one-commit-per-task atomicity: sections 6 and 7-9 were all edited in the same session before the first commit, so the working file was reverted to a Task-1-only snapshot, committed, then advanced to the full Task-2 state and committed again — both commits verified independently against their own task's acceptance criteria before being made.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 — plan-text precision defect] The plan's `<read_first>` text states "eight deliberate-zero notes"; the measured count is 6**
- **Found during:** Task 1, confirming §9.2's deliberate-zero notes by absence
- **Issue:** `29-07-PLAN.md`'s own `<read_first>` block names "the eight 'deliberate zero' notes naming types absent at v0.9.0." A direct grep of `MIGRATION.md` §9.2 for `**Note on` paragraphs returns exactly 6 (Plan 22-01, Phases 23/24/25/26/28).
- **Fix:** Recorded the measured count (6) and the exact discrepancy in section 6's own prose, following the same house pattern `29-04-SUMMARY.md` already established twice in this exact phase (its own FR-count and orphan-behavior-count findings) rather than inventing a seventh/eighth note or padding the count.
- **Files modified:** `.project/v0.10.0/09-program-acceptance-audit.md`
- **Verification:** `grep -n '\*\*Note on' MIGRATION.md` (scoped to §9.2) returns 6 matches; every named type in all six is confirmed absent from the v0.9.0 export by direct grep.
- **Committed in:** `3137b683` (Task 1 commit)

**2. [Rule 3 — blocking-issue fix, environment setup] `mdbook build docs/` failed on a missing `mermaid.min.js`/`mermaid-init.js` pair**
- **Found during:** Task 2's `<verify>` step
- **Issue:** This worktree's `docs/` directory had never had `mdbook-mermaid install docs/` run against it — the two generated, gitignored asset files (`.gitignore:20-22`, "re-generated at build time via mdbook-mermaid install") were absent, so `mdbook build` failed at the HTML-backend rendering step with "Unable to copy /workspace/.../docs/mermaid.min.js ... No such file or directory."
- **Fix:** Ran `mdbook-mermaid install docs/`, which writes the two gitignored asset files. Not a code or content defect — a one-time per-worktree setup step this worktree had not yet had run. Re-ran `mdbook build docs/`, which then exited 0 with "No broken links found."
- **Files modified:** none (the two generated files are gitignored, not tracked, not staged)
- **Verification:** `mdbook build docs/` exits 0, `git status --short` shows no new untracked files from this step.
- **Committed in:** N/A (no tracked file changed)

---

**Total deviations:** 2 (1 Rule 1 plan-precision recording, 1 Rule 3 environment-setup fix) — no production code touched, no scope creep, both fully bounded within this plan's own `<files>` scope.
**Impact on plan:** None on the deliverable's correctness. Both are documented so a future reader understands why the recorded note-count differs from the plan text and why `mdbook-mermaid install` was needed before the docs build could succeed.

## Threat Flags

None. This plan adds no new attack surface — it is a documentation artifact over already-shipped, already-security-reviewed code and configuration, and every command run was read-only (greps, `cargo semver-checks`, `cargo check`, `cargo deny check`, `cargo audit`, `cargo test`, `mdbook build`) or a local, gitignored asset-generation step (`mdbook-mermaid install`).

## Known Stubs

None. Sections 6-9 are fully evidenced with re-run commands and real anchors; the "Accepted deviation" and "Maintainer sign-off" sections are, by design, not evidenced-and-closed — they are explicitly deferred to a human decision, which is the point of D-16/D-17, not a stub. Section 10 remains an explicit, correctly-labeled placeholder owned by plan 29-09.

## User Setup Required

None — no external service configuration required. (This worktree needed a one-time `mdbook-mermaid install docs/` local asset-generation step, documented above under Deviations; it produces no tracked file and requires no user action beyond what this plan already performed.)

## Next Phase Readiness

- Plan 29-08 can proceed to triage `.planning/WINDOWS.md`'s open rows, including writing the new `deviation` row this plan proposed (but did not write) for the pre-existing 72-warning `cargo doc` condition and the D-16 bench-overhead deviation.
- Plan 29-09 can proceed to fill section 10 (release readiness) once the version bump lands, and the phase's UAT/`/gsd-verify-work` step can tick the seven maintainer sign-off boxes this plan left unchecked.
- The D-16 deviation is now recorded in two of its four named artefacts (this audit, the published docs); the root `CHANGELOG.md` entry (29-09) and the `WINDOWS.md` row (29-08) remain to be written by their owning plans.
- No blockers. `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md` checkboxes, `.planning/WINDOWS.md`, `MIGRATION.md`, the semver allowlist, any `.rs`/`Cargo.toml`/CI file, and plan 29-06's files (`docs/src/api-reference/*`, `docs/src/SUMMARY.md`, `.project/v0.10.0/00-program-overview.md`) were left untouched per the worktree isolation instructions.

## Self-Check: PASSED

- `.project/v0.10.0/09-program-acceptance-audit.md` — FOUND, modified
- `docs/src/operations/observability.md` — FOUND, modified
- Commit `3137b683` (Task 1) — FOUND in `git log --oneline`
- Commit `c9502836` (Task 2) — FOUND in `git log --oneline`
- `cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0` → `Summary no semver update required` — CONFIRMED (re-run twice in this session)
- Section 6 range: `current-exports.txt` present, `ADR-0048` present, `required method` present, zero `Verdict: pending` — CONFIRMED
- Section 7 range: `M-B-0[1-4]` ≥ 4 matches, `ENG-08` cited — CONFIRMED
- `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets --locked` → `Finished`, 0 errors — CONFIRMED
- `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok` — CONFIRMED
- `cargo audit` → `warning: 10 allowed warnings found`, exit 0 — CONFIRMED
- `cargo doc --workspace --no-deps` → 72 `warning:` lines — CONFIRMED
- `cargo test --features web-server --test v0_9_config_boot` → `9 passed; 0 failed` — CONFIRMED
- `cargo test -p paladin-web --test openapi_golden_v0_9` → `6 passed; 0 failed` — CONFIRMED
- `grep -c '^- \[ \]'` on the audit doc → `7`; `grep -c '^- \[x\]'` → `0` — CONFIRMED
- `grep -c 22.18` / `18.46` on both files ≥ 1 — CONFIRMED
- `mdbook build docs/` → exit 0, "No broken links found" — CONFIRMED
- `git diff --name-only | grep -c '\.rs$'` across both commits → `0` — CONFIRMED

---
*Phase: 29-program-gates-release*
*Completed: 2026-09-10*
