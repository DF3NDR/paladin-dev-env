---
phase: 27-platform-api
plan: 21
subsystem: testing
tags: [ci, sdk-generation, smoke-test, hermetic, sdk-clients, mock-llm, npm]

# Dependency graph
requires:
  - phase: 27-platform-api (plan 18)
    provides: "scripts/sdk-smoke/{run.sh,smoke.py,smoke.ts,package.json,tsconfig.json,smoke-config.yml} and the sdk-clients CI job that runs them"
provides:
  - "scripts/sdk-smoke/mock-llm.py — stdlib-only, loopback OpenAI-compatible chat-completions stub with a --self-test mode; no network egress, no real credential"
  - "scripts/sdk-smoke/lib-boot.sh — the single shared boot/teardown definition (smoke_boot_start/smoke_boot_stop) for run.sh and smoke-http.sh"
  - "scripts/sdk-smoke/smoke-http.sh — generator-free curl round trip proving a run reaches exactly `completed` locally, no Java/Docker required"
  - "scripts/sdk-smoke/package-lock.json — committed lockfile so `npm ci` succeeds and the TypeScript half of sdk-clients actually runs"
  - "scripts/sdk-smoke/smoke.py and smoke.ts — both now require the exact SUCCESS_STATUS ('completed') via an extracted, self-testable decision function, never any terminal status as a pass"
affects: [27-24, 29-ship]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A source-only shell library (lib-boot.sh) with no `set -e` of its own, sourced by every caller that owns its own `set -euo pipefail` — the single-definition-not-two pattern for shared shell boot/teardown logic."
    - "A pure success-decision function (evaluate_terminal_status / evaluateTerminalStatus) extracted from polling logic so a --self-test flag can exercise pass/fail branches directly, with no server or generated client involved."

key-files:
  created:
    - scripts/sdk-smoke/mock-llm.py
    - scripts/sdk-smoke/lib-boot.sh
    - scripts/sdk-smoke/smoke-http.sh
    - scripts/sdk-smoke/package-lock.json
  modified:
    - scripts/sdk-smoke/run.sh
    - scripts/sdk-smoke/smoke-config.yml
    - scripts/sdk-smoke/package.json
    - scripts/sdk-smoke/smoke.py
    - scripts/sdk-smoke/smoke.ts
    - .gitignore

key-decisions:
  - "run.sh's Task 2 change (installing the generated TypeScript client separately via `npm install --no-save`, after `npm ci`) was written in the same edit pass as Task 1's lib-boot.sh sourcing change, since both touch the same file's TypeScript subshell block. Both changes are committed in the Task 1 commit (5fa19186); the Task 2 commit (e3476030) has no further run.sh diff — its own acceptance criteria (grep -c 'no-save' == 1) were already satisfied by the Task 1 commit. Recorded here so the per-task commit history reads correctly against the plan's own per-task file lists."
  - "mock-llm.py's /models route returns `{data: [{id: 'gpt-4', object: 'model'}]}` (matching `get_available_models`'s `data[].id` read) even though no run-submission code path in this codebase calls it — confirmed by grepping the application layer for `validate_model`/`get_available_models` callers (none outside test doubles). Implemented anyway per the plan's own <behavior> spec, since a stub that only satisfies the one route actually exercised is a narrower contract than the plan asked for."
  - "smoke.ts's ambient-module type-check (`tsc --strict` against a hand-written `paladin-sdk` stub) was NOT committed — same convention 27-18-SUMMARY.md established: the real generated client's field/method shape is unproven locally (no Java/Docker), and a committed local stub could silently diverge from CI's real generator output without anyone noticing. The stub used to prove the type-check locally lived only in the scratchpad directory, never in the tree."

requirements-completed: [PLAT-06]

coverage:
  - id: D1
    description: "A run submitted against the smoke boot reaches `completed` end to end, locally, with no network egress and no credential — the hermetic loopback LLM stub (mock-llm.py) plus the shared boot (lib-boot.sh) plus the generator-free curl round trip (smoke-http.sh)."
    requirement: "PLAT-06"
    verification:
      - kind: e2e
        ref: "cargo build --bin paladin-server --features web-server && scripts/sdk-smoke/smoke-http.sh — exit 0, output ends 'SDK smoke (HTTP, generator-free): completed', re-run twice, stable both times, zero mock-llm requests reaching anything but 127.0.0.1"
        status: pass
      - kind: unit
        ref: "python3 scripts/sdk-smoke/mock-llm.py --self-test — exit 0, prints 'self-test: ok'"
        status: pass
    human_judgment: false
  - id: D2
    description: "A committed package-lock.json makes `npm ci` succeed in scripts/sdk-smoke/, so the TypeScript half of the sdk-clients CI job actually runs; the generated client installs separately via --no-save, never as an unresolvable file: dependency in the lockfile."
    requirement: "PLAT-06"
    verification:
      - kind: other
        ref: "node -e checks confirm lockfileVersion>=2 and package.json has no `dependencies` key; (cd scripts/sdk-smoke && npm ci) exits 0; git status --porcelain scripts/sdk-smoke empty afterwards"
        status: pass
    human_judgment: false
  - id: D3
    description: "Both smoke.py and smoke.ts require the terminal status to be exactly 'completed' via an extracted, self-testable decision function — any other terminal status or an unreached deadline fails loudly with the observed status and the run's own error text."
    requirement: "PLAT-06"
    verification:
      - kind: unit
        ref: "python3 -m py_compile scripts/sdk-smoke/smoke.py clean; python3 scripts/sdk-smoke/smoke.py --self-test — exit 0, 6/6 decision-function cases (completed passes; failed/halted/cancelled/None/unknown each fail)"
        status: pass
      - kind: other
        ref: "smoke.ts type-checks clean (tsc 5.9.3, --strict) against a hand-written ambient-module stub of the expected Configuration/AssistantsApi/RunsApi shape — the real generated client cannot be produced locally (no Java/Docker); CI's own sdk-clients job is the first real proof, per 27-18-SUMMARY.md's own documented precedent (WINDOWS.md id 30)"
        status: pass
    human_judgment: true
    rationale: "smoke.ts's real-client field/method-name assumptions are unproven locally by design (no generator available in this devcontainer) — CI's sdk-clients job running both live smokes end to end is the only place the TypeScript half's full round trip (including this plan's own exact-status assertion) can actually be observed against the real generated client."

duration: ~50min
completed: 2026-09-08
status: complete
---

# Phase 27 Plan 21: Hermetic sdk-clients Smoke — Loopback Stub, Committed Lockfile, Exact-Status Gate

**Closes verification gap 3: `mock-llm.py` (loopback OpenAI-compatible stub) + `lib-boot.sh` (single shared boot) make a submitted run reach `completed` with zero network egress, a committed `package-lock.json` makes `npm ci` actually run the TypeScript half, and both `smoke.py`/`smoke.ts` now require the exact `completed` status via a self-testable decision function instead of accepting any terminal status as a pass.**

## Performance

- **Duration:** ~50 min
- **Started:** 2026-09-08T13:43:00Z (approx, worktree base `6920e1fa`)
- **Completed:** 2026-09-08T13:58:06Z
- **Tasks:** 3 (`type="tracer" tdd="true"`, `type="auto"`, `type="auto" tdd="true"`)
- **Files modified:** 10 (4 created, 6 modified)

## Accomplishments

- `scripts/sdk-smoke/mock-llm.py`: a dependency-free `http.server` stub answering `POST {prefix}/chat/completions` with the exact `OpenAIResponse` shape `crates/paladin-llm/src/openai/adapter.rs` deserialises (confirmed against that file's own mockito test fixtures), and `GET {prefix}/models` with a one-entry `data` list. Binds `127.0.0.1` only. `--self-test` starts it on an ephemeral port, posts one request, validates every required field, checks the 404 path, and exits 0.
- `scripts/sdk-smoke/lib-boot.sh`: the single shared boot/teardown definition (`smoke_boot_start`/`smoke_boot_stop`) — starts the mock LLM stub, waits for it, exports `OPENAI_BASE_URL` at it plus the existing `APP_RUN_STORE_*`/`APP_WAYPOINT_STORE_*`/`OPENAI_API_KEY` variables, locates and starts `paladin-server`, waits for `/health`. `run.sh` and the new `smoke-http.sh` both source it instead of each carrying their own copy.
- `scripts/sdk-smoke/smoke-http.sh`: a generator-free curl round trip (list assistants → submit run → poll to terminal) that exits 0 only on an exact `completed` status, printing the run's `error` field on any other terminal status. Verified end to end against a real `cargo build`ed `paladin-server`: reaches `completed` in ~0.3s locally, zero network egress, re-run stable.
- `scripts/sdk-smoke/smoke-config.yml`: `llm_url` and the new `llm.openai` block now point at the loopback stub (`http://127.0.0.1:18081/v1`) instead of the real, unauthenticated `https://api.openai.com/v1` that made every submitted run legitimately fail with "Authentication failed".
- `scripts/sdk-smoke/package.json` / `package-lock.json`: the unresolvable `file:` dependency on `../../target/sdk/typescript` is gone from `package.json` (the generated client is a build artefact, not a lockfile-pinnable dependency); a real `package-lock.json` (lockfileVersion 3, `npm install --package-lock-only`) is committed so `npm ci` succeeds. `run.sh` installs the generated client separately afterward via `npm install --no-save`.
- `scripts/sdk-smoke/smoke.py` / `smoke.ts`: both extract a pure `evaluate_terminal_status`/`evaluateTerminalStatus` function comparing against a single `SUCCESS_STATUS = "completed"` constant with exact, case-sensitive equality (PLAT-06 `precision`) — `TERMINAL_STATUSES` still decides only when polling stops, never success. Any other terminal status, or a poll deadline reached with none observed (PLAT-06 `boundary`), fails loudly with both the observed status and the run's own `error` text. `smoke.py` gained a `--self-test` flag (guards the `openapi_client` import so it runs without the generated client installed) exercising all 6 cases (`completed` passes; `failed`/`halted`/`cancelled`/`None`/`unknown` each fail).
- `.gitignore`: `scripts/sdk-smoke/node_modules/`, `dist/` and `__pycache__/` — confirmed no untracked build output survives an `npm ci` + self-test cycle.

## Task Commits

Each task was committed atomically:

1. **Task 1: Hermetic stub + shared boot + generator-free round trip** - `5fa19186` (fix) — includes run.sh's full rewrite (both the lib-boot.sh sourcing change and the Task 2 `npm install --no-save` line; see key-decisions)
2. **Task 2: Committed lockfile** - `e3476030` (fix) — package.json/package-lock.json/.gitignore only; run.sh already carried its Task 2 content from the Task 1 commit
3. **Task 3: Exact-status gate on both smokes** - `64764f85` (fix)

**Plan metadata:** committed alongside this SUMMARY (worktree mode — STATE.md/ROADMAP.md updates deferred to the orchestrator).

## Files Created/Modified

- `scripts/sdk-smoke/mock-llm.py` — loopback OpenAI-compatible chat-completions stub, `--self-test` mode.
- `scripts/sdk-smoke/lib-boot.sh` — shared `smoke_boot_start`/`smoke_boot_stop`.
- `scripts/sdk-smoke/smoke-http.sh` — generator-free curl round trip.
- `scripts/sdk-smoke/run.sh` — sources `lib-boot.sh`; installs the generated TS client via `--no-save` after `npm ci`.
- `scripts/sdk-smoke/smoke-config.yml` — points at the loopback stub instead of the real OpenAI endpoint.
- `scripts/sdk-smoke/package.json` — dropped the unresolvable `file:` dependency.
- `scripts/sdk-smoke/package-lock.json` — new, committed.
- `scripts/sdk-smoke/smoke.py` — `SUCCESS_STATUS`, `evaluate_terminal_status`, `--self-test`.
- `scripts/sdk-smoke/smoke.ts` — `SUCCESS_STATUS`, `evaluateTerminalStatus`.
- `.gitignore` — `scripts/sdk-smoke/{node_modules,dist,__pycache__}/`.

## Decisions Made

See `key-decisions` in frontmatter. In prose:

1. **`run.sh`'s Task 1 and Task 2 changes were written in one edit pass and committed together in the Task 1 commit** — both touch the same TypeScript subshell block, and splitting them into two separate diffs against the same lines would have required re-editing already-correct code. Task 2's own acceptance criteria were re-verified against the Task 1 commit and pass.
2. **`mock-llm.py` implements the `/models` route even though no production code path calls it during a run** — confirmed via `grep -rn "validate_model\|get_available_models"` across `src/application/` (only test doubles implement it). Implemented per the plan's own `<behavior>` spec anyway, since a narrower stub would silently diverge from what was asked for.
3. **The TypeScript ambient-module stub used to type-check `smoke.ts` locally was never committed** — same reasoning 27-18-SUMMARY.md recorded: it would risk silently diverging from the real generator's output with no CI signal, and this plan's own files_modified frontmatter does not list it.

## Deviations from Plan

None — plan executed exactly as written. Both existing prior deviations from 27-18 (the standalone `smoke-config.yml`, the tolerant `_get`/`field` helpers for typed-model vs. plain-dict/object shapes) were preserved unchanged; this plan only edited the endpoint URLs and status-comparison logic inside files 27-18 already created.

## Issues Encountered

- **The TypeScript half's generated-client field/method-name assumptions remain unproven locally** (no Java/Docker in this devcontainer, same gap 27-18-SUMMARY.md documented as `WINDOWS.md` id 30). `smoke.ts` type-checks clean against a hand-written stub, and its logic mirrors `smoke.py`'s (proven end-to-end via `--self-test` and the real `smoke-http.sh` round trip), but the actual `npm ci` + generated-client install + `tsc` + `node dist/smoke.js` sequence in `run.sh` could not be exercised against a real generated client here — that remains CI's own `sdk-clients` job's job, per this plan's own `<verification>` block ("CI only").
- **`shellcheck --severity=warning` flagged four issues in the first draft of `lib-boot.sh`** (SC2034 on `API_KEY`/`AGENT_ID` appearing unused within the file itself — they're consumed by callers after sourcing; SC2164 on a bare `cd`; SC2034 on an unused loop variable in the second `for` loop). Fixed by exporting the two caller-consumed variables, adding `|| return 1` to the `cd`, and renaming both loop variables to `_i`/`_j`. All three scripts now pass `shellcheck --severity=warning` clean.

## User Setup Required

None — no external service configuration required. The hermetic loopback stub and `paladin-server` boot entirely from temp-file SQLite stores, the in-memory run queue, and a stdlib-only Python stub — no Docker, no real LLM credential, no network egress.

## Next Phase Readiness

- The `sdk-clients` CI job's Python-side gate (`smoke.py`) and server-side boot (`lib-boot.sh`, `mock-llm.py`) are proven locally end to end: a run submitted against the smoke profile reaches `completed`, hermetically, with a script proving it (not inspection).
- The TypeScript-side gate (`npm ci` succeeding, `smoke.ts`'s exact-status check) is proven up to the point local tooling allows (lockfile install, type-check against a stub); the live generated-client round trip is CI-only, as documented in Issues Encountered and the plan's own `<verification>` block.
- Plan 27-24 (which owns `ci.yml`) can now point the `sdk-clients` job at this hermetic boot with no further script changes needed on this plan's side.
- No blockers to closing this gap-closure plan.

## Self-Check: PASSED

**Files verified to exist:**
- FOUND: `scripts/sdk-smoke/mock-llm.py`
- FOUND: `scripts/sdk-smoke/lib-boot.sh`
- FOUND: `scripts/sdk-smoke/smoke-http.sh`
- FOUND: `scripts/sdk-smoke/run.sh`
- FOUND: `scripts/sdk-smoke/smoke-config.yml`
- FOUND: `scripts/sdk-smoke/package.json`
- FOUND: `scripts/sdk-smoke/package-lock.json`
- FOUND: `scripts/sdk-smoke/smoke.py`
- FOUND: `scripts/sdk-smoke/smoke.ts`
- FOUND: `.gitignore` (modified)

**Commits verified to exist (git log --oneline):**
- FOUND: `5fa19186` fix(27-21): hermetic loopback LLM stub + shared smoke boot (Task 1)
- FOUND: `e3476030` fix(27-21): committed lockfile makes npm ci succeed (Task 2)
- FOUND: `64764f85` fix(27-21): both smokes require the exact terminal status (Task 3)

**Verification commands re-run and confirmed passing:**
- `python3 scripts/sdk-smoke/mock-llm.py --self-test` → `self-test: ok`
- `grep -c 'chat/completions' scripts/sdk-smoke/mock-llm.py` → `5` (≥1); `grep -c '127.0.0.1' scripts/sdk-smoke/mock-llm.py` → `6` (≥1)
- `grep -c 'OPENAI_BASE_URL' scripts/sdk-smoke/lib-boot.sh` → `1`; `grep -c 'lib-boot.sh' scripts/sdk-smoke/run.sh` → `3` (≥1)
- `grep -vE '^\s*#' scripts/sdk-smoke/smoke-config.yml | grep -c 'api.openai.com'` → `0`
- `grep -c 'smoke_boot_start' scripts/sdk-smoke/smoke-http.sh` → `1`
- `bash -n scripts/sdk-smoke/run.sh && bash -n scripts/sdk-smoke/lib-boot.sh && bash -n scripts/sdk-smoke/smoke-http.sh` → exit `0`
- `cargo build --bin paladin-server --features web-server && scripts/sdk-smoke/smoke-http.sh` → exit `0`, output contains `completed` (re-run twice, stable both times)
- `test -f scripts/sdk-smoke/package-lock.json` → exit `0`; `node -e` lockfileVersion/no-dependencies check → exit `0`
- `grep -c 'typescript' scripts/sdk-smoke/package-lock.json` → `3` (≥1); `grep -c 'no-save' scripts/sdk-smoke/run.sh` → `1`
- `grep -c 'scripts/sdk-smoke/node_modules/' .gitignore` → `1`; `grep -c 'scripts/sdk-smoke/dist/' .gitignore` → `1`
- `(cd scripts/sdk-smoke && npm ci)` → exit `0`; `git status --porcelain scripts/sdk-smoke` → empty
- `python3 -m py_compile scripts/sdk-smoke/smoke.py` → exit `0`; `python3 scripts/sdk-smoke/smoke.py --self-test` → exit `0`, `self-test: ok (6/6 cases)`
- `grep -c 'SUCCESS_STATUS' scripts/sdk-smoke/smoke.py` → `9` (≥3); `grep -c 'SUCCESS_STATUS' scripts/sdk-smoke/smoke.ts` → `6` (≥3)
- `grep -c 'error' scripts/sdk-smoke/smoke.py` → `8` (≥2)
- `shellcheck --severity=warning` on all three shell scripts → clean (exit `0`)
- No unexpected file deletions in any of the three task commits (`git diff --diff-filter=D --name-only HEAD~1 HEAD` empty for each)
- No Rust files touched — `git diff --name-only 6920e1fa HEAD` matches the plan's `files_modified` list exactly (10 files); `cargo fmt`/`cargo check` skipped per plan instructions

---
*Phase: 27-platform-api*
*Plan: 21*
*Completed: 2026-09-08*
