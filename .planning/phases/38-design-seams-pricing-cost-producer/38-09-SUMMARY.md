---
phase: 38-design-seams-pricing-cost-producer
plan: 09
subsystem: release-gates
tags: [rust, semver, migration, changelog, api-surface, rustdoc, release-record]

# Dependency graph
requires:
  - phase: 38-design-seams-pricing-cost-producer (plans 01-08)
    provides: "Every public-API change this phase made (ADR-0052/0053, Cost/CurrencyCode/PriceRow/PriceTable/CostTally, TreasurerConfig, PricingLlmAdapter, LlmResponse.cost, PaladinResult.cost, TraceEvent.cost, TraceDispatcher::total_cost, JSON/table herald currency, ExecutionMetadata::from_run_finished, HeraldTraceSink) that this plan measures and registers"
provides:
  - "Empirical cargo-semver-checks 0.50.0 measurement of every Phase 38 public-API change against both the CI-pinned v0.9.0 baseline (all 11 CI packages) and the published v0.10.1 baseline (--release-type minor, the 7 changed packages)"
  - "MIGRATION.md §9.2 extended: PaladinResult (cost), Settings (treasurer) Extended-note additions; a new TraceEvent::NodeFinished/RunFinished N/A extension for the v0.10 -> v0.11 migration guide (Phase 46, CURR-23)"
  - "CHANGELOG.md [Unreleased] Added/Changed entries covering the whole phase; five per-crate CHANGELOG.md [Unreleased] bullets"
  - "Refreshed .project/current-exports.txt (4025 items) via the CI-pinned nightly-2026-09-20 toolchain; make api-surface exits 0"
  - "Zero rustdoc warnings workspace-wide (three pre-existing/Phase-38-introduced broken intra-doc links fixed): cost.rs, herald.rs, worker.rs"
  - "A clean, documented run of every project release gate on the phase's final tree: cargo test --workspace, cargo fmt --check, make clean-code, make security, make check-gates, make openapi (no drift)"
affects: ["39-treasurer-ledger", "46-mdbook-treasurer-docs (CURR-23)"]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Empirical semver-checks derivation (D-27, Phase 31 precedent): every MIGRATION.md registration in this plan is copy-pasted from an actual cargo semver-checks check-release run's failure block, never predicted -- including a diagnostic run with a crate-wide Cargo.toml suppression temporarily disabled and immediately reverted, to distinguish 'already covered by an existing blanket allow' from 'genuinely unsuppressed'"

key-files:
  created: []
  modified:
    - MIGRATION.md
    - CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-herald/CHANGELOG.md
    - .project/current-exports.txt
    - crates/paladin-core/src/platform/container/cost.rs
    - crates/paladin-core/src/platform/container/herald.rs
    - src/application/services/run/worker.rs

key-decisions:
  - "No .cargo/semver-checks-allowlist.toml or Cargo.toml lint-table entry was added in this plan. PaladinResult.cost and Settings.treasurer both fire constructible_struct_adds_field, empirically confirmed by temporarily disabling each crate's existing blanket allow and re-running the 0.10.1/--release-type minor check -- both pairs already have an allowlist entry from an earlier phase, and the set-equality gate dedups by crate|type, not by lint, so nothing new was needed."
  - "TraceEvent::NodeFinished/RunFinished.cost fires enum_struct_variant_field_added against the published v0.10.1 baseline -- a genuinely unsuppressed lint (crates/paladin-core/Cargo.toml's lints table has no entry for it) -- but is invisible to the CI-pinned v0.9.0 comparison because TraceEvent postdates that baseline entirely. Registered as an N/A row extension for the v0.10 -> v0.11 migration guide, per the plan's own explicit instruction; no allowlist entry, since X-10/the CI gate only track breaks against the v0.9.0 baseline."
  - "The CI-parity check (--baseline-version 0.9.0, no --release-type override) reports '0 checks: 0 pass, 254 skip. Summary no semver update required' for ALL 11 packages, unconditionally -- not specific to Phase 38. Confirmed empirically: cargo-semver-checks treats 0.9.0 -> 0.10.1 (the in-tree, unreleased-for-v0.11 version) as already a pre-1.0 'major-equivalent' bump, and skips lint evaluation entirely once the declared version already permits any change. This is a real, measured, out-of-scope finding (ci.yml is not in this plan's files_modified) -- documented below and in STATE.md, not fixed here."
  - "grep -q 'cost::Cost' .project/current-exports.txt fails, by design of the extraction tool, not by omission: cargo-public-api only enumerates the FACADE crate's (paladin-ai/paladin) own signatures, never traversing into a `pub use`-re-exported module living in another crate (paladin-core) to list its descendant items individually -- confirmed by checking the pre-existing `container::garrison`/`TokenUsage` precedent, neither of which has a standalone struct line either. Cost reaches the surface only indirectly, through PriceTable (TreasurerConfig::price_table's return type) and the module-level `pub use ...container::cost` re-export line -- both present. Adding a facade-level function returning bare Cost purely to satisfy this grep would be an unrequested, unjustified public-API surface addition (Rule 4 territory) with no CONTEXT.md or plan basis; not done. TreasurerConfig and HeraldTraceSink, the acceptance criterion's other two greps, both pass."
  - "Installed shellcheck (apt), cargo-audit 0.22.2, cargo-deny 0.20.2 and cargo-semver-checks 0.50.0 (all absent in this execution environment) to run the full gate suite and the D-27 empirical method as the plan requires."

requirements-completed: [PRICE-01, PRICE-02, PRICE-03]

coverage:
  - id: D1
    description: "Every one of the 11 CI packages passes cargo semver-checks against the CI-pinned v0.9.0 baseline (--default-features); the loop and the migration-allowlist/check-gates set-equality gate both exit 0 on the phase's final tree"
    requirement: PRICE-03
    verification:
      - kind: other
        ref: "cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0 (all 11 CI packages, each exit 0)"
        status: pass
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh"
        status: pass
      - kind: other
        ref: "make check-gates"
        status: pass
    human_judgment: false
  - id: D2
    description: "The published v0.10.1 baseline (--release-type minor) measurement for every package Phase 38 changed captures the one genuine, unsuppressed break (TraceEvent's enum_struct_variant_field_added) and confirms the two already-suppressed ones (PaladinResult, Settings), all registered in MIGRATION.md §9.2"
    requirement: PRICE-03
    verification:
      - kind: other
        ref: "cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.10.1 --release-type minor (paladin-ai, paladin-ai-core, paladin-ports, paladin-llm, paladin-battalion, paladin-herald, paladin-web)"
        status: pass
    human_judgment: false
  - id: D3
    description: "MIGRATION.md §9.2 rows for PaladinResult, Settings and TraceEvent::NodeFinished/RunFinished carry a dated Phase 38 extension; the allowlist stays set-equal (16 crate|type pairs, unchanged) since no new deliberate-breaking entry was needed"
    requirement: PRICE-03
    verification:
      - kind: other
        ref: "./scripts/check-migration-allowlist.sh (16 pairs, set-equal)"
        status: pass
    human_judgment: false
  - id: D4
    description: "CHANGELOG.md gains one [Unreleased] section (Added + new Changed) naming every Phase 38 public-facing change and pointing at MIGRATION.md §9.2; each of the five changed crates' own CHANGELOG.md [Unreleased] names its own cost/pricing addition"
    requirement: PRICE-01
    verification:
      - kind: other
        ref: "grep -c '^## \\[Unreleased\\]' CHANGELOG.md == 1; per-crate greps for cost|pricing under each Unreleased section, all non-zero"
        status: pass
    human_judgment: false
  - id: D5
    description: "The public-surface baseline is refreshed with the CI-pinned nightly-2026-09-20 toolchain; TreasurerConfig and HeraldTraceSink are present; make api-surface exits 0 against the refreshed baseline"
    requirement: PRICE-01
    verification:
      - kind: other
        ref: "PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface-update && make api-surface"
        status: pass
    human_judgment: false
  - id: D6
    description: "The phase's final tree passes every commit gate CLAUDE.md names: cargo test --workspace (0 failures across every test result line), cargo fmt --check, make clean-code (fmt/clippy -D warnings/shell lint/check/rustdoc zero-warning bar/public-API examples gate), make security (audit + deny, 0 new advisories), make check-gates; make openapi produces no diff on crates/paladin-web/openapi.json"
    requirement: PRICE-03
    verification:
      - kind: other
        ref: "cargo test --workspace"
        status: pass
      - kind: other
        ref: "cargo fmt --check"
        status: pass
      - kind: other
        ref: "make clean-code"
        status: pass
      - kind: other
        ref: "make security"
        status: pass
      - kind: other
        ref: "make check-gates"
        status: pass
      - kind: other
        ref: "make openapi && git diff --exit-code crates/paladin-web/openapi.json"
        status: pass
    human_judgment: false
  - id: D7
    description: "Manual credential-handling review over the full Phase 38 diff: no API key is logged or Debug-formatted, the pricing warn-once log interpolates only a bare model name, and no new HTTP client was added anywhere in the phase"
    requirement: PRICE-03
    verification: []
    human_judgment: true
    rationale: "This is a manual source-inspection review per security.instructions.md, not something a unit test asserts -- grep-scanned the whole phase diff (git diff 8d76aa2a~1..HEAD -- crates src) for credential-shaped identifiers and reqwest client construction, then read pricing.rs's two log::warn! call sites directly to confirm only `model` (a bare model-name string) is interpolated. Findings are stated in prose below; a human reviewer re-reading the same diff would reach the same two conclusions (clean; two log call sites, both benign)."

duration: ~110min
completed: 2026-09-26
status: complete
---

# Phase 38 Plan 09: Design Seams — Phase Closeout on the Project's Release Gates Summary

**Every Phase 38 public-API change measured empirically with cargo-semver-checks against both the CI-pinned v0.9.0 baseline and the published v0.10.1 baseline, registered in MIGRATION.md §9.2, described in the CHANGELOG, and the phase's final tree run through every commit and security gate the project requires before a phase is sealed — with two measured, documented discrepancies between the plan's predicted tool output and what the tools actually report.**

## Performance

- **Duration:** ~110 min
- **Started:** 2026-09-26T15:04:00Z (approx, per STATE.md's prior session timestamp)
- **Completed:** 2026-09-26T16:55:49Z
- **Tasks:** 2 (Task 1: semver measurement + MIGRATION.md registration; Task 2: CHANGELOG/API-baseline/full gate suite)
- **Files modified:** 10 (1 in Task 1's commit, 10 across both — three of Task 2's files were rustdoc fixes discovered while running the gate suite, not originally in either task's declared file list)

## Accomplishments

- **Task 1 (`c8815b90`):** Installed `cargo-semver-checks` 0.50.0 (`cargo install --locked`). Ran the exact CI-parity loop (`--default-features --baseline-version 0.9.0`) across all 11 published CI packages: every one reports `0 checks: 0 pass, 254 skip — Summary no semver update required` (see Key Decisions/Deviations for why this is a measured, unconditional finding, not a Phase-38-specific pass). `paladin-web` required a one-time local workaround (below) because its `utoipa-swagger-ui` build script downloads a zip from `github.com`, which this sandbox's egress proxy blocks with a 403. Then ran the published `v0.10.1` baseline with `--release-type minor` for the 7 packages Phase 38 changed: `paladin-ai-core` reports one genuine failure — `enum_struct_variant_field_added` on `TraceEvent::NodeFinished.cost` and `TraceEvent::RunFinished.cost` — every other package reports "no semver update required." A follow-up diagnostic run, with `crates/paladin-core/Cargo.toml`'s and the root `Cargo.toml`'s `constructible_struct_adds_field = "allow"` line temporarily disabled and immediately reverted, empirically confirmed the two additive-field changes the plan expected (`PaladinResult.cost`, `Settings.treasurer`) both fire that lint and are already covered by the existing crate-wide suppression. Extended the `PaladinResult` and `Settings` §9.2 rows with dated Phase 38 notes, and extended the `TraceEvent::NodeFinished`/`RunFinished` row with an `N/A` note for the `v0.10` → `v0.11` migration guide (Phase 46, CURR-23) — no allowlist entry, since the CI `semver` job's `v0.9.0` baseline cannot observe a break in a type that postdates it. `./scripts/check-migration-allowlist.sh` and `make check-gates` both confirm the allowlist stays set-equal (16 `crate | type` pairs, unchanged).
- **Task 2 (`79d7f4e3`):** Added a `## [Unreleased]` `### Added` block to `CHANGELOG.md` covering every Phase 38 plan not yet described (ADR-0052/0053, non-streaming `LlmResponse.cost`, JSON/table herald currency, the trace-stream/`PaladinResult` cost carriers, `ExecutionMetadata::from_run_finished` + `HeraldTraceSink`), plus a new `### Changed` section (currency-formatted cost, the table herald's real metadata, the always-present streamed `ChunkMetadata.execution`, and the five structs that gained a `cost` field). Added matching `[Unreleased]` bullets to the five changed crates' own `CHANGELOG.md` files. Regenerated `.project/current-exports.txt` with `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20` (4025 items, purely additive); `make api-surface` exits 0. Ran the full gate suite on the final tree: `cargo fmt --check`, `cargo test --workspace` (every `test result:` line reports `0 failed`), `make clean-code`, `make security`, `make check-gates`, and `make openapi` (`git diff --exit-code crates/paladin-web/openapi.json` — no HTTP schema gained a cost field). `make clean-code`'s `doc-check` step surfaced three rustdoc intra-doc-link warnings (two more than the one already logged in `deferred-items.md`); fixed all three (see Deviations) so the ADR-0033 zero-warning bar holds. Performed the manual credential-handling review over the full phase diff.

## Task Commits

1. **Task 1: Measure every Phase 38 API change with cargo-semver-checks and register what it reports** - `c8815b90` (docs)
2. **Task 2: CHANGELOG entries, refreshed API baseline, and the full gate suite on the final tree** - `79d7f4e3` (docs)

**Plan metadata:** (this commit)

## Files Created/Modified

- `MIGRATION.md` — §9.2 extensions for `PaladinResult`, `Settings`, `TraceEvent::NodeFinished`/`RunFinished`
- `CHANGELOG.md` — `[Unreleased]` `### Added` block completed for the whole phase; new `### Changed` section
- `crates/{paladin-core,paladin-ports,paladin-llm,paladin-battalion,paladin-herald}/CHANGELOG.md` — `[Unreleased]` bullets for each crate's own Phase 38 surface
- `.project/current-exports.txt` — refreshed public API baseline (4025 items, CI-pinned `nightly-2026-09-20`)
- `crates/paladin-core/src/platform/container/cost.rs` — module-doc `Cost`/`CurrencyCode`/`PriceRow` links switched to fully-qualified paths (rustdoc fix)
- `crates/paladin-core/src/platform/container/herald.rs` — `from_run_finished`'s `TraceEvent::RunFinished` link switched to a fully-qualified path (rustdoc fix)
- `src/application/services/run/worker.rs` — `with_herald`'s links to two private items (`run_model_label`, `engine_factory`) switched to plain code spans (rustdoc fix)
- `.planning/phases/38-design-seams-pricing-cost-producer/deferred-items.md` — marked the 38-04-logged `cost.rs` rustdoc warning resolved by this plan

## Decisions Made

See `key-decisions` in the frontmatter for the full empirical reasoning. In short: no new `.cargo/semver-checks-allowlist.toml` or `Cargo.toml` lint-table entry was needed anywhere (every genuinely-fired lint was already covered by an existing crate-wide suppression, confirmed by a temporary-disable-and-revert diagnostic); the one truly new finding (`TraceEvent`'s struct-variant field addition) is recorded as migration-guide guidance, not a CI-gated row, because it postdates the CI's pinned `v0.9.0` baseline entirely.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `paladin-web`'s cargo-semver-checks build failed: `utoipa-swagger-ui`'s build script cannot download from `github.com`**
- **Found during:** Task 1, the `paladin-web` leg of the CI-parity `--baseline-version 0.9.0` loop
- **Issue:** `cargo semver-checks` builds each crate in an isolated target directory that has never run `utoipa-swagger-ui`'s build script before; that script tries to `curl` `https://github.com/swagger-api/swagger-ui/archive/refs/tags/v5.17.14.zip`, which this sandbox's egress proxy answers with a 403 (confirmed via `curl -sSL "https://github.com/..."` → `HTTP 403`), producing an `InvalidArchive("Could not find EOCD")` panic and failing the whole `rustdoc` generation step — an environment network-policy limitation, not a Phase 38 code defect.
- **Fix:** Re-zipped the already-extracted `swagger-ui-5.17.14/dist/` directory (present in the workspace's own `target/debug/build/utoipa-swagger-ui-*/out/` from an earlier, successful in-tree build) into a local `.zip` under the scratchpad directory, and set `SWAGGER_UI_DOWNLOAD_URL=file://<that zip>` for every `cargo semver-checks` invocation touching `paladin-web`. The build script's own documented `file://` protocol support (`build.rs`'s `SWAGGER_UI_DOWNLOAD_URL` handling) picks this up with no other change.
- **Files modified:** none in the repository — this is a local, ephemeral `/tmp` scratchpad artifact and a shell environment variable, never committed.
- **Verification:** `cargo semver-checks check-release --package paladin-web --default-features --baseline-version 0.9.0` and the `--baseline-version 0.10.1 --release-type minor` run both succeed with this override in place.
- **Committed in:** N/A (no repository file changed)

**2. [Rule 1 - Bug] Three rustdoc intra-doc-link warnings blocking `make doc-check`/`make clean-code`'s ADR-0033 zero-warning bar**
- **Found during:** Task 2, running `make clean-code` on the final tree
- **Issue:** `cargo doc --workspace --no-deps` reported 6 warnings: the 3 already logged in `deferred-items.md` from plan 38-02 (`cost.rs`'s module doc linking bare `[`Cost`]`/`[`CurrencyCode`]`/`[`PriceRow`]`, unresolved because a module-level `//!` doc comment's bare-name link resolution does not reach sibling items defined later in the same file the way an item's own `///` doc comment's does), plus two new ones this phase's own plans introduced and never re-ran `make doc-check` to catch: `herald.rs`'s `from_run_finished` doc linking bare `[`TraceEvent::RunFinished`]` with no `use` in scope (38-08), and `worker.rs`'s `with_herald` doc linking to two PRIVATE items, `[`run_model_label`]` and `[`Self::engine_factory`]` (38-08) — links inside a PUBLIC method's doc comment cannot resolve to a private item under default `cargo doc` (no `--document-private-items`).
- **Fix:** `cost.rs` and `herald.rs`: switched the bare-name links to the already-proven-working fully-qualified-path form (`` [`crate::platform::container::cost::Cost`] ``, `` [`TraceEvent::RunFinished`](crate::platform::container::trace::TraceEvent::RunFinished) ``), matching a sibling link in the same file (`herald.rs` line 585) that already used this form successfully. `worker.rs`: switched the two private-item links to plain code spans (`` `run_model_label` ``, `` the per-run `engine_factory` ``), matching plan 38-04's own established precedent for exactly this situation (`price_or_warn`).
- **Files modified:** `crates/paladin-core/src/platform/container/cost.rs`, `crates/paladin-core/src/platform/container/herald.rs`, `src/application/services/run/worker.rs`
- **Verification:** `cargo doc --workspace --no-deps 2>&1 | grep -i warning` produces no output; `cargo clippy -p paladin-ai-core -p paladin-ai --lib --features web-server -- -D warnings` clean; the full `make clean-code` run (fmt, lint, lint-shell, check, doc-check, check-api-examples) exits 0.
- **Committed in:** `79d7f4e3` (Task 2 commit)

**3. [Rule 3 - Blocking] `shellcheck`, `cargo-audit`, `cargo-deny` and `cargo-semver-checks` absent from the execution environment**
- **Found during:** Task 1 (semver-checks) and Task 2 (`make clean-code`'s `lint-shell` step, `make security`)
- **Issue:** None of the four tools this plan's own `<action>` text calls for were pre-installed; `make clean-code` failed outright at `lint-shell` (`shellcheck not found`) before ever reaching `check`/`doc-check`/`check-api-examples`.
- **Fix:** `cargo install --locked cargo-semver-checks --version 0.50.0` (the CI-pinned version), `apt-get install -y shellcheck`, `cargo install --locked cargo-audit`, `cargo install --locked cargo-deny` — exactly as the plan's own text instructs ("Install the CI-pinned tool if absent" / "install with `cargo install --locked` if absent").
- **Files modified:** none (tool installs only, no `Cargo.lock`/`Cargo.toml` change — confirmed via `git diff --stat Cargo.lock` producing no output after every gate ran)
- **Verification:** `make clean-code` and `make security` both exit 0 on the re-run.
- **Committed in:** N/A (no repository file changed)

---

**Total deviations:** 3 (1 environment network workaround, 1 rustdoc bug fix, 1 environment tooling install). **Impact on plan:** The rustdoc fix (#2) is the only one touching tracked files, and it is a direct, minimal, zero-behavior-change fix required for this plan's own explicitly-stated gate (`make clean-code`/ADR-0033) to pass — no scope creep. The other two are ephemeral environment-preparation steps the plan's own text anticipated ("install ... if absent") and leave no trace in the repository.

## Two measured discrepancies between the plan's `must_haves`/`acceptance_criteria` and the tools' actual output

These are not deviations from the plan's *instructions* (which were followed exactly, empirically, per D-27) — they are places where the plan's own *predicted* tool output does not match what the pinned tool versions actually produce on this tree today. Recorded here in full per the project's "derived from the tool's actual output, never guessed" standard, rather than silently forcing a match:

1. **The CI-pinned `--baseline-version 0.9.0` semver check is currently a no-op for every package, unconditionally — not specific to Phase 38.** `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` (no `--release-type` override) reports `0 checks: 0 pass, 254 skip` for all 11 CI packages. This is `cargo-semver-checks`' own documented behavior for pre-1.0 crates: it treats a `0.9.x → 0.10.y` version difference as already a "major-equivalent" bump (the tool's SemVer-for-`0.x` convention), and once the *declared* version already permits any possible breaking change, it skips lint evaluation entirely rather than computing which specific lints would fire. Because the workspace's in-tree version has been `0.10.1` (the last real release) since before Phase 38 began, and the CI job's baseline is hardcoded to the two-milestones-old `0.9.0`, this specific check has been vacuous for every plan in this phase, and will remain so for any plan in this milestone, until the version is bumped for the eventual `v0.11.0` release. This is a real, useful, measured finding about the *current state of the CI gate*, not a Phase 38 defect — `ci.yml` is not in this plan's `files_modified`, so it is documented here and flagged to `STATE.md`'s Blockers/Concerns rather than "fixed." The plan's own instruction to run the check "exactly as CI's semver job runs it" was followed to the letter; this is what that produces today.
2. **`grep -q 'cost::Cost' .project/current-exports.txt` fails — by the extraction tool's own design, not by omission.** `cargo-public-api` only enumerates the *facade* crate's (`paladin`/`paladin-ai`) own item signatures; it never traverses into a `pub use`-re-exported module living in a *different* crate (`paladin-core`) to list that module's descendant items individually. This is confirmed as a pre-existing, systemic pattern, not something Phase 38 introduced: the same tool run shows no standalone `pub struct`/`pub enum` line for `TokenUsage`, `PaladinResult`'s own definition, or any other `paladin_core::platform::container::*` type either — each reaches the baseline only indirectly, through whatever facade-crate function happens to take or return it (e.g. `TreasurerConfig::price_table() -> Result<PriceTable, String>`, which is why `PriceTable` *does* appear). `Cost` has no such facade-crate-local function returning it directly, so the literal substring `cost::Cost` does not appear — but the module *is* listed (`pub use paladin::core::platform::container::cost`), and the other two required substrings, `TreasurerConfig` (62 occurrences) and `HeraldTraceSink` (28 occurrences), both pass. Adding a facade-level function that returns bare `Cost`, purely to satisfy this one substring grep, would be an unrequested, unjustified new public-API surface addition with no basis in `CONTEXT.md`'s decisions or this plan's own `<action>` text — not done, per the deviation-rule boundary against inventing scope not asked for.

## Issues Encountered

- `cargo-audit`'s yanked-crate lookup (a network call per dependency, separate from its offline advisory-DB scan) reported ~726 `503 Service Unavailable` errors against the sparse crates.io index through this sandbox's egress proxy. These do not affect the actual vulnerability scan (which reads the local, git-cloned RustSec advisory database) — `make security` still completed and reported `advisories ok, bans ok, licenses ok, sources ok` with `9 allowed warnings found` (all pre-existing `unsound`/`yanked` transitive-dependency warnings already covered by `.cargo/audit.toml`'s 5-entry ignore list and `deny.toml`'s mirrored exceptions; none is new to Phase 38, which added zero dependencies — confirmed via `git diff --stat Cargo.lock` producing no output across every gate run in this plan).
- Disk space required active management: installing `cargo-semver-checks` plus its isolated per-package build directories (`target/semver-checks/`, up to ~5.6 GB) repeatedly pushed free space toward the ~5 GB floor `CLAUDE.md`'s project notes warn about. Removed `target/semver-checks/` (never `target/debug/incremental`, per the standing instruction) after each round of semver-checks runs, restoring 9-11 GB free before the next heavy build (`cargo test --workspace`, `make clean-code`).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- **PRICE-01, PRICE-02 and PRICE-03 are delivered on a tree that passes every project gate**, per this plan's own success criterion: `cargo test --workspace`, `cargo fmt --check`, `make clean-code`, `make security`, `make check-gates`, `./scripts/check-migration-allowlist.sh`, and `make openapi` (no drift) all exit 0 on the final commit.
- **Coverage attribution:** the 82% workspace line-coverage floor (ADR-0006) is read from CI's `coverage` job, not measured locally — this devcontainer has no Docker (the same standing concern `STATE.md` has carried since the v0.10.0 close; the pending user-owned todo `2026-08-13-verify-local-coverage-reproduction` is unchanged by this plan).
- **Two measured, documented findings carried forward, not fixed here** (see the dedicated section above): the CI `semver` job's `v0.9.0` baseline is currently a no-op for any package until the version is bumped for `v0.11.0`; `cost::Cost` cannot appear in `.project/current-exports.txt` as a literal substring given how `cargo-public-api` scopes its extraction. Neither blocks this phase's own success criterion, and neither is a Phase 38 code defect — both are flagged to `STATE.md`'s Blockers/Concerns for whichever future phase next touches `ci.yml`'s `semver` job or performs the `v0.11.0` version bump.
- Phase 38 (design-seams-pricing-cost-producer) is now fully closed: ADR-0052/0053 recorded (38-01), the pure cost engine and streamed producer proven end-to-end (38-02), the operator price table and production wiring boot-validated (38-03), non-streaming pricing across fallback hops (38-04), JSON/table herald currency and real metadata (38-05), the trace-stream cost carriers and `TraceDispatcher::total_cost` (38-06), `PaladinResult.cost` and the engine bridge (38-07), the engine-path `ExecutionMetadata` producer and `HeraldTraceSink` (38-08), and this plan's release-record closeout (38-09).
- No blockers to starting Phase 39 (Treasurer ledger).

---
*Phase: 38-design-seams-pricing-cost-producer*
*Completed: 2026-09-26*

## Self-Check: PASSED

All modified files and commit hashes verified present on disk / in `git log --oneline --all`:
- `MIGRATION.md` — FOUND (contains `Extended, Phase 38 (PRICE-03, D-10)` for `PaladinResult`, `Extended, Phase 38 (PRICE-01, D-07)` for `Settings`, and the `TraceEvent::NodeFinished`/`RunFinished` `enum_struct_variant_field_added` extension)
- `CHANGELOG.md` — FOUND (exactly one `## [Unreleased]` line, containing `PricingLlmAdapter`, `treasurer`, `Cost`, `HeraldTraceSink`, `currency`, `MIGRATION.md`)
- `.project/current-exports.txt` — FOUND (4025 items; contains `TreasurerConfig`, `HeraldTraceSink`; does not contain the literal substring `cost::Cost`, documented above)
- `crates/paladin-core/src/platform/container/cost.rs`, `herald.rs`, `src/application/services/run/worker.rs` — FOUND (zero rustdoc warnings on re-run)
- `c8815b90` (Task 1) — FOUND
- `79d7f4e3` (Task 2) — FOUND

Re-ran plan-level `<verification>` on the final tree: the 11-package `cargo semver-checks` loop against `v0.9.0` exits 0 for every package; `./scripts/check-migration-allowlist.sh`, `make check-gates`, `make api-surface`, `make clean-code`, `make security`, `cargo test --workspace`, `cargo fmt --check` all exit 0; `make openapi` leaves `crates/paladin-web/openapi.json` unchanged. All task-level `<acceptance_criteria>` re-verified against the final source, with the two documented exceptions above.
