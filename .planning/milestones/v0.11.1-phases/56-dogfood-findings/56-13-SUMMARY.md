---
phase: 56-dogfood-findings
plan: 13
subsystem: dogfood-findings
tags: [cli, setup-check, onboarding, anthropic, provider-probe, redaction, df-19, dogf-06, stage-c, gap-closure]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "56-08 keyed walk (runbook K8 read `Anthropic API validation failed (404 Not Found)`), DF-19 findings-log entry"
provides:
  - "src/application/cli/commands/provider_probe.rs: the one private provider-key probe module for setup-check and onboarding (Anthropic, OpenAI, DeepSeek), one client (10 s timeout, Policy::none()), one failure shape"
  - "anthropic_probe_model() sourced from paladin_llm::anthropic::adapter::ANTHROPIC_DEFAULT_MODEL, with a drift-tested mirror for builds without llm-anthropic"
  - "Seven provider_probe regression tests (each watched failing first)"
  - "CHANGELOG [Unreleased] ### Fixed bullet and one docs line in cli-setup-check.md"
  - "DF-19 closed in the findings log (in v0.11.1, fixed by 343300aff, 889b5f1bb), with the maintainer's real-key re-check"
affects: [56-14, 56-10, 56-verification, SHIP-10]

tech-stack:
  added: []
  patterns:
    - "Every provider reply reaches an error only through diagnostic_excerpt (redact, then bound)"
    - "Probe model is read from the adapter's seeded default, never a literal; a drift test pins the no-adapter mirror to it"

key-files:
  created:
    - src/application/cli/commands/provider_probe.rs
    - .planning/phases/56-dogfood-findings/56-13-SUMMARY.md
  modified:
    - src/application/cli/commands/mod.rs
    - src/application/cli/commands/setup_check.rs
    - src/application/cli/commands/onboarding.rs
    - docs/src/appendix/cli-setup-check.md
    - CHANGELOG.md
    - .project/v0.11.1/02-dogfood-findings-log.md

key-decisions:
  - "PD-13-1: probe model is ANTHROPIC_DEFAULT_MODEL (public, pinned to a seeded row by paladin-llm's anthropic_default_model_is_seeded); paladin-llm's pub(crate) registry stays unwidened"
  - "PD-13-2: all three validators moved into one module; the two drifted copies (setup-check dropped the body, onboarding echoed it raw) are gone"
  - "PD-13-4: shared client uses redirect(Policy::none()) because reqwest strips Authorization but not x-api-key across hosts"
  - "PD-13-5: CHANGELOG ### Fixed only, no MIGRATION row"

patterns-established:
  - "provider_probe failure text: `<Provider> API validation failed for model '<model>' (<status>): <excerpt>` (Anthropic) / `<Provider> API validation failed (<status>): <excerpt>` (model-less probes)"

requirements-completed: []

duration: ~2 sessions (Tasks 1-2, checkpoint, Tasks 3-4)
completed: 2026-11-03
---

# Phase 56 Plan 13: DF-19 Anthropic key probe Summary

**`paladin-cli setup-check` and `onboarding` now validate an Anthropic key through one shared probe that posts the Anthropic adapter's seeded default model instead of a retired claude-3 id, names the model and a redacted, bounded provider reply on failure, and never follows a redirect; the maintainer's real-key re-run reads `✓ Anthropic`.**

Plan-start SHA: `e6aa06db099109fdbc399bc1ffd79634a81b2373`.

Requirements: DOGF-06 deliberately NOT marked complete (DF-26 is still open, closed by Stage C plan 56-14); DOGF-01 was already complete.

Commit trailers: the orchestrator named `Claude Fable 5.1`; this continuation executor is Sonnet 5.5, so the Task 4 commit carries the harness-provided `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>` and the session trailer. Tasks 1-2 were committed by the earlier executor.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | `343300aff` | fix(56-13): probe Anthropic keys with the adapter's seeded default model and name it on failure (DF-19) |
| 2 | `889b5f1bb` | fix(56-13): share the provider-key probes, redact their replies, never follow redirects (DF-19) |
| 3 | n/a | Maintainer re-check (human-verify), approved 2026-11-03 |
| 4 | `71ba278d8` | docs(56-13): DF-19 disposition in v0.11.1 (DF-19, DOGF-06) |

## Red-then-green record

Task 1 (tracer), `cargo test -p paladin-ai --features cli,web-server --lib provider_probe`:

- RED against the moved, unchanged code: 0 passed / 5 failed.
  - `anthropic_probe_model_is_the_adapters_seeded_default`: left `claude-3-haiku-20240307`, right `claude-haiku-5-5`.
  - `anthropic_probe_posts_the_probe_model_and_passes_on_200`: the mock answered 501 on the body mismatch.
  - `anthropic_probe_failure_names_the_model_and_the_provider_reply`: the message lacked `for model '…'`.
  - `anthropic_probe_failure_redacts_the_key_from_the_reply`: the message lacked `[REDACTED]`.
  - `anthropic_probe_does_not_follow_a_redirect`: the 307 was followed.
- GREEN: 5 passed / 0 failed.

Task 2, same command:

- RED: 5 passed / 2 failed. `openai_probe_failure_carries_a_redacted_reply` and `deepseek_probe_failure_carries_a_redacted_reply` both failed on the `[REDACTED]` assert (the moved code dropped the body).
- GREEN: 7 passed / 0 failed.

OpenAI and DeepSeek hard-coded no model: both `GET …/v1/models` and send none, so nothing needed sourcing from the registry for them.

## Maintainer re-check

Run on 2026-11-03 (UTC date as relayed by the orchestrator), on commit `889b5f1bb`. Command (key entered with `read -rs`, then unset):

```
cargo run -q -p paladin-ai --features cli --bin paladin-cli -- setup-check --verbose
```

Relayed output, verbatim. The `...` lines are the maintainer's own elisions (OpenAI, DeepSeek, Redis and Qdrant rows omitted). Before writing, the block was checked for key material (the plan's key-prefix grep on the paste: 0 hits; the orchestrator confirmed the same):

```
┌═══════════════════════┐
│ Paladin Setup Check │
└═══════════════════════┘

⠏ LLM provider checks complete
⠏ Service checks complete
LLM Provider:
...
  ✓ Anthropic
ℹ   All checks passed
...
System:
  ✓ Paladin CLI v0.11.0
ℹ   All checks passed
  ✓ Rust Toolchain (rustc 1.97.1 (8bab26f4f 2026-07-14))
ℹ   All checks passed
```

The Anthropic row reads `✓ Anthropic`: the probe model is served for the maintainer's account. Runbook K8 read `✗ Anthropic … 404 Not Found` for the same command before the fix. DF-19 is closed by the command that found it.

## Findings-log check

`./scripts/check-findings-log.sh` after the DF-19 disposition (exit 1, by design):

```
🔍 Checking the findings log for entries without a final disposition ...
❌ Findings-log check failed (BAD)
...
.project/v0.11.1/02-dogfood-findings-log.md:631: DF-26: OPEN (the last Disposition bullet reads 'open')
parsed 26 entries from .project/v0.11.1/02-dogfood-findings-log.md
```

DF-19 is no longer named. DF-26 (logged by the orchestrator after plan 56-12) is the only open entry and is closed by Stage C plan 56-14, so the plan's "prints OK" expectation is not met yet; that is expected and DF-26 was not touched. The log diff for Task 4 is `1 0` (one insertion, 0 deletions). The disposition bullet was inserted as the last line of DF-19's entry (the log's last line belongs to DF-26, so this is an in-place insertion, not a file-end append).

Disposition bullet written:

`- **Disposition (2026-11-03):** in v0.11.1 — fixed by 343300aff, 889b5f1bb (plan 56-13); regression: anthropic_probe_model_is_the_adapters_seeded_default, anthropic_probe_posts_the_probe_model_and_passes_on_200, anthropic_probe_failure_names_the_model_and_the_provider_reply, anthropic_probe_does_not_follow_a_redirect; maintainer re-check 2026-11-03 ✓ Anthropic`

## Security review

Manual credential-handling review (`security.instructions.md`), recorded by the Task 1-2 executor:

- (a) Every non-2xx probe body reaches an error only through `paladin_llm::redaction::diagnostic_excerpt` (redact, then bound to 512 chars). Pinned by the three `*_redact*` / `*_carries_a_redacted_reply` tests.
- (b) `probe_client()` is the only reqwest client builder in `provider_probe.rs` and sets `redirect(Policy::none())`; `onboarding.rs` builds none. Pinned by `anthropic_probe_does_not_follow_a_redirect` (the second host receives 0 requests).
- (c) No `println!` / log call in either command interpolates a key.
- (d) Keys travel only in the `x-api-key` (Anthropic) and `Authorization` (OpenAI, DeepSeek) request headers.
- The maintainer's real key never reached the repo, a commit, or this SUMMARY (T-56-51): `read -rs` entry, `unset` after, redacted output pasted only.

## Gates

As reported by the Task 1-2 executor (no cargo build was run in this continuation):

- `cargo fmt --all -- --check`: clean.
- `cargo clippy -p paladin-ai --features cli,web-server --lib -- -D warnings`: clean, and the same with `--profile test`.
- `make api-surface`: unchanged (4325 items).
- `./scripts/check-semver.sh --package paladin-ai`: 223 pass / 0 failures at patch.
- `make check-gates`: OK.
- No manifest or lockfile change (`git diff --quiet` on `Cargo.toml`, `Cargo.lock`, `crates/*/Cargo.toml`: true; re-confirmed against the plan-start SHA in this continuation).
- MIGRATION.md unchanged by this plan (PD-13-5; `git diff --quiet e6aa06db0 HEAD -- MIGRATION.md` clean).
- The docs edit adds no YAML fence, so `make check-doc-config` is not triggered.

## Semver

D-00b restated from Task 2's gates: `provider_probe` is a private module and every moved function stays private, so there is no public signature change. `check-semver.sh --package paladin-ai` reported 223 pass / 0 failures at `--release-type patch`; MIGRATION section 11.2 is still header-only (no row) and the semver allowlist has no entry.

## Deviations from Plan

None - plan executed as written. Two notes, neither a deviation from intent:

- Task 4's plan text expected `check-findings-log.sh` to print OK once DF-19 closed. The orchestrator logged DF-26 after the plan was written, so the checker still exits 1 naming only DF-26. Per the continuation instructions this is expected and DF-26 was left alone.
- The regression-test list in the DF-19 disposition also cites `anthropic_probe_posts_the_probe_model_and_passes_on_200` (per the orchestrator's text); the plan's own list also named `anthropic_probe_failure_redacts_the_key_from_the_reply`, which is covered under Security review (a).

## Authentication gates

None for the agent. The Task 3 re-check was the planned maintainer-run `checkpoint:human-verify`, approved.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path or file-access surface: the three existing probe requests moved into one module and gained redaction and a no-redirect client (T-56-48, T-56-49, T-56-50, T-56-51 mitigated).

## Stage C inputs

None new. DF-19 is closed. DF-26 remains the only open finding and is closed by plan 56-14.

## Self-Check: PASSED

- FOUND: `343300aff`, `889b5f1bb`, `71ba278d8` (git commits verified).
- FOUND: `.project/v0.11.1/02-dogfood-findings-log.md` carries the DF-19 disposition; log diff `1 0`.
- Key-prefix grep on this SUMMARY: 0 hits.
