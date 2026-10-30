---
phase: 46-docs-currency-hygiene
reviewed: 2026-10-24T00:00:00Z
depth: standard
files_reviewed: 41
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - .github/workflows/ci.yml
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-battalion/Cargo.toml
  - crates/paladin-battalion/src/commander.rs
  - crates/paladin-battalion/src/conclave_execution_service.rs
  - crates/paladin-battalion/src/phalanx_service.rs
  - crates/paladin-content/CHANGELOG.md
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-core/Cargo.toml
  - crates/paladin-core/src/platform/container/battalion/conclave.rs
  - crates/paladin-core/src/platform/container/battalion/legacy_keys.rs
  - crates/paladin-core/src/platform/container/battalion/mod.rs
  - crates/paladin-eval/CHANGELOG.md
  - crates/paladin-herald/CHANGELOG.md
  - crates/paladin-llm/CHANGELOG.md
  - crates/paladin-llm/Cargo.toml
  - crates/paladin-llm/src/registry/seeded.rs
  - crates/paladin-memory/CHANGELOG.md
  - crates/paladin-notifications/CHANGELOG.md
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/Cargo.toml
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-web/CHANGELOG.md
  - docs/src/SUMMARY.md
  - docs/src/api-reference/migration-guide.md
  - docs/src/api-reference/upgrading.md
  - docs/src/appendix/release-automation.md
  - docs/src/appendix/release-recovery.md
  - docs/src/architecture/treasurer.md
  - docs/src/contributing/adr-index.md
  - docs/src/contributing/development-setup.md
  - docs/src/getting-started/configuration.md
  - docs/src/user-guides/switching-providers.md
  - scripts/check-dependency-allowlist.sh
  - scripts/check-migration-allowlist.sh
  - src/application/cli/config/battalion_config.rs
  - src/infrastructure/resilience/circuit_breaker.rs
  - tests/consumer_surface_muster.rs
  - tests/scripts/check-dependency-allowlist_test.sh
  - tests/scripts/check-migration-allowlist_test.sh
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
status: issues_found
---

# Phase 46: Code Review Report

**Reviewed:** 2026-10-24
**Depth:** standard
**Files Reviewed:** 41
**Status:** issues_found

## Summary

Phase 46 is mostly documentation retargeting (MIGRATION.md section 9 to section 10), plus a handful of code
changes: the load-time refusal of removed v0.10 Battalion keys (`legacy_keys.rs`), the Commander's
one-time Maneuver warning, two shell guards, and consumer-muster pins.

What I verified by running it (`CARGO_INCREMENTAL=0`, offline):

- `make check-gates` passes, including both new/changed guards. Both `tests/scripts/*_test.sh` harnesses pass
  (15 and 16 assertions).
- `cargo check --test consumer_surface_muster` compiles.
- `cargo test -p paladin-ai-core --lib -- battalion` (124 tests), the Commander
  `maneuver_ignored_aegis_policies` test, and the CLI `cli_conclave_yaml_timeout_seconds_still_loads` test
  (needs `--features cli`) all pass.
- The `legacy_keys.rs` wire-struct mirrors the public structs field for field. It carries no defaults the
  originals lacked, never echoes a value (`IgnoredAny`), and refuses a `null` value. I found no defect in it.
- The documentation claims I spot-checked hold: Cadence defaults, the 1m-366d period bound, migrations 007-014,
  the `redis-cadence` feature name, the DR-1..DR-26 and M-B-05..M-B-22 id sets, the publish order, the
  `0.10.0` and `0.10.1` release dates against the git tags, and the mdBook anchors and cross-links.
  The Treasurer worked example (22,500,000 nano-units) and the `warn_at` 79/80/81 percent example are
  arithmetically right.

No security issues were found. The new guards read files and print key names only. The refusal path is
redaction-safe.

The three warnings are correctness-of-documentation or behaviour-versus-documentation defects in text this
phase wrote or left contradicting itself.

## Warnings

### WR-01: Maneuver warning and "adds no bound of its own" docs are wrong once an explicit `ManeuverConfig` is set, and the documented M-B-11 remediation triggers exactly that

**File:** `crates/paladin-battalion/src/commander.rs:956-969` (warning), `:1744-1748` (`error_strategy`), `:98-101`, `:1514-1516`, `:1784-1786` (new doc text); `MIGRATION.md` M-B-11; `docs/src/api-reference/upgrading.md:37`

**Issue:**

The new `warn!` fires whenever `aegis.on_error` or `aegis.retry` is set, before the code decides whether
`self.maneuver_config` is present. When an explicit `ManeuverConfig` exists, `unwrap_or_else(default_maneuver_config)`
is never called. In that case the Maneuver reads none of `aegis`, so the message text ("Maneuver reads only
aegis.timeout.run_timeout, as one whole-flow clock") is false for that run.

The bigger problem is that `CommanderBuilder::error_strategy()` (line 1744) does
`self.maneuver_config.unwrap_or_default()` and stores `Some(..)`. `ManeuverConfig::default()` carries
`timeout: Some(300s)` (`maneuver/mod.rs:66`). M-B-11, and the Upgrading page row that condenses it, tell
operators to call `CommanderBuilder::error_strategy(ContinueParallel)` to restore v0.10 behaviour.

Following that advice therefore silently discards a configured `aegis.timeout.run_timeout`. A caller who set 45 s
gets 300 s. A caller who set nothing, where the derived config would have been unbounded (`None`), now gets 300 s.
The new rustdoc ("The Commander adds no bound of its own", "a Maneuver uses `run_timeout` as one whole-flow
clock") is accurate only on the derived-config path. The `BattalionConfig.aegis` rustdoc does mention the
replacement, but neither M-B-11 nor the Commander docs warn that `error_strategy()` triggers it.

**Fix:**

Gate the warning on the derived path, and make the explicit-config case honest in the docs and the remediation.

```rust
let maneuver_config = match self.maneuver_config.clone() {
    Some(explicit) => explicit, // aegis is not read at all on this path
    None => {
        let ignored = maneuver_ignored_aegis_policies(&self.config.aegis);
        if !ignored.is_empty() {
            warn!(/* existing message */);
        }
        self.default_maneuver_config()
    }
};
```

Add one sentence to M-B-11 and the Upgrading row: `error_strategy(..)` and `maneuver_config(..)` replace the
aegis-derived Maneuver config, including its timeout, so set `ManeuverConfig.timeout` explicitly alongside it.
Alternatively, have `CommanderBuilder::error_strategy` start from the aegis-derived config rather than
`ManeuverConfig::default()`.

### WR-02: Two release-note sentences still say a v0.10 document with removed keys "still deserializes ... ignored", contradicting this phase's D-23 refusal

**File:** `CHANGELOG.md:1438-1439` (Unreleased, Phase 44 "Removed" bullet) and `crates/paladin-core/CHANGELOG.md:133-134` (Phase 44 "Changed" bullet)

**Issue:**

Phase 46 plan 46-08 makes `BattalionConfig` and `ConclaveConfig` refuse a document carrying `timeout_seconds`,
`retry_policy` or `error_strategy`. Both the root and the `paladin-core` changelogs record the new behaviour in
a fresh Phase 46 entry, but leave the older Phase 44 bullets unchanged:

- Root: "a v0.10 document still deserializes, the removed keys are ignored and `aegis` is `Aegis::default()`".
- Core: "A v0.10 `BattalionConfig` document still deserializes; its removed keys are ignored and `aegis` is
  `Aegis::default()`."

Both are in the unreleased v0.11.0 section, so the shipped release notes will state both behaviours. A reader
who stops at the Phase 44 bullet concludes the upgrade is safe, which is the silent-unbounded-run hazard D-23
exists to remove. The MIGRATION.md rows for M-B-06 and `BattalionConfig` were updated; these two were missed.

**Fix:**

Edit both bullets in place, for example: "a v0.10 document carrying a removed key is refused at load, naming
the key and its `aegis.*` replacement (Phase 46, D-23; MIGRATION.md 10.1 M-B-06); a document without them loads
with `aegis` as `Aegis::default()`."

### WR-03: `execute_with_cancellation` rustdoc overstates what survives cancellation for `FirstSuccess`

**File:** `crates/paladin-battalion/src/phalanx_service.rs:179-183`

**Issue:**

The new paragraph says: "Only waiting and backing-off Paladins stop. An attempt already in flight when the
token fires runs to its end and keeps its concurrency permit until then." That holds for `CollectAll` and
`Majority`, where `execute_collect_all` `tokio::spawn`s each Paladin and the tasks are detached when the
`select!` drops the parent future.

It does not hold for `FirstSuccess`. `execute_first_success` (lines 397-445) builds un-spawned `BoxFuture`s and
drives them with `select_ok` inside `execute_internal`. When the biased `cancelled()` arm wins, `select!` drops
that future and every in-flight attempt is dropped at its next await point. Nothing runs to its end. The same
drop happens to the losing futures once `select_ok` returns.

The second half of the paragraph (no semaphore for `FirstSuccess`) is correct, which makes the first half's
unconditional wording easy to trust.

**Fix:**

Scope the statement to the aggregation strategies that spawn tasks, for example: "Under `CollectAll` and
`Majority`, each Paladin runs in a spawned task, so an attempt already in flight when the token fires runs to
its end and keeps its permit. Under `FirstSuccess` the attempts are dropped with the call."

## Info

### IN-01: `check-dependency-allowlist.sh` pins keys, not packages, so a renamed surplus crate passes

**File:** `scripts/check-dependency-allowlist.sh:70-97`

**Issue:**

The comparison is on `[dependencies]` keys only, and the header documents this as deliberate (so the
`paladin_core` rename compares as written). The consequence is that
`tokio = { package = "reqwest", version = "0.12" }` satisfies the guard, as does a `package = ".."` change on the
`paladin_core` key. The guard's stated purpose is to stop provider SDKs and transport clients entering the two
innermost crates. It is trivially bypassable by an alias, accidentally or otherwise.

**Fix:**

For each dependency whose value is a table with a `package` key, compare `package` (or `key` when absent)
against an expected `(key, package)` pair. Pin `paladin_core -> paladin-ai-core` explicitly and add a harness
case that plants a `package =` alias.

### IN-02: `check-dependency-allowlist.sh` checks that `python3` exists but not that it has `tomllib`

**File:** `scripts/check-dependency-allowlist.sh:58-61`

**Issue:**

`tomllib` is Python 3.11 or later. On an older interpreter (a developer machine or an older runner image), the
guard dies with an `ImportError` traceback instead of the script's own clear error. The failure is still
non-zero, so there is no silent pass.

**Fix:**

```bash
python3 -c 'import tomllib' 2>/dev/null || { echo "ERROR: python3 >= 3.11 (tomllib) is required." >&2; exit 1; }
```

### IN-03: `circuit_breaker.rs` module-level state diagram contradicts the type's new doc

**File:** `src/infrastructure/resilience/circuit_breaker.rs:17-19` (versus the new doc at `:100-106`)

**Issue:**

The new IN-05 paragraph correctly says a `Permanent` or `Unknown` failure in `HalfOpen` neither re-opens nor
closes the breaker. The module header's diagram, a few lines above in the same file, still states
`HalfOpen --[failure occurs]--> Open` and `Closed --[failure_threshold exceeded]--> Open` without qualifying
"failure" as `Transient`. A reader of the module docs gets the old rule.

**Fix:**

Change the diagram labels to "Transient failure occurs" and add a one-line note that other failures leave the
state unchanged.

### IN-04: Stale and redundant crate counts around the twelve-crate update

**File:** `docs/src/appendix/release-automation.md:76-77`; `docs/src/appendix/release-checklist.md:25`; `scripts/finalize-crate-changelogs.sh:8`

**Issue:**

- `release-automation.md` line 76-77 now reads "The twelve-crate order ... the twelve publishable crates",
  which is redundant wording from a partial edit.
- `release-checklist.md:25` still says "across all ten crates", and
  `finalize-crate-changelogs.sh:8` still says "all ten crate" changelogs. The same page set (`release-automation.md`,
  `release-recovery.md`, `development-setup.md`) was updated to eleven or twelve. `release.yml` comments still say
  "eleven-crate". These are pre-existing, but the phase's stated purpose is currency of exactly this count.

**Fix:**

Tighten the sentence to "The twelve publishable crates, in the order `release.yml`'s `publish-crates` job
publishes them (package names, dependency-first):". Update the two remaining "ten" mentions, and the
`release.yml` comments if they are in scope.

### IN-05: Stale and over-long comments in `check-migration-allowlist_test.sh`

**File:** `tests/scripts/check-migration-allowlist_test.sh:16-17`, `:85`, `:321`

**Issue:**

The header comment edit left one comment line well over 100 characters (line 16-17). Line 321 still reads
"live PRIM-05 evidence: 15 pairs = 15 pairs", although the real tree now reports 112 pair lines and the header
was deliberately de-numbered. Line 85 documents `migration_row` as producing "a single well-formed §9.2 data
row", while it is now used under both registers.

**Fix:**

Reword line 321 to "the live PRIM-05 evidence that the pair sets match", line 85 to "a single well-formed
register data row", and re-wrap the header comment.

---

_Reviewed: 2026-10-24_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
