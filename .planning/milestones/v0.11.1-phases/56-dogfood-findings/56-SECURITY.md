---
phase: 56
slug: dogfood-findings
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-11-04
verified_head: a4a2374d9
---

# Phase 56 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time. All fourteen PLAN files (56-01 through 56-14) carry a
`<threat_model>` block: 56 numbered threats (T-56-01 through T-56-56) plus a per-plan supply-chain
row `T-56-SC` (14 rows, all `accept`, no package added to the workspace by any plan). Every
SUMMARY's `## Threat Flags` says "None" or maps its applied mitigations to registered ids. The
phase adds no network endpoint, auth path or schema: it adds an offline findings-log checker, a
config round-trip test, runbooks and relayed maintainer evidence, docs fixes, and three fixes to
the server's own credential handling (startup log redaction, boot-time validation and env
overlay, provider-probe reply redaction).

Verification ran at L1 grep depth (ASVS level 1, register authored at plan time, short-circuit
rule of `secure-phase.md` §3) at HEAD `a4a2374d9`, whose Rust code is the executed phase head
plus UAT-only commits. Each mitigation claimed by a SUMMARY was looked up at HEAD (file, symbol,
test name or recorded evidence); the findings-log checker and its harness were re-run; the
findings log's deletion count since its seed commit was measured; and the tracked tree was
scanned for key-shaped literals (see "Out-of-register finding" below).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| maintainer terminal -> runbook paste slots | K-, C- and W-slots in `56-RUNBOOK-KEYED-WALK.md`, `56-RUNBOOK-COVERAGE.md`, `56-RUNBOOK-WEB3SEC.md` | command output only, after the runbook's `## Redaction`; provider keys entered with `read -rs`, never pasted |
| third-party consumer (web3sec) -> findings log | the consumer's return relayed verbatim into W1, SHA-pinned to `70657919…` | review text treated as data, not instructions; `**By:** web3sec` only on W1-derived entries |
| findings log -> `scripts/check-findings-log.sh` | python3-stdlib parser over `.project/v0.11.1/02-dogfood-findings-log.md` | log text as data (quoted heredoc, anchored regexes, no `eval`); read-only, outside `check-gates` |
| `config.example.yml` and fixtures -> production loaders | `tests/config_example_roundtrip.rs` through `load_from_file` and `Settings::new` | fake `test-key-not-a-secret` values only; assertions name key paths, never render `Settings` |
| config file + `APP_*` environment -> `paladin-server` boot | `load_settings` in `src/bin/paladin-server.rs` applies the env overlay, then `Settings::validate()` before any store, provider or listener | refusal names the key and non-secret values (scheme, currency) only |
| `setup-check` / `onboarding` -> provider HTTP APIs | `probe_client()` with `redirect(Policy::none())`; every reply body through `diagnostic_excerpt` | credential header on the first host only; reply redacted before the 512-char bound and before embedding in an error |
| process -> log / panic text | `settings_summary` at startup; hand-written redacting `Debug` on `Settings`, `ApiKeyConfig`, `QueueConfig`, `FileStorageConfig`, `EmailAdapterConfig` | five non-credential facts; `<redacted>` for every secret field |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation (evidence at HEAD) | Status |
|-----------|----------|-----------|----------|-------------|-------------------------------|--------|
| T-56-01 | Repudiation (false assurance) | `check-findings-log.sh` passing vacuously | high | mitigate | `ZERO_ENTRIES`, `UNCLOSED_FENCE`, `UNREADABLE` present in `scripts/check-findings-log.sh`; final `parsed N` line (re-run at HEAD: `parsed 36 entries`, exit 0); harness `tests/scripts/check-findings-log_test.sh` 43 assertions pass | closed |
| T-56-02 | Tampering | malformed or self-referential disposition passing | high | mitigate | `BAD_DUP_TARGET`, `NO_POINTER`, `BAD_OUT` classes present with fire fixtures in the harness | closed |
| T-56-03 | Tampering (input handling) | log content interpreted as code | medium | mitigate | log path passed as an argument to a quoted python3 heredoc; 0 `eval` occurrences; anchored regexes | closed |
| T-56-04 | Denial of service (CI) | checker turning CI red on a legitimately open log | medium | mitigate | 0 references in `.github/workflows/`; `check-findings-log` target is outside `check-gates` in `Makefile` | closed |
| T-56-05 | Tampering | the checker rewriting the log | low | mitigate | no write mode; harness no-mutation checksum cases | closed |
| T-56-06 | Information disclosure | assertion messages formatting `Settings` | medium | mitigate | `render_path` key-path-only messages in `tests/config_example_roundtrip.rs`; 15 `test-key-not-a-secret` placeholders in fixtures | closed |
| T-56-07 | Tampering (silent config drop) | a section silently ignoring an operator key | high | mitigate | one-directional key-path diff through both loaders; zero-unexplained assertion; gaps logged as DF entries and promoted (56-02-SUMMARY) | closed |
| T-56-08 | Repudiation (false assurance) | an `EXEMPT` / `KNOWN_GAPS` row masking a key | high | mitigate | `stale_rows` present; `serialised_settings_carry_no_llm_api_key_and_no_webhook_secret` present; drift-guard test | closed |
| T-56-09 | Tampering | example edited to make the test pass | medium | mitigate | P5; diff limited to `steps: []` (56-02-SUMMARY); `shipped_example_*` tests re-run | closed |
| T-56-10 | Elevation (scope) | flipping sections to refuse unknown keys | medium | mitigate | P7; deferred to Stage C, decided there (56-12 refuses only validator-rejected values, not unknown keys) | closed |
| T-56-11 | Information disclosure | a real key reaching a paste slot, history, repo or `.env` | high | mitigate | 3 `read -rs` entries, `## Redaction` and `## Cleanup` sections in `56-RUNBOOK-KEYED-WALK.md`; tree scan at HEAD finds no phase-56 artifact with a key-shaped literal | closed |
| T-56-12 | Spoofing / repudiation | fabricated web3sec return triaged as the consumer's | high | mitigate | P1; W1 relayed verbatim; 5 SHA `70657919…` references across `56-RUNBOOK-WEB3SEC.md` and `56-WEB3SEC-RETURN-REQUEST.md`; UAT test 3 (P1/P4/P11) confirmed by the maintainer | closed |
| T-56-13 | Repudiation (false assurance) | local coverage figure computed differently from CI's | medium | mitigate | CI LF/LH snippet embedded in `56-RUNBOOK-COVERAGE.md` with `int()` truncation (4 refs); same-commit rule; UAT test 2 confirmed by the maintainer | closed |
| T-56-14 | Tampering | return markup confusing the checker | low | accept | W1 lives in the runbook, not the log; see AR-56-02 | closed |
| T-56-15 | Repudiation (false assurance) | DF-04 / DF-01 closed on stale evidence | high | mitigate | every proof re-run on HEAD with per-name `ok` greps (56-04-SUMMARY); D-00e failure path withholds the closing bullet | closed |
| T-56-16 | Tampering | append-only log rewritten | medium | mitigate | `git diff --numstat 8e02a457e..HEAD` on the log: 0 deleted lines (measured at HEAD) | closed |
| T-56-17 | Denial of service (local) | disk exhaustion misread as a failure | low | mitigate | PD-04-1 cleanup and retry; never logged as a finding | closed |
| T-56-18 | Information disclosure | `src/main.rs` `{:?}` of `Settings` | critical | mitigate | reproduced and logged DF-07 S1 (56-05); fixed by 56-11 (T-56-38); 0 Debug-formats of settings remain in `src/main.rs` | closed |
| T-56-19 | Information disclosure | probe log or evidence carrying a key | medium | mitigate | fake key only; `<fake-key>` in evidence; acceptance grep (56-05-SUMMARY) | closed |
| T-56-20 | Denial of service / tampering | server accepting a `trace` value its type rejects | high | mitigate | reproduced as DF S2 (56-05); fixed by 56-12 (T-56-44) | closed |
| T-56-21 | Repudiation (misleading config) | `mermaid_url` documented as effective | low | mitigate | comment-only correction (P5); round-trip tests re-run | closed |
| T-56-22 | Repudiation (false evidence) | walk resolving path deps or the workspace lockfile | high | mitigate | 2 `mktemp -d` scratch dirs and the `grep -c 'path = '` check recorded in `56-FIRST-RUN-WALK.md:22,32`; `--locked` install | closed |
| T-56-23 | Information disclosure | a real key used during the unkeyed walk | high | mitigate | provider keys unset in every command (D-14); onboarding without a TTY; `.env` listed and deleted (P2) (56-06-SUMMARY) | closed |
| T-56-24 | Tampering (supply chain) | compiling the published release's crates.io deps | medium | accept | exact CI-audited set via `--locked`; throwaway target deleted; see AR-56-03 | closed |
| T-56-25 | Denial of service (local) | disk exhaustion from two large builds | medium | mitigate | PD-06-2 free-first, debuginfo off, targets deleted between steps | closed |
| T-56-26 | Tampering (supply chain via docs) | unpinned install line | medium | mitigate | both `cargo install … paladin-ai` lines under `docs/` and `README.md` carry `--locked`; 0 without | closed |
| T-56-27 | Repudiation | closing a docs finding without proof | low | mitigate | P4; every closing bullet names a `## Contained fixes` row (56-07-SUMMARY) | closed |
| T-56-28 | Tampering | frozen `MIGRATION.md` §9/§10 or `[0.11.0]` edited | medium | mitigate | D-00c; acceptance asserted unchanged (56-07-SUMMARY) | closed |
| T-56-29 | Information disclosure | a real key pasted into K-slots and committed | critical | mitigate | `## Redaction` before paste; 56-08 scanner over every staged file, proven on a planted fake; tree scan at HEAD: no phase-56 file carries a key-shaped literal | closed |
| T-56-30 | Information disclosure | onboarding's `.env` with the real key left on disk | high | mitigate | `## Cleanup` and 3 `cleaned` lines in `56-RUNBOOK-KEYED-WALK.md` (K9); UAT test 4 confirmed by the maintainer | closed |
| T-56-31 | Repudiation | stumbles summarised away | low | mitigate | every K-row names a DF or `no stumble` | closed |
| T-56-32 | Repudiation (false assurance) | todo closed on figures from different commits | high | mitigate | same-commit lookup by SHA; integer `LH/LF` with `int()`; UAT test 2 confirmed by the maintainer | closed |
| T-56-33 | Information disclosure | `.env` values in pasted C-slots | medium | mitigate | runbook `## Redaction`; C-slots hold command output only | closed |
| T-56-34 | Tampering | `ci.yml` edit breaking the workflow unseen | medium | mitigate | one label line; python3 YAML parse; CI Workflow Lint job (56-09-SUMMARY) | closed |
| T-56-35 | Spoofing / repudiation | fabricated return logged as the consumer's | high | mitigate | P1; triage only from W1; SHA compared; UAT test 3 confirmed | closed |
| T-56-36 | Repudiation (false assurance) | phase declared closeable with an S1/S2 unfixed, or D-16 defaulted by the agent | high | mitigate | P11 held (UAT test 3); checker exit 0 at HEAD with every entry's last disposition final; commit `1fb83d85f` records the DOGF-02 close gate OK | closed |
| T-56-37 | Tampering | `REQUIREMENTS.md` edited beyond the DOGF-02 row | low | mitigate | acceptance limited the diff to DOGF-02 lines (56-10-SUMMARY) | closed |
| T-56-38 | Information disclosure | `src/main.rs` startup `info!` line | critical | mitigate | whole-config format removed (0 Debug-formats of settings in `src/main.rs`); `settings_summary` present; `settings_summary_names_no_credential` present | closed |
| T-56-39 | Information disclosure | derived `Debug` on five config types | high | mitigate | 5 hand-written `impl Debug` (`Settings`, `ApiKeyConfig`, `QueueConfig`, `FileStorageConfig`, `EmailAdapterConfig`); 15 `<redacted>` literals in `src/config/`; `settings_debug_names_every_serialized_field` present | closed |
| T-56-40 | Information disclosure | credentials already written by v0.9.0–v0.11.0 binaries | high | mitigate | `MIGRATION.md` §11.8 rotation-and-purge step present; `CHANGELOG.md` `### Security` entry present | closed |
| T-56-41 | Information disclosure | URL fields that may embed userinfo in derived `Debug` | low | accept | only whole-config log site removed; `settings_summary` renders no URL; see AR-56-04 | closed |
| T-56-42 | Information disclosure | `Serialize` of config types still emits secrets | low | accept | no shipped path serialises `Settings` outward; see AR-56-05 | closed |
| T-56-43 | Repudiation (false assurance) | a redaction test that never planted its sentinel | medium | mitigate | each test asserts the planted value is present before formatting and `<redacted>` after (56-11-SUMMARY) | closed |
| T-56-44 | Tampering (configuration integrity) | boot accepting validator-rejected `trace` / `agent_runtime` values | medium | mitigate | `.validate()` called at `src/bin/paladin-server.rs:139,143` before any store or listener; refusal tests present | closed |
| T-56-45 | Repudiation (silent observability loss) | `trace.otel.enabled: true` without the `otel` feature | medium | mitigate | 4 `FeatureNotCompiled` refs; `load_settings_refuses_otel_enabled_without_the_otel_feature` present | closed |
| T-56-46 | Information disclosure | refusal line echoing an `otel.headers` token | medium | mitigate | validators echo key names and non-secret values only; otel test plants a header sentinel and asserts absence (56-12-SUMMARY) | closed |
| T-56-47 | Denial of service (availability on upgrade) | a v0.11.0-bootable config now refuses | low | mitigate | `M-B-37` and §11.8 pre-upgrade step; `load_settings_accepts_the_shipped_example_config` present | closed |
| T-56-48 | Information disclosure | provider reply embedded in setup-check / onboarding errors | high | mitigate | `diagnostic_excerpt` present (exact key, patterns, then 512-char bound); `anthropic_probe_failure_redacts_the_key_from_the_reply`, `deepseek_probe_failure_carries_a_redacted_reply` present | closed |
| T-56-49 | Information disclosure | `x-api-key` / `Authorization` forwarded on a redirect | medium | mitigate | `probe_client()` sets `redirect(Policy::none())`; `anthropic_probe_does_not_follow_a_redirect` present | closed |
| T-56-50 | Spoofing / integrity of the check result | a valid key reported invalid | medium | mitigate | `anthropic_probe_model_is_the_adapters_seeded_default` present; mocked 200 and 404 tests; maintainer re-check `✓ Anthropic` on `889b5f1bb` (UAT test 4) | closed |
| T-56-51 | Information disclosure | the maintainer's real key during the re-check | high | mitigate | `read -rs` then `unset`; redacted paste only; `56-13-SUMMARY.md` carries 0 `sk-` | closed |
| T-56-52 | Tampering (configuration integrity) | env-injected `trace` / `agent_runtime` / `web_server` values | medium | mitigate | overlay applied before `Settings::validate()`; `load_settings_refuses_a_zero_trace_channel_capacity_from_the_environment`, `…_an_invalid_agent_runtime_value_from_the_environment` present; residual (env-setter gains nothing new) accepted in-plan | closed |
| T-56-53 | Repudiation (silently ignored operator control) | a documented knob having no effect | medium | mitigate | three `applies` tests and the lib contract test; `M-B-38` present (5 M-B-37/38 refs) | closed |
| T-56-54 | Information disclosure | an overlay value echoed into a log or refusal | low | mitigate | no new log line; `read_env` logs nothing; no secret-shaped env variable in the three sections (56-14 manual review) | closed |
| T-56-55 | Denial of service (availability on upgrade) | a stray env variable now applies | low | mitigate | `M-B-38` listing; `load_settings_leaves_file_values_when_no_overlay_is_set` present | closed |
| T-56-56 | Tampering (test isolation) | env leaking between tests | low | mitigate | 15 `#[serial]` in `src/bin/paladin-server.rs`; `Drop` restore in the bin helper | closed |
| T-56-SC | Tampering | package installs (14 per-plan rows) | low | accept | no manifest or `Cargo.lock` change in any plan (each SUMMARY's `git diff --quiet`; clean at HEAD); see AR-56-01 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-56-01 | T-56-SC | No plan adds a package to the workspace; scratch installs are the project's own published crate under `--locked` into roots that are deleted; `serde_yaml` 0.9 was already in `Cargo.lock` | plan author (56-01..14 `<threat_model>`), confirmed at secure-phase | 2026-11-04 |
| AR-56-02 | T-56-14 | The relayed return lives in a runbook the checker never reads; DF entries quote divergences inside code fences the checker ignores | plan author (56-03), confirmed at secure-phase | 2026-11-04 |
| AR-56-03 | T-56-24 | Building the published release's dependency set is the exact set CI audited at release time (`--locked`); built in a throwaway target and deleted | plan author (56-06), confirmed at secure-phase | 2026-11-04 |
| AR-56-04 | T-56-41 | URL fields carry a credential only if an operator embeds one; the single whole-config log site is removed and `settings_summary` renders no URL (PD-11-5) | plan author (56-11), confirmed at secure-phase | 2026-11-04 |
| AR-56-05 | T-56-42 | No shipped code path serialises `Settings` outward; `skip_serializing` would change the 56-02 round-trip contract and is a separate decision (PD-11-6) | plan author (56-11), confirmed at secure-phase | 2026-11-04 |

*Accepted risks do not resurface in future audit runs.*

---

## Out-of-register finding (escalated, not counted toward threats_open)

The tree scan for key-shaped literals that verifies T-56-11 and T-56-29 found one match that is
**not** a test fake and **not** a Phase 56 artifact:

- `.project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md:53` carries a
  complete `sk-proj-…` OpenAI project-key literal (164 characters) inside a "should be (FIXED)"
  shell example. The file entered the tree no later than commit `928c6d5f9` (2026-07-30,
  "chore: moved project to .project"), so the literal is also in git history.

This predates Phase 56 and crosses none of the phase's trust boundaries, so by the per-phase
contract it does not count toward `threats_open`. It is escalated to the maintainer as a
credential-handling finding in its own right: **rotate the key if it was ever live, then scrub
the literal from the file** (and decide whether history needs purging). A standing tracked-tree
scan for key-shaped literals, as 56-08's staged-file scanner does per commit, would have caught
it; that guard is a candidate for `check-gates`.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-04 | 70 (56 numbered + 14 `T-56-SC`) | 70 | 0 | secure-phase orchestrator, L1 grep-depth at HEAD `a4a2374d9` |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-04 (out-of-register finding above awaits maintainer action)
