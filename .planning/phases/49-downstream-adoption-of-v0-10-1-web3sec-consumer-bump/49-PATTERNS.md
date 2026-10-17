# Phase 49: Downstream Adoption of v0.10.1 (web3sec Consumer Bump) - Pattern Map

**Mapped:** 2026-10-14
**Files analyzed:** 7 (1 new test, 1 Cargo.toml edit, 2 doc edits, 3 new phase docs; optional Makefile alias noted)
**Analogs found:** 7 / 7 (the muster has role-match analogs only; no existing test is a compile-only pin test)

No CONTEXT.md exists. The file list comes from 49-RESEARCH.md ("Recommended Project Structure", Pattern 3, plan decomposition). The 278-line draft muster lives in 49-RESEARCH.md (lines past 315 were not re-read here). Use it as the body source and this file for conventions.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `tests/consumer_surface_muster.rs` | test (compile-only contract) | transform (compile-time) | `tests/entity_rename_guard.rs` (header and doc style); `tests/helpers/mock_paladin_port.rs` (trait-impl imports) | role-match |
| `Cargo.toml` `[[test]] consumer_surface_muster` | config | n/a | `vision_integration` and `e2e_platform_api` entries, `Cargo.toml` ~lines 366-369, 408-411 | exact |
| `MIGRATION.md` `### Library consumers` under §9.8 | docs | n/a | §9.8 checklist (starts line 882); §9.2 rows (line 178) | role-match |
| `CHANGELOG.md` `[Unreleased]` entries | docs | n/a | `### Added` (line 11), `### Documentation` (line 1381 is a released section) | exact |
| `49-CONSUMER-ADOPTION-SPEC.md` | phase doc | n/a | `48-PATTERNS.md`, `48-VALIDATION.md` (spec-like); none is a close analog | partial |
| `49-RETURN-report.md` | phase doc (template) | n/a | `48-UAT.md` (frontmatter and status fields) | partial |
| `49-MUSTER-RED-EVIDENCE.md` | phase doc (evidence) | n/a | `.planning/phases/48-base-entity-rename/COVERAGE.md`, `48-SECURITY.md` | role-match |
| `Makefile` `check-consumer-surface` (optional) | config | n/a | `check-release-consistency` convention, `Makefile:221-226` | role-match |

## Pattern Assignments

### `tests/consumer_surface_muster.rs` (test, compile-time)

**Analogs:** `tests/entity_rename_guard.rs`, `tests/legacy_removal_guard.rs`, `tests/treasurer_vocabulary_guard.rs`, `tests/helpers/mock_paladin_port.rs`

**Header convention** (`entity_rename_guard.rs` lines 1-10; `treasurer_vocabulary_guard.rs` lines 1-5). Use a module `//!` doc that:
- names the phase and requirement id (e.g. "Phase 49 consumer-surface muster (ADOPT-04)");
- explains the rule and why the guard exists;
- ends with a run command block:
```rust
//! ```bash
//! cargo test --test consumer_surface_muster
//! ```
```
Add the strictness policy and "red here means update the pin and its §9.2 row in the same commit" (RESEARCH Pattern 1).

**Precedent to differ from.** The three guards are std-only repository scans. They use `std::fs`, `std::path`, `AtomicU32` (guard lines 49-52), a positive-control self-test, and `concat!`-assembled forbidden strings so the file never matches its own scan. The muster is NOT a scan.

**Scanner interaction (critical).** `entity_rename_guard` scans `.rs` files, skipping only itself, `.planning`, `.git`, `CHANGELOG.md`, `MIGRATION.md` and the semver allowlist. The muster is scanned. It must NOT contain `Node<`, `Node::new`, `battalion::ErrorStrategy`, `battalion::NodeError`, or a `battalion::{...}` import naming them. `treasurer_vocabulary_guard` bans the fixture term outside `.planning/` and `.project/`, and bans the officer word under `examples/`, `benches/` and `fixtures/` (the muster sits in `tests/`, so only the fixture term matters). Describe removals only in MIGRATION.md or `.planning/`. After writing, run all three guards alongside the muster.

**Imports for trait stubs** (`tests/helpers/mock_paladin_port.rs` lines 5-19). The facade is `paladin`, and the direct path deps are `paladin_ports` and `paladin_memory`:
```rust
use async_trait::async_trait;
use std::sync::Arc;
use paladin::application::services::paladin::error::PaladinError;
use paladin::core::platform::container::paladin::Paladin;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_port::{
    PaladinPort, PaladinResult, PaladinStream, PaladinStreamChunk, StopReason,
};
```
**Trait impl shape** (same file, lines 41-50). Implement only the required methods (`LlmPort` has 6, `PaladinPort` has `execute`, `execute_stream`, `validate`, `ArsenalPort` has 3), then coerce to `Arc<dyn Trait>`:
```rust
#[async_trait]
impl PaladinPort for MockPaladinPort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> { ... }
```
The muster's stubs must be never-called and must not use `unwrap`, `expect` or `panic!` (return `Err(...)`). Other implementers to cross-check signatures: `tests/helpers/mock_arsenal_adapter.rs`, `tests/helpers/mock_llm_adapter.rs`.

**Core pattern.** Follow RESEARCH Pattern 1 (the pin-class table) and Pattern 2:
- put every pin in a never-called `fn` or `async fn`;
- add exactly one trivial `#[test]` so `cargo test` reports a passing target;
- never execute a pin (no `MCPClient::connect_stdio`, no `DeepSeekConfig::from_env()` call);
- pin full struct literals with no `..`, except `CommissaryPlan` and `TokenUsage`;
- exclude `looks_like_auth_failure` (private) and the removed items DR-1, DR-2 and DR-3;
- carry one documented `#[allow(...)]` (RESEARCH says the draft needs exactly one).

---

### `Cargo.toml` `[[test]]` table (config)

**Analog:** `Cargo.toml` lines 326-420 (`vision_integration`, `e2e_platform_api`). Insert beside them:
```toml
[[test]]
name = "vision_integration"
path = "tests/integration/vision_integration_test.rs"
required-features = ["vision", "llm-openai", "llm-anthropic"]
```
New entry (name must equal the file stem so Cargo produces one target):
```toml
[[test]]
name = "consumer_surface_muster"
path = "tests/consumer_surface_muster.rs"
required-features = ["llm-anthropic", "llm-deepseek"]
```
The three guard tests are not declared and use auto-discovery. The muster is declared only because it needs features.

---

### `MIGRATION.md` `### Library consumers` (docs)

**Analog:** §9.8 "Upgrade checklist" (`MIGRATION.md:882`). It is an ordered list of bold-led imperative items that cite concrete files and commands plus §-anchors, e.g.:
```
1. **Back up state.** Snapshot every state directory ... (§9.4's ...)
3. **Update config — nothing is required.** ... (§9.5)
   **Phase 40 note (TENANT-01, the one exception — see §9.5):** ...
```
Copy these conventions:
- bold lead-in per item, cross-references written as `§9.2`, and bold "Phase N note" call-outs;
- the new `###` goes at the END of §9.8, which is the end of the file region after the step-7 "Verify" item;
- do NOT add a row to §9.2 and do NOT add a `## 9.9`. `scripts/check-migration-allowlist.sh` reads only §9.2, so a `###` under §9.8 is safe;
- content comes from the RESEARCH drift register DR-1..DR-11 (cross-reference the existing §9.2 rows) plus the D-2 pitfall: the `total_tokens` fallback is not additive under `TokenUsage`'s `Add`.

MIGRATION.md is exempt from the entity and legacy guards by name, so it may name the removed items.

---

### `CHANGELOG.md` `[Unreleased]` (docs)

**Analog:** `CHANGELOG.md` lines 8-14. This is Keep-a-Changelog with `### Added` under `## [Unreleased]`, and bold-led, multi-line, wrapped-at-100 bullets that name the phase and plan:
```
- **Rate pacing, the Cadence (PACE-02; Phase 43 plan 43-01).** A new `CadencePort` ...
```
Add an Added bullet such as "**Consumer-surface muster (ADOPT-04; Phase 49).** ..." and a Documentation or Changed bullet for the consumer note. Check whether `[Unreleased]` already has a `### Documentation` heading before adding one (the heading at line 1381 belongs to a released section). Do not touch released sections.

---

### `49-MUSTER-RED-EVIDENCE.md` (phase doc)

**Analogs:** `48-SECURITY.md` (frontmatter plus evidence tables, "Evidence was checked on disk, not copied from the SUMMARYs") and `COVERAGE.md`. Frontmatter style:
```yaml
---
phase: 48
slug: base-entity-rename
status: verified
created: 2026-10-14
---
```
Record per mutation: the command, the exit code, the error location (must be the muster file), and that the only `could not compile` line names the muster target. Also record `git status --short` clean afterward, and the three scripted mutations from RESEARCH "Proven Red" (Commissary bool, dropped facade re-export, field pin).

### `49-RETURN-report.md` (phase doc template)

**Analog:** `48-UAT.md`. Frontmatter is `status`, `phase`, `source`, `started`, `updated`, followed by a "Current Test" section. Fields the consumer fills are `PENDING`, including the consumer SHA and gate totals (≥2,592 / 0 / 2). Paladin-side facts are filled in: tag object `7593ab4d…`, commit `f7dae267`, and `4b7bed87` byte-identical.

### `49-CONSUMER-ADOPTION-SPEC.md` (phase doc)

**Analogs:** `48-PATTERNS.md` and `48-VALIDATION.md` for tone only. Content comes from handoff §7 plus the D-2 lossless design plus pre-flight greps (RESEARCH "Consumer-side deliverable" section, not re-read here). ADOPT-01..03 stay Pending until the return report is filed (plan 49-05, `autonomous: false`).

### `Makefile` `check-consumer-surface` (optional)

**Analog:** the `check-release-consistency` convention (`Makefile:221-226`). The target uses the `## help text` style and the `$(CARGO)` variable. Keep it OUT of `check-gates`:
```make
.PHONY: check-consumer-surface
check-consumer-surface: ## Compile and run the consumer-surface muster (ADOPT-04); not part of check-gates (it builds)
	@$(CARGO) test --test consumer_surface_muster
```

## Shared Patterns

### Clean break, zero API change
**Apply to:** every file.
- No `pub` flips, shims, `#[deprecated]` aliases, `from_total` constructors or `LlmResponse::new`.
- Zero new §9.2 rows and zero semver-allowlist changes.
- `make api-surface` must show no diff.

### Ubiquitous language
**Apply to:** all new code and docs. Use "muster" and "roll", never "compat check". `Entity<T>` stays generic. "node" means only a WarGraph/Campaign vertex.

### Gate sequence per CLAUDE.md
**Apply to:** the test and Cargo.toml plans.
- Run `cargo test` (muster plus the three guards), then `cargo fmt --check`, then `cargo clippy --workspace --all-targets --all-features -- -D warnings`, then `make api-surface`.
- `make clean-code` already compiles the muster via `cargo check --all-targets`.
- A compile-only test adds no instrumented library line, so the 82% coverage floor is unaffected.

### No unwrap, expect or panic
Test code may use them per the CLAUDE.md "library code" wording. The muster stubs should return `Err` instead, because they are never called.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| compile-time `use` + fn-pointer pins inside `tests/consumer_surface_muster.rs` | test | compile-time | The existing guards are runtime std-only file scanners. Nothing in the repo is a compile-only contract test (`trybuild` and `compile_fail` are absent). Use RESEARCH Pattern 1 and the draft muster. |
| `49-CONSUMER-ADOPTION-SPEC.md` | phase doc | n/a | There is no prior external-consumer spec. Use RESEARCH and handoff §7. |

## Metadata

**Analog search scope:** `tests/*.rs`, `tests/helpers/`, root `Cargo.toml` `[[test]]` tables, `MIGRATION.md` §9.8, `CHANGELOG.md` head, `.planning/phases/48-base-entity-rename/`
**Files scanned:** about 12 (RESEARCH.md was read only through line 315 of 778; the draft muster body and the consumer-spec sections were not re-read)
**Pattern extraction date:** 2026-10-14
