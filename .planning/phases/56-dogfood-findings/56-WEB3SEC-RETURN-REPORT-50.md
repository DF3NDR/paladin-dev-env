# RETURN-REPORT-50 — web3sec Phase 50 consumer return report to Paladin

**From:** web3sec (Web3 Security Paladin), Phase 50 "Upgrade Paladin submodule to v0.11.0"
**To:** Paladin maintainers (v0.11.0 "Crate Release", Paladin P46 docs window and after)
**Date:** 2026-10-30 · **Pin:** `v0.11.0` = `70657919beadbd341a72be2d0107080017a803f0`
**Requirement:** BUMP-07 (this report). Sibling: [PALADIN-FEEDBACK-P46.md](PALADIN-FEEDBACK-P46.md) (already sent).

This report carries SHAs, totals, import paths and findings only. It contains no secrets and no raw
`Refusal.detail` text.

## 1. Consumer commit (C1)

| Item | Value |
|---|---|
| C1 consumer commit | `3ad5d03f` (full `3ad5d03fba2aea8a7e1a702f3c280f5131ca4495`), `build(deps): bump paladin to v0.11.0` |
| Carries | the gitlink move, the crates/ compile adaptation (19 files, applied as a 3-way patch of the dogfood diff), and `Cargo.lock`, in ONE commit |
| Pin | `git ls-tree HEAD paladin` = `70657919…`, `git -C paladin describe --exact-match --tags HEAD` = `v0.11.0` |
| Superseded | the earlier local-only pin `4b7bed87` (`feature/v0.10.0-Commisary-Treasurer-plan`) is recorded as superseded in the C1 body; the previous consumer pin was `35fd8390` (v0.10.1 ancestor) |

C1 was not a cherry-pick of dogfood commit `87b4ee3` (that commit bundles gitlink and lockfile). The behaviour
work followed as separate TDD commits (see section 5) and the security lockfile bump as its own commit.

## 2. D-2: lossless TokenUsage

**Choice:** `TokenUsage` is carried whole (prompt, completion, cache and reasoning counts) from the armory tally to
`PaladinResult.usage`. The 50-02 placeholder `TokenUsage::new(report.token_count, 0)` was removed. Every budget
reader keeps a derived `total_tokens() -> u32` accessor, so WarChest/TallyStick budgeting is byte-identical (D-05).
`total_tokens` is never a second stored figure.

| web3sec type | Was | Now |
|---|---|---|
| `audit_invariants::armory::AuthoredProperty` (authored-property tally) | `total_tokens: u32` | `usage: TokenUsage` + `total_tokens()` |
| `armory::send_prompt` (private) | `-> (String, u32, ReplyEnding)` | `-> (String, TokenUsage, ReplyEnding)` |
| `fuzz::FuzzOutcome` (`Inconclusive`, `CoverageIllusionDiscarded`, `Registered`) | `total_tokens: u32` | `usage: TokenUsage` + `total_tokens()` + `usage()` |
| `fuzz::HypothesisReport` | `total_tokens: u32` | `usage: TokenUsage` + `total_tokens()` |
| `fuzz::PortfolioOutcome` | `total_tokens: u32` | `usage: TokenUsage` + `total_tokens()` |
| `deductive::DeductiveOutcome` | `total_tokens: u32` | `usage: TokenUsage` + `total_tokens()` + `usage()` |
| `phalanx_dispatch::RoutineReport` (private) | `token_count: u32` | `usage: TokenUsage` → `PaladinResult { usage: report.usage }` |

Portfolio reconcile detail: the unattributed failed-attempt remainder is booked as total-only completion tokens so
`total_tokens()` equals the former `max(report, stick.spent())` exactly. Frozen goldens keep their pre-existing
Debug shape through a test-side normaliser (`freeze_usage_shape`), not a re-bless.

## 3. Gate totals

| Gate | Baseline (re-measured, `HEAD ea0b8bce`) | Phase 50 final |
|---|---|---|
| `cargo test --workspace --no-fail-fast` | 2,766 passed / 0 failed / 2 ignored (82 suites) | **2,821 passed / 0 failed / 2 ignored** (+55) |
| `cargo fmt --check` | clean | exit 0 (inside `make clean-code`) |
| `cargo clippy -- -D warnings` | clean | exit 0 (inside `make clean-code`) |
| `cargo check` | clean | exit 0 (inside `make clean-code`) |
| `make check-all-features` | clean | exit 0 |
| no new non-test `unwrap`/`expect`/`panic!` | n/a | `check_no_unwrap_added.sh faf641db` exit 0 |

**Security lockfile bump:** commit `a2e531d0` `build(deps): security lockfile bump`, `Cargo.lock` only, 9 packages
(tokio 1.52.3→1.54.1, rustls 0.23.41→0.23.46, rustls-webpki 0.103.13→0.103.16, quinn 0.11.11→0.11.12,
quinn-proto 0.11.16→0.11.19, quinn-udp 0.5.15→0.5.16, aws-lc-rs 1.17.1→1.19.0, spin 0.9.8→0.9.9, coupled
aws-lc-sys 0.42.0→0.47.0). `cargo audit` went from 17 vulnerabilities to 7; every targeted advisory (tokio
RUSTSEC-2026-0386, rustls, rustls-webpki, quinn-proto, spin yank) is cleared.

**Operator-approved deviation (50-09, 2026-10-29):** the plan's `cargo audit && cargo deny check` pass/fail was
relaxed to "all T-50-02 target advisories resolved; residual pre-existing advisories and the missing `deny.toml`
documented as accepted-and-deferred". Seven residual advisories remain (h2, crypto-bigint, pulldown-cmark, rsa with
no fix, ruint, webpki-roots, webpki-root-certs), all pre-existing on the old pin. `cargo deny` fails licenses
because the repo has no `deny.toml`. Tracked in
`.planning/todos/pending/2026-10-29-security-hardening-deny-toml-and-residual-advisories.md`.

**Hermetic smoke:** `audit-cli greet`, `sandbox-smoke-test --runtime runsc` and the mock end-to-end IT pass. Two
docker-it fork tests (`fork_sandbox_env_never_contains_rpc_secret`, `audit-sandbox fork_daemon_it`) fail identically
on the old pin `35fd8390` (devcontainer Warden-forward-network topology); not a Paladin or bump regression.

### Live parity (BUMP-06 live half)

**Live smoke (orchestrator, runsc).** Anthropic `claude-opus-4-8` through `fuzz_llm_it`: PASS, registered,
non-truncated, 2,689 total tokens (prompt 1,506 / completion 1,183, cache and reasoning counts present). Provider
name unchanged through the TallyStick → Scrivener → ProviderBreaker → MissiveLlm stack. DeepSeek `deepseek-flash`
through `triage_llm_it`: provider healthy (completion returned, triage classified raw 6 → 3 candidates, 0 of 1
batch failed); the one assertion failure is the stale test in finding 7.6, not the provider.

**Benchmark parity.** Scoped 9-entry credited-subset corpus, `corpus_version 2026-09-28.v9`, identical flags to
BASE-03 (doctrine breadth, corroboration on, `DEDUCTIVE_PROVIDER=anthropic`, `FUZZ_PROVIDER=anthropic`, runtime
`runsc`, no `--prove`).

| Run | bench_run_id | Pin / build |
|---|---|---|
| Frozen BASE-03 baseline | `02828189-4b2a-44ee-9404-be8a5ba38a8e` | old pin `35fd8390` |
| Post-bump r1 | `4ea964a6-0288-4141-9101-7d8191fd33ec` | `70657919`, build `628b99c6` |
| Post-bump r2 | `efd7d06e-c9e7-4c8b-832b-19fc88bf5dc8` | `70657919`, build `628b99c6` |
| Post-bump r3 | `ac98bbd6-0093-4615-9644-9ecf58a78649` | `70657919`, build `628b99c6` |

Written tolerance rule (`benchmarks/README.md`, BASE-03 note, section 6): majority-of-3. Each of the 5 credited
findings must reproduce its detection verdict in at least 2 of 3 repeats, and the tare precision control must stay
`deprioritized-pre_disclosed` in at least 2 of 3.

| Check | Repeats holding | Verdict |
|---|---|---|
| `adversarial-over-suppression-seed` | 3/3 (`fuzz-corroborated(corroboration_violated)` each time) | HOLD |
| `clarahacks-ch-0004` | 3/3 (`fuzz-corroborated(corroboration_violated)` each time) | HOLD |
| `clarahacks-ch-0023` | 3/3 (`not-fuzz` each time) | HOLD |
| `the-rewarder-seed` | 3/3 (`not-fuzz` each time) | HOLD |
| `free-rider-seed` | 1/3 (detected only in r3) | FAILS >=2/3 |
| tare precision control | 1/3 (`deprioritized-pre_disclosed` only in r3) | FAILS >=2/3 |

**Formal result:** parity fails the written bar on 2 of 6 checks. **Operator decision (2026-10-30): "Accept +
document".** Both misses are bump-orthogonal, pre-existing LLM non-determinism, not v0.11.0 regressions:

- The four real target detections hold 3/3 with identical credited paths; 0 refusals, 0 fresh-deploy rejections;
  fuzz and deductive providers unchanged (anthropic `claude-opus-4-8`).
- `free-rider-seed` is a historically noisy RegressionFloor DVD seed. Its deductive prompt hash is byte-identical
  across BASE-03 and all three repeats (`acd982aca7ea`, 132,947 B): identical input, output flake.
- Every tare deviation traces to one intermittent DeepSeek pre-disclosed-match call returning `Empty completion:
  finish_reason=length … reasoning likely consumed the entire max_tokens budget` (failed r1 and r2, succeeded r3).
  In r1 the near-miss was not counted; in r2 the same empty completion led to the finding being counted as a
  detection rather than deprioritized. This is the pre-existing DeepSeek reasoning-budget empty-completion mode,
  which v0.11.0's typed `EmptyCompletion` → `TruncatedCompletion` handling (D-05) is the correct response to.

**Model-string note:** BASE-03 history rows show `deepseek/deepseek-v4-flash`; the repeats show
`deepseek/deepseek-flash`. This is the 50-06 default-model alias rename, same backend (live echo re-confirmed in
50-06). `audit-cli bench --compare 02828189 <repeat>` was run for all three repeats.

## 4. Import inventory reconciled with the muster

Paladin P46 already pinned the four web3sec muster gaps in PALADIN-FEEDBACK-P46.md; the remaining "pinned but
unused" muster items are forward-looking adoption pins. The web3sec `use paladin::` / `paladin_ports::` /
`paladin_memory::` inventory (73 paths at the dogfood) was re-run after the behaviour work. **New paths** in
consumer code since the dogfood:

| Path | Introduced by | Use |
|---|---|---|
| `paladin::infrastructure::cadence::build_cadence` | 50-06 | one process-wide Cadence per run |
| `paladin::infrastructure::cadence::with_cadence` | 50-06 | wraps all ten `build_*_llm` adapters |
| `paladin::config::treasurer::CadenceConfig` | 50-06 | `CadenceConfig::default()` |
| `LlmError::Refusal { .. }` | 50-03/04/08 | typed, never-retried `Refused` dispositions |
| `LlmError::RateLimitExceeded { .. }` | 50-03/04/08 | explicit transient arm in every stage classifier |

Everything else resolves through the facade paths already in the dogfood set (`Aegis` family, `*Config::new` plus
`with_*`, `Entity<T>`, `TokenUsage::new`, `LlmResponse::new`, `with_aegis`). `ReasoningEffort` is referenced only as
pass-through arguments in the four decorators' `check_reasoning` forwarding; no P51 effort adoption happened.

## 5. Divergences from MIGRATION §10.8

1. **Window pin knob (D-06).** The intent (keep today's effective 200,000 Anthropic / 64,000 DeepSeek ration
   windows) is kept, but the knob differs: the cap is applied at the Commissary site (`build_deductive_commissary`
   replaces `Commissary::from_port`, using `pinned_ration_window`), not via adapter `context_window` or
   `model_overrides`, because the adapter-wide pin is unconstructible for DeepSeek (finding 7.1).
2. **Cadence composed, no in-adapter 429 retry (D-08).** We compose Cadence as a pacing gate and add explicit
   `RateLimitExceeded` transient arms. We did NOT add a bounded 429 retry loop; the single retry stays in each
   stage's transient arm. This is pacing parity, not retry parity (finding 7.3).
3. **Refusal disposition spread beyond deductive.** §10.8 only needs the shape-proof arm. We route typed `Refusal`
   to an explicit never-retried `Refused` error in deductive, triage, tribunal, prover, design-doc and the armory,
   without adding a new `DegradationReason`.
4. **Decorator forwarding.** `model_profile` and `check_reasoning` are forwarded in all four decorators now (8
   one-liners), as a prerequisite for adopting P51.
5. **Commit shape.** Not a literal single commit: C1 (pointer + compile adaptation + lock), then TDD behaviour
   commits per plan, then a separate lockfile commit `a2e531d0`.
6. **DeepSeek default.** Swapped `deepseek-v4-flash` → `deepseek-flash` (live-listed, seeded) rather than relying on
   the alias.

## 6. SURFACE-02 harmonisation position

**Adopted.** web3sec adopted the harmonised `*Config::new` plus `with_*` construction forms in the C1
adaptation, and does not rely on struct-literal construction of Paladin configs. The D-11 consumer
feedback handoff was already sent as [PALADIN-FEEDBACK-P46.md](PALADIN-FEEDBACK-P46.md) (7 guide corrections, 4
muster gaps, no blockers); nothing in this report supersedes it.

## 7. New findings to relay

1. **DeepSeek adapter-wide window pin is unconstructible below `max_output_tokens`.** Setting
   `DeepSeekConfig.context_window = Some(64_000)` fails adapter construction (the adapter-wide knob cannot be set
   below the model's `max_output_tokens`), so a consumer cannot pin a smaller DeepSeek ration window through the
   adapter. We cap at the Commissary site instead. Covered by `deductive::pin_tests`. Consider a documented
   per-role or Commissary-level window cap.
2. **`deepseek-v4-flash` → `deepseek-flash` alias behaviour.** The old string was an alias to the same backend (live
   echo, 2026-10-28), no longer present in the provider listing. Benchmark history rows therefore carry two strings
   for one backend. The guide should say which strings are aliases and for how long they resolve.
3. **Cadence is pacing, not retry, for a pre-v0.10 consumer.** A consumer upgrading from before v0.10 had
   in-adapter bounded 429 retries (DeepSeek up to 4, Anthropic up to 3 calls, ~100 ms·2ⁿ). Composing Cadence
   restores back-pressure (gate and wait) but the failing call surfaces `RateLimitExceeded` unchanged and the inner
   call count stays 1. The migration guide should frame Cadence as the replacement for retry-on-429 only with a
   stage-level retry, and state that the default `max_wait` (300 s) surfaces `RateLimitExceeded`.
4. **MIGRATION §10.8 tooling still cites `9.8`-style section labels.** Tooling and cross-references in the guide
   and muster still use `9.8`-style labels where §10.8 is current.
5. **Intermittent DeepSeek reasoning-budget empty-completion under a tight `max_tokens` budget.** web3sec's
   pre-disclosed-match and triage calls to `deepseek-flash` intermittently return `Empty completion:
   finish_reason=length … reasoning likely consumed the entire max_tokens budget` (failed in 2 of 3 post-bump
   repeats, succeeded in 1). v0.11.0's typed `EmptyCompletion` makes the failure visible and mappable to
   `TruncatedCompletion`, but it moves a previously silent condition into benchmark variance. Worth a Paladin-side
   note on reasoning-model budgets, and a possible budget-aware default or reasoning-share hint for DeepSeek.
6. **Stale consumer test, not a Paladin finding (FYI).** `crates/audit-agents/tests/triage_llm_it.rs:215`
   asserts `finding.hypothesis.contains("Severity:")`, which contradicts FU-4 (commit `8d242970`, which stripped the
   Severity/Difficulty preamble). The test is `#[ignore]`d, predates Phase 50, and Phase 50 touched neither
   `into_finding_draft` nor the test. It is not a model drift or a v0.11.0 regression. Cleanup todo:
   `.planning/todos/pending/2026-10-29-stale-triage-llm-it-severity-preamble-assertions.md`.
