---
phase: 44-legacy-clean-break-removal
plan: 11
subsystem: migration-register
tags: [migration, semver-checks, allowlist, x-10, clean-break, d-27]
status: complete

requires:
  - phase: 44-03
    provides: measured Conclave / breaker delta table (9.1 M-B-09, M-B-10)
  - phase: 44-06
    provides: approved node_errors wire shape and the v0.10 strict-rejection test (9.1 M-B-08, 9.4 note)
  - phase: 44-07
    provides: Commander / Conclave on Aegis (9.1 M-B-06, M-B-09, M-B-11)
  - phase: 44-10
    provides: the deletions the D-27 diagnostic measured
provides:
  - MIGRATION.md 9.1 rows M-B-06..M-B-11, the 9.4 stored-JSON bullet and the 9.7 correction
  - ten Phase 44 rows in MIGRATION.md 9.2, every Y/N cell ending with the measured result
  - fifteen-entry Phase 44 block in .cargo/semver-checks-allowlist.toml
  - struct_missing / enum_missing / enum_variant_missing allows in paladin-ai-core and a new lints table (function_missing, module_missing) in paladin-battalion
affects: [44-12, 44-13]

tech-stack:
  added: []
  patterns:
    - "Measured, not predicted: every lint id and Y/N verdict comes from the D-27 run below"

key-files:
  created: []
  modified:
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - crates/paladin-core/Cargo.toml
    - crates/paladin-battalion/Cargo.toml

key-decisions:
  - "The paladin-ai facade fired no lint for the removed retry re-export, so there is no paladin-ai | retry row; the paladin-battalion | retry row records that"
  - "BattalionResult node_errors retype is an N row (the tool models no field type change), with a 0-lints note and no allowlist entry"
  - "Root Cargo.toml unchanged: paladin-ai fired no Phase 44 lint"

patterns-established:
  - "Manifest comments name removed items by concept, never by path form (44-13 scans *.toml outside dot-directories)"

requirements-completed: []
requirements-advanced: [LEGACY-04]  # stays pending until 44-12 (CHANGELOG, ADR conforms) and 44-13 (guard) land

duration: ~1h30min
completed: 2026-10-11
---

# Phase 44 Plan 11: X-10 register for the legacy clean break Summary

**Every Phase 44 break is registered from a measured cargo-semver-checks 0.50.0 run against the published 0.10.1: six 9.1 behaviour rows, one 9.4 stored-JSON bullet, ten 9.2 rows (nine `Y`, one `N`), fifteen allowlist entries and five new crate-level allow lines, with the set-equality gate green.**

## Task Commits

1. **Task 1: 9.1 rows, 9.4 bullet, 9.7 correction** - `03e3068c` (docs)
2. **Task 2: D-27 measurement, 9.2 rows, allowlist block, allow lines** - `e767fc74` (docs)

## Tool

`cargo semver-checks` was not installed. The CI-pinned prebuilt `v0.50.0` `x86_64-unknown-linux-gnu` release asset was downloaded with `curl` (HTTP 200) from the official `obi1kenobi/cargo-semver-checks` GitHub release into the session scratchpad, extracted there, and invoked by absolute path (`cargo-semver-checks 0.50.0`). It was never placed in the repository, committed or added to a manifest. tarball sha256 `52a65dc88dc53fa8b57d6087954eb52cda149ca03bcfca78ce3fdecd23f893c4`.

## D-27 diagnostic (0.50.0 vs the published 0.10.1, `--release-type minor`, `--default-features`, crate-wide allows disabled)

Each manifest was byte-copied, its `[package.metadata.cargo-semver-checks.lints]` allow lines removed, the tool run, and the manifest restored from the copy with `git diff --quiet` confirmed.

| Package | Checks | Lints fired (crate-wide) | Phase 44 lints | Manifest |
|---|---|---|---|---|
| `paladin-ai-core` | 196 (187 pass, 9 fail) | `constructible_struct_adds_field`, `enum_marked_non_exhaustive`, `enum_missing`, `enum_no_repr_variant_discriminant_changed`, `enum_struct_variant_field_added`, `enum_variant_missing`, `inherent_method_missing`, `struct_missing`, `struct_pub_field_missing` | `BattalionConfig`: `constructible_struct_adds_field` (`aegis`), `inherent_method_missing` (3 builders), `struct_pub_field_missing` (3 fields). `RetryPolicy`: `struct_missing`. `ErrorStrategy`: `enum_missing`. `NodeError`: `struct_missing`. `BattalionError`: `enum_variant_missing` (`Timeout`). `PaladinError`: `enum_variant_missing` (`LlmError`), `inherent_method_missing` (`is_retryable`, `is_terminal`). `ConclaveConfig`: `inherent_method_missing` (`with_timeout`), `struct_pub_field_missing` (`timeout_seconds`). `ConclaveError`: `enum_variant_missing` (`Timeout`). `BattalionResult`: none | RESTORED_CLEAN |
| `paladin-battalion` | 196 (193 pass, 3 fail) | `enum_struct_variant_field_added`, `function_missing`, `module_missing` | `retry`: `function_missing` (`should_retry`, `calculate_retry_delay`), `module_missing` (`retry`) | RESTORED_CLEAN (no crate-wide table existed; nothing to strip) |
| `paladin-herald` | 196 (196 pass) | none | none | RESTORED_CLEAN (no table) |
| `paladin-ports` | 196 (193 pass, 3 fail) | `constructible_struct_adds_field`, `enum_unit_variant_changed_kind`, `enum_variant_marked_non_exhaustive` | none (all earlier phases) | RESTORED_CLEAN |
| `paladin-eval` (`publish = true`) | 196 (196 pass) | none | none | RESTORED_CLEAN (no table) |
| `paladin-ai` | 196 (195 pass, 1 fail) | `constructible_struct_adds_field` | none (`AgentRuntimeDeps.cadence`, `WebhookPayload.halt_reason`, `Settings.treasurer`, `ApiKeyConfig.tenant`, `BearerTokenAuthConfig.tenant`, all earlier phases); the facade's removed `retry` re-export fired nothing | RESTORED_CLEAN |

Earlier-phase items among those lints (listed, not registered here): `PaladinResult.cost` (Phase 42/43), `RunFinishStatus` and `RunEventKind` (`enum_marked_non_exhaustive`), `RunFinishStatus::AwaitingInput` discriminant 3 to 4, `TraceEvent::NodeFinished.cost`, `TraceEvent::RunFinished.cost` and `.halt_reason`, `RunOutcome::Halted.cause` (Phase 42, `paladin-battalion`), and the `paladin-ports` and `paladin-ai` fields above.

The `node_errors` element retype (`BattalionResult`) fired no lint, as research A1 predicted, so its 9.2 row is `N` with the measured `0 on this item` note and has no allowlist entry.

**Confirmation with the final manifests** (0.10.1, same options): `paladin-ai-core` now reports only `enum_no_repr_variant_discriminant_changed` and `enum_struct_variant_field_added` (188 checks, 2 fail) and `paladin-battalion` only `enum_struct_variant_field_added` (194 checks, 1 fail) -- all earlier-phase, none of this plan's. This shows the new allow lines cover exactly the lints measured above and that every lint id is accepted by the tool.

## CI-parity (`--package <pkg> --default-features --baseline-version 0.9.0`, the `ci.yml` list)

| Package | Exit | Note |
|---|---|---|
| `paladin-ai` | 0 | 0 checks, 254 skip (the documented no-op against 0.9.0) |
| `paladin-ai-core` | 0 | same; re-run on the final manifests, 0 |
| `paladin-ports` | 0 | same |
| `paladin-battalion` | 0 | same; re-run on the final manifests, 0 |
| `paladin-herald` | 0 | same |
| `paladin-llm` | 0 | same |
| `paladin-memory` | 0 | same |
| `paladin-storage` | 0 | same |
| `paladin-notifications` | 0 | same |
| `paladin-content` | 0 | same |
| `paladin-web` | **101, not measured** | see Deviation 1 |

As in 43-13, the 0.9.0 run proves nothing about Phase 44 (the baseline is a no-op); the 0.10.1 run above is the measurement.

## Verification Results (all actually run)

| Check | Result |
|---|---|
| `./scripts/check-migration-allowlist.sh` | set-equal (70 pair lines across both lists) |
| `bash tests/scripts/check-migration-allowlist_test.sh` | 9 assertions passed (run on the committed tree; on the dirty tree only its own no-mutation assertion failed, as designed) |
| `make check-gates` | exit 0 (allowlist set-equal, 12 crates in publish order) |
| `cargo check --workspace` | exit 0 |
| `cargo fmt --check` | clean |
| `awk '/^## 9\.2 /,/^## 9\.3 /' MIGRATION.md \| grep -c 'plan 44-11'` | 10 |
| Plan gate-161 grep over `git diff b362d6cb` of the three manifests | prints nothing (only allow lines, the new table header and comments changed) |
| Task 1 verify loop (`M-B-06`..`M-B-11`, the 9.4 bullet) and 9.7 `Phase 44 (v0.11) removes them outright` | present |

Not run (nothing they cover changed): `make api-surface` (no source or `pub` item changed; the baseline was regenerated in 44-10), `make deny` / `make security` (no dependency change), `cargo test --workspace` (documentation, manifest metadata and allowlist edits only; `cargo check --workspace` passed). The two known `paladin-ai` baseline test failures in `deferred-items.md` were therefore not exercised.

## Deviations from Plan

1. **[Environmental] `paladin-web` CI-parity not measured.** `cargo semver-checks` could not build rustdoc for the published `paladin-web` 0.9.0: its dependency `utoipa-swagger-ui` 9.0.2 has a build script that downloads the Swagger UI archive from GitHub, and the session's GitHub proxy answered `403` ("GitHub access to this repository is not enabled for this session"), so the archive was invalid (`Could not find EOCD`), exit 101. I did not work around the proxy. `paladin-web` has no diff in Phase 44 (`git diff --stat b362d6cb HEAD -- crates/paladin-web` is empty), the other ten packages exit 0, and `paladin-web` is not part of the D-27 0.10.1 diagnostic list. CI will run it with network access.
2. **[Environmental] Disk.** Free space fell to 270 MB during the CI-parity loop. I deleted the regenerable `target/semver-checks` caches (the tool's own rustdoc builds) and nothing else; no `ENOSPC` occurred and `target/debug` was not touched.
3. **[Permission] Proxy diagnostics refused.** An early attempt to read the session proxy README and status endpoint was refused by the permission classifier; I did not retry or route around it and used only the plan's own `curl` download step, which succeeded.
4. **[Wording] A `Y` row for `paladin-battalion | retry` in place of a `paladin-ai | retry` row.** The plan allowed a facade row only if the facade's run fired a lint; it did not, so the facade re-export is described in the `paladin-battalion | retry` row.

No Rule 1-4 code deviations. No auth gates. No files outside the plan's `files_modified` list changed (root `Cargo.toml` deliberately untouched).

## Items for the operator

- `enum_no_repr_variant_discriminant_changed` (`RunFinishStatus::AwaitingInput` 3 to 4), `enum_struct_variant_field_added` (`TraceEvent` fields in `paladin-ai-core`, `RunOutcome::Halted.cause` in `paladin-battalion`) fire against 0.10.1 and have no crate-wide allow. They are earlier-phase items (Phase 42), their rows say `N/A -- absent at the v0.9.0 baseline`, and CI compares against 0.9.0, so nothing fails. Not touched here (the 43-13 summary flagged the `RunOutcome` one already).
- LEGACY-04 stays pending: 44-12 (CHANGELOG, ADR-0059 `conforms`) and 44-13 (source-tree guard) remain. `requirements.mark-complete` was not run.

## Known Stubs

None.

## Threat Flags

None. T-44-28 mitigated (measured per package, one row per pair, set-equality gate green); T-44-SC mitigated (exact pinned asset over TLS into the scratchpad, never committed); T-44-29 mitigated (cache deleted when space ran low).

## Self-Check: PASSED

- Commits `03e3068c` and `e767fc74` are on `claude/laughing-dirac-e0h2ax`.
- Modified files exist: `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `crates/paladin-core/Cargo.toml`, `crates/paladin-battalion/Cargo.toml`.
- Manifests: `git diff --quiet` was true for every stripped manifest after each restore; the final `git status` showed only the four intended files.
