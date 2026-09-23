---
phase: 31
slug: lossless-token-accounting
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-14
updated: 2026-09-15
validated: 2026-09-15
---

# Phase 31 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded by plan-phase from `31-RESEARCH.md` § Validation Architecture; the planner filled the
> Per-Task Verification Map from each plan's `<verify><automated>` blocks. Promoted to
> `validated` by `/gsd-validate-phase` on 2026-09-15 after re-running every task-level filter
> against the merged tree (see § Validation Audit 2026-09-15).

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (workspace, includes doctests) + `cargo llvm-cov` for the 82 % workspace line-coverage floor (ADR-0006; llvm-cov excludes doctests — run `cargo test --doc` separately) |
| **Config file** | none dedicated — workspace `Cargo.toml` + per-crate `[dev-dependencies]` (`mockito` for adapter harnesses, `tokio` test runtime) |
| **Quick run command** | the narrowest `cargo test -p <crate> <filter>` for the type just touched — e.g. `cargo test -p paladin-ai-core --lib token_usage`, `cargo test -p paladin-battalion --lib engine`, `cargo test -p paladin-llm --lib --all-features conformance`, `cargo test -p paladin-herald`, `cargo test -p paladin-web --lib agent_controller` |
| **Full suite command** | `cargo test --workspace --all-features --no-fail-fast && cargo fmt --check && cargo clippy --workspace -- -D warnings` (then, at the phase gate, `cargo llvm-cov --workspace --fail-under-lines 82`) |
| **Estimated runtime** | quick filter ~1-60 s warm (measured 2026-09-15: 0-7 s for most filters, 57 s for `paladin-battalion --lib engine`, 43 s for `paladin-ai --lib --features cli`); `cargo test --workspace` ~3-8 min warm; workspace clippy ~70-140 s warm (also run by the pre-commit hook); `cargo llvm-cov` ~10-20 min |

**`paladin-llm` feature-gate rule (added by the 2026-09-15 audit).** `paladin-llm`'s default
features are only `openai` + `mock`. The `anthropic`, `deepseek`, `gemini`, `openai-compatible`,
`grok`, `kimi`, `qwen` and `ollama` adapters — and every conformance-suite instance for them — are
compiled out unless the feature is on. A filter such as `cargo test -p paladin-llm --lib anthropic`
therefore **exits 0 having selected zero tests**, which reads as green. Every `paladin-llm`
command in this file MUST carry `--all-features` (or the specific `--features <adapter>`), and any
new row must be checked for a non-zero `test result: ok. N passed` line, not just exit status.

---

## Sampling Rate

- **After every task commit:** Run the task's `<verify><automated>` block — the narrowest `cargo test -p <crate> <filter>` above (the pre-commit hook additionally runs `cargo fmt --check` and workspace clippy on every commit)
- **After every plan wave:** Run `cargo test --workspace --all-features --no-fail-fast`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check` (`--no-fail-fast` because the pre-existing `cli_isolation` failure under `--all-features` — `deferred-items.md` — otherwise stops the run before most crate suites execute)
- **Before `/gsd-verify-work`:** Full suite must be green, plus the phase gate: `make clean-code`, `cargo llvm-cov --workspace --fail-under-lines 82`, `make security`, `cargo doc --workspace --no-deps` (zero warnings), `mdbook build docs/`, the `semver` CI job's two steps (per-package `cargo semver-checks check-release --baseline-version 0.9.0` + the row-level allowlist ↔ §9.2 set-equality), `make openapi` diff-clean, and the Python-client generation job
- **Max feedback latency:** ~60 s for a task-level filter; ~8 min for a wave-level full run

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 31-01-00 | 01 | 1 | ACCT-01 | — | N/A | checkpoint:decision | N/A — blocking human confirmation of the one-way wire/API shape (D-01/D-02/D-07/D-13/D-24); exempt from the automated-verify rule | N/A | ✅ resolved `proceed-as-decided` under auto-mode (31-01 SUMMARY § Decisions Made) |
| 31-01-01 | 01 | 1 | ACCT-01 | T-31-01 / T-31-02 / T-31-03 | Saturating arithmetic cannot wrap a hostile figure; no credential-shaped literal in the new doc examples | unit (tracer, TDD) | `cargo test -p paladin-ai-core --lib token_usage && cargo test -p paladin-ai-core --doc token_usage` | ✅ | ✅ green — 21 lib + 4 doc (2026-09-15) |
| 31-01-02 | 01 | 1 | ACCT-01 | T-31-02 | No fabricated `Some(0)` introduced while migrating ~96 literals | workspace build + suite | `cargo check --workspace --all-targets --all-features && cargo test --workspace --all-features --no-fail-fast && cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings` | ✅ | ✅ green — 6760 passed / 1 pre-existing `cli_isolation` failure (31-07 gate run; not re-run by this audit) |
| 31-02-01 | 02 | 2 | ACCT-02 | T-31-04 / T-31-05 | Poisoned-mutex recovery instead of a panicking `expect` in the run-total accumulator | integration (tracer, TDD) | `cargo test -p paladin-ai-core --lib platform::container && cargo test -p paladin-battalion --lib engine && cargo test -p paladin-battalion --lib formation_service && cargo test -p paladin-battalion --lib phalanx_service` | ✅ (`RecordingPaladinPort::set_output_with_usage`; D-30 round-trip / cache-hit / zero-node tests in `engine/mod.rs`) | ✅ green — 549 + 545 + 10 + 15 (2026-09-15) |
| 31-02-02 | 02 | 2 | ACCT-02 | T-31-06 | SSE payload gains numeric counts only; no new principal or content exposed | workspace build + suite | `cargo check --workspace --all-targets --all-features && cargo test --workspace --all-features --no-fail-fast && cargo fmt --check && cargo clippy --workspace --all-targets --all-features -- -D warnings` | ✅ | ✅ green — same 31-07 gate run as 31-01-02 |
| 31-02-03 | 02 | 2 | ACCT-02 | T-31-05 / T-31-07 | No legacy-shape deserializer; pre-phase rows report a default usage | unit (TDD) | `cargo test -p paladin-ai-core --lib token_usage && cargo test -p paladin-ai-core --lib execution_result && cargo test -p paladin-ai-core --lib waypoint && cargo test --workspace --all-features --no-fail-fast` | ✅ (`legacy_json_deserialises_with_default_usage`; `from_total` has 0 remaining references) | ✅ green — 21 + 9 + 40 (2026-09-15); workspace per 31-07 |
| 31-03-01 | 03 | 3 | ACCT-03 | T-31-08 / T-31-09 | Response bodies still redacted BEFORE truncation in every stream-parse error path; `diagnostic_excerpt` call count unchanged | integration, mockito (tracer, TDD) | `cargo test -p paladin-llm --lib --all-features compat && cargo test -p paladin-llm --lib --all-features openai_compatible && cargo test -p paladin-ports --lib llm_port && cargo test -p paladin-ports --doc` | ✅ harness exists (mockito; `map_compat_usage` in `compat/engine.rs`) | ✅ green — 92 + 43 + 21 + 137 (2026-09-15; command corrected, see audit) |
| 31-03-02 | 03 | 3 | ACCT-03 | T-31-09 / T-31-12 | No credential-shaped literal in the new provider fixtures | integration, mockito (TDD) | `cargo test -p paladin-llm --lib --all-features openai && cargo test -p paladin-llm --lib --all-features deepseek` | ✅ | ✅ green — 30 + 49 (2026-09-15; command corrected, see audit) |
| 31-03-03 | 03 | 3 | ACCT-03 | T-31-10 / T-31-11 | The absent-usage `warn!` names the provider only; no estimate substituted for a billed count | integration (TDD) | `cargo test -p paladin-ai --lib paladin_execution_service && cargo test --workspace --all-features --no-fail-fast && cargo clippy --workspace --all-targets --all-features -- -D warnings` | ✅ (`MockLlmAdapter::with_no_streamed_usage`) | ✅ green — 57 (2026-09-15); workspace + clippy per 31-07 |
| 31-04-01 | 04 | 4 | ACCT-03 | T-31-13 / T-31-16 | Redact-before-truncate intact in the Anthropic adapter; new non-zero-cache fixture carries no key-shaped literal | integration, mockito (tracer, TDD) | `cargo test -p paladin-llm --lib --all-features anthropic` | ✅ (`map_claude_usage`, `merge_claude_stream_usage`, dedicated parity stand-in test) | ✅ green — 35 (2026-09-15; command corrected, see audit) |
| 31-04-02 | 04 | 4 | ACCT-03 | T-31-14 / T-31-15 | No unbounded accumulation across an adversarial event stream | integration, mockito (TDD) | `cargo test -p paladin-llm --lib --all-features gemini` | ✅ (`map_gemini_usage`, `GeminiFixture`) | ✅ green — 79 (2026-09-15; command corrected, see audit) |
| 31-04-03 | 04 | 4 | ACCT-03 | T-31-13 | Documented exception states what a server-dependent adapter cannot guarantee | integration + docs build (TDD) | `cargo test -p paladin-llm --all-features conformance && mdbook build docs/` | ✅ (`conformance.rs` `CASE_COUNT == 9`, `streaming_usage_equals_non_streaming_usage`; mock + Anthropic stand-ins) | ✅ green — 82 conformance tests; mdbook exit 0 (2026-09-15; command corrected, see audit) |
| 31-05-01 | 05 | 5 | ACCT-04 | T-31-17 / T-31-19 | Rendered output gains numeric counts only; a coexisting bare total is asserted equal to the object | unit (tracer, TDD) | `cargo test -p paladin-herald --lib json_herald` | ✅ | ✅ green — 24 (2026-09-15) |
| 31-05-02 | 05 | 5 | ACCT-04 | T-31-17 / T-31-18 | Per-Paladin table row count unchanged; only the column count grows | unit (TDD) | `cargo test -p paladin-herald && cargo test -p paladin-ai --lib --features cli cli` | ✅ (`format_token_usage_summary` shared helper) | ✅ green — 50 + 210 (2026-09-15) |
| 31-05-03 | 05 | 5 | ACCT-04 | — | N/A | doc build + docs-currency grep | `cargo test --workspace --doc && mdbook build docs/ && cargo doc --workspace --no-deps && test -z "$(grep -rn 'token_count' docs/src --include='*.md' \| grep -v 'memory-management\.md' \| grep -v 'domain-model\.md' \| grep -v 'api-reference/')" && test -z "$(grep -n 'token_usage: TokenUsage' docs/src/user-guides/battalion-patterns.md)"` | ✅ | ✅ green — both greps empty, mdbook exit 0 (2026-09-15); 485 doctests + `cargo doc` exit 0 per 31-07 (77 pre-existing rustdoc warnings, out of scope — `deferred-items.md`) |
| 31-06-01 | 06 | 5 | ACCT-02, ACCT-05 | T-31-20 / T-31-21 / T-31-23 | Committed OpenAPI baseline regenerated in the same commit; no `utoipa` dependency crossed into `paladin-core` | integration + baseline (tracer, TDD) | `cargo test -p paladin-web --lib agent_controller && make openapi && cargo test -p paladin-web openapi_matches_committed_baseline && git diff --exit-code crates/paladin-web/openapi.json` | ✅ (`TokenUsageResponse`; `execute_response_exception_is_narrowly_scoped` in `openapi_golden_v0_9.rs`) | ✅ green — 40 + regen diff-clean + 1 (2026-09-15) |
| 31-06-02 | 06 | 5 | ACCT-02 | T-31-20 / T-31-22 / T-31-23 | No route, handler signature or auth middleware layering changed; `paladin-ports` gains no web dependency | integration (TDD) | `cargo test -p paladin-ports --lib run_inspector_port && cargo test -p paladin-ai --lib run::inspector && cargo test -p paladin-ai --lib run::events && cargo test -p paladin-web && cargo test --workspace --all-features --no-fail-fast` | ✅ (`node_finished_payload_carries_six_key_usage_object`, `run_finished_payload_carries_six_key_usage_object`) | ✅ green — 3 + 12 + 12 + 238 (2026-09-15); workspace per 31-07 |
| 31-07-01 | 07 | 6 | ACCT-05 | T-31-24 / T-31-25 | Lint ids derived empirically from the tool's output, never guessed | CI tool, local | `cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0 && cargo semver-checks check-release --package paladin-ports --default-features --baseline-version 0.9.0 && cargo semver-checks check-release --package paladin-web --default-features --baseline-version 0.9.0` | ✅ (`cargo-semver-checks` 0.50.0 installed; allowlist rows landed in `e59b094e`) | ✅ green — all three exit 0 (2026-09-15) but with `0 checks: 0 pass, 254 skip` each: at 0.9.0→0.10.0 the tool treats the step as major and evaluates nothing, so exit 0 alone is not evidence. Substantive evidence is 31-07 Task 1's `--release-type minor` run (lint ids `inherent_method_missing`, `constructible_struct_adds_field` observed) plus the allowlist ↔ §9.2 set-equality step (both directions empty diff, 31-07 SUMMARY) |
| 31-07-02 | 07 | 6 | ACCT-05 | T-31-24 | Changelog names the two corrected under-reports with the before/after formula | gate + docs build | `node /workspace/.claude/gsd-core/bin/gsd-tools.cjs query check api-coverage.verify-pre .planning/phases/31-lossless-token-accounting && mdbook build docs/` | ✅ | ✅ green — both exit 0 (2026-09-15) |
| 31-07-03 | 07 | 6 | ACCT-05 | T-31-08 / T-31-13 / T-31-26 | Manual credential-handling review of the phase diff recorded as the closing evidence for both high-severity threats | phase gate | `make clean-code && cargo test --workspace --all-features --no-fail-fast && cargo test --workspace --doc && make security && cargo doc --workspace --no-deps && mdbook build docs/ && cargo llvm-cov --workspace --fail-under-lines 82` | ✅ | ✅ green — 31-07 gate run: 6760/1-known, 485 doctests, `make security` exit 0, llvm-cov 90.30 % (≥ 82 floor); credential review recorded in 31-07 SUMMARY + 31-SECURITY.md (not re-run by this audit — llvm-cov alone is 10-20 min) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

**Existing infrastructure covers all phase requirements.** No new test framework or fixture
scaffolding is required — only new test CASES inside existing harnesses:

- [x] `crates/paladin-llm/src/conformance.rs` — `llm_conformance_suite!` + `ConformanceFixture` (per-adapter mockito harness) — the ACCT-03 parity case `streaming_usage_equals_non_streaming_usage` landed here as the ninth case (`CASE_COUNT == 9`)
- [x] `crates/paladin-battalion/src/engine/test_support.rs` — `RecordingPaladinPort` (gained `set_output_with_usage` for the ACCT-02 round-trip test)
- [x] `crates/paladin-llm/src/mock.rs` — `MockLlmAdapter::with_token_usage_struct` (streaming impls attach it to the terminal chunk) and the new `with_no_streamed_usage()` knob (D-16/D-17)
- [x] `cargo-semver-checks` 0.50.0 (CI pin) — installed locally (confirmed 2026-09-15), allowlist rows derived empirically (D-27)
- [x] `cargo-llvm-cov` — the coverage job's tool; `make coverage` locally (installed, confirmed 2026-09-15)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Provider streaming-usage frame shape against a LIVE provider (field names verified only via documentation/WebSearch this session — Context7 was unavailable) | ACCT-03 | Needs a real API key; mockito fixtures encode the documented shape, not a fresh capture. The existing `crates/paladin-llm/examples/live_vendor_smoke.rs` cannot substitute: it probes only the non-streaming `generate()` path, covers Kimi/Qwen/Grok/Gemini only, and is explicitly never run in CI. **Still outstanding as of 2026-09-15 — no plan SUMMARY recorded a live observation.** | With `OPENAI_API_KEY` / `ANTHROPIC_API_KEY` / `GEMINI_API_KEY` set, run one streamed `paladin-cli` execution per provider and confirm the terminal chunk's `usage` is `Some` with non-zero prompt AND completion; record the observation in `deferred-items.md` or the Phase 32 CONTEXT |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies — 19 of 20 rows carry a real command; the single exception (`31-01-00`) is a `checkpoint:decision`, which has no automated form by definition
- [x] Sampling continuity: no 3 consecutive tasks without automated verify — the longest gap is one task (`31-01-00`), immediately followed by two verified tasks
- [x] Wave 0 covers all MISSING references — RESEARCH.md § Wave 0 Gaps records none; every harness this phase needs (`llm_conformance_suite!`, `RecordingPaladinPort`, `MockLlmAdapter::with_token_usage_struct`, mockito, `cargo-semver-checks` 0.50.0, `cargo-llvm-cov`) already exists
- [x] No watch-mode flags — every command is a single-shot `cargo`/`make`/`mdbook`/`node` invocation
- [x] Feedback latency < 480s — per-task filters run in ~1-60 s warm and wave-level workspace runs in ~3-8 min. One deliberate exception: `31-07-03` is the phase gate itself and includes `cargo llvm-cov` (~10-20 min); it is the last task of the last wave, so no other task waits on it
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** planner sign-off 2026-09-15 — map filled from the seven plans' `<verify><automated>` blocks. **Validated 2026-09-15 by `/gsd-validate-phase`** — every task-level filter re-run against the merged tree at `d29f56fb`; `status: validated`.

---

## Validation Audit 2026-09-15

| Metric | Count |
|--------|-------|
| Requirements audited | 5 (ACCT-01 … ACCT-05) |
| Requirements COVERED | 5 |
| Requirements PARTIAL / MISSING | 0 |
| Gaps found | 5 (command-spec, not test-coverage) |
| Resolved | 5 (commands corrected in place; no new test files needed) |
| Escalated | 0 |
| Manual-only still outstanding | 1 (live-provider streaming usage observation, ACCT-03) |

**What the audit did.** Re-ran all 31 task-level commands from the map (every narrow
`cargo test` filter, both docs-currency greps, `make openapi` + baseline test + diff, the
`api-coverage.verify-pre` gate, `mdbook build`) against the merged tree. All exited 0. The
full-workspace, `llvm-cov`, `make security` and credential-review rows were NOT re-run; their
status cites plan 31-07's recorded gate evidence, which `31-VERIFICATION.md` (status: passed,
5/5) independently spot-checked.

**The five gaps.** Rows 31-03-01, 31-03-02, 31-04-01, 31-04-02 and 31-04-03 cited
`cargo test -p paladin-llm --lib <adapter>` without `--all-features`. Because `paladin-llm`'s
default features are only `openai` + `mock`, those filters exit 0 having selected **zero**
tests (`deepseek`, `anthropic`, `gemini`: 0 run) or a fraction (`compat` 2 of 92,
`openai_compatible` 2 of 43, `conformance` 19 of 82). The tests themselves exist and pass under
`--all-features` (49 / 35 / 79 / 92 / 43 / 82). The plans' executors ran the correct
`--all-features` form (31-04 SUMMARY: 516 `paladin-llm` lib tests), so no coverage was ever
lost — but the contract as written would have let a regression in those adapters read as
green. Fix: `--all-features` added to every `paladin-llm` command; feature-gate rule recorded
under § Test Infrastructure. No `gsd-nyquist-auditor` spawn was needed because no test was
missing — the workflow's "no gaps → Step 6" path applies to the test-coverage question.

**Not a Nyquist gap, noted for completeness.** Row 31-05-03's `cargo doc --workspace --no-deps`
exits 0 with 77 pre-existing rustdoc warnings (31-07 measured from a cold build); the plan's
"zero warnings" wording is unmet for out-of-scope reasons already logged in `deferred-items.md`.

**Second exit-0-while-checking-nothing shape, noted.** Row 31-07-01's literal
`cargo semver-checks check-release --baseline-version 0.9.0` reports `0 checks, 254 skip` on every
crate because the crates are already bumped to 0.10.0 (the tool classifies the step as major and
skips all lints). The CI `semver` job's second step — the row-level allowlist ↔ MIGRATION.md §9.2
set-equality comparison — is the step that actually gates ACCT-05, together with the
`api-coverage.verify-pre` check in row 31-07-02. Anyone re-running 31-07-01 locally must pass
`--release-type minor` to see real lint output, per 31-07 SUMMARY's recorded pattern.
