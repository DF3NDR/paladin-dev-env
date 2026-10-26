---
phase: 43
slug: rate-pacing
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-10-07
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
- **After every plan wave:** Run `cargo test --workspace` plus, when a Redis server is reachable, `cargo test -p paladin-storage --features redis-cadence,redis-cache --lib -- --nocapture cadence node_cache`
- **Before `/gsd-verify-work`:** Full suite must be green (43-13 Task 2 runs the whole gate and the live Redis suites)
- **Max feedback latency:** 60 seconds per task command

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 43-01-01 | 01 | 1 | PACE-02 | T-43-01, T-43-02 | delay math never panics on hostile numbers; state only on a 429 | unit (paused clock) | `cargo test -p paladin-ports --lib cadence_port && cargo test -p paladin-storage --lib cadence` | ❌ W0 (created in task) | ⬜ pending |
| 43-01-02 | 01 | 1 | PACE-02 | T-43-03, T-43-04, T-43-05 | first 429 surfaced once, decorator never retries, unknown config keys rejected | tracer (mockito end to end) | `cargo test -p paladin-ai --lib cadence_tracer_paces_a_real_openai_429_end_to_end` | ❌ W0 | ⬜ pending |
| 43-01-03 | 01 | 1 | PACE-02 | -- | N/A (registers) | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 43-02-01 | 02 | 2 | PACE-01 | T-43-07 | hints carry numbers only; Display unchanged | unit + workspace check | `cargo test -p paladin-ports --lib rate_limit_hints && cargo check --workspace --all-targets --all-features` | ❌ W0 | ⬜ pending |
| 43-02-02 | 02 | 2 | PACE-02 | T-43-06, T-43-08 | explicit delay is a minimum; delay beyond max_wait refused without sending or recording | unit (paused clock) | `cargo test -p paladin-llm --lib cadence` | ✅ (extends 43-01) | ⬜ pending |
| 43-02-03 | 02 | 2 | PACE-01 | -- | N/A (registers, D-27) | gate | `./scripts/check-migration-allowlist.sh && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 43-03-01 | 03 | 3 | PACE-01 | T-43-09, T-43-11 | bounded, panic-free parse; ceiling clamp | unit (table) | `cargo test -p paladin-llm --lib rate_limit_headers` | ❌ W0 | ⬜ pending |
| 43-03-02 | 03 | 3 | PACE-01, PACE-02 | T-43-10, T-43-12 | no raw header in errors; Anthropic first 429; quota 429 permanent | mockito | `cargo test -p paladin-llm --all-features --lib openai && cargo test -p paladin-llm --all-features --lib anthropic` | ✅ (extends) | ⬜ pending |
| 43-03-03 | 03 | 3 | PACE-01 | T-43-SC | dependency promotion audited | gate | `make security && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 43-04-01 | 04 | 4 | PACE-01, PACE-02 | T-43-13 | no adapter retries a 429 internally | mockito + unit | `cargo test -p paladin-llm --all-features --lib compat && cargo test -p paladin-llm --all-features --lib deepseek && cargo test -p paladin-llm --all-features --lib gemini` | ✅ (extends) | ⬜ pending |
| 43-04-02 | 04 | 4 | PACE-01, PACE-02 | T-43-13 | every fixture: 429 observed once with its delay | conformance | `cargo test -p paladin-llm --all-features --lib rate_limit_is_surfaced_once_with_its_retry_delay` | ❌ W0 (case 10) | ⬜ pending |
| 43-05-01 | 05 | 5 | PACE-02 | T-43-16, T-43-18 | bounded state; concurrent 429s escalate once | contract (paused clock) | `cargo test -p paladin-storage --lib cadence` | ❌ W0 (contract_tests.rs) | ⬜ pending |
| 43-05-02 | 05 | 5 | PACE-02 | T-43-17 | streams paced; spread never undercuts a provider delay | unit (paused clock) | `cargo test -p paladin-llm --lib cadence && cargo build -p paladin-llm --no-default-features` | ✅ (extends) | ⬜ pending |
| 43-05-03 | 05 | 5 | PACE-02 | -- | N/A (registers) | gate | `make security && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 43-06-01 | 06 | 6 | PACE-02 | T-43-20, T-43-21, T-43-22 | pace budget bounds the stay; hops still happen | unit (paused clock, SC2) | `cargo test -p paladin-llm --lib fallback` | ✅ (extends) | ⬜ pending |
| 43-06-02 | 06 | 6 | PACE-02 | -- | config-built chains paced by default | unit | `cargo test -p paladin-ai --lib config::agent_runtime && cargo test -p paladin-ai --lib application::services::paladin::middleware::resilience` | ✅ (extends) | ⬜ pending |
| 43-07-01 | 07 | 7 | PACE-03 | T-43-23..T-43-27 | URL redacted; scripts injection-free; bounded timeouts; TTL on every key | contract (live Redis) | `cargo test -p paladin-storage --features redis-cadence --lib cadence -- --nocapture` | ❌ W0 (redis.rs) | ⬜ pending |
| 43-07-02 | 07 | 7 | PACE-03 | T-43-26 | fleet gate from server clock | integration (live Redis) | `cargo test -p paladin-ai --features redis-cadence --lib cadence_fleet -- --nocapture` | ❌ W0 | ⬜ pending |
| 43-08-01 | 08 | 8 | PACE-05 | T-43-29..T-43-31 | never Err; one warning per outage; probe back-off | unit (paused clock, injected failing port) | `cargo test -p paladin-storage --lib cadence::resilient` | ❌ W0 (resilient.rs) | ⬜ pending |
| 43-08-02 | 08 | 8 | PACE-05 | T-43-29, T-43-32 | dead Redis degrades within the timeout budget; still paced | integration (no server needed) | `cargo test -p paladin-storage --features redis-cadence --lib degrades_within_the_timeout_budget && cargo test -p paladin-ai --lib cadence_with_redis_down_still_paces` | ❌ W0 | ⬜ pending |
| 43-09-01 | 09 | 9 | PACE-03, PACE-05 | T-43-33..T-43-35 | env-var name only; lazy connect; missing feature is a boot error | unit (#[serial] env) | `cargo test -p paladin-ai --lib config::treasurer && cargo test -p paladin-ai --features redis-cadence --lib infrastructure::cadence` | ✅ (extends) | ⬜ pending |
| 43-09-02 | 09 | 9 | PACE-02 | T-43-36 | one process-wide wiring; every server port paced | unit + build | `cargo test -p paladin-ai --features web-server --lib infrastructure::web && cargo build --bin paladin-server --features web-server,redis-cadence` | ✅ (extends) | ⬜ pending |
| 43-10-01 | 10 | 10 | PACE-04 | T-43-37, T-43-39 | owner-only unlock; fail-open lock | contract (paused clock) | `cargo test -p paladin-storage --lib cadence && cargo check --workspace --all-targets --all-features` | ✅ (extends) | ⬜ pending |
| 43-10-02 | 10 | 10 | PACE-04 | T-43-38, T-43-40, T-43-41 | stale fenced write ignored at the cache | contract (live Redis) | `cargo test -p paladin-storage --features redis-cadence,redis-cache --lib -- --nocapture cadence node_cache` | ✅ (extends) | ⬜ pending |
| 43-11-01 | 11 | 11 | PACE-04 | T-43-42..T-43-44 | node executes once; bounded waits; cancellation honoured | engine unit (paused clock) | `cargo test -p paladin-battalion --lib stampede_lock` | ❌ W0 (stampede_lock_tests) | ⬜ pending |
| 43-11-02 | 11 | 11 | PACE-04 | -- | N/A (registers) | gate | `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 43-12-01 | 12 | 12 | PACE-01 | T-43-46 | only official sources count | evidence file check | `grep -c 'x-ratelimit-reset-requests' .planning/phases/43-rate-pacing/43-PROVIDER-HEADER-EVIDENCE.md` | ❌ (created in task) | ⬜ pending |
| 43-12-02 | 12 | 12 | PACE-01 | T-43-46 | operator confirms OpenAI names | checkpoint:human-verify | manual (see Manual-Only table) | -- | ⬜ pending |
| 43-12-03 | 12 | 12 | PACE-01 | T-43-47 | verified-source rustdoc; suite green | unit | `cargo test -p paladin-llm --all-features --lib` | ✅ | ⬜ pending |
| 43-13-01 | 13 | 13 | PACE-01..05 | T-43-48 | N/A (ADR, term table, changelogs) | gate | `./scripts/check-changelogs.sh && cargo test --test treasurer_vocabulary_guard` | ✅ | ⬜ pending |
| 43-13-02 | 13 | 13 | PACE-01..05 | T-43-48..T-43-50 | full gate + manual credential review | gate | `cargo test --workspace && cargo clippy --workspace --all-targets --all-features -- -D warnings && make security && make check-gates` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Every test file below is created inside the task that first needs it (TDD red-first); no separate
scaffold plan is required because the framework (`cargo test`, tokio, mockito, `MockLlmAdapter`) is
already installed.

- [ ] `crates/paladin-ports/src/output/cadence_port.rs` test module -- policy table (PACE-02), owner 43-01 Task 1
- [ ] `crates/paladin-storage/src/cadence/in_memory.rs` test module + `tokio` `test-util` dev-dependency -- paused-clock adapter tests (PACE-02), owner 43-01 Task 1
- [ ] `src/infrastructure/cadence.rs` test module -- the tracer (PACE-02), owner 43-01 Task 2
- [ ] `crates/paladin-ports/src/output/rate_limit_hints.rs` test module (PACE-01), owner 43-02 Task 1
- [ ] `crates/paladin-llm/src/rate_limit_headers.rs` test module -- parse matrix (PACE-01), owner 43-03 Task 1
- [ ] conformance case 10 + `CASE_COUNT` pin 10 (PACE-01/02), owner 43-04 Task 2
- [ ] `crates/paladin-storage/src/cadence/contract_tests.rs` -- shared contract (PACE-02/03/04/05), owner 43-05 Task 1 (lock clauses 43-10 Task 1)
- [ ] `crates/paladin-storage/src/cadence/redis.rs` live tests + CI job `redis-cadence-integration` (PACE-03), owner 43-07
- [ ] `crates/paladin-storage/src/cadence/resilient.rs` tests with an injected failing port (PACE-05), owner 43-08 Task 1
- [ ] engine `stampede_lock_tests` (PACE-04), owner 43-11 Task 1
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
- [x] Feedback latency < 60 s per task command
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** {pending / approved YYYY-MM-DD}
