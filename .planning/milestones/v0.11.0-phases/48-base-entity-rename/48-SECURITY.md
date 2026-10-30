---
phase: 48
slug: base-entity-rename
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-10-14
---

# Phase 48 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Every Phase 48 plan (48-01 through 48-09) carried a `<threat_model>` block, so the register below
was authored at plan time and this audit verifies mitigations rather than scanning for new threats
(`register_authored_at_plan_time: true`, ASVS level 1, block threshold `high`). Verified on HEAD
`9ee3ad5b` (phase base `3cac94f`). Evidence was checked on disk, not copied from the SUMMARYs.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| persisted JSON on disk -> `FileCitadel` -> domain types | a snapshot written by an older binary (or by anyone with write access to the state directory) is parsed into `PaladinState` | Citadel snapshot files (`paladin-<uuid>.json`), untrusted bytes |
| persisted JSON (Citadel files, caller-owned stores, queue payloads) -> `Entity<T>` / `ContentItem` / `PromptItem` | documents written by v0.10 (or crafted) are read by v0.11 through the serde read-aliases | entity payloads, old (`node`) and new (`data` / `entity`) key spellings |
| persisted version records -> `EntityVersion<T>` | a version record stored by v0.10 (or crafted) is read by v0.11 (`node_id` / `node_data` aliases) | version history records |
| persisted Citadel snapshots -> unified `PaladinState` | v0.10 snapshots whose `paladin` payload was the citadel placeholder type are read into the real `Paladin` | `PaladinData` / `PaladinStatus` encodings |
| committed fixtures -> repository | captured runtime output becomes tracked content | five pre-rename JSON fixtures |
| crates.io -> local toolchain | `cargo-public-api` and `cargo-semver-checks` installed to reproduce CI gates | executable tooling (supply chain) |
| scripted bulk rewrite -> source tree | perl regexes decided which `Node` tokens (Stage A, 120 files) and `.node` accesses (Stage B, 108 files) are entity-sense | source integrity, graph-vocabulary freeze |
| repository contents -> guard verdict | `tests/entity_rename_guard.rs` decides whether a push re-introduces an old name or moves a graph definition | repository scan, symlinks, temp trees |
| decision record -> downstream consumer and operators | ADR-0060, `MIGRATION.md` and the semver register tell the downstream system and operators what is compatible and what is not | migration guidance, licence for the X-03 break |
| documentation -> developers and AI agents | `CLAUDE.md` and `.github/copilot-instructions.md` are loaded into every agent session and steer generated code | agent instructions |
| phase evidence -> verifier and reviewers | the SUMMARYs and ADR are what `/gsd-verify-work` and reviewers rely on | gate results, coverage claims, requirement traceability |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-48-01 | Information Disclosure | the five committed fixtures | low | mitigate | Fixtures built from `Default` impls and literal strings; the only credential-shaped string across all five is the field name `max_tokens` in `prompt.json`; gitleaks pre-commit hook passed in CI run 38491601823 | closed |
| T-48-02 | Tampering | fixture provenance | medium | mitigate | Captured by plan 48-01 from the unmodified tree (commit 28c9bc5 is the only commit touching `tests/fixtures/pre_rename/`); every fixture carries only v0.10 keys; harness compares against the unedited bytes | closed |
| T-48-03 | Tampering | `FileCitadel::load_paladin` parsing an attacker-influenced snapshot | low | accept | Parsing path unchanged: derived serde only, no custom `Deserialize`, no `deny_unknown_fields` change; both-keys and no-key cases rejected (see T-48-13). Accepted risk AR-48-01 | closed |
| T-48-04 | Repudiation | a removal disputed as unlicensed, or licensed by the wrong record | medium | mitigate | ADR-0060 (commit f0535858, plan 48-02) is the first `(48-0` commit on the branch, before any removal; its Decision (a) scopes the X-03 supersession to Phase 48 only and names ADR-0051/0059 as shape, not licence | closed |
| T-48-05 | Information Disclosure (misleading assurance) | serde read-aliases read as an API compatibility promise | medium | mitigate | ADR-0060 Decision (a) states the rule as `no API shim` with the data-only carve-out; `MIGRATION.md` 9.1 M-B-12 and 9.4 state the downgrade asymmetry; `Entity<T>` rustdoc `# Serialization` repeats it | closed |
| T-48-06 | Repudiation | vocabulary rule claimed as total while entity-sense identifiers survive in the retained items | low | mitigate | ADR-0060 "Bounded exception to the vocabulary rule" names `Field.nid`, `FieldVersion*.node_id` and `get_node_field_versions`; the review todo carries the same list; the guard does not ban them | closed |
| T-48-07 | Tampering | a version record carrying both key spellings | low | mitigate | serde rejects `duplicate field`; pinned by `entity_version_rejects_both_key_spellings` (`entity_version_service.rs:635`); no custom deserializer | closed |
| T-48-08 | Tampering (integrity) | a graph-sense `node_id` renamed with the entity sense | medium | mitigate | 48-03's code commit 56213e47 touched only the planned file set; graph-sense `node_id` (1,151 occurrences) untouched; verifier confirmed `node_error.rs`, `node_cache.rs` and the schema byte-identical to the phase base | closed |
| T-48-09 | Repudiation | retained base items changed beyond what D-05 allows | low | mitigate | 48-03 changed only the `version_service::` import lines in the two retained services; guard rule (d) leaves their paths unbanned; `base/mod.rs` records the retention reason | closed |
| T-48-SC (48-03) | Tampering (supply chain) | `cargo install cargo-public-api` | high | mitigate | Installed pinned to 0.52.0 with `--locked` (the version the plan named; `cargo-public-api 0.52.0` on this host); into `~/.cargo/bin` only; `Cargo.lock` untouched by the phase (byte-identical to base at closeout). Pre-existing residual outside this phase: CI's own `Install cargo-public-api` step in `ci.yml` is unpinned | closed |
| T-48-10 | Tampering (integrity) | citadel unification changing what a v0.10 snapshot deserializes to | medium | mitigate | 48-01's compat test and FileCitadel tracer unedited and green; `v0_10_citadel_statuses_load_into_the_unified_status` (`entity_serde_compat.rs:168`) covers every v0.10 status | closed |
| T-48-11 | Denial of Service (availability) | a v0.11 snapshot in `Reasoning` status is unreadable by a v0.10 binary | low | accept | Inherent to D-10 (owner decision); pinned by the status test and documented for operators in `MIGRATION.md` 9.4. Accepted risk AR-48-02 | closed |
| T-48-12 | Tampering (integrity) | Stage A regex retargeting a graph-sense `Node` token | medium | mitigate | All 120 rewritten files classified against a re-run of the script; engine/eval/storage diffs reviewed line by line; `legacy_removal_guard` 3/3 and `entity_rename_guard` 5/5 green; graph definitions pinned by guard rule (c) | closed |
| T-48-13 | Tampering | a document supplying both the old and the new key | low | mitigate | `pre_rename_json_carrying_both_payload_keys_is_rejected`, `content_item_carrying_both_entity_keys_is_rejected`, `json_without_a_payload_key_is_rejected` (`entity_serde_compat.rs:237-268`); derived serde only on `wrapper.rs`, `content.rs`, `prompt.rs` | closed |
| T-48-14 | Tampering (integrity) | a graph-sense `.node` silently retargeted | medium | mitigate | 18 graph-sense `.node` lines diffed byte for byte (exit 0); residual `.node` grep empty; guard payload-access rule passes on the real tree | closed |
| T-48-15 | Denial of Service (availability) | v0.10 reader meeting v0.11-written JSON during a rolling upgrade or downgrade | medium | mitigate | `Entity<T>` rustdoc `# Serialization` (`wrapper.rs:26`); `MIGRATION.md` 9.1 M-B-12 and 9.4 (readers before writers, back up before upgrade, restore-only downgrade); `new_writes_never_emit_the_old_keys` pins the write side | closed |
| T-48-16 | Tampering (integrity of agent instructions) | an unintended edit to `CLAUDE.md` | low | mitigate | Phase diff on `CLAUDE.md` is 3 insertions, 1 deletion (within the 4-line cap); Treasurer vocabulary rows unchanged | closed |
| T-48-17 | Information Disclosure (misleading docs) | docs presenting paths or fields that do not exist | low | mitigate | Domain-model page rewritten from `wrapper.rs`; zero `Node<T>` mentions remain in `domain-model.md`, `README.md`, `crates/paladin-core/README.md`; `make doc-check` and `make check-doc-examples` green at closeout | closed |
| T-48-18 | Repudiation | a registered lint or count guessed rather than measured | medium | mitigate | D-27 diagnostic run on the final tree with cargo-semver-checks 0.50.0 against 0.10.1 (11 lints on `paladin-ai-core`, 1 on `paladin-ai`); manifests restored and proven with `cmp`; `make check-migration-allowlist` set-equal | closed |
| T-48-19 | Tampering (gate weakening) | crate-wide allow lines suppress a future accidental break of the same lint class | low | accept | Phase 25/44 shape; `module_missing` and `trait_missing` allow lines in `crates/paladin-core/Cargo.toml:118-119` are each paired with named 9.2 rows and allowlist entries; CI checks set equality. Accepted risk AR-48-03 | closed |
| T-48-20 | Denial of Service (availability) | operators upgrade writers before readers or downgrade without a backup | medium | mitigate | `MIGRATION.md` 9.1 M-B-12 and 9.4 state the order, the backup step and the restore-only downgrade, including the `Reasoning` status case | closed |
| T-48-SC (48-07) | Tampering (supply chain) | `cargo install cargo-semver-checks` | high | mitigate | Installed pinned with `--locked` (`cargo-semver-checks 0.50.0` on this host); into `~/.cargo/bin` only; `Cargo.lock` untouched by the phase | closed |
| T-48-21 | Repudiation (gate erosion) | skip set or look-alike list widened to make the real tree pass | medium | mitigate | 48-08 Task 2 diffed the skip functions and `HISTORY_FILES` against Task 1's commit (identical); the real-tree scan found zero hits on first run with no source edits; the 48-09 docs-less carve-out is keyed only on `docs/` absence and backed by a positive control | closed |
| T-48-22 | Denial of Service (flaky gate) | control tests colliding under parallel libtest threads | low | mitigate | Per-test temporary trees named from `std::process::id()` plus an atomic counter (`entity_rename_guard.rs:566`), removed on drop; real-tree scan is read-only | closed |
| T-48-23 | Tampering | the scan following a symlink out of the repository | low | mitigate | Walker uses `DirEntry::file_type()` (`entity_rename_guard.rs:483`) and does not follow symlinks; dot-directories and `target` skipped | closed |
| T-48-24 | Repudiation | a gate or coverage figure claimed without being run or measured | medium | mitigate | 48-09 SUMMARY lists one line per gate with its exit code; `make security` and the coverage percentage were recorded as NOT run locally rather than claimed (`.planning/WINDOWS.md` rows 68-69); `make security` since passed locally (exit 0, 2026-10-14) and is recorded on UAT item 2 | closed |
| T-48-25 | Information Disclosure | a credential-handling regression slipping through the rename | low | mitigate | Manual credential-handling review over the whole phase diff: every hit is a mechanical rename (`user.data.password_hash`, a `Field` doc comment, `max_tokens`); no log statement, redaction, Debug derive, redirect policy or HTTP client changed | closed |
| T-48-26 | Repudiation | requirement IDs or traceability silently altered while fixing wording | low | mitigate | REQUIREMENTS.md still has exactly five `ENTITY-0x` requirement lines and five traceability rows; corrections are dated text-only edits | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-48-01 | T-48-03 | The snapshot parsing path is unchanged by the phase (derived serde, typed fields); a crafted snapshot in the state directory was already in the threat surface before Phase 48 and the new alias edge cases are pinned by tests. | plan 48-01 disposition; confirmed at this audit | 2026-10-14 |
| AR-48-02 | T-48-11 | A v0.11 snapshot captured mid-`Reasoning` cannot be read by v0.10. The asymmetry follows from the owner's D-10 decision to unify the placeholder status enum with the real one; operators are told to restore from backup to downgrade (`MIGRATION.md` 9.4). | plan 48-04 disposition; owner decision D-10 (2026-10-13) | 2026-10-14 |
| AR-48-03 | T-48-19 | The two new crate-wide `allow` lines (`module_missing`, `trait_missing`) also hide a future accidental break of the same class. This is the established Phase 25/44 register shape; each line is paired with a named 9.2 row and allowlist entry, and the pairing is checked by CI. Reviewed per phase. | plan 48-07 disposition; confirmed at this audit | 2026-10-14 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-14 | 27 | 27 | 0 | /gsd-secure-phase 48 orchestrator (grep-level verification, ASVS L1, HEAD 9ee3ad5b) |

Notes from this audit:

- Supply-chain tooling: the local installs matched the versions the plans named (`cargo-public-api` 0.52.0, `cargo-semver-checks` 0.50.0) and were made with `--locked`. In `ci.yml`, `cargo-semver-checks` is pinned at 0.50.0 via `taiki-e/install-action`, but the `Install cargo-public-api` step runs an unpinned `cargo install`; that predates Phase 48 and is noted here for a later hardening pass, not counted as a Phase 48 threat.
- `make security` could not run during execution (tools absent) and was recorded as unrun rather than claimed; it has since passed locally (cargo-audit 0.22.2, cargo-deny 0.20.2: advisories, bans, licenses and sources ok). The red CI Security Audit and License jobs predate Phase 48 (five RustSec advisories on `quinn-proto`, fixed on this branch in 1ff46a35, plus the unmaintained `unsafe-libyaml` advisory governed in 12c6a5fa). CI confirmation on the current head is tracked by `48-UAT.md` item 2.
- Residual, not a gap: the retained base items keep entity-sense identifiers (`Field.nid`, `FieldVersion*.node_id`, `get_node_field_versions`) by owner decision D-05; ADR-0060 records them as the bounded exception and the content-system review todo owns their fate.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-14
