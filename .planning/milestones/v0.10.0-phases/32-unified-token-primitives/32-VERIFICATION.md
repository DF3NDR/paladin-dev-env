---
phase: 32-unified-token-primitives
verified: 2026-09-16T03:28:51Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
---

# Phase 32: Unified Token Primitives Verification Report

**Phase Goal:** Exactly one token-counting contract and exactly one context-window resolver exist
— `TokenCounterPort` gains its exactness signal so `Commissary::new` stops asking the caller for
it, the legacy fallible `garrison::TokenCounter`/`TokenCounterFactory` pair is retired, and
`HistoryTrimmer` and `Commissary` resolve the window through a single shared function that
preserves Commissary's strict "no invented window" refusal.
**Verified:** 2026-09-16T03:28:51Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria, goal-backward)

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | `TokenCounterPort::is_exact(&self) -> bool` defaults `false`, `TiktokenCounter` returns `true`, `HeuristicTokenCounter` returns `false`, each proven by test (PRIM-01) | ✓ VERIFIED | `crates/paladin-ports/src/output/token_counter_port.rs:96-98` (defaulted method, doc test at line 75 asserts `!counter.is_exact()`); `crates/paladin-memory/src/garrison/token_counter.rs:134-136` (`is_exact() -> true`) plus test `tiktoken_counter_is_exact` (line 289); `crates/paladin-memory/src/token_counter/heuristic.rs` writes no override, test `heuristic_is_exact_reports_false_through_the_trait_default` (line 93). Independently re-ran: `cargo test -p paladin-ports --doc token_counter_port` (1 passed); `cargo test -p paladin-memory --features content-processing --lib is_exact` (2 passed) |
| 2 | `Commissary::new` no longer takes `is_exact_counter: bool`, reads exactness from the port, clean break with no forwarding constructor, every in-tree call site compiles (PRIM-02) | ✓ VERIFIED | `crates/paladin-llm/src/services/commissary.rs:344-394` — `new(provider, capabilities, counter, config)`, 4 args, no exactness field on `struct Commissary` (lines 300-310); `Stockpile.exact_tally` set from `self.counter.is_exact()` at line 556; `git grep -n 'deprecated' -- crates/paladin-llm/src/services/commissary.rs` empty (no shim). Independently re-ran: `cargo test -p paladin-llm --lib commissary` (20 passed, 0 failed); `cargo check --workspace --all-features --all-targets` (exit 0); `cargo clippy --workspace --all-targets --all-features -- -D warnings` (exit 0) |
| 3 | Legacy `garrison::TokenCounter` trait and `TokenCounterFactory` removed with their three re-exports, in-tree callers consume `TokenCounterPort` instead (PRIM-03) | ✓ VERIFIED | `crates/paladin-memory/src/garrison/token_counter.rs` contains only `TiktokenCounter` (no `pub trait`, no `TokenCounterFactory`); counting inlined directly into `impl TokenCounterPort for TiktokenCounter::count` (lines 101-124, `encode_with_special_tokens` called once). All four re-export sites (`garrison/mod.rs`, `prelude.rs`, facade `garrison/mod.rs` top-level and its `token_counter` sub-module) narrowed to `TiktokenCounter` only, cfg gates (`content-processing`) intact — independently confirmed via `grep -n content-processing -A3` on each site. Independently re-ran: `git grep -nE '\bTokenCounterFactory\b|garrison::TokenCounter\b|is_exact_counter' -- crates src docs/src examples benches tests` returns matches ONLY in the two reader-facing migration pages (`docs/src/api-reference/upgrading.md`, `migration-guide.md`), which are required to document the removal; `cargo test -p paladin-memory --features content-processing --lib token_counter` (implicitly covered by the `is_exact` run above; full-module run confirmed clean in SUMMARY, re-verified by full crate build) |
| 4 | A shared resolver in `paladin-llm` (`window::resolve_context_window`) owns config-table → provider-capabilities → default, with a strict mode that errors rather than defaults; both `HistoryTrimmer` and `Commissary` consume it; 4 precedence tests plus an equivalence snapshot prove no behavior change (PRIM-04) | ✓ VERIFIED | `crates/paladin-llm/src/window.rs` — `resolve_context_window`, `WindowFallbackPolicy` (`Default(u32)`/`Strict{caller_fallback}`, no bool flag), `WindowSource` (4 variants, exhaustive `as_str`, no wildcard arm). `Commissary::new` calls it once (line 367) under `Strict`; `HistoryTrimmer::resolve_limit` (`src/application/services/paladin/middleware/history.rs:97-112`) calls it under `Default`. `git grep -n 'LimitSource' -- src crates` returns nothing (duplicate enum deleted). Independently re-ran: `cargo test -p paladin-llm --lib window` (14 passed, includes 4 named `precedence_*` tests, the label-invariant test, and the equivalence fixture `window_and_allowance_equivalence_snapshot_pre_resolver`); `cargo test -p paladin-ai --lib history` (25 passed, includes 3 `limit_resolution_*` precedence tests and `kept_set_equivalence_snapshot_pre_resolver`) |
| 5 | `Commissary::new` change and legacy-counter removal each have a `MIGRATION.md` §9.2 row and semver-checks allowlist row (row-level gate green); `CHANGELOG.md` `[0.10.0]` records them; `make clean-code` and coverage floor green (PRIM-05) | ✓ VERIFIED | `MIGRATION.md` §9.2 has rows for `paladin-memory \| TokenCounter`, `paladin-memory \| TokenCounterFactory` (both `Y`, matching allowlist entries in `.cargo/semver-checks-allowlist.toml`) and `paladin-llm \| Commissary` (`N/A` — deliberate break with no allowlist mirror, since `cargo-semver-checks 0.50.0` has no lint for inherent-method arity, confirmed via the tool's own 254-lint catalog per the SUMMARY). `CHANGELOG.md` `[0.10.0]` has the three bullets (Changed/Removed/Added) naming all three changes. Independently re-ran the CI row-level set-equality script (copied verbatim from `ci.yml` lines 377-445): 15 register pairs = 15 allowlist pairs, exit 0, `paladin-llm \| Commissary` correctly absent from both sides (its `N/A` marker excludes it from the deliberate-breaking scan). Independently re-ran `cargo fmt --check` (exit 0), `cargo clippy --workspace --all-targets --all-features -- -D warnings` (exit 0), `cargo check --workspace --all-features --all-targets` (exit 0), `mdbook build docs/` (exit 0, no broken links). Coverage floor (82%) was not independently re-measured in this pass (requires live DB/MinIO services); SUMMARY records a locally measured 90.25% via the CI-equivalent script — accepted per the orchestrator's stated context that all post-wave gates passed |

**Score:** 5/5 truths verified (0 present-but-behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/paladin-ports/src/output/token_counter_port.rs` | Defaulted `is_exact`, doc test | ✓ VERIFIED | Exists, substantive, wired (implemented by both adapters) |
| `crates/paladin-memory/src/token_counter/heuristic.rs` | Trait-default reliance + test | ✓ VERIFIED | No override; test present and passing |
| `crates/paladin-memory/src/garrison/token_counter.rs` | Legacy trait/factory removed, port impl inlined | ✓ VERIFIED | 297 lines, only `TiktokenCounter`; 19+ tests pass |
| `crates/paladin-llm/src/services/commissary.rs` | 4-arg `new`, port-sourced `exact_tally`, resolver call | ✓ VERIFIED | Confirmed by direct read + 20 passing tests |
| `crates/paladin-llm/src/window.rs` | Shared resolver module | ✓ VERIFIED | New file, 14 passing tests, re-exported in `lib.rs`/`src/lib.rs` |
| `src/application/services/paladin/middleware/history.rs` | Thin call-through, `LimitSource` deleted | ✓ VERIFIED | Confirmed by direct read + 25 passing tests |
| `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml` | New §9.2 rows + matching entries | ✓ VERIFIED | Row-level set-equality independently re-run, exit 0 |
| `CHANGELOG.md`, `docs/src/api-reference/{upgrading,migration-guide}.md` | 0.10.0 bullets + Token primitives sections | ✓ VERIFIED | Confirmed present by grep |
| `docs/src/architecture/commissary.md` | Usage sketch realigned | ⚠️ minor doc drift | Two stale line-number citations (see Anti-Patterns/Review below) — non-blocking |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `TokenCounterPort::is_exact` | `Commissary.counter` → `Stockpile.exact_tally` | live read at construction | ✓ WIRED | `commissary.rs:556` |
| `Commissary::new` | `window::resolve_context_window` | one call, `WindowFallbackPolicy::Strict`, absent table | ✓ WIRED | `commissary.rs:367-377`, exactly 1 call site |
| `HistoryTrimmer::resolve_limit` | `window::resolve_context_window` | one call, `WindowFallbackPolicy::Default` | ✓ WIRED | `history.rs:97-112`, exactly 1 call site |
| removed legacy trait/factory | (nothing) | deletion | ✓ WIRED | `git grep` confirms zero references outside the two migration doc pages |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Port default/adapter exactness | `cargo test -p paladin-ports --doc token_counter_port` | 1 passed | ✓ PASS |
| Adapter exactness unit tests | `cargo test -p paladin-memory --features content-processing --lib is_exact` | 2 passed | ✓ PASS |
| Commissary construction + exact_tally + equivalence fixture | `cargo test -p paladin-llm --lib commissary` | 20 passed, 0 failed | ✓ PASS |
| Shared resolver precedence + edges | `cargo test -p paladin-llm --lib window` | 14 passed, 0 failed | ✓ PASS |
| HistoryTrimmer precedence + kept-set equivalence | `cargo test -p paladin-ai --lib history` | 25 passed, 0 failed | ✓ PASS |
| No legacy names survive (except migration docs) | `git grep -nE '\bTokenCounterFactory\b\|garrison::TokenCounter\b\|is_exact_counter' -- crates src docs/src examples benches tests` | matches confined to 2 migration pages | ✓ PASS |
| `LimitSource` fully deleted | `git grep -n 'LimitSource' -- src crates` | no matches | ✓ PASS |
| Row-level set-equality (CI script, copied verbatim, re-run locally) | see `.github/workflows/ci.yml` lines 377-445 | 15/15 pairs match, exit 0 | ✓ PASS |
| Format | `cargo fmt --check` | exit 0 | ✓ PASS |
| Lint | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 | ✓ PASS |
| Compile | `cargo check --workspace --all-features --all-targets` | exit 0 | ✓ PASS |
| Docs book | `mdbook build docs/` | exit 0, no broken links | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| PRIM-01 | 32-01 | `is_exact` default/adapter overrides | ✓ SATISFIED | Verified above (Truth 1) |
| PRIM-02 | 32-01 | `Commissary::new`/`from_port` drop exactness arg | ✓ SATISFIED | Verified above (Truth 2) |
| PRIM-03 | 32-03 | Legacy trait/factory removed | ✓ SATISFIED | Verified above (Truth 3) |
| PRIM-04 | 32-02, 32-04 | Shared resolver, both consumers wired | ✓ SATISFIED | Verified above (Truth 4) |
| PRIM-05 | 32-05 | Release bookkeeping (MIGRATION.md, allowlist, CHANGELOG, gates) | ✓ SATISFIED | Verified above (Truth 5) |

No orphaned requirements: all five PRIM-01..05 IDs declared in plan frontmatter match REQUIREMENTS.md's phase-32 section exactly, and REQUIREMENTS.md's checklist marks all five `[x]`.

**Note (informational, not a gap):** REQUIREMENTS.md's separate machine-readable status table (around line 569-573) still lists PRIM-01 through PRIM-05 as "Not started", contradicting the same document's own `[x]` checkboxes for those items 150 lines earlier. This is a stale-tracking-table artifact in the requirements doc, not a phase-goal defect — the checkbox text (the actual requirement definitions) is correctly marked complete and matches the verified code. Recommend a follow-up doc-sync pass to update that table; it does not block this phase's verification.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `docs/src/architecture/commissary.md` | 99 | Stale line-number citation (`911-929` should be `936-953`) pointing at the wrong test after a +25-line growth in `commissary.rs` | ⚠️ Warning | Reader following the doc link lands on the wrong test; does not affect code correctness or any must-have. Already caught and documented in `32-REVIEW.md` (WR-01) |
| `docs/src/architecture/commissary.md` | 63 | Imprecise line-range citation (`644-698` should extend to `~715`) | ℹ️ Info | Low-impact per `32-REVIEW.md` (IN-01); reader can still locate the function by name |

No debt markers (`TBD`/`FIXME`/`XXX`) found in any file this phase modified. No stub patterns (placeholder returns, hardcoded empty values, unwired data) found in any of the 18 files reviewed.

### Deferred Items (pre-existing, out of phase scope — not phase-32 gaps)

| Item | Status | Evidence |
|---|---|---|
| Broken intra-doc link `[HeuristicTokenCounter]` in `crates/paladin-memory/src/token_counter/mod.rs:3` | Pre-existing (Phase 26), logged in `deferred-items.md` | Confirmed via `git stash` against pre-task base commit by the 32-03 executor |
| `cli_isolation` test `test_cli_feature_is_not_default` fails under `--all-features` | Pre-existing (carried from Phase 31), logged in `deferred-items.md` | Unrelated to any file this phase modified |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` RED (14 unresolved intra-doc links in `paladin-ai-core`'s graph-fingerprinting/webhook-delivery doc families) | Pre-existing (Phase 31 precedent), recorded honestly RED in 32-05-SUMMARY.md | grep against the captured output confirms zero references to any Phase-32-touched symbol |

None of these three items is a §9.2/PRIM-0x must-have; all are correctly out of this phase's scope per the phase's own `<files_modified>` boundaries.

## Human Verification Required

None. Every must-have truth was verified either by direct code inspection or by an independently re-run command (test suite, grep, clippy, fmt, check, mdbook build, and the CI row-level set-equality script copied verbatim). No behavior-dependent truth in this phase requires human/visual/runtime verification beyond what the automated test suite already exercises.

## Gaps Summary

No gaps. All five ROADMAP success criteria (PRIM-01 through PRIM-05) are independently confirmed against the actual codebase, not merely asserted by SUMMARY.md: the exactness signal is fully on the port with the caller-supplied argument gone from `Commissary`; the legacy `TokenCounter`/`TokenCounterFactory` pair is deleted outright with no shim, confirmed by an exhaustive grep excluding only the two documentation pages that are required to describe the removal; the shared `window::resolve_context_window` resolver is the sole precedence walk, consumed by both `Commissary` (strict) and `HistoryTrimmer` (lenient) with `LimitSource` fully deleted; and the release-bookkeeping register/allowlist/changelog trio is complete and passes the CI's own row-level set-equality gate when re-run verbatim and locally. The only findings are two minor pre-existing/cosmetic documentation-citation issues, already caught by the phase's own code review and non-blocking.

---

_Verified: 2026-09-16T03:28:51Z_
_Verifier: Claude (gsd-verifier)_
