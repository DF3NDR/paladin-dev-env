---
phase: 43-rate-pacing
plan: 05
subsystem: llm-pacing
tags: [cadence, rate-limit, 429, contract-tests, streaming, jitter, bounded-state, semver]

requires:
  - phase: 43-rate-pacing
    provides: CadencePort and InMemoryCadence (43-01); CadenceLlmAdapter with the typed retry_after minimum and the D-06 max_wait refusal (43-02)
provides:
  - paladin_storage::cadence::contract_tests, the shared CadencePort contract suite (11 clauses, run_all, run_all_paused) that 43-07 and 43-08 run unchanged
  - InMemoryCadence::with_capacity and DEFAULT_KEY_CAPACITY (4096), with idle-first then soonest-expiring eviction and one capacity-only warning
  - generate_stream paced like generate in CadenceLlmAdapter (gate before delegating, call-level and first-item 429 recorded, first Ok chunk resets the streak, peeked item re-attached)
  - a per-waiter additive spread, spread = u * min(wait / 10, 1 s), so a cleared gate does not release a herd
  - rand 0.8 as a required paladin-llm dependency
affects: [43-06, 43-07, 43-08, 43-09, 43-13]

tech-stack:
  added: []
  patterns:
    - "One pub async fn per contract rule over &dyn Port plus the policy, jitter pinned by the caller, run at both jitter ends, a plain (non-cfg(test)) module so an integration tier can reuse it"
    - "A pure, clamped, never-subtracting spread function (waiter_spread) separate from the randomness, so the arithmetic is table-tested"
    - "First-item peek and re-attach (stream::iter(peeked).chain(rest)) to observe a stream's outcome without consuming it"

key-files:
  created:
    - crates/paladin-storage/src/cadence/contract_tests.rs
  modified:
    - crates/paladin-storage/src/cadence/in_memory.rs
    - crates/paladin-storage/src/cadence/mod.rs
    - crates/paladin-llm/src/cadence.rs
    - crates/paladin-llm/Cargo.toml
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "The paused-clock clause uses tokio::time::sleep, not advance: advance needs tokio's test-util feature, which a plain module cannot assume (and which is dev-only in paladin-storage); on a paused clock an idle sleep jumps to exactly its deadline and every duration is a whole millisecond"
  - "The saturation clause drives the streak with a zero provider delay (each record finds the gate already clear and escalates), so a streak of 10 000 needs no sleeping and the same clause stays feasible against a live Redis"
  - "note_outcome was split into note_success and note_failure so the stream path never holds a &Result<Box<dyn Stream + Send>> across an await (that type is not Sync)"
  - "No new public setting for the spread: it is internal, and the only tuning knobs stay the treasurer.cadence keys"

patterns-established:
  - "Contract clauses are jitter-agnostic: they assert the floor (wait >= base, never near-zero thrash) and the ceiling (min(max, base * 2^(streak-1))) that hold for every fraction; the backend runs the suite at jitter 0.0 and near 1.0"
  - "Tolerance for non-paused comparisons is half the base back-off; only the _paused clause asserts exact instants"

requirements-completed: [PACE-02]

duration: about 2 h wall clock (four commits between 14:55 and 15:25 UTC plus verification)
completed: 2026-10-08
status: complete
---

# Phase 43 Plan 05: Cadence contract suite, bounded state, stream gating and waiter spread Summary

**Every PACE-02 edge rule is now an executable CadencePort contract clause the in-process adapter passes at both jitter ends; streamed calls are paced like buffered ones; waiters one gate releases are spread without ever undercutting a provider delay; and in-process pacing state is bounded at 4096 keys.**

## Performance

- **Tasks:** 3 of 3
- **Commits:** 1c73259d, 93b122d9, d012e7bc (fix), 22787e39
- **Files:** 1 created, 6 modified

## Accomplishments

- `contract_tests.rs`: clauses `unknown_key_is_clear_and_creates_no_state`, `first_delayless_429_gates_for_base`, `gate_boundary_is_closed_at_not_before_paused`, `escalation_doubles_the_ceiling_after_the_gate_clears`, `in_flight_429_does_not_escalate`, `explicit_retry_after_raises_but_never_lowers_an_active_gate`, `explicit_retry_after_is_the_exact_gate`, `success_resets_the_streak_and_is_idempotent`, `streak_saturates_without_overflow`, `keys_are_exact_byte_equal`, `concurrent_429s_escalate_at_most_once`, plus `run_all` and `run_all_paused`. Each key is namespaced by its clause; a private poll-based `join_all` drives the 16 concurrent recorders without adding a futures dependency.
- `InMemoryCadence`: `DEFAULT_KEY_CAPACITY = 4096`, `with_capacity` (0 treated as 1). A new key at the cap first reclaims entries whose gate elapsed with streak 0, then evicts the entry with the earliest `not_before`; one `log::warn!` under `paladin::cadence` names the capacity only (a test asserts no key or model text appears).
- `CadenceLlmAdapter::generate_stream`: waits on the gate (including the D-06 refusal) before delegating; a call-level 429 is recorded and returned unchanged; the first item is peeked and re-attached, a first-item 429 is recorded, a first `Ok` resets a non-zero streak, later items (including a mid-stream error) pass through unrecorded.
- Waiter spread: after a non-zero reading each waiter sleeps `wait + u * min(wait / 10, 1 s)` (saturating add, fraction clamped, NaN as zero) and re-reads the gate before sending. Module docs gained the streaming rule and the spread rule.
- `rand = { workspace = true }` is a plain `paladin-llm` dependency; every `dep:rand` entry is gone from the provider feature lists. `cargo tree -p paladin-llm --no-default-features -e normal --depth 1 | grep -c 'rand v'` prints 1.
- Registered: MIGRATION 9.3 (rand required), 9.2 row (`N`, so the allowlist stays set-equal), the 9.1 M-B-05 row (streaming and spread), CHANGELOG bullet.

## Red / green record

- **Task 1:** the in-memory test module (13 contract invocations and the capacity tests) was written first; it failed to compile (`no method named with_capacity`). After the implementation two of my own tests failed (an entry-count assertion placed after a clause that deliberately ends with a 429, and a "never names a key" assertion that collided with the word "soonest" in the message); both were test bugs, fixed. 22 pass. Mutation check: disabling the in-flight branch (`if false && now < entry.not_before`) made five contract tests fail (`concurrent_429s_escalate_at_most_once`, `explicit_retry_after_raises_but_never_lowers_an_active_gate`, `in_flight_429_does_not_escalate`, and both `run_all` tests), then the file was restored from a backup.
- **Task 2:** tests written first against a stub `waiter_spread` returning zero; 8 failed (`stream_waits_on_the_gate_before_delegating`, `stream_gate_beyond_max_wait_is_refused_without_calling_the_provider`, `stream_call_level_429_is_recorded_and_returned_unchanged`, `stream_first_item_429_is_recorded_and_the_item_is_still_delivered`, `stream_first_ok_chunk_resets_a_nonzero_streak`, the 20-caller test and both `waiter_spread` tests). `dropped_waiting_future_leaves_no_state`, `spread_never_reduces_an_explicit_retry_after` and `stream_error_after_the_first_item_passes_through_unrecorded` pass before the change by design (they pin invariants the old code already had and the new code must keep). 27 cadence tests pass after.

## Task Commits

1. **Task 1: contract suite and bounded in-process state** - `1c73259d`
2. **Task 2: stream gating, waiter spread, rand required** - `93b122d9`
3. **Fix to Task 1: no tokio test-util in the plain contract module** - `d012e7bc`
4. **Task 3: register (MIGRATION 9.1/9.2/9.3, CHANGELOG)** - `22787e39`

## Verification

- `cargo test -p paladin-storage --lib cadence`: 22 passed (includes both capacity tests); `cargo test -p paladin-storage --lib`: 253 passed.
- `cargo test -p paladin-llm --lib cadence`: 27 passed (acceptance asked for 16); `cargo test -p paladin-llm --all-features --lib`: 606 passed; `cargo build -p paladin-llm --no-default-features` and `cargo clippy -p paladin-llm --no-default-features -- -D warnings`: clean.
- `cargo test -p paladin-ai --lib cadence`: 11 passed, including the 43-01 end-to-end tracer.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`: clean; `cargo fmt --check`: clean; `RUSTDOCFLAGS='-D warnings' cargo doc -p paladin-llm -p paladin-storage --no-deps --all-features`: clean.
- `./scripts/check-migration-allowlist.sh`: set-equal; 9.3 `rand` mentions: 2; `make security`: exit 0; `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface`: "API surface unchanged" (see Deviation 1).
- Manual credential-handling review: no new log line carries a key, model, URL or header value. The only new warning names the capacity; the existing trace line names provider, model and durations.

## Deviations from Plan

### Auto-fixed Issues

**1. [Plan acceptance wording] `.project/current-exports.txt` does not contain `DEFAULT_KEY_CAPACITY`**
- **Found during:** Task 3
- **Issue:** the acceptance line expects the baseline to contain `DEFAULT_KEY_CAPACITY`. The baseline is extracted from the `paladin` facade crate only, which does not re-export `paladin-storage::cadence`, so no sub-crate symbol can appear (same as Deviation 3 of 43-01, 2 of 43-02, 1 of 43-03).
- **Fix:** none. `make api-surface` exits 0 ("API surface unchanged") and the symbol is covered by the 9.2 register. No re-export was added.
- **Commit:** 22787e39 (no baseline change)

**2. [Rule 1 - Bug, in my own Task 1 commit] `tokio::time::advance` in a non-test module**
- **Found during:** Task 3 (`make api-surface` failed to build rustdoc: `advance` is gated behind tokio's `test-util`)
- **Issue:** `contract_tests` is deliberately plain (not `cfg(test)`), but its paused clause called `tokio::time::advance`, which only compiled because `test-util` is a dev-dependency. A non-test `cargo build` feature set, and the rustdoc build, rejected it.
- **Fix:** the clause uses `tokio::time::sleep`; all values are whole milliseconds so the exact-instant assertions still hold on a paused clock (the paused test stayed green). Also verified with `cargo build -p paladin-storage`.
- **Files modified:** `crates/paladin-storage/src/cadence/contract_tests.rs`
- **Commit:** d012e7bc

**3. [Rule 3 - Blocking] Two 43-02 tests asserted the wait exactly**
- **Found during:** Task 2
- **Issue:** `gate_of_exactly_max_wait_is_waited_out` asserted the elapsed time equals 300 s and `reset_derived_delay_is_clamped_to_max_backoff` equalled `max_backoff`; the spread now (correctly) lengthens a wait by up to 1 s.
- **Fix:** both assert `>= delay` and `<= delay + 1 s + 2 ms` (timer rounding). The minimum is still enforced exactly.
- **Commit:** 93b122d9

**4. [Rule 3 - Blocking] clippy `collapsible_if`**
- **Found during:** Task 2
- **Issue:** `note_success` had a nested `if`/`if let` that `clippy -D warnings` rejects.
- **Fix:** a let-chain (edition 2024).
- **Commit:** 93b122d9

**5. [Extra beyond the plan] 9.1 row M-B-05 extended**
- Streaming now being paced and the waiter spread are user-visible behaviour, so the existing behavioral-change row notes them (the plan listed only 9.3, 9.2 and CHANGELOG). No new row, no new allowlist entry.

**Total deviations:** 5 (1 acceptance wording, 1 Rule 1, 2 Rule 3, 1 additive docs). **Impact:** none on the planned behaviour.

## Incident

While running the mutation check I restored the file with `git checkout <file>` after copying a backup; because Task 1 was not yet committed this discarded the working copy, and the backup copy (taken just before the mutation) was used to restore it byte-for-byte. The full suite was re-run green afterwards and nothing was lost. Lesson recorded: use a backup-and-copy-back (or commit first) for mutation checks, never `git checkout` on uncommitted work.

## Deferred Issues

- Cadence tolerance in the non-paused clauses is half the base back-off. Plan 43-07 should confirm that is wide enough for a real Redis round trip with the base it picks (tens of milliseconds); if it is flaky there, widen `slack()` in `contract_tests.rs` rather than weakening an assertion.
- The saturation clause performs 10 000 sequential `record_rate_limited` calls; against live Redis that is on the order of seconds. If 43-07 finds it too slow, lower `SATURATION_STREAK` (it is a public constant) for the Redis tier only.
- A full `cargo test --workspace --all-features` was not run (disk headroom about 7.4 GB); per-crate runs are listed above.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model. Mitigations implemented: T-43-16 (4096-key bound, idle-first eviction, one warning; `capacity_*` tests), T-43-17 (additive per-waiter spread and re-read after waking; the 20-caller test), T-43-18 (the in-flight rule under one lock; the 16-recorder clause asserts streak 1), T-43-19 (the capacity warning names only the capacity; asserted).

## Next Phase Readiness

43-07 (`RedisCadence`) and 43-08 (`ResilientCadence`) call `contract_tests::run_all` (and `run_all_paused` only on a paused clock) with their own policy and jitter pinned. 43-06 can reuse `generate_stream` pacing for fallback hops.

## Self-Check: PASSED

`crates/paladin-storage/src/cadence/contract_tests.rs` exists; commits 1c73259d, 93b122d9, d012e7bc and 22787e39 are present in `git log` on `claude/laughing-dirac-e0h2ax`.
