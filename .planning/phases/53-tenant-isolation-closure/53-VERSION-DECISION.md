# Phase 53 version decision (v0.11.1 patch versus v0.12.0)

Plan 53-08, success criterion 5, requirement R9, CONTEXT D-02. This file records the patch-versus-minor
decision for the Dogfood Cleanup milestone on measured evidence taken on the phase head named below.

## Semver evidence

- **Head SHA measured:** `4ef9ddd2aa7886f3e73041e0927b27c41801fc3b` (`4ef9ddd2a`), branch `claude/laughing-dirac-e0h2ax`,
  working tree clean at the start and end of the run.
- **Date:** 2026-10-31 (run started 15:06:34Z; semver leg 15:06:44Z to 15:11:55Z).
- **Source of the lines:** local reproduction of the CI job `Semver Checks (vs v0.11.0)` with the same script CI runs
  (not the PR job; the blocking checkpoint of Task 2 did not fire).
- **Command:** `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh` (`cargo semver-checks check-release --package <pkg>
  --default-features --baseline-version 0.11.0 --release-type patch` for each of the twelve published packages).
- **Classification: GREEN** (twelve package lines, each with a non-zero check count and no `fail`; verdict `0 failure(s)`).
  Script exit status 0. Raw log: `/tmp/claude-0/-home-user-paladin-dev-env/c62b0292-6a28-5efd-8101-f37a9af13e46/scratchpad/p53-08-head4ef9/semver.log`.

Lines copied verbatim from the script's output:

```
semver: paladin-ai-core: 223 checks: 223 pass, 31 skip
semver: paladin-ports: 223 checks: 223 pass, 31 skip
semver: paladin-herald: 223 checks: 223 pass, 31 skip
semver: paladin-battalion: 223 checks: 223 pass, 31 skip
semver: paladin-llm: 223 checks: 223 pass, 31 skip
semver: paladin-memory: 223 checks: 223 pass, 31 skip
semver: paladin-web: 223 checks: 223 pass, 31 skip
semver: paladin-notifications: 223 checks: 223 pass, 31 skip
semver: paladin-content: 223 checks: 223 pass, 31 skip
semver: paladin-storage: 223 checks: 223 pass, 31 skip
semver: paladin-eval: 223 checks: 223 pass, 31 skip
semver: paladin-ai: 223 checks: 223 pass, 31 skip
semver: 12 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)
```

The per-package counts equal Phase 52's green baseline (`223 checks: 223 pass, 31 skip`,
`52-SEMVER-RED-EVIDENCE.md`): phase 53 added public items (inherent methods, `pub` consts) and no lint fired.

## Gate sweep

All six commands ran to completion on head `4ef9ddd2a` (logs under
`/tmp/claude-0/-home-user-paladin-dev-env/c62b0292-6a28-5efd-8101-f37a9af13e46/scratchpad/p53-08-head4ef9/`):

| Gate | Result line (exit status) |
|---|---|
| `make check-migration-allowlist` | `Allowlist is set-equal to the MIGRATION.md §11.2 deliberate-breaking register (crate|type pairs).` and `Every crate-wide cargo-semver-checks allow is mirrored by an allowlist entry (crate|lint pairs).` (exit 0) |
| `make check-gates` | `✅ OK: 12 crate(s) checked in publish order; every versioned workspace dependency (normal, dev and build) resolves to an earlier CRATES position, and CRATES exactly matches the cargo-metadata publishable set.` (exit 0) |
| `make security` | `advisories ok, bans ok, licenses ok, sources ok` (exit 0) |
| `make api-surface` | `✅ API surface unchanged` (4325 items) (exit 0) |
| `make clean-code` | `✅ doc-check passed (ADR-0033)` then `All 105 D-05 public API entry points carry a plural '# Examples' heading.` (exit 0; fmt, clippy, check and doc-check all passed) |
| `cargo test --workspace --no-fail-fast` | 62 test-result lines, 7774 passed, 229 ignored, 2 failed (exit 101): the only failures are the two known, pre-existing, network-bound tests `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `...::build_run_api_persists_run_traces_when_trace_persist_is_set` (`the run never reached a terminal status`, `run_api_wiring.rs:3036`), owned by TESTS-01 / Phase 55 and untouched by this phase; every other test result line is `ok`, including `openapi::tests::openapi_matches_committed_baseline` |

Known exceptions (recorded, not fixed, skipped or deleted): the two `build_run_api_persists_*` tests above. No other gate
is red.

- **coverage:** CI `Coverage` job (`cargo llvm-cov --fail-under-lines`, 82 % workspace floor, ADR-0006). `cargo-llvm-cov` is
  not installed in this environment, so this is CI-only evidence and is not claimed locally.
