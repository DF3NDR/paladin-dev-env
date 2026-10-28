---
phase: 52-ci-release-pipeline-repair
plan: 04
subsystem: infra
tags: [release, shell-guards, github-releases, release-body-size, finalize]

requires:
  - phase: 52-02
    provides: "scripts/release-body-fit.sh: release_body_limit, release_body_length, release_body_fit (footer-stripping, line-boundary fit) and the RELEASE_BODY_FIT_LIB_ONLY=1 sourcing seam"
provides:
  - "scripts/finalize-release-body.sh: _frb_fit_composed_body, a composed-body size guard that runs between compose_release_body and gh release edit"
  - "tests/scripts/finalize-release-body_test.sh: Cases 26, 27, 28a-28c, 29a-29d (129 assertions total, up from 84)"
affects: [52-08]

tech-stack:
  added: []
  patterns:
    - "Re-fit only the curated section to limit minus the artifact part, then recompose from the same artifact inputs so the artifact tail stays byte-identical"
    - "Fit in the caller's shell with stderr held in a file and always relayed, so the helper's size log reaches the runner and one specific failure can be recognised"
    - "Read a fitted file with $(cat f; printf x) so command substitution never strips its trailing newline"

key-files:
  created: []
  modified:
    - scripts/finalize-release-body.sh
    - tests/scripts/finalize-release-body_test.sh

key-decisions:
  - "PD-04-1: _frb_fit_composed_body takes the six artifact inputs after OUTPUT CURATED TAG REPO (OUTPUT CURATED TAG REPO DIGEST IMAGE_REF SIZE_MB ASSET_LIST SUMS_NAME SBOM_ASSET) because the plan requires re-running compose_release_body with the same artifact inputs"
  - "PD-04-2: the helper is loaded lazily by _frb_load_fit_lib (called from main right after --tag validation, and from the guard itself), so lib-only sourcing for compose tests never needs it and a missing helper is a named failure before any gh call"
  - "PD-04-3: the 'leaves no room' failure fires directly when the budget is below 1 and, for a positive but footer-sized budget, when the helper reports 'is too small to hold the truncation footer'; any other helper failure (invalid UTF-8, bad tag) is relayed as the helper's own named error"
  - "PD-04-4: the guard never calls release_body_reserve, so finalize is unaffected by PALADIN_RELEASE_BODY_RESERVE (the reserve exists for the create step)"

patterns-established:
  - "An early return when the body fits is an optimisation only: even without it the helper copies an already-fitting curated section unchanged, so finalize stays byte-idempotent by construction"

requirements-completed: [CIREL-05]

coverage:
  - id: D1
    description: "A composed body over PALADIN_RELEASE_BODY_LIMIT is re-fitted by cutting only the curated section; the published body is within the limit, carries the footer once, and its artifact part is byte-identical to the untruncated composition"
    requirement: "CIREL-05"
    verification:
      - kind: unit
        ref: "tests/scripts/finalize-release-body_test.sh Case 26 (red against the unmodified script, green after the guard)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Finalize is byte-idempotent over its own truncated output (three runs identical, footer singular, no new truncation logged)"
    requirement: "CIREL-05"
    verification:
      - kind: unit
        ref: "tests/scripts/finalize-release-body_test.sh Case 27"
        status: pass
    human_judgment: false
  - id: D3
    description: "Bodies at or under the limit (including exactly the limit and a pre-footered curated section) publish byte-identical to the plain composition with no fit logged"
    requirement: "CIREL-05"
    verification:
      - kind: unit
        ref: "tests/scripts/finalize-release-body_test.sh Cases 28a, 28b, 28c"
        status: pass
    human_judgment: false
  - id: D4
    description: "Artifact sections that leave no room, a footer-sized budget, an invalid limit and a missing helper each fail by name before gh release edit"
    requirement: "CIREL-05"
    verification:
      - kind: unit
        ref: "tests/scripts/finalize-release-body_test.sh Cases 29a, 29b, 29c, 29d (call_log shows no EDIT)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Live Finalize Release Body job on a real tag with a long changelog (D-12: no live release call in this phase)"
    requirement: "CIREL-05"
    verification: []
    human_judgment: true
    rationale: "D-12 verification posture is make test-shell-guards plus shellcheck plus CI's actionlint; the first live exercise is the next real release"

duration: about 25min
completed: 2026-10-28
status: complete
---

# Phase 52 Plan 04: Finalize release-body size guard Summary

**`finalize-release-body.sh` now re-fits only the curated changelog text to `limit - artifact part` with the shared `release-body-fit.sh` helper before `gh release edit`, so a body created near the cap plus the appended artifact sections can no longer turn the tag's `Finalize Release Body` job red, and re-runs stay byte-identical.**

## Performance

- **Duration:** about 25 min
- **Completed:** 2026-10-28
- **Tasks:** 2 of 2 (Task 1 was the tracer)
- **Files:** 2 modified (0 created)

## Accomplishments

- CIREL-05 / D-11 (finalize side): `_frb_fit_composed_body` measures the composed OUTPUT with `release_body_length` against `release_body_limit`. When it fits it returns at once. Otherwise it computes the artifact part's length (total minus the curated part as compose normalised it), sets `budget = limit - artifact part`, fits the curated text to that budget with `release_body_fit`, recomposes with `compose_release_body` from the same artifact inputs, and re-measures, failing by name if the result is still over the limit. The call sits between `compose_release_body` and `"${GH}" release edit` with `REPO="${GITHUB_REPOSITORY:-}"`; no other line of the main flow changed.
- The artifact sections are never touched: Case 26 asserts the bytes from the artifacts marker onward are `cmp`-identical to the untruncated composition, and Case 28c repeats that at limit minus one.
- Idempotency (T-52-18): Case 27 runs finalize three times over the stub's round-tripped body; runs 2 and 3 are `cmp`-identical to run 1, log no `release-body-fit:` line, and the footer count is 1.
- Named failures, all before `gh release edit` (T-52-16): artifact sections alone over the limit (29a), a positive budget too small for the footer (29c), invalid `PALADIN_RELEASE_BODY_LIMIT` (29b, `must be a positive integer`), and a missing helper (29d, `release-body-fit.sh not found`, no gh call at all).
- The script header documents the size guard, `PALADIN_RELEASE_BODY_LIMIT`, `release-body-fit.sh`, and the new non-zero exit paths.

## Task Commits

1. **Task 1: composed-body guard, over-cap fit and two-run identity (tracer)** - `95cb71f91` (feat)
2. **Task 2: under-cap identity, named failures, header documentation** - `e7142970c` (test)

Metadata commit: see git log (docs(52-04)).

## Tracer evidence (Task 1)

- **Red first:** Cases 26-27 were run against the unmodified script. The harness finished with `5/101 assertion(s) failed`. Failing lines:
  - `FAIL: case26 published body is 8277 chars, expected 1..4000`
  - `FAIL: case26 expected the truncation sentinel once, got 0`
  - `FAIL: case26: the truncation is logged by size -- expected to find 'release-body-fit: original'`
  - `FAIL: case26: the last curated entry was cut -- did not expect to find 'entry 100:'`
  - `FAIL: case27 expected one footer after three runs, got 0`
  The 84 pre-existing assertions all passed, and the unmodified script already composed the artifact part identically (so the tail `cmp` passed, as expected: only the size behavior was missing).
- **Green:** after the guard, `./tests/scripts/finalize-release-body_test.sh` printed `101 assertion(s) passed` (84 + 17). The fixture's curated section is 7,737 chars with a fenced block spanning chars 2,258-5,422; the published body at limit 4000 is 3,933 chars and the cut lands inside the fence, so the helper's appended closing fence is exercised.
- **Mutation check:** with `budget=$((limit - artifact_chars))` replaced by `budget=$((limit))` the harness failed 10 assertions (Case 26 over-limit body, tail identity, sentinel, and all of Case 27); restored byte-identical (verified with `cmp`).
- **Tracer gate:** per the run's checkpoint policy the executor-rule tracer checkpoint was not stopped on; the evidence above is recorded for the orchestrator.

## Task 2 mutation notes

- Disabling the `is too small to hold the truncation footer` match fails Case 29c (`expected to find 'leaves no room'`); restored.
- Two mutations of the early return (`fits` test disabled; `fits` test lowered to half the limit) are **not** caught, and cannot be: the early return is an optimisation. If it is bypassed, `release_body_fit` copies an already-fitting curated section unchanged and the recompose reproduces the same bytes, so the published body is identical. This is the by-design reason finalize is byte-idempotent. Both were reverted.

## Verification (plan level)

- `./tests/scripts/finalize-release-body_test.sh`: 129 assertions passed (84 pre-existing + 45 new).
- `make test-shell-guards` exit 0 (all 16 harnesses); `make check-gates` exit 0.
- `shellcheck --severity=warning` clean on `scripts/finalize-release-body.sh` and `tests/scripts/finalize-release-body_test.sh`.
- `git diff --quiet HEAD~2 -- scripts/release-body-fit.sh` is clean: 52-02's helper is untouched. `grep -c '^# --- Case 2[67]'` prints 2; `grep -c '^# --- Case 2[89]'` prints 7.
- No cargo build or test was run (disk constraint; none needed). No live GitHub release call was made (D-12). `actionlint` is not installed here and this plan edits no workflow file. Harness scratch directories are removed by the harness's own trap; none remain under `$TMPDIR`.

## Credential-handling review (security.instructions.md)

- No body text is logged: the only new output is the helper's size line and named `ERROR:` messages that carry sizes, paths and variable names only.
- No HTTP client or credential header was added; `gh release edit` and its token are unchanged and still receive the body as a file via `--notes-file`.
- The scratch directory comes from `mktemp -d` (mode 0700) and is removed on every return path, including failures.
- Outcome: no findings. T-52-16, T-52-17 and T-52-18 mitigated as planned.

## Decisions Made

Followed the plan; see `key-decisions` (PD-04-1 to PD-04-4) for the choices inside Claude's discretion.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Extra arguments on `_frb_fit_composed_body`**
- **Found during:** Task 1
- **Issue:** The plan names the signature `_frb_fit_composed_body OUTPUT CURATED TAG REPO` but also requires re-running `compose_release_body` with "the same artifact inputs", which the function cannot know from those four arguments.
- **Fix:** Appended the six artifact inputs (`DIGEST IMAGE_REF SIZE_MB ASSET_LIST SUMS_NAME SBOM_ASSET`) as trailing arguments. The first four arguments are exactly as specified.
- **Files modified:** `scripts/finalize-release-body.sh`
- **Commit:** `95cb71f91`

**2. [Attribution] Commit trailer model name**
- The orchestrator brief specified `Co-Authored-By: Claude Fable 5.1`; the harness attribution reminder for this session names `Claude Sonnet 5.5`, which is the model actually running. As in plan 52-02, both task commits carry the Sonnet 5.5 trailer plus the `Claude-Session` line (the first commit was amended before any further work).

### Additions beyond the plan's listed cases

- Cases 28c (exactly-at-limit identity and limit-minus-one fit), 29c (footer-sized budget) and 29d (missing helper) were added to pin edges the plan's behavior list implies but does not enumerate.

## Issues Encountered

- A scratch log written with an unset shell variable landed at `/green.log` (a stray file at the filesystem root, created by this run, 4 KB or less). The harness blocked my attempt to delete it as a dangerous removal, and I did not work around that. It is outside the repository and harmless; it can be removed by hand with `rm /green.log`.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path or trust-boundary file access beyond the plan's threat model.

## Self-Check: PASSED

Files verified present: `scripts/finalize-release-body.sh`, `tests/scripts/finalize-release-body_test.sh`. Commits `95cb71f91` and `e7142970c` verified in git log.
