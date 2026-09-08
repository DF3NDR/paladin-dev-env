---
phase: 27-platform-api
plan: 24
subsystem: testing
tags: [ci, cargo-public-api, api-surface, rustdoc, github-actions, e2e]

# Dependency graph
requires:
  - phase: 27-platform-api (wave 1, plans 27-19..27-23)
    provides: the merged code whose public API surface this plan's baseline regeneration must be taken after
provides:
  - a toolchain-order-independent public-API baseline extraction (scripts/normalize-api-bounds.py)
  - a regenerated .project/current-exports.txt proven to contain no real API change
  - a named e2e-platform-api CI job running the PRD 06 acceptance-1 lifecycle on every push/PR
affects: [27-platform-api (plan 27-25 checkpoint), any future phase touching public API surface or the platform-api web-server feature]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "stdlib-only Python filter with a --self-test mode piped into a bash extraction pipeline under set -euo pipefail"
    - "declared-vs-selected CI guard: assert a non-zero passing test count rather than trusting a bare exit code (matches postgres-integration/redis-node-cache/redis-run-queue house pattern)"

key-files:
  created: [scripts/normalize-api-bounds.py]
  modified: [scripts/extract-public-api.sh, .project/current-exports.txt, .github/workflows/ci.yml]

key-decisions:
  - "Marker set is a closed, explicit tuple (Send/Sync/Unpin/Freeze/UnsafeUnpin) rather than any core::marker::* path or any trait -- a broader match risks reordering semantically-ordered bounds that are not auto-traits."
  - "e2e-platform-api job carries no needs: edge, matching the plan's requirement that it neither gates nor is gated by another job; promoting it to a required branch-protection check is left to whoever administers the ruleset."
  - "actionlint is not on PATH in this devcontainer; the python3/PyYAML safe_load parse check (already required by acceptance criteria) stands in, as the plan's own acceptance criteria anticipated."

requirements-completed: [PLAT-06]

coverage:
  - id: D1
    description: "scripts/normalize-api-bounds.py canonicalises adjacent auto-trait marker bound ordering, with a --self-test mode covering convergence, sorted-run emission, byte-identical passthrough of ordinary lines, and a real RunWorkerPool<W> line from both observed toolchain orderings"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "python3 scripts/normalize-api-bounds.py --self-test"
        status: pass
      - kind: integration
        ref: "printf 'a + core::marker::Sync + core::marker::Send)\\n' | normalize vs printf 'a + core::marker::Send + core::marker::Sync)\\n' | normalize -- byte-identical output"
        status: pass
    human_judgment: false
  - id: D2
    description: ".project/current-exports.txt regenerated through the canonicalising extraction; proven to contain no real public-API change via normalised-old-vs-new empty diff and equal pub-item counts (3763 = 3763)"
    requirement: "PLAT-06"
    verification:
      - kind: integration
        ref: "diff -u <(normalize old baseline, filtered) <(new baseline, filtered) -- empty"
        status: pass
      - kind: integration
        ref: "./scripts/check-api-surface.sh .project/current-exports.txt (run twice, both green)"
        status: pass
    human_judgment: false
  - id: D3
    description: "e2e-platform-api CI job runs cargo test --features web-server --test e2e_platform_api on every push/PR, with no needs: edge, and fails if the run selects zero tests"
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "python3 -c \"...yaml.safe_load...job.get('runs-on') and job.get('steps') and not job.get('needs')\""
        status: pass
      - kind: integration
        ref: "cargo test --features web-server --test e2e_platform_api -- 'test result: ok. 1 passed'"
        status: pass
    human_judgment: false

# Metrics
duration: 11min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 24: Canonicalise API-Surface Bound Ordering and Add e2e-platform-api CI Job Summary

**A closed-set marker-bound normaliser makes the `api-surface` CI gate toolchain-order-independent, and a new `e2e-platform-api` job runs PRD 06's flagship end-to-end lifecycle test on every push and PR instead of only locally.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-09-08T14:06:14Z
- **Completed:** 2026-09-08T14:17:28Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments
- `scripts/normalize-api-bounds.py`: a standard-library-only stdin/stdout filter that sorts maximal runs of adjacent auto-trait marker bounds (`Send`/`Sync`/`Unpin`/`Freeze`/`UnsafeUnpin`) alphabetically, leaving every other token on every line byte-identical; `--self-test` covers convergence, sorted-run emission with a trailing non-marker token left untouched, byte-identical passthrough, and a real `RunWorkerPool<W>` line pulled from the actual baseline in both observed toolchain orderings.
- `.project/current-exports.txt` regenerated through the canonicalising extraction and proven to contain no real public-API change: the previous committed baseline, piped through the same normaliser and diffed against the new baseline (both with generated-timestamp/total-count header lines filtered), produces an empty diff; the `^pub ` item count is unchanged (3763 = 3763); `git diff --stat` shows only 11 changed lines (the `RunWorkerPool<W>` `where`-clause bound orderings plus the header timestamp/version lines).
- `e2e-platform-api` CI job added to `.github/workflows/ci.yml`, immediately after `sdk-clients` and before `coverage`: builds with the `web-server` feature, runs only the `e2e_platform_api` test binary, and asserts a passing test-result line with a non-zero count before the job can report success -- closing the gap where the PRD 06 acceptance-1 lifecycle test (full assistant -> run -> SSE -> `AwaitingInput` -> webhook -> resume -> complete -> history -> fork) passed locally but was never reached by any existing CI job.

## Task Commits

Each task was committed atomically:

1. **Task 1: A canonicaliser makes the extraction independent of auto-trait bound ordering** - `f6debee4` (feat, tracer)
2. **Task 2: Regenerate the baseline and prove the only difference is ordering** - `fa80bd95` (chore)
3. **Task 3: A CI job runs the PRD 06 acceptance-1 end-to-end test on every push and PR** - `4c2dc14b` (feat)

_Task 1 was a `type="tracer"` task: committed first, then its `<verify>` was re-run end-to-end (self-test + the two observed-order convergence check) before proceeding to Tasks 2 and 3, per the tracer feedback gate._

## Files Created/Modified
- `scripts/normalize-api-bounds.py` - new stdlib-only canonicaliser for auto-trait marker bound ordering, with `--self-test`
- `scripts/extract-public-api.sh` - pipes `cargo public-api --simplified` output through the normaliser before appending to the baseline
- `.project/current-exports.txt` - regenerated in canonical form; no new/removed public items
- `.github/workflows/ci.yml` - new `e2e-platform-api` job running the PRD 06 acceptance-1 test with a zero-selected-tests guard

## Decisions Made
- The marker set the normaliser matches is a closed, explicit tuple (not any `core::marker::*` path, not any trait) — a named module constant with a comment explaining the risk a broader match would carry.
- `e2e-platform-api` has no `needs:` edge, so it runs independently of every other job (matches the plan's behavior requirement and the `sdk-clients` job's own independence).
- `actionlint` is unavailable in this devcontainer; the plan's own acceptance criteria anticipated this and specified the `python3`/PyYAML parse check as the substitute — recorded here as required.

## Deviations from Plan

None - plan executed exactly as written. All three tasks' acceptance criteria and `<verify>` commands passed on the first attempt; no auto-fixes, no architectural questions, no blockers.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required. Note per the plan: adding the `e2e-platform-api` job does not make it a *required* branch-protection check — promoting it belongs to whoever administers the repository's ruleset, outside this repo's files.

## Next Phase Readiness
- The `api-surface` CI job now compares against a toolchain-order-independent baseline and should remain green across a future nightly bump that reorders synthesised auto-trait bounds again.
- `e2e_platform_api` (PRD 06 acceptance criterion 1) now runs on every push and pull request via the new `e2e-platform-api` job, and cannot report success without having actually run at least one test.
- Plan 27-25's checkpoint is the place where the `api-surface` job reporting "API surface unchanged" and the new `e2e-platform-api` job going green on CI itself (not just locally) gets asserted — this plan proved both locally; CI-side confirmation is out of this plan's scope per its own `<verification>` section.

---
*Phase: 27-platform-api*
*Completed: 2026-09-08*
