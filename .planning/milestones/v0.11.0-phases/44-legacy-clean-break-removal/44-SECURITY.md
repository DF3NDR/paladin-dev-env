---
phase: 44
slug: legacy-clean-break-removal
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-10-11
---

# Phase 44 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time — all 13 PLAN files (44-01 .. 44-13) carry a
`<threat_model>` block; 35 threats in total (29 `mitigate`, 6 `accept`, 0 `transfer`).
Verification depth: ASVS L1 (mitigation present in the cited file). Every `mitigate`
control below was located by grep in the implementation, not taken from SUMMARY prose;
`make security` and `make check-migration-allowlist` were re-run directly on this audit.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| caller config -> Battalion services (Formation, Phalanx, Campaign, Commander, Conclave) | `BattalionConfig.aegis` (possibly deserialized from JSON) controls attempt counts, bounds, backoff and error handling | retry / timeout / handler policy — untrusted when deserialized |
| operator YAML -> CLI Battalion config | the CLI's `timeout_seconds` / `retry_attempts` keys become Aegis policy | operator-supplied durations and counts |
| LLM provider response -> error classification | provider-controlled message text reaches retry and breaker decisions | provider error text (already redacted upstream, D-34); classification must read only `transience()` |
| PaladinPort result -> logs / NodeError | provider error text crosses into structured records and log lines | redacted error text; must never carry keys, inputs or outputs |
| BattalionResult -> herald output / stored JSON | per-node failure text leaves the process through heralds and exported metadata | `NodeError { node_id, attempt, transience, source }` |
| stored JSON (v0.10) -> BattalionConfig / BattalionResult | previously written documents are read back after the clean break | old keys (ignored) / old `node_errors` shape (rejected) |
| published crates -> downstream consumer | public items leave `paladin-ai-core`, `paladin-battalion` and the facade | breaking API removals under ADR-0059 |
| documentation -> operators and the downstream consumer | docs decide whether readers believe a run is time-bounded | timeout semantics (per attempt, not per run) |
| repository -> release | the register and gates decide what ships as breaking | MIGRATION.md §9.x rows, allowlist, CHANGELOG |
| downloaded tool binary -> developer machine | an external release asset executed during the D-27 diagnostic | `cargo-semver-checks` v0.50.0 tarball |
| source tree -> later changes | a later commit could reintroduce a removed name, a shim or a second timeout site | guard-test scope |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-44-01 | Denial of Service | Campaign without its 300 s whole-run wrapper | medium | mitigate | per-attempt bound through the one runner: `crates/paladin-battalion/src/aegis_attempt.rs:204` (`attempt_bound(aegis.timeout)`); module doc `:8` states the bound is per attempt (D-02); Commander builder keeps 300 s per attempt `commander.rs:1814`; 9.1 row M-B-06 | closed |
| T-44-02 | Denial of Service | retry amplification from a hostile or mistaken policy | medium | mitigate | `BattalionConfig::validate_aegis` (`crates/paladin-core/src/platform/container/battalion/mod.rs:191`) rejects `RetryPredicate::Custom`, zero `run_timeout` / `idle_timeout`; `Aegis::validate` rejects `max_attempts == 0`; tests `validate_aegis_rejects_zero_max_attempts`, `_rejects_custom_retry_predicate`, `_rejects_zero_durations` (`mod.rs:954-1007`); backoff capped by `max_interval` in `engine/retry.rs:51-56` | closed |
| T-44-03 | Information Disclosure | runner retry log lines | low | mitigate | `aegis_attempt.rs:233-238`: `warn!` carries Paladin name, attempt, transience and delay only (comment cites T-44-03); 44-12 credential review item 2 enumerates every added log macro | closed |
| T-44-04 | Tampering | malformed deserialized `aegis` (zero durations, unsupported handlers) | low | mitigate | `campaign_service.rs:205` calls `validate_aegis()` at `execute` entry before any port call; test `campaign_rejects_invalid_aegis_before_any_node_runs` (`:1140`) | closed |
| T-44-05 | Repudiation | an unlicensed or undocumented public-API removal | medium | mitigate | `.planning/decisions/0059-legacy-clean-break-removal.md` landed in 44-02 before any removal plan; D-00a scope sentence "superseded for Phase 44 only" (`:28`); register and allowlist in MIGRATION.md §9.2 and `.cargo/semver-checks-allowlist.toml` | closed |
| T-44-06 | Spoofing | citing ADR-0051 as licence for a later phase | low | mitigate | ADR-0059 `:18` and `:28`: "This ADR does not inherit ADR-0051 ... never as its licence ... A later removal records its own supersession" | closed |
| T-44-07 | Tampering | retry / breaker decisions driven by provider-controlled message text | medium | mitigate | Conclave predicate reads `error.transience() == Transience::Transient` (`conclave_execution_service.rs:371`); both breaker sites `src/infrastructure/resilience/circuit_breaker.rs:238` and `:297` gate `on_failure()` on `transience()`; test `conclave_retry_predicate_ignores_message_text` (`:764`) and the breaker "Permanent with retry-ish message text must not count" test (`circuit_breaker.rs:538`) | closed |
| T-44-08 | Denial of Service | breaker no longer opens on Permanent or Unknown LLM failures | low | accept | accepted risk AR-44-01; recorded for operators in MIGRATION.md §9.1 row M-B-10 | closed |
| T-44-09 | Information Disclosure | rewritten rustdoc and tests | low | accept | accepted risk AR-44-02; 44-12 credential review item 4 confirms no HTTP client or redirect policy changed | closed |
| T-44-10 | Denial of Service | a long Formation has no whole-run bound (D-02) | medium | accept | accepted risk AR-44-03; `formation_service.rs:166` bounds each attempt via `validate_aegis` + runner; 9.1 row M-B-06 | closed |
| T-44-11 | Tampering | an unsupported handler slipping past validation and treated as Absorb | low | mitigate | `validate_aegis` at `formation_service.rs:166`; fail-closed wildcard arm returns `ValidationError` (`:266-270`); test `formation_rejects_invalid_aegis_before_any_paladin_runs` (`:801`) asserts zero Paladin calls | closed |
| T-44-12 | Denial of Service | retry amplification across a wide Phalanx (N × max_attempts) | medium | mitigate | `Aegis` derives `Default` with `retry: None` (`aegis.rs:42-46`) so no retry by default; `validate_aegis` at both Phalanx entry points (`phalanx_service.rs:159`, `:185`); permit held across the whole attempt sequence, test `phalanx_concurrency_limit_holds_a_permit_across_retries` (`:1516`); backoff capped by `max_interval` | closed |
| T-44-13 | Information Disclosure | AggregationError message embeds each failure's NodeError display | low | accept | accepted risk AR-44-04; 44-12 credential review item 1 traces the text to the already-redacted `LlmFailure.message` | closed |
| T-44-14 | Denial of Service | a panicking unwrap on a closed semaphore or missing majority entry | low | mitigate | non-test `phalanx_service.rs` contains zero `unwrap()` (grep up to `#[cfg(test)]` on this audit); closed limiter and missing majority become recorded failures / `BattalionError`; follow-up `dc55f18e` also rejects a limit above `Semaphore::MAX_PERMITS` instead of panicking | closed |
| T-44-15 | Information Disclosure | the richer herald line | medium | mitigate | test `herald_node_error_line_adds_no_provider_detail` (`crates/paladin-herald/src/markdown_herald.rs:870`) pins that provider and HTTP status are not added; 44-12 credential review item 1 | closed |
| T-44-16 | Tampering | old or crafted JSON replayed into the new `node_errors` type | low | mitigate | strict typed deserialization — no `serde(alias)`, `untagged` or compatibility path under `crates/paladin-core/src/platform/container/battalion/` (grep on this audit); test `battalion_result_rejects_v0_10_node_errors_shape` (`mod.rs:1653`) | closed |
| T-44-17 | Repudiation | an unconfirmed change to a persisted contract | medium | mitigate | blocking `checkpoint:decision` recorded in 44-06-SUMMARY (option-a, 2026-10-11); ADR-0059 §(g) carries the checkpoint outcome sentence; MIGRATION.md §9.1 M-B-08 and §9.4 | closed |
| T-44-18 | Denial of Service | ChainOfCommand / Grove routed through the Commander are unbounded | medium | accept | accepted risk AR-44-05; `CommanderBuilder` default keeps 300 s per attempt (`commander.rs:1814`); `execute` rustdoc `# Timeouts` tells callers to wrap in `tokio::time::timeout` (`:440-447`); 9.1 row M-B-06 | closed |
| T-44-19 | Denial of Service | Conclave retry volume | low | mitigate | `ConclaveConfig::with_retry_attempts` clamps to `min(5)` (`crates/paladin-core/src/platform/container/battalion/conclave.rs:159`); backoff via `engine::retry::backoff_delay` (`conclave_execution_service.rs:342`) capped by `max_interval`; Transient-only predicate (`:371`) | closed |
| T-44-20 | Tampering | an invalid policy reaching the Conclave or the Commander | low | mitigate | `validate_aegis` at `CommanderBuilder::build` (`commander.rs:1825`), `Conclave::validate` (`conclave.rs:318`, test `conclave_validate_delegates_to_validate_aegis`) and `ConclaveExecutionService::execute` (`conclave_execution_service.rs:109`) | closed |
| T-44-21 | Denial of Service | CLI runs lose the whole-run 600 s / 300 s bound | low | accept | accepted risk AR-44-06; 9.1 row M-B-06 names the CLI change explicitly | closed |
| T-44-22 | Tampering | `timeout_seconds: 0` from operator YAML | low | mitigate | CLI zero check retained at `src/application/cli/config/battalion_config.rs:323-325`; `validate_aegis` rejects a zero bound before any Paladin runs | closed |
| T-44-23 | Denial of Service | readers assuming a whole-Battalion timeout that no longer exists | medium | mitigate | 23 "per attempt" statements across `docs/src`; `orchestration.md:338` "Timeouts are per attempt, not per run. There is no whole-Battalion wall clock"; the `tokio::time::timeout` wrapper shown in `fault-tolerance.md:84`, `battalion-patterns.md:326`, `orchestration.md:341`, `appendix/conclave-pattern.md:233` | closed |
| T-44-24 | Repudiation | docs teaching removed APIs | low | mitigate | `tests/legacy_removal_guard.rs` scans `.md` files including `docs/src`, skipping only the history set (`migration-guide.md`, `upgrading.md`, every `CHANGELOG.md`, `MIGRATION.md`, dot-directories) | closed |
| T-44-25 | Tampering | a shim or alias silently reviving a removed name | low | mitigate | no `deprecated` / `serde(alias` / `untagged` / `pub use .. as` under core battalion (grep on this audit); guard rule 6 (`legacy_removal_guard.rs:231 core_redeclaration_hits`) forbids re-declaring or aliasing the removed names in the core module | closed |
| T-44-26 | Denial of Service | a v0.10 config document failing to load after the cut | low | mitigate | no `deny_unknown_fields` on `BattalionConfig`; `aegis` is `#[serde(default)]`; test `battalion_config_v0_10_document_deserializes_with_default_aegis` (`mod.rs:1064`) | closed |
| T-44-27 | Repudiation | an unregistered public-surface removal | medium | mitigate | baseline `.project/current-exports.txt` regenerated in commit `9a9b1d3f` (44-10); 44-12 gate `make api-surface` exit 0 "API surface unchanged" (4274 items); §9.2 rows + allowlist in 44-11 | closed |
| T-44-28 | Repudiation | an undocumented breaking change reaching the downstream consumer | medium | mitigate | one §9.2 row per crate\|type pair with the D-27 measured cell (MIGRATION.md `:296-304`); `make check-migration-allowlist` re-run on this audit: "Allowlist is set-equal to the MIGRATION.md §9.2 deliberate-breaking register"; 9.1 rows M-B-06..M-B-10 and §9.4 for stored data | closed |
| T-44-SC | Tampering | the downloaded `cargo-semver-checks` binary (supply chain) | medium | mitigate | 44-11-SUMMARY `:63`: exact CI-pinned v0.50.0 asset from the official `obi1kenobi/cargo-semver-checks` release over TLS into the session scratchpad, invoked by absolute path, never committed or added to a manifest; tarball sha256 recorded; `Cargo.lock` has no commit in the phase window and `semver-checks` appears in manifests only as lint metadata | closed |
| T-44-29 | Denial of Service | the diagnostic cache exhausting disk | low | mitigate | 44-11-SUMMARY `:120`: free space monitored, regenerable `target/semver-checks` cache deleted at 270 MB free, no ENOSPC | closed |
| T-44-30 | Information Disclosure | credential leakage introduced anywhere in the phase (logs, herald lines, error messages) | high | mitigate | four-item manual credential review with file:line evidence in 44-12-SUMMARY `:87-92` (redaction path untouched, every added log macro enumerated, no credential type gained `Debug`/`Serialize`, zero `reqwest::Client`/`redirect::Policy` diff lines); `make security` re-run on this audit: exit 0, advisories/bans/licenses/sources ok; clippy `-D warnings` exit 0 in the 44-12 gate; information-minimisation test `herald_node_error_line_adds_no_provider_detail` present | closed |
| T-44-31 | Tampering | dependency supply chain | low | mitigate | no commit touches `Cargo.lock` since 2026-10-09; `make security` (cargo-audit + cargo-deny) exit 0 on this audit | closed |
| T-44-32 | Repudiation | a change shipping without a record | low | mitigate | CHANGELOG.md entries cite `MIGRATION.md §9.2` / `§9.4` (`:187`, `:212`, `:229`); ADR-0059 Conformance section (`:149-157`) names the proving tests per decision group | closed |
| T-44-33 | Tampering | reintroduction of a removed name, alias or whole-run timeout in a legacy-pattern file | medium | mitigate | `tests/legacy_removal_guard.rs`: six rules over `.rs/.md/.toml/.yaml/.yml`; planted-tree control `scanner_reports_planted_legacy_uses` (`:690`), negative control `scanner_accepts_a_clean_tree` (`:727`), live scan `legacy_surfaces_are_gone_from_the_tree` (`:739`); file-scoped rules assert their targets exist; 44-12 gate: 3 passed | closed |
| T-44-34 | Repudiation | a guard silenced by widening its skip list or allowed look-alikes | low | mitigate | skip list fixed to the declared history set (`legacy_removal_guard.rs:70-74`); first-run scan output recorded in 44-13-SUMMARY | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-44-01 | T-44-08 | A Permanent or Unknown LLM failure (e.g. a 401 from a bad key) no longer trips the breaker, so each such call reaches the provider. Each fails fast at the provider, is cheap and is not retried; the change is visible to operators in MIGRATION.md §9.1 M-B-10. | D-14 (44-CONTEXT), plan 44-03 | 2026-10-10 |
| AR-44-02 | T-44-09 | 44-03 rewrites rustdoc and tests only: no new log line, no credential handling, no HTTP client; error text still flows through the already-redacted `LlmFailure.message`. | plan 44-03 | 2026-10-10 |
| AR-44-03 | T-44-10 | A long Formation has no whole-run bound; each attempt is bounded when `aegis.timeout` is set. A caller needing a whole-run bound wraps the call in `tokio::time::timeout` (documented in `aegis` rustdoc, docs/src and MIGRATION.md §9.1 M-B-06). | D-02 (44-CONTEXT), plan 44-04 | 2026-10-10 |
| AR-44-04 | T-44-13 | `AggregationError` embeds each failure's `NodeError` display: Paladin name, attempt, transience and the already-redacted error text — the same text v0.10 put in its `"{name}: {error}"` strings. Redaction happens upstream before truncation (D-34). | plan 44-05 | 2026-10-10 |
| AR-44-05 | T-44-18 | ChainOfCommand and Grove routed through the Commander have no clock; the `CommanderBuilder` default keeps 300 s per attempt for the patterns that read `aegis`, and `Commander::execute` rustdoc directs callers to wrap for a whole-run bound. Recorded in MIGRATION.md §9.1 M-B-06. | D-02 / D-03 (44-CONTEXT), plan 44-07 | 2026-10-11 |
| AR-44-06 | T-44-21 | CLI runs lose the whole-run 600 s / 300 s bound; the same figure now bounds each Paladin attempt. The CLI rustdoc, YAML template comment and `per_attempt_timeout` doc say so; MIGRATION.md §9.1 M-B-06 names the CLI change. | D-02 (44-CONTEXT), plan 44-08 | 2026-10-11 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-11 | 35 | 35 | 0 | /gsd-secure-phase orchestrator (ASVS L1 grep-depth; auditor subagent skipped under the short-circuit rule: planned register, threats_open 0, L1) |

Unregistered flags: none — every `## Threat Flags` entry in the 13 SUMMARYs maps to a registered threat ID.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-11
