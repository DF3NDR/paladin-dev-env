---
phase: 46-docs-currency-hygiene
fixed_at: 2026-10-24T00:00:00Z
review_path: /home/user/paladin-dev-env/.planning/phases/46-docs-currency-hygiene/46-REVIEW.md
iteration: 1
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 46: Code Review Fix Report

**Fixed at:** 2026-10-24
**Source review:** /home/user/paladin-dev-env/.planning/phases/46-docs-currency-hygiene/46-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 3 (Critical: 0, Warning: 3; Info findings out of scope)
- Fixed: 3
- Skipped: 0

## Fixed Issues

### WR-01: Maneuver warning and "adds no bound of its own" docs are wrong once an explicit `ManeuverConfig` is set, and the documented M-B-11 remediation triggers exactly that

**Files modified:** `crates/paladin-battalion/src/commander.rs`, `MIGRATION.md`, `docs/src/api-reference/upgrading.md`
**Commit:** df85c999
**Status:** fixed: requires human verification (behavioural logic change)
**Applied fix:**
- Warning gating: new private `Commander::maneuver_warned_aegis_policies()` returns the ignored policies only on the aegis-derived path (no explicit `ManeuverConfig`, or a config materialised from aegis by `error_strategy`). The `warn!` in the Maneuver arm now uses it, so an explicit `maneuver_config(..)` draws no warning.
- Remediation: `CommanderBuilder::error_strategy` no longer does `unwrap_or_default()` (which imposed `ManeuverConfig::default()`'s 300 s timeout). It now records the strategy and `build()` lays it over `default_maneuver_config()` (aegis-derived timeout kept, an unset timeout stays unbounded), or over an explicit config if one was set. `maneuver_config(..)` called after `error_strategy(..)` still wins outright, and the reverse order still layers the strategy over the explicit config, as before. A private `maneuver_config_from_aegis` field on `Commander` (same pattern as the existing private `strategy_selection`) keeps the warning applying to the `error_strategy`-only case.
- Rustdoc updated (Commander feature list, `CommanderBuilder` defaults, `build()`, `error_strategy`, `maneuver_config`, `default_maneuver_config`) to state the explicit-config exception and the replacement semantics.
- MIGRATION.md M-B-11 (frozen section 10): minimal correctness edit to the row text only. The warning clause now says "whose config is derived from `aegis`" (the gating makes the old unconditional wording false), and the remediation clause records that `error_strategy` keeps the aegis-derived timeout while `maneuver_config(..)` replaces the whole config. The Upgrading page row condenses the same two points. Nothing restructured.
- Tests added: `maneuver_warning_fires_only_on_the_derived_config_path`, `error_strategy_keeps_the_aegis_derived_maneuver_timeout` (45 s kept; unset stays `None`, not 300 s), `error_strategy_and_maneuver_config_compose_in_call_order`. The existing `maneuver_ignored_aegis_policies_names_only_the_fields_maneuver_drops` and `test_commander_builder_with_error_strategy` remain green.
- Verification: `cargo fmt --all -- --check`, `cargo clippy -p paladin-battalion --all-targets -- -D warnings`, `cargo test -p paladin-battalion --lib commander` (65 passed), `make check-changelogs`, `make check-migration-allowlist`, `make api-surface` (unchanged, 4323 items).

### WR-02: Two release-note sentences still say a v0.10 document with removed keys "still deserializes ... ignored", contradicting this phase's D-23 refusal

**Files modified:** `CHANGELOG.md`, `crates/paladin-core/CHANGELOG.md`
**Commit:** 065d4542
**Applied fix:** Rewrote the Phase 44 bullet in the Unreleased section of each file: a v0.10 document carrying a removed key is refused at load, naming the key and its `aegis.*` replacement (Phase 46, D-23; MIGRATION.md 10.1 M-B-06), and a document without them loads with `aegis` as `Aegis::default()`. `[0.10.1]` and older sections are untouched (the hunks sit above line 163 in the core file and 1850 in the root file). `make check-changelogs` and `make check-migration-allowlist` pass.

### WR-03: `execute_with_cancellation` rustdoc overstates what survives cancellation for `FirstSuccess`

**Files modified:** `crates/paladin-battalion/src/phalanx_service.rs`
**Commit:** 5186f128
**Applied fix:** Rewrote the paragraph per strategy, after confirming in the source that `execute_majority` delegates to `execute_collect_all` (which `tokio::spawn`s each Paladin) and `execute_first_success` drives un-spawned futures through `select_ok`. `CollectAll` and `Majority`: an in-flight attempt runs to its end and keeps its permit. `FirstSuccess`: attempts are dropped with the call at their next await point. The `max_concurrency` sentence is kept. Doc-only; fmt, clippy (`-D warnings`), `phalanx` lib tests (31 passed), `make api-surface` all clean.

## Skipped Issues

None.

## Notes

- Work was done on the main working tree as instructed by the caller (no separate worktree), so no recovery sentinel or temp branch exists.
- Not run, per the known environment facts: `make security` (pre-existing RUSTSEC-2026-0386 tokio advisory) and the network-dependent `paladin-ai --lib` `build_run_api_persists_*` tests.
- Info findings IN-01..IN-05 were out of scope and not touched.

---

_Fixed: 2026-10-24_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
