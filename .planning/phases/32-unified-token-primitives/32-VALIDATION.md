---
phase: 32
slug: unified-token-primitives
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-15
validated: 2026-09-16
---

# Phase 32 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (libtest unit tests + rustdoc doctests) plus bash guard-script harnesses under `tests/scripts/*_test.sh` |
| **Config file** | `Cargo.toml` (workspace), `Makefile` (`test-*`, `check-gates`, `test-shell-guards`, `lint-shell`), `scripts/coverage.sh` |
| **Quick run command** | `cargo test -p paladin-ports --doc token_counter_port && cargo test -p paladin-memory --features content-processing --lib token_counter && cargo test -p paladin-llm --lib commissary && cargo test -p paladin-llm --lib window && cargo test -p paladin-ai --lib history && ./scripts/check-migration-allowlist.sh` |
| **Full suite command** | `make clean-code && cargo test --workspace --all-features --no-fail-fast && make test-shell-guards && make check-gates && mdbook build docs/` |
| **Estimated runtime** | Quick: ~7 s warm (79 tests + guard); full: several minutes (workspace build + clippy) |

---

## Sampling Rate

- **After every task commit:** Run the quick run command above
- **After every plan wave:** Run the full suite command above
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 7 seconds (quick command, warm cache)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 32-01-00 | 01 | 1 | PRIM-01, PRIM-02 | — | N/A — blocking decision checkpoint (D-08/D-09 one-way doors), resolved by the user on 2026-09-15 (32-01-SUMMARY "Checkpoint Decision") | checkpoint | N/A — human decision, exempt by design | — | ✅ resolved |
| 32-01-01 | 01 | 1 | PRIM-01, PRIM-02 | T-32-01 / T-32-02 / T-32-03 | Port default `is_exact()` is `false`; only `TiktokenCounter` overrides to `true`; `Stockpile.exact_tally` is read live from the injected port, never from a caller bool; `Debug` prints no secret | unit + doctest | `cargo test -p paladin-ports --doc token_counter_port && cargo test -p paladin-memory --features content-processing --lib is_exact && cargo test -p paladin-llm --lib commissary` | ✅ | ✅ green (1 + 2 + 20 passed, re-run 2026-09-16) |
| 32-01-02 | 01 | 1 | PRIM-02 | T-32-12 | Reader-facing docs describe the port-sourced exactness and the 4-arg constructor; no credential-shaped literal in new prose | docs build + grep | `! git grep -qn 'is_exact_counter' -- docs/src ':!docs/src/api-reference/upgrading.md' ':!docs/src/api-reference/migration-guide.md' && git grep -qn 'is_exact' -- docs/src/architecture/commissary.md && mdbook build docs/` (see note 1) | ✅ | ✅ green |
| 32-02-01 | 02 | 2 | PRIM-04 | T-32-13 | Pre-resolver equivalence snapshots (`window_and_allowance_equivalence_snapshot_pre_resolver`, `kept_set_equivalence_snapshot_pre_resolver`) committed green before any resolver code, so a later behavior change cannot hide | snapshot (unit) | `cargo test -p paladin-llm --lib commissary && cargo test -p paladin-ai --lib history` | ✅ | ✅ green (20 + 25 passed) |
| 32-02-02 | 02 | 2 | PRIM-04 | T-32-05 / T-32-06 / T-32-07 / T-32-08 | `WindowFallbackPolicy::Strict { caller_fallback: None }` returns `Err(UnknownContextWindow)` rather than inventing a window; `WindowSource::as_str` is exhaustive; lookup is a plain `HashMap::get`; error text embeds only the model name | unit | `cargo test -p paladin-llm --lib window` (14 tests: 4 `precedence_*`, `source_label_invariant_walks_every_variant`, edges) | ✅ | ✅ green (14 passed) |
| 32-03-01 | 03 | 2 | PRIM-03 | T-32-09 / T-32-10 | Inlined `count` path keeps the 1000-entry cache ceiling and never calls `get_bpe_from_model` per count; legacy trait/factory are deleted, not aliased | unit + rustdoc lint | `cargo test -p paladin-memory --features content-processing --lib token_counter && cargo clippy -p paladin-memory --all-targets --features content-processing -- -D warnings` (see note 2) | ✅ | ✅ green (19 passed); rustdoc half ❌ red on a pre-existing Phase 26 link only (note 2) |
| 32-03-02 | 03 | 2 | PRIM-03 | T-32-11 / T-32-12 | All four narrowed re-exports stay under intact `content-processing` cfg gates; zero references to the removed names outside the two migration pages | grep + compile + docs build | `! git grep -qnE '\bTokenCounterFactory\b\|garrison::TokenCounter\b\|is_exact_counter' -- crates src docs/src examples benches tests ':!docs/src/api-reference/upgrading.md' ':!docs/src/api-reference/migration-guide.md' && cargo check --workspace --all-features --all-targets && cargo check --workspace --all-targets && mdbook build docs/` | ✅ | ✅ green |
| 32-04-01 | 04 | 3 | PRIM-04 | T-32-13 / T-32-14 | `Commissary::new` calls the shared resolver exactly once under `Strict`; the resolver error maps to the pre-existing `UndeclaredContextWindow` variant with unchanged text; no ad-hoc `.or(` precedence walk survives | unit + grep | `cargo test -p paladin-llm --lib commissary && cargo test -p paladin-llm --lib window && ! git grep -qn '\.or(' -- crates/paladin-llm/src/services/commissary.rs` | ✅ | ✅ green |
| 32-04-02 | 04 | 3 | PRIM-04 | T-32-15 / T-32-16 | `HistoryTrimmer::resolve_limit` logs `WindowSource::as_str` (config/provider/default substrings preserved); explicit `Err` arm returns the configured default, no panic path; `LimitSource` deleted | unit + grep | `cargo test -p paladin-ai --lib history && ! git grep -qn 'LimitSource' -- src crates && git grep -qn 'resolve_context_window' -- src/application/services/paladin/middleware/history.rs` (25 tests incl. 3 `limit_resolution_*`) | ✅ | ✅ green (25 passed) |
| 32-05-01 | 05 | 4 | PRIM-05 | T-32-17 / T-32-18 | Every discovery run carries `--release-type minor` and reports a non-zero check count; the `content-processing` runs are the ones that observe the gated removal | tool run (slow, network baseline) | `cargo semver-checks check-release --package <crate> [--features content-processing] --baseline-version 0.9.0 --release-type minor` × 6, each asserted to print `[1-9][0-9]* checks` (32-05-PLAN Task 1 verify) | ✅ | ✅ green (6 runs recorded verbatim in 32-05-SUMMARY; CI `semver` job; not re-run in this audit — see note 3) |
| 32-05-02 | 05 | 4 | PRIM-05 | T-32-19 / T-32-20 | MIGRATION.md §9.2 deliberate-breaking `crate \| type` pairs are set-equal to `.cargo/semver-checks-allowlist.toml`; the `paladin-llm \| Commissary` row is `N/A` and correctly excluded; no credential-shaped literal in new register/changelog text | offline guard + docs build + grep | `./scripts/check-migration-allowlist.sh && mdbook build docs/ && grep -q '^## Token primitives' docs/src/api-reference/upgrading.md && grep -q '^### Token primitives' docs/src/api-reference/migration-guide.md` | ✅ (guard added by this audit) | ✅ green (15 = 15 pairs) |
| 32-05-03 | 05 | 4 | PRIM-05 | T-32-21 | Phase gates (fmt, clippy `-D warnings`, workspace tests, doctests, mdbook, security, 82% coverage floor) run and recorded, not asserted | full gate | `make clean-code && cargo test --workspace --all-features --no-fail-fast && cargo test -p paladin-ports --doc && mdbook build docs/ && make check-gates` (see note 4) | ✅ | ✅ green (32-05-SUMMARY + 32-VERIFICATION independent re-run of fmt/clippy/check/mdbook) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

### Notes on the map

1. **32-01-02 command corrected.** The plan's original grep (`! git grep -qn 'is_exact_counter' -- docs/src`) is red on the phase-final tree because plan 32-05 later added the two reader-facing migration pages, which are *required* to name the removed argument. Plan 32-05 Task 3's own exit grep already excludes exactly those two pages; the map records that corrected form. Intent (no stale usage in docs) holds.
2. **32-03-01 rustdoc half is red for a pre-existing reason.** `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-memory --features content-processing --no-deps` exits 101 with exactly one error: `unresolved link to HeuristicTokenCounter` in `crates/paladin-memory/src/token_counter/mod.rs:3`, introduced in Phase 26 (`69a56dd6`), outside every file Phase 32 modified, and logged in `deferred-items.md`. Re-run in this audit 2026-09-16 and confirmed to be the only error. PRIM-03 remains fully covered by the 19 unit tests, the exit grep and the workspace compile checks. Fixing the link is a one-line follow-up owned by a later phase.
3. **32-05-01 is slow and networked.** The six `cargo semver-checks` discovery runs need the `0.9.0` baseline and take minutes; they belong to the wave-level and CI cadence (`semver` job), not the per-commit quick command. `cargo-semver-checks 0.50.0` is installed in the devcontainer, so the command is reproducible locally when needed.
4. **Coverage floor is CI-automated but service-dependent locally.** `make coverage` mirrors CI's `coverage` job (`--fail-under-lines 82`) but requires `make services-up` (Redis/MinIO). 32-05-SUMMARY records a local measurement of 90.25% via that path; this audit did not re-measure. `cargo test --workspace --all-features` also carries one known, pre-existing red target (`cli_isolation::test_cli_feature_is_not_default`, an `--all-features` vs `cli`-isolation conflict carried from Phase 31, see `deferred-items.md`).

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No Wave 0 stubs, fixtures or framework installs were needed: every task's verify block runs on the workspace's stock `cargo test`, `git grep`, `mdbook` and `make` surfaces.

---

## Manual-Only Verifications

All phase behaviors have automated verification. The one human step in the phase (task 32-01-00) is a blocking design-decision checkpoint, not a verification, and is recorded as resolved in 32-01-SUMMARY.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (none were MISSING)
- [x] No watch-mode flags
- [x] Feedback latency < 10s (quick command ~7 s warm)
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-09-16

---

## Validation Audit 2026-09-16

| Metric | Count |
|--------|-------|
| Requirements audited | 5 (PRIM-01..05) |
| Tasks mapped | 12 (incl. 1 checkpoint) |
| Gaps found | 1 |
| Resolved | 1 |
| Escalated | 0 |

**Gap found and resolved.** PRIM-05's row-level register↔allowlist set-equality gate existed only as an inline `run:` step in `.github/workflows/ci.yml` (`semver` job). It could not be sampled locally after a task commit without hand-copying the workflow step, which is exactly what the 32-05 executor and the phase verifier each had to do. Resolved by adding a local mirror following the offline-guard house pattern:

| File | Purpose | Verified by |
|------|---------|-------------|
| `scripts/check-migration-allowlist.sh` | No-argument offline guard; awk/grep copied verbatim from the CI step; `MIGRATION_FILE`/`ALLOWLIST_FILE` env overrides for fixtures; scratch dir via `mktemp -d`; named non-zero on missing inputs | `./scripts/check-migration-allowlist.sh` → 15 = 15 pairs, exit 0 |
| `tests/scripts/check-migration-allowlist_test.sh` | Regression harness, 9 assertions, failing cases first (missing Y row, missing allowlist entry, missing inputs), then the pipe-inside-backticks row, the duplicate-row collapse, the both-empty baseline, the real tree, and a no-mutation check | `./tests/scripts/check-migration-allowlist_test.sh` → 9 passed; `make test-shell-guards` → all suites green |
| `Makefile` | `check-migration-allowlist` target, appended to the `check-gates` composite | `make check-gates` → exit 0 |

The inline CI step is unchanged and stays authoritative; `ci.yml`, `MIGRATION.md`, the allowlist and all implementation files were not modified.

**Audit finding on the auditor's own report.** The subagent reported `make lint-shell` clean, but that target lints `git ls-files '*.sh'`, so untracked new scripts are never scanned. A direct `shellcheck --severity=warning` on the two new files surfaced one error-severity finding: a wrapped comment line beginning `# shellcheck-clean.` is parsed as a malformed shellcheck directive (SC1073/SC1072) and aborts the parse. Reworded so the word no longer starts a line; both files re-linted clean, then staged so the make target actually covers them. Lesson recorded for future audits: stage new shell files before trusting `make lint-shell`.

**Independent re-runs performed by this audit (2026-09-16):** the five per-crate test commands (79 tests, ~7 s), every grep-based verify block from plans 01, 03, 04 and 05 on the phase-final tree, `mdbook build docs/`, the paladin-memory rustdoc lint (red, pre-existing cause confirmed), the new guard, its regression test, `make test-shell-guards`, `make lint-shell` and `make check-gates`.
