---
phase: 43-rate-pacing
plan: 13
subsystem: governance-closeout
tags: [cadence, adr-0058, changelog, semver, x-10, phase-gate, credential-review]

requires:
  - phase: 43-rate-pacing
    provides: the completed Cadence stack (43-01..43-11) and the provider header evidence (43-12)
provides:
  - ADR-0058 (rate pacing, the Cadence), indexed in PROMOTION.md with next free number 0059
  - the Cadence row in the ubiquitous-language table (D-07)
  - [Unreleased] CHANGELOG entries in paladin-ports, paladin-llm, paladin-storage and paladin-battalion, and a Known limitations paragraph in the root CHANGELOG
  - every Phase 43 MIGRATION 9.2 row carrying its empirical cargo-semver-checks result; one row corrected from N to Y
  - the full phase gate run and recorded, with the manual credential-handling review
affects: [phase-44, phase-46, phase-47]

tech-stack:
  added: []
  patterns:
    - "Measure, do not predict: a 9.2 row the semver tool contradicts is corrected, with its allowlist entry in the same commit"
    - "An ADR records where the planner read a locked decision (put_fenced), with the alternative reading and its cost"

key-files:
  created:
    - .planning/decisions/0058-rate-pacing-cadence.md
  modified:
    - .planning/decisions/PROMOTION.md
    - .github/copilot-instructions.md
    - CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - MIGRATION.md
    - .cargo/semver-checks-allowlist.toml
    - Cargo.toml
    - docs/src/appendix/provider-expansion.md
    - docs/src/contributing/contributing-providers.md

key-decisions:
  - "ADR-0058 records the defaulted NodeCachePort::put_fenced as the one deliberate reading of D-00d/D-11, with the inherent-method fallback if the operator rejects it"
  - "AgentRuntimeDeps.cadence is corrected from N to Y in the X-10 register: the tool reports constructible_struct_adds_field, already covered by the root crate-wide allow, so only an allowlist entry and a comment were added"
  - "The two provider guides showing a wrong RateLimitExceeded shape were corrected here, as 43-02 deferred them to this plan, although they are not in the plan's files_modified list"

patterns-established:
  - "Phase closeout order: register measured and committed first, then the gate run on the committed tree"

requirements-completed: [PACE-01, PACE-02, PACE-03, PACE-04, PACE-05]

duration: ~40 min
completed: 2026-10-09
status: complete
---

# Phase 43 Plan 13: Rate Pacing Closeout Summary

**ADR-0058 records the Cadence design and every place the planner read a locked decision, Cadence joins the ubiquitous language, every Phase 43 public-API change is measured against the published 0.10.1 and registered, and the full workspace gate, the live Redis suites and the credential-handling review are green and on record.**

## Performance

- **Tasks:** 2 of 2
- **Commits:** 2b4ea630 (Task 1), 550ac55c (Task 2)
- **Files:** 1 created, 12 modified

## Accomplishments

- **ADR-0058** (`.planning/decisions/0058-rate-pacing-cadence.md`, Accepted, Code Conformance `conforms`): D-00a..D-00f carried forward; D-01..D-14 with their CONTEXT reversibility ratings (D-10, D-12..D-14 carry none in CONTEXT, which the ADR says); the resolutions of research Open Questions 1-7; the composition order `Pricing(Cadence(provider))` and `Pricing(Fallback(Cadence(hop1), Cadence(hop2)))`; the 43-07 and 43-08 flagged assumptions; the known limitations; the 43-12 evidence summary; Considered Options taken from the discussion log; Code Locations; Code Conformance with test names each found by `grep`; Downstream Consumers.
- Every item the earlier executors asked 43-13 to record is in the ADR: D-02 extended to DeepSeek and Gemini (OQ1); `put_fenced` and no lock renewal with `lock_ttl_secs = 120` (OQ2, OQ3); the chain-builder wrapping site (OQ4); the `log::warn!` under `paladin::cadence` with no `TraceEvent` variant (OQ5); Gemini `RetryInfo` left unparsed and provider dimensions for OpenAI and Anthropic only (OQ6); quota-class 429 mapping to `UsageLimitExceeded` (OQ7); re-read-after-acquire in the engine lock loop (43-11); the `%lock:`/`%fence:` key layout, the percent-escaping of the provider half and the 24 h wait clamp (43-07, 43-10); the fixed `paladin:cadence` namespace (43-07, 43-09), with the note that `RedisCadenceConfig::with_key_prefix` exists but the server config exposes no key; and an unreadable fallback gate not counting as a zero wait (43-06).
- **PROMOTION.md**: the 0058 row after 0057, `Next free ADR number: 0059`, and a dated note in the house voice; no existing row touched.
- **Term table (D-07)**: a `Cadence` row after `Commissary` in `.github/copilot-instructions.md` (definition and `crates/paladin-llm/src/cadence.rs` as the plan specified). `grep -n 'fn officer_word\|fn fixture_term' tests/treasurer_vocabulary_guard.rs` returns lines 43 and 48 only: the guard enumerates the Treasurer officer word and the downstream fixture term, nothing else, so it needed no change (`cargo test --test treasurer_vocabulary_guard`: 3 passed, 0 failed).
- **CHANGELOGs**: `[Unreleased]` entries naming `Cadence` in all four crates (ports: port, lock types, hints, `LlmError` reshape, `put_fenced`; llm: decorator, header parser, first-429 surfacing, quota mapping, pace-first fallback, `rand`/`httpdate`; storage: in-memory, Redis and resilient adapters, `redis-cadence`, fenced node-cache write, shared redaction; battalion: `WarEngine::with_cadence`). The root CHANGELOG Phase 43 entry now closes with a `Known limitations` paragraph naming the fixed `paladin:cadence` namespace, the outage blind spot (A1-A3 of 43-08), Gemini's unparsed `RetryInfo`, CLI one-shot commands outside D-08's composition roots, hand-built `FallbackLlmAdapter` chains that never call `with_cadence`, and the unrenewed lock. `./scripts/check-changelogs.sh`: all 11 crates covered.
- **Docs**: `docs/src/appendix/provider-expansion.md` (match arm and the rate-limiting advice) and `docs/src/contributing/contributing-providers.md` (the "Missing Retry Logic" pitfall, rewritten around surfacing the first 429 with `hints_from_headers` and `map_http_status_with_hints`) no longer show the `RateLimitExceeded { retry_after }` shape that never matched the real type. Both blocks are `rust,ignore`; `./scripts/check-doc-config.sh` passes.

## D-27 diagnostic (cargo-semver-checks 0.50.0 vs the published 0.10.1, `--release-type minor`, crate-wide allows disabled)

The tool was not installed; the prebuilt `v0.50.0` release binary (`x86_64-unknown-linux-gnu`, the version CI pins) was downloaded with `curl` through the session proxy into the session scratchpad, outside the repository, and never committed. Each manifest was restored from a byte copy and `git diff --quiet <manifest>` confirmed.

| Package | Run | Checks | Lints fired | Phase 43 items among them | Manifest |
|---|---|---|---|---|---|
| `paladin-ports` | default | 196 (193 pass, 3 fail) | `constructible_struct_adds_field` (`SubmitRun.attributed_to`, `RunQuery.scope`, `RunOutcomeRecord.halt_reason`, `LlmResponse.cost`, `CreateRunSchedule.created_by`), `enum_unit_variant_changed_kind` and `enum_variant_marked_non_exhaustive` (`LlmError::RateLimitExceeded`) | `LlmError::RateLimitExceeded` only (two lints, both already allowed since 43-02) | RESTORED_CLEAN |
| `paladin-llm` | default | 196 (196 pass) | none | none | RESTORED_CLEAN |
| `paladin-storage` | `--features redis-cadence` (the baseline lacks it; the tool warns and proceeds) | 196 (196 pass) | none | none | RESTORED_CLEAN |
| `paladin-battalion` | default | 196 (195 pass, 1 fail) | `enum_struct_variant_field_added` (`RunOutcome::Halted.cause`) | none (a Phase 42 item) | RESTORED_CLEAN |
| `paladin-ai` | `--default-features`, and again with `--features redis-cadence` | 196 (195 pass, 1 fail) both times | `constructible_struct_adds_field` (`Settings.treasurer`, `ApiKeyConfig.tenant`, `BearerTokenAuthConfig.tenant`, `WebhookPayload.halt_reason`, `AgentRuntimeDeps.cadence`) | `AgentRuntimeDeps.cadence` | RESTORED_CLEAN |

The first `paladin-ai` attempt without `--default-features` failed in the tool's own rustdoc build of `paladin-memory` (a `qdrant-client` `VectorParams` field the freshly resolved dependency added), because the tool then enables every feature; CI's form is `--default-features`, which builds. That is a tool-environment issue, not a Phase 43 defect.

**Reconciliation.** One row was contradicted: `AgentRuntimeDeps` (43-06) was registered `N`; the tool reports `constructible_struct_adds_field`. It is now `Y` with an allowlist entry (`paladin-ai | AgentRuntimeDeps`, PACE-02) in `.cargo/semver-checks-allowlist.toml`. The root crate-wide `constructible_struct_adds_field` allow already covers it, so no lint line was added; a PACE-02 comment was added above the table in the root `Cargo.toml` (comment lines only; the four crate manifests are unchanged). All 20 Phase 43 rows now end with the measured result; the `LlmError` row (`Y`, from 43-02) was re-measured and unchanged. A stale sentence in 9.5 ("Environment overrides arrive in plan 43-09") was removed. `./scripts/check-migration-allowlist.sh`: set-equal.

**CI-parity against 0.9.0** (`--package <pkg> --default-features --baseline-version 0.9.0`, the five packages): all exit 0, each "0 checks: 0 pass, 254 skip" -- the already-documented no-op noted in STATE.md. It proves nothing about Phase 43; the 0.10.1 run is the measurement.

**Not Phase 43, flagged for the operator:** `RunOutcome::Halted.cause` (Phase 42, `paladin-battalion`) fires `enum_struct_variant_field_added` against 0.10.1 and `paladin-battalion` has no crate-wide allow for it. Its 9.2 row says `N/A -- absent at the v0.9.0 baseline`, which is true for the CI baseline. Left untouched.

## Phase gate (on the committed tree, after both task commits)

| Command | Result |
|---|---|
| `cargo test --workspace --no-fail-fast -- --skip build_run_api_persists` | exit 0; 58 suites, 6845 passed, 0 failed, 229 ignored; no `SKIP:` line (includes doc tests) |
| `cargo test --workspace --doc` | exit 0; 604 passed, 0 failed, 210 ignored |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | exit 0 |
| `cargo fmt --check` | exit 0 |
| `cargo build -p paladin-llm --no-default-features` | exit 0 |
| `make security` | exit 0 (advisories, bans, licenses, sources ok; the duplicate-crate lines are cargo-deny warnings, not failures) |
| `make check-gates` | exit 0 (allowlist set-equal; 12 crates in publish order) |
| `./scripts/check-doc-config.sh` | exit 0; 153 YAML blocks, 0 failed |
| `cargo test --test treasurer_vocabulary_guard` | 3 passed, 0 failed |
| `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | exit 0, "API surface unchanged" (4275 items) |
| `./scripts/check-migration-allowlist.sh`, `./scripts/check-changelogs.sh` | exit 0 |
| `cargo test -p paladin-ai --features web-server --lib -- --skip build_run_api_persists` | exit 0; 1346 passed, 2 filtered |

**Live Redis** (`redis-server` 7.0.15 on port 6380, `--save "" --appendonly no`; `CADENCE_REDIS_TEST_URL=redis://127.0.0.1:6380/2`, `NODE_CACHE_REDIS_TEST_URL=redis://127.0.0.1:6380/0`), each with `--nocapture` and a `SKIP:` count of 0:

- `cargo test -p paladin-storage --features redis-cadence --lib cadence`: 97 passed.
- `cargo test -p paladin-storage --features redis-cache --lib node_cache`: 31 passed (the seven `put_fenced` tests included).
- `cargo test -p paladin-ai --features redis-cadence --lib cadence_fleet`: 1 passed (`cadence_fleet_429_on_one_worker_slows_the_other`).
- Extra: `cargo test -p paladin-storage --features redis-cadence,redis-cache --lib`: 351 passed; `cargo test -p paladin-ai --features redis-cadence --lib cadence`: 21 passed (includes the live `build_cadence_redis_backend_shares_state_between_two_workers`).
- The server was shut down with `redis-cli -p 6380 shutdown nosave` and a following `ping` was refused.

**Not run here, and why.**

- **Coverage (82 percent floor, ADR-0006):** `cargo-llvm-cov` is not installed; the floor is gated by CI's `coverage` job and is not measured by this plan.
- **`actionlint`:** not installed; the `redis-cadence-integration` job added in 43-07 was parsed as YAML there, not linted.
- **Pre-existing environment failures, not caused by this phase:** `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `..._persists_run_traces_when_trace_persist_is_set` (`--features web-server`) time out in this sandbox at the pre-phase commit; they were skipped with `--skip build_run_api_persists` and not touched.
- **`cargo test --workspace --all-features`:** not run (disk headroom); the all-features surface is covered by the workspace clippy, `paladin-ai --features web-server`, and the feature-specific Redis runs above.

## Manual credential-handling review (security.instructions.md)

| Check | Evidence |
|---|---|
| Response bodies redacted before truncation on every 429 path (unchanged) | `crates/paladin-llm/src/http_status.rs:138-139` (`redact_credentials` then `bounded_excerpt`) for `map_http_status_with_hints`; `openai/adapter.rs:386` (`diagnostic_excerpt(body, &api_key)`) and `:393` (`map_http_status_with_hints`); `anthropic/adapter.rs:427` (`redact_credentials`). The 429 arm itself carries only `RateLimitHints` (parsed integers and durations, no string field), so no body text reaches `RateLimitExceeded`. |
| The 429 header snapshot reads no credential header | `openai/adapter.rs:58-71` and `anthropic/adapter.rs:66-81` call `hints_from_headers(family, now, \|name\| headers.get(name)...)`, and the parser only ever asks for the constants at `rate_limit_headers.rs:84-147` (`retry-after`, `retry-after-ms`, the six `x-ratelimit-*`, the `anthropic-ratelimit-` family, `Date`); `grep -in 'authorization\|api-key\|bearer' rate_limit_headers.rs` is empty. The Gemini, DeepSeek and compat sites (`gemini/adapter.rs:110`, `deepseek/adapter.rs:54`, `compat/engine.rs:57`) use the Generic family. Tests assert the raw `6m0s` string is absent from rendered errors (`openai/adapter.rs:1315-1317`). |
| No log line under `paladin::cadence` interpolates a key, URL or raw header | The full set of call sites: `cadence.rs:212-236` and `:305-315` (provider, model, durations, `CadenceError`), `fallback.rs:380-400` (provider, model, durations, `CadenceError`), `resilient.rs:293-322` (`CadenceError` and the multiplier; one `warn`, one `info`), `in_memory.rs:186-198` (the capacity only). The engine's lock lines (`superstep.rs:226`, `:304-308`, `:315`) name the node id and the `CadenceError`, never the lock key, which embeds a rendered-input hash; `node_cache/redis.rs:323` names only the numeric token. |
| `CadenceError` and the Redis adapters never carry a URL | `cadence_port.rs:238-257` (messages "never carry a URL or a credential"); `redis.rs:575-579` (`backend_error` carries the redis crate's description only), `:395-403` (an invalid URL is reported redacted). Test `debug_and_errors_never_render_the_password` (`redis.rs:1398-1425`, URL `redis://:hunter2@127.0.0.1:1/0`) and `an_unparsable_url_is_rejected_without_echoing_it` (`:1314-1321`); both ran in the live suite. |
| `RedisCadenceConfig`, `RedisCadence`, `CadenceConfig` and `CadenceBackend` render no secret in `Debug` or `Serialize` | `redis.rs:337-347` and `:525-535` (hand-written `Debug` routed through `redact_connection_url`); `src/config/treasurer.rs:93-105` and `:122-124`: `CadenceBackend::Redis { url_env }` and `CadenceConfig` derive `Debug`/`Serialize` but hold only the variable NAME. `src/infrastructure/cadence.rs:111-118` puts only `url_env` in messages and reads the value into `RedisCadence::new` directly; the server's boot line (`src/bin/paladin-server.rs:80-89`) names the backend kind only, asserted by `cadence_boot_summary_names_the_backend_kind_and_never_the_url_or_variable`. |
| No HTTP client was added or now follows redirects | `git diff 919fb3ce HEAD -- '*.rs'` contains no added or removed line matching `reqwest::Client`, `ClientBuilder` or `redirect`; the Cadence adds no HTTP client (Redis is a socket client), and every adapter client keeps its existing `redirect::Policy::none()`. |
| Dependency supply chain (T-43-50) | `git diff 919fb3ce HEAD -- Cargo.lock` adds three dependency edges (`httpdate`, `rand 0.8.6`, `paladin-storage` as a dev-only edge of `paladin-llm`) and no package; `cargo tree -p paladin-llm -e normal` shows no `redis` and no `paladin-storage`; `make security` exits 0. |

No credential-handling defect was found, so no code changed in this review.

## Task Commits

1. **Task 1: ADR-0058, PROMOTION.md, the Cadence term-table row, the CHANGELOG entries and the corrected provider guides** - `2b4ea630`
2. **Task 2: D-27 measurement, X-10 register reconciliation (the `AgentRuntimeDeps` row corrected to Y, its allowlist entry)** - `550ac55c`

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The 9.2 row for `AgentRuntimeDeps` was wrong**
- **Found during:** Task 2 (the D-27 diagnostic on `paladin-ai`)
- **Issue:** 43-06 registered `AgentRuntimeDeps.cadence` as `N`; the tool reports `constructible_struct_adds_field`.
- **Fix:** row marked `Y`, allowlist entry added, PACE-02 comment in the root `Cargo.toml`; the existing crate-wide allow already suppresses the lint.
- **Files modified:** `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml`, `Cargo.toml`
- **Commit:** 550ac55c

**2. [Rule 2 - Missing critical documentation] Two provider guides showed a wrong `RateLimitExceeded` shape**
- **Found during:** Task 1 (43-02 deferred them to this plan, and the orchestrator named them)
- **Issue:** `docs/src/appendix/provider-expansion.md` and `docs/src/contributing/contributing-providers.md` showed `RateLimitExceeded { retry_after }` with a numeric field and advised adapters to retry 429s, contradicting D-02.
- **Fix:** corrected both; not in the plan's `files_modified`, so noted here.
- **Commit:** 2b4ea630

**3. [Rule 3 - Blocking] The scratchpad `target/semver-checks` cache (5.8 GB) filled the disk**
- Free disk fell to 1.8 GB after the five-package diagnostic. `target/semver-checks` (regenerable, ignored by git) was deleted; no ENOSPC occurred and `target/debug` was left intact.

**4. [Plan wording] `ls .planning/decisions/0058-*.md`** confirmed the number was free before writing (no collision, so no renumbering note was needed in PROMOTION.md).

**Total deviations:** 3 auto-fixed or environmental plus one wording note. **Impact:** none on behaviour; one register correction.

## Known limitations carried (all stated in ADR-0058 and the root CHANGELOG)

The fixed `paladin:cadence` namespace; the Redis-outage blind spot and pre-outage gates invisible to the fallback; Gemini `RetryInfo` unparsed; CLI one-shot commands unpaced; hand-built fallback chains without `with_cadence`; no lock renewal and a server that attaches no node cache; the compat engine's `error_override` hook receiving no hints; `RedisNodeCache::invalidate` counting fence markers.

## Open items for the operator

- **`PROJECT.md` Key Decisions** has rows for ADR-0049..0057 but none for ADR-0058; the plan did not list `.planning/PROJECT.md`, so it was not edited here. Phase 40 added its row in the ADR commit (D-00f); the orchestrator or phase-transition step should add one.
- **`put_fenced`** is the one deliberate reading of D-00d/D-11; the ADR records the fallback (an inherent method, lock-only protection) if the operator rejects it.
- The Phase 42 `RunOutcome::Halted.cause` finding above.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. T-43-48 (register matches the tool: measured, one row corrected, allowlist set-equal), T-43-49 (credential review above, no leak found, `make security` and clippy `-D warnings` green) and T-43-50 (no new package in `Cargo.lock`) are mitigated.

## Self-Check: PASSED

- FOUND: `.planning/decisions/0058-rate-pacing-cadence.md`, `.planning/phases/43-rate-pacing/43-13-SUMMARY.md`
- FOUND: commits 2b4ea630 and 550ac55c in `git log` on `claude/laughing-dirac-e0h2ax`
- `git status` is clean apart from this SUMMARY and the state files; no downloaded binary is in the repository (it lives in the session scratchpad); the Redis server on port 6380 is shut down.
