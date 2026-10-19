---
phase: 50-provider-parity-config-driven-model-swap
verified: 2026-10-19T16:30:00Z
status: gaps_found
score: 13/14 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 9/9
  gaps_closed:
    - "WR-03 (G-50-5a remainder (b)): a hopped-past fallback bill is priced against its own model and counted into ATTEMPT_SPEND at the hop, so a Battalion attempt cut off by the run deadline or idle bound while a later hop is in flight is charged for it once (WINDOWS row 78 fixed)"
  gaps_remaining: []
  regressions:
    - "SC6 clause 'make security stays green': cargo audit exits 1 on HEAD e45abf77 with RUSTSEC-2026-0367/0368/0369 on zip 3.0.0 (build-dependency of utoipa-swagger-ui 9.0.2). Not caused by Phase 50 (Cargo.lock unchanged since 3886b94c; advisories published 2026-10-09/10); the previous report accepted this clause from the 50-UAT.md CI record, which predates the audit run that now fails."
gaps:
  - truth: "SC6: make clean-code, make check-gates, make api-surface, make security and the 82 % coverage floor stay green"
    status: partial
    reason: "make security (make audit) fails on the current tree: cargo audit exits 1 with three vulnerabilities in zip 3.0.0 (RUSTSEC-2026-0367, -0368, -0369; patched in zip >= 9.0.0). zip is pulled in only as a build-dependency of utoipa-swagger-ui 9.0.2 (vendored swagger-ui extraction at build time), is not a runtime dependency, and was not touched by any Phase 50 plan. The project's security rule (security.instructions.md) treats a new advisory as a blocker unless allowlisted in .cargo/audit.toml with a reason, and SC6 names make security explicitly. All other SC6 clauses hold."
    artifacts:
      - path: "Cargo.lock"
        issue: "zip 3.0.0 (via utoipa-swagger-ui 9.0.2 [build-dependencies]) carries three open RustSec advisories"
      - path: ".cargo/audit.toml"
        issue: "none of RUSTSEC-2026-0367/0368/0369 is allowlisted; the ignore list ends at RUSTSEC-2026-0195"
    missing:
      - "Either bump utoipa-swagger-ui to a release that depends on zip >= 9.0.0 (then rerun make security and make api-surface), or add RUSTSEC-2026-0367/0368/0369 to .cargo/audit.toml with a written reason (build-time extraction of a vendored, trusted swagger-ui archive; no runtime exposure) and cite it in CHANGELOG/deferred-items"
      - "Run make security to exit 0 before the phase is sealed, per security.instructions.md"
deferred: []
human_verification: []
---

# Phase 50: Provider Parity & Configuration-Driven Model Swap Verification Report

**Phase Goal:** Any Paladin agent can be moved between the nine LLM providers by changing configuration only: equivalent behaviour, or an early build failure naming provider, model and parameter. Library-level APIs matter as much as server config.
**Verified:** 2026-10-19T16:30:00Z, on HEAD `e45abf77`
**Status:** gaps_found (one gap, outside the phase's own code; see Gaps Summary)
**Re-verification:** Yes. The prior report (2026-10-19T12:00Z, `passed`, 9/9) predates gap-closure plan 50-32 (code commit `7edec618`, docs `dea5ee5a`, ledger `13a87145`, review `e45abf77`). This pass re-checked the nine prior must-haves for regression and fully verified the five plan 50-32 truths, with particular attention to the G-50-5a clause about counting hopped-past fallback hops, which is the behaviour 50-32 changed. SUMMARY and REVIEW claims were treated as leads only; I read the diff `git diff 3886b94c..HEAD` for the four code paths and ran the named tests myself.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | ROADMAP SC1: nine-provider swap matrix, config-only, wire body and outcome asserted; red before the fixes | VERIFIED (regression-checked) | `tests/provider_swap_matrix.rs` untouched by 50-32 (`git diff --stat 3886b94c..HEAD` lists only the four code files plus docs/ledger). Plan 50-32's Task 3 gate recorded `cargo test -p paladin-ai --test provider_swap_matrix --features llm-all` at 52 passed on `7edec618`; no commit since touches `src/`, `crates/`, `Cargo.*` or `tests/`. Not re-run (integration binary; outside the targeted-test brief). |
| 2 | SC2: OpenAI sends `query`; per-provider/per-model `Range`/`Fixed`/`NotSent` under ADR-0061 | VERIFIED (regression-checked) | No adapter or `ModelProfile` file changed since the prior report. Orchestrator gate on this HEAD: `cargo test --workspace --lib --bins --no-fail-fast` = 14 binaries, 5169 passed, 2 failed (the pre-existing WINDOWS row 73 `run_api_wiring` pair, untouched by Phase 50). |
| 3 | SC3: `llm.<provider>` blocks via the public library API; nine facade pairs; YAML unchanged; api-surface refreshed | VERIFIED (regression-checked) | Orchestrator gate on this HEAD: `make api-surface` = 4298 items, unchanged. The two new `HoppedBills` methods live in `paladin-ports`, which the root facade does not re-export (A3-50-32), so the root baseline is unaffected. |
| 4 | SC4: dated per-model registry; Commissary/`resolve_context_window` read it; kimi-k3 at 1,048,576 | VERIFIED (regression-checked) | Registry files untouched since the prior report; covered by the green workspace lib run above. |
| 5 | SC5: every adapter maps empty output, typed refusal, usage cap, cache read/write, reasoning, stop sequences; fallback hops send/key their own model | VERIFIED (regression-checked) | Adapters untouched. `fallback.rs` received a docs-only edit (diff shows `//!` and `///` lines only; `after_failure` at 590-626 is byte-identical in logic: hop decision, `carry_hopped_bill` before `record_hop`, `AllProvidersFailed` on exhaustion). `cargo test -p paladin-llm --lib --all-features -- pricing:: fallback::` = 70 passed, 0 failed (18 + 52, matching the SUMMARY). |
| 6 | SC6: nine adapters in conformance suite and live smoke; mdBook page; MIGRATION/CHANGELOG carry every break; `make clean-code`, `make check-gates`, `make api-surface`, `make security`, 82 % coverage stay green; no library `unwrap`/`expect`/`panic!` | PARTIAL | Holds: `make build`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `make api-surface` green on this HEAD (orchestrator); `make check-gates`, `make check-migration-allowlist`, `make check-api-examples` exit 0 (50-32 Task 3); no `unwrap`/`expect`/`panic!` added outside `#[cfg(test)]` (scan below); MIGRATION DR-18 / M-B-21 and the three CHANGELOGs describe 50-32 as additive. **Fails:** `make security` is red. I ran `cargo audit` on HEAD: exit 1, three vulnerabilities in `zip 3.0.0` (RUSTSEC-2026-0367, -0368, -0369, dated 2026-10-09/10, patched `>= 9.0.0`). `cargo tree -i zip -e all` shows the only path is `[build-dependencies] utoipa-swagger-ui v9.0.2`; `Cargo.lock`/`Cargo.toml` are unchanged since `3886b94c`; `.cargo/audit.toml` does not allowlist them. Plan 50-32 logged this in `deferred-items.md` as out of scope, but SC6 and `security.instructions.md` ("must pass before a phase is sealed; treat a new advisory as a blocker unless allowlisted with a reason") make it a verification gap, not a note. |
| 7 | G-50-5b: OpenAI sends `max_completion_tokens` | VERIFIED (regression-checked) | `openai/adapter.rs` and `openai/vision.rs` untouched since the prior report; WINDOWS row 75 still `fixed`. |
| 8 | G-50-5c: Kimi adapter-wide fallback sends no sampling values | VERIFIED (regression-checked) | `kimi/adapter.rs` untouched; WINDOWS row 76 still `fixed`. |
| 9 | G-50-5a: every billable category reaches `TokenUsage`; vendor total reconciled; billed empty/refused calls carry usage, cost and a ledger row; failed/timed-out Battalion attempts **and hopped-past fallback hops** are counted | VERIFIED (hopped-hop clause re-verified in full, see WR-03 breakdown) | Unchanged clauses: `usage_reconcile.rs`, `response_guards::bill`, `BilledUsage`, `superstep.rs::race_attempt` (lines 677-708: fresh `SpendScope` installed outside the attempt future, `recorded_spend(&spend_scope)` on both expiry arms; `git diff --stat 3886b94c..HEAD -- crates/paladin-battalion/` is empty). The hopped-hop clause is now stronger than at the prior report: the bill reaches the attempt scope at the hop rather than at call return, so the "attempt dropped mid-call" case the prior report could not cover is covered by passing tests (rows 10-14). |
| 10 | 50-32 T1: a hopped-past bill reaches `ATTEMPT_SPEND` at the hop, priced; an attempt the run deadline or idle bound drops while a later hop is in flight reports hop 1's priced bill and the superstep settle charges it once; buffered and streamed | VERIFIED (behavioural) | `spend_scope.rs:462-474` `HoppedBills::record`: price through the installed pricer, then `record_incurred_spend` at once, then push with the returned `counted` flag; `record_hopped_bill` (509-511) calls it; `fallback.rs:614-616` calls `carry_hopped_bill` at the one hop decision point before `record_hop`. Ran: `hop_bill_counts_when_the_attempt_is_dropped_mid_call` (buffered, `tokio::time::timeout` around `ATTEMPT_SPEND.scope(.., service.execute(..))`, asserts `calls()==1`, `usage==u1`, `cost==Some(c1)` after the drop), `streamed_hop_bill_counts_when_the_attempt_is_dropped_mid_open`, and `engine_charges_a_hop_bill_when_the_attempt_is_cut_mid_call` (real `WarEngine` over the real service, `TimeoutKind::Run` and `TimeoutKind::Idle` variants, asserts the run's ledger row is `(c1, 1 settlement)`): all `ok`. |
| 11 | 50-32 T2: price first; `PricingLlmAdapter` installs its pricer before delegating; first pricer wins; bill priced against its own model via `price_or_warn`; never counted unpriced while a pricing decorator is in the stack; `cost: None` otherwise | VERIFIED (behavioural) | `pricing.rs:243-246` (`generate`) and `280-283` (`generate_stream`, on the handle captured before the open) call `install_hop_pricer` before `inner.generate*`; `install_hop_pricer` (213-216) closes over `Arc<PriceTable>` and prices `&bill.model` through `price_or_warn`. `install_pricer` uses `OnceLock::set(..).is_ok()` (382-387) and its rustdoc carries the plan's required sentence verbatim. Ran: `the_first_installed_pricer_wins`, `a_recorded_hop_bill_is_priced_and_counted_at_once`, `a_bill_with_no_pricer_is_counted_unpriced` (paladin-ports), `hop_bills_are_priced_and_counted_at_the_hop` (paladin-llm, both `generate` under a deadline and `generate_stream`): all `ok`. |
| 12 | 50-32 T3: exactly once; per-entry counted flag; service takes with `count_and_take` and never re-records hop bills; still folds and settles them in the call's one row with per-model breakdown; covers served, exhausted, structured, every stream exit, and retried-hop calls | VERIFIED (behavioural) | `count_and_take` (437-448) drains under the lock then counts only `!entry.counted`. Service take sites: buffered helper `3053`, stream open-error `4053`, terminal chunk `4137`, dropped receiver `4224`, terminal error `4250`, no-final-marker `4268`: all `count_and_take`; `grep` finds no remaining `hopped.take()`/`stream_hopped.take()`. Non-test `record_incurred_spend` calls (2014, 2063, 3797, 3807, 4048, 4129, 4244) each take only `failure.bills`, `response`, or `e.billed_usage()`, never a hop bill. Folding and settling preserved: `failure.hop_bills.iter().chain(&failure.bills)` fold (2006-2009) and `priced_bill_parts(&failure.hop_bills, ..).chain(..)` settle (2017-2021); success path folds `hop_bills` then settles in one row (2060-2078); `settle_streamed_call` chains hop parts then `own`. Ran: `count_and_take_counts_only_what_was_not_counted`, `push_neither_prices_nor_counts`, `hop_bills_are_counted_once_across_the_calls_retries` (asserts `calls()==3`, cost `2*c1+c2`, one ledger row), plus the six plan 50-30 tests `hopped_refusal_bill_is_settled_with_the_served_call`, `hopped_refusal_bill_is_settled_on_the_stream`, `hopped_bills_survive_chain_exhaustion`, `hopped_bills_survive_stream_exhaustion`, `structured_call_records_its_hopped_bill`, `a_call_with_no_hop_bill_settles_one_part`: all `ok`. |
| 13 | 50-32 T4: failing first; red output recorded; guards green before and after | VERIFIED | Structural: at `3886b94c`, `record_hopped_bill` is `HOPPED_BILLS.try_with(\|bills\| bills.push(billed))` (push only, no pricing, no scope record) and the file contains zero occurrences of `install_pricer`/`count_and_take`, so the five new paladin-ports tests cannot compile and the four behaviour tests must fail on their `calls()==1` / ledger-row assertions. The SUMMARY's recorded red messages ("hop 1's bill is already counted  left: 0 right: 1", "run-deadline: the boundary settles hop 1's priced bill once  left: None", "counted at the hop  left: 0 right: 1", E0599 on `install_pricer`/`count_and_take`) match the test sources verbatim. The red run itself is a process record I cannot replay; its content is consistent with the tree. |
| 14 | 50-32 T5: docs match the code; 0cdbc302 limit text removed; CHANGELOGs `### Added`; WINDOWS WR-03 row opened and closed via the tool | VERIFIED | `spend_scope.rs:82-96` subsection "Counted at the hop, however the attempt ends"; `grep -c 'when its model call ends'` is 0 in `spend_scope.rs` and `MIGRATION.md`. MIGRATION DR-18 (line 1018) describes `install_pricer` before delegation, `count_and_take`, the out-of-tree decorator rule, "additive, no compile break", "9.1 M-B-21". `crates/paladin-ports/CHANGELOG.md:27-31` lists both methods as additive; `crates/paladin-llm/CHANGELOG.md:84` and root `CHANGELOG.md:50-52,1197` describe the fix. WINDOWS row 78 (`spend_scope.rs`, WR-03, closing condition "priced and counted at its hop, proven by paused-clock drop tests") is `fixed`; frontmatter `open_count: 12, fixed_count: 29, total_count: 78`. ROADMAP line 242 now `[x]` and 50-32 plan line 1484 `[x]`. |

**Score:** 13/14 truths verified (0 present-but-behavior-unverified). The one PARTIAL is SC6's `make security` clause.

### WR-03 behaviour: confirmed

The behaviour plan 50-32 set out to change is in the tree and proven by tests I ran on HEAD `e45abf77`:

| Step | Where | Evidence |
|---|---|---|
| Pricer installed before the chain runs | `pricing.rs:243-246`, `280-283` | Both `generate` and `generate_stream` call `install_hop_pricer` before `inner.*`; first pricer wins (`OnceLock`). |
| Hop decided once, bill carried | `fallback.rs:590-626` | `after_failure`: only on a real hop (`hops_past` and a next element) does `carry_hopped_bill` run, before `record_hop`; an exhausted chain records nothing (bill stays in `AllProvidersFailed`). Unchanged logic from 50-30. |
| Priced and counted at the hop | `spend_scope.rs:462-474` | `record`: pricer (if any, if `cost.is_none()`) then `record_incurred_spend`, then push with `counted`; no lock held across either. |
| Not counted again | `spend_scope.rs:437-448`; service take sites listed in truth 12 | `count_and_take` skips counted entries; the service records only the call's own bills. |
| Engine reads it on a drop | `superstep.rs:692-704` (unchanged) | Scope outside the future; `recorded_spend` on run-deadline and idle expiry. |
| Proof | 3 root tests + 1 paladin-llm test + 5 paladin-ports tests | All pass (`cargo test -p paladin-ports --lib spend_scope`: 16 passed; `paladin-llm pricing:: fallback::`: 70 passed; root 10 named: 10 passed). |

### Disconfirmation pass

1. **Partially met requirement:** SC6's `make security` clause (gap above).
2. **A test that could pass without testing the claim:** the streamed drop test wraps `drain_final_chunk` and asserts after the timeout; because hop 1's bill is recorded inside the `ATTEMPT_SPEND` scope that the test holds outside the dropped future, the assertion genuinely depends on the at-hop record (the pre-fix code would leave `calls()==0`, which is exactly the recorded red). The engine test asserts the ledger row, not just the scope, so it also covers the superstep settle. No vacuous test found.
3. **An error path with no dedicated test:** the "stream ended without a final marker" exit (`4262-4273`) takes with `count_and_take` but no test drives a hop bill through that exit specifically. Under 50-32 the bill is already counted at the hop, so this exit is only the settle/safety net; `hopped_bills_survive_stream_exhaustion` covers the open-error exit. Info, not a gap.

### Required Artifacts

| Artifact | Status | Details |
|---|---|---|
| `crates/paladin-ports/src/output/spend_scope.rs` (`install_pricer`, `count_and_take`, counted flag, eager record) | VERIFIED | 810 lines; both `pub fn` present once each; private `HopEntry`, `HopPricer`, hand-written `Debug` using `try_lock` (never the closure); wired from `pricing.rs`, `fallback.rs`, the execution service. |
| `crates/paladin-llm/src/pricing.rs` (`install_hop_pricer` before delegation) | VERIFIED | Present on both paths; after-call `price_hopped_bills` passes retained as idempotent safety net. |
| `crates/paladin-llm/src/fallback.rs` | VERIFIED | Docs-only edit; hop logic unchanged. |
| `src/application/services/paladin/paladin_execution_service.rs` (`count_and_take` at every take site; `CallFailure.hop_bills`; three drop tests + retries guard) | VERIFIED | Six take sites; `hop_bill_counts_when_the_attempt_is_dropped_mid_call` at 9984; `StallingLlmPort` 9928; local `PaladinPortAdapter` 10009. |
| `MIGRATION.md` DR-18 / M-B-21 | VERIFIED | `count_and_take` and `install_pricer` named; consumer rules stated. |
| `.planning/WINDOWS.md` row 78 | VERIFIED | WR-03, `fixed`, both transitions via the tool (timestamps 13:00Z open, 15:50Z fixed). |

### Key Link Verification

| From | To | Via | Status |
|---|---|---|---|
| `pricing.rs` `generate`/`generate_stream` | `HoppedBills::install_pricer` | `HOPPED_BILLS.try_with(..install_hop_pricer..)` before `inner.*` | WIRED |
| `spend_scope.rs::record_hopped_bill` | `ATTEMPT_SPEND` | `HoppedBills::record` -> pricer -> `record_incurred_spend` -> `counted` flag | WIRED |
| execution service (6 take sites) | `HoppedBills::count_and_take` | buffered helper + five streamed exits | WIRED |
| `superstep.rs::race_attempt` | attempt `SpendScope` after drop | `recorded_spend(&spend_scope)` on both expiry arms (unchanged) | WIRED |
| `FallbackLlmAdapter::after_failure` | `record_hopped_bill` | `carry_hopped_bill` on a real hop only | WIRED |

### Data-Flow Trace (Level 4)

Not a rendering phase; the equivalent trace is "does a real bill reach the ledger": `engine_charges_a_hop_bill_when_the_attempt_is_cut_mid_call` reads back `InMemoryTreasuryLedger::spend` grouped by run and gets `(c1, 1)` for both timeout kinds. FLOWING.

### Behavioral Spot-Checks (run by me on HEAD `e45abf77`)

| Behavior | Command | Result |
|---|---|---|
| spend_scope unit tests incl. the five new ones | `cargo test -p paladin-ports --lib spend_scope` | 16 passed, 0 failed |
| Pricing decorator installs pricer; fallback regression | `cargo test -p paladin-llm --lib --all-features -- pricing:: fallback::` | 70 passed, 0 failed |
| WR-03 drop tests, retries guard, six 50-30 exactly-once tests | `cargo test -p paladin-ai --lib -- <10 names>` | 10 passed, 0 failed |
| Security gate | `cargo audit` | **exit 1**: 3 vulnerabilities (`zip 3.0.0`) plus allowed unmaintained warnings |
| `zip` provenance | `cargo tree -i zip -e all` | only `[build-dependencies] utoipa-swagger-ui v9.0.2` |

Cited from the orchestrator (not re-run): workspace lib/bins 5169 passed / 2 failed (row 73 pair); `make api-surface` 4298 unchanged; `make build`, `cargo fmt --check`, `cargo clippy -- -D warnings` green. Not re-run: the matrix integration test, coverage, Docker suites, mdBook, live smoke (prior report and `50-UAT.md`; no code since touches them). No ENOSPC was hit (11 GB free at start).

### Probe Execution

Step 7c: no `probe-*.sh` scripts are declared by the phase or exist under `scripts/*/tests/`. The live probe is the `live_vendor_smoke` example recorded in `50-SEED-PROBE.md` / `50-USAGE-PROBE.md`. SKIPPED (needs vendor credentials; operator run on record; unaffected by 50-32).

### Requirements Coverage

Plan 50-32 declares `PARITY-05, PARITY-06`. All eight PARITY IDs are `[x]` in REQUIREMENTS.md and mapped `Phase 50 | Complete` (lines 456-462); no orphans.

| Requirement | Status | Evidence |
|---|---|---|
| PARITY-05 | SATISFIED | Truths 5, 9, 10-12 (billed refusals counted at the hop) |
| PARITY-06 | SATISFIED | Truth 11: each hop bill priced against its own model (D-15) |
| PARITY-01..04, 07, 08 | SATISFIED (regression-checked) | Truths 1-4, 6 (conformance clauses), 7 |

### Anti-Patterns Found

Scan of all `+` lines in `git diff 3886b94c..HEAD` for the four code files: every `unwrap`/`expect`/`panic!` hit is inside a `#[cfg(test)]` region (`spend_scope.rs` tests begin at 514; `pricing.rs` tests at 354 behind `feature = "mock"`; root hunks at 8370+ are test modules). No `TODO`/`FIXME`/`XXX`/`TBD`/`HACK` added. `HoppedBills::lock` and `Debug` recover poisoned locks without panicking.

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `Cargo.lock` | 7467 | `zip 3.0.0` with RUSTSEC-2026-0367/0368/0369 (build-dep of `utoipa-swagger-ui 9.0.2`) | WARNING (gap) | `make security` red; SC6 clause fails. Not introduced by Phase 50. |
| `paladin_execution_service.rs` | 410-416, 3004-3010 | Stale/garbled rustdoc on `CallFailure` and the `# Errors` of `execute_with_retry_and_temperature` still says `bills` carries hop bills (review IN-01) | Info | Private-item docs; code is correct (`hop_bills` is separate). Worth a docs follow-up. |
| `fallback.rs` | 230, 234 | Doc lines over 100 columns (IN-02) | Info | Style only; `cargo fmt --check` passes. |
| `spend_scope.rs` | 355-365 / 437-448 | A bill counted unpriced can later be priced by `price_unpriced` for the ledger but not the scope (IN-03) | Info | Documented; unreachable in-tree because `PricingLlmAdapter` installs its pricer first. |
| `spend_scope.rs` | 352-355 | Lower-case fragment in `install_pricer` rustdoc (IN-04) | Info | The fragment is the sentence plan 50-32's must-have requires verbatim; a style conflict, not a defect. |
| WINDOWS row 73 | | `run_api_wiring` pair fails in this sandbox | Info | Pre-existing since the 10-13 baseline; not a Phase 50 item. |
| WINDOWS row 77 | | Grok `cost_in_usd_ticks` not parsed | Info | Owner-accepted deferral; unchanged. |

### Human Verification Required

None new. The prior report's human items remain closed by `50-UAT.md`; the 50-32 behaviours are covered by passing automated tests.

### Gaps Summary

The phase goal is achieved in code and WR-03 is confirmed closed: a hopped-past fallback bill is priced against its own model and counted into the attempt scope at the hop, counted exactly once, and charged by the Battalion superstep settle even when the engine drops the attempt mid-call, on the buffered and streamed paths, with the real engine. All nine prior must-haves hold on `e45abf77` except one clause of SC6.

The single gap is SC6's `make security stays green`: `cargo audit` exits 1 on this tree because of three RustSec advisories (published 2026-10-09/10) against `zip 3.0.0`, which enters only as a build-dependency of `utoipa-swagger-ui 9.0.2`. Phase 50 did not add or change this dependency, and plan 50-32 logged it in `deferred-items.md`, but `security.instructions.md` says `make security` must pass before a phase is sealed and that a new advisory is a blocker unless allowlisted with a reason, and SC6 names the gate. No dependency bump closes it: every published `utoipa-swagger-ui` release pins `zip` below the patched 9.0.0 (9.0.2 requires `^3.0`, 10.0.0/10.0.1 require `^8.6`; checked against the crates.io index on 2026-10-19). The advisories concern `ZipArchive::extract`/`ZipStreamReader::extract` (symlink targets, permissions through symlinks, SUID/SGID bits); the `utoipa-swagger-ui` build script never calls `extract`, it walks entries with `enclosed_name()` and writes them itself, and with the `vendored` feature the archive is the crate's own embedded bundle, read at build time only. Closing the gap is therefore an owner decision (as `50-VALIDATION.md` already records): allowlist the three IDs in `SECURITY-EXCEPTIONS.md` with that rationale, mirrored into `.cargo/audit.toml` and `deny.toml` per ADR-0024, then rerun `make security` and `scripts/check-advisory-register.sh`.

**This may be intentional.** If the owner accepts the `deferred-items.md` deferral and wants the phase sealed with the advisory open, add to this file's frontmatter and re-run verification:

```yaml
overrides:
  - must_have: "SC6: make security stays green"
    reason: "RUSTSEC-2026-0367/0368/0369 are on zip 3.0.0, a build-only dependency of utoipa-swagger-ui 9.0.2 (vendored swagger-ui extraction); no runtime exposure; not touched by Phase 50; fix tracked in deferred-items.md for a dependency-owning plan"
    accepted_by: "<owner>"
    accepted_at: "<ISO timestamp>"
```

---

_Verified: 2026-10-19T16:30:00Z_
_Verifier: Claude (gsd-verifier)_
