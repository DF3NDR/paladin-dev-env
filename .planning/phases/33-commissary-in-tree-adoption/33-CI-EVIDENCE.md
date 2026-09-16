# Phase 33 Commissary In-Tree Adoption — CI Evidence Record (plan 33-06)

**Phase:** 33-commissary-in-tree-adoption
**Branch:** `feature/phase-33` (this worktree: `worktree-agent-a6a62eb40e484355a`, merged back onto
`feature/phase-33` by the orchestrator after this plan returns)
**Head SHA at sweep time:** `69500c9b51a37f11215037c49318d76ea017dab3` — the tip of `feature/phase-33`
at dispatch of this plan, carrying plans 33-01 through 33-05 and the wave-4 tracking commit. This is
the "final commit" D-24 names: no plan after this one changes source, and this plan's own two commits
are docs-only (this file, and the audit §11 append), so the gate sweep below is valid evidence for
the phase's actual shipped tree.
**Written:** 2026-09-16

This record has two parts, in the `29-CI-EVIDENCE.md` shape: a **Local sweep** (every gate this
devcontainer can run without Docker — all run and recorded below, against the actual `0.10.0` tree)
and a **CI-run table** for the newest workflow runs available to this branch. `feature/phase-33` has
never been pushed — a plain `git ls-remote --heads origin feature/phase-33` at sweep time returns
nothing — so no CI run exists yet for any Phase 33 commit. The CI-run table below cites the most
recent green run on the sibling `feature/phase-32` branch as the pre-Phase-33 base, exactly as
`29-CI-EVIDENCE.md` did for `feature/phase-26`. This plan does not push and does not run
`gh run watch` — pushing and re-triggering CI is the orchestrator's job, per every prior release-gate
plan's own operating instructions.

---

## Local sweep

| # | Command | Result (verbatim/summarized) | Verdict |
|---|---------|-------------------------------|---------|
| 1 | `grep -rn 'truncate_to_token_budget' crates src docs examples benches` (D-19 exit grep 1) | No matches (exit 1) | ✅ PASS — F6 closed |
| 2 | `grep -rnE '\.len\(\) */ *4' crates/paladin-memory/src` (D-19 exit grep 2) | No matches (exit 1) | ✅ PASS — F6 closed |
| 3 | `grep -c TBD MIGRATION.md` | `0` | ✅ PASS |
| 4 | `make check-migration-allowlist` | 15 `crate\|type` pairs in both the MIGRATION.md §9.2 register and the allowlist, set-equal in both directions | ✅ PASS |
| 5 | `make check-gates` | Per-crate CHANGELOG coverage (11/11), package-name allow-list (12/12), advisory-exception register (11 rows vs 11 deny.toml + 5 `.cargo/audit.toml` ignore entries), workflow inline-suppression scan (7 files, 160 steps, 0 inline ignores), workflow trigger-policy table (7/7), CodeQL dismissal register (6/6), plus row 4's set-equality check — all pass, exit 0 | ✅ PASS |
| 6 | `cargo test --features web-server --test v0_9_config_boot` | `test result: ok. 9 passed; 0 failed; 0 ignored` — same 9-test count `29-CI-EVIDENCE.md` row 8 recorded | ✅ PASS |
| 7 | `cargo test -p paladin-web --test openapi_golden_v0_9` | `test result: ok. 7 passed; 0 failed; 0 ignored` — one more than `29-CI-EVIDENCE.md` row 9's 6 (`execute_response_exception_is_narrowly_scoped` was added since Phase 29; not a Phase 33 change — Phase 33 touches no OpenAPI surface) | ✅ PASS |
| 8 | `cargo semver-checks check-release --package paladin-ai --default-features --baseline-version 0.9.0` | `Checking paladin-ai v0.9.0 -> v0.10.0 (major change)` / `0 checks: 0 pass, 254 skip` / `Summary no semver update required` | ✅ PASS |
| 9 | same, `--package paladin-ai-core` | Identical shape: major change, `0 checks: 0 pass, 254 skip` | ✅ PASS |
| 10 | same, `--package paladin-ports` | Identical shape | ✅ PASS |
| 11 | same, `--package paladin-battalion` | Identical shape | ✅ PASS |
| 12 | same, `--package paladin-herald` | Identical shape | ✅ PASS |
| 13 | same, `--package paladin-llm` | Identical shape — confirms no change to `paladin-llm`'s public surface this phase (D-04), matching 33-05's own Run 5 finding | ✅ PASS |
| 14 | same, `--package paladin-memory` | Identical shape — the RAG retrieval API break (`RagRetrievalResult`/`RagRetrievalError`/`format_for_prompt`) produces `0 checks: 0 pass, 254 skip` under the CI job's exact command (no `--release-type minor`), because at `0.9.0 -> 0.10.0` with major version `0`, cargo-semver-checks classifies the diff as a "major change" and skips every lint outright — this is the SAME behavior `29-CI-EVIDENCE.md` row 10 recorded for all eleven crates at the v0.10.0 bump, not a new or different result. The empirical zero-fired-lint finding 33-05 recorded (under `--release-type minor`, which forces evaluation) still stands as the tool-coverage explanation; this row confirms the CI job's own literal invocation reaches the identical "no semver update required" verdict | ✅ PASS |
| 15 | same, `--package paladin-storage` | Identical shape | ✅ PASS |
| 16 | same, `--package paladin-notifications` | Identical shape | ✅ PASS |
| 17 | same, `--package paladin-content` | Identical shape | ✅ PASS |
| 18 | same, `--package paladin-web` | Identical shape | ✅ PASS |
| 19 | `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets --locked` | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 4m 50s` — zero errors, zero warnings in the tail | ✅ PASS |
| 20 | `make publish-dry-run` (`release-check` prerequisite — `clean-code` + `cargo test --workspace` + `cargo test --workspace --doc` + `cargo audit` + `build-release` — then `cargo publish --workspace --dry-run`) | Every `test result:` line in the full run reports `0 failed` (28 distinct test binaries/doctest bundles, largest `paladin-ai` at 981 unit tests). `cargo publish --workspace --dry-run` packaged and verified **twelve** crates in dependency order (`paladin-ai-core` → `paladin-ports` → `paladin-herald` → `paladin-llm` → `paladin-notifications` → `paladin-storage` → `paladin-web` → `paladin-battalion` → `paladin-content` → `paladin-memory` → `paladin-eval` → `paladin-ai`), each ending `warning: aborting upload due to dry run`. `grep -c Uploading` and `grep -c "aborting upload due to dry run"` both return 12; `paladin-doc-examples` (`publish = false`) does not appear anywhere in the output | ✅ PASS (12/12, non-empty, dependency order, 0 test failures) |
| 21 | `make api-surface` | `✅ API surface extracted... (3959 items)` / `✅ API surface unchanged` — zero drift against the baseline plan 33-05 regenerated in the same commit as its CHANGELOG edit | ✅ PASS |
| 22 | `make clean-code` (`fmt` + `clippy --workspace --all-targets -- -D warnings` + `lint-shell` + `cargo check --workspace --all-targets`) | All four sub-targets exit 0; `✅ shellcheck clean`; zero clippy warnings | ✅ PASS |
| 23 | `make security` (`cargo audit` + `cargo deny check`) | `cargo audit`: `warning: 10 allowed warnings found` — `dotenv` (RUSTSEC-2021-0141), `fxhash` (RUSTSEC-2025-0057), `number_prefix` (RUSTSEC-2025-0119), `paste` (RUSTSEC-2024-0436), `rustls-pemfile` (RUSTSEC-2025-0134), `smartstring` (RUSTSEC-2026-0249), `event-listener` (RUSTSEC-2026-0221), `scc` (RUSTSEC-2026-0205), `chacha20` (yanked), `spin` (yanked) — all ten pre-existing/unmaintained/yanked-transitive, none introduced by this phase's own two dependency-graph changes (`paladin-llm` as a `paladin-memory` production dependency, `proptest` as a `paladin-memory` dev-dependency — neither pulls a new advisory). The exact ten differ in composition from `29-CI-EVIDENCE.md`'s six-named set (`rustls-pemfile`/`smartstring`/`event-listener`/`scc`/`chacha20`/`spin`) because the RustSec advisory DB has grown four more unmaintained-crate notices since 2026-09-10 (`dotenv`, `fxhash`, `number_prefix`, `paste`) — none of the four names a crate this phase touches. `cargo deny check`: `advisories ok, bans ok, licenses ok, sources ok`, exit 0 (informational `duplicate`/`yanked` warnings for `axum`/`base64`/`chacha20`/`spin` version pairs are pre-existing, cross-referenced against the same transitive dependency trees, not new) | ✅ PASS |
| 24 | `cargo test -p paladin-ai --lib limit_resolution` (D-20 regression, Phase 32 PRIM-04) | `running 3 tests` — `limit_resolution_prefers_the_config_table`, `limit_resolution_falls_back_to_the_default`, `limit_resolution_falls_back_to_provider_capabilities`, all `ok`. `test result: ok. 3 passed; 0 failed` | ✅ PASS (non-zero passed count) |
| 25 | `cargo test -p paladin-ai --lib kept_set_equivalence_snapshot_pre_resolver` (D-20 regression, Phase 32 PRIM-04) | `running 1 test` — `kept_set_equivalence_snapshot_pre_resolver ... ok`. `test result: ok. 1 passed; 0 failed` | ✅ PASS (non-zero passed count) |
| 26 | `cargo doc --workspace --no-deps` (the exact `lint` job "Check documentation" command, zero-tolerance) | **73** `warning:` lines (up from the corpus audit §8's 72, measured 2026-09-16 same-day baseline; +1 net drift over the intervening period, not attributable to this phase — see below) | ⚠️ CARRIED, pre-existing, **not a gate** (Phase 29 §8, Phase 32 32-05 precedent) |
| 27 | `grep -c '^## \[Unreleased\]' CHANGELOG.md` | `0` | ✅ PASS |
| 28 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'rag'` | `17` | ✅ PASS |
| 29 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -c 'TokenUsage'` | `4` | ✅ PASS |
| 30 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -c 'Commissary'` | `15` | ✅ PASS |
| 31 | `grep -c 'v0.11.0' CHANGELOG.md` / `MIGRATION.md` | `0` / `0` | ✅ PASS |
| 32 | 82% coverage floor (CI's exact `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1`, via `make coverage`) | **Not measurable locally.** `command -v docker` → `docker: command not found` (exit 1); `redis-cli -h 127.0.0.1 -p 6379 ping` → connection refused; `curl http://127.0.0.1:9000/minio/health/live` → no response (`000`). `make coverage` was attempted and hung past 60s during `scripts/coverage.sh`'s service-probe chain (hostname resolution for the `redis`/`minio` compose-network aliases, which do not exist outside a Docker network) and was terminated — it does not fail fast here the way it does when a `TEST_REDIS_HOST` override is set and simply unreachable; with no override and no compose network, the probe stalls. **CI-attributed**: the CI `coverage` job (`.github/workflows/ci.yml`, `cargo llvm-cov` step) is the evidence source for this gate, to be read once this branch is pushed | ⚠️ CI-ATTRIBUTED (job: `coverage`) — see folded-todo answer below |

**Local sweep verdict: 30/32 unconditionally green, 1 carried pre-existing condition explicitly
recorded (row 26), 1 gate this devcontainer cannot run at all and is honestly labelled CI-attributed
rather than a claimed local pass (row 32). Zero new failures caused by this phase.**

### The `cargo doc --workspace --no-deps` condition — carried, not a gate

Re-measured live: **73 `warning:` lines**, one more than the corpus audit §8's 72 (measured
2026-09-16, same calendar day, likely a different HEAD within the day's own work). Grepped for every
symbol this phase's own plans introduced (`RagRetrievalResult`, `RagRetrievalError`, `ShedItem`,
`rag_omission_marker`, `with_token_counter`, `Commissary`) against the captured output: the only hit
is a single, pre-existing warning in `crates/paladin-llm/src/services/commissary.rs:89` (`public
documentation for 'pessimistic_tokens_per_1000_bytes' links to private item
'PESSIMISTIC_TOKENS_PER_1000_BYTES'`) — a Phase 30-vintage `Commissary` doc comment this phase did
not touch (no plan in this phase modifies `commissary.rs`; `git log --oneline -- crates/paladin-llm`
across 33-01 through 33-06 is empty). **Zero of the 73 warnings originate in a file this phase
created or modified.** Per Phase 29 §8 and the Phase 32 `32-05-SUMMARY.md` precedent, this condition
is recorded, not fixed, and explicitly **not a gate**: SHIP-04's own requirement text names only "no
NEW broken intra-doc links" plus green semver/MSRV (both true above), and no Phase 33 plan lists
`commissary.rs` or any of the other warning-bearing files under `files_modified`.

### The coverage floor — folded todo answered

**`2026-08-13-verify-local-coverage-reproduction.md`**, folded into this plan as a verification note
per `33-CONTEXT.md`'s Folded Todos entry: **the local run did not reproduce CI's figure, and could
not — for two independent reasons, both stated plainly rather than left implicit.** First, this
devcontainer has no Docker (`docker: command not found`) and no reachable Redis/MinIO on any of the
script's fallback addresses (`redis:6379`, `localhost:6380`, `minio:9000`, `localhost:9010`), so
`scripts/coverage.sh`'s service-probe chain has no target to resolve against and the command cannot
complete at all, let alone produce a comparable figure. Second, the todo's own text names a **stale**
target: `82.39%` is the v0.8.0-era workspace figure `.planning/decisions/0006-coverage-gate.md`
recorded on 2026-08-13 (the todo's own creation date) — every coverage measurement recorded since
(Phase 28: 90.28%, Phase 31: 90.30%/90.17%, Phase 32: 90.25%) has been in the low 90s, comfortably
above the 82% ADR-0006 floor the CI job actually gates on. Even a Docker-capable environment would
not be "reproducing 82.39%" today; it would be reproducing a ~90% figure. This answer is recorded as
a verification note, not a resolution — the todo keeps its pending, no-`resolves_phase` status
exactly as its own text and every prior phase's identical disposition (Phases 25, 26, 27, 28, 29, 31,
32) requires.

---

## CI-run table

**`feature/phase-33` has never been pushed** — `git ls-remote --heads origin feature/phase-33`
returns nothing at sweep time. No Phase 33 plan has a CI run of its own. The most recent available
run for any pushable workflow is on the sibling `feature/phase-32` branch, dated **before Phase 33
began** (Phase 33 execution started 2026-09-16 per `.planning/STATE.md`; this run completed
2026-09-16T14:44 UTC, roughly three hours before this plan's own sweep). This table proves the
pre-Phase-33 base was fully green; it does **not** prove anything about this phase's own five
commits, which is exactly why the Local sweep above re-runs every gate this devcontainer can run
against the actual Phase-33 tree at `69500c9b`.

| Workflow | Run ID (URL) | Conclusion | SHA | Notes |
|---|---|---|---|---|
| `.github/workflows/ci.yml` | `35110626010` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/35110626010 | **success** | `2940a9a99869aa4c541025412cfc75e43a66d49f` | 37 jobs, 0 failed (all `success` or `skipped` — `Publish Dry Run` and tag/manual-gated jobs skip by design, not failures); this is the Phase 32 close commit, the base `feature/phase-33` forked from |
| `.github/workflows/codeql.yml` | `35110626325` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/35110626325 | **success** | `2940a9a99869aa4c541025412cfc75e43a66d49f` | Advisory-only per `security.instructions.md` |
| `.github/workflows/feature-flags.yml` | `35110626123` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/35110626123 | **success** | `2940a9a99869aa4c541025412cfc75e43a66d49f` | All feature-flag legs, including `otel`/`dev-ui`/`llm-all` unification |
| `pre-commit` | `35110626100` — https://github.com/DF3NDR/paladin-dev-env/actions/runs/35110626100 | **success** | `2940a9a99869aa4c541025412cfc75e43a66d49f` | |
| `.github/workflows/docs.yml` | — none for `feature/phase-33` or this `feature/phase-32` commit — | **not run** | — | PR-gated (`pull_request` trigger only); no PR is open for either branch yet. `mdbook build docs/` was not re-run in this plan's own sweep (out of D-24's named gate list for this plan; Phase 29/32 both ran it as their own release-readiness check, but 33-CONTEXT.md D-24 does not list it among this phase's re-seal gates) |

**What this table proves:** the commit `feature/phase-33` forked from was fully green across every
pushable workflow before this phase's own five plans landed. **What it does NOT claim:** a CI run of
any kind on any Phase 33 commit — none exists, because the branch has not been pushed. The
orchestrator pushes this branch (or opens the PR) and appends the real pre-merge run table, then
later the post-merge run on the tagged `main` merge commit — exactly as `27-CI-EVIDENCE.md`,
`28-CI-EVIDENCE.md` and `29-CI-EVIDENCE.md` did for their own phases (Phase 29 D-21's two-SHA rule).
This plan does not push and does not run `gh run watch`, per every prior release-gate plan's own
operating instructions.

---

## Summary and what remains

**What this record proves:** every gate this devcontainer CAN run locally against the actual
Phase-33 tree (head `69500c9b`) is green or explicitly carried (32/32 accounted for: 30
unconditional passes, 1 named pre-existing carried condition, 1 honestly CI-attributed gate), including
all eleven publishable crates' semver checks (0 checks evaluated at the `0.9.0 -> 0.10.0` major-change
boundary, matching Phase 29's own precedent exactly), the MSRV floor, `make clean-code`, `make
security`, a non-empty twelve-crate dependency-ordered dry-run publish with zero test failures across
the full workspace, a drift-free API-surface baseline, both Phase 29 backward-compatibility test
targets, the Phase 32 PRIM-04 regression check (green, no code change), the CHANGELOG register
grep, and the D-19 exit grep (F6 closed, row 1-2). The base commit `feature/phase-33` forked from was
fully green across all four pushable workflows before this phase began.

**What this record does NOT claim:** a CI run of any kind on any Phase 33 commit, and a local
measurement of the 82% coverage floor — both gaps are structural (no push yet; no Docker in this
devcontainer), not a result this plan is hiding. The orchestrator pushes this branch and supplies
both the real pre-merge run and, later, the post-merge run on the tagged `main` merge commit.

---

*Phase: 33-commissary-in-tree-adoption*
*Written: 2026-09-16*
