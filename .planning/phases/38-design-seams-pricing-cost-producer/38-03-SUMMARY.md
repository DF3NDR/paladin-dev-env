---
phase: 38-design-seams-pricing-cost-producer
plan: 03
subsystem: config
tags: [rust, config, treasurer, pricing, fixed-point, tdd, boot-validation]

# Dependency graph
requires:
  - phase: 38-design-seams-pricing-cost-producer (plan 02)
    provides: "Cost, CurrencyCode, PriceRow, PriceTable, cost_of_call in paladin-core; PricingLlmAdapter/with_pricing in paladin-llm (D-09 decorator shape)"
provides:
  - "TreasurerConfig/PriceRowConfig operator config section (treasurer:), Settings.treasurer + get_treasurer_config, hand-rolled exact-integer decimal-string parser (no rust_decimal, no f32/f64, no new dependency)"
  - "TreasurerConfig::price_table/validate: path-precise errors (treasurer.currency, treasurer.pricing.{model}.{axis}) for negative/malformed/too-fine/overflowing prices and bad currencies; empty-by-default, inert when omitted"
  - "with_pricing installed on both production run paths: build_agent_registry (config-loaded + runtime-provisioned agents via build_agent) and paladin_port_from_settings (run engine); FacadeProvisioner::with_treasurer builder"
  - "paladin-server boot-time validation: settings.get_treasurer_config().validate() runs immediately after Settings::load_from_file, before any agent/provider/engine is built"
affects: [38-04, 38-05, 38-06, 38-07, 38-08, 39-treasurer-ledger]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Config sub-struct mirrors AgentRuntimeConfig's idiom: Default + validate() + EnvOverridable, inert-when-omitted, config-file-only map (treasurer.pricing has no env form, matching ToolCallLimitConfig.per_tool)"
    - "Hand-rolled exact-integer decimal parser: split on '.', checked_mul/checked_add accumulation, right-pad fractional digits to 9 and add as remainder -- no floating point, no new crate"
    - "validate() == price_table().map(|_| ()) so validation and the table the wiring builds can never disagree"
    - "Price table built once per boot path (build_agent_registry, paladin_port_from_settings, provision) BEFORE any provider is resolved, so an invalid price fails hermetically"

key-files:
  created:
    - src/config/treasurer.rs
  modified:
    - src/config/mod.rs
    - src/config/settings.rs
    - src/config/user_config.rs
    - src/core/platform/mod.rs
    - config.example.yml
    - src/infrastructure/web/agent_host.rs
    - src/infrastructure/web/facade_provisioner.rs
    - src/bin/paladin-server.rs
    - .project/current-exports.txt
    - CHANGELOG.md

key-decisions:
  - "Hand-rolled exact-integer decimal parser instead of rust_decimal (CONTEXT.md Claude's Discretion #1): the grammar is deliberately narrow (unsigned digits, optional 1-9-digit fraction), exact integer arithmetic is simple to prove with the 12-test contract, and it keeps the dependency count flat -- verified via `grep -c rust_decimal Cargo.toml Cargo.lock` printing 0 for both files."
  - "Only treasurer.currency has an env override (APP_TREASURER_CURRENCY); treasurer.pricing is config-file only, mirroring ToolCallLimitConfig.per_tool (CONTEXT.md Claude's Discretion #3)."
  - "An empty price table installs no decorator at all (with_pricing returns Arc::ptr_eq the same inner port) -- verified by 38-02's existing with_pricing_on_an_empty_table_returns_the_same_arc test, reused unchanged by this plan's wiring (CONTEXT.md Claude's Discretion #3)."
  - "Price table is built and validated FIRST in every boot/build path (build_agent_registry, paladin_port_from_settings, FacadeProvisioner::provision) -- before any provider factory call -- so an invalid price fails hermetically without needing a working provider key."
  - "TDD gate followed literally for Task 1: RED commit (79e4b266) ships the full type/test module with parse_price_nanos_per_million and price_table deliberately stubbed (parser always Err(Malformed); price_table ignores self.pricing), producing 5/12 failing tests; GREEN commit (8b4456cd) replaces both stubs with the real implementation, all 12 pass."
  - "Regenerated .project/current-exports.txt with the CI-pinned PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 per CLAUDE.md's api-surface gate: 112 additive items (TreasurerConfig, PriceRowConfig, Settings.treasurer/get_treasurer_config, FacadeProvisioner::with_treasurer, the core::platform::container::cost re-export), 0 removals -- purely additive, no MIGRATION.md §9.2 row needed (D-00g)."

requirements-completed: [PRICE-01, PRICE-03]

coverage:
  - id: D1
    description: "An operator can write a treasurer: section (currency + per-model pricing map with prompt/completion required, cache_read/cache_write/reasoning optional) copied verbatim from a provider sheet, and it round-trips through Settings::load_from_file byte-identically for dotted/mixed-case model keys"
    requirement: PRICE-01
    verification:
      - kind: unit
        ref: "src/config/treasurer.rs#tests::default_treasurer_config_is_inert"
        status: pass
      - kind: unit
        ref: "src/config/treasurer.rs#tests::optional_axes_parse_and_default_to_parent"
        status: pass
      - kind: unit
        ref: "src/config/treasurer.rs#tests::model_keys_survive_loading_verbatim"
        status: pass
    human_judgment: false
  - id: D2
    description: "Omitting the treasurer section changes nothing: Settings.treasurer == TreasurerConfig::default(), validate() Ok, and config.example.yml's active treasurer: block round-trips to the same default"
    requirement: PRICE-01
    verification:
      - kind: unit
        ref: "src/config/treasurer.rs#tests::v0_10_config_resolves_treasurer_inert"
        status: pass
      - kind: unit
        ref: "src/config/treasurer.rs#tests::example_config_treasurer_block_round_trips_to_default"
        status: pass
    human_judgment: false
  - id: D3
    description: "A negative, malformed, too-fine (>9 decimal places), or overflowing price, a bad currency, an empty model key, a missing required axis, or an unknown axis name is rejected at config validation with a path-precise error, never silently accepted or reinterpreted"
    requirement: PRICE-01
    verification:
      - kind: unit
        ref: "src/config/treasurer.rs#tests::validate_rejects_malformed_prices"
        status: pass
      - kind: unit
        ref: "src/config/treasurer.rs#tests::validate_rejects_too_fine_and_overflowing_prices"
        status: pass
      - kind: unit
        ref: "src/config/treasurer.rs#tests::validate_rejects_bad_currency"
        status: pass
      - kind: unit
        ref: "src/config/treasurer.rs#tests::validate_rejects_empty_model_key"
        status: pass
      - kind: unit
        ref: "src/config/treasurer.rs#tests::missing_required_axis_or_unknown_axis_fails_to_load"
        status: pass
    human_judgment: false
  - id: D4
    description: "Exact integer decimal-string parsing: boundary values (0, 0.000000001, i64::MAX at 9223372036.854775807) parse exactly; APP_TREASURER_CURRENCY overrides the file value and is validated identically to a file value"
    requirement: PRICE-01
    verification:
      - kind: unit
        ref: "src/config/treasurer.rs#tests::parses_decimal_prices_exactly"
        status: pass
      - kind: unit
        ref: "src/config/treasurer.rs#tests::env_override_currency"
        status: pass
    human_judgment: false
  - id: D5
    description: "Both production run paths are priced whenever the table is non-empty: build_agent_registry (config-loaded + runtime-provisioned agents via build_agent) and paladin_port_from_settings (run engine) each wrap the resolved LlmPort with with_pricing, proven end-to-end with a real MockLlmAdapter stream reaching the exact cost 22_500_000 nanos USD"
    requirement: PRICE-03
    verification:
      - kind: unit
        ref: "src/infrastructure/web/agent_host.rs#tests::priced_agent_stream_reports_cost"
        status: pass
    human_judgment: false
  - id: D6
    description: "An invalid treasurer price aborts each build path with an error naming treasurer.pricing before any provider is resolved: build_agent_registry, paladin_port_from_settings, and FacadeProvisioner::provision"
    requirement: PRICE-01
    verification:
      - kind: unit
        ref: "src/infrastructure/web/agent_host.rs#tests::build_agent_registry_rejects_invalid_treasurer_price"
        status: pass
      - kind: unit
        ref: "src/infrastructure/web/facade_provisioner.rs#tests::paladin_port_from_settings_rejects_invalid_treasurer_price"
        status: pass
      - kind: unit
        ref: "src/infrastructure/web/facade_provisioner.rs#tests::provisioner_rejects_invalid_treasurer_price"
        status: pass
    human_judgment: false
  - id: D7
    description: "paladin-server refuses to start with 'invalid treasurer configuration: ...' before building any agent, engine, or provider, when the config carries a bad price"
    requirement: PRICE-01
    verification:
      - kind: manual_procedural
        ref: "src/bin/paladin-server.rs: settings.get_treasurer_config().validate() placed immediately after Settings::load_from_file and before build_agent_registry; grep-verified ordering (line 84 between lines 77 and 89), and cargo build -p paladin-ai --features web-server --bin paladin-server succeeds"
        status: pass
    human_judgment: true
    rationale: "No integration test spawns the actual paladin-server binary against a malformed config.yml and asserts process exit; the ordering and message-format claim is verified by source inspection and the unit-level HostBuildError/ProvisionError tests (D6) that share the exact same TreasurerConfig::validate() call. A human running `PALADIN_CONFIG=<bad-price-config> paladin-server` and observing the refusal would close this gap fully."

duration: ~23min
completed: 2026-09-26
status: complete
---

# Phase 38 Plan 03: Config Seam — Operator Price Table & Boot-Validated Pricing Wiring Summary

**Operator-writable `treasurer:` price table with a hand-rolled exact-decimal parser (no `rust_decimal`, no floats), boot-time validation, and `with_pricing` installed on both production run paths (config-loaded agents, runtime-provisioned agents, and the run engine).**

## Performance

- **Duration:** ~23 min
- **Started:** 2026-09-26T00:40:00Z
- **Completed:** 2026-09-26T01:03:00Z
- **Tasks:** 2 (Task 1: TDD config section; Task 2: production wiring + boot validation)
- **Files modified:** 10 (1 created, 9 modified, plus the generated API surface baseline and CHANGELOG)

## Accomplishments

- **Task 1 (`79e4b266` RED, `8b4456cd` GREEN):** `src/config/treasurer.rs` gives operators a
  `treasurer:` config section — one ISO currency plus a `pricing` map keyed by bare model name,
  each row carrying required `prompt`/`completion` and optional `cache_read`/`cache_write`/
  `reasoning` as decimal strings per 1M tokens. A hand-rolled `parse_price_nanos_per_million`
  parses the narrow grammar (digits, optional `.` plus 1-9 fractional digits) with pure
  `checked_mul`/`checked_add` integer arithmetic — no `rust_decimal`, no `f32`/`f64` anywhere in
  the file (grep-verified: 0 hits for both). `TreasurerConfig::price_table`/`validate` reject a
  negative, malformed, too-fine (>9 decimal places), or overflowing price and a bad currency with
  a path-precise message naming the full `treasurer.pricing.{model}.{axis}` location; `validate()`
  is exactly `price_table().map(|_| ())` so the two can never disagree. `deny_unknown_fields` and
  required (non-`Option`) `prompt`/`completion` fields mean a typo'd axis or a missing required
  field fails to load via serde, before validation ever runs. Followed the RED/GREEN TDD gate
  literally: the RED commit ships the type/test module with both core functions deliberately
  stubbed (parser always `Err(Malformed)`, `price_table` ignoring `self.pricing`), producing
  exactly the 5 expected failures out of 12 named tests; the GREEN commit replaces both stubs with
  the real implementation and all 12 pass.
- **Task 2 (`0289d487`):** `with_pricing` (from 38-02) is now installed on both production `LlmPort`
  resolution sites the research named: `build_agent` (shared by config-load's
  `build_agent_registry` and `FacadeProvisioner`'s runtime `POST /agents` path) and
  `paladin_port_from_settings` (the run engine's shared `PaladinPort`). Each site builds its price
  table from `settings.get_treasurer_config().price_table()` **before** resolving any provider, so
  an invalid price aborts hermetically with an error naming `treasurer.pricing` — proven for
  `build_agent_registry`, `paladin_port_from_settings`, and `FacadeProvisioner::provision`.
  `paladin-server`'s `run()` additionally validates the treasurer config immediately after
  `Settings::load_from_file`, before any agent, provider or engine is constructed, refusing to
  start with `"invalid treasurer configuration: ..."`. `FacadeProvisioner` gained a `treasurer`
  field and `with_treasurer` builder so `from_settings` and a hand-built provisioner both price
  identically. An end-to-end test (`priced_agent_stream_reports_cost`) proves the exact
  composition `build_agent` performs: a real `MockLlmAdapter` stream wrapped by `with_pricing`
  reaches `ChunkMetadata.cost` at exactly 22,500,000 nanos USD for 1,000 prompt / 2,000 completion
  tokens.
- **Post-implementation gate (`7d45a0d2`):** `cargo fmt --check` flagged formatting drift across
  all three touched files; ran `cargo fmt`, re-ran the full test + clippy suite (no regressions),
  then regenerated `.project/current-exports.txt` with the CI-pinned
  `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20` per CLAUDE.md's `make api-surface` gate (112 additive
  items, 0 removals — purely additive per D-00g, no `MIGRATION.md` §9.2 row) and added the
  corresponding `CHANGELOG.md [Unreleased]` entry.

## Task Commits

Each task was committed atomically:

1. **Task 1 (RED): add failing tests for treasurer price-table config** - `79e4b266` (test)
2. **Task 1 (GREEN): implement exact decimal parsing and price-table validation** - `8b4456cd` (feat)
3. **Task 2: install pricing on both production run paths with boot validation** - `0289d487` (feat)
4. **Post-task gate: cargo fmt, API surface baseline, CHANGELOG** - `7d45a0d2` (chore)

**Plan metadata:** (this commit)

_Note: Task 1 used the plan's `tdd="true"` RED/GREEN split with two commits; Task 2 is a single
`type="auto"` commit; the fourth commit is a deviation (CLAUDE.md's mandatory `cargo fmt` +
`make api-surface` pre-commit gate), documented below._

## Files Created/Modified

- `src/config/treasurer.rs` - `TreasurerConfig`, `PriceRowConfig`, `parse_price_nanos_per_million`,
  `price_table()`/`validate()`, `EnvOverridable` impl, 12 named tests
- `src/config/mod.rs` - `pub mod treasurer;` + `pub use ...::{PriceRowConfig, TreasurerConfig};`
- `src/config/settings.rs` - `Settings.treasurer`, `get_treasurer_config()`, `validate()` extended
- `src/config/user_config.rs` - test-fixture `Settings` literal gains `treasurer: ...::default()`
- `src/core/platform/mod.rs` - `pub use paladin_core::platform::container::cost;`
- `config.example.yml` - documented `treasurer:` section (active `currency`/`pricing: {}`, two
  commented example rows)
- `src/infrastructure/web/agent_host.rs` - `build_agent` gains `price_table` param + `with_pricing`
  call; `build_agent_registry` builds/validates the table first; 2 new tests
- `src/infrastructure/web/facade_provisioner.rs` - `FacadeProvisioner.treasurer` +
  `with_treasurer`; `paladin_port_from_settings` and `provision` build the table first; 2 new tests
- `src/bin/paladin-server.rs` - boot-time `settings.get_treasurer_config().validate()`
- `.project/current-exports.txt` - regenerated public API baseline (112 additive items)
- `CHANGELOG.md` - `[Unreleased]` entry for this plan's operator-facing config surface

## Decisions Made

- Hand-rolled exact-integer decimal parser instead of `rust_decimal` (no new dependency, narrow
  grammar, simple to prove exactly).
- `treasurer.pricing` is config-file only (no env override); only `treasurer.currency` has one
  (`APP_TREASURER_CURRENCY`).
- Price table built and validated first in every boot/build path, before any provider factory
  call, so an invalid price fails hermetically.
- TDD RED/GREEN gate followed literally for Task 1 (stubbed functions in the RED commit, real
  implementation in GREEN).
- API surface baseline regenerated and a CHANGELOG entry added per CLAUDE.md's mandatory
  pre-commit gate (documented as a deviation below).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `cargo fmt --check` failures across all three touched files**
- **Found during:** Post-Task-2 verification (before final plan close-out)
- **Issue:** Several multi-line expressions (a `parse().map_err()` chain, a `BTreeMap::from`
  literal, two `assert!(matches!(...))` calls, and a `Result`-wrapping closure in `provision`)
  were written in a style `rustfmt` collapses/reflows differently, so `cargo fmt --check` failed —
  a hard gate under CLAUDE.md's "before committing a parent task" protocol.
- **Fix:** Ran `cargo fmt`, then re-ran the full `config::treasurer` + `infrastructure::web` test
  suite and `cargo clippy --all-targets --features web-server -- -D warnings` to confirm no
  behavioral regression from the reformat.
- **Files modified:** `src/config/treasurer.rs`, `src/infrastructure/web/agent_host.rs`,
  `src/infrastructure/web/facade_provisioner.rs`
- **Verification:** `cargo fmt --check` exits clean; all 12 `config::treasurer` tests and all 22
  `infrastructure::web` tests still pass; clippy clean.
- **Committed in:** `7d45a0d2`

**2. [Rule 2 - Missing Critical] `make api-surface` drift not refreshed, no CHANGELOG entry**
- **Found during:** Post-Task-2 verification
- **Issue:** This plan adds public facade items (`Settings.treasurer`, `TreasurerConfig`,
  `PriceRowConfig`, `FacadeProvisioner::with_treasurer`, the `core::platform::container::cost`
  re-export). CLAUDE.md's completion protocol requires `make api-surface` to pass before
  committing a parent task, and any intentional surface change to be refreshed via
  `make api-surface-update` plus a `CHANGELOG.md` entry — neither had been done yet.
- **Fix:** Ran `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 ./scripts/check-api-surface.sh` first to
  confirm the diff was purely additive (0 removals, 112 additions), then
  `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 ./scripts/extract-public-api.sh
  .project/current-exports.txt` to refresh the baseline, and added the corresponding
  `CHANGELOG.md [Unreleased]` bullet.
- **Files modified:** `.project/current-exports.txt`, `CHANGELOG.md`
- **Verification:** `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 ./scripts/check-api-surface.sh
  .project/current-exports.txt` now reports "API surface unchanged".
- **Committed in:** `7d45a0d2`

---

**Total deviations:** 2 auto-fixed (1 formatting/lint, 1 missing-critical-process-step per
CLAUDE.md). **Impact on plan:** Both are process-gate fixes required by the project's own
completion protocol, not scope creep; no design or behavior changed as a result — the fmt fix is
whitespace-only and the API-surface/CHANGELOG fix documents an already-additive, already-shipped
surface change.

## Issues Encountered

- `cargo-deny` is not installed in this execution environment (`error: no such command: 'deny'`),
  so `make security`/`make deny` could not be run directly. This is an environment limitation, not
  a plan regression: `git diff --stat` confirms `Cargo.lock` and `Cargo.toml` are byte-identical
  across all four of this plan's commits (zero new dependencies), so the dependency graph
  `cargo-deny`/`cargo-audit` would scan is unchanged from the last known-good state.
- No integration test spawns the actual `paladin-server` binary against a malformed config and
  asserts its process-level refusal-to-start behavior (coverage `D7`, `human_judgment: true`) —
  the ordering and message format are verified by source inspection plus the unit-level
  `HostBuildError`/`ProvisionError` tests that share the identical `TreasurerConfig::validate()`
  call `paladin-server` itself invokes.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- The operator-facing `treasurer:` config section and both production pricing wiring points
  (config-load/runtime-provisioned agents, run engine) are complete, tested, and boot-validated.
  ROADMAP Phase 38 success criterion 1 (an operator can configure a per-model price table;
  default empty; malformed/negative rejected at config validation) is fully satisfied by this
  plan.
- 38-04 (non-streaming `generate()` pricing, `LlmResponse.cost`) can build directly on
  `TreasurerConfig::price_table()` and the now-wired `with_pricing` call sites — no further
  config or wiring work needed there.
- 38-06/38-07 (`TraceDispatcher::total_cost`, agent-loop run-level total) can consume
  `settings.get_treasurer_config()` the same way `build_agent_registry` and
  `paladin_port_from_settings` do.
- No blockers. The legacy CLI commands (`paladin-cli agent`, `battalion`) remain explicitly
  out of scope for pricing in this phase (flagged in the plan's objective; Phase 39 LEDGR-04).

---
*Phase: 38-design-seams-pricing-cost-producer*
*Completed: 2026-09-26*

## Self-Check: PASSED

All created files and commit hashes verified present on disk / in `git log --oneline --all`:
- `src/config/treasurer.rs` — FOUND
- `.planning/phases/38-design-seams-pricing-cost-producer/38-03-SUMMARY.md` — FOUND
- `79e4b266` (Task 1 RED) — FOUND
- `8b4456cd` (Task 1 GREEN) — FOUND
- `0289d487` (Task 2) — FOUND
- `7d45a0d2` (post-task gate: fmt/API-surface/CHANGELOG) — FOUND

Re-ran plan-level `<verification>` commands: `cargo test -p paladin-ai --lib config::treasurer`
(12/12 pass), `cargo test -p paladin-ai --lib --features web-server infrastructure::web` (22/22
pass), `cargo build -p paladin-ai --features web-server --bin paladin-server` (succeeds), `cargo
clippy -p paladin-ai --all-targets --features web-server -- -D warnings` (clean). All task-level
`<acceptance_criteria>` greps re-verified against the final (post-fmt) source.
