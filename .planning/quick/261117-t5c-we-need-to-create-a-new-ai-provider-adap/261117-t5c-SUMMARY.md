---
phase: quick-261117-t5c
plan: 01
subsystem: paladin-llm
tags: [llm, provider, zai, glm, compat-engine, docs]
status: complete
requires:
  - ADR-0045 (provider selection; its reintroduction trigger is met by this request)
  - ADR-0046 (facade feature forwarding)
provides:
  - "paladin_llm::zai (ZaiAdapter, ZaiConfig, ZAI_DEFAULT_BASE_URL, ZAI_DEFAULT_MODEL, ZAI_FALLBACK_MODELS); feature zai, facade feature llm-zai (in llm-all, not default)"
  - "provider name `zai` through LlmProviderFactory, configured by ZAI_API_KEY / ZAI_BASE_URL / ZAI_MODEL / ZAI_TIMEOUT_SECONDS only"
affects:
  - crates/paladin-llm (adapter, factory, bridge, registry, live_vendor_smoke example)
  - paladin facade (feature-gated re-export)
  - docs, both CHANGELOGs, ADR-0045
tech-stack:
  added: []
  patterns: ["thin preset on CompatEngine", "empty seeded table plus ReasoningWire::Unsupported until a live probe exists"]
key-files:
  created:
    - crates/paladin-llm/src/zai/mod.rs
    - crates/paladin-llm/src/zai/adapter.rs
    - crates/paladin-llm/tests/golden/reasoning_unset/zai.json
  modified:
    - crates/paladin-llm/src/config/llm.rs
    - crates/paladin-llm/examples/live_vendor_smoke.rs
    - crates/paladin-llm/src/provider_factory.rs
    - crates/paladin-llm/src/config/bridge.rs
    - crates/paladin-llm/src/registry/seeded.rs
    - Cargo.toml
    - src/lib.rs
key-decisions:
  - "Task 2 resolved by the project owner: option-b (env-only-now). No LlmConfig.zai field; an llm.zai block is a later-release follow-up (next milestone, no version promised)."
  - "Stop-sequence cap stays at 1 (fail-closed reading of conflicting docs.z.ai prose vs schema)."
  - "LlmConfig::validate accepts a registry-known default_provider that has no llm.<name> field (found by the option-b RED test: it rejected default_provider zai)."
metrics:
  tasks: 3
  commits: 2
  completed: 2026-11-18
---

# Phase quick-261117-t5c Plan 01: z.ai (GLM) provider adapter Summary

Curated `paladin_llm::zai` provider as a thin preset on the shared OpenAI-compatible `CompatEngine`, wired through factory, layered env bridge, registry, conformance suite, facade, live-smoke harness, env/devcontainer passthrough, docs, both CHANGELOGs and an ADR-0045 amendment. Env-configured only (owner decision option-b); no live z.ai call was ever made.

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 (tracer) | `7a56836` | feat(llm): add z.ai (GLM) provider adapter preset on the compat engine |
| 2 (decision) | none | owner chose option-b (env-only-now) |
| 3 | `6858079` | feat(llm): wire z.ai through live smoke, docs and changelog |

## Verified facts (all read on docs.z.ai, accessed 2026-11-17; vendor-documented, not live-measured)

| # | Fact | Value | Source |
|---|------|-------|--------|
| F1 | Base URL | `https://api.z.ai/api/paas/v4` | docs.z.ai/api-reference/introduction |
| F2 | Chat path/body | `POST {base}/chat/completions`, OpenAI chat body | docs.z.ai/api-reference/llm/chat-completion, /api-reference/introduction |
| F3 | Auth | `Authorization: Bearer <key>` | docs.z.ai/api-reference/introduction |
| F4 | Default model | `glm-5.3` | chat-completion examples; guides/overview/overview; guides/llm/glm-5.3 |
| F5 | Fallback models | `glm-5.3`, `glm-5.2`, `glm-5.1`, `glm-5` | guides/overview/overview; chat-completion text-model enum |
| F6 | Coding-plan endpoint | `https://api.z.ai/api/coding/paas/v4`, separate key family | docs.z.ai/devpack/quick-start; /devpack/notice/api-key-management |
| F7 | Temperature | inclusive `[0.0, 1.0]` | chat-completion `temperature` |
| F8 | Request fields | temperature, max_tokens, top_p, stop carried; penalties not; stop cap read fail-closed as 1 | chat-completion `ChatCompletionTextRequest` |
| F9 | Thinking control | `thinking: {type}` and `reasoning_effort` (candidate wire only; shipped wire `Unsupported`) | chat-completion |
| F10 | Thinking default | enabled; so 300 s timeout default | guides/llm/glm-5.3 |
| F11 | Streamed usage | only in the last chunk (the finish chunk); no `stream_options` documented | guides/capabilities/streaming |
| F12 | `/models` | none documented | docs.z.ai/llms.txt |

### Optional facts that fell back (fail-closed) and why

- Mainland BigModel endpoint: not on docs.z.ai, omitted entirely.
- Context window / max output: documented, but the seeded-table single-source rule seeds nothing from docs alone, so the declared window is unset.
- Prices: the crate carries none (ADR-0052); operators supply a `PriceTable`.
- Seeded table empty, `ReasoningWire::Unsupported`, `error_override: None`: no live seed, effort or quota-body probe exists.
- Usage semantics: engine default kept; not live-measured.
- `response_format` support: not read on docs.z.ai; documented as unverified in agent-runtime.md.

## Task 3 (option-b) work

- `config/llm.rs`: RED test first (`validate_accepts_zai_as_the_default_provider_when_compiled_in` failed with `Invalid default provider: zai`), then GREEN in `LlmConfig::validate`. Also pinned: an `llm.zai` YAML section loads, `get_provider_config("zai")` is `None`, and a build without the feature still rejects the name. Rustdoc states the section "is not read". `LlmConfig` keeps its nine provider fields.
- `live_vendor_smoke`: Z.ai `ProviderSpec` row (`live_model_list: false` because no listing endpoint is documented), `build_zai`, smoke-port arm, and `zai_effort_plan` over the documented controls; counts 9 to 10; `required-features` and `make test-llm-examples` list `zai`. Compiled and tested offline only (69 tests); the offline `--effort --dry-run` renders the Z.ai plan.
- `.env.example`, `config.example.yml` (comment-only, no YAML key), `.devcontainer/{docker-compose.yml,paladin-env.sh,README.md}`.
- Docs: provider-expansion (column plus Streamed-usage row, "yes (usage frame)" with the documented last-chunk shape explained), switching-providers (new env-only/unprofiled section; no table rows because the drift tests pin the nine-provider tables), reasoning-effort (prose bullet, no table row), agent-runtime, feature-flags, configuration (ZAI_* table), installation, crate-map, crate README. Only model ids written: `glm-5.3` (in `ZAI_FALLBACK_MODELS`). No prices.
- Both CHANGELOGs under `[Unreleased]`; ADR-0045 amendment appended (original table untouched).

## Gate results

| Gate | Result |
|------|--------|
| `cargo fmt --all --check` | pass |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` and `cargo clippy --workspace -- -D warnings` | pass |
| `cargo test -p paladin-llm --all-features` | 1595 passed, 0 failed; doc tests 36 passed |
| `cargo test --workspace --lib --bins` | 5606 passed, 0 failed |
| `cargo test --workspace --doc` | 689 passed, 0 failed |
| `cargo test --test config_example_roundtrip` | 14 passed |
| `make test-llm-examples` | 69 passed |
| `make check-gates` | exit 0 |
| `make api-surface` | unchanged (see below) |
| `scripts/check-semver.sh --package paladin-llm` | 223 checks, 223 pass, 0 failures |
| `scripts/check-semver.sh --package paladin-ai` | 223 checks, 223 pass, 0 failures |
| `make security` | FAILED, pre-existing and unrelated (see below) |

- **api-surface**: made to work legitimately: `rustup toolchain install nightly-2026-09-20` (the Makefile/CI pin) and `cargo install cargo-public-api --version 0.52.0 --locked` (the version in the baseline header; the earlier failure was the unpinned latest). The check reported "API surface unchanged": every z.ai export is behind the non-default `llm-zai` feature, so the default-feature baseline `.project/current-exports.txt` needs no refresh.
- **security**: `cargo-audit` (0.23.0) reports 11 vulnerabilities and `cargo-deny` (0.21.0) fails on advisories (bans, licenses, sources ok): lettre 0.11.22 (RUSTSEC-2026-0560 to 0565), libsqlite3-sys 0.30.1 (RUSTSEC-2026-0459), rmcp 2.1.0 (RUSTSEC-2026-0457, 0462), zip 3.0.0 (RUSTSEC-2026-0545, 0548), plus chrono / chrono-tz unmaintained. None involves this change: no dependency or `Cargo.lock` change. Not allowlisted; needs its own owner decision.
- **Coverage**: `cargo llvm-cov` is not installed; CI's `coverage` job is the gate for the 82% floor.
- **Environmental note**: the disk filled once during the facade semver run; generated caches under `target/` (`semver-checks`, `doc`, `debug/incremental`) were deleted and the run repeated.

## Manual credential-handling review (security.instructions.md)

- (a) Redact before bound: `crates/paladin-llm/src/compat/engine.rs:1083` routes error bodies through `diagnostic_excerpt` (`crates/paladin-llm/src/redaction.rs:247`: redact, then bound). Proven for z.ai by `a_non_2xx_body_that_echoes_the_key_does_not_leak_it` (`crates/paladin-llm/src/zai/adapter.rs:1037`), over 400/401/403/500.
- (b) No log statement interpolates the key: `zai/adapter.rs` has no `log::`/`println!` calls; the shared engine's `log::` calls (`engine.rs:848, 1684-1701`) format provider/model names, a key-less URL (`base_url_without_userinfo`) and already-redacted errors. `ZaiConfig` derives `Clone` only (`zai/adapter.rs:169`, no serialisation derive) and prints the key as `<redacted>` via `RedactedKey` (`:215-218`, test `:1311`). The live-smoke additions print no credential (variable names only).
- (c) Redirects are not followed: `redirect_policy: Some(reqwest::redirect::Policy::none())` at `zai/adapter.rs:436`; test `a_redirect_is_not_followed_and_the_second_host_is_never_contacted` (`:1070`).
- (d) `.devcontainer/paladin-env.sh`: the addition only adds `ZAI_API_KEY` to the name list and a label; `paladin-keys` still prints `set (N chars)`, never a value (`paladin-env.sh:92-99`).

## Deviations from Plan

**1. [Rule 1 - Bug] `LlmConfig::validate` rejected `default_provider: "zai"`.** Found by the option-b RED test. With no `llm.zai` field, `validate()` fell through to "Invalid default provider" even though the factory builds `zai`. Fix: accept a name the provider registry knows (feature-aware). Files: `crates/paladin-llm/src/config/llm.rs`. Commit `6858079`. Recorded in both CHANGELOGs.

**2. [Rule 3] Live-smoke effort plan.** The `_ => Vec::new()` default arm would fail the harness's own "every provider has a plan" tests, so a Z.ai plan over the documented controls was added instead of using the default.

**3. docs: no table rows for Z.ai in switching-providers / reasoning-effort.** Their drift tests (`switching_providers_page_lists_every_seeded_model`, the reasoning-effort page check) pin the nine-provider tables, and Z.ai has no seeded models or shipped wire, so prose sections were added instead.

## Deferred Issues

- **Follow-up for the next milestone (target 0.12.0, no version promised in docs): add the `llm.zai` config block as its own phase.** It is a public-API break (`LlmConfig` is constructible and not `#[non_exhaustive]`); it needs the option-a work from the plan: `LlmConfig.zai` field, `CONFIG_RECOGNISED_SPELLINGS`, `agent_host.rs` key loop, `config.example.yml` and fixture blocks, MIGRATION.md section 12.2 register row, crate-wide `constructible_struct_adds_field` allow with a mirrored allowlist entry, `check-migration-allowlist.sh` re-pointing, and z.ai joining `tests/provider_swap_matrix.rs`. Until then z.ai is absent from the swap matrix.
- **Live evidence for z.ai**: run `live_vendor_smoke` (`--probe`, `--usage`, `--effort`) with a real key to seed models and justify a reasoning wire.
- **Pre-existing vacuity guards**: three `provider_factory` tests (`every_config_recognised_and_compiled_provider_name_resolves_through_create`, `create_from_config_builds_each_provider_from_its_block`, `concurrent_create_from_config_builds_independent_adapters`) fail on any build with no block-driven provider, e.g. `--no-default-features --features zai` (and `--features mock`, which fails identically without this work). Not caused by this change; CI does not run those builds.
- **`make security` advisories** listed above (pre-existing).

## Known Stubs

None. (The empty `ZAI` seeded table is intentional and documented: no live probe exists.)

## Threat Flags

None beyond the plan's threat model (T-zai-01 to T-zai-07 mitigated and tested; T-zai-08 and T-zai-SC accepted).

## Self-Check: PASSED

- `crates/paladin-llm/src/zai/adapter.rs`, `crates/paladin-llm/src/zai/mod.rs`: present.
- Commits `7a56836` and `6858079`: present on `claude/stoic-wright-60wtxw`.
- Plan acceptance greps: `ZAI_API_KEY` in `.env.example`, `docker-compose.yml`, `paladin-env.sh`, `configuration.md`; 7 Z.ai mentions in provider-expansion.md; `zai` in both CHANGELOGs; ADR amendment heading count 1; `LlmConfig` keeps 9 provider fields; "is not read" statement in `config/llm.rs`.
