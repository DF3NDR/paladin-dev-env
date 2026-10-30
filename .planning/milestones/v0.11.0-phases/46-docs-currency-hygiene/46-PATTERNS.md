# Phase 46: Docs Currency & Hygiene - Pattern Map

**Mapped:** 2026-10-24
**Files analyzed:** 22 new/modified groups
**Analogs found:** 20 / 22 (two are bookkeeping edits with no code analog)

Line numbers are at HEAD (`ac3a52e0`). RESEARCH.md "Findings That Change the Plan" (F1-F12) takes precedence over CONTEXT where they differ.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `scripts/check-dependency-allowlist.sh` (new, D-21) | gate script | batch / manifest read | `scripts/check-migration-allowlist.sh` + `scripts/check-crate-names.sh` (python `tomllib`) | exact |
| `tests/scripts/check-dependency-allowlist_test.sh` (new) | test harness | batch | `tests/scripts/check-migration-allowlist_test.sh` | exact |
| `Makefile` (`check-gates`, new target) | config | n/a | `Makefile:212-214, 248` | exact |
| `.github/workflows/ci.yml` (new guard step, awk widen) | config | n/a | existing `semver` job step (~441) and `cargo-deny` job gate steps (~120-161) | exact |
| `scripts/check-migration-allowlist.sh` (awk widen to `(9\|10)`) | gate script | batch | itself (line 91) | exact |
| `crates/paladin-core/src/platform/container/battalion/legacy_keys.rs` (new, D-23) | model (serde wire struct) | transform | `cost.rs:70-110` (`CurrencyCode`), `vault.rs:70,218-230` (`Namespace`) | role-match |
| `battalion/mod.rs`, `battalion/conclave.rs` (attribute + rustdoc) | model | transform | same, `cost.rs` | role-match |
| `crates/paladin-battalion/src/commander.rs` (Maneuver `warn!`, rustdoc) | service | request-response | Campaign's one-time `warn!` in the same crate (locate with `grep -rn "warn!" crates/paladin-battalion/src`) | role-match |
| `tests/consumer_surface_muster.rs` (4 pins, D-04) | test (compile-only) | n/a | pins in the same file | exact |
| `docs/src/architecture/treasurer.md` (new) | doc | n/a | `docs/src/architecture/commissary.md` | exact |
| `docs/src/SUMMARY.md` | config | n/a | Commissary entry | exact |
| `docs/src/getting-started/configuration.md` (`## Treasurer` block, Pricing) | doc | n/a | its own "Treasurer allowances" (389) and "Cadence" (516) sections | exact |
| `config.example.yml` (`cadence:` block) | config | n/a | its `treasurer:` block (279-324) | exact |
| `MIGRATION.md` (§10) | doc | n/a | its own §9 skeleton (headings at 22, 188, 441, 468, 506, 632, 944, 950, 1005) | exact |
| `docs/src/api-reference/{upgrading,migration-guide}.md`, `user-guides/switching-providers.md` | doc | n/a | existing pages | exact |
| `docs/src/contributing/development-setup.md`, `appendix/release-{automation,recovery}.md`, `CHANGELOG.md` (+11 crate changelogs) | doc | n/a | in-place edits; edit list in RESEARCH "CURR-22" | exact |
| `.planning/decisions/0015,0059-*.md` | ADR | n/a | dated-note amendments | role-match |
| `.planning/WINDOWS.md`, `STATE.md`, `ROADMAP.md`, `REQUIREMENTS.md` | ledger | n/a | `gsd-tools windows` (append, then waive/fixed) | no code analog |
| 8 archived `VALIDATION.md` files | planning doc | n/a | `28-VALIDATION.md` (only non-template) | role-match |

## Pattern Assignments

### `scripts/check-dependency-allowlist.sh` (gate script, batch)

**Analog:** `scripts/check-migration-allowlist.sh`; manifest parsing from `scripts/check-crate-names.sh`.

**Header and strictness** (check-migration-allowlist.sh:1-12, 53-72): long header comment (purpose, usage, exit codes, "do not improve parsing without updating ci.yml"), then:
```bash
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATION_FILE="${MIGRATION_FILE:-${WORKSPACE_ROOT}/MIGRATION.md}"
...
if [ ! -f "${MIGRATION_FILE}" ]; then
    echo "ERROR: MIGRATION.md not found at ${MIGRATION_FILE}" >&2
    exit 1
fi
SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-migration-allowlist.XXXXXX")"
trap 'rm -rf "${SCRATCH}"' EXIT
```
Copy: workspace root derived from script location, env-var overrides (`CORE_MANIFEST`, `PORTS_MANIFEST`) so the harness can use fixtures, missing input is a named failure, `mktemp -d` scratch with trap, print both sets before `diff -u`, `::error::` message on mismatch.

**Manifest parsing** (check-crate-names.sh:44-83): python3 presence check, then `python3 - args <<'PY'` using `tomllib.load(fh)` on a binary handle. Read the `[dependencies]` table KEYS (not `cargo metadata`, which resolves renames; RESEARCH alternatives). Also include a zero-result guard (check-crate-names prints `ZERO_CRATES`/FAIL on an empty set) so a broken parse is never a silent pass.

**Pinned set:** per RESEARCH F7, `paladin-core` = ADR-0015's fourteen names, `paladin-ports` = ADR-0015's eleven plus `log` (twelve). Amend ADR-0015 with a dated note naming `log` BEFORE pinning. Literal list in the script mirrored in the ADR is acceptable (CONTEXT discretion). Must be shellcheck-clean at `--severity=warning` (`make lint-shell`).

### `tests/scripts/check-dependency-allowlist_test.sh` (harness)

**Analog:** `tests/scripts/check-migration-allowlist_test.sh` (lines 1-90).
- `set -uo pipefail` (not `-e`), `GUARD="${WORKSPACE_ROOT}/scripts/..."`, existence check, `SCRATCH=$(mktemp -d ...)` plus `trap cleanup EXIT`, `FAILED`/`ASSERTIONS` counters.
- Helper builders write fixtures (`write_migration`, `migration_row`, `write_allowlist`); here write minimal `Cargo.toml` fixtures with an extra `[dependencies]` key.
- Failing cases FIRST ("prove it fails"): planted surplus crate in core, planted surplus in ports, missing manifest, empty set. Passing case last runs the guard on the real tree, then asserts the real tree was not mutated.
- `make test-shell-guards` discovers `tests/scripts/*_test.sh` by glob, so no registration is needed.

### `Makefile` + `ci.yml`

**Analog:** Makefile:212-214 and :248.
```make
.PHONY: check-migration-allowlist
check-migration-allowlist: ## Verify ...
	@./scripts/check-migration-allowlist.sh
...
check-gates: check-changelogs check-crate-names check-advisory-register check-workflow-suppressions check-workflow-triggers check-codeql-dismissals check-migration-allowlist check-publish-order ## Run all offline release-gate guards
```
Add `check-dependency-allowlist` as a sibling target and append it to the `check-gates` prerequisites. CI does not call `make check-gates`; it runs each script as its own step, so add a step in the `cargo-deny` job beside the others.

### `scripts/check-migration-allowlist.sh`, `ci.yml`, harness (F1 widen)

Only the register window changes (script line 91, `ci.yml` ~441, harness `write_migration` lines 44-56):
```bash
awk '/^## (9|10)\.2 /{flag=1; next} /^## (9|10)\.3 /{flag=0} flag' ...
```
Keep script and ci.yml byte-identical in the awk. Update the `§9.2` header comments. Add a harness case that a `Y` row under `## 10.2` is read. No allowlist TOML edit.

### `legacy_keys.rs` (D-23 wire struct)

**Analog:** `cost.rs:70` and `vault.rs:70`:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CurrencyCode(String);

impl TryFrom<String> for CurrencyCode {
    type Error = CostError;
    fn try_from(value: String) -> Result<Self, Self::Error> { Self::new(&value) }
}
```
Differences to apply: the existing analogs use `try_from` plus `into` on a newtype. For D-23 use `try_from` only, on `BattalionConfig` and `ConclaveConfig`, whose `Serialize` stays derived, so the struct shape and public surface are unchanged:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "legacy_keys::BattalionConfigWire")]
pub struct BattalionConfig { ... }   // battalion/mod.rs:55-56
```
The private wire struct mirrors the fields, adds `Option<serde::de::IgnoredAny>` (or equivalent) fields for ONLY the four removed keys, and its `TryFrom` returns an error naming the key, its `aegis.*` replacement and the §10.1 row. Do not use `deny_unknown_fields`. The message text and removed names must live in this new file because `tests/legacy_removal_guard.rs` rule 6 (lines ~230-250) forbids the names as whole words in production code of `battalion/mod.rs` and `conclave.rs`, and rule 5 covers five service files incl. `commander.rs`. The wire types need `#[derive(Deserialize)]` with `#[serde(default)]` per field matching the struct's existing defaults (read `mod.rs:56-` fully before writing). RESEARCH has a verified prototype ("Code Examples", past line 437 of 850; not read here, planner should read it). TDD: invert `battalion_config_v0_10_document_deserializes_with_default_aegis` (`battalion/mod.rs:~1062-1085`) first.

### `tests/consumer_surface_muster.rs` (four pins, D-04)

**Analog:** same file. Rules (header lines 9-18): compile-time only, pins live in uncalled functions, the single `#[test]` body is comments (lines 797-801), full struct literals without `..` where the consumer constructs, read pins where the consumer reads.

Read-pin shape to copy for `Stockpile::render()` and `BattalionResult` counts (lines 623-632):
```rust
fn stockpile_read_pin(stockpile: &Stockpile) -> (usize, usize, u32, u32, bool) {
    (stockpile.dispensed.len(), stockpile.shed.len(), stockpile.prompt_tokens,
     stockpile.allotted_tokens, stockpile.exact_tally)
}
```
Signature pin shape (lines 608-618): `let _dispense: fn(&Commissary, &str, &Consignment) -> Result<Stockpile, CommissaryError> = Commissary::dispense;`. Use this for `Stockpile::render` (defined `commissary.rs:219`). `BattalionResult` fields (`battalion/mod.rs:583-595`) follow `node_error_read_pin` (486-506), which iterates and reads. The prelude glob needs one `use paladin::prelude::*;` inside a nested `mod`/fn scope to avoid name clashes with the file's explicit imports (see `prelude_names_agree`, :96, and `prelude_aegis_names_agree`, :511). `ModelProfile` through `paladin_ports::output::llm_port::ModelProfile` (re-export `llm_port.rs:239-241`) follows `prelude_and_ports_agree` (:87). Update header lines 23-24 and :542/:554 citations if §9 to §10 retargeting touches them (F4). `[[test]]` already has `required-features = ["llm-anthropic","llm-deepseek"]` (Cargo.toml:473-476); no change.

### `docs/src/architecture/treasurer.md` (new)

**Analog:** `docs/src/architecture/commissary.md`. Section order: H1 and intro paragraph (crate, file path, facade re-export, ADR pointer); `## Concept` (bulleted responsibilities in bold-code lead-ins, then a "framework owns X, callers own policy / fail-loud" paragraph); `## The model` (two-column `Type | Purpose` table, then "This page names exactly this surface -- no more"); `## Flow` (mermaid `flowchart LR`); `## Usage sketch` (fenced `rust,ignore` with a stated reason, citing source file and line ranges); in-tree caller section; ADR links. Add a `## Rate pacing (Cadence)` section (D-08) linking ADR-0058 and the configuration block, and link `api-reference/platform-api.md#allowance-refusals` rather than duplicating the 429 contract. Worked example is `rust,ignore` or an `{{#include}}` of a compiled anchor in `crates/doc-examples/src/` (make check-doc-examples). Any YAML fence must parse (check-doc-config). Add the entry under Architecture in `SUMMARY.md` next to Commissary; relative links must resolve (mdbook-linkcheck, `warning-policy = "error"`). Vocabulary guards: `treasurer_vocabulary_guard`, `entity_rename_guard` scan new pages.

### `docs/src/getting-started/configuration.md` (`## Treasurer`)

**Analog:** existing sections at :389-515 (allowance) and :516-596 (Cadence). Column set is exactly `| Key | Type | Default | Env override |`, e.g.:
```
| `treasurer.allowance.warn_at` | integer percent `0..=100` | `80` | `APP_TREASURER_ALLOWANCE_WARN_AT` |
| `treasurer.allowance.webhook.url` | non-empty `http(s)` URL | none | none |
```
Regroup the two sections under one `## Treasurer` heading as `###` subsections (existing `###` children such as "Window and limit model" drop to `####` or stay as appropriate), add a new `### Pricing` (`prompt`, `completion`, `cache_read`, `cache_write`, `reasoning`, `currency`, missing-row meaning). Derive each row from `src/config/treasurer.rs` including its `validate()` rule (D-10). Intro paragraph then fenced YAML then table, as in the existing sections. Check anchors referenced elsewhere before renaming headings (`grep -rn "treasurer-allowances\|treasurer-rate-pacing" docs/ README*`).

### `config.example.yml`

Analog: its `treasurer:` block (279-324). Add a `cadence:` block; keep it commented because the file is loaded by tests (`src/infrastructure/web/agent_host.rs:2293-2336`, `src/config/web_server.rs:145-180`).

### `MIGRATION.md` (§10)

**Analog:** §9 skeleton; copy heading text verbatim so the awk windows and anchors match:
`## 10.1 Behavioral changes (user-visible without code changes)`, `## 10.2 Rust API changes (compile-affecting; the X-10 register)`, `## 10.3 Toolchain & dependencies`, `## 10.4 Persistence & schema migrations`, `## 10.5 Configuration & environment`, `## 10.6 HTTP API`, `## 10.7 Deprecations`, `## 10.8 Upgrade checklist`.

Row formats to copy: §9.1 `| M-B-xx | **Bold lead (REQ, Phase, plans).** prose |`; §9.2 table header `| Crate | Type / Trait | Change | Mitigation | Deliberate-breaking? | Req |` with the crate cell as the published package name (`paladin-ai-core`), first column starting `` | ` `` (the awk selects rows by that); §9.8 subsection `### Library consumers: v0.10.1 → v0.11.0` (:1005, moves whole into §10.8, keeping its "consumer-shaped index over register rows, not a new register row" wording). Move-set is `git diff -U0 v0.10.1..HEAD -- MIGRATION.md`, not D-02's list (F2): also §9.3 (450, 464-465), §9.7 (946, 948), §9.8 step 3 TENANT-01 note (975-982). Four rows edited in place (`PaladinResult`, `Settings`, `ExecuteResponse`, `TraceEvent::NodeFinished/RunFinished`) must be restored from `git show v0.10.1:MIGRATION.md` in §9.2 and re-added as new same-key rows in §10.2 (F3). Retitle (line 1) to `# Upgrading Paladin`, keep the 2026-09-20 addendum (:15-20). Avoid the literal placeholder token the CI grep rejects (ci.yml:483-503). Apply G-1..G-7 per RESEARCH table; G-2 code block pattern is `node_error_read_pin` (muster :486-506) with wildcard arm.

### `commander.rs` Maneuver warn (D-24)

Analog: Campaign's one-time `warn!` for ignored `on_error: Absorb`/`retry`; locate it with `grep -rn "warn!" crates/paladin-battalion/src` and `grep -rn "Once\|AtomicBool" crates/paladin-battalion/src` (not read in this pass). Copy its guard mechanism and message shape. Keep rule-5 words (`timeout(`, `timeout_seconds`) out of `commander.rs` production code. Rustdoc target: `BattalionConfig.aegis` (battalion/mod.rs:61-70, existing bullet list "Which fields each pattern honours").

### Decision/ledger edits

- ADR-0015 and ADR-0059: append a dated note (house rule: historical records are amended, not rewritten). PROMOTION.md unchanged (no new ADR).
- WINDOWS rows: sequential `gsd-tools windows append --kind deviation --phase 46 --description ...` then `windows waive <id> "<reason>"` or `windows fixed <id>` (F6). Never in parallel; ids 80-82 expected.
- Nyquist: follow `28-VALIDATION.md` structure; reconcile-only (skip the workflow's AskUserQuestion and auditor steps, F9).

## Shared Patterns

### Gate-script house shape
**Source:** `scripts/check-migration-allowlist.sh`, `scripts/check-crate-names.sh`, `tests/scripts/*_test.sh`.
**Apply to:** the D-21 guard and its harness. Four parts always travel together: script, harness, Makefile target in `check-gates`, `ci.yml` step.

### Same-commit registration
**Source:** CONTEXT "Established Patterns".
**Apply to:** D-23 (code + §10.1 row + ADR-0059 note + CHANGELOG line + inverted test in one commit); §10 move + awk widen + harness in one commit.

### Historical-vs-live citations
**Apply to:** F4 retargeting. Only retarget `§9.x` citations that name a v0.11 row (live `[Unreleased]` CHANGELOG, `switching-providers.md:137`, `paladin-llm/src/registry/seeded.rs` comments, `conclave_execution_service.rs:745`, muster header). Sealed phases, ADRs and `[0.10.x]` sections stay.

### Guards that scan new text
`tests/legacy_removal_guard.rs`, `entity_rename_guard`, `treasurer_vocabulary_guard`, `make check-doc-config`, `make check-doc-examples`, `make lint-shell`: run after any docs or script addition.

## No Analog Found

| File | Role | Reason |
|---|---|---|
| `.planning/WINDOWS.md` rows | ledger | written only through `gsd-tools windows`; no source pattern |
| STATE/ROADMAP/REQUIREMENTS edits | bookkeeping | prose table rewrites |

## Metadata

**Analog search scope:** `scripts/`, `tests/scripts/`, `tests/consumer_surface_muster.rs`, `Makefile`, `docs/src/architecture`, `docs/src/getting-started`, `MIGRATION.md`, `crates/paladin-core/src/platform/container/`.
**Not read (planner must open):** RESEARCH.md lines 438-850 (D-23 prototype, pitfalls, guard design, Nyquist procedure, open questions); the Campaign `warn!` site; `src/config/treasurer.rs`.
**Pattern extraction date:** 2026-10-24
