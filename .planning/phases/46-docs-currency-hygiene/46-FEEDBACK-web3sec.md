# Handoff: consumer dogfood findings for v0.11.0 (input to Paladin Phase 46)

**From:** Web3 Security Paladin ("web3sec"), the downstream library consumer of `DF3NDR/paladin-dev-env`.
**To:** the Paladin-dedicated agent, for **Phase 46 (Docs Currency & Hygiene)**, before the Phase 47 tag.
**Date:** 2026-10-24.
**Code basis:** branch `claude/laughing-dirac-e0h2ax` @ `e28e43b5` ("docs(phase-51): mark Phase 51 complete, advance
to Phase 46"). Line numbers are at that commit.

> Filed verbatim by the Phase 46 discuss session on 2026-10-24 as the fifth web3sec handoff; the
> earlier four are `49-HANDOFF-web3sec.md`, `50-HANDOFF-web3sec-parity.md`,
> `51-HANDOFF-web3sec-reasoning-effort.md` and the 50.1 handoff recorded in `50.1-CONTEXT.md`.

## 1. TL;DR

web3sec adapted its whole workspace from its pin `35fd839` to `e28e43b5` in an isolated branch. It followed
MIGRATION.md "Library consumers" as written, using **`paladin::` facade paths only**.

**Result:**
- **No surface blockers.** Every 50.1 ask (SURFACE-01..06) landed and worked as specified. No `paladin-ai-core`
  dependency was needed.
- **Fully mechanical.** 124 compile sites (28 production, 96 test). `cargo check` and `clippy -D warnings` are clean.
- **Tests unchanged:** `cargo test --workspace` gives **2,683 passed / 0 failed / 2 ignored** before and after, identical
  suite by suite.

What remains are **seven guide corrections** and **four small muster gaps**. All are docs or tests, and Phase 46 can
absorb them. Nothing here needs a code change before the tag. §4 lists one optional code suggestion.

## 2. Guide corrections (MIGRATION.md)

| # | Where | Problem | Suggested fix | Severity |
|---|---|---|---|---|
| G-1 | DR-3 (`:1042`) | Names `paladin_core::base::entity::Entity<T>`, which a facade consumer can't reach. | Use `paladin::core::base::entity::Entity`, as every other DR row does and as the muster pins. | Low |
| G-2 | M-B-08 (`:33`) | "Render `source` where you read `error`" changes operator-visible text, because `Display` prefixes `"paladin error: "` / `"llm error: "` (`node_error.rs:113-115`). | Add: to keep the v0.10 text, read `message` from the `Paladin` / `Llm` / `Function` arms (it equals the old `PaladinError` display); `node_id` is the Paladin's name. | Medium |
| G-3 | DR-9 (`:1048`), M-B-06 (`:31`) | Says a Phalanx timeout surfaces "inside `AggregationError`", which is true only with `on_error: None`. With `Absorb` (DR-1's recommended `ContinueOnError` replacement), the run returns `Ok` and the timeout is a `node_errors` entry with `source: Timeout(Run)` (`phalanx_service.rs:233-272`). A consumer that only handles `Err` silently stops recording timeouts. | State both outcomes. | Low-medium |
| G-4 | `:1140` | "`claude-opus-4-8` and `deepseek-v4-pro` (the two models web3sec runs)". web3sec's DeepSeek default is `deepseek-v4-flash`, which is unseeded and absent from DeepSeek's `/models` listing (live check 2026-10-24). | Drop the parenthetical, or say "two of the models web3sec runs". On the Switching-providers page, add a note that `deepseek-flash` is listed and `deepseek-v4-flash` is not. | Medium (factual) |
| G-5 | Cadence recipe (`:1108-1111`) | Uses `CadenceConfig::default()` and `compose_llm(llm, &prices, ..)` without import paths, and doesn't mention the pacing-only `with_cadence`. | Name `paladin::config::treasurer::CadenceConfig`, `PriceTable`, and `paladin::infrastructure::cadence::{build_cadence, with_cadence, compose_llm}`. | Low |
| G-6 | `:1120` vs DR-21 (`:1060`) | The decorator bullet names only `model_profile`; DR-21 says decorators "should" forward `check_reasoning`; the trait docs say **must** for both. | Make the bullet name both, with "must". | Low |
| G-7 | 429 wording for pre-v0.10 consumers | "No 429 pacing unless Cadence is composed" undersells it. From `35fd839`, DeepSeek used to make up to 4 calls and Anthropic up to 3, with backoff, on `RateLimitExceeded`; now there are 0 retries (`deepseek/adapter.rs:989-999`). | One explicit sentence in the "Starting before v0.10.0" block. | Low |

## 3. Muster gaps (`tests/consumer_surface_muster.rs`)

These are used by web3sec after adaptation but not pinned:

1. `Stockpile::render()`.
2. `BattalionResult.{paladin_success_count, paladin_failure_count, per_paladin_times}`.
3. The `paladin::prelude::*` glob. web3sec imports it in `audit-agents/src/lib.rs`, so removing a prelude item breaks it.
4. `ModelProfile` through the `paladin_ports::output::llm_port::ModelProfile` re-export. The muster pins only
   `paladin_ports::output::model_profile::ModelProfile`.

## 4. Optional (code; owner's call, not needed for the tag)

- **Make an uncomposed 429 visible.** A direct-adapter consumer now loses its 429 retries with no signal. A one-time
  `warn!` the first time an adapter returns `RateLimitExceeded` with no Cadence layer would surface it.

## 5. What web3sec does next

- **Branch.** The adaptation lives on web3sec branch `upgrade/paladin-v0.11.0` (3 commits on `0ff826c`), pinned to
  `e28e43b5`.
- **Formal upgrade.** web3sec's formal upgrade (its Phase 50) runs after its current Phase 48. It re-pins detached on
  the `v0.11.0` tag in one consumer commit and returns the report the single-bump todo asks for.
- **Late-moving tag.** If anything above changes the consumer surface after `e28e43b5`, web3sec re-runs the adaptation
  against the tag.
