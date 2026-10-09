---
phase: 43-rate-pacing
plan: 12
subsystem: llm-provider-headers
tags: [cadence, rate-limit-headers, openai, anthropic, verification, evidence, rustdoc]

requires:
  - phase: 43-rate-pacing
    provides: rate_limit_headers.rs header constants and parser, the OpenAI insufficient_quota branch (43-03, 43-04); the completed Cadence stack (43-11)
provides:
  - 43-PROVIDER-HEADER-EVIDENCE.md, a per-name verification record (22 names) with source URL, date, status and quote or operator attestation
  - verified-source rustdoc on every rate_limit_headers.rs constant and on the OpenAI INSUFFICIENT_QUOTA code
affects: [43-13]

tech-stack:
  added: []
  patterns:
    - "Fetch what the sandbox can reach, record a verbatim quote, and route only the unreachable remainder to one operator checkpoint"
    - "Distinguish VERIFIED (quote) from VERIFIED (operator, date): an attestation is never presented as a fetched quote"

key-files:
  created:
    - .planning/phases/43-rate-pacing/43-PROVIDER-HEADER-EVIDENCE.md
  modified:
    - crates/paladin-llm/src/rate_limit_headers.rs
    - crates/paladin-llm/src/openai/adapter.rs

key-decisions:
  - "OpenAI rows are recorded VERIFIED (operator, 2026-10-09) with no quote column content, because the sandbox egress proxy blocked every OpenAI host; only Anthropic rows carry fetched verbatim quotes"
  - "The operator confirmed retry-after-ms is not documented by OpenAI, so it stays an optional extra parsed for the OpenAI-compatible family and is documented as such rather than as a verified OpenAI header"
  - "No constant, parser test or quota branch changed: the operator gave no corrections"

patterns-established:
  - "Rustdoc cites the verified source, date and evidence file for every provider wire name the code depends on"

requirements-completed: [PACE-01]

duration: 2 sessions (Task 1 before the operator checkpoint, Tasks 2-3 after)
completed: 2026-10-09
status: complete
---

# Phase 43 Plan 12: Provider Header Verification Summary

**Every provider rate-limit header name and error code the Cadence relies on is now recorded against an official source: Anthropic's fetched and quoted verbatim, OpenAI's confirmed by the operator, and each cited in the constant's rustdoc.**

## Performance

- **Tasks:** 3 of 3 (Task 2 was the operator checkpoint, resolved with "Pass")
- **Files modified:** 3 (1 created, 2 edited)

## Accomplishments

- Fetched `platform.claude.com/docs/en/api/rate-limits` and `/errors` and quoted verbatim: `retry-after`, the twelve `anthropic-ratelimit-{requests,tokens,input-tokens,output-tokens}-{limit,remaining,reset}` headers (RFC 3339 resets), and `enforced_spend_limit_reached` (the spend-cap 429 has no `retry-after`).
- Recorded that `platform.openai.com`, `developers.openai.com`, `help.openai.com` and `cookbook.openai.com` are unreachable from the sandbox, listed the ten OpenAI items under "For the operator", and applied the operator's approval: the six `x-ratelimit-*` names, the Go-style duration reset format, `Retry-After` semantics, `retry-after-ms` not being documented, and `insufficient_quota`.
- Replaced every "Pending operator verification" sentence in `rate_limit_headers.rs` (eight "Verified against" citations now) and in `openai/adapter.rs` with source, date and evidence-file citations, and added the Anthropic citation to the header family constant.

## Task Commits

1. **Task 1: Verify provider header names, write evidence record** - `6596aea5` (docs)
2. **Task 2: Operator confirmation checkpoint** - no commit; resolved by the operator's "Pass" on 2026-10-09
3. **Task 3: Record outcome, verified-source rustdoc** - `04c0ed67` (docs)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Bare URLs in rustdoc failed `RUSTDOCFLAGS='-D warnings'`**
- **Found during:** Task 3 verification
- **Issue:** the verified-source sentences contained bare `https://...` URLs, tripping `rustdoc::bare_urls`
- **Fix:** wrapped each URL in angle brackets
- **Files modified:** `crates/paladin-llm/src/rate_limit_headers.rs`, `crates/paladin-llm/src/openai/adapter.rs`
- **Commit:** `04c0ed67`

**2. [Scope note] `openai/adapter.rs` rustdoc edited**
- The plan's Task 3 lists this file only for a possible `insufficient_quota` correction, but the `INSUFFICIENT_QUOTA` constant carried its own "Pending operator verification" sentence tied to this checkpoint, so it was replaced with the verified-source form too. No behaviour changed.

**Total deviations:** 1 auto-fixed, 1 scope note. **Impact:** none on behaviour.

## Verification

- `cargo test -p paladin-llm --all-features --lib`: 637 passed, 0 failed
- `cargo test -p paladin-llm --all-features --doc rate_limit_headers`: 5 passed
- `cargo clippy -p paladin-llm --all-features --all-targets -- -D warnings`: clean
- `cargo fmt --check`: clean
- `RUSTDOCFLAGS='-D warnings' cargo doc -p paladin-llm --no-deps --all-features`: clean
- `grep -c 'Pending operator verification'` on `rate_limit_headers.rs`: 0; `grep -c 'Verified against'`: 8

## Caveats

- The OpenAI rows rest on the operator's attestation, not a quote fetched by the executor. This is stated in the evidence file and in the status column, per threat T-43-46 (no name is marked verified from search excerpts or third-party pages).
- The security.instructions.md manual credential-handling review is unaffected: only documentation comments changed in credential-adjacent files. `make security` was not re-run for a comment-only change.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- FOUND: `.planning/phases/43-rate-pacing/43-PROVIDER-HEADER-EVIDENCE.md`
- FOUND: commits `6596aea5` and `04c0ed67`
