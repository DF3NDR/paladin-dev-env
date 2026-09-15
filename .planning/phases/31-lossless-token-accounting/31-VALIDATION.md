---
phase: 31
slug: lossless-token-accounting
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-14
updated: 2026-09-15
---

# Phase 31 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded by plan-phase from `31-RESEARCH.md` § Validation Architecture; the planner fills the
> Per-Task Verification Map from each plan's `<verify><automated>` blocks.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace, includes doctests) + `cargo llvm-cov` for the 82 % workspace line-coverage floor (ADR-0006; llvm-cov excludes doctests — run `cargo test --doc` separately) |
| **Config file** | none dedicated — workspace `Cargo.toml` + per-crate `[dev-dependencies]` (`mockito` for adapter harnesses, `tokio` test runtime) |
| **Quick run command** | the narrowest `cargo test -p <crate> <filter>` for the type just touched — e.g. `cargo test -p paladin-core token_usage`, `cargo test -p paladin-battalion engine`, `cargo test -p paladin-llm conformance`, `cargo test -p paladin-herald`, `cargo test -p paladin-web openapi` |
| **Full suite command** | `cargo test --workspace --all-features && cargo fmt --check && cargo clippy --workspace -- -D warnings` (then, at the phase gate, `cargo llvm-cov --workspace --fail-under-lines 82`) |
| **Estimated runtime** | quick filter ~5-60 s warm; `cargo test --workspace` ~3-8 min warm; workspace clippy ~70-140 s warm (also run by the pre-commit hook); `cargo llvm-cov` ~10-20 min |

---

## Sampling Rate

- **After every task commit:** Run the task's `<verify><automated>` block — the narrowest `cargo test -p <crate> <filter>` above (the pre-commit hook additionally runs `cargo fmt --check` and workspace clippy on every commit)
- **After every plan wave:** Run `cargo test --workspace --all-features`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`
- **Before `/gsd-verify-work`:** Full suite must be green, plus the phase gate: `make clean-code`, `cargo llvm-cov --workspace --fail-under-lines 82`, `make security`, `cargo doc --workspace --no-deps` (zero warnings), `mdbook build docs/`, the `semver` CI job's two steps (per-package `cargo semver-checks check-release --baseline-version 0.9.0` + the row-level allowlist ↔ §9.2 set-equality), `make openapi` diff-clean, and the Python-client generation job
- **Max feedback latency:** ~60 s for a task-level filter; ~8 min for a wave-level full run

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 31-01-00 | 01 | 1 | ACCT-01 | — | N/A | checkpoint:decision | N/A — blocking human confirmation of the one-way wire/API shape (D-01/D-02/D-07/D-13/D-24); exempt from the automated-verify rule | N/A | ⬜ pending |
| 31-01-01 | 01 | 1 | ACCT-01 | T-31-01 / T-31-02 / T-31-03 | Saturating arithmetic cannot wrap a hostile figure; no credential-shaped literal in the new doc examples | unit (tracer, TDD) | `cargo test -p paladin-ai-core --lib token_usage && cargo test -p paladin-ai-core --doc token_usage` | ✅ (new cases in existing module) | ⬜ pending |
| 31-01-02 | 01 | 1 | ACCT-01 | T-31-02 | No fabricated `Some(0)` introduced while migrating ~96 literals | workspace build + suite | `cargo check --workspace --all-targets --all-features && cargo test --workspace --all-features && cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings` | ✅ | ⬜ pending |
| 31-02-01 | 02 | 2 | ACCT-02 | T-31-04 / T-31-05 | Poisoned-mutex recovery instead of a panicking `expect` in the run-total accumulator | integration (tracer, TDD) | `cargo test -p paladin-ai-core --lib platform::container && cargo test -p paladin-battalion --lib engine && cargo test -p paladin-battalion --lib formation_service && cargo test -p paladin-battalion --lib phalanx_service` | ✅ harness exists (`RecordingPaladinPort`) | ⬜ pending |
| 31-02-02 | 02 | 2 | ACCT-02 | T-31-06 | SSE payload gains numeric counts only; no new principal or content exposed | workspace build + suite | `cargo check --workspace --all-targets --all-features && cargo test --workspace --all-features && cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings` | ✅ | ⬜ pending |
| 31-02-03 | 02 | 2 | ACCT-02 | T-31-05 / T-31-07 | No legacy-shape deserializer; pre-phase rows report a default usage | unit (TDD) | `cargo test -p paladin-ai-core --lib token_usage && cargo test -p paladin-ai-core --lib execution_result && cargo test -p paladin-ai-core --lib waypoint && cargo test --workspace --all-features` | ✅ | ⬜ pending |
| 31-03-01 | 03 | 3 | ACCT-03 | T-31-08 / T-31-09 | Response bodies still redacted BEFORE truncation in every stream-parse error path; `diagnostic_excerpt` call count unchanged | integration, mockito (tracer, TDD) | `cargo test -p paladin-llm --lib compat && cargo test -p paladin-llm --lib openai_compatible && cargo test -p paladin-ports --lib llm_port && cargo test -p paladin-ports --doc` | ✅ harness exists (mockito) | ⬜ pending |
| 31-03-02 | 03 | 3 | ACCT-03 | T-31-09 / T-31-12 | No credential-shaped literal in the new provider fixtures | integration, mockito (TDD) | `cargo test -p paladin-llm --lib openai && cargo test -p paladin-llm --lib deepseek` | ✅ | ⬜ pending |
| 31-03-03 | 03 | 3 | ACCT-03 | T-31-10 / T-31-11 | The absent-usage `warn!` names the provider only; no estimate substituted for a billed count | integration (TDD) | `cargo test -p paladin-ai --lib paladin_execution_service && cargo test --workspace --all-features && cargo clippy --workspace --all-targets --all-features -- -D warnings` | ✅ mock adapter exists | ⬜ pending |
| 31-04-01 | 04 | 4 | ACCT-03 | T-31-13 / T-31-16 | Redact-before-truncate intact in the Anthropic adapter; new non-zero-cache fixture carries no key-shaped literal | integration, mockito (tracer, TDD) | `cargo test -p paladin-llm --lib anthropic` | ✅ captured fixtures exist | ⬜ pending |
| 31-04-02 | 04 | 4 | ACCT-03 | T-31-14 / T-31-15 | No unbounded accumulation across an adversarial event stream | integration, mockito (TDD) | `cargo test -p paladin-llm --lib gemini` | ✅ `GeminiFixture` exists | ⬜ pending |
| 31-04-03 | 04 | 4 | ACCT-03 | T-31-13 | Documented exception states what a server-dependent adapter cannot guarantee | integration + docs build (TDD) | `cargo test -p paladin-llm && mdbook build docs/` | ✅ `conformance.rs` + mdBook exist | ⬜ pending |
| 31-05-01 | 05 | 5 | ACCT-04 | T-31-17 / T-31-19 | Rendered output gains numeric counts only; a coexisting bare total is asserted equal to the object | unit (tracer, TDD) | `cargo test -p paladin-herald --lib json_herald` | ✅ | ⬜ pending |
| 31-05-02 | 05 | 5 | ACCT-04 | T-31-17 / T-31-18 | Per-Paladin table row count unchanged; only the column count grows | unit (TDD) | `cargo test -p paladin-herald && cargo test -p paladin-ai --lib cli` | ✅ | ⬜ pending |
| 31-05-03 | 05 | 5 | ACCT-04 | — | N/A | doc build | `cargo test --workspace --doc && mdbook build docs/ && cargo doc --workspace --no-deps` | ✅ | ⬜ pending |
| 31-06-01 | 06 | 5 | ACCT-02, ACCT-05 | T-31-20 / T-31-21 / T-31-23 | Committed OpenAPI baseline regenerated in the same commit; no `utoipa` dependency crossed into `paladin-core` | integration + baseline (tracer, TDD) | `cargo test -p paladin-web --lib agent_controller && make openapi && cargo test -p paladin-web --lib openapi_matches_committed_baseline && git diff --exit-code crates/paladin-web/openapi.json` | ✅ baseline test + `make openapi` exist | ⬜ pending |
| 31-06-02 | 06 | 5 | ACCT-02 | T-31-20 / T-31-22 / T-31-23 | No route, handler signature or auth middleware layering changed; `paladin-ports` gains no web dependency | integration (TDD) | `cargo test -p paladin-ports --lib run_inspector_port && cargo test -p paladin-ai --lib run::inspector && cargo test -p paladin-ai --lib run::events && cargo test -p paladin-web && cargo test --workspace --all-features` | ✅ | ⬜ pending |
| 31-07-01 | 07 | 6 | ACCT-05 | T-31-24 / T-31-25 | Lint ids derived empirically from the tool's output, never guessed | CI tool, local | `cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0 && cargo semver-checks check-release --package paladin-ports --default-features --baseline-version 0.9.0 && cargo semver-checks check-release --package paladin-web --default-features --baseline-version 0.9.0` | ✅ job exists; ❌ new allowlist rows | ⬜ pending |
| 31-07-02 | 07 | 6 | ACCT-05 | T-31-24 | Changelog names the two corrected under-reports with the before/after formula | gate + docs build | `node /workspace/.claude/gsd-core/bin/gsd-tools.cjs query check api-coverage.verify-pre .planning/phases/31-lossless-token-accounting && mdbook build docs/` | ✅ `COVERAGE.md` written at plan time | ⬜ pending |
| 31-07-03 | 07 | 6 | ACCT-05 | T-31-08 / T-31-13 / T-31-26 | Manual credential-handling review of the phase diff recorded as the closing evidence for both high-severity threats | phase gate | `make clean-code && cargo test --workspace --all-features && cargo test --workspace --doc && make security && cargo doc --workspace --no-deps && mdbook build docs/ && cargo llvm-cov --workspace --fail-under-lines 82` | ✅ targets exist | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

**Existing infrastructure covers all phase requirements.** No new test framework or fixture
scaffolding is required — only new test CASES inside existing harnesses:

- [x] `crates/paladin-llm/src/conformance.rs` — `llm_conformance_suite!` + `ConformanceFixture` (per-adapter mockito harness) — the ACCT-03 parity case is added here
- [x] `crates/paladin-battalion/src/engine/test_support.rs` — `RecordingPaladinPort` (gains a `set_output_with_usage` seam for the ACCT-02 round-trip test)
- [x] `crates/paladin-llm/src/mock.rs` — `MockLlmAdapter::with_token_usage_struct` (streaming impls attach it to the terminal chunk)
- [x] `cargo-semver-checks` 0.50.0 (CI pin) — install locally with `cargo install cargo-semver-checks --version 0.50.0 --locked` if absent, so allowlist rows are derived empirically (D-27)
- [x] `cargo-llvm-cov` — the coverage job's tool; `make coverage` locally

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Provider streaming-usage frame shape against a LIVE provider (field names verified only via documentation/WebSearch this session — Context7 was unavailable) | ACCT-03 | Needs a real API key; mockito fixtures encode the documented shape, not a fresh capture | With `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` / `GEMINI_API_KEY` set, run one streamed `paladin-cli` execution per provider and confirm the terminal chunk's `usage` is `Some` with non-zero prompt AND completion; record the observation in the plan SUMMARY |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — 19 of 20 rows carry a real command; the single exception (`31-01-00`) is a `checkpoint:decision`, which has no automated form by definition
- [x] Sampling continuity: no 3 consecutive tasks without automated verify — the longest gap is one task (`31-01-00`), immediately followed by two verified tasks
- [x] Wave 0 covers all MISSING references — RESEARCH.md § Wave 0 Gaps records none; every harness this phase needs (`llm_conformance_suite!`, `RecordingPaladinPort`, `MockLlmAdapter::with_token_usage_struct`, mockito, `cargo-semver-checks` 0.50.0, `cargo-llvm-cov`) already exists
- [x] No watch-mode flags — every command is a single-shot `cargo`/`make`/`mdbook`/`node` invocation
- [x] Feedback latency < 480s — per-task filters run in ~5-60 s warm and wave-level workspace runs in ~3-8 min. One deliberate exception: `31-07-03` is the phase gate itself and includes `cargo llvm-cov` (~10-20 min); it is the last task of the last wave, so no other task waits on it
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** planner sign-off 2026-09-15 — map filled from the seven plans' `<verify><automated>` blocks; `status` stays `draft` until `/gsd-validate-phase` promotes it
