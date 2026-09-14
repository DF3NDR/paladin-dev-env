---
phase: 31
slug: lossless-token-accounting
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-09-14
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
| 31-01-01 | 01 | 1 | ACCT-01 | T-31-01 / — | N/A | unit | `cargo test -p paladin-core --lib token_usage` | ✅ (new cases in existing module) | ⬜ pending |
| 31-0X-XX | — | — | ACCT-02 | — | N/A | integration | `cargo test -p paladin-battalion --lib engine` + `formation_service` + `phalanx_service` (D-30 round-trip + split regression) | ✅ harness exists (`RecordingPaladinPort`) | ⬜ pending |
| 31-0X-XX | — | — | ACCT-03 | — | Response bodies redacted before truncation in any new stream-parse error path | integration (mockito) | `cargo test -p paladin-llm streaming_usage_equals_non_streaming_usage` (new `llm_conformance_suite!` case, CASE_COUNT 8→9) + `cargo test -p paladin-ai --lib paladin_execution_service` | ✅ harness exists (`conformance.rs`) | ⬜ pending |
| 31-0X-XX | — | — | ACCT-04 | — | N/A | unit | `cargo test -p paladin-herald --lib json_herald` + `markdown_herald` | ✅ | ⬜ pending |
| 31-0X-XX | — | — | ACCT-05 | — | No credential-shaped literal in fixtures or docs | CI + local | `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` for `paladin-ai-core`, `paladin-ports`, `paladin-web`; the `ci.yml` allowlist awk comparison run locally; `make openapi` then `cargo test -p paladin-web --lib openapi`; `cargo llvm-cov --workspace --fail-under-lines 82` | ✅ jobs/targets exist | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*(The planner replaces the `31-0X-XX` placeholder rows with one row per task, carrying each task's real `<verify><automated>` command and threat reference.)*

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

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 480s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
