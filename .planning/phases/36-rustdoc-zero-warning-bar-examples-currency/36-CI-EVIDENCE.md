# Phase 36 Rustdoc Zero-Warning Bar & Examples Currency — CI Evidence Record (plan 36-13)

**Phase:** 36-rustdoc-zero-warning-bar-examples-currency
**Branch:** `feature/phase-33` (unpushed — this branch has never had an `origin/feature/phase-33`
upstream; `git rev-parse --abbrev-ref --symbolic-full-name @{u}` fails with "no upstream
configured", exactly as `33-CI-EVIDENCE.md` recorded for the same branch one phase earlier)
**Head SHA at seed time:** `12aaf84e367ed8ae40dd7a5931925ba2976c934b` — the tip of
`feature/phase-33` carrying plans 36-01 through 36-13's Tasks 1 and 2 (the closure map, the two
`WINDOWS.md` row flips, and the changelog append). This plan is paused at its Task 3 checkpoint;
no further commit lands before the push this file's PENDING table describes.
**Written:** 2026-09-17

This record follows the `33-CI-EVIDENCE.md` house shape: a **Local sweep** of every gate this
devcontainer can run without a pushed branch (all run and recorded below, against the actual
tree at the head SHA above), and a **CI-run table** — here explicitly **PENDING**, because
plan 36-13's own Task 3 is a `checkpoint:human-verify` the orchestrator directed this executor
not to resolve: no `git push`, no remote branch, no PR, and no `gh run` invocation. The
maintainer performs the push and records the real run themselves, filling in the second table
below.

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

## CI-run table — PENDING, to be recorded by the maintainer

**No CI run exists yet for any Phase 36 commit.** `feature/phase-33` has never been pushed, so
there is no `origin/feature/phase-33` for a workflow to trigger against. This executor was
explicitly directed not to push, not to open a PR, and not to invoke `gh run` in this session —
pushing and recording the real run is the maintainer's own step, exactly as `33-CI-EVIDENCE.md`
left it for the orchestrator one phase earlier.

| Job | Step | Run ID | Conclusion | Notes |
|---|---|---|---|---|
| `lint` | `Check documentation` | _(pending)_ | _(pending)_ | pre-existing default-features bar |
| `lint` | `Check documentation (all features, -D warnings)` | _(pending)_ | _(pending)_ | **the step D-12 makes a required check — this row is the phase's actual acceptance evidence** |
| `test` | `Run doc tests` | _(pending)_ | _(pending)_ | `cargo test --workspace --doc`, expect 462 passed / 0 failed / 210 ignored to match the local figure |
| `examples` (`Example Muster (Feature Matrix)`) | all 7 build steps + `Assert all 62 example binaries were produced` | _(pending)_ | _(pending)_ | expect 62 expected == 62 found, matching the local `make check-examples` figure |

### What fills this table in

1. `git push -u origin feature/phase-33`
2. Wait for the run to complete, then: `gh run list --branch feature/phase-33 --limit 5`
3. `gh run view <id> --json jobs` — read each job's `conclusion` and each step's `conclusion`
   for the four rows above, and fill in the run ID and conclusions.
4. If any CI figure disagrees with the local figures in the Local sweep table above, the CI
   figure is authoritative (D-03): record the difference plainly in a new subsection here,
   do not edit the local sweep table to match, and do not explain the difference away.

---

*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Written: 2026-09-17*
*Status: local sweep complete (8/8 green); CI-run table pending the maintainer's push, per this
plan's Task 3 checkpoint.*
