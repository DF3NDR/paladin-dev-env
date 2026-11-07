---
phase: 57
slug: register-hygiene-close-out
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-11-05
verified_head: d12d1d51e
---

# Phase 57 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time. All sixteen PLAN files (57-01 through 57-16) carry a
`<threat_model>` block: 58 numbered threats (T-57-01 through T-57-58) plus a per-plan supply-chain
row `T-57-SC` on the two plans that touch dependency governance or a port trait (57-01, 57-02; both
`accept`, no package or lockfile change). Every SUMMARY's `## Threat Flags` says "None" or maps its
applied mitigations to registered ids. The phase adds no network endpoint, auth path or schema: it
governs one new RustSec advisory, replaces the thread-ownership rule (latest run -> earliest
attributed run) behind the existing tenant gate, closes six archived review warnings in code
(run-error redaction, S3 error redaction, list truncation, effort-refusal label, commit-point
emission, admission settlement, zero `Retry-After` pacing, fence-marker TTL), and seals the
planning registers (WINDOWS ledger, review dispositions, judgment read, debug archive).

Verification ran at L1 grep depth (ASVS level 1, register authored at plan time, short-circuit
rule of `secure-phase.md` §3) at HEAD `d12d1d51e`, whose Rust code is the executed phase head
`bf15bcdad` (the `57-VERIFICATION.md` head) plus docs-only phase-completion commits. Each
mitigation claimed by a SUMMARY was looked up at HEAD (file, symbol, test name or recorded
evidence); `scripts/check-review-dispositions.sh` (strict default) and
`scripts/check-advisory-register.sh` were re-run; `Cargo.lock` and every manifest were diffed over
the phase range `bc0267e5..HEAD`; the archived REVIEW.md files' deletion count over the same range
was measured; `MIGRATION.md` hunks were located against its section headings; and the phase's
changed files and artifacts were scanned for key-shaped literals (every hit is a documented test
fake: `AKIAEXAMPLE0000000000`, `sk-planted-51wr01-secret`).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| RustSec advisory-db -> release gates | a suppressed advisory stops `make security` and CI from failing on it | advisory ids in `SECURITY-EXCEPTIONS.md`, `.cargo/audit.toml`, `deny.toml`; set-equality checked by `check-advisory-register.sh` |
| planning registers -> milestone audit and `/gsd-ship` | the audit trusts `WINDOWS.md` `open_count`, review dispositions, judgment-read verdicts and SUMMARY frontmatter as written | status and reason cells; tool-only edits with byte-identical round-trip proof |
| tenant principal -> thread visibility (`/v1/threads*`, `POST /v1/runs`, fork) | access is decided from the owner run (earliest attributed) via `owner_run_for_thread` + `permits_thread` | tenant id vs stored `submitted_by`; a foreign tenant gets the byte-identical missing-thread `404` |
| run store rows -> authorization read | a corrupt or newer-schema row is input to an access decision | single-row `LIMIT 1` owner read; `UnknownSchemaVersion` hidden as `404`, skipped in lists |
| LLM provider response -> run error -> `GET /v1/runs/{id}` and log | provider text may embed credentials | `bounded_run_error` (redact, then bound) before persist and `warn!` |
| LLM provider / out-of-tree `LlmPort` -> `check_reasoning` error -> `ConfigurationError` | vendor response text may ride inside an `LlmError` payload | `llm_error_label` renders the variant kind only |
| S3/RustFS -> adapter errors -> callers and logs; CI runner -> public CI logs | response bodies echo access-key id and request detail; a presigned URL is a live credential | `s3_err_summary` on every `map_err`; `without_url()` on every reqwest error in the presigned test |
| Paladin -> operator webhook receiver | the signature header is the receiver's only authenticity check | empty secret treated as absent; boot warning names the key, never the URL |
| run worker -> event bus / herald / trace -> SSE subscribers and spend readers | wire and audit state must match the durable run row | terminal event only through `on_committed` after writes and ack |
| HTTP client -> admission handler future (cancellable) ; Treasurer -> notice store and webhook queue | a disconnect can drop the handler at any await; the notice is the operator's spend alert | `AdmissionSettlement` drop guard, give-back on failed enqueue; failures never propagate to the run |
| provider `Retry-After` -> `LlmError::RateLimitExceeded` -> fleet pacing gate | provider-controlled values decide how long the whole fleet waits | zero filtered to `None`; reset-derived delay clamped to `max_backoff`; 429 recorded in a spawned, awaited task |
| stampede-lock holders -> Redis fencing counter and node-cache fence marker | the fence decides which worker's write is kept | marker TTL `clamp(2 * entry ttl, 5 min, 30 min)` from one shared definition |
| local repo -> remote branch and CI | the push publishes the phase; CI job ids are the evidence for ledger rows | phase branch only, never forced; rows flipped only on green jobs for the pushed SHA |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation (evidence at HEAD) | Status |
|-----------|----------|-----------|----------|-------------|-------------------------------|--------|
| T-57-01 | Elevation / Tampering (exploitable advisory hidden by a suppression) | `RUSTSEC-2026-0429` exception | high | mitigate | Row `id = "RUSTSEC-2026-0429"` in `SECURITY-EXCEPTIONS.md:178` with build-dependency-only path; control re-measured from the `utoipa-swagger-ui` 9.0.2 build script (`by_index` + `enclosed_name()`, no `.extract(` call) and `cargo tree -i zip@3.0.0` showing `[build-dependencies]` only (57-01-SUMMARY `## Compensating control`); `make security` exit 0 re-run in 57-VERIFICATION | closed |
| T-57-02 | Repudiation (register and mirrors drift) | `SECURITY-EXCEPTIONS.md`, `.cargo/audit.toml`, `deny.toml` | medium | mitigate | `scripts/check-advisory-register.sh` re-run at HEAD: `16 register row(s) checked against 16 deny.toml and 9 .cargo/audit.toml ignore entries; all clauses satisfied`, exit 0; `make security` run separately (57-01-SUMMARY, 57-VERIFICATION) | closed |
| T-57-03 | Repudiation (requirement claimed complete without evidence) | four archived SUMMARYs (45-01, 51-13, 51-14, 51-15) | medium | mitigate | `requirements-completed` frontmatter with appended dated evidence note on each (57-01-SUMMARY lines 32-35); read back with `summary-extract` | closed |
| T-57-04 | Tampering (debug record lost) | `.planning/debug/` | low | mitigate | three sessions present under `.planning/debug/resolved/` (`kimi-fallback-profile-stale`, `openai-max-completion-tokens`, `vendor-token-usage-fidelity`); moved with `git mv`, none deleted | closed |
| T-57-05 | Elevation of privilege (cross-tenant thread takeover by a later Admin or schedule run) | `owner_run_for_thread` + `permits_thread` | high | mitigate | SQL `ORDER BY (tenant_id IS NULL) ASC, submitted_at ASC, run_id ASC LIMIT 1` (`crates/paladin-storage/src/run/sqlite.rs:77`); `owner_run_for_thread_ignores_later_admin_and_unattributed_runs` present on 4 adapters; ADR-0063 Decision 1 amended | closed |
| T-57-06 | Elevation (a tenant claims a schedule-only thread as thread birth) | the `None` meaning | high | mitigate | earliest-run fallback in the same query; `owner_run_for_thread_falls_back_to_the_earliest_run_when_none_is_attributed` present on 4 adapters | closed |
| T-57-07 | Denial of service (one corrupt older row locks the owner out) | SQL owner read | medium | mitigate | `LIMIT 1` single-row read; `owner_run_for_thread_decodes_only_the_owner_row` present on 4 adapters; port doc at `run_repository_port.rs:415` | closed |
| T-57-08 | Tampering (semver break for downstream implementors) | `RunRepositoryPort` | medium | mitigate | defaulted trait body; `scripts/check-semver.sh --package` patch-level runs with 0 failures for three packages; `make api-surface` unchanged (57-02-SUMMARY lines 101-103) | closed |
| T-57-09 | Elevation of privilege (submit or fork onto a thread the tenant only touched later) | `ensure_thread_visible` | high | mitigate | owner-run guard (3 files); `submit_guard_reads_the_owner_run_not_the_newest_run` present in `src/application/services/run/submission.rs`; tracer submit assertions over SQLite (57-03-SUMMARY) | closed |
| T-57-10 | Information disclosure (existence oracle: `500` vs `404`) | `load_visible_thread` | medium | mitigate | `thread_owner_error_for_scope` (`thread_controller.rs:750`) hides `Serialization` and `UnknownSchemaVersion`; `newer_schema_owner_row_hides_the_thread_from_a_tenant` present | closed |
| T-57-11 | Denial of service (one newer-schema row fails every tenant's list) | `visible_thread_page` | medium | mitigate | skip-and-log arm at `thread_controller.rs:826-831`; `list_threads_skips_a_newer_schema_thread_for_a_tenant` present | closed |
| T-57-12 | Information disclosure (log content) | skip-and-log line | low | mitigate | line logs only `{e}` (the error's `Display`) and "thread skipped as not visible"; the list test asserts the response body carries no injected text (57-03-SUMMARY) | closed |
| T-57-13 | Information disclosure (residual: `GET /v1/runs/{id}` newer-schema `500`) | `run_repo_error_for_scope` | low | accept | residual T-53-10 confined to the run routes; ADR-0063 amendment records it; see AR-57-02 | closed |
| T-57-14 | Repudiation (published contract misstates the shipped rule) | ADR-0063, guides | medium | mitigate | `0063-thread-and-job-tenant-visibility.md:7,27` amendment paragraph quoting every replaced sentence; 57-04-SUMMARY `## Sweep` lists every remaining `latest run` hit with its justification; 0 hits for the ownership phrasing in `docs/src` at HEAD | closed |
| T-57-15 | Information disclosure / availability on upgrade (legacy mixed-tenant thread changes owner) | MIGRATION 11.8 | medium | mitigate | `M-B-24` row (`MIGRATION.md:14`) and 11.8 step 12 (`MIGRATION.md:148`) name the effect and the pre-upgrade check | closed |
| T-57-16 | Tampering (frozen release records edited) | MIGRATION parts 9-10, `[0.11.0]` | low | mitigate | every `MIGRATION.md` hunk over `bc0267e5..HEAD` lands at lines 11-150; part 10 begins at line 160 (part 11 is lines 9-159); CHANGELOG hunks at lines 33-60 (`[Unreleased]`) | closed |
| T-57-17 | Information disclosure (secret in persisted run error or log) | `persist_failure` | high | mitigate | `bounded_run_error` (`worker.rs:154`) calls `redact_secret_patterns` then `bounded_excerpt`; used at `worker.rs:2097` before persist and `warn!`; `bounded_run_error_redacts_before_bounding` present in `worker_tests.rs` | closed |
| T-57-18 | Spoofing (forged operator notices via empty-key HMAC) | operator webhook signing | medium | mitigate | `operator_webhook_secret_warning_line` (`run_api_wiring.rs:828`) treats empty as absent (`is_some_and(!is_empty)`); `platform-api.md:586` states it; `operator_webhook_secret_warning_line_names_the_key_and_never_the_url` and `_is_silent_with_a_secret` present | closed |
| T-57-19 | Information disclosure (warning leaks the URL or a value) | `operator_webhook_secret_warning_line` | medium | mitigate | sentinel URL `hook-sentinel-url` test asserts absence (`run_api_wiring.rs:2927`) | closed |
| T-57-20 | Spoofing (residual: absent secret still signs with the empty key) | operator webhook | medium | accept | boot warning plus documentation (`platform-api.md:586`, MIGRATION 11.8) is the L1 control; a boot refusal is outside D-00b's additive patch; see AR-57-03 | closed |
| T-57-21 | Information disclosure (access-key id and request detail in error text) | `minio.rs` `map_err` sites | high | mitigate | `s3_err_summary` (`minio.rs:122`) used 20 times across the adapter's 16 `map_err` sites; `s3_err_summary_never_echoes_the_response_body` present (`minio.rs:1299`) | closed |
| T-57-22 | Information disclosure (presigned URL in CI logs) | presigned-URL integration test | high | mitigate | five `.map_err(|e| e.without_url())` sites in `tests/integration/file_storage_integration_tests.rs:418-464`; CI job `126917997844` ran `test_presigned_urls` ok (57-REGISTER-EVIDENCE row 93) | closed |
| T-57-23 | Tampering / integrity (keys silently dropped; truncation hidden) | `list_files` | medium | mitigate | `list_page_request` with `max_keys` (`minio.rs:155-160`), `has_more`/token from the page (`minio.rs:614-636`); `list_request_sends_the_limit_as_max_keys_never_as_a_delimiter` present; RustFS case `test_list_files_honours_limit_and_drops_no_keys` ok in CI job `126917997844` (row 90) | closed |
| T-57-24 | Denial of service (open multipart uploads leak storage) | multipart docs | low | mitigate | rustdoc at `minio.rs:1043,1071-1072` tells callers to call `abort_multipart_upload` | closed |
| T-57-25 | Repudiation (a human verdict recorded that the owner did not give) | Human column | high | mitigate | `57-JUDGMENT-READ.md` carries a separate `LLM verdict` column and a `Human: confirm / reject` column; all 26 Human cells read `confirm (2026-11-04)` from the owner's reply `57-07: confirm all`; `pending: 0` | closed |
| T-57-26 | Information disclosure (credential violation in Phase 40's diff unnoticed) | credential checklist | medium | mitigate | C1-C6 re-run over the diff with hunk evidence (`57-JUDGMENT-READ.md:47-52`); all `held`, each confirmed | closed |
| T-57-27 | Information disclosure (scratch diff kept on disk) | scratch file | low | mitigate | diff read through the GitHub compare API (57-07-SUMMARY:103); no `.diff`/`.patch`/scratch file enters the tree over `bc0267e5..HEAD`; working tree clean | closed |
| T-57-28 | Repudiation (vacuous dispositions: closed with no proof) | `57-REVIEW-DISPOSITIONS.md` | medium | mitigate | `CLOSED_NO_POINTER` and `NO_EVIDENCE` classes in `scripts/check-review-dispositions.sh`; strict run at HEAD: `checked 51 finding(s) across 13 review file(s); 0 failure(s)` | closed |
| T-57-29 | Elevation (a live security warning disposed silently) | the sweep | high | mitigate | `## Live in tree` section present (`57-REVIEW-DISPOSITIONS.md:82`); the one security-class live item (51 WR-01) fixed in 57-12; strict checker 0 failures | closed |
| T-57-30 | Tampering (archived review text altered) | 6 REVIEW.md files | low | mitigate | `git diff --numstat bc0267e5..HEAD` over every archived `*-REVIEW.md`: 4 added, 0 deleted lines per file | closed |
| T-57-31 | Repudiation (closure asserted without evidence) | Phase 38/44/47/51 rows | medium | mitigate | per-row command recorded in 57-09-SUMMARY; strict checker 0 failures | closed |
| T-57-32 | Elevation (live security warning left unclassified) | LIVE rows | high | mitigate | PD-09-1 class per row under `## Live in tree`; 57-10 triage decided fix-in-phase for 51 WR-01 (57-12) | closed |
| T-57-33 | Tampering (archive edited) | 4 REVIEW.md files | low | mitigate | same numstat measurement: 0 deleted lines | closed |
| T-57-34 | Elevation (live security or tenant defect waived without decision) | the triage | high | mitigate | blocking `checkpoint:decision` recorded in 57-10-SUMMARY; no correctness or security row waived; waivers confined to rows 64-66, 77, 79 and 101-108 with owner and basis (57-11-SUMMARY lines 79-80) | closed |
| T-57-35 | Repudiation (closure asserted for 41/43/45 items) | the register | medium | mitigate | gap-closure fixes landed as 57-12..57-16 with tests; strict checker 0 failures | closed |
| T-57-36 | Tampering (archive edited) | 3 REVIEW.md files | low | mitigate | same numstat measurement: 0 deleted lines | closed |
| T-57-37 | Repudiation (a row closed on stale or red evidence) | WINDOWS reason cells | high | mitigate | rows 68/90/93 cite CI run `41927940508` jobs on pushed SHA `4b115d511` (`Security Audit` `126917997616`, `License & Dependency Policy` `126917997392`, `Integration Tests` `126917997844`, all `success`); `43-REVIEW WR-01` flipped only after the Redis node-cache job `126917997409` log (57-REGISTER-EVIDENCE lines 22-36, 106) | closed |
| T-57-38 | Tampering (ledger corrupted by hand edit) | `.planning/WINDOWS.md` | medium | mitigate | tool-only status changes; byte-identical `parseLedger`/`renderLedger` round-trip after every edit (57-11-SUMMARY:22,61); frontmatter `open_count: 0, waived_count: 53, fixed_count: 55, total_count: 108` | closed |
| T-57-39 | Elevation (a security row waived to reach `open_count: 0`) | waivers | medium | mitigate | waivers only for 64-66/77/79 and triage-named items 101-108; D-03 shape and no-pipe clause checked per row (57-11-SUMMARY:79-80,116) | closed |
| T-57-40 | Tampering (unreviewed push to a shared branch) | `git push` | low | accept | phase branch `claude/laughing-dirac-e0h2ax` only, `git push -u`, never forced (57-REGISTER-EVIDENCE:144); see AR-57-04 | closed |
| T-57-41 | Information disclosure (vendor text or a key in a `ConfigurationError`) | `PaladinBuilder::effort_refusal` | high | mitigate | `llm_error_label` (`paladin_builder.rs:1638`) renders the variant kind only with a `_ => "unknown error"` arm; `effort_refusal_renders_only_the_error_kind_for_a_failed_check` and `_through_a_fallback_chain_` present with the planted secret | closed |
| T-57-42 | Repudiation (operator loses the vendor reason) | refusal message | low | accept | model, provider and kind kept; vendor text deliberately not logged (EA-57-10); see AR-57-05 | closed |
| T-57-43 | Information disclosure (out-of-tree port puts vendor text in `UnsupportedParameter.reason`) | `UnsupportedParameter` arm | low | accept | arm unchanged by contract (ADR-0061); in-tree adapters build it from profile data; see AR-57-06 | closed |
| T-57-44 | Tampering / Repudiation (wire and trace claim a terminal outcome the store never committed) | `run_agent` exits | medium | mitigate | `on_committed` hook runs after both writes and ack (`worker.rs:1983-2014, 2084-2121`); five write-failure tests `agent_completion_whose_status_write_fails_…`, `…outcome_write_fails…`, `…ack_fails…`, `agent_failure_whose_persist_fails…`, `agent_kind_halt_whose_outcome_write_fails…` present | closed |
| T-57-45 | Information disclosure (write-failure warning logs run output or input) | success-arm `warn!` | medium | mitigate | `worker.rs:1858-1861` logs `run.run_id` and `{repo_err}` only, with the P-13-3 comment | closed |
| T-57-46 | Repudiation (failed run under-reports billed spend) | Err arm `NodeFinished` | medium | mitigate | `ATTEMPT_SPEND` scope (`worker.rs:1767`); `failed_agent_run_reports_the_spend_it_incurred_before_failing` present in `worker_tests.rs` | closed |
| T-57-47 | Denial of service / cost (a nacked run is redelivered and repeats its LLM call) | success-arm nack | low | accept | same exposure as v0.11.0 lease-expiry redelivery (EA-57-12); see AR-57-07 | closed |
| T-57-48 | Repudiation (operator spend alert consumed by a transient failure or disconnect) | `confirm`, `admit_and_persist` | medium | mitigate | `AdmissionSettlement` with `impl Drop` (`submission.rs:201-237`); `a_submission_dropped_before_its_row_is_written_gives_the_notice_back` and `…after_its_row_is_written_confirms_the_notice` present | closed |
| T-57-49 | Denial of service (client provokes a notice failure to refuse or stall runs) | admission path | medium | mitigate | `notices_store_failure_never_blocks_admission` and `enqueue_failure_is_logged_and_never_blocks` present in `treasurer/tests.rs` | closed |
| T-57-50 | Information disclosure (new logs carry the webhook URL, a secret or a key) | new `warn!`/`error!` lines | medium | mitigate | `submission.rs:243-260` lines name `run_id` and `{error}` only; 57-14-SUMMARY acceptance grep | closed |
| T-57-51 | Denial of service (detached tasks accumulate on many disconnects) | detached settlement tasks | low | accept | one short task per dropped admission, no retry loop; see AR-57-08 | closed |
| T-57-52 | Denial of service (zero `Retry-After` disables pacing) | `minimum_delay` | high | mitigate | `err.retry_after().filter(|d| !d.is_zero())?` (`cadence.rs:340`); `a_zero_retry_after_gates_at_least_the_base_delay`, `repeated_zero_delay_429s_escalate_the_streak` present | closed |
| T-57-53 | Denial of service (a 429 lost on cancellation) | `note_failure` | medium | mitigate | `runtime.spawn` record task awaited (`cadence.rs:301`); `a_429_is_recorded_when_the_generate_future_is_dropped_mid_record` and `…stream_future…` present | closed |
| T-57-54 | Denial of service (spawned record tasks pile up against a hung Redis) | record task | low | accept | one `record_rate_limited` per observed 429, bounded by the port's timeout; see AR-57-09 | closed |
| T-57-55 | Tampering (provider shortens pacing below its stated minimum) | `minimum_delay` | low | mitigate | `a_nonzero_retry_after_below_base_is_still_honoured_exactly` and `explicit_retry_after_is_honoured_as_a_minimum` present | closed |
| T-57-56 | Denial of service / cost (valid cache writes refused after a counter expiry) | `put_fenced` marker TTL | medium | mitigate | `fence_marker_ttl_ms` shared in `redis_fence.rs:75`, used at `node_cache/redis.rs:309`; `fence_marker_never_outlives_the_counter_that_numbers_it` and live `fenced_write_after_the_counter_expires_is_accepted` present | closed |
| T-57-57 | Tampering (late holder's duplicate write accepted after the marker) | node cache entry | low | accept | deterministic result for the same fingerprint, node id and input hash (ADR-0058 (g)); see AR-57-10 | closed |
| T-57-58 | Information disclosure (live test prints the Redis URL) | test `SKIP:` lines | low | accept | house pattern; CI endpoint with no credential; see AR-57-11 | closed |
| T-57-SC | Tampering (supply chain) | package installs (57-01, 57-02) | low | accept | `git diff --stat bc0267e5..HEAD -- Cargo.lock Cargo.toml crates/*/Cargo.toml` empty at HEAD; see AR-57-01 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-57-01 | T-57-SC | No plan adds a package; `Cargo.lock` and every manifest are byte-identical across the phase range (the zip advisory is governed by a register row, not a dependency change) | plan author (57-01, 57-02 `<threat_model>`), confirmed at secure-phase | 2026-11-05 |
| AR-57-02 | T-57-13 | The newer-schema `500` on `GET /v1/runs/{id}` is the recorded residual T-53-10, now confined to the run routes; the thread routes hide it (PD-03-1, ADR-0063 amendment) | plan author (57-03), confirmed at secure-phase | 2026-11-05 |
| AR-57-03 | T-57-20 | Refusing boot without a webhook secret is outside D-00b's additive patch; detection (boot warning naming the key) plus documentation is the L1 control; candidate for the next minor | plan author (57-05), confirmed at secure-phase | 2026-11-05 |
| AR-57-04 | T-57-40 | The push targets the phase branch only and is never forced; the maintainer controls the PR into `main` | plan author (57-11), confirmed at secure-phase | 2026-11-05 |
| AR-57-05 | T-57-42 | Model, provider and error kind are kept in the refusal; dropping the vendor text is the point of the fix (EA-57-10); a redaction-first diagnostic is a separate change | plan author (57-12), confirmed at secure-phase | 2026-11-05 |
| AR-57-06 | T-57-43 | `UnsupportedParameter.reason` is the port's own refusal wording by contract (ADR-0061); in-tree adapters build it from profile data only | plan author (57-12), confirmed at secure-phase | 2026-11-05 |
| AR-57-07 | T-57-47 | A nacked run's redelivery is the same exposure as v0.11.0's lease-expiry redelivery; a durable-write failure is already an incident (EA-57-12) | plan author (57-13), confirmed at secure-phase | 2026-11-05 |
| AR-57-08 | T-57-51 | One short detached task per dropped admission, bounded by its store and queue calls, no retry loop | plan author (57-14), confirmed at secure-phase | 2026-11-05 |
| AR-57-09 | T-57-54 | One `record_rate_limited` task per observed 429, bounded by the port's own timeout | plan author (57-15), confirmed at secure-phase | 2026-11-05 |
| AR-57-10 | T-57-57 | A late duplicate write carries the same deterministic result for the same graph fingerprint, node id and input hash (ADR-0058 (g)); recorded in MIGRATION and the ADR | plan author (57-16), confirmed at secure-phase | 2026-11-05 |
| AR-57-11 | T-57-58 | The live test's `SKIP:` line prints a CI test endpoint that carries no credential; existing house pattern | plan author (57-16), confirmed at secure-phase | 2026-11-05 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-05 | 60 (58 numbered + 2 `T-57-SC`) | 60 | 0 | secure-phase orchestrator, L1 grep-depth at HEAD `d12d1d51e` |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-05
