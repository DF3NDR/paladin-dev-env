# Phase 29 Program Gates & Release — CI Evidence Record (plan 29-09)

**Phase:** 29-program-gates-release
**Branch:** `feature/phase-26`
**Base SHA at dispatch:** `5e0c979a0786c7c35b06af5bf73ce1c80a13ed4d` — the SHA this plan's own Task
1/2/3 commits sit on top of (all eight prior Phase 29 plans, 01–08).
**Written:** 2026-09-10

This record has three parts: a **Local sweep** (everything this devcontainer can prove without
Docker — all run and recorded below), a **CI-run table** for the newest workflow runs this
`feature/phase-26` branch actually has (which predate every Phase 29 commit — none of Phases 22–29
has ever been pushed past commit `77912ac8`, a Phase 28 post-merge-evidence commit), and a note on
what remains for the pre-merge and post-merge pushes to supply. This plan does not push and does
not run `gh run watch` — per its own operating instructions, pushing and re-triggering CI belongs
to the orchestrator.

---

## Local sweep

Every command below was run in this worktree, on top of this plan's own Task 1 (version bump,
`83d219f1`) and Task 2 (changelog stamping, `3019ed8e`) commits — both `.rs`-file-free
(`git diff --name-only 5e0c979a..HEAD | grep -c '\.rs$'` is 0), so every Rust-level result below
exercises the *bumped* `0.10.0` tree, not a stale pre-bump build.

| # | Command | Result (verbatim/summarized) | Verdict |
|---|---------|-------------------------------|---------|
| 1 | `cargo fmt --all --check` | Exit 0, no output | ✅ PASS |
| 2 | `cargo clippy --workspace --all-targets -- -D warnings` | `Finished` in 1m 01s, exit 0, zero warnings | ✅ PASS |
| 3 | `cargo clippy --workspace --all-targets --features otel -- -D warnings` | `Finished` in 29.13s, exit 0, zero warnings | ✅ PASS |
| 4 | `cargo clippy --workspace --all-targets --features dev-ui -- -D warnings` | `Finished` in 33.01s, exit 0, zero warnings | ✅ PASS |
| 5 | `cargo clippy --workspace --all-targets --features web-server -- -D warnings` | `Finished` in 0.68s (cache hit off the default-feature build), exit 0, zero warnings | ✅ PASS |
| 6 | `cargo test --workspace` | Every crate `test result: ok`, aggregate **0 failed** (paladin-ai 974, paladin-battalion 733+14 ignored, paladin-web 310, paladin-eval 585+doc, every other member green). A first run of this same command hit 3 transient timeouts in `paladin-ai`'s `cancel_tests`/`stream_tests` (30s-budget async tests racing under this devcontainer's concurrent-test load); re-run individually with `--test-threads=1` — all 3 passed in isolation — and a clean full-workspace re-run was 0 failed. Recorded as devcontainer-load flakiness in three pre-existing, unrelated-to-this-plan async timing tests, not a regression: this plan's own commits touch zero `.rs` files. | ✅ PASS (0 failed on the recorded clean run) |
| 7 | `cargo test --test evals` | `e2e-2-approval-gate.eval::approve ... ok` / `::deny ... ok` / `e2e-3-map-reduce-fault-tolerance.eval::recovering_worker ... ok` / `e2e-1-crash-resume.eval::crash_after_superstep_3 ... ok` — `test result: ok. 4 passed; 0 failed` | ✅ PASS |
| 8 | `cargo test --features web-server --test v0_9_config_boot` | `test result: ok. 9 passed; 0 failed` | ✅ PASS |
| 9 | `cargo test -p paladin-web --test openapi_golden_v0_9` | `test result: ok. 6 passed; 0 failed` | ✅ PASS |
| 10 | `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` for the eleven pre-existing publishable crates (`paladin-ai`, `paladin-ai-core`, `paladin-ports`, `paladin-battalion`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-storage`, `paladin-notifications`, `paladin-content`, `paladin-web`) | Every crate: `Checking <pkg> v0.9.0 -> v0.10.0 (major change)` / `0 checks: 0 pass, 254 skip` / `Summary no semver update required` — the bumped tree reports `0.9.0 -> 0.10.0` for every crate, matching RESEARCH.md's prediction that the per-crate `[package.metadata.cargo-semver-checks.lints]` suppressions still carry the nine allowed changes and every other lint stays clean | ✅ PASS (11/11) |
| 11 | `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets --locked` | `Finished dev profile [unoptimized + debuginfo] target(s) in 3m 34s` | ✅ PASS |
| 12 | `make security` (`cargo deny check` + `cargo audit`) | `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`; `cargo audit` → `warning: 10 allowed warnings found` (same pre-existing RustSec IDs as `28-CI-EVIDENCE.md`/`29-07-SUMMARY.md`: `rustls-pemfile`, `smartstring`, `event-listener`, `scc`, `chacha20`, `spin`), exit 0 | ✅ PASS |
| 13 | `./scripts/check-release-consistency.sh --tag v0.10.0` | `✅ OK: 12 publishable package(s) checked, all match tag version '0.10.0' with a changelog section for it.` (the CI-conclusion clause is explicitly skipped locally — no `--sha`, not a GitHub Actions run — matching the script's own documented local-run behavior) | ✅ PASS |
| 14 | `cargo publish --workspace --dry-run` | **Twelve** crates packaged and verified, each ending in `warning: aborting upload due to dry run`, zero errors. Verified in this exact dependency order: `paladin-ai-core` → `paladin-ports` → `paladin-llm` → `paladin-storage` → `paladin-battalion` → `paladin-content` → `paladin-eval` → `paladin-herald` → `paladin-memory` → `paladin-notifications` → `paladin-web` → `paladin-ai`. `paladin-doc-examples` (`publish = false`) does not appear anywhere in the output — correctly skipped as unpublishable, not silently dropped. `grep -c Uploading` and `grep -c "aborting upload due to dry run"` both return 12. | ✅ PASS (12/12, non-empty, dependency order) |
| 15 | `mdbook build docs/` | `[INFO mdbook_linkcheck] No broken links found` — exit 0 | ✅ PASS |
| 16 | `cargo doc --workspace --no-deps` | **72** `warning:` lines — re-measured live, matching `29-RESEARCH.md`'s and `29-07-SUMMARY.md`'s prior live counts exactly. This is the *exact* command the `lint` job's "Check documentation" step (`ci.yml:62-63`) runs with **zero tolerance**, meaning that CI step is failing on `main` right now, independent of this plan's own work. **Not a Phase 29 blocker**: SHIP-04's requirement text asks only for "no NEW broken intra-doc links" plus green semver/MSRV jobs — it does not require this warning count to reach zero, and D-25's bounded doc sweep does not include fixing it. Carried, recorded here rather than silently omitted, per `.planning/WINDOWS.md` row disposition already discussed in `29-07-SUMMARY.md` Section 8 (a `deviation` row for this condition was proposed there but is explicitly NOT filed by this plan — `.planning/WINDOWS.md` is out of this plan's file scope per the orchestrator's instructions). | ⚠️ CARRIED, pre-existing, out of D-25 scope (not a new break from this plan) |

**Local sweep verdict: 15/16 unconditionally green, 1 carried pre-existing condition explicitly recorded (row 16), 0 new failures caused by this plan.**

**Not run locally, by design (Docker/Java unavailable in this devcontainer, per every prior phase's own recorded gap):** the Postgres/Redis Tier-2 live-server contract suites and the `sdk-clients` job's real `openapi-generator-cli` run — all of these are read green from the CI-run table below (the most recent `feature/phase-26` run, `77912ac8`), never claimed passed locally.

---

## CI-run table

**This branch has never been pushed past `77912ac8`** — a Phase 28 post-merge-evidence commit
(`docs(28): append definitive post-merge CI evidence`). None of Phase 29's nine plans (01 through
this one, 09) have a CI run of their own; the newest available run for any of the `ci`, `docs`, or
`feature-flags` workflows is the one below, on `77912ac8`, dated **before this phase started**
(2026-09-09T14:46 UTC — Phase 29 execution began 2026-09-10 per `.planning/STATE.md`). This table
proves the pre-Phase-29 base was fully green; it does **not** prove anything about this plan's own
version-bump/changelog commits, which is exactly why the Local sweep above re-runs every command
this devcontainer can run against the actual bumped tree. The orchestrator pushes and supplies the
real pre-merge run (on the final Phase-29 SHA, including this plan's commits) and, later, the
post-merge run on the tagged `main` merge commit — exactly as `27-CI-EVIDENCE.md`/`28-CI-EVIDENCE.md`
recorded for their own phases.

| Workflow | Run ID (URL) | Conclusion | SHA | Notes |
|---|---|---|---|---|
| `.github/workflows/ci.yml` | `34365812871` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/34365812871 | **success** | `77912ac8` | All 33 jobs green (Semver Checks vs v0.9.0, MSRV 1.88, Coverage, API Surface Tracking, License & Dependency Policy, Docker Build, all eleven Crate Isolation legs, Postgres/Redis/Ollama live-server suites, `sdk-clients`, Kubernetes Smoke Test); `Publish Dry Run` and `End-to-End Tests` **skipped** (tag/manual-trigger gated, not failures); `Benchmark Regression Signal (Non-Blocking)` skipped by design. |
| `.github/workflows/codeql.yml` | `34365812918` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/34365812918 | **success** | `77912ac8` | Advisory-only per `security.instructions.md`. |
| `.github/workflows/feature-flags.yml` | `34365812964` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/34365812964 | **success** | `77912ac8` | All legs including `otel`, `dev-ui`, `llm-all` unification. |
| `pre-commit` | `34365812872` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/34365812872 | **success** | `77912ac8` | |
| `.github/workflows/docs.yml` | — none on this branch — | **not run** | — | `docs.yml`'s `push` trigger is deliberately restricted to `main` (see the workflow's own header comment: a path-filtered trigger on feature branches made non-docs PRs permanently unmergeable, PR #31). No pull request has been opened for `feature/phase-26`, so the `pull_request`-triggered "Build MDBook" leg (a required status check on `main`) has never run for this branch. **Not claimed green** — `mdbook build docs/` was run locally instead (Local sweep row 15, exit 0, "No broken links found"), and the real `docs.yml` "Build MDBook" run is owed to the PR the orchestrator opens for this branch. |

`Coverage` (run `34365812871`, job `102514106872`): `Lines: 108175/119822 = 90.28%` — above the 82%
floor (ADR-0006), consistent with `28-CI-EVIDENCE.md`'s post-merge figure (90.28%) since this run
predates any coverage-affecting commit since Phase 28 closed.

`Semver Checks (vs v0.9.0)` on this same run: **success**, reported against the pre-bump `0.9.0`
tree (this SHA predates the bump) — not the same claim as Local sweep row 10 above, which re-runs
semver-checks against the actual bumped `0.10.0` tree and is the evidence D-21 actually needs.

---

## Summary and what remains

**What this record proves:** every gate this devcontainer CAN run locally against the *actual
bumped `0.10.0` tree* is green or explicitly carried (16/16 accounted for, 15 unconditional passes
plus 1 named pre-existing condition), including all eleven pre-existing crates' semver checks now
reporting `0.9.0 -> 0.10.0` with the same allowlist, the MSRV floor, security scans, the dry-run
publish across a non-empty twelve-crate set in dependency order, and the docs build. The base commit
this branch sits on (`77912ac8`) was fully green across all four pushable workflows before Phase 29
began.

**What this record does NOT claim:** a CI run of any kind on any Phase 29 commit — none exists,
because this branch has not been pushed since before Phase 29 started. `docs.yml`'s "Build MDBook"
job (the actual required status check) has also never run for this branch, for the separate,
structural reason that it is PR-gated and no PR exists yet. Both gaps are closed the same way:
the orchestrator pushes this branch (or opens the PR) and appends the real pre-merge run table,
then later the post-merge run on the tagged `main` merge commit — exactly as `27-CI-EVIDENCE.md`
and `28-CI-EVIDENCE.md` did for their own phases. This plan does not push and does not run
`gh run watch`, per its own operating instructions.

---

*Phase: 29-program-gates-release*
*Written: 2026-09-10*
