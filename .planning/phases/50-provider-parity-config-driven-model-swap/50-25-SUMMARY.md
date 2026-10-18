---
phase: 50-provider-parity-config-driven-model-swap
plan: 25
subsystem: llm
tags: [rust, paladin-ports, paladin-llm, paladin-battalion, paladin-eval, llm-error, constructors, gap-closure, g-50-5a, parity]
status: complete
gap_closure: true
gap_ids: [G-50-5a]

requires:
  - phase: 50
    provides: "50-12 LlmError::Refusal and the D-11a EmptyCompletion payload; 50-24 (wave order)"
provides:
  - "LlmError::empty_completion(detail) and LlmError::refusal(provider, model, detail): public, rustdoc'd, doc-tested constructors that build exactly today's values (same Display, Permanent)"
  - "Tracer test llm_error_constructors_build_todays_values (paladin-ports)"
  - "Seven consumer files (paladin-eval scenario.rs; paladin-battalion llm_decision.rs, llm_failure.rs; paladin-llm kimi/adapter.rs, openai/adapter.rs, fallback.rs, conformance.rs) build through the constructors and match EmptyCompletion { .. } / Refusal { .., .. }, so plan 50-26's struct variants need not touch them"
affects: [50-26]

tech-stack:
  added: []
  patterns:
    - "Constructors land ahead of a variant reshape so consumers move off the literal shape additively, one plan before the break (the RateLimitExceeded / rate_limited(..) precedent)"
    - "A struct pattern with `..` (`EmptyCompletion { .. }`) is valid against a tuple variant, so non-binding patterns can migrate before the shape change"

key-files:
  created: []
  modified:
    - crates/paladin-ports/src/output/llm_port.rs
    - crates/paladin-eval/src/scenario.rs
    - crates/paladin-battalion/src/llm_decision.rs
    - crates/paladin-battalion/src/llm_failure.rs
    - crates/paladin-llm/src/kimi/adapter.rs
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/fallback.rs
    - crates/paladin-llm/src/conformance.rs

key-decisions:
  - "The constructors are permanent public API (A2-50-25), not a shim: after plan 50-26 they are the only way to build either variant outside paladin-ports"
  - "Additive only: no variant shape, Display, transience or wire behaviour changed, so no MIGRATION 9.2 row and no CHANGELOG entry (the billing change's entries land with plan 50-26 and plan 50-31)"

patterns-established:
  - "Rustdoc on a constructor says it is the supported way to build the variant and that a later field will not change it"

requirements-completed: [PARITY-05]

duration: 63min
completed: 2026-10-18
---

# Phase 50 Plan 25: LlmError constructors for empty and refused calls; seven consumers shape-proofed (G-50-5a) Summary

**`LlmError::empty_completion(detail)` and `LlmError::refusal(provider, model, detail)` now exist as public,
doc-tested constructors. Each builds exactly today's value. The seven consumer files outside the guard-owning
adapters no longer depend on the current tuple or full-field shapes, so plan 50-26's struct variants compile
there unchanged. Nothing a caller can observe changed.**

## Performance

- **Duration:** about 63 min, most of it build time. One workspace build was lost to ENOSPC and re-run (see
  Verification).
- **Started:** 2026-10-18T14:24:00Z
- **Completed:** 2026-10-18T15:27:00Z
- **Tasks:** 2 of 2. Task 1 was the tracer. Its gate passed in autonomous mode and was re-run on the committed
  tree: "Tracer verified end-to-end - expanding".
- **Files modified:** 8 (0 created)

## Accomplishments

- **Task 1 (tracer, `c6505351`).**
  - Red first: `llm_error_constructors_build_todays_values` failed to compile (`E0599`: no associated function
    `empty_completion` / `refusal`).
  - Green: both constructors were added beside `rate_limited(..)`, each with an `# Examples` doc test that uses no
    `?` and no `unwrap`. Both doc tests ran and passed.
  - The test checks `empty_completion("x")` renders `Empty completion: x`, matches `EmptyCompletion { .. }` and is
    `Permanent`. It checks `refusal("p", "m", "d")` renders `Refused by provider 'p' (model 'm'): d`, binds the three
    values through `Refusal { provider, model, detail, .. }` and is `Permanent`.
  - The paladin-eval and paladin-battalion sites moved onto the constructors.
- **Task 2 (`05814715`).** The four paladin-llm files moved onto the constructors and shape-proof patterns.

### Migrated sites (old -> new)

| File | Line (post-change) | Old form | New form |
|---|---|---|---|
| `crates/paladin-eval/src/scenario.rs` | 420 | `LlmError::EmptyCompletion(format!(..))` | `LlmError::empty_completion(format!(..))` |
| `crates/paladin-battalion/src/llm_decision.rs` | 310 | `LlmError::EmptyCompletion(_) => "empty completion"` | `LlmError::EmptyCompletion { .. } => "empty completion"` |
| `crates/paladin-battalion/src/llm_decision.rs` | 332 (test) | `LlmError::Refusal { provider: .., model: .., detail: .. }` | `LlmError::refusal("openai", "m", "sensitive vendor text")` |
| `crates/paladin-battalion/src/llm_failure.rs` | 288 (test) | `LlmError::EmptyCompletion("finish_reason=length, content empty".to_string())` | `LlmError::empty_completion("finish_reason=length, content empty")` |
| `crates/paladin-battalion/src/llm_failure.rs` | 371 (test) | `LlmError::EmptyCompletion("finish_reason=length".to_string())` | `LlmError::empty_completion("finish_reason=length")` |
| `crates/paladin-battalion/src/llm_failure.rs` | 404 (test) | `LlmError::Refusal { provider: .., model: .., detail: .. }` | `LlmError::refusal("anthropic", "m", "d")` |
| `crates/paladin-llm/src/kimi/adapter.rs` | 750, 782, 1119 (tests) | `Err(LlmError::EmptyCompletion(_))` | `Err(LlmError::EmptyCompletion { .. })` |
| `crates/paladin-llm/src/openai/adapter.rs` | 2893, 2981 (tests) | `Err(LlmError::Refusal { provider, model, detail })` | `Err(LlmError::Refusal { provider, model, detail, .. })` |
| `crates/paladin-llm/src/openai/adapter.rs` | 2933, 3006 (tests) | `Err(LlmError::EmptyCompletion(_))` | `Err(LlmError::EmptyCompletion { .. })` |
| `crates/paladin-llm/src/fallback.rs` | 2065 (test helper `refusal`) | `LlmError::Refusal { provider: .., model: .., detail: .. }` | `LlmError::refusal(provider, model, "I can't help with that.")` |
| `crates/paladin-llm/src/conformance.rs` | 1342, 1376 | `err @ LlmError::EmptyCompletion(_)` | `err @ LlmError::EmptyCompletion { .. }` |

Already shape-proof and left alone:
- `llm_failure.rs:117` `Refusal { provider, .. }`
- `fallback.rs` `Refusal { .. }` patterns (510, 2273, 2278, 2306, 2344)
- `conformance.rs:1418/1424` `Refusal { .. }` / `Refusal { provider, detail, .. }`
- `llm_decision.rs:311` `Refusal { .. }`

No assertion changed at any site.

## Task Commits

1. **Task 1 (tracer): LlmError constructors for empty and refused calls**: `c6505351` (refactor)
2. **Task 2: paladin-llm consumers use shape-proof LlmError forms**: `05814715` (refactor)

## Deviations from Plan

None. The plan was executed as written.

Smaller calls, within scope:
- The constructors' rustdoc says "a later change adds a slot for the usage of a call the vendor billed". It does not
  name plan 50-26, because the text is public API documentation. Plan 50-26 rewrites it when the slot lands.
- `cargo semver-checks check-release --package paladin-ports --default-features --baseline-version 0.9.0` exits 0,
  as the plan requires. Note that it reports `paladin-ports v0.9.0 -> v0.10.1 (major change)` and runs 0 checks
  (254 skipped). In this 0.x range, 0.9 -> 0.10 is a major bump, so the CI shape passes without evaluating any lint.
  The D-27 run in plan 50-26 (baseline 0.10.1, `--release-type minor`) is the one that measures.

## Verification

- **Task 1 verify, all exit 0, re-run after the commit (tracer gate):**
  - `cargo test -p paladin-ports`: 255 unit tests passed, and doc tests 198 passed / 94 ignored. The 94 ignored doc
    tests were already ignored; the two new constructor doc tests ran and passed.
  - `cargo test -p paladin-battalion --lib llm_`: 30 passed.
  - `cargo test --doc -p paladin-battalion`: 61 passed, 52 ignored.
  - `cargo test -p paladin-eval --lib`: 56 passed.
- **Task 2 verify, all exit 0:**
  - `cargo test -p paladin-llm --all-features --lib`: 1184 passed.
  - `cargo build --workspace --all-targets --all-features`: finished. The first run's output was lost to ENOSPC.
    After `target/debug/incremental` was deleted, the re-run finished, `exit=0`.
  - `cargo semver-checks ... --baseline-version 0.9.0`: "no semver update required" (see above).
- **Acceptance greps:**
  - `pub fn empty_completion` 1, `pub fn refusal` 1.
  - `EmptyCompletion(` code lines in the three Task 1 files: 0. In the four Task 2 files: 0.
  - `LlmError::refusal(` in `fallback.rs`: 1.
- **Plan-level gates, all run after the last code commit:**
  - `cargo test -p paladin-battalion --lib`: 877 passed.
  - `cargo test -p paladin-llm` (default features): 448 passed. The crate sets `doctest = false`.
  - `cargo fmt --check`: clean.
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean.
  - `make clean-code`: exit 0, including doc-check and check-api-examples, with no formatting diff.
  - `make check-gates`: exit 0. `make check-api-examples`: 105 entry points ok.
  - `make api-surface`: "API surface unchanged" (4298 items).
- **`make security`: exit 2. This plan did not cause it.**
  - `cargo-audit` reports exactly the three known vulnerabilities, RUSTSEC-2026-0367/0368/0369 in `zip 3.0.0`, plus
    9 allowed warnings.
  - This is the open owner decision in `50-VALIDATION.md`. No `Cargo.toml` or `Cargo.lock` changed.
- **Diff check for `unwrap(` / `expect(` / `panic!(`:** the one added occurrence is a `panic!` in the
  `#[cfg(test)]` tracer test. The non-test additions are the two constructors, which contain none, and their doc
  examples contain none either.
- **Manual credential-handling review:** no credential path was touched. There is no HTTP client, header, redirect
  policy, log line or redaction change. The constructors only assemble the strings they are given. Refusal
  `detail` is still redacted and then bounded by the guard before it reaches them (D-00a, T-50-25-02 accepted).
- **Not run:** Docker-bound integration tests, and live vendor calls (no key; there is no wire change).
- **Environment:** the first workspace build filled the disk to 0 bytes free. `target/debug/incremental` was
  deleted, along with 76 stale duplicate test executables in `target/debug/deps` (older hashes of the same test
  targets, about 3.9 GB). Later builds ran with `CARGO_INCREMENTAL=0`. About 4.9 GB was free after the gates.

## Known Stubs

None.

## Threat Flags

None. Against the plan's register:
- T-50-25-01 (mitigated): no assertion changed; every touched crate's suite and doc tests ran; each site is listed
  above.
- T-50-25-02 (accepted): no log line or label was added.

## Flagged assumptions

- A1-50-25 held: `EmptyCompletion { .. }` compiles against the tuple variant (both commits built
  `--workspace --all-targets --all-features`).
- A2-50-25 held: the constructors are additive. `make api-surface` is unchanged, because the facade lists
  `LlmError` by path only, and the semver CI shape passes.

## Self-Check: PASSED

- All 8 modified files exist.
- Commits `c6505351` and `05814715` exist on `claude/laughing-dirac-e0h2ax`.
- `grep -c 'pub fn empty_completion' crates/paladin-ports/src/output/llm_port.rs` is 1.
