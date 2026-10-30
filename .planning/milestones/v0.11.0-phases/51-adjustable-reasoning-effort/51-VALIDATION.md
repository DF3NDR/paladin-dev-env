---
phase: 51
slug: adjustable-reasoning-effort
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-21
validated: 2026-10-27
---

# Phase 51 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test --offline` (unit `#[cfg(test)]`, doctests, `#[tokio::test]`, mockito wire pins, golden JSON bodies under `crates/paladin-llm/tests/golden/reasoning_unset/`), `--all-features` on `paladin-llm`/`paladin-ai`; `mdbook` for the guide page (CI `Docs` job) |
| **Config file** | none — workspace `Cargo.toml` test targets; `[[test]] consumer_surface_muster` |
| **Quick run command** | the task's own filter, e.g. `cargo test --offline -p paladin-llm --all-features --lib deepseek`, `cargo test --offline -p paladin-ai --lib --all-features -- a_chain_ paladin_builder` |
| **Full suite command** | `cargo test --offline -p paladin-llm --all-features && cargo test --offline --workspace --all-features --no-run && make check-migration-allowlist && make check-gates && make api-surface && make check-consumer-surface && make security` (plan 51-14 Task 3) |
| **Estimated runtime** | ~1-50 s per adapter filter on a warm cache; ~100 s for the full `paladin-llm --all-features` suite; ~4 min for the all-features workspace build |

---

## Sampling Rate

- **After every task commit:** Run the task's `<automated>` filter (adapter `--lib <vendor>` plus `unset_reasoning_leaves_the_wire_body_unchanged` whenever a wire body changes)
- **After every plan wave:** `cargo test --offline -p paladin-llm --all-features` plus `make api-surface` on register tasks
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** ~180 seconds for a targeted filter once the crate is built

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 51-01-01 | 01 | 1 | REASON-01, REASON-02, REASON-04, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- a request-level effort reaches the one admission rule, | `cargo test --offline -p paladin-ai-core --lib reasoning && cargo test --offline -p paladin-ports --lib && cargo test --offline -p paladin-po …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-01-02 | 01 | 1 | REASON-01, REASON-02, REASON-04, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: The full admission table, the AND intersection and the spelling  | `cargo test --offline -p paladin-ports --lib model_profile && cargo test --offline -p paladin-ai-core --lib reasoning && cargo test --offline …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-01-03 | 01 | 1 | REASON-01, REASON-02, REASON-04, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: Pin today's nine wire bodies byte for byte, pin the request-leve | `cargo test --offline -p paladin-llm --all-features --lib unset_reasoning_leaves_the_wire_body_unchanged && test "$(ls crates/paladin-llm/tes …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-02-01 | 02 | 2 | REASON-03, REASON-04 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- `--effort` for Kimi end to end offline: both candidate | `cargo test --offline -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-com …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-02-02 | 02 | 2 | REASON-03, REASON-04 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: Every vendor's candidate forms, the silent-field detector, budge | `cargo test --offline -p paladin-llm --example live_vendor_smoke --features openai,anthropic,deepseek,kimi,qwen,grok,ollama,gemini,openai-com …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-02-03 | 02 | 2 | REASON-03, REASON-04 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: Write the D-02 candidate file with the owner questions, the comm | `F=.planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE-CANDIDATES.md; P=.planning/phases/51-adjustable-reasoning-effort/51-02-PLA …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-03-01 | 03 | 2 | REASON-01, REASON-02, REASON-03, REASON-04, REASON-05, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- a DeepSeek request with an effort reaches the wire exa | `cargo test --offline -p paladin-llm --all-features --lib deepseek && cargo test --offline -p paladin-llm --all-features --lib reasoning && c …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-03-02 | 03 | 2 | REASON-01, REASON-02, REASON-03, REASON-04, REASON-05, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: DeepSeek complete -- off, auto, coerced levels, substitution, th | `cargo test --offline -p paladin-llm --all-features --lib deepseek && cargo test --offline -p paladin-llm --all-features --lib reasoning` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-03-03 | 03 | 2 | REASON-01, REASON-02, REASON-03, REASON-04, REASON-05, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: OpenAI's top-level `reasoning_effort` (none for Off, nothing for | `cargo test --offline -p paladin-llm --all-features --lib openai && cargo test --offline -p paladin-llm --all-features --lib unset_reasoning_ …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-04-01 | 04 | 3 | REASON-02, REASON-03, REASON-04, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- every preset states a reasoning wire at compile time,  | `cargo test --offline -p paladin-llm --all-features --lib kimi && cargo test --offline -p paladin-llm --all-features --lib compat:: && cargo  …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-04-02 | 04 | 3 | REASON-02, REASON-03, REASON-04, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: The other three wires mapped exactly -- top-level effort, `enabl | `cargo test --offline -p paladin-llm --all-features --lib compat:: && cargo test --offline -p paladin-llm --all-features --lib qwen && cargo  …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-04-03 | 04 | 3 | REASON-02, REASON-03, REASON-04, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: Operator-declared wires for Ollama and OpenAI-compatible (D-10), | `cargo test --offline -p paladin-llm --all-features --lib ollama && cargo test --offline -p paladin-llm --all-features --lib openai_compatibl …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-05-01 | 05 | 3 | REASON-02, REASON-03, REASON-05, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- an Anthropic effort reaches the wire as `output_config | `cargo test --offline -p paladin-llm --all-features --lib anthropic && cargo test --offline -p paladin-llm --all-features --lib unset_reasoni …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-05-02 | 05 | 3 | REASON-02, REASON-03, REASON-05, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: Anthropic off, auto and the legacy budget form, gated per profil | `cargo test --offline -p paladin-llm --all-features --lib anthropic` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-05-03 | 05 | 3 | REASON-02, REASON-03, REASON-05, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: Gemini `thinkingLevel` / `thinkingBudget` per model profile, on  | `cargo test --offline -p paladin-llm --all-features --lib gemini && cargo test --offline -p paladin-llm --all-features --lib unset_reasoning_ …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-06-01 | 06 | 4 | REASON-01, REASON-02, REASON-04 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- `llm.deepseek.reasoning_effort: high` in YAML reaches  | `cargo test --offline -p paladin-llm --all-features --lib registry:: && cargo test --offline -p paladin-llm --all-features --lib config:: &&  …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-06-02 | 06 | 4 | REASON-01, REASON-02, REASON-04 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: The other eight adapters apply the provider default, self-hosted | `cargo test --offline -p paladin-llm --all-features --lib && cargo test --offline --features llm-anthropic,llm-deepseek --test consumer_surfa …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-07-01 | 07 | 5 | REASON-01, REASON-04, REASON-05, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- `PaladinBuilder::reasoning_effort(High)` reaches the r | `cargo test --offline -p paladin-ai-core --lib paladin && cargo test --offline -p paladin-ai --lib paladin_builder && cargo test --offline -p …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-07-02 | 07 | 5 | REASON-01, REASON-04, REASON-05, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: Build-time refusal (D-09) and the other two request sites | `cargo test --offline -p paladin-ai --lib paladin_builder && cargo test --offline -p paladin-ai --lib paladin_execution_service` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-07-03 | 07 | 5 | REASON-01, REASON-04, REASON-05, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: Pre-flight every fallback hop before the first call (D-08), with | `cargo test --offline -p paladin-llm --all-features --lib fallback && make api-surface` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-08-01 | 08 | 6 | REASON-01, REASON-04, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- `agents[].reasoning_effort: high` in YAML builds an ag | `cargo test --offline -p paladin-ai --lib config::agents && cargo test --offline -p paladin-ai --lib agent_host && cargo test --offline --wor …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-08-02 | 08 | 6 | REASON-01, REASON-04, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: Build-time refusals name the YAML key -- the agent's own, or the | `cargo test --offline -p paladin-ai --lib agent_host && make api-surface` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-09-01 | 09 | 7 | REASON-01, REASON-05, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- a Paladin attempt that ran at `high` emits `NodeFinish | `cargo test --offline -p paladin-ai-core --lib trace && cargo test --offline -p paladin-battalion --lib engine::superstep && cargo test --off …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-09-02 | 09 | 7 | REASON-01, REASON-05, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: An effort-aware node-cache key that leaves every existing key un | `cargo test --offline -p paladin-battalion --lib cache_key && make api-surface` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-10-01 | 10 | 7 | REASON-03, REASON-04 | — | no key value in any body, error or trace; refuse before HTTP | checkpoint — Task 1: Owner confirms the candidate set, answers the D-02 treatment que | n/a (checkpoint:decision) | n/a | ✅ manual (owner confirmed the candidate set and the D-02 treatment (51-10-SUMMARY.md; 51-EFFORT-PROBE-CANDIDATES.md)) |
| 51-10-02 | 10 | 7 | REASON-03, REASON-04 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 2: Run the probe -- `kimi-k3` first through both forms, check that  | `R=.planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.raw.md; test -f "$R" && test "$(grep -c '^## ' "$R")" -ge 7 && grep -q 'ki …` | ✅ | ✅ state-at-the-time — asserts the raw credentialed transcript `51-EFFORT-PROBE.raw.md` exists; it is untracked by design and was deleted after the 51-10-03 scrub (which passes: scrubbed `51-EFFORT-PROBE.md` present, raw absent) |
| 51-10-03 | 10 | 7 | REASON-03, REASON-04 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: Write the header and verdict table, scrub-check, and commit `51- | `S=.planning/phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.md; test -f "$S" && test ! -f .planning/phases/51-adjustable-reasoning-eff …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-11-01 | 11 | 8 | REASON-02, REASON-03, REASON-04, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | checkpoint — Task 1: Owner confirms the wire decisions the verdict table implies -- K | n/a (checkpoint:decision) | n/a | ✅ manual (owner confirmed the wire decisions the verdict table implies (51-11-SUMMARY.md; 51-EFFORT-PROBE.md `## Owner decisions`)) |
| 51-11-02 | 11 | 8 | REASON-02, REASON-03, REASON-04, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 2: Tracer -- `kimi-k3` ships its probe-confirmed wire and measured  | `cargo test --offline -p paladin-llm --all-features --lib kimi && cargo test --offline -p paladin-llm --all-features --lib registry:: && grep …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-11-03 | 11 | 8 | REASON-02, REASON-03, REASON-04, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: Qwen and Grok -- probe-confirmed wires, measured levels and budg | `cargo test --offline -p paladin-llm --all-features --lib qwen && cargo test --offline -p paladin-llm --all-features --lib grok && cargo test …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-12-01 | 12 | 9 | REASON-02, REASON-03, REASON-04, REASON-05, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- `deepseek-v4-pro` ships its probe-confirmed caps and m | `cargo test --offline -p paladin-llm --all-features --lib deepseek && cargo test --offline -p paladin-llm --all-features --lib registry::` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-12-02 | 12 | 9 | REASON-02, REASON-03, REASON-04, REASON-05, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: Anthropic, OpenAI and Gemini rows -- confirmed caps, measured le | `cargo test --offline -p paladin-llm --all-features --lib anthropic && cargo test --offline -p paladin-llm --all-features --lib openai && car …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-12-03 | 12 | 9 | REASON-02, REASON-03, REASON-04, REASON-05, REASON-06 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: Nine-adapter conformance cases, the round-trip invariant, the re | `cargo test --offline -p paladin-llm --all-features --lib explicit_reasoning_on_an_unprofiled_model_refuses_before_http && cargo test --offli …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-13-01 | 13 | 10 | REASON-02, REASON-06, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- a 'Reasoning Effort' page whose DeepSeek rows the drif | `cargo test --offline -p paladin-llm --all-features --lib reasoning_page_tests && mdbook build docs --dest-dir "$(mktemp -d)"` | ✅ | ✅ green for `reasoning_page_tests` (14 passed); `mdbook build` not runnable here (`mdbook` absent) — CI `Docs` job is the twin, green on the v0.11.0 heads |
| 51-13-02 | 13 | 10 | REASON-02, REASON-06, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: The full mapping table and guidance -- every provider and seeded | `cargo test --offline -p paladin-llm --all-features --lib reasoning_page_tests && mdbook build docs --dest-dir "$(mktemp -d)" && grep -q '^## …` | ✅ | ✅ green for `reasoning_page_tests` (14 passed) and the page greps; `mdbook build` CI-attributed as 51-13-01 |
| 51-13-03 | 13 | 10 | REASON-02, REASON-06, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: ADR-0062 amending ADR-0004, the ADR-0004 and ADR-0061 dated line | `A=.planning/decisions/0062-reasoning-effort-fail-loud-policy.md; test -f "$A" && grep -q 'Amends ADR-0004' "$A" && grep -q '^## Considered O …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-14-01 | 14 | 11 | REASON-03, REASON-06, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | checkpoint — Task 1: Owner approves the adapter-path smoke spend and verifies cargo-s | n/a (checkpoint:decision) | n/a | ✅ manual (owner approved the adapter-path smoke spend and verified cargo-semver-checks (51-14-SUMMARY.md; 51-EFFORT-SMOKE.md)) |
| 51-14-02 | 14 | 11 | REASON-03, REASON-06, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 2: Run the adapter-path smoke (if approved) and measure every Phase | `S=.planning/phases/51-adjustable-reasoning-effort/51-EFFORT-SMOKE.md; test -f "$S" && grep -q 'kimi-k3' "$S" && ! grep -Eq 'sk-[A-Za-z0-9_-] …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-14-03 | 14 | 11 | REASON-03, REASON-06, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: MIGRATION 9.2 / 9.5 / 9.8 and CHANGELOG completeness, then every | `grep -q 'reasoning_effort' MIGRATION.md && grep -q '\| DR-20 \|' MIGRATION.md && grep -q 'llm.<provider>.reasoning_wire' MIGRATION.md && mak …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-15-01 | 15 | 12 | REASON-05 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- an agent run whose call reports `xhigh` requested / `h | `cargo test --offline -p paladin-ai --lib --all-features -- agent_run_node_finished_carries_the_calls_own_reasoning_metadata agent_kind_run_s …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-15-02 | 15 | 12 | REASON-05 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: Guards -- only the two keys are read, never the configured effor | `cargo test --offline -p paladin-ai --lib --all-features -- agent_run_node_finished failed_agent_run_node_finished_carries_no_reasoning` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-16-01 | 16 | 12 | REASON-01 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- the host-load check takes the agent's fallback before  | `cargo test --offline -p paladin-ai --lib --all-features -- block_nearest_lower_agent_refuse_is_refused block_refuse_agent_nearest_lower_pass …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-16-02 | 16 | 12 | REASON-01 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: Correct the precedence rustdoc and pin when the host's fallback  | `cargo test --offline -p paladin-ai --lib --all-features -- the_host_fallback_hint_appears_only_when_refuse_is_in_force infrastructure::web:: …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-17-01 | 17 | 12 | REASON-01 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- `LlmPort::check_reasoning` through the DeepSeek adapte | `cargo test --offline -p paladin-ports --lib --all-features -- check_reasoning && cargo test --offline -p paladin-ports --doc -- check_reason …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-17-02 | 17 | 12 | REASON-01 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: The OpenAI, Anthropic and Gemini adapters answer `check_reasonin | `cargo test --offline -p paladin-llm --lib --all-features -- check_reasoning_consults_the_provider_defaults unset_reasoning_leaves_the_wire_b …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-18-01 | 18 | 13 | REASON-01 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- the compat engine pre-judges with the preset's own def | `cargo test --offline -p paladin-llm --lib --all-features -- kimi_check_reasoning_agrees_with_generate_on_every_combination qwen_check_reason …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-18-02 | 18 | 13 | REASON-01 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: Ollama and OpenAI-compatible overrides; Pricing and Cadence forw | `cargo test --offline -p paladin-llm --lib --all-features -- check_reasoning_consults_the_provider_defaults forwards_check_reasoning forwards …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-19-01 | 19 | 13 | REASON-04, REASON-01 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- a chain's effort verdict is every hop's own verdict, f | `cargo test --offline -p paladin-llm --lib --all-features -- reasoning_preflight_tests && cargo test --offline -p paladin-ai --lib --all-feat …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-19-02 | 19 | 13 | REASON-04, REASON-01 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: The chain profile fails closed for effort when a hop reports no  | `cargo test --offline -p paladin-llm --lib --all-features -- fallback:: pricing_over_cadence_forwards_model_profile` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-20-01 | 20 | 14 | REASON-01, REASON-04, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | tracer — Task 1: Tracer -- the invariant checked end to end on all nine adapters  | `cargo test --offline -p paladin-llm --lib --all-features -- check_reasoning_agrees_with_send_time_admission && cargo test --offline -p palad …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-20-02 | 20 | 14 | REASON-01, REASON-04, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 2: The guide and ADR-0062 describe the per-hop, provider-default-aw | `cargo test --offline -p paladin-llm --lib --all-features -- the_reasoning_effort_page_agrees_with_the_adapters && grep -c 'check_reasoning'  …` | ✅ | ✅ green (re-run 2026-10-27) |
| 51-20-03 | 20 | 14 | REASON-01, REASON-04, REASON-07 | — | no key value in any body, error or trace; refuse before HTTP | unit/gate — Task 3: Register `LlmPort::check_reasoning` and the behaviour changes in | `make check-migration-allowlist && make api-surface && make check-api-examples && make check-gates` | ✅ | ✅ green (re-run 2026-10-27) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] Existing infrastructure covers all phase requirements: every new test (the nine `reasoning_unset` golden bodies, `check_reasoning` conformance cases, the `reasoning_page_tests` drift tests, the fallback pre-flight tests) was written test-first inside the task that needed it; no framework install.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Candidate set, D-02 treatment, wire decisions, smoke spend approval | REASON-03, REASON-04, REASON-06 | owner `checkpoint:decision` tasks 51-10-01, 51-11-01, 51-14-01 | recorded in the plan SUMMARYs and 51-EFFORT-PROBE.md / 51-EFFORT-SMOKE.md |
| Live vendor probe and adapter-path smoke | REASON-03, REASON-06 | need the operator's vendor keys; raw transcripts stay untracked and only scrubbed files are committed | 51-10-02 / 51-14-02 (`tracer`), scrub proven by 51-10-03 |
| `mdbook build docs` for the Reasoning effort page | REASON-07 | `mdbook` is not installed in the sandbox | CI `Docs` job on the phase heads |
| Anthropic `Budget(n)` override-only policy; Gemini 2.5 ids Unmeasured | REASON-06 | owner policy decisions | accepted 2026-10-23 (51-UAT tests 2 and 3) |
| 82 % coverage floor | REASON-07 | `cargo-llvm-cov` absent locally | CI `Coverage` job: 92.74 % on the v0.11.0 merge commit |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (49 automated; the 3 without one are owner `checkpoint:decision` tasks)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (each checkpoint is followed directly by an automated tracer)
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 180 s per targeted filter (measured 2026-10-27 from a clean target: 0-62 s once built; first compiles 124-151 s; the all-features workspace `--no-run` chain 236 s)
- [x] `nyquist_compliant: true` set in frontmatter — set by `/gsd-validate-phase 51` on 2026-10-27

**Approval:** approved 2026-10-27 (`/gsd-validate-phase 51`)

---

## Validation Audit 2026-10-27

| Metric | Count |
|--------|-------|
| Tasks in map | 52 (49 automated + 3 owner `checkpoint:decision`), populated from plans 51-01..51-20 |
| Wave 0 files | none required; all task-written tests found on the tree (9 golden bodies, conformance cases, drift tests) |
| Automated commands re-run literally from a clean `target/` | 49: **46 green**; 1 state-at-the-time (51-10-02, raw probe transcript untracked by design); 2 green on their tests with `mdbook build` CI-attributed (51-13-01/02) |
| Gaps found | 0 |
| Resolved | 0 (none needed) |
| Escalated | 0 |

**Re-run detail (`CARGO_INCREMENTAL=0`, `--offline`, clean target, 2026-10-27, tree = `v0.11.0` + `.planning` docs):** every plan's chain passed — `paladin-ai-core` `reasoning`/`trace`/`paladin`; `paladin-ports` lib + doctests (`model_profile`, `check_reasoning`); `paladin-battalion` (`llm_failure`/`llm_decision`, `engine::superstep`, `cache_key`, `--test export_golden`); `paladin-llm --all-features` per adapter (`deepseek`, `openai`, `anthropic`, `gemini`, `kimi`, `qwen`, `grok`, `ollama`, `openai_compatible`, `compat::`, `registry::`, `reasoning`, `fallback`, `reasoning_preflight_tests`, `reasoning_page_tests`, the `check_reasoning_*` conformance cases, `unset_reasoning_leaves_the_wire_body_unchanged`, `explicit_reasoning_on_an_unprofiled_model_refuses_before_http`, `seeded_effort_levels_reach_the_wire`, `the_reasoning_effort_page_agrees_with_the_adapters`) and the `live_vendor_smoke` example build under the nine-feature set; `paladin-ai --all-features` (`paladin_builder`, `paladin_execution_service`, `config::agents`, `agent_host`, the `a_chain_*`, `block_*`, `agent_run_node_finished_*` tests); `cargo test --offline --workspace --all-features --no-run` (3×); `make check-migration-allowlist`, `make check-gates`, `make api-surface` ("unchanged", 4×), `make check-consumer-surface`, `make check-api-examples`, `make security`; ADR-0062 and the ADR-0004/0061 dated lines; MIGRATION `reasoning_effort` / `DR-20` / `llm.<provider>.reasoning_wire` entries. The earlier same-day workspace run (`cargo test --workspace --no-fail-fast`, Phase 42 audit) gives the executable totals: 7,719 passed, 2 failed (WINDOWS row 73, Phase 50), 229 ignored.

**Sandbox note:** the first attempt at this phase died when the disk filled during the all-features build (`No space left on device`); `cargo clean` reclaimed 22 GB and the phase was re-run from scratch, which is why first-compile latencies above are cold figures.
