---
phase: 55
slug: correctness-fixes-test-hygiene-lint-bar
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-11-02
verified_head: 6690a56d8
---

# Phase 55 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time. All thirteen PLAN files (55-01 through 55-13) carry a
`<threat_model>` block (35 threats: T-55-01 through T-55-33 plus T-55-SC). Every SUMMARY's
`## Threat Flags` says "None" or maps to a registered id (55-06-SUMMARY has no heading; its line
119 says no new threat surface). The phase adds no network endpoint, auth path or trust boundary:
it replaces panicking stubs with typed refusals, adds a check-only stub-panic gate, turns an
unpriced-model failure into a halt, adds a boot warning, makes two tests hermetic and fixes lint
and rustdoc.

Verification ran through the gsd-security-auditor at HEAD `6690a56d8`, whose Rust code is the
verified head `e332ecc3d` plus the comment-only commit `84fb828f9`. The auditor re-ran the
stub-panic gate (624 files, 19 test-only, 0 findings) and its harness (88 assertions), and took
test evidence from runs recorded at code-identical SHAs (`55-CORRECTNESS-EVIDENCE.md`,
`55-REVIEW-FIX.md`, `55-VERIFICATION.md`). It re-did the manual credential-handling review of
`.github/instructions/security.instructions.md` at HEAD, because the one recorded in the evidence
file predates the WR-01 review fix (see T-55-21).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| library caller -> storage adapter | `ContentListRepository` callers on the sqlite / mysql backends | method name and backend in a fixed refusal; no backend error text |
| run worker -> caller, SSE, webhook receiver | the halt object of an agent-kind run whose model lost its price | `{"reason":"model_unpriced","model":"<name>"}`: model name only, no tenant, key or figure |
| assistant store -> boot log | stored agent models scanned for a missing price at boot | model name (`escape_debug`-escaped) and a slug `AssistantId` position; no error text |
| unit test -> network | the two `build_run_api_persists_*` trace tests | none: a scripted in-process `CannedAnswerPaladinPort` |
| contributor edit -> CI gate | `scripts/check-no-stub-panics.sh` in `make check-gates` and `ci.yml` | file:line findings only; no exemption input |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation (evidence at HEAD) | Status |
|-----------|----------|-----------|----------|-------------|-------------------------------|--------|
| T-55-01 | Denial of service | `ContentListRepository` on sqlite / mysql panicking | high | mitigate | Refusal helpers at `sqlite_content_repository.rs:650-654` and `mysql_content_repository.rs:627-631`; 10 `Err(unfinished_content_list(..))` bodies each, the only two impls of the 10-method trait; 0 stubs outside tests; `content_list_repository_refuses_every_method_on_sqlite` (:839) / `_on_mysql` (:809), watched failing first (55-01-SUMMARY:124-125) | closed |
| T-55-02 | Repudiation (false assurance) | the stub-panic gate passing vacuously or missing a site | high | mitigate | `NO_PACKAGES` / `ZERO_FILES` / `UNREADABLE` at `check-no-stub-panics.sh:921-936`; unreadable files collected, not skipped (:819-827); fails closed on an interpreter crash; harness fixtures for each failure, `cfg(not(test))`, out-of-line test modules and blanking; 26-line red run recorded (evidence:47-85); wired at `Makefile:228-230,272`, `ci.yml:188-189`; harness green at HEAD after the WR-03 rewrites | closed |
| T-55-03 | Information disclosure | refusal message carrying backend text, a URL or a key | low | mitigate | One fixed template per file (:651-653 / :628-630), method and backend literal only; `assert_refused` compares the exact string | closed |
| T-55-04 | Tampering | a later edit silencing the gate with an exemption | medium | mitigate | The only environment read is the test seam `STUB_PANICS_ROOT` (:153), set by neither `Makefile` nor `.github/`; header (:134-136) and failure text (:971-972) state zero exemptions; 50+ fire cases in the harness | closed |
| T-55-SC | Tampering | package installs | low | accept | No manifest or lockfile changed since phase start; see AR-55-01 | closed |
| T-55-05 | Information disclosure | `HaltReason::ModelUnpriced` leaking a tenant, key or figures | high | mitigate | Variant carries `model` only (`allowance.rs:256-261`), produced from `model.to_string()` (`treasurer/derive.rs:127`); one `wire_json` builder (:382-401) feeds the run row (`run_controller.rs:475`), SSE (`events.rs:255,555`) and webhook (`worker.rs:493-495`); exact-string tests at `allowance.rs:1434,1448,1456` | closed |
| T-55-06 | Elevation of privilege / Repudiation | an unpriced model at dispatch calling the LLM unmetered | high | mitigate | `worker.rs:1872-1873` maps to `Halt`; the `Halt` arm (:1680-1691) returns before the LLM; `run_agent` is the only agent-kind dispatch path; `agent_kind_run_whose_model_became_unpriced_halts_without_calling_the_llm` asserts `call_count() == 0` (`worker_tests.rs:2983`) | closed |
| T-55-07 | Tampering | OpenAPI document drifting from the wire form | medium | mitigate | `openapi_matches_committed_baseline` (`paladin-web/src/openapi.rs:264`); `openapi.json:3984` matches the `run_controller.rs:444-453` rustdoc; `make openapi` | closed |
| T-55-08 | Repudiation | rollback to a build without the variant misreading rows | low | accept | Unreadable `halt_reason` reads as `None` with the run id logged (`run/sqlite.rs:328-332`, `run/postgres.rs:311-315`; test `an_unreadable_stored_halt_reason_reads_as_none_not_an_error`); `MIGRATION.md` M-B-31; see AR-55-02 | closed |
| T-55-09 | Repudiation (false red / green) | cancel test racing dispatch registration | low | mitigate | `#[cfg(test)] pub(crate) is_registered` (`cancel.rs:221-224`) polled inside a 30 s timeout (`cancel_tests.rs:360-362`); fixed sleep gone; 20/20 runs recorded (evidence:116-125), 10/10 in verification | closed |
| T-55-10 | Repudiation | commander tests reading as missing error-path coverage | low | mitigate | Pointers at `commander.rs:3805,3836,3873,3921`, Phalanx gap stated at :3922-3923; targets exist at `tests/integration/commander_error_paths_test.rs:53,92,153,218` | closed |
| T-55-11 | Tampering | a lint silenced by an attribute | medium | mitigate | 0 added `#[allow(` / `#[expect(` in the phase's `.rs` diff; cfg gates at `cadence.rs:361,374`, `conformance.rs:278`; 13-combination clippy matrix all 0 (evidence:141-160) | closed |
| T-55-12 | Repudiation | a seed flag changed without measured evidence | medium | mitigate | `seeded.rs:204-210` keeps `.with_reasoning(false)` citing the 50.1 probe and 51-EFFORT-PROBE records; pinned by `anthropic_opus_4_8_seed_matches_the_transcript` (`registry/mod.rs:1733`) | closed |
| T-55-13 | Tampering (data integrity) | fabricated ids presented as real | medium | mitigate | `EntityNotFound` at `field_version_service.rs:507-510` (purge handles it at :558-563); `Uuid::nil()` sentinel at `collection_versioning_service.rs:432,434`; tests at :631, :649, :1168, :1193, :1206 | closed |
| T-55-14 | Tampering | base-layer `pub` item renamed or removed | medium | mitigate | 0 removed `pub` lines under `paladin-core/src/base`; only the two fence bodies changed outside comments and tests; `entity_rename_guard` 8 passed (re-run in validation); `api-surface` unchanged; semver 12/12 green | closed |
| T-55-15 | Repudiation | `ingest_content` reporting success while storing nothing | medium | mitigate | `INGEST_CONTENT_UNFINISHED` (`file_content_fetcher.rs:16`) returned after validation (:195); `test_ingest_content_refuses_valid_content_as_unfinished` (:313) pins the text | closed |
| T-55-16 | Tampering (prompt integrity) | fabricated "expected output" steering subtask calls | low | mitigate | `planning_service.rs:418-423` sets `String::new()`; the prompt omits a blank line (:508); tests at :729, :748, :778 | closed |
| T-55-17 | Repudiation | FIXES-04 dropped or recorded closed when re-homed | medium | mitigate | `REQUIREMENTS.md:165-168` dated note (left unticked after `phase.complete`), v2 entry at :280+, :356 `Re-homed (next minor)`; `ROADMAP.md:445`; inventory B5; `deferred-items.md:7-9` | closed |
| T-55-18 | Tampering | eval schema golden drifting beyond descriptions | low | mitigate | 4 changed golden lines since phase start, all `"description":`; `schema_matches_golden` (`paladin-eval/tests/schema_golden.rs:21`) | closed |
| T-55-19 | Repudiation (false assurance) | rustdoc suppression re-added or doc text deleted | medium | mitigate | 0 `allow`/`expect(rustdoc` in `crates/*/src` and `src`; no rustdoc lint table in `Cargo.toml`; `#![warn(rustdoc::broken_intra_doc_links)]` kept at `paladin-ports/src/lib.rs:51`; both bars recorded (55-08-SUMMARY:86) | closed |
| T-55-20 | Tampering (egress) | trace tests sending a model call off the machine | medium | mitigate | `CannedAnswerPaladinPort` (`run_api_wiring.rs:3831-3860`) injected at :3924; `env -u` run (evidence:102-114) and `unshare -rn` run (55-VERIFICATION) both pass | closed |
| T-55-21 | Information disclosure | boot warning naming a tenant, key name or value | high | mitigate | Line built from the model alone (`run_api_wiring.rs:805-816`), test at :2021-2049 asserts no `tenant`, `api_key` or `sk-`; the WR-01 walk `collect_agent_version_models` (:904-936) logs nothing and drops error text; the stop line (:941-952) carries only a slug `AssistantId` or `at start`; the phase's only log lines are :959 and :962. Manual credential review re-done at HEAD: no new HTTP client or redirect, no credential type gained `Debug` | closed |
| T-55-22 | Denial of service | boot blocked by the warning or an unreadable store | medium | mitigate | Reporter returns `()` (:957-964); the scan returns a value, never a `Result` (:852-892), and stops at a read error (:866-872, :875-880); outer followed-cursor and empty-page guard (:885), inner `next <= prev` / empty-page guard (:930); called only in the Treasurer branch with no `?` (:1202-1243); tests at :2316, :2359, :2411, :2441, :2482 and boot-still-succeeds at :2513. Residual: no time or page cap (R-55-01) | closed |
| T-55-23 | Elevation of privilege | production code injecting an arbitrary engine port | medium | mitigate | `pub(crate)` seam (:1017, :1029); public `build_run_api` (:616) and `build_run_api_with_cadence` (:984) pass `None` (:1002); `Some(..)` built only at :2542 and :3924 inside `mod tests` | closed |
| T-55-24 | Tampering (log forging) | stored model string forging log lines | low | mitigate | `model.escape_debug()` at :814, unchanged by WR-01; newline / CR / ESC case at :2043-2048 | closed |
| T-55-25 | Repudiation (false assurance) | suppression fixed for one rustdoc bar only | medium | mitigate | 0 suppressions in llm, storage, notifications; `8ea9f1357`, `d53d4233c`, `23b6426d8` each remove one attribute only; both bars recorded (55-10-SUMMARY:85) | closed |
| T-55-26 | Repudiation (false assurance) | report or ADR claiming an unmeasured green | medium | mitigate | `doc-coverage-report.md:3-9` cites `14ee125e7` (exists), 0 percentage figures; bars re-run at the phase head (evidence:170-177) | closed |
| T-55-27 | Repudiation | ADR-0033 decided text edited instead of amended | low | mitigate | 0 removed lines in `0033-cargo-doc-warning-bar.md` since phase start; dated amendment block | closed |
| T-55-28 | Repudiation | behaviour change unrecorded or frozen section edited | medium | mitigate | `MIGRATION.md` additions only, in §11.1/§11.6/§11.8, before §10; CHANGELOG additions only above `## [0.11.0]`; `check-changelogs` and `check-migration-allowlist` pass (re-run in validation) | closed |
| T-55-29 | Repudiation | base-layer review decisions the code does not support | low | mitigate | Every test name in `55-BASE-LAYER-REVIEW.md` resolves to an existing `fn`; spot-checked lines match HEAD | closed |
| T-55-30 | Repudiation (false assurance) | a gate recorded as passing that was not run | medium | mitigate | Gate rows carry the head SHA (evidence:234-251); coverage and `actionlint` labelled CI-only; verification re-ran the affected bars at `e332ecc3d` after all review fixes | closed |
| T-55-31 | Repudiation | WINDOWS rows 67 / 73 closed without proof | medium | mitigate | `WINDOWS.md:84` row 67 `waived` naming `unpriced_reachable_models_table`; `:90` row 73 `fixed` naming both hermetic tests and the seam; counters 13/40/34/87 consistent | closed |
| T-55-32 | Tampering (semver integrity) | a red semver line made green by registering a break | high | mitigate | `.cargo/semver-checks-allowlist.toml` unchanged since phase start; `MIGRATION.md` §11.2 byte-identical; no manifest changed; semver 12/12, 0 failures (evidence:213-226) | closed |
| T-55-33 | Information disclosure | a credential captured into the evidence | medium | mitigate | 0 `sk-[A-Za-z0-9_-]{20,}` hits in the phase directory; no sandbox credential value in the phase directory, `docs/`, `MIGRATION.md` or `CHANGELOG.md` (compared by value, not printed) | closed |

*Status: open · closed*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-55-01 | T-55-SC | The phase installs and adds no package: `git diff` of `Cargo.toml`, `Cargo.lock` and `crates/*/Cargo.toml` since phase start is empty, so there is no supply-chain surface to verify. | plan 55-01 | 2026-11-02 |
| AR-55-02 | T-55-08 | A rollback to a pre-variant build reads a stored `model_unpriced` halt reason as `None` and logs the run id rather than erroring (`run/sqlite.rs:328-332`, `run/postgres.rs:311-315`, pinned by test); the change is recorded as `MIGRATION.md` M-B-31. | plan 55-02 | 2026-11-02 |

*Accepted risks do not resurface in future audit runs.*

---

## Review-Surfaced Residuals (not in the plan register)

None reopens a register threat or adds a trust boundary; listed so the record is complete. The
gate above does not count them.

| Ref | Relates to | Summary | Where it stands |
|-----|------------|---------|-----------------|
| R-55-01 (55-REVIEW IN-04) | T-55-22 | The boot pricing scan has no time or page cap. Since WR-01 it reads every version of every assistant, so a store read that hangs (rather than errors) would hold boot, and a store that hands out fresh cursors forever is bounded only by its own size or `u32` versions. | Advisory hardening; medium, below the block threshold. |
| R-55-02 | T-55-21, T-55-24 | The stop line prints `AssistantId` through `Display` unescaped. Safe because `AssistantId::new` enforces `[a-z0-9][a-z0-9_-]{0,63}` (`assistant.rs:101-120`); `#[serde(transparent)]` deserialisation skips that check, so only a corrupted store row could carry other bytes. | Advisory. |
| R-55-03 | T-55-21 | The credential review recorded in `55-CORRECTNESS-EVIDENCE.md:266-267` predates WR-01 (old stop-line text and line numbers). Re-done at HEAD by this audit with the line numbers above. | Superseded by this record. |
| R-55-04 | T-55-05 | The `model_unpriced` herald line does not escape the model name (`allowance.rs:355-356`). | Already listed under Known residuals in the evidence file. |
| R-55-05 | T-55-02, T-55-04 | The stub-panic gate is lexical; its residual limits (five macro-token shapes, `include!(concat!(..))`, proc macros) are named in its header (`check-no-stub-panics.sh:115-132`). | Documented; 55-VERIFICATION judged them hardening, not a gap. |
| R-55-06 | T-55-01 | No CI job builds `paladin-storage --features mysql`, so the mysql refusal test runs only locally (passed in validation 2026-11-02). | Advisory; carried in 55-VERIFICATION. |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-02 | 35 | 35 | 0 | /gsd-secure-phase 55 (gsd-security-auditor at `6690a56d8`, ASVS L1, block_on high; verdict SECURED) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-02
