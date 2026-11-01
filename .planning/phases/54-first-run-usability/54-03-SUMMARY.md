---
phase: 54-first-run-usability
plan: 03
subsystem: docs
tags: [docs, readme, versioning, first-run, pins]

requires:
  - phase: 54-first-run-usability
    provides: "54-01 refusal of a ${...} HTTP API key (README text now says so)"
provides:
  - "README install pin and Current version line at the workspace version"
  - "Every workspace pin on quickstart, installation, crate-map, feature-flags and sentinel pages at the workspace version"
  - "README first-run paragraph that instructs a literal http.auth.api_keys entry"
  - "Version-free prose banners in crate-map.md and installation.md"
affects: [54-07 version-string gate, 54-09 evidence]

tech-stack:
  added: []
  patterns:
    - "Prose that names the current release is made version-free; only the four gated classes carry a literal workspace version"

key-files:
  created: []
  modified:
    - README.md
    - docs/src/getting-started/quickstart.md
    - docs/src/getting-started/installation.md
    - docs/src/api-reference/crate-map.md
    - docs/src/api-reference/feature-flags.md
    - docs/src/appendix/sentinel.md

key-decisions:
  - "Banners (crate-map l.12, installation l.48 and l.74) made version-free rather than bumped (PD-03-1)"

requirements-completed: [FIRST-02, FIRST-01]

status: complete
---

# Phase 54 Plan 03: First-run pins at the workspace version Summary

**README and five first-run doc pages now pin `paladin-ai` at the workspace version 0.11.0 (36 pins plus the README status line), and the README server paragraph leads to a config that boots with a literal API key.**

## Facts

- Plan-start SHA: `fb392c3aa1ea1d8080e595bd3ad98031edc2de7a`
- Computed `V` (root `[package].version`, via `tomllib`): `0.11.0`
- Duration: about 5 minutes; 2 tasks; 6 files

## Prove-then-lock hit lists (re-measured before editing)

Task 1 (`grep -nE '"0\.(5|10\.0)"|0\.8\.0|v0\.10\.0'` over README, quickstart, installation): README l.108 (`"0.5"`), l.183 (`0.8.0`); quickstart l.24-26; installation l.48 (prose), 53, 54, 57, 60, 63, 66, 74 (prose), 78. Identical to the plan's `<interfaces>` list.

Task 2 (`grep -nE '"0\.(5|10\.0)"|v0\.10\.0'` over crate-map, feature-flags, sentinel): crate-map banner l.12 plus 8 pins (l.190, 198, 212, 227-231); feature-flags 18 pins (l.132 through l.391, l.362 and l.391 being `//!` lines); sentinel l.66. Matches the expected 8, 18, 1 and banner.

## Task Commits

1. Task 1 (tracer): `b9d91ac60` docs(54-03): README and getting-started pins read the workspace version; README first run uses a literal key
2. Task 2: `20fbd29a0` docs(54-03): crate map, feature flags and Sentinel pins read the workspace version

Tracer gate (auto mode): Task 1 `<verify>` passed end to end before Task 2 began.

## Verification

- Task 1 and Task 2 `<verify>` commands both pass.
- No `"0.5"`, `"0.10.0"`, `0.8.0` or `v0.10.0` remains in the six files.
- Diff filters: no `tokio`/`serde` line removed; the removed lines in the api-reference and sentinel files are only stale-version lines.
- `git diff --stat fb392c3` lists exactly the six `files_modified`.
- README no longer contains "come from the environment, not the config file".

## Decisions Made

- Followed PD-03-1: banners made version-free so nothing outside the gated pattern classes can go stale at the Phase 58 bump.
- Realigned the README `# default features` comment after the pin grew from `"0.5"` to `"0.11.0"`.

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None.

## Threat Flags

None. Documentation-only change; T-54-08 and T-54-09 mitigated by the diff checks above (`migration-guide.md` untouched).

## Self-Check: PASSED

All six modified files exist; commits `b9d91ac60` and `20fbd29a0` present in `git log`.
