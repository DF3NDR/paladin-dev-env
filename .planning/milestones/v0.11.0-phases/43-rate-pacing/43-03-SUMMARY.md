---
phase: 43-rate-pacing
plan: 03
subsystem: llm-pacing
tags: [rate-limit, 429, retry-after, response-headers, openai, anthropic, quota, cadence]

requires:
  - phase: 43-rate-pacing
    provides: CadencePort and the CadenceLlmAdapter decorator (43-01); the typed RateLimitExceeded { retry_after, hints } carrier and RateLimitHints (43-02)
provides:
  - rate_limit_headers module in paladin-llm (un-gated, closure-based): RateLimitHeaderFamily, hints_from_headers, parse_retry_after, parse_go_duration and the header-name constants
  - map_http_status_with_hints beside map_http_status (which keeps its signature and delegates with None)
  - OpenAI and Anthropic adapters that snapshot 429 headers before the body is consumed, on both the buffered and streaming paths
  - Anthropic surfaces its first 429 (D-02 Anthropic half)
  - quota-class 429s (OpenAI insufficient_quota, Anthropic enforced_spend_limit_reached) mapped to the permanent UsageLimitExceeded
  - the surface registered on MIGRATION 9.1/9.2/9.3 and CHANGELOG; httpdate promoted to a direct paladin-llm dependency
affects: [43-04, 43-05, 43-06, 43-12, 43-13]

tech-stack:
  added: [httpdate 1.0 (direct edge; already in Cargo.lock at 1.0.3 via hyper, no new package)]
  patterns:
    - "Header parsing at the HTTP edge through a lookup closure, so the module names no HTTP-client type and stays un-gated"
    - "A thin map_error wrapper over map_error_with_hints, the wrapper cfg(test) so existing mapping tests compile unchanged"
    - "Quota/spend-cap detection compares only a short JSON identifier and renders no body text"

key-files:
  created:
    - crates/paladin-llm/src/rate_limit_headers.rs
  modified:
    - crates/paladin-llm/src/http_status.rs
    - crates/paladin-llm/src/lib.rs
    - crates/paladin-llm/Cargo.toml
    - crates/paladin-llm/src/openai/adapter.rs
    - crates/paladin-llm/src/anthropic/adapter.rs
    - Cargo.lock
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "An unusable Retry-After falls through to retry-after-ms, then to the reset of an exhausted dimension; hints_from_headers returns None when nothing usable parsed, so a bare 429 carries no hints"
  - "Go-duration arithmetic is exact integer nanoseconds (saturating u128), not f64, so 6ms and 1h2m3s are exact; fractions keep nine digits"
  - "map_error is kept as a cfg(test) wrapper rather than a dead non-test function; every production path (including OpenAI get_available_models) calls map_error_with_hints"
  - "The pub mod rate_limit_headers carries only its inner //! docs; an outer /// in lib.rs would merge with them (the 40-06 rustdoc rule)"

patterns-established:
  - "Pure parser, no shared state: concurrent 429s on different tasks parse independently"
  - "Every duration leaving the parser is clamped to CADENCE_DELAY_CEILING (24 h); every numeric field is a checked parse; every Some is a number or a Duration, never a raw header string"

requirements-completed: [PACE-01, PACE-02]

duration: about 25 min of execution (three commits between 14:40 and 14:50 UTC, plus verification)
completed: 2026-10-08
status: complete
---

# Phase 43 Plan 03: Provider rate-limit headers for OpenAI and Anthropic Summary

**A shared panic-free header parser fills RateLimitHints from a real OpenAI or Anthropic 429 (Retry-After in both forms, x-ratelimit-* Go durations, anthropic-ratelimit-* RFC 3339 resets), Anthropic surfaces its first 429, and quota-class 429s become the permanent UsageLimitExceeded.**

## Performance

- **Tasks:** 3 of 3
- **Commits:** cf1fb1a6, e4ea8602, 35427737
- **Files:** 1 created, 8 modified (about 2,060 lines added, most of it tests and rustdoc)

## Accomplishments

- `rate_limit_headers.rs`: `hints_from_headers(family, now, lookup)`, `parse_retry_after`, `parse_go_duration`, `RateLimitHeaderFamily` (`Generic`, `OpenAi`, `Anthropic`) and the nine header-name constants. Precedence is `Retry-After` (delta-seconds, plain decimal, or any of the three HTTP-date forms), then `retry-after-ms`, then the reset of an exhausted dimension. Values over 128 bytes, Go durations over 64 bytes or 8 groups, signs, exponents, NaN, inf and non-ASCII digits are all refused; decimals go through `Duration::try_from_secs_f64`; every delay is clamped to `CADENCE_DELAY_CEILING`. Rustdoc quotes Anthropic's header semantics from `platform.claude.com/docs/en/api/rate-limits` (retrieved 2026-10-07) and marks every OpenAI header "Pending operator verification against the official OpenAI rate-limits guide (plan 43-12 checkpoint)"; `retry-after-ms` is documented as an Azure/`openai-python` extra, not an OpenAI-documented header.
- `map_http_status_with_hints` in `http_status.rs`: identical table, the 429 row carries the hints. `map_http_status` keeps its signature and doctest and delegates with `None`.
- OpenAI and Anthropic: the 429 header snapshot is taken before `response.text()` consumes the response on the buffered and streaming paths (and on OpenAI's `get_available_models`). `map_error` became a `cfg(test)` wrapper over `map_error_with_hints`, so every existing mapping test compiled unchanged.
- D-02 (Anthropic half): `execute_with_retry` treats `RateLimitExceeded { .. }` as non-retryable. A mocked 429 is observed exactly once; network and 5xx retries are untouched.
- OQ7: an OpenAI 429 whose `error.code` or `error.type` is `insufficient_quota`, and an Anthropic 429 whose `error.details.error_code` is `enforced_spend_limit_reached`, map to `UsageLimitExceeded` before the generic arm. The probe compares a short identifier and renders nothing.

## Red / green record

- **Task 1 (parser):** the module was first written with the real public signatures and stub bodies plus the full table tests. Result: 20 failed, 3 passed (the three that trivially expect `None`). The real implementation then passed all 23 on the first run. The five `map_http_status_with_hints` tests were authored in the same edit as the function (they cannot compile without it), so they have no separate red run.
- **Task 2 (OpenAI):** 8 tests written first against the unchanged adapter: 5 failed (both hint tests, the stream hint test, and both quota tests), 3 passed trivially. Green after the change: 91 openai tests.
- **Task 2 (Anthropic):** 8 new tests written first: 7 failed. Green after the change: 47 anthropic tests, including exact `Date`-relative resets (110, 105, 101 and 160 s).

## Task Commits

1. **Task 1: shared header parser and `map_http_status_with_hints`** - `cf1fb1a6`
2. **Task 2: OpenAI and Anthropic header snapshot, first-429 surfacing, quota-class mapping** - `e4ea8602`
3. **Task 3: register on MIGRATION 9.1/9.2/9.3, CHANGELOG** - `35427737`

## Verification

- `cargo test -p paladin-llm --lib rate_limit_headers`: 23 passed. `cargo test -p paladin-llm --all-features`: 595 passed, 0 failed. `cargo test -p paladin-llm --all-features --doc` for `map_http_status` (2) and `rate_limit_headers` (5): pass.
- `cargo build -p paladin-llm --no-default-features`: exit 0 (the module is un-gated).
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean. `cargo fmt --check`: clean. `RUSTDOCFLAGS='-D warnings' cargo doc -p paladin-llm --no-deps --all-features`: clean.
- `./scripts/check-migration-allowlist.sh`: allowlist set-equal to the 9.2 register. 9.3 `httpdate` mentions: 1. 9.2 `map_http_status_with_hints` mentions: 1.
- `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: exit 0, "API surface unchanged" (see Deviation 1). `make security`: exit 0 (advisories, bans, licenses and sources ok).
- Regression: `cargo test -p paladin-ai --lib cadence` (11 passed, including `cadence_tracer_paces_a_real_openai_429_end_to_end`) and the root `unit` target's `test_anthropic_rate_limit_429` pass.
- Manual credential-handling review: the snapshot reads only rate-limit and `Date` headers, never a credential header; `RateLimitHints` has no string field; no new log line or error embeds a header value or body text; both rendered-error tests assert the raw `6m0s` and `2023-11-14T` strings are absent. Both HTTP clients keep `redirect::Policy::none()` and the 3xx pre-checks are untouched.

## Deviations from Plan

### Auto-fixed Issues

**1. [Plan acceptance wording] `.project/current-exports.txt` does not contain `hints_from_headers`**
- **Found during:** Task 3
- **Issue:** the acceptance line says the baseline contains `hints_from_headers`. The baseline is extracted from the `paladin` facade crate only, and the facade does not re-export `paladin_llm::rate_limit_headers` or `http_status`, so none of this plan's symbols can appear. Same situation as Deviation 3 of 43-01 and Deviation 2 of 43-02.
- **Fix:** none. `make api-surface` exits 0 with "API surface unchanged"; the symbols are covered by the 9.2 register (checked by `check-migration-allowlist.sh` and the plan's greps). No re-export was added just to satisfy the grep, which would widen the public surface.
- **Commit:** 35427737 (no baseline change)

**2. [Rule 1 - Bug, in my own test] A rendered-error assertion rejected a legitimately parsed number**
- **Found during:** Task 2 (green run)
- **Issue:** the OpenAI test asserted `149984` was absent from the `Debug` rendering, but `149984` is the parsed `remaining` integer and is correctly present. Only raw strings (`6m0s`) must be absent.
- **Fix:** dropped the `149984` assertion, kept `6m0s` and added a comment on the distinction.
- **Commit:** e4ea8602

**3. [Rule 3 - Blocking] OpenAI `get_available_models` also called `map_error`**
- **Found during:** Task 2
- **Issue:** making `map_error` a `cfg(test)` wrapper (to avoid a dead-code warning under `-D warnings`) broke the non-test build because `get_available_models` still called it.
- **Fix:** gave that path the same header snapshot and `map_error_with_hints`. No behaviour change for non-429 statuses.
- **Commit:** e4ea8602

**4. [Rule 3 - Blocking] Outer `///` doc on `pub mod rate_limit_headers` in `lib.rs`**
- **Found during:** Task 1
- **Issue:** the module has its own `//!` block; an outer `///` merged with it and duplicated the module example (the Phase 40-06 rustdoc rule).
- **Fix:** replaced the outer doc with a plain comment.
- **Commit:** cf1fb1a6

**Total deviations:** 4 (1 acceptance wording, 1 Rule 1, 2 Rule 3). **Impact:** none on behaviour.

## Notes

- The plan's `--doc` verify command was expected to be a no-op because `paladin-llm` sets `[lib] doctest = false`. It is not: the doctests do run under `cargo test -p paladin-llm --all-features --doc`, and they pass.
- The plan file's tail contained a stray embedded tool-call fragment (a `verify.plan-structure` invocation); it was ignored as instructed.
- Crate-level `CHANGELOG.md` entries for `paladin-llm` are owned by plan 43-13 per the phase artifact table, so the crate changelog was not touched here.

## Deferred Issues

- A full `cargo test --workspace --all-features` was not run (disk headroom about 7.9 GB). Run per-crate: `paladin-llm` all-features (595), `paladin-ai --lib cadence` (11), root `unit` target (the Anthropic 429 case). Workspace `clippy --all-targets --all-features -D warnings` and `fmt --check` are clean.
- Open for the 43-12 operator checkpoint: the OpenAI header names, `Retry-After` semantics and the `insufficient_quota` code string are still marked pending verification (research A1, A3, A4). If `insufficient_quota` is wrong the mapping never fires and the 429 is paced and bounded by `max_backoff_ms`.
- Other adapters (DeepSeek, compat family, Gemini) still build `rate_limited(None)` and ignore `Retry-After`; 43-04 routes them through the generic family of this same parser.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. Mitigations implemented: T-43-09 (checked parsing, `try_from_secs_f64`, saturating sums, 64-byte/8-group Go bound, ceiling clamp, table tests with NaN, inf, `1e400`, negatives and overflow), T-43-10 (numbers only; rendered-error tests), T-43-11 (clamp to 24 h here; D-06 `max_wait` refusal from 43-02), T-43-SC (`httpdate` already in the lockfile; `make security` passes with the direct edge). T-43-12 accepted: no HTTP client was added or changed.

## Next Phase Readiness

43-04 can call `hints_from_headers(RateLimitHeaderFamily::Generic, ..)` and `map_http_status_with_hints` from every remaining adapter and add the conformance case `rate_limit_is_surfaced_once_with_its_retry_delay`. Gemini's body-level `RetryInfo` stays unparsed this phase (OQ6).

## Self-Check: PASSED

`crates/paladin-llm/src/rate_limit_headers.rs` exists; commits cf1fb1a6, e4ea8602 and 35427737 are present in `git log` on `claude/laughing-dirac-e0h2ax`.
