---
phase: 43
slug: rate-pacing
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-07
validated: 2026-10-27
---

# Phase 43 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` -- `#[tokio::test]` with `start_paused = true` for every timing assertion, `mockito` 1.7.0 for provider HTTP, `MockLlmAdapter` (`mock` feature) for ports, live Redis for the `redis-cadence`/`redis-cache` halves (self-skipping locally, mandatory in CI job `redis-cadence-integration`) |
| **Config file** | none -- Cargo features select the halves: `redis-cadence`, `redis-cache` (paladin-storage, root passthrough), `web-server` (root web composition tests); live URLs from `CADENCE_REDIS_TEST_URL` / `NODE_CACHE_REDIS_TEST_URL` |
| **Quick run command** | `cargo test -p paladin-llm --lib cadence && cargo test -p paladin-storage --lib cadence` |
| **Full suite command** | `cargo test --workspace && cargo test --workspace --doc && cargo test -p paladin-storage --features redis-cadence,redis-cache --lib -- --nocapture cadence node_cache` |
| **Estimated runtime** | quick ~30-60 s; full workspace several minutes (CI-attributed coverage, STATE.md) |

---

## Sampling Rate

- **After every task commit:** Run the task's own `<automated>` command (a module-scoped `cargo test -p <crate> --lib <module>`, under 60 s)
- **After every plan wave:** Run `cargo test --workspace` plus, when a Redis server is reachable, `NODE_CACHE_REDIS_TEST_URL=redis://<host>:<port>/<db> cargo test -p paladin-storage --features redis-cadence,redis-cache --lib -- --nocapture cadence node_cache` (env var named at validation: the node-cache suite reads `NODE_CACHE_REDIS_TEST_URL`, not `CADENCE_REDIS_TEST_URL`)
- **Before `/gsd-verify-work`:** Full suite must be green (43-13 Task 2 runs the whole gate and the live Redis suites)
- **Max feedback latency:** 60 seconds per task command

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 43-01-01 | 01 | 1 | PACE-02 | T-43-01, T-43-02 | delay math never panics on hostile numbers; state only on a 429 | unit (paused clock) | `cargo test -p paladin-ports --lib cadence_port && cargo test -p paladin-storage --lib cadence` | ✅ (created in task) | ✅ green |
| 43-01-02 | 01 | 1 | PACE-02 | T-43-03, T-43-04, T-43-05 | first 429 surfaced once, decorator never retries, unknown config keys rejected | tracer (mockito end to end) | `cargo test -p paladin-ai --lib cadence_tracer_paces_a_real_openai_429_end_to_end` | ✅ | ✅ green |
| 43-01-03 | 01 | 1 | PACE-02 | -- | N/A (registers) | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 43-02-01 | 02 | 2 | PACE-01 | T-43-07 | hints carry numbers only; Display unchanged | unit + workspace check | `cargo test -p paladin-ports --lib rate_limit_hints && cargo check --workspace --all-targets --all-features` | ✅ | ✅ green |
| 43-02-02 | 02 | 2 | PACE-02 | T-43-06, T-43-08 | explicit delay is a minimum; delay beyond max_wait refused without sending or recording | unit (paused clock) | `cargo test -p paladin-llm --lib cadence` | ✅ (extends 43-01) | ✅ green |
| 43-02-03 | 02 | 2 | PACE-01 | -- | N/A (registers, D-27) | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 43-03-01 | 03 | 3 | PACE-01 | T-43-09, T-43-11 | bounded, panic-free parse; ceiling clamp | unit (table) | `cargo test -p paladin-llm --lib rate_limit_headers` | ✅ | ✅ green |
| 43-03-02 | 03 | 3 | PACE-01, PACE-02 | T-43-10, T-43-12 | no raw header in errors; Anthropic first 429; quota 429 permanent | mockito | `cargo test -p paladin-llm --all-features --lib openai && cargo test -p paladin-llm --all-features --lib anthropic` | ✅ (extends) | ✅ green |
| 43-03-03 | 03 | 3 | PACE-01 | T-43-SC | dependency promotion audited | gate | `make security && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 43-04-01 | 04 | 4 | PACE-01, PACE-02 | T-43-13 | no adapter retries a 429 internally | mockito + unit | `cargo test -p paladin-llm --all-features --lib compat && cargo test -p paladin-llm --all-features --lib deepseek && cargo test -p paladin-llm --all-features --lib gemini` | ✅ (extends) | ✅ green |
| 43-04-02 | 04 | 4 | PACE-01, PACE-02 | T-43-13 | every fixture: 429 observed once with its delay | conformance | `cargo test -p paladin-llm --all-features --lib rate_limit_is_surfaced_once_with_its_retry_delay` | ✅ (case 10) | ✅ green |
| 43-05-01 | 05 | 5 | PACE-02 | T-43-16, T-43-18 | bounded state; concurrent 429s escalate once | contract (paused clock) | `cargo test -p paladin-storage --lib cadence` | ✅ (contract_tests.rs) | ✅ green |
| 43-05-02 | 05 | 5 | PACE-02 | T-43-17 | streams paced; spread never undercuts a provider delay | unit (paused clock) | `cargo test -p paladin-llm --lib cadence && cargo build -p paladin-llm --no-default-features` | ✅ (extends) | ✅ green |
| 43-05-03 | 05 | 5 | PACE-02 | -- | N/A (registers) | gate | `make security && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 43-06-01 | 06 | 6 | PACE-02 | T-43-20, T-43-21, T-43-22 | pace budget bounds the stay; hops still happen | unit (paused clock, SC2) | `cargo test -p paladin-llm --lib fallback` | ✅ (extends) | ✅ green |
| 43-06-02 | 06 | 6 | PACE-02 | -- | config-built chains paced by default | unit | `cargo test -p paladin-ai --lib config::agent_runtime && cargo test -p paladin-ai --lib application::services::paladin::middleware::resilience` | ✅ (extends) | ✅ green |
| 43-07-01 | 07 | 7 | PACE-03 | T-43-23..T-43-27 | URL redacted; scripts injection-free; bounded timeouts; TTL on every key | contract (live Redis) | `cargo test -p paladin-storage --features redis-cadence --lib cadence -- --nocapture` | ✅ (redis.rs) | ✅ green |
| 43-07-02 | 07 | 7 | PACE-03 | T-43-26 | fleet gate from server clock | integration (live Redis) | `cargo test -p paladin-ai --features redis-cadence --lib cadence_fleet -- --nocapture` | ✅ | ✅ green |
| 43-08-01 | 08 | 8 | PACE-05 | T-43-29..T-43-31 | never Err; one warning per outage; probe back-off | unit (paused clock, injected failing port) | `cargo test -p paladin-storage --lib cadence::resilient` | ✅ (resilient.rs) | ✅ green |
| 43-08-02 | 08 | 8 | PACE-05 | T-43-29, T-43-32 | dead Redis degrades within the timeout budget; still paced | integration (no server needed) | `cargo test -p paladin-storage --features redis-cadence --lib degrades_within_the_timeout_budget && cargo test -p paladin-ai --lib cadence_with_redis_down_still_paces` | ✅ | ✅ green |
| 43-09-01 | 09 | 9 | PACE-03, PACE-05 | T-43-33..T-43-35 | env-var name only; lazy connect; missing feature is a boot error | unit (#[serial] env) | `cargo test -p paladin-ai --lib config::treasurer && cargo test -p paladin-ai --features redis-cadence --lib infrastructure::cadence` | ✅ (extends) | ✅ green |
| 43-09-02 | 09 | 9 | PACE-02 | T-43-36 | one process-wide wiring; every server port paced | unit + build | `cargo test -p paladin-ai --features web-server --lib infrastructure::web && cargo build --bin paladin-server --features web-server,redis-cadence` | ✅ (extends) | ✅ green |
| 43-10-01 | 10 | 10 | PACE-04 | T-43-37, T-43-39 | owner-only unlock; fail-open lock | contract (paused clock) | `cargo test -p paladin-storage --lib cadence && cargo check --workspace --all-targets --all-features` | ✅ (extends) | ✅ green |
| 43-10-02 | 10 | 10 | PACE-04 | T-43-38, T-43-40, T-43-41 | stale fenced write ignored at the cache | contract (live Redis) | `NODE_CACHE_REDIS_TEST_URL=redis://<host>:<port>/<db> cargo test -p paladin-storage --features redis-cadence,redis-cache --lib -- --nocapture cadence node_cache` (env var named at validation: the node-cache suite reads `NODE_CACHE_REDIS_TEST_URL`, not `CADENCE_REDIS_TEST_URL`) | ✅ (extends) | ✅ green |
| 43-11-01 | 11 | 11 | PACE-04 | T-43-42..T-43-44 | node executes once; bounded waits; cancellation honoured | engine unit (paused clock) | `cargo test -p paladin-battalion --lib stampede_lock` | ✅ (stampede_lock_tests) | ✅ green |
| 43-11-02 | 11 | 11 | PACE-04 | -- | N/A (registers) | gate | `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ✅ green |
| 43-12-01 | 12 | 12 | PACE-01 | T-43-46 | only official sources count | evidence file check | `grep -c 'x-ratelimit-reset-requests' .planning/phases/43-rate-pacing/43-PROVIDER-HEADER-EVIDENCE.md` | ✅ (`43-PROVIDER-HEADER-EVIDENCE.md`, 2 hits) | ✅ green |
| 43-12-02 | 12 | 12 | PACE-01 | T-43-46 | operator confirms OpenAI names | checkpoint:human-verify | manual (see Manual-Only table) | -- | ✅ manual (operator attestation 2026-10-09, recorded as such in 43-PROVIDER-HEADER-EVIDENCE.md and 43-12-SUMMARY.md) |
| 43-12-03 | 12 | 12 | PACE-01 | T-43-47 | verified-source rustdoc; suite green | unit | `cargo test -p paladin-llm --all-features --lib` | ✅ | ✅ green |
| 43-13-01 | 13 | 13 | PACE-01..05 | T-43-48 | N/A (ADR, term table, changelogs) | gate | `./scripts/check-changelogs.sh && cargo test --test treasurer_vocabulary_guard` | ✅ | ✅ green |
| 43-13-02 | 13 | 13 | PACE-01..05 | T-43-48..T-43-50 | full gate + manual credential review | gate | `cargo test --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings && make security && make check-gates` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Every test file below is created inside the task that first needs it (TDD red-first); no separate
scaffold plan is required because the framework (`cargo test`, tokio, mockito, `MockLlmAdapter`) is
already installed.

- [x] `crates/paladin-ports/src/output/cadence_port.rs` test module -- policy table (PACE-02), owner 43-01 Task 1
- [x] `crates/paladin-storage/src/cadence/in_memory.rs` test module + `tokio` `test-util` dev-dependency -- paused-clock adapter tests (PACE-02), owner 43-01 Task 1
- [x] `src/infrastructure/cadence.rs` test module -- the tracer (PACE-02), owner 43-01 Task 2
- [x] `crates/paladin-ports/src/output/rate_limit_hints.rs` test module (PACE-01), owner 43-02 Task 1
- [x] `crates/paladin-llm/src/rate_limit_headers.rs` test module -- parse matrix (PACE-01), owner 43-03 Task 1
- [x] conformance case 10 + `CASE_COUNT` pin 10 (PACE-01/02), owner 43-04 Task 2
- [x] `crates/paladin-storage/src/cadence/contract_tests.rs` -- shared contract (PACE-02/03/04/05), owner 43-05 Task 1 (lock clauses 43-10 Task 1)
- [x] `crates/paladin-storage/src/cadence/redis.rs` live tests + CI job `redis-cadence-integration` (PACE-03), owner 43-07
- [x] `crates/paladin-storage/src/cadence/resilient.rs` tests with an injected failing port (PACE-05), owner 43-08 Task 1
- [x] engine `stampede_lock_tests` (PACE-04), owner 43-11 Task 1
- [x] Framework install: none needed (all dependencies present; `httpdate` already in `Cargo.lock`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| OpenAI rate-limit header names (`x-ratelimit-{limit,remaining,reset}-{requests,tokens}`), reset duration format, `Retry-After` semantics and the `insufficient_quota` code match the official OpenAI documentation | PACE-01 | the sandbox egress proxy blocks `platform.openai.com` and `developers.openai.com`; the official OpenAPI spec on GitHub does not document these headers (research A1/A2/A4) | 43-12 Task 2 checkpoint: open the official rate-limits guide and error-codes guide, compare against `43-PROVIDER-HEADER-EVIDENCE.md`, reply "approved" or list corrections |
| Live Redis suites run without `SKIP:` | PACE-03, PACE-04 | needs a Redis server (CI job `redis-cadence-integration` makes it mandatory; locally `redis-server --port 6391`) | `CADENCE_REDIS_TEST_URL=redis://127.0.0.1:6391/2 cargo test -p paladin-storage --features redis-cadence --lib cadence -- --nocapture` and check no `SKIP:` line |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (the one exception is the 43-12 Task 2 human-verify checkpoint, bracketed by automated tasks)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60 s per task command (plan-time figure; measured 2026-10-27 on a warm cache: 0-45 s for a targeted filter once the crate is built, 84-105 s on the first compile of a crate, 98 s for the full `paladin-llm --all-features --lib` suite)
- [x] `nyquist_compliant: true` set in frontmatter — plan-time value confirmed by `/gsd-validate-phase 43` on 2026-10-27 (`status` moved from `draft` to `validated`)

**Approval:** approved 2026-10-27 (`/gsd-validate-phase 43`)

---

## Validation Audit 2026-10-27

| Metric | Count |
|--------|-------|
| Tasks in map | 31 (30 automated + 1 `checkpoint:human-verify`, 43-12-02, manual-only by design) |
| Wave 0 files | 9 of 9 present (`cadence_port.rs`, `cadence/in_memory.rs`, `src/infrastructure/cadence.rs`, `rate_limit_hints.rs`, `rate_limit_headers.rs`, `cadence/contract_tests.rs`, `cadence/redis.rs`, `cadence/resilient.rs`, `43-PROVIDER-HEADER-EVIDENCE.md`); `mod stampede_lock_tests` and the `CASE_COUNT` pin present; CI job `redis-cadence-integration` present; all 5 named tests found by `fn` name |
| Verification targets re-run on the final tree | 29 distinct cargo targets + 6 gates: **all green**, including every live-Redis leg against a local `redis-server` (0 `SKIP:` lines); the one red target carries only the two pre-existing Phase 50 sandbox-deadline tests of WINDOWS.md row 73 |
| Gaps found | 0 |
| Resolved | 0 (none needed) |
| Escalated | 0 |
| Map corrections | 1 (below) |

**Re-run detail (`CARGO_INCREMENTAL=0`, warm cache, 2026-10-27, tree = `v0.11.0` + `.planning` docs):**

- `paladin-ports` (`cadence_port` 10, `rate_limit_hints` 7); `paladin-storage` (`cadence` 66, `cadence::resilient` 24); `paladin-llm` (`cadence` 35, `rate_limit_headers` 24, `fallback` 78; `--no-default-features` builds); `paladin-llm --all-features` (`openai` 218, `anthropic` 166, `compat` 178, `deepseek` 132, `gemini` 162, `rate_limit_is_surfaced_once_with_its_retry_delay` 10 — one per conformance case, pinned by `CASE_COUNT`; full `--lib` 1,525); `paladin-battalion --lib stampede_lock` 8; `paladin-ai` (`cadence_tracer_paces_a_real_openai_429_end_to_end`, `cadence_with_redis_down_still_paces`, `config::agent_runtime` 36, `config::treasurer` 49, `middleware::resilience` 15; `--test treasurer_vocabulary_guard` 3); `paladin-ai --features redis-cadence` (`infrastructure::cadence` 11); `cargo build --bin paladin-server --features web-server,redis-cadence` OK: **all green, 0 failed**.
- **Live Redis** (`redis-server --port 6391`, local, no Docker): `CADENCE_REDIS_TEST_URL=redis://127.0.0.1:6391/2 cargo test -p paladin-storage --features redis-cadence --lib cadence -- --nocapture` → 97 passed, 0 `SKIP:`; `degrades_within_the_timeout_budget` 2 passed; `cargo test -p paladin-ai --features redis-cadence --lib cadence_fleet -- --nocapture` → 1 passed, 0 `SKIP:`; `NODE_CACHE_REDIS_TEST_URL=redis://127.0.0.1:6391/3 ... --features redis-cadence,redis-cache --lib -- --nocapture cadence node_cache` → 128 passed (19 `node_cache::redis` live tests), 0 `SKIP:`. The Manual-Only "Live Redis suites run without `SKIP:`" item is therefore satisfied locally this time, not only in CI.
- `paladin-ai --features web-server --lib infrastructure::web` (43-09-02): 92 passed, **2 failed** — `build_run_api_persists_no_run_traces_by_default` and `build_run_api_persists_run_traces_when_trace_persist_is_set` (`run_api_wiring.rs:2369`), the WINDOWS.md row 73 pair (Phase 50, pre-existing sandbox 10 s worker deadline, green in CI). Not Phase 43 tests; the Phase 43 wiring tests in that module passed. Row 73 stays open with Phase 50.
- Gates: `check-migration-allowlist.sh` OK; `check-changelogs.sh` OK; `make check-gates` OK; `make security` exit 0; `grep -c x-ratelimit-reset-requests 43-PROVIDER-HEADER-EVIDENCE.md` = 2; `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean (134 s); `make api-surface` "unchanged" on this tree (Phase 41 audit, same day). `cargo test --workspace` on this tree: 7,719 passed / 2 failed (row 73) / 229 ignored (Phase 42 audit, same day).
- Manual-only: OpenAI header names rest on the operator's 2026-10-09 attestation, recorded as an attestation and never as a fetched quote (43-PROVIDER-HEADER-EVIDENCE.md, 43-12-SUMMARY.md); the credential-handling review is recorded in 43-13-SUMMARY.md.

**Map correction:** row 43-10-02's live node-cache command skipped under `CADENCE_REDIS_TEST_URL` alone (10 `SKIP:` lines at `redis://127.0.0.1:6380/0`) because `node_cache/redis.rs` reads `NODE_CACHE_REDIS_TEST_URL`; the command now names it. The plan-time sign-off ticks (`nyquist_compliant: true` at `status: draft`) were written before execution; this audit is the first run against the executed tree and confirms them.
