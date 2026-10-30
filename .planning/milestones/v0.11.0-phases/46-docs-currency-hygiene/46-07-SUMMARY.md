---
phase: 46-docs-currency-hygiene
plan: 07
subsystem: docs
tags: [mdbook, treasurer, allowance, pricing, cadence, adr-index, config-reference]

requires:
  - phase: 38-43
    provides: the built Treasurer (ledger, pricing, admission, mid-run halt) and Cadence
provides:
  - "docs/src/architecture/treasurer.md: the Treasurer concept page, reachable from the book's Architecture navigation"
  - "configuration.md: one ## Treasurer block with a shared key reference, a new Treasurer pricing section, and the regrouped allowance and Cadence sections"
  - "config.example.yml: commented cadence block with the defaults and the Redis url_env form"
  - "adr-index.md: row 0050 refreshed and rows 0052-0054, 0056-0062 added"
affects: [46-13 upgrading.md and migration pointers, 46-14 citation retargeting]

tech-stack:
  added: []
  patterns:
    - "One shared key-reference table (Key, Type, Default, Validation, Env override) for a whole config subtree"
    - "Concept page mirrors commissary.md's shape; HTTP contracts linked, never duplicated"

key-files:
  created:
    - docs/src/architecture/treasurer.md
  modified:
    - docs/src/SUMMARY.md
    - docs/src/contributing/adr-index.md
    - docs/src/getting-started/configuration.md
    - config.example.yml

key-decisions:
  - "Cadence is one section of the Treasurer page (D-08); no separate Cadence page exists"
  - "Treasurer page states bounded overshoot and the tumbling-window double-spend explicitly rather than promising an allowance can never be overspent"
  - "ADR 0055 (dev/test reference object store) left out of the index under the index's own consumer/operator-visibility rule"
  - "The webhook secret appears only as the env variable name; no YAML block in either file carries a secret value"

patterns-established:
  - "Interim anchor then switch: Task 1 linked #treasurer-allowances, Task 2 switched to #treasurer once the block existed"

requirements-completed: [CURR-23]

duration: 26min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 07: Treasurer page and configuration reference Summary

**A Treasurer concept page in the book's Architecture section and one `## Treasurer` configuration reference (shared key table, new pricing contract, exact warn and period boundaries, Cadence), with a commented `cadence:` block in the example config and a current ADR index.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-10-24T13:55:10Z
- **Completed:** 2026-10-24T14:21:17Z
- **Tasks:** 2
- **Files modified:** 5 (1 created)

## Accomplishments
- `treasurer.md` covers the append-only ledger derived on read (ADR-0053), allowance admission in tumbling UTC windows (ADR-0056), mid-run enforcement and the halt contract (ADR-0052, ADR-0057), pricing as the cost source and one `## Rate pacing (Cadence)` section (ADR-0058); its type table names exactly the ten required items and the page is listed after Commissary in `SUMMARY.md`.
- `configuration.md` gains one `## Treasurer` block: a single key table covering every public field of `TreasurerConfig`, `PriceRowConfig`, `AllowanceConfig`, `AllowanceEntryConfig`, `AllowanceWebhookConfig`, `CadenceConfig` and `CadenceBackend` with all ten `APP_TREASURER_*` overrides; a new `### Treasurer pricing` section (nine-place decimal contract, half-up-once cost rule, saturating sums, the three unpriced-model cases including WINDOWS row 67); the allowance and Cadence sections demoted with unchanged heading text so the inbound anchors still resolve.
- The warn rule is stated as `balance x 100 >= ceiling x warn_at` with a 79/80/81 percent example (`0` never warns, `100` never warns, above `100` rejected); the allowance period range `1m` to `366d` is stated with `0m` and `367d` rejected.
- `config.example.yml` gains a commented `cadence:` block (all defaults plus the Redis `url_env` form); the three example-config tests still pass.
- `adr-index.md`: row 0050 now says built in v0.11.0 and links the page; ten rows added in number order.

## Task Commits

1. **Task 1 (tracer): Treasurer concept page, SUMMARY entry, ADR index** - `19192b46` (docs)
2. **Task 2: one Treasurer configuration reference and the example config** - `f300f59c` (docs)

**Plan metadata:** the docs commit that follows this SUMMARY (STATE, ROADMAP, REQUIREMENTS).

## Files Created/Modified
- `docs/src/architecture/treasurer.md` - the Treasurer concept page
- `docs/src/SUMMARY.md` - Treasurer entry after Commissary
- `docs/src/contributing/adr-index.md` - refreshed row 0050; rows 0052-0054 and 0056-0062
- `docs/src/getting-started/configuration.md` - `## Treasurer` block, key reference, pricing section, regrouped sections, exact warn and period rules
- `config.example.yml` - commented `cadence:` block

## Decisions Made
- Followed the plan as specified. Honesty choices: the page states bounded overshoot (one superstep, one model response, one streamed call, same-instant admissions, tumbling windows) and names DNS rebinding as a documented limitation of the webhook address guard; it claims no role bypasses an allowance and says an unpriced WarGraph node call settles nothing.

## Deviations from Plan

None - plan executed exactly as written. Tracer gate (auto semantics): the tracer's `<verify>` (static checks, `make check-doc-config`, `make check-doc-examples`, the three guard tests) passed before the tracer commit and again on the committed content (the guards and doc checks were re-run after Task 2 on the same page); expansion proceeded. `cargo test --offline` worked from the local cache, so no online fallback was needed.

**Total deviations:** 0.

## Issues Encountered
- None. `mdbook` is not installed here, so the book build and `mdbook-linkcheck` legs are CI-only; every relative link target on the new page was checked with `test -f`, the new nav entry and anchors were checked by inspecting `SUMMARY.md` and the headings directly.

## Verification Run
- Task 1 verify command: static checks pass; `make check-doc-config` (160 blocks, 0 failed), `make check-doc-examples` (0 failed), `treasurer_vocabulary_guard` (3), `entity_rename_guard` (8), `legacy_removal_guard` (3) pass.
- Task 2 verify command: verify script prints `missing []`; `make check-doc-config` (161 blocks, 0 failed); `cargo test --offline -p paladin-ai --lib example_config` (3 passed); the three guards pass.
- Plan-level: `make check-gates` exits 0; `make check-doc-examples` exits 0. `make security` was not run (known unrelated cargo-deny advisory RUSTSEC-2026-0386, tokio, pre-existing).
- Credential review: no YAML or Rust example in either file carries a secret value; the webhook secret is named only as `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET`.

## Known Stubs

None.

## Threat Flags

None - docs-only plus commented example config; no new endpoint, auth path or schema.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Plans 46-13 and 46-14 can link `architecture/treasurer.md` and `configuration.md#treasurer`. The Phase 46 CURR-23 book pages for the Treasurer are done; the remaining CURR-23 items (MIGRATION guide, upgrading page) belong to other plans.

## Self-Check: PASSED

Created and modified files exist on disk; commits `19192b46` and `f300f59c` are present in `git log`.

---
*Phase: 46-docs-currency-hygiene*
*Completed: 2026-10-24*
