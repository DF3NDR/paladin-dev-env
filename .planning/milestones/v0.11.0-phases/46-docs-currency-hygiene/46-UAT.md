---
status: complete
phase: 46-docs-currency-hygiene
source: 46-01-SUMMARY.md, 46-02-SUMMARY.md, 46-03-SUMMARY.md, 46-04-SUMMARY.md, 46-05-SUMMARY.md, 46-06-SUMMARY.md, 46-07-SUMMARY.md, 46-08-SUMMARY.md, 46-09-SUMMARY.md, 46-10-SUMMARY.md, 46-11-SUMMARY.md, 46-12-SUMMARY.md, 46-13-SUMMARY.md, 46-14-SUMMARY.md
started: 2026-10-24T17:16:32Z
updated: 2026-10-24T19:44:09Z
---

## Current Test

[testing complete]

## Tests

### 1. Release docs state twelve crates and dated 0.10.x changelog headings
expected: Open docs/src/appendix/release-automation.md, docs/src/appendix/release-recovery.md and docs/src/contributing/development-setup.md: each states twelve publishable crates (not eleven) and paladin-eval appears in the Trusted Publishing and Credential History rows with its evidence class ("proven by publish", run 35659477719). In CHANGELOG.md and each crate CHANGELOG, the [0.10.0] and [0.10.1] heading dates equal the tag commit dates shown by `git log -1 --format=%cs v0.10.0` and `v0.10.1`.
result: pass

### 2. Eight archived VALIDATION files are out of draft
expected: The VALIDATION.md files for Phases 22, 24, 28, 29, 30, 34, 36 and 36.1 each read `status: validated` and `nyquist_compliant: true` in frontmatter, carry a populated per-task map (no `| TBD |` cells) and a `## Gaps` section reading "None." Phase 28's file cites its two closed UAT sign-offs as the basis for the D-15 promotion.
result: pass

### 3. Dependency allowlist guard catches drift
expected: `make check-dependency-allowlist` exits 0 on HEAD (paladin-core 14 keys, paladin-ports 12 including `log`, matching the dated 2026-10-24 amendment in ADR-0015). Temporarily adding a surplus dependency line to crates/paladin-ports/Cargo.toml makes it exit non-zero naming the key; reverting restores exit 0. `make test-shell-guards` passes the 15-assertion harness and `make check-gates` lists check-dependency-allowlist as a member and exits 0.
result: pass

### 4. Treasurer concept page and configuration reference
expected: docs/src/SUMMARY.md lists a Treasurer page under Architecture right after Commissary, and docs/src/architecture/treasurer.md covers the ledger, allowance windows, mid-run halt, pricing and a "Rate pacing (Cadence)" section. docs/src/user-guide/configuration.md has one `## Treasurer` block with a key table covering all APP_TREASURER_* overrides, a "Treasurer pricing" section and the warn rule example (79/80/81 percent) plus the allowance period range 1m to 366d. config.example.yml has a commented `cadence:` block with defaults and the Redis url_env form. docs/src/appendix/adr-index.md row 0050 says built in v0.11.0 and rows 0052-0054 and 0056-0062 are present.
result: pass

### 5. Stored v0.10 configs with removed keys are refused with a guiding error
expected: Deserializing a BattalionConfig document that still carries `timeout_seconds`, `retry_policy` or `error_strategy` fails with one error that names each key present, its aegis.* replacement and MIGRATION row M-B-06 (never a value). A ConclaveConfig with `timeout_seconds` is refused the same way, pointing at battalion_config.aegis.timeout.run_timeout. A v0.11 document, or one with an unrelated unknown key, still loads, and the CLI's own conclave YAML `timeout_seconds` still works. `cargo test -p paladin-battalion legacy_keys` passes.
result: pass

### 6. Maneuver warns once when ignored aegis policies are set
expected: Running a Commander with the Maneuver strategy and a BattalionConfig whose `aegis.on_error` or `aegis.retry` is set logs exactly one warn per execution reading "Maneuver {name} ignores aegis.on_error and aegis.retry: Maneuver reads only aegis.timeout.run_timeout, as one whole-flow clock". With neither field set no warning appears. The BattalionConfig.aegis rustdoc has a Maneuver bullet, and Commander, CommanderBuilder and build() rustdoc state the Commander adds no bound of its own. `cargo test -p paladin-battalion maneuver_ignored` passes.
result: pass

### 7. MIGRATION.md is a two-part upgrade guide
expected: MIGRATION.md opens with "# Upgrading Paladin", then section 10 (v0.10.1 to v0.11.0) with headings 10.1 through 10.8, including a freeze note dated 2026-10-24 against commit e28e43b5, an empty-deprecations statement in 10.7 and a six-step operator checklist in 10.8. Section 9 (v0.9.0 to v0.10.0) sits below with one pointer line under each 9.x heading and its original text otherwise unchanged. v0.11 rows cite 10.x and v0.10 rows cite 9.x. `grep -c TBD MIGRATION.md` prints 0 and `make check-migration-allowlist` exits 0 reading both the 9.2 and 10.2 registers.
result: pass

### 8. Changelogs and allowlist cite section 10
expected: Every v0.11 line in CHANGELOG.md [Unreleased] and in the battalion, core, llm, ports, web and herald crate [Unreleased] sections cites MIGRATION.md section 10.x (no live 9.x citation left for a v0.11 row; historical 9.x citations below [Unreleased] remain). CHANGELOG.md [Unreleased] has a `### Documentation` subsection with four bullets and a Changed bullet for the dependency-allowlist gate. The .cargo/semver-checks-allowlist.toml header names both section 9.2 and 10.2, and the Phase 38-51 comment blocks cite 10.2.
result: pass

### 9. Book Upgrading page leads with v0.11.0 and the DeepSeek note is present
expected: docs/src/api-reference/upgrading.md opens with "Upgrading from v0.10.1 to v0.11.0" holding a Behavioral changes table (M-B-05 to M-B-22, each once), a Library consumers index (DR-1 to DR-26 grouped by topic) and a six-step Operator checklist, linking MIGRATION.md section 10 anchors; the v0.9.x to v0.10.0 content follows below unchanged. docs/src/api-reference/migration-guide.md carries a pointer line to the v0.11.0 record. docs/src/user-guide/switching-providers.md has a dated (2026-10-24), attributed note that DeepSeek lists deepseek-flash and not deepseek-v4-flash, and cites section 10.8 for DR-18 and DR-19.
result: pass

### 10. Source comments cite the moved section 10 rows
expected: tests/consumer_surface_muster.rs's red-muster rule names the section 10.2 row and the section 10.8 library-consumer table; the two D-11 exhaustive-match pin comments cite 10.2; crates/paladin-llm/src/registry/seeded.rs carries three "MIGRATION section 10.8" comments; conclave_execution_service.rs cites "10.1 rows". `grep -rnE 'MIGRATION(\.md)?.?(§|section )?9\.[0-9]' crates tests docs/src` returns only intentional v0.10 history references. `make check-consumer-surface` passes.
result: pass

### 11. Stockpile::render pinned as fn(&Stockpile) -> String
expected: Stockpile::render pinned as fn(&Stockpile) -> String
result: pass
source: automated
coverage_id: D1

### 12. BattalionResult counters read pin
expected: BattalionResult paladin_success_count / paladin_failure_count (usize) and per_paladin_times (HashMap<String, u64>) read pin
result: pass
source: automated
coverage_id: D2

### 13. prelude glob import resolves
expected: use paladin::prelude::*; glob import site resolving PaladinBuilder, LlmPort, BattalionResult
result: pass
source: automated
coverage_id: D3

### 14. llm_port::ModelProfile re-export is the same type
expected: ModelProfile through paladin_ports::output::llm_port re-export is the same type as output::model_profile::ModelProfile
result: pass
source: automated
coverage_id: D4

## Summary

total: 14
passed: 14
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps

[none yet]
