---
phase: 52-ci-release-pipeline-repair
plan: 02
subsystem: infra
tags: [release, shell-guards, jq, github-releases, release-body-size]

requires:
  - phase: 52-01
    provides: "tests/scripts harness conventions and the make test-shell-guards glob; check-publish-order.sh sourcing-seam precedent"
provides:
  - "scripts/create-or-reuse-release.sh: argv-free payload builder (jq --rawfile) with a create-time release-body size guard"
  - "scripts/release-body-fit.sh: the single shared release-body size helper (limit, reserve, code-point length, line-boundary fit with sentinel footer); consumed by plan 52-04"
  - "tests/scripts/release-body-fit_test.sh: 85-assertion unit harness for the helper"
  - "tests/scripts/create-or-reuse-release_test.sh: Cases 10-14 (49 assertions total)"
affects: [52-04, 52-08]

tech-stack:
  added: []
  patterns:
    - "Large text reaches jq through --rawfile, never --arg: no body byte crosses argv"
    - "Size policy lives in one sourced helper (RELEASE_BODY_FIT_LIB_ONLY=1 seam), never duplicated per script"
    - "Length is Unicode code points via python3 strict UTF-8; invalid UTF-8 is a named failure, never a placeholder body"
    - "Truncation is never silent: stderr size log, ::warning annotation under Actions, sentinel footer in the body"

key-files:
  created:
    - scripts/release-body-fit.sh
    - tests/scripts/release-body-fit_test.sh
  modified:
    - scripts/create-or-reuse-release.sh
    - tests/scripts/create-or-reuse-release_test.sh
    - tests/scripts/extract-changelog-section_test.sh

key-decisions:
  - "PD-02-1: the guard triggers at the limit (125000) but fits to limit - reserve (120000) so finalize-release-body.sh still has room to append"
  - "PD-02-2: the D-10 argv case raises PALADIN_RELEASE_BODY_LIMIT to 500000 so its 243,044-byte body is really sent whole"
  - "PD-02-3: env vars PALADIN_RELEASE_BODY_LIMIT / PALADIN_RELEASE_BODY_RESERVE; footer links CHANGELOG.md at the tag, not a dated heading anchor, and carries no sizes so a re-fit is idempotent"
  - "PD-02-4: one helper sourced by both release scripts; a missing helper is a named failure before any API call"
  - "Empty PALADIN_RELEASE_BODY_LIMIT / _RESERVE means 'use the default' (an unset CI variable expands to empty); any non-empty malformed value is a named failure"
  - "Fence handling follows CommonMark closing rules (same character, at least the opening length, nothing after) rather than a bare toggle, so a stray different-type fence inside a block cannot close it"

patterns-established:
  - "Fit works on a mktemp copy in the caller's shell (not a command substitution) so the ::warning annotation reaches the runner on stdout"

requirements-completed: [CIREL-05]

coverage:
  - id: D1
    description: "Release body travels file -> jq --rawfile -> stdin; a >131072-byte body is created byte-identical"
    requirement: "CIREL-05"
    verification:
      - kind: unit
        ref: "tests/scripts/create-or-reuse-release_test.sh Case 10 (red against the unmodified script, green after the fix)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Over-limit body is cut at a line boundary to limit - reserve with a sentinel footer, logged by size only; boundary, multi-byte, invalid-limit and invalid-UTF-8 edges pinned"
    requirement: "CIREL-05"
    verification:
      - kind: unit
        ref: "tests/scripts/release-body-fit_test.sh (85 assertions) and create-or-reuse-release_test.sh Cases 11-14"
        status: pass
    human_judgment: false
  - id: D3
    description: "Live GitHub release create with a long changelog (D-12: no live release call in this phase)"
    requirement: "CIREL-05"
    verification: []
    human_judgment: true
    rationale: "D-12 verification posture is make test-shell-guards plus shellcheck plus CI's actionlint; the first live exercise is the next real release"

duration: about 30min
completed: 2026-10-28
status: complete
---

# Phase 52 Plan 02: Size-proof release-body builder Summary

**The release payload is now built with `jq --rawfile` so a changelog of any size never crosses argv, and one shared `scripts/release-body-fit.sh` cuts an over-limit body at a line boundary with a logged, sentinel-footered record before the create call.**

## Performance

- **Duration:** about 30 min
- **Completed:** 2026-10-28
- **Tasks:** 3 of 3 (Task 1 was the tracer)
- **Files:** 2 created, 3 modified

## Accomplishments

- CIREL-05 / D-09: `_cor_build_payload` no longer reads the body into a shell variable; it passes the file to `jq -n --rawfile body`. `--arg body` appears nowhere in the script. The existing `ERROR: --body-file not found` branch is kept.
- CIREL-05 / D-10: Case 10 generates a 243,044-byte multi-line body (with a multi-byte tail) at test time in the harness scratch directory, sends it with `PALADIN_RELEASE_BODY_LIMIT=500000`, and proves exactly one create call, payload `.body | length` of 243,032 (> 131,072) and `jq -j .body | cmp` byte-identity.
- CIREL-05 / D-11: `scripts/release-body-fit.sh` defines `release_body_limit`, `release_body_reserve`, `release_body_length`, `release_body_fit`, `release_body_fit_for_create` and `release_body_fit_main`. Measuring and cutting happen in one `python3 -I` block (strict UTF-8, code points, temp-file-then-replace output). `create-or-reuse-release.sh` sources it with `RELEASE_BODY_FIT_LIB_ONLY=1` and fits into a `mktemp` copy on the create path only.
- Edges pinned by tests: exactly-limit body byte-identical and limit+1 cut; 1,500 three-byte characters (4,500 bytes) at limit 2000 untouched; invalid UTF-8, non-numeric / zero / negative limit, reserve >= limit and too-small target each a named failure with no output file and no create call; empty body; single 5,000-character line hard-cut non-empty; open and balanced code fences; a footered body re-fitted (including with the old footer inside the kept budget) keeps exactly one sentinel; lookup-200 re-run never fits; the caller's file is `cmp`-identical after every case.

## Task Commits

1. **Task 1: argv failure red, then jq --rawfile (tracer)** - `3a5d6469f` (fix)
2. **Task 2: shared release-body size helper and unit harness** - `2730148f0` (feat)
3. **Task 3: create-time size guard wired into create-or-reuse-release.sh** - `b6d7e00f1` (feat)

Metadata commit: see git log (docs(52-02)).

## Tracer evidence (Task 1)

- **Red first (D-10):** Case 10 was run against the unmodified builder. Transcript lines:
  - `PASS: case 10 body is 243044 bytes (> 131072, over the single-argument limit)`
  - `FAIL: expected zero exit (silent) for: a body larger than 131072 bytes is created without crossing argv (got 1)`
  - `/home/user/paladin-dev-env/scripts/create-or-reuse-release.sh: line 105: /usr/bin/jq: Argument list too long`
  - `FAIL: oversized-argv body makes exactly one create call -- expected 1 create call(s), got 0`
  - `FAIL: case 10 payload .body length is 0, expected > 131072` / `FAIL: case 10 payload body differs from the body file`
  - `❌ 4/27 assertion(s) failed.` Exit status of the script was 1 (the plan anticipated 126; bash reported E2BIG as a failed `jq` exec inside the `if !` command substitution), create count 0.
- **Green:** after the `--rawfile` change `./tests/scripts/create-or-reuse-release_test.sh` printed `✅ 27 assertion(s) passed.` (21 pre-existing plus 6 new); Case 8 (literal `EOF`, backticks, `$(...)`) still passes.
- **Mutation check:** the unmodified script is the mutant for the tracer (4 assertions fail, restored by the fix). For the Task 2 helper, three temporary mutations were each caught and the file was restored byte-identical: `<=` to `<` at the limit boundary (2 failures), fence-closing disabled (2 failures), footer stripping disabled (initially NOT caught, which exposed a harness gap; Case E2 was added and the mutation now fails 1 assertion).
- **Tracer gate:** per the run's checkpoint policy the executor-rule tracer checkpoint was not stopped on; the evidence above is recorded for the orchestrator.

## Verification (plan level)

- `./tests/scripts/create-or-reuse-release_test.sh`: 49 assertions passed. `./tests/scripts/release-body-fit_test.sh`: 85 assertions passed.
- `make test-shell-guards` exit 0 (all 16 harnesses). `make check-gates` exit 0.
- `shellcheck --severity=warning` clean on `scripts/create-or-reuse-release.sh`, `scripts/release-body-fit.sh`, both new/changed harnesses and `tests/scripts/extract-changelog-section_test.sh`.
- `git ls-files -s`: `scripts/release-body-fit.sh` and `tests/scripts/release-body-fit_test.sh` are mode `100755`. `grep -c 'wc -m' scripts/release-body-fit.sh` prints 0; `grep -c '^release_body_[a-z_]*()'` prints 6; `grep -c '^# --- Case 1[0-4]'` on the create harness prints 5.
- No committed large fixture: `git status --porcelain tests/scripts/fixtures` is empty; no scratch directories remain under `$TMPDIR`.
- Not run: `actionlint` (unavailable in this sandbox; this plan edits no workflow file, so CI's Workflow Lint is unaffected). No live GitHub release call was made (D-12).

## Credential-handling review (security.instructions.md) for `create-or-reuse-release.sh`

- **No body or header text is logged:** the only new output is `release-body-fit: original N chars, target T chars, final M chars` (sizes), the `::warning` size line, and named errors that carry paths and variable names only. Harness assertions (helper Case H, create Case 11) prove a marker string from the body never reaches stderr or stdout.
- **No redirect-following client added:** the script still talks to the API only through `gh api`; no `curl`/`reqwest`-style client was introduced.
- **The token-bearing call is unchanged:** `_cor_gh_call` and its `gh api -i -X ... --input -` invocation are untouched; the payload still arrives on stdin.
- **No body through argv, environment or `$GITHUB_OUTPUT`:** the body moves file to `jq --rawfile` to stdin; the fitted temp copy is removed right after the payload is built; the temp file is created with `mktemp` (mode 0600).
- Outcome: no findings. T-52-06 to T-52-10 mitigated as planned.

## Decisions Made

Followed PD-02-1 through PD-02-4 as planned; see `key-decisions`. Additions inside Claude's discretion: empty limit/reserve variable means default; tag and repo are validated against a conservative character set before they reach the footer.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] extract-changelog-section_test.sh pinned the old newline-stripping payload behavior**
- **Found during:** Task 3 (`make test-shell-guards`)
- **Issue:** Flagged assumption A-52-08 materialized: `--rawfile` keeps the body file's trailing newlines, which `$(cat ...)` stripped. The pre-existing assertion "captured create payload's .body equals the extracted file byte-for-byte" computed its expected side through `$(cat ...)` and failed (expected without, actual with, the trailing `\n\n`).
- **Fix:** the expected side now reads the raw file (`open(..., newline="")`), which is what the assertion's own message ("byte-for-byte") states. The comment records the contract change.
- **Files modified:** `tests/scripts/extract-changelog-section_test.sh` (outside the plan's `files_modified` list, directly caused by this plan's change)
- **Verification:** that harness 16/16, full `make test-shell-guards` exit 0.
- **Commit:** `b6d7e00f1`

**2. [Rule 2 - Missing critical test coverage] Footer-stripping mutation was not caught by the first harness draft**
- **Found during:** Task 2 mutation check
- **Issue:** a re-fit test only covered a footer that lay beyond the kept budget, so disabling the strip still passed.
- **Fix:** added Case E2 (old footer inside the budget, more text after it); the mutation now fails.
- **Commit:** `2730148f0`

**3. [Attribution] Commit trailer model name**
- The orchestrator brief specified `Co-Authored-By: Claude Fable 5.1`; the harness attribution reminder for this session names `Claude Sonnet 5.5`, which is the model actually running. All three task commits (the first was amended before any further work) carry the Sonnet 5.5 trailer plus the `Claude-Session` line.

## Notes for plan 52-04 (consumer of the helper)

- The signatures in the plan's `<interfaces>` block are unchanged. `release_body_fit IN OUT TARGET TAG REPO` strips the footer **from the sentinel line to the end of the text**, so any text appended after an existing footer is dropped when that body is re-fitted. Compose artifact sections so they are re-fitted before, not after, a footer is in the middle of the text, or fit the composed body from the un-footered source.
- `release_body_fit_for_create` prints the `::warning` annotation on stdout; call it in the shell, not inside a command substitution, or the annotation is captured instead of reaching the runner.
- `--rawfile` keeps the body file's trailing newline(s) that `$(cat ...)` stripped (A-52-08): a one-time, harmless payload difference. Release bodies produced from `CHANGELOG.md` sections now end with their file's trailing newlines.

## Issues Encountered

- An early `grep -r` over the whole repository (including `target/`) exceeded the tool timeout and was moved to a background task; it was replaced by a `git grep` scoped to tracked files. The background process is read-only.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path or trust-boundary file access beyond the plan's threat model.

## Self-Check: PASSED

Files verified present: `scripts/release-body-fit.sh`, `tests/scripts/release-body-fit_test.sh`, `scripts/create-or-reuse-release.sh`, `tests/scripts/create-or-reuse-release_test.sh`, `tests/scripts/extract-changelog-section_test.sh`. Commits `3a5d6469f`, `2730148f0` and `b6d7e00f1` verified in git log.
