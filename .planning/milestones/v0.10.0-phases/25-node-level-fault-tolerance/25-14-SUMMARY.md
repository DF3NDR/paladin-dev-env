---
phase: 25-node-level-fault-tolerance
plan: 14
subsystem: docs
tags: [aegis, fault-tolerance, user-guide, migration-register, semver-checks, msrv, coverage, security-review, traceability]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance (plans 25-01 through 25-13)
    provides: "The whole Aegis surface this plan documents, registers and gates: Transience/NodeError/Aegis value types, the retry loop, EngineRegistries and the validation matrix, map_http_status, llm_failure, attempt history and the structured failure path, FallbackLlmAdapter + served_by, timeouts/heartbeat/RunTimeoutExceeded, Route/Absorb/Custom handlers, the E2E-3 replacement and stress binary, NodeCachePort + backends + engine integration"
provides:
  - "docs/src/user-guides/fault-tolerance.md -- the Aegis user guide (D-32), registered after the Parley page, every Rust sample compiled from crates/doc-examples/src/fault_tolerance.rs via mdBook {{#include}}"
  - "MIGRATION.md 9.1/9.2/9.3/9.4/9.5/9.7 resolved for Phase 25 (D-30): the PaladinPort default-method row (N), the Phase 25 deliberate-zero note, no-new-dependency + redis-cache feature, no SQL migration, NodeCacheConfig with every APP_NODE_CACHE_* var, run_timeout_secs landed, 9.7 confirmed empty"
  - "The 9.2 Y rows' Crate cell corrected to the published package name paladin-ai-core so ci.yml's allowlist set-equality step actually passes"
  - "CHANGELOG.md [Unreleased] entries for FT-01..FT-06 and the v5 fingerprint bump"
  - "08-traceability-matrix.md rows G-08 and G-10..G-14 anchored to 86 verified test functions"
  - "RedisNodeCacheConfig never Debug-prints redis_password (T-25-70 manual review outcome)"
  - "Recorded gate evidence on the phase's final commit: semver vs 0.9.0, MSRV 1.88, make security, clippy -D warnings, fmt, 89.34% workspace line coverage"
affects: [26-agent-runtime, 29-ship]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Guide samples live as ANCHOR regions in the paladin-doc-examples crate and are {{#include}}d, so a documented sample cannot drift from the landed API (cargo check -p paladin-doc-examples is the drift test)"
    - "A register's machine-checked column (MIGRATION.md 9.2 Crate for Y rows) uses the crates.io package name, because the CI check compares it by exact string against the allowlist"
    - "Gate evidence is recorded with command, exit code, figure and commit; a tier that cannot run locally is CI-only, never green"

key-files:
  created:
    - docs/src/user-guides/fault-tolerance.md
    - crates/doc-examples/src/fault_tolerance.rs
  modified:
    - docs/src/SUMMARY.md
    - crates/doc-examples/src/lib.rs
    - MIGRATION.md
    - CHANGELOG.md
    - .cargo/semver-checks-allowlist.toml
    - .project/v0.10.0/08-traceability-matrix.md
    - crates/paladin-storage/src/node_cache/redis.rs
    - .planning/phases/25-node-level-fault-tolerance/deferred-items.md

key-decisions:
  - "Guide samples are compiled examples, not doc tests: the guide's code lives in crates/doc-examples/src/fault_tolerance.rs (nine ANCHOR regions) and is pulled in with {{#include}}, matching orchestration.md/content-processing.md's established shape; the crate has doctest = false so cargo check -p paladin-doc-examples is the drift test"
  - "The 9.2 register's four FT-owned Y rows had their Crate cell corrected from paladin-core to paladin-ai-core (and the allowlist's migration_row mirrors updated): ci.yml's set-equality step compares the two files' crate names by exact string, and the reproduction of that step against the pre-fix tree FAILED (paladin-core vs paladin-ai-core) even though both sides counted five. A column note in 9.2 now states the rule."
  - "The PaladinResult row's Crate cell also moved to paladin-ai-core: the struct is defined in paladin_core::platform::container::execution_result and only re-exported by paladin-ports; the per-crate semver suppression lives in crates/paladin-core/Cargo.toml"
  - "RedisNodeCacheConfig's derived Debug over a raw redis_password was replaced by a redacting manual impl (Rule 2, security): the manual credential-handling review this plan mandates flagged it on code this phase added; the pre-existing RedisQueueConfig it mirrored is recorded in deferred-items, not changed"
  - "Coverage was measured with ci.yml's exact cargo llvm-cov flags run directly rather than through scripts/coverage.sh, which hard-fails on unreachable Redis/MinIO; under --features integration-tests,llm-all the Docker-dependent Redis-queue and S3 modules are feature-gated out (redis-queue / s3-storage), so the compiled scope is identical to CI's and no Docker-gated test ran or failed"
  - "The first cargo test --workspace run failed one Phase 24 wall-clock guard (parley_resume_stress::stress_run_completes_within_the_timeout_guard) under a ~7 load average; the standalone re-run and the full second workspace run both passed. Recorded as a flake finding in deferred-items rather than hidden; nothing in Phase 25 touches that test"

requirements-completed: [FT-01, FT-02, FT-03, FT-04, FT-05, FT-06]

coverage:
  - id: D1
    description: "A fault-tolerance guide exists, is registered after the Parley page, states all four limitations (idle-timeout degradation, Append replay hazard, raw-content error_field/Waypoint warning, R-23-01 still accepted), the node-kind matrix and the redact-then-bound rule; every sample compiles; the book builds"
    requirement: FT-01
    verification:
      - kind: other
        ref: "cargo check -p paladin-doc-examples (exit 0); cd docs && mdbook build (exit 0, 'No broken links found'); docs/src/SUMMARY.md line 25 links user-guides/fault-tolerance.md"
        status: pass
      - kind: other
        ref: "guide contains idle_timeout, Append, error_field, EdgeConditionEvaluator, Battalion, Gate; ! grep -qiE 'R-23-01 (is )?(now )?(closed|mitigated|resolved)' passes"
        status: pass
    human_judgment: false
  - id: D2
    description: "MIGRATION.md 9.2 holds the four FT-owned rows resolved plus a PaladinPort default-method row (N); Y rows and allowlist entries are set-equal in both directions by ci.yml's own script; the Phase 25 deliberate-zero note names every new-in-0.10 type touched; 9.1/9.3/9.4/9.5/9.7 resolved"
    requirement: FT-01
    verification:
      - kind: other
        ref: "scratchpad reproduction of ci.yml's set-equality step: SET-EQUAL PASS, Y rows 5, [[entry]] blocks 5 (FAIL before the crate-name fix)"
        status: pass
      - kind: other
        ref: "12/12 Task 2 acceptance greps pass (PaladinPort row, 'default method', deliberate zero naming StateNodeError + EngineRegistries, NodeCacheConfig, APP_NODE_CACHE_ENABLED, redis-cache, no 'plumbing-only this phase', 9.7 'no item', CHANGELOG Aegis, audit.toml unchanged)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Traceability rows G-08 and G-10..G-14 name only tests that exist in the named files"
    requirement: FT-02
    verification:
      - kind: other
        ref: "scratchpad verify_matrix_anchors.py parses the written rows: G-08 7, G-10 14, G-11 12, G-12 25, G-13 8, G-14 20 anchors; 86/86 resolve"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every X-10/X-11 gate is green on the phase's final commit with command, exit code and figure recorded (see Gate Evidence); the manual credential-handling review is performed and recorded; R-23-01 is re-listed as accepted; Redis/Postgres tiers are CI-only"
    requirement: FT-06
    verification:
      - kind: other
        ref: "Gate Evidence table below (semver 11/11 exit 0; MSRV exit 0; make security exit 0; clippy exit 0; fmt exit 0; cargo llvm-cov --fail-under-lines 82 exit 0 at 89.34%)"
        status: pass
      - kind: integration
        ref: "Redis node-cache tier (crates/paladin-storage/src/node_cache/redis.rs live-server tests) and Postgres Waypoint tier -- CI-only via redis-cache-integration / postgres-integration; self-skipped locally, never run against a server here"
        status: unknown
    human_judgment: false

duration: ~70min
completed: 2026-09-06
status: complete
---

# Phase 25 Plan 14: Aegis Guide, MIGRATION Register Close-Out and Gate Evidence Summary

**The phase is documented, registered and gated: a user guide covering the whole Aegis surface with its four real limitations and every sample compiled from the doc-examples crate; `MIGRATION.md` §9.1–§9.7 resolved for Phase 25 with a deliberate-zero note over every absent row and a register bug fixed that made CI's allowlist set-equality step fail (`paladin-core` vs `paladin-ai-core`); 86 verified traceability anchors; and every X-10/X-11 gate green on `462a1442` — semver clean against 0.9.0 for all eleven packages, MSRV 1.88 clean, `make security` clean, clippy/fmt clean, and 89.34% workspace line coverage against the 82% floor — plus one Rule 2 security hardening the mandated credential review surfaced.**

## Performance

- **Duration:** ~70 min (2026-09-06T02:16:40Z worktree spawn → 03:2xZ SUMMARY), across one session interruption after the first cargo runs were launched (no work lost; both runs completed and were read back)
- **Tasks:** 3 (all `auto`)
- **Files modified:** 10 (2 created, 8 modified)

## Accomplishments

### Task 1 — the guide (`c8d6aa57`)

- `docs/src/user-guides/fault-tolerance.md`, "Aegis: Retry, Timeout, Error Handlers, Model Fallback and Node Caching", in `parley-and-chronicle.md`'s shape (TOC, concept-before-API, tables, limitation callouts). Covers, in order: what an `Aegis` is and how it attaches (`set_aegis`/`with_default_aegis`, per-node-wins-wholesale, fail-closed validation); the `Transience` taxonomy and why a config error and a 503 must not retry identically; retry with the exact formula `min(initial_interval × backoff_factor^(n−2), max_interval) + jitter`, defaults and the three predicate arms; timeouts (wall clock vs progress-aware idle, what counts as progress, the nested run-level budget and `EngineRun`); handlers with the worked `book → cancel` compensation chain, validation rules, the loop bound, Custom-handler Parley and the worker-template rule; model fallback (hop rule, first-chunk rule, `FallbackHop`/`served_by`, circuit-breaker boundary); node caching (key composition, closed TTL boundary, hit/miss semantics, best-effort backend, the `NodeCacheConfig` env table); the D-12 node-kind matrix; the four limitations; the security notes (redact-then-bound, by-value classification, redacted `Debug`, trusted cached deltas).
- `crates/doc-examples/src/fault_tolerance.rs`: nine `ANCHOR` regions (`attach`, `transience`, `retry`, `custom_predicate`, `timeout`, `heartbeat`, `handlers`, `compensation`, `custom_handler`, `fallback`, `cache`) pulled into the guide with `{{#include}}` — the same mechanism `orchestration.md`/`content-processing.md` already use — so a sample cannot drift from the landed API.
- `docs/src/SUMMARY.md`: registered immediately after the Parley entry (line 25).

### Task 2 — the register (`9518c5d8`)

- **§9.2:** `PaladinPort` row — `execute_observed` default method, `N`, FT-FR-09. A Phase 25 deliberate-zero note in the Phase 23/24 form covering the `WarGraph` aegis sidecar and `validate`'s `EngineRegistries` signature, `NodeContext.attempt/heartbeat`, `NodeExecutionRecord.attempts/cache_hit`, `WaypointStatus::Failed.node_error`, the eighteen new `EngineError` variants, `TraceEvent`'s `attempt`/`cache_hit`/`FallbackHop`, `FieldSpec.cache` and the `StateNodeError` rename, ending with the same "read the absence as this note" sentence. **Register bug fixed:** the four FT-owned `Y` rows (25-02, 25-08) spelt the crate `paladin-core`; the allowlist (per its own schema) spells the published package `paladin-ai-core`; ci.yml's set-equality step compares the two by exact string and the verbatim local reproduction **failed** on the pre-fix tree despite both sides counting five. The cells now read `paladin-ai-core`, the `PaladinResult` row's cell too (the type is defined in `paladin-core` and only re-exported by `paladin-ports`; its suppression lives in `crates/paladin-core/Cargo.toml`), and a column note states the rule. Re-run: `SET-EQUAL: PASS`, 5 `Y` rows, 5 `[[entry]]` blocks. Still exactly five entries — nothing added, nothing suppressed.
- **§9.1:** a Phase 25 note — no behavioral change; every Aegis capability is opt-in per node in code, a v0.9 graph declares none, `run_timeout_secs` keeps `None`, `map_http_status` changes only which typed variant a status becomes; the two accepted caveats (idle-timeout degradation, R-23-01) named as caveats, not changes.
- **§9.3:** no new dependency, verified by manifest read (`rand` already direct on `paladin-battalion` — correcting D-30's "if not already"; `redis` already optional on `paladin-storage`; `safe_iterators` and the `blake3` edge are edits to existing lines; `Cargo.lock` gained one line and no package); the `redis-cache` feature and its facade passthrough, in no default set.
- **§9.4:** none — Redis keys and additive `#[serde(default)]` fields inside the JSON payload column, `BATTLEFIELD_SCHEMA_VERSION` unchanged; the fingerprint `v4 → v5` bump appended to the existing fingerprint bullet.
- **§9.5:** `NodeCacheConfig` mirroring the `WaypointStoreConfig` entry — every field, default and `APP_NODE_CACHE_*` var, off by default, `redis_password` never `Debug`-printed; `run_timeout_secs` updated from plumbing-only to landed (FT-03, D-20, 25-09) with its nesting semantics; Aegis policies add no config struct or env var (grep-verified).
- **§9.7:** confirmed empty — the legacy Battalion `RetryPolicy`/`ErrorStrategy`/`NodeError` and timeout handling remain undeprecated (X-03).
- **CHANGELOG.md `[Unreleased]`:** six `Added` entries (taxonomy, Aegis retry, timeouts + run budget, handlers, `FallbackLlmAdapter`, node cache) and a `Changed` entry for the `v5` fingerprint, in the file's existing style.
- **Traceability:** G-08 and G-10…G-14 anchored to the exact test functions and files (7 + 14 + 12 + 25 + 8 + 20 = 86 anchors), every one verified to exist by parsing the written rows; the Redis tier's anchors are labelled CI-only in the row itself.

### Task 3 — gate evidence and the review (`462a1442` + this SUMMARY)

See **Gate Evidence** and **Security** below. The one code change is the Rule 2 hardening in `crates/paladin-storage/src/node_cache/redis.rs`.

## Task Commits

| Task | Commit | Message |
|---|---|---|
| 1 | `c8d6aa57` | docs(25-14): add the Aegis fault-tolerance user guide with compiled examples |
| 2 | `9518c5d8` | docs(25-14): close out the MIGRATION.md register, CHANGELOG and traceability anchors |
| 3 | `462a1442` | fix(25-14): never Debug-print RedisNodeCacheConfig.redis_password |
| — | (this commit) | docs(25-14): complete the close-out plan (SUMMARY, deferred-items) |

All committed with `--no-verify` per `workflow.worktree_skip_hooks=true`; the hook's checks (`cargo fmt`, workspace clippy) were run explicitly below.

## Gate Evidence

Every gate below ran in this worktree against commit **`462a1442`** (the phase's final code commit; the SUMMARY commit that follows is docs-only). Disk was read before and after each heavy gate; it never dropped below 121 GB free (the 40 GB stop rule never triggered).

| # | Gate | Command (verbatim) | Exit | Measured |
|---|---|---|---|---|
| 1 | Workspace tests (ci.yml `test`) | `cargo test --workspace` | **0** (2nd run) | 41 binaries, 4413 passed / 0 failed / 226 ignored. **First run exit 101**: `parley_resume_stress::stress_run_completes_within_the_timeout_guard` (Phase 24, 10 s wall-clock guard) failed with the container at load ~7/8 cores; `cargo test --test parley_resume_stress` alone → 34/34 in 3.71 s; the full re-run → green. Recorded, not hidden (deferred-items). Disk 138 → 133 GB. |
| 2 | Named integration binaries | `cargo test --test e2e_muster_defer_order` / `--test e2e_compensation_chain` / `--test aegis_retry_stress` | 0 / 0 / 0 | 37 / 5 / 37 passed |
| 3 | Semver vs 0.9.0 (ci.yml `semver`, cargo-semver-checks **0.50.0**, the pin) | `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` for each of `paladin-ai paladin-ai-core paladin-ports paladin-battalion paladin-herald paladin-llm paladin-memory paladin-storage paladin-notifications paladin-content paladin-web` | **0 × 11** | every package "Summary no semver update required" (194–196 checks pass, 58–60 skip). The five registered deliberate-breaking lints are silenced at the tool level by the per-crate `[package.metadata.cargo-semver-checks.lints]` `allow` entries, so by construction they do not appear in the output; their register is §9.2 + the allowlist, set-equal by ci.yml's own script (**PASS**, 5 = 5). No unregistered break. Baseline crates were served from the local registry cache (all eleven `*-0.9.0.crate` present; crates.io also answered 200). Disk 133 → 128 GB. |
| 4 | Security | `make security` (= `cargo audit` + `cargo deny check`) | **0** | `advisories ok, bans ok, licenses ok, sources ok`. `cargo audit`: 0 vulnerabilities, "10 allowed warnings" — six `unmaintained` (all already listed in `deny.toml`), two `unsound` informational (`RUSTSEC-2026-0221` event-listener, `RUSTSEC-2026-0205` scc), two `yanked` (`chacha20 0.10.0`, `spin 0.9.8`). None is a vulnerability class; both tools pass under their configured policy. **`.cargo/audit.toml` unchanged** (`git diff --name-only HEAD -- .cargo/audit.toml` → 0 lines); no new suppression added. The unsound/yanked warnings are surfaced for the orchestrator (see below). |
| 5 | Coverage (ci.yml `coverage`, cargo-llvm-cov **0.8.7**, ADR-0006 floor) | `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines 82 -- --test-threads=1` (the exact command `scripts/coverage.sh` execs; the script itself is not runnable here because it hard-fails on unreachable Redis/MinIO before measuring) | **0** | **Lines 78442 / 87801 = 89.34 %** (functions 8043 / 9775 = 82.28 %), summed from `lcov.info` exactly as ci.yml's "Coverage summary" step does. 42 binaries, 4269 passed / 0 failed / 41 ignored, no Docker-gated test executed or failed: the Redis-queue module is `#[cfg(feature = "redis-queue")]` and S3 is `#[cfg(feature = "s3-storage")]`, neither in `integration-tests,llm-all`, so the instrumented scope equals CI's. Run **once**; `cargo llvm-cov clean --workspace` (exit 0) immediately after. Disk 128 → 123 → 127 GB. |
| 6 | MSRV (ci.yml `msrv`) | `cargo +1.88 check --workspace --all-features --all-targets` (rustc 1.88.0) | **0** | 3m41s. Disk 127 → 125 GB. |
| 7 | Clippy | `cargo clippy --workspace --all-targets --all-features -- -D warnings` | **0** | 0 warnings |
| 8 | Format | `cargo fmt --all -- --check` | **0** | — |
| 9 | Check | `cargo check --workspace --all-targets --all-features` | **0** | (also `cargo build --workspace --all-features --all-targets` exit 0, Task 2's acceptance, 6m48s) |
| 10 | Doc tests | `cargo test --workspace --doc` | **0** | 376 passed / 0 failed / 207 ignored across 12 crates (paladin 115, paladin_core 74, paladin_battalion 51, paladin_ports 119, paladin_llm 7, paladin_memory 10) |
| 11 | Modified crates | `cargo test -p paladin-storage --lib` / `--features redis-cache --lib` / `cargo test -p paladin-doc-examples --lib` / `cargo test --doc -p paladin-storage` / `--doc -p paladin-doc-examples` | 0 / 0 / 0 / 0 / 0 | 58 / 67 (incl. `debug_rendering_never_prints_the_password`) / 1 / 0 / 0 passed |
| 12 | mdBook | `mdbook-mermaid install docs/` (regenerates the gitignored assets, as `docs.yml` does) then `cd docs && mdbook build` | **0** | "No broken links found" |

**CI-only, never run locally (Docker absent, RESEARCH Environment Availability):** the Redis node-cache Tier-2 contract suite (`redis_node_cache_runs_the_full_contract_suite`, `redis_keys_are_namespaced_by_the_configured_prefix`, `redis_ttl_is_set_on_the_server_not_only_in_the_payload` — the `redis-cache-integration` job), the Postgres Waypoint contract tier (`postgres-integration`), the Redis-queue integration module (`docker-integration`), and `scripts/coverage.sh` in its service-backed form. Each self-skips or is feature-gated out here; none is reported as green.

## Security

**Manual credential-handling review (security.instructions.md; T-25-70) — performed on every path Phase 25 added that touches an API key or an external response body:**

| Check | Path | Finding |
|---|---|---|
| Response bodies redacted **before** truncation | `crates/paladin-llm/src/http_status.rs::map_http_status` | `redact_credentials(body, api_key)` (line 91) precedes `bounded_excerpt(&redacted, RESPONSE_EXCERPT_CHAR_BUDGET)` (line 92); the 400 overflow predicate reads the full redacted body, only the bounded excerpt is emitted; pinned by `excerpt_is_redacted_before_it_is_bounded`. All nine adapters route through it (25-05). `LlmFailure.message` and `NodeErrorSource::Llm` carry the source's `Display` of that already-redacted string. **Pass.** |
| No log statement interpolates an API key | `fallback.rs` (`record_hop` `warn!`: provider names, `transience`, the redacted error display; `debug!` on sink rejection), `superstep.rs` node-cache `warn!`/`debug!` (node id + `NodeCacheError` display), no other log lines in the new files | No key reaches a log. **Pass.** |
| No config type carrying a credential is `Debug`-formatted outward | `src/config/node_cache.rs::NodeCacheConfig` — manual `Debug`, `[REDACTED]`, pinned by test. **Pass.** `crates/paladin-storage/src/node_cache/redis.rs::RedisNodeCacheConfig` — **derived `Debug`** over `redis_password` (mirrored verbatim from the pre-existing `RedisQueueConfig`, D-27). Not `Debug`-formatted anywhere in-tree, but a public type any consumer could `{:?}`. **Fixed** in `462a1442` (Rule 2): manual `Debug`, `[REDACTED]`, `debug_rendering_never_prints_the_password`. Both types still derive `Serialize` (removing it is a semver-major trait removal) — recorded in deferred-items. |
| Credential-bearing HTTP clients do not follow redirects | Phase 25 added no HTTP client (the fallback adapter composes ports; the Redis cache speaks RESP). Of the existing adapters, the six Phase 17 ones and `CompatEngine` set `redirect::Policy::none()`; **`openai`, `anthropic` and `deepseek` still use reqwest's default follow policy** (pre-existing since v0.8, untouched by this phase — 25-05 changed only their non-2xx mapping). reqwest strips `Authorization` cross-host but not Anthropic's `x-api-key`. **Open finding, out of this close-out's scope**, recorded in deferred-items for Phase 26 RT-06 or a `fix(llm)`. |
| Cached deltas / keys | `engine/cache_key.rs` hashes model, system prompt, temperature, max loops, stop words — never an API key; the key embeds the graph fingerprint; the Redis key prefix is configuration (documented backend concern, PRD §5). **Pass.** |

**R-23-01 — accepted, not closed.** A hanging `EdgeConditionEvaluator` remains unbounded: the per-attempt `run_timeout`/`idle_timeout` and the run-level `EngineLimits.run_timeout` wrap *node execution*, not edge evaluation. `EngineLimits::max_supersteps` stays the run-level bound. Re-listed here, in the guide's "Limitations" section (item 4) and in MIGRATION.md §9.1's Phase 25 note, in `23-SECURITY.md`'s own wording, rather than claimed mitigated by the new timeout machinery (D-34, T-25-71).

**Advisories surfaced (no action taken, none is a vulnerability):** `cargo audit` reports two `unsound` informational advisories not in `.cargo/audit.toml` — `RUSTSEC-2026-0221` (`event-listener 5.4.1`) and `RUSTSEC-2026-0205` (`scc 2.4.0`) — and two `yanked` versions (`chacha20 0.10.0`, `spin 0.9.8`). Both tools exit 0 under the repo's configured policy (these classes warn, they do not fail), so the gate is green as configured; they are listed so the orchestrator can decide whether SECURITY-EXCEPTIONS.md should record them. No suppression was added by this plan (T-25-69).

**Threat register outcomes:** T-25-68 mitigated (semver 11/11 clean; set-equality now genuinely passes — and the pre-fix FAIL is exactly the silent-mismatch this threat describes); T-25-69 mitigated (`make security` 0, `audit.toml` unchanged); T-25-70 mitigated (review above, one fix); T-25-71 accepted (R-23-01 re-listed); T-25-72 mitigated (Redis/Postgres tiers recorded CI-only, never green); T-25-73 mitigated (86/86 anchors verified by parsing the written rows); T-25-SC accepted (no package installed; §9.3 records `rand`/`redis` already declared).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] §9.2 `Y` rows and the allowlist were not set-equal under ci.yml's own check**
- **Found during:** Task 2 (the plan's "confirm by counting, not by assuming" instruction, executed as a verbatim reproduction of the CI step)
- **Issue:** the four FT-owned rows spelt the crate `paladin-core`; the allowlist spells the crates.io package `paladin-ai-core`. Counts matched (5/5) but `diff -u` of the two crate-name sets failed — the `semver` job's set-equality step would fail on the merged tree.
- **Fix:** Crate cells → `paladin-ai-core` (three FT-01 rows and the `PaladinResult` row), allowlist `migration_row` mirrors updated, a column note in §9.2 states the rule. Entry count unchanged at five.
- **Files:** `MIGRATION.md`, `.cargo/semver-checks-allowlist.toml` — **Commit:** `9518c5d8`

**2. [Rule 2 - Missing critical functionality (security)] `RedisNodeCacheConfig` derived `Debug` over a raw password**
- **Found during:** Task 3's mandated manual credential-handling review
- **Issue:** a config type carrying a credential was `Debug`-derivable outward (security.instructions.md rule 2), unlike its sibling `NodeCacheConfig`.
- **Fix:** manual `Debug` rendering `redis_password` as `[REDACTED]`; test `debug_rendering_never_prints_the_password`. No API or semver impact (`Debug` output is not a semver surface; the trait is still implemented).
- **Files:** `crates/paladin-storage/src/node_cache/redis.rs` — **Commit:** `462a1442`

**3. [Rule 3 - Blocking] mdBook could not build: gitignored mermaid assets absent from the fresh worktree**
- **Fix:** `mdbook-mermaid install docs/`, exactly as `.github/workflows/docs.yml` does; the regenerated files stay gitignored. No committed change.

### Plan-text interpretations (no code deviation)

- **`scripts/coverage.sh` is not runnable locally** (it exits 1 on unreachable Redis/MinIO before measuring). The `cargo llvm-cov` command it `exec`s was run directly with identical flags; scope equivalence is argued and verified above (Docker-gated modules are feature-gated out of `integration-tests,llm-all`).
- **A first attempt to add `--no-fail-fast --ignore-run-fail`** (to guarantee an lcov even if a Docker-gated binary failed) was rejected by cargo-llvm-cov 0.8.7 as mutually exclusive before anything compiled; the single real run used ci.yml's exact flags. Coverage was therefore measured exactly once, per the orchestrator's rule.
- **`cargo test --workspace` ran twice** (not a coverage run): the first run's single failure was a pre-existing Phase 24 wall-clock guard under load; both runs are recorded in Gate Evidence.
- **`lcov.info` is a tracked file** and the coverage run overwrote it; it was restored with `git checkout -- lcov.info` rather than committed. Recorded in deferred-items.

## Issues Encountered

- The session was interrupted once while the Task 2 acceptance build and a storage test run were in flight; both finished on their own and were read back (exit 0 each), no work was lost and nothing was re-run needlessly.
- A `pgrep`-based wait loop matched its own command line and was replaced by log-sentinel waits.

## Known Stubs

None. The guide's samples are real, compiled code; no placeholder values, skipped tests or unrun `<verify>` steps were introduced by this plan. (`.planning/WINDOWS.md` was not touched from this worktree.)

## Threat Flags

None. No new network endpoint, auth path, file access pattern or schema at a trust boundary; the one code change narrows an existing type's `Debug` output.

## For the orchestrator's attention

1. **§9.2 crate-name fix (`9518c5d8`)** — the merged tree's `semver` job would otherwise fail its set-equality step; worth a glance since it edits rows two earlier plans wrote.
2. **Two `unsound` and two `yanked` advisories** surface in `cargo audit` as allowed warnings (details above); gate green as configured, decision on SECURITY-EXCEPTIONS.md is yours.
3. **Open review finding, out of scope:** openai/anthropic/deepseek clients follow redirects with a credential header (deferred-items).
4. **Flaky Phase 24 guard** `parley_resume_stress::stress_run_completes_within_the_timeout_guard` under load (deferred-items).
5. **`lcov.info` is tracked** — candidate `chore` to untrack and ignore (deferred-items).

## Self-Check: PASSED

- FOUND: `docs/src/user-guides/fault-tolerance.md`, `crates/doc-examples/src/fault_tolerance.rs`, `.planning/phases/25-node-level-fault-tolerance/25-14-SUMMARY.md`
- FOUND commits: `c8d6aa57`, `9518c5d8`, `462a1442`
- No tracked-file deletions across the plan's range; `lcov.info` restored; working tree clean before this SUMMARY.

---
*Phase: 25-node-level-fault-tolerance*
*Completed: 2026-09-06*
