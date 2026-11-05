---
status: testing
phase: 57-register-hygiene-close-out
source: 57-01-SUMMARY.md, 57-02-SUMMARY.md, 57-03-SUMMARY.md, 57-04-SUMMARY.md, 57-05-SUMMARY.md, 57-06-SUMMARY.md, 57-07-SUMMARY.md, 57-08-SUMMARY.md, 57-09-SUMMARY.md, 57-10-SUMMARY.md, 57-11-SUMMARY.md, 57-12-SUMMARY.md, 57-13-SUMMARY.md, 57-14-SUMMARY.md, 57-15-SUMMARY.md, 57-16-SUMMARY.md
started: 2026-11-05T00:55:10Z
updated: 2026-11-05T01:02:11Z
---

## Current Test
<!-- OVERWRITE each test - shows where we are -->

number: 15
name: Treasurer notices survive enqueue failure and client disconnect
expected: |
  If enqueueing the operator-delivery for a won once-per-window allowance notice fails, the notice is given back so the next admission in the window wins it again (no notice is lost). Treasurer::confirm and ::abandon run detached, so a client that disconnects mid-submission (or on the HTTP agent routes) does not cancel settlement, and a submission future dropped after admission still settles: confirm if the run row exists, otherwise give back. MIGRATION row M-B-44, a CHANGELOG Fixed bullet and the platform-api.md delivery-semantics sentence record this.
awaiting: user response

## Tests

### 1. Security gate passes with the fourth zip exception
expected: Run `make security` on the current head. cargo-audit and cargo-deny both exit 0. SECURITY-EXCEPTIONS.md lists four zip 3.0.0 rows (the newest is RUSTSEC-2026-0429) under the same compensating control, each re-dated per D-31 with review_date "2027-03-31 or the first planning day of the next milestone, whichever is earlier", and every advisory id in that register also appears in .cargo/audit.toml and deny.toml.
result: pass

### 2. Diagnosed debug sessions are archived
expected: `.planning/debug/resolved/` holds kimi-fallback-profile-stale.md, openai-max-completion-tokens.md and vendor-token-usage-fidelity.md (moved with git mv, history intact). No diagnosed-but-already-fixed session remains at the top of `.planning/debug/`, and each archived session is closed by a fixed WINDOWS row (76, 75, 74).
result: pass

### 3. Thread ownership follows the earliest attributed run
expected: With a thread whose first attributed run belongs to tenant A, a later Admin run or scheduled run on the same thread does not move it. Tenant A still sees the thread in the tenant thread list and through the single-thread gate, and submit/fork on it is permitted for A; a different tenant gets the same not-found answer as before. The paladin-web HTTP tracer over on-disk SQLite that proves this passes, and the five owner_run_for_thread_* contract clauses pass on the in-memory, SQLite and PostgreSQL run repositories.
result: pass

### 4. Owner-run rule is documented everywhere it is read
expected: openapi.json and the thread endpoint rustdoc describe ownership as the earliest attributed run. ADR-0063 is amended in place with that rule, quoting each replaced sentence. MIGRATION.md 11.1 rows M-B-24/M-B-25, section 11.6 and a new 11.8 operator step 12 (legacy mixed-tenant threads) state it, and both the root and paladin-web [Unreleased] changelogs carry a Fixed entry for the rule and the WINDOWS row 85 fix.
result: pass

### 5. Failed-run errors are redacted then bounded; empty webhook secret warns at boot
expected: A run that fails with a provider error that embeds a response body stores (and logs in the worker's failed-persist warning) text that is redacted first and then capped at the 2048-character budget, so no secret can straddle the cut. An operator-webhook secret configured as an empty string behaves as absent, and the server logs exactly one boot warning that the operator webhook has no secret; a configured non-empty secret still produces the same signature bytes as before.
result: pass

### 6. S3 listing pages correctly and S3 errors never carry a response body
expected: `list_files` with ListOptions.limit uses that limit as the S3 max-keys page size and walks continuation tokens, so no key is skipped across pages and has_more / continuation_token reflect the store's truncation state. Every S3 error rendered by the adapter (including retry exhaustion) reads only "HTTP <status> <code>" or a fixed label, never a body. The multipart rustdoc says a rejected part leaves the upload open and the caller must abort, and the presigned-URL integration test cannot print a signed URL.
result: pass

### 7. Credential-handling checklist C1-C6 re-run over the Phase 40 diff only
expected: 57-JUDGMENT-READ.md carries six credential-handling rows (C1-C6 from security.instructions.md) scoped to the Phase 40 diff range 629ef660...be3a9030, each with fresh file:line or re-run test evidence and a Human cell reading "confirm (2026-11-04)". You, as owner, stand by those six confirms. (Coverage D2, REG-04: credential-handling review is a manual control with no tool gate, so your confirm is the verdict.)
result: pass

### 8. Owner's bounded human read is recorded per row
expected: All 20 prohibition rows P40-01..P40-20 in 57-JUDGMENT-READ.md have a Human cell that reads "confirm (2026-11-04)" written from your own reply, with Claude's reading kept to the verdict/evidence/note columns, and 40-VALIDATION.md's Manual-Only row points at 57-JUDGMENT-READ.md as the recorded human read. You stand by that "confirm all". (Coverage D3, REG-04: a human verdict on LLM-judge results.)
result: pass

### 9. Review-dispositions checker passes strict over the full archive
expected: `./scripts/check-review-dispositions.sh` (strict, no --pending-ok) exits 0 and reports 0 failure(s) over 51 findings in 13 review files, deriving every WR/CR id (plus Phase 40 IN-01 and IN-04) from the archived NN-REVIEW.md headings rather than from the register. `tests/scripts/check-review-dispositions_test.sh` passes all 22 fixture cases (one firing fixture per failure class plus ok, --pending-ok and --phases cases). Each archived REVIEW.md for Phases 38-51 carries an appended "## Dispositions (Phase 57)" pointer section with nothing deleted.
result: pass

### 10. Register and triage agree: every finding closed or waived
expected: 57-REVIEW-DISPOSITIONS.md reads closed 43 / waived 8 / open 0 across all 51 findings. The "## Triage (Phase 57, 2026-11-04)" section records your option-a choice (7 fix-in-phase, 8 waive), each of the 8 waived rows names you as owner with the triage's revisit and basis text, and none of the 7 fix-in-phase items still carries a LIVE marker.
result: pass

### 11. WINDOWS ledger is fully dispositioned
expected: `.planning/WINDOWS.md` frontmatter reads open_count 0, waived_count 53, fixed_count 55, total_count 108. Rows 85-100 (minted during this phase) are each marked fixed on a re-run test, a re-measurement or a CI job id, and waiver rows 101-108 each name the owner of record plus revisit and basis. Nothing in the ledger would block `/gsd-ship`.
result: pass

### 12. Register evidence file ties the seal to the pushed head
expected: 57-REGISTER-EVIDENCE.md records the WINDOWS and review-disposition counts, the REG-01/03/04/05/06 read-backs, the phase gates, and the CI run and job ids on the pushed Phase 57 head (run 41927940508 on 4b115d511 or later), plus the orphaned-deferrals list; the head it names matches what is on origin for this branch.
result: pass

### 13. Reasoning-effort refusal shows a fixed label, never vendor text
expected: When check_reasoning fails with any LlmError other than UnsupportedParameter, PaladinBuilder::effort_refusal produces a ConfigurationError whose message is the fixed per-variant label only; no provider response text appears in the error or in logs. MIGRATION.md row M-B-41 and a CHANGELOG [Unreleased] Security bullet record the change.
result: pass

### 14. Worker emits its terminal event only after the commit, and failed attempts carry partial spend
expected: run_agent sends exactly one RunFinished wire event, and only after the exit's durable status/outcome writes and queue ack, on the completed, failed and both halt exits. A failed status or outcome write on the success path logs one warning (run id and repository error only) and nacks with a 1 s delay. When a run attempt fails after some LLM spend, the Err-arm NodeFinished event reports the spend billed before the failure. MIGRATION rows M-B-42/M-B-43 and two CHANGELOG Fixed bullets exist.
result: pass

### 15. Treasurer notices survive enqueue failure and client disconnect
expected: If enqueueing the operator-delivery for a won once-per-window allowance notice fails, the notice is given back so the next admission in the window wins it again (no notice is lost). Treasurer::confirm and ::abandon run detached, so a client that disconnects mid-submission (or on the HTTP agent routes) does not cancel settlement, and a submission future dropped after admission still settles: confirm if the run row exists, otherwise give back. MIGRATION row M-B-44, a CHANGELOG Fixed bullet and the platform-api.md delivery-semantics sentence record this.
result: [pending]

### 16. Cadence treats a zero provider delay as none and never loses a 429 record
expected: A 429 carrying Retry-After 0, a past HTTP-date, or a zero reset makes CadenceLlmAdapter::minimum_delay return None, so the port waits at least its own base back-off (escalating with streak) instead of opening a zero-length gate. note_failure records the 429 in a spawned task that the call awaits, so a caller dropped after the provider answered still leaves the record and the next call's gate reads it. paladin-llm and root CHANGELOG Fixed entries, MIGRATION rows M-B-45/M-B-46 and ADR-0058 (d)/(i) amendments exist.
result: [pending]

### 17. Node-cache fence marker never outlives its fencing counter
expected: The Redis node-cache fence marker TTL is clamp(2 x entry ttl, 5 min, 30 min), so for an on-time lock holder it expires no later than the Redis Cadence fencing counter that numbered it. Both TTL rules live once in the crate-private paladin-storage redis_fence module (feature-gated like redis_url) with no public item or feature-flag change. paladin-storage and root CHANGELOG Fixed entries, MIGRATION row M-B-47 and the ADR-0058 (g) amendment and (i) residual bullet exist.
result: [pending]

### 18. Worksheet of the 20 Phase 40 judgment-tier prohibitions, each with a verbatim statement, its 40-VERIFICATION row, fresh evidence and a tier
expected: Worksheet of the 20 Phase 40 judgment-tier prohibitions, each with a verbatim statement, its 40-VERIFICATION row, fresh evidence and a tier
result: pass
source: automated
coverage_id: D1

### 19. Phase 42 judgment-tier subset recorded as accepted as-is
expected: Phase 42 judgment-tier subset recorded as accepted as-is
result: pass
source: automated
coverage_id: D4

## Summary

total: 19
passed: 16
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps

[none yet]
