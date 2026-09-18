# Phase 36 Rustdoc Zero-Warning Bar & Examples Currency — CI Evidence Record (plan 36-13)

**Phase:** 36-rustdoc-zero-warning-bar-examples-currency
**Branch:** `feature/phase-33`, pushed with upstream `origin/feature/phase-33` by the maintainer.
**Head SHA at seed time:** `12aaf84e367ed8ae40dd7a5931925ba2976c934b` — the tip of
`feature/phase-33` carrying plans 36-01 through 36-13's Tasks 1 and 2 (the closure map, the two
`WINDOWS.md` row flips, and the changelog append).
**Head SHA at the recorded CI run:** `20195975c1c2665abb169b287fa178353d672bd2` — two maintainer
commits landed on top of the seed-time head after the push (`2ca02eab chore: updated GSD config`,
`20195975 chore: fix end of file automation`); neither touches `src/`, `crates/`, `examples/`,
`Cargo.toml`/`Cargo.lock`, `Makefile` or `.github/workflows/`, so the local sweep below (captured
at the seed-time head) remains valid evidence for the pushed tree's actual gate behavior.
**Written:** 2026-09-17. **Updated:** 2026-09-18, with the real CI run recorded below.

This record follows the `33-CI-EVIDENCE.md` house shape: a **Local sweep** of every gate this
devcontainer can run without a pushed branch (all run and recorded below, against the actual
tree at the head SHA above), and a **CI-run table** recording the real run the maintainer's push
triggered. Per this plan's Task 3 instructions the push itself was performed by the maintainer,
not by an executor; the run's job and step data below were read live via `gh run view` and
`gh api …/jobs/<id>/logs` and cross-checked against the raw job logs.

---

## Local sweep — the two bar commands and their supporting local gates

All commands below were run by plan 36-12 against the tree that became this phase's closing
state (`36-evidence/36-12-closing-measurement.txt` and `36-evidence/36-12-check-examples.txt`
carry the full verbatim captures this table summarizes); the head SHA above carries no source
change since that measurement (plan 36-13 touches only `.planning/`, `CHANGELOG.md` and
`WINDOWS.md`), so the figures still hold at this file's head SHA.

| # | Command | Local result | Verdict |
|---|---------|---------------|---------|
| 1 | `cargo doc --workspace --no-deps` (ci.yml's "Check documentation" step, byte-identical invocation) | exit 0; 0 `warning:` lines (was 73 total / 65 content diagnostics across 8 crates at the phase's own baseline) | GREEN |
| 2 | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` (ci.yml's new "Check documentation (all features, -D warnings)" step, byte-identical invocation) | exit 0 (was exit 101 at baseline) | GREEN |
| 3 | `make doc-check` (steps 1-2 above plus `cargo test --workspace --doc`, the single local source of truth plan 36-12 added) | all three steps green; doctests **462 passed / 0 failed / 210 ignored** — identical to the phase's own baseline figure | GREEN |
| 4 | `cargo test --workspace --doc` (the workspace doctest run CI's `test` job "Run doc tests" step also runs, not duplicated in the lint job) | 462 passed / 0 failed / 210 ignored | GREEN |
| 5 | `make check-examples` (local mirror of the CI Example Muster job's 7-invocation feature-matrix split, plan 36-12) | exit 0; **62/62 example binaries present** (`scripts/check-all-examples.sh`'s own binary-count assertion) | GREEN |
| 6 | `make api-surface` | 3959 items, unchanged from the phase's own baseline and from every intermediate plan | GREEN (no drift) |
| 7 | `cargo fmt --all -- --check` | exit 0 | GREEN |
| 8 | `cargo check --workspace --all-targets --all-features` | exit 0 | GREEN |

**Local sweep verdict: 8/8 green.** Every gate this devcontainer can run without a pushed
branch passes at the head SHA above. What this sweep does **not** and cannot prove: that the
exact CI runner (its own toolchain pin, its own cache state, its own Example Muster job running
all 8 gated `[[example]]` targets under their real `required-features` sets rather than this
devcontainer's local mirror script) reaches the same result. That is exactly what D-12 requires
a real pushed-branch run for, and exactly why the table below is PENDING rather than filled in
from local evidence.

### The exact CI step names this run must show green

From `.github/workflows/ci.yml` at the head SHA above:

- **`lint` job**, step `Check documentation` (line 62) — the pre-existing default-features bar,
  unchanged by this phase.
- **`lint` job**, step `Check documentation (all features, -D warnings)` (line 70) — the new
  all-features bar plan 36-12 added; this is the step D-12 makes a required check.
- **`examples` job**, named `Example Muster (Feature Matrix)` (line 512) — builds all 62
  example targets across 7 invocations (1 default-features bulk build + 6 feature-gated steps),
  then its own step `Assert all 62 example binaries were produced` (line 586) checks the
  expected-vs-found binary count.
- **`test` job**, step `Run doc tests` (line 506) — `cargo test --workspace --doc`, already
  covered by the local sweep above and not duplicated by the new lint-job step.

---

## CI-run table — recorded, real pushed-branch run (D-12)

The maintainer pushed `feature/phase-33` (final head `20195975c1c2665abb169b287fa178353d672bd2`,
two no-op-for-this-phase commits past the seed-time head recorded above). The push triggered
four workflow runs on that head SHA; the relevant one is `.github/workflows/ci.yml` run
**35290763563** — https://github.com/DF3NDR/paladin-dev-env/actions/runs/35290763563. (The other
three same-push runs: `feature-flags.yml` 35290763619, `codeql.yml` 35290763575 — advisory-only
per `security.instructions.md`, not a merge gate — and `pre-commit` 35290763583, which concluded
`success`.)

Data below was read live via `gh run view 35290763563 --json jobs` and cross-checked against the
raw per-job logs downloaded with `gh api repos/<owner>/<repo>/actions/jobs/<id>/logs` (ANSI
escapes stripped with `sed 's/\x1b\[[0-9;]*m//g'`) — every figure quoted in the Notes column was
read directly out of the log text, not assumed from the step's green checkmark alone.

| Job | Job ID | Step | Conclusion | Notes |
|---|---|---|---|---|
| `lint` (`Code Quality`) | 105432882707 | `Check documentation` | **success** | Pre-existing default-features bar. Raw log (`ci-job-lint.log`, step starting line 1550) shows the pipeline `cargo doc --workspace --no-deps 2>&1 \| tee /tmp/doc-output.txt && ! grep -q "warning:" /tmp/doc-output.txt` running to `Finished`/`Generated` with **zero** `warning:` lines in the captured output — matches the local figure (0, down from 73 baseline) exactly. |
| `lint` (`Code Quality`) | 105432882707 | `Check documentation (all features, -D warnings)` | **success** | **The step D-12 makes a required check — this is the phase's actual acceptance evidence.** Raw log (step starting line 1691) shows `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` running to `Finished`/`Generated` with no error and step conclusion `success` — matches the local figure (exit 0, down from exit 101 baseline) exactly. |
| `test` (`Unit Tests (stable)`) | 105432882852 | `Run doc tests` | **success** | `cargo test --workspace --doc`'s 13 per-crate `test result:` lines (`ci-job-unit.log`, step starting line 5134) sum to **462 passed / 0 failed / 210 ignored** — summed by hand from each crate's line (`paladin` 147/0/18, `paladin_core` 95/0/38, `paladin_battalion` 59/0/52, `paladin_content` 0/0/0, `paladin_doc_examples` 0/0/0, `paladin_eval` 4/0/2, `paladin_herald` 0/0/6, `paladin_llm` 8/0/0, `paladin_memory` 12/0/0, `paladin_notifications` 0/0/0, `paladin_ports` 137/0/94, `paladin_storage` 0/0/0, `paladin_web` 0/0/0) — **identical** to the local closing figure in `36-evidence/36-12-closing-measurement.txt`. `Unit Tests (beta)` (job 105432882777) ran the identical step with the same `success` conclusion. |
| `examples` (`Example Muster (Feature Matrix)`) | 105434525009 | `Build examples (default features — 54 auto-discovered targets)` | **success** | First of 7 build invocations. |
| `examples` (`Example Muster (Feature Matrix)`) | 105434525009 | `Build examples (vision — vision_analysis, vision_battalion)` | **success** | |
| `examples` (`Example Muster (Feature Matrix)`) | 105434525009 | `Build examples (content-processing — document_processing)` | **success** | |
| `examples` (`Example Muster (Feature Matrix)`) | 105434525009 | `Build examples (web-server — http_service_host, webhook_receiver)` | **success** | |
| `examples` (`Example Muster (Feature Matrix)`) | 105434525009 | `Build examples (web-server,dev-ui — platform_api_client)` | **success** | |
| `examples` (`Example Muster (Feature Matrix)`) | 105434525009 | `Build examples (redis-cache — node_result_cache)` | **success** | |
| `examples` (`Example Muster (Feature Matrix)`) | 105434525009 | `Build examples (otel — observability_otel_export)` | **success** | |
| `examples` (`Example Muster (Feature Matrix)`) | 105434525009 | `Assert all 62 example binaries were produced` | **success** | Raw log line: `Expected: 62 example binaries; found: 62` (`ci-job-examples.log` line 1733) — **identical** to the local `make check-examples` figure (62/62). |

**Job-level conclusions confirming the above:** `Code Quality` job → `success`; `Unit Tests
(stable)` job → `success`; `Unit Tests (beta)` job → `success`; `Example Muster (Feature Matrix)`
job → `success`.

### Local-versus-CI agreement (D-03)

Every figure recorded above **agrees exactly** with the local closing measurement in
`36-evidence/36-12-closing-measurement.txt`: zero default-feature `warning:` lines, all-features
documentation exit 0, 462 passed / 0 failed / 210 ignored doctests, and 62/62 example binaries.
**No disagreement was found between the local and CI figures for any of the four gates this run
proves.** Per D-03 the CI figure is what is recorded as authoritative in this table regardless;
the fact that it matches the local figure exactly is stated here as a finding, not assumed.

### Overall run status at recording time

Run 35290763563 was still `status: in_progress` (`conclusion: ""`) when this table was recorded
(2026-09-18, ~00:40 UTC) — the four gate-relevant jobs above (`Code Quality`, `Unit Tests
(stable)`, `Unit Tests (beta)`, `Example Muster (Feature Matrix)`) had all already completed with
conclusion `success`; the jobs still running at that time were `Integration Tests` and `Docker
Build` (both `in_progress`, neither one of the four gates this checkpoint verifies).
`Benchmark Regression Signal (Non-Blocking)` and `Publish Dry Run` had already concluded
`skipped`, by design (non-blocking / tag-gated). This record states the overall run status
honestly rather than claiming a final green the run had not yet reached at recording time; the
four rows this checkpoint exists to prove are unambiguously `success` and do not depend on the
remaining jobs' outcome.

---

### Final run conclusion (appended 2026-09-18 01:39 UTC, after the run finished)

The section above was written while the run was still in progress and is left unedited as the
honest record of what was known at recording time. Run 35290763563 has since **completed**:

| Field | Value |
|---|---|
| Overall | `status: completed`, `conclusion: success` (finished 2026-09-18T01:39:21Z) |
| Jobs | 37 total — **34 `success`**, 3 `skipped` by design |
| Skipped by design | `Benchmark Regression Signal (Non-Blocking)` (non-blocking), `Publish Dry Run` (tag-gated), `End-to-End Tests` (its own trigger condition) |
| Jobs still running at recording time | `Integration Tests` → `success`; `Docker Build` → `success`; `Coverage` → `success`; `Kubernetes Smoke Test` → `success` |

Nothing in the completed run changes any figure in the CI-run table above, and no job that was
green at recording time regressed. The `Coverage` job is called out explicitly because it is the
one gate this devcontainer cannot measure locally (no Docker); it concluded `success` on this
head SHA.

---

*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Written: 2026-09-17. Updated: 2026-09-18 with the real CI run, then again with its final
conclusion.*
*Status: local sweep complete (8/8 green); CI run 35290763563 **completed `success`** — all four
checkpoint-relevant jobs (`Code Quality`, `Unit Tests (stable)`, `Unit Tests (beta)`, `Example
Muster (Feature Matrix)`) concluded `success`, with every measured figure matching the local
closing measurement exactly, and the whole run finished green (34 success, 3 skipped by design).
See "Final run conclusion" above; the in-progress note below it is retained unedited as the
record of what was known when the table was first written.*
