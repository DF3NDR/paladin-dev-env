---
phase: 50-provider-parity-config-driven-model-swap
fixed_at: 2026-10-17T00:00:00Z
review_path: .planning/phases/50-provider-parity-config-driven-model-swap/50-REVIEW.md
iteration: 1
fix_scope: critical_warning
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 50: Code Review Fix Report

**Fixed at:** 2026-10-17
**Source review:** `.planning/phases/50-provider-parity-config-driven-model-swap/50-REVIEW.md`
**Iteration:** 1
**Scope:** Critical and Warning (IN-01..IN-06 left alone)

**Summary:**
- Findings in scope: 6
- Fixed: 6
- Skipped: 0

All six commits are on `claude/laughing-dirac-e0h2ax` (not pushed). Work was done in an isolated worktree and
fast-forwarded onto the branch; the worktree, temp branch and recovery sentinel are gone. A seventh commit
(`004dcae7`) adds the paladin-llm CHANGELOG note.

## Fixed Issues

### CR-01: A vendor refusal is retried as if it were transient

**Status:** fixed
**Commit:** `5f2746fa`
**Files modified:** `crates/paladin-llm/src/anthropic/adapter.rs`, `crates/paladin-llm/src/compat/engine.rs`,
`crates/paladin-llm/src/deepseek/adapter.rs`, `crates/paladin-llm/src/gemini/adapter.rs`,
`crates/paladin-llm/src/conformance.rs`
**Applied fix:** Added `LlmError::Refusal { .. }` to the non-retryable set of the Anthropic, shared-engine,
DeepSeek and Gemini retry loops (the review named two; DeepSeek and Gemini carry the same set and also classify
inside the retried closure, so they had the same defect). OpenAI classifies outside its retry loop and needed no
change. Updated the "Non-retryable set" doc comment in `compat/engine.rs`. The conformance case
`refusal_is_typed_and_permanent` now registers its mock with `.expect(1)` and calls `assert_async()`. Red-first:
with only the test change, the five shared-engine fixtures (Kimi, Qwen, Grok, Ollama, OpenAI-compatible) failed;
all nine pass after the fix.

### CR-02: A streamed SSE event split across two network chunks is silently dropped

**Status:** fixed (EOF-without-terminal-frame behaviour deliberately not changed, see below)
**Commit:** `857cd9c1`
**Files modified:** new `crates/paladin-llm/src/sse.rs`, `crates/paladin-llm/src/lib.rs`,
`crates/paladin-llm/src/anthropic/adapter.rs`, `openai/adapter.rs`, `deepseek/adapter.rs`, `gemini/adapter.rs`,
`compat/engine.rs`, `crates/paladin-llm/src/conformance.rs`
**Applied fix:** New crate-private `SseLines` buffer: holds bytes until a line's newline arrives, decodes only
whole lines (so a multi-byte character split across reads is intact), strips `\r\n`, bounds a never-terminated
line at 4 MiB, and `finish()` releases a final line that had no trailing newline (needed so existing
no-trailing-newline bodies keep working). The byte stream is chained with an end-of-body marker so `finish()`
runs inside the same `flat_map` closure. Wired into all five streaming paths (Anthropic, OpenAI, DeepSeek,
Gemini via `parse_sse_lines`, shared engine). New conformance case
`stream_frames_split_across_reads_are_reassembled` serves each fixture's stream body in separate writes cut inside
the first and last `data:` payload; it failed for all nine adapters before the fix and passes after (run 3x for
flakiness). `CASE_COUNT` pinned test moved 17 to 18; case sits in the behaviour group so the transport-only
trivial fixture is unaffected. Unit tests in `sse.rs` cover split lines, multi-line reads, a split multi-byte
character, the final-line flush and the bound.
**Not done (decision for the owner):** the review's optional suggestion that a stream reaching EOF with no
terminal frame should yield `Err(ProcessingError)`. It changes behaviour for every adapter and could break
bodies that legitimately omit a terminal frame, so it was left as-is. The docs table in
`switching-providers.md` already states that Anthropic and Gemini do not classify a stream without a terminal
frame.

### WR-01: Explicit stop_words defeat the client-side StopWordDetected check; build refuses them

**Status:** fixed: requires human verification (semantic decision)
**Commit:** `2739c420`
**Files modified:** `src/application/services/paladin/paladin_builder.rs`,
`src/application/services/paladin/paladin_execution_service.rs`, `tests/provider_swap_matrix.rs`,
`crates/paladin-llm/src/grok/adapter.rs`, `crates/paladin-llm/src/sampling.rs`,
`crates/paladin-llm/src/registry/mod.rs`, `docs/src/user-guides/switching-providers.md`
**Decision recorded:** option 2 of the review, as directed. The client-side `check_stop_words` stays the source of
truth; `StopSupport::NotSent` is no longer a build refusal in `PaladinBuilder::validate`. The sampler already
omits an unsupported `stop` at send time and records it in `paladin.params_omitted`, so such an agent builds, runs,
and still aborts with `StopWordDetected`. A count above a numeric maximum (`Supported { max: Some(n) }`) is still
refused by name, which keeps the existing test for it; say if you want that relaxed too, since the sampler omits
that case at send time as well. ADR files were not edited.
**Known residual:** on a model that does accept `stop`, the stop words are still forwarded as wire
`stop_sequences`, so a vendor may end generation at the word and the client-side check will not fire. This is the
review's issue paragraph 3 and is not changed by option 2; option 1 (stop forwarding `Paladin.data.stop_words` at
the three call sites in the execution service) would close it and is a separate, larger decision. Documented in
the switching-providers parameter-policy table.
**Tests:** `stop_words_the_profile_does_not_take_are_refused_by_name` became
`stop_words_on_a_model_that_sends_no_stop_still_build`; new
`an_output_containing_a_stop_word_aborts_with_stop_word_detected` in the execution service; the
`provider_swap_matrix` case `*_same_agent_is_equivalent_or_refused_by_name` now expects no wire `stop` for a
`NotSent` model (temperature still asserted). The matrix (50 tests, `--features llm-all`) passes.

### WR-02: max_retries and timeout_seconds unvalidated; back-off overflows or sleeps for hours

**Status:** fixed
**Commit:** `946f5f86`
**Files modified:** new `crates/paladin-llm/src/backoff.rs`, `crates/paladin-llm/src/lib.rs`,
`crates/paladin-llm/src/openai/adapter.rs`, `compat/engine.rs`, `deepseek/adapter.rs`,
`crates/paladin-llm/src/config/layers.rs`, `crates/paladin-llm/src/config/llm.rs` (doc only)
**Applied fix:** New saturating `exponential_backoff(base, attempt, cap)` (checked shift and checked multiply,
capped at 10 s) used by the three loops; defaults (3 retries) are unchanged. `layers::resolve` now refuses
`timeout_seconds == 0` and `max_retries > 10` (`MAX_RETRIES_CEILING`), naming the provider and the key. Unit tests
for the helper (attempts up to `u32::MAX`) and for the refusals (block and environment sources, the ceiling itself
allowed).
**Limit:** validation is in the layered resolution path (factory/config). A caller building an adapter directly
with `with_max_retries(u32::MAX)` or the lenient `From<&LlmProviderConfig>` conversion is not refused, but the
back-off can no longer overflow or exceed 10 s per wait in that case.

### WR-03: live_vendor_smoke restates stale model constants; its own drift test fails

**Status:** fixed
**Commit:** `9594c640`
**Files modified:** `crates/paladin-llm/examples/live_vendor_smoke.rs`, `Makefile`
**Applied fix:** Imported `ANTHROPIC_DEFAULT_MODEL` and `DEEPSEEK_DEFAULT_MODEL` from the adapters and removed the
restated copies; `ANTHROPIC_MODELS` and `DEEPSEEK_MODELS` are built from them (each seeded table holds exactly the
default today) and the drift test fails if a table grows. `cargo test -p paladin-llm --example live_vendor_smoke
--features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible` now passes (24 tests,
including `the_static_model_lists_match_the_adapters`). Added a `make test-llm-examples` target so the example's
tests have a local entry point.
**Not done:** the review suggested running example tests in CI. `.github/workflows/ci.yml` was not edited; the
gap (CI never runs example tests) remains until someone wires `make test-llm-examples` into a job.

### WR-04: LlmProviderConfig redacts the key in Debug but still serialises it

**Status:** fixed
**Commit:** `fff2952f`
**Files modified:** `crates/paladin-llm/src/config/llm.rs`
**Applied fix:** `#[serde(default, skip_serializing)]` on `api_key`. Round-trip grep first: no
Serialize-then-Deserialize of `LlmProviderConfig`, `LlmConfig`, `Settings` or `AgentRuntimeDeps` exists in source
or tests (config is loaded from files and the environment via `config::Config`; the only serialisation test of the
block checks skipped empty fields). `skip_serializing` was chosen over `serialize_with` a placeholder because a
`"<redacted>"` string written to a dump and read back would become a literal credential; with skip, a round trip
yields an empty key that the provider's environment variable fills. New test `the_api_key_is_never_serialized`
covers the block, `LlmConfig`, and the round trip. `paladin-ai` config tests (251) and `consumer_surface_muster`
pass.

## Verification run

- `cargo fmt --check`: clean.
- `cargo clippy -p paladin-llm --all-targets --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-compatible,mock,vision -- -D warnings`: clean. Also `cargo clippy -p paladin-ai --all-targets` and the `provider_swap_matrix` test target (`--features llm-all`): clean. `cargo check` with single features (anthropic, gemini, openai, deepseek, kimi, ollama): no warnings.
- `cargo test -p paladin-llm --lib --features ...` (all adapters + mock + vision): 1137 passed, 0 failed.
- `cargo test --test unit llm`: 55 passed. `cargo test --test provider_swap_matrix --features llm-all`: 50 passed.
- Not run: the full workspace suite, `make api-surface` (no public item added or changed: `sse` and `backoff` are
  private modules, the new `MAX_RETRIES_CEILING` is `pub(crate)`), `make security`.

---

_Fixed: 2026-10-17_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
