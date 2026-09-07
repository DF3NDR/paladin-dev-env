---
phase: 26-agent-runtime-enhancements
fixed_at: 2026-09-07T16:40:03Z
review_path: .planning/phases/26-agent-runtime-enhancements/26-REVIEW.md
iteration: 1
findings_in_scope: 5
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 26: Code Review Fix Report

**Fixed at:** 2026-09-07T16:40:03Z
**Source review:** .planning/phases/26-agent-runtime-enhancements/26-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 5 (CR-01, CR-02, WR-01, WR-02, WR-03 — `fix_scope: critical+warning`;
  IN-01 out of scope per the workflow config, and the WR-01 fix did not incidentally cover it)
- Fixed (behavioral): 4 (CR-01, CR-02, WR-01, WR-03)
- Documented (no re-architecture, per the workflow's own instruction for this finding): 1 (WR-02)
- Skipped: 0

## Fixed Issues

### CR-01: `ToolResultFormatter::format_result`'s error text bypassed redact-then-bound entirely

**Files modified:** `src/infrastructure/adapters/arsenal/tool_result_formatter.rs`
**Commit:** `bc2cd4f3`
**Applied fix:** Extracted a shared `sanitize_tool_text` helper (redact_secret_patterns then
bounded_excerpt, matching `format_error`'s existing sequence) and wired it into
`format_result`'s `Error:` branch (the `ArmamentResult { success: false, .. }` business-failure
path — the actual call site every `ArsenalPort::invoke` success takes). `format_error` itself was
refactored to call the same helper, so there is exactly one place tool-error text is sanitized,
matching the review's stated preference over a second inline duplicate. Combined with WR-03 in
one commit since both fixes land in the same function and share the new helper — splitting them
would have required either committing dead code or partially reverting the helper.

Verified with two new failing-first unit tests
(`format_result_redacts_a_secret_in_the_business_failure_error_before_bounding` and the WR-03
companion below), both passing after the fix; confirmed red beforehand via a background test run.

### WR-03: `ToolResultFormatter::format_result`'s success `Output:` field was also unredacted/unbounded

**Files modified:** `src/infrastructure/adapters/arsenal/tool_result_formatter.rs`
**Commit:** `bc2cd4f3` (same commit as CR-01 — see above)
**Applied fix:** The success branch's `Output:` field now also routes through
`sanitize_tool_text` before being embedded into the model-facing text and Garrison.

Verified with `format_result_redacts_a_secret_in_the_success_output_before_bounding` (new test,
confirmed red then green) plus the full existing 10-test `tool_result_formatter` suite (all still
pass, unmodified behavior for every non-credential-shaped case).

### CR-02: A `Finish` returned by a later `after_model` hook was silently dropped when the run was already finishing via an earlier `before_model` `Finish`

**Files modified:** `src/application/services/paladin/paladin_execution_service.rs`
**Commit:** `12c7562e`
**Applied fix:** The `BeforeOutcome::Finish` arm now captures `run_after`'s
`Option<FinalResult>` and falls back to the original `before_model` `Finish`'s `result` only when
`run_after` returns `None` (`.unwrap_or(result)`), then returns `effective_result.stop_reason`
instead of the original `result.stop_reason` — mirroring the already-correct sibling
post-model-call path 60 lines below, exactly as the review's suggested fix code specified.

Verified with a new failing-first test,
`after_model_finish_over_a_synthetic_view_overrides_the_before_model_finish`, using two new test
middlewares (`FinishBeforeMiddleware`, `FinishAfterMiddleware`) chained so an earlier
`before_model` `Finish` (mimicking `ModelCallLimit`) is followed by a later `after_model` `Finish`
(mimicking a `Guardrail` `Finish` rule, including mutating `resp.content` in place the same way
`Guardrail::apply_to_field` documents doing). Confirmed the test fails before the fix (asserted
`stop_reason == StopReason::CallLimit` was returned instead of `Completed`) and passes after.
Full 54-test `paladin_execution_service` unit suite re-run clean.

**Note for reviewer (verification limitation):** this is a control-flow fix (which `FinalResult`
wins), not a logic-heavy computation, and is covered by a new unit test that exercises the exact
scenario described in the review (an earlier `before_model` Finish followed by a later
`after_model` Finish with a different `stop_reason`), so it is recorded as `fixed`, not `fixed:
requires human verification`.

### WR-01: `redact_secret_patterns`'s `key=`/`token=` markers over-matched on ordinary words

**Files modified:** `crates/paladin-llm/src/redaction.rs`
**Commit:** `f26647a8`
**Applied fix:** `redact_token_after` now requires a word boundary immediately before `marker`:
the character before it (if any) must not be alphanumeric, or `marker` must be at the start of
the scanned text. Underscore is intentionally NOT treated as boundary-breaking, so the common
real-world spellings `api_key=`/`access_token=` are still redacted — only the review's cited
false-positive shapes (`monkey=`, `donkey=`, `turkey=`, `jockey=` immediately followed by `=`)
are excluded. Applied to the shared `redact_token_after` helper, so it also benefits the
`Bearer `/`sk-`/`AKIA` markers, with no observed regression (verified below).

Verified with a new failing-first test,
`redact_secret_patterns_does_not_misfire_on_ordinary_words_ending_in_key_or_token`, asserting
each benign word is left untouched AND that a real `api_key=<value>` occurring later in the same
text is still redacted (guards against the fix over-correcting into a blanket disable). Full
11-test `redaction` module suite and the full 420-test `paladin-llm` crate suite (covering every
provider adapter that calls `redact_credentials`/`redact_secret_patterns`/`diagnostic_excerpt`)
re-run clean — no regression in the existing `Bearer`/`sk-`/`AKIA`/JWT-triple coverage.

## Documented (not re-architected, per workflow instruction)

### WR-02: `execute_json_schema`/`execute_structured_call` bypass the entire `ExecutionMiddleware` chain

**Files modified:** `crates/paladin-ports/src/output/structured_executor_port.rs`,
`src/application/services/paladin/structured.rs`,
`src/application/services/paladin/paladin_execution_service.rs`,
`docs/src/user-guides/agent-runtime.md`
**Commit:** `45c05fbf`
**Applied fix:** Per this workflow's explicit instruction for WR-02 ("do NOT re-architect the
structured path... fix it as documentation"), no behavioral change was made. Added a prominent
rustdoc section to `StructuredExecutorPort` (the object-safe trait), to `StructuredExecutorExt`
(the typed extension), and to the concrete `impl StructuredExecutorPort for
PaladinExecutionService` block, each naming exactly which middleware classes
(`Guardrail`/`VaultRecallMiddleware`/`ToolCallLimit`/`TokenBudget`/`ModelCallLimit`/custom) are
silently inert on this surface and why. Added a matching paragraph to the "Structured Output"
section of `docs/src/user-guides/agent-runtime.md`. Recorded as `documented`, not `fixed` —
consistent with the workflow's own instruction to record this finding this way rather than
force a design change or leave it half-applied.

## Documentation drift follow-up

The behavioral fixes above (CR-01, WR-03, WR-01) widen what `M-B-03` (Phase 26's tool-error
sanitization change) documents. Added a `### Fixed` entry to `CHANGELOG.md`'s `[Unreleased]`
section and an amendment paragraph to `MIGRATION.md` §9.1's `M-B-03` row, both noting: (1)
sanitization now also covers `ArmamentResult`'s business-failure and success paths, not just the
`ArsenalError`/handoff `Err` path, and (2) the `key=`/`token=` marker's word-boundary fix. No
migration action is required — both changes narrow what gets sanitized correctly (add
sanitization where none existed; remove a false-positive class) without altering the
`tool_error_mode` policy itself. Committed together as `cfc664d5`.
`.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md`, `26-VERIFICATION.md`
and any `*-SUMMARY.md` were left untouched, per the workflow's explicit instruction.

## Final gates (all green, run against the full fix commit stack)

- `cargo fmt --all --check` — clean
- `cargo check --workspace --all-targets --all-features` — clean (4m 32s cold)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean (2m 18s cold)
- `cargo test -p paladin-ai --lib` — 723 passed, 0 failed
- `cargo test -p paladin-llm --all-features --lib` — 420 passed, 0 failed
- `cargo test -p paladin-ports --lib structured_executor_port` — 8 passed, 0 failed

---

_Fixed: 2026-09-07T16:40:03Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
