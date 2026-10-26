---
phase: 46-docs-currency-hygiene
plan: 11
subsystem: docs-migration-guide
tags: [migration, curr-23, d-01, d-02, d-03, d-06, web3sec, freeze]

requires:
  - phase: 46-10
    provides: two-part MIGRATION.md (section 10 above the kept v0.10 record in section 9) and the widened register window
  - phase: 46-08
    provides: corrected M-B-06 / BattalionConfig / ConclaveConfig / DR-8 rows
  - phase: 46-09
    provides: M-B-11 D-24 sentence
provides:
  - "Section 10 cross-references resolve inside part 10 (v0.11 rows cite 10.x; v0.10 rows keep 9.x)"
  - "web3sec's seven guide corrections G-1..G-7 folded into section 10"
  - "Section 10 opening paragraph and freeze note (frozen 2026-10-24 against e28e43b5)"
  - "Section 10.7 empty-deprecations statement; section 10.8 six-step operator checklist"
affects: [46-12, 46-13, 46-14, 47]

tech-stack:
  added: []
  patterns:
    - "Cross-reference rewrite by script with an explicit keep-list, re-scanned for leftovers, instead of a blanket 9.x -> 10.x replace"

key-files:
  created: []
  modified:
    - MIGRATION.md

key-decisions:
  - "DR-11 (StopReason) cites both registers: 10.2 for AllowanceHalted, 9.2 for the v0.10 CallLimit / TokenBudget row"
  - "DR-21 and the LlmPort register row now say a decorator must (not should) forward check_reasoning, so the guide matches the trait docs and the G-6 bullet"
  - "D-06 stays deferred: no warn! for an uncomposed 429 was added and the guide claims no such signal"

requirements-completed: [CURR-23]

duration: 20 min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 11: Freeze the v0.10.1 to v0.11.0 guide Summary

Section 10 of `MIGRATION.md` is frozen: its cross-references resolve inside part 10, web3sec's G-1..G-7 corrections are folded in as the handoff wrote them, and it opens with a "what v0.11.0 is" paragraph, a freeze note (2026-10-24, `e28e43b5`), an empty 10.7 and a six-step operator checklist in 10.8.

## Accomplishments

- **Tracer (commit `2d9d3a17`):** 84 citations inside part 10 retargeted from 9.x to 10.x (prose `section 9.x` / `§9.x`, the DR table's Register column, the "Phase 50 behaviour changes" bullets, DR-21's "see 9.8", M-B-xx cites). 17 matches were kept at 9.x (list below). DR-3 now names `paladin::core::base::entity::Entity<T>` (G-1). The tracer verify was re-run on the committed tree before expansion ("Tracer verified end-to-end -- expanding"); part 9 stayed byte-identical to the `v0.10.1` text plus pointers.
- **G-2..G-7 (commit `1d3d6f93`):**
  - G-2: M-B-08 now tells a consumer to read `message` from the `NodeErrorSource::Paladin`, `Llm` and `Function` arms to keep the v0.10 text, with a list-indented `rust` worked example under the 10.1 table (mirrors the muster's `node_error_read_pin`, `paladin::` imports, `_` arm for `#[non_exhaustive]`).
  - G-3: M-B-06 and DR-9 state both Phalanx timeout outcomes (`Err(AggregationError)` with `on_error: None`; `Ok` plus a `node_errors` entry with `source: Timeout(Run)` under `Absorb`) and that a consumer handling only `Err` stops recording timeouts. Cited range `crates/paladin-battalion/src/phalanx_service.rs` lines 239-280, re-checked at HEAD (failure handling at 239, `Absorb` arm at 257, `node_errors` collection 275-280).
  - G-4: "(two of the models web3sec runs)".
  - G-5: Cadence recipe with `paladin::config::treasurer::CadenceConfig`, `paladin::infrastructure::cadence::{build_cadence, with_cadence, compose_llm}`, `prices: Arc<paladin::core::platform::container::cost::PriceTable>` (`compose_llm` takes `&Arc<PriceTable>`), and `with_cadence` named as the pacing-only form.
  - G-6: the decorator bullet reads "must forward `model_profile` and `check_reasoning`" and says what an unforwarded `check_reasoning` loses.
  - G-7: the "Starting before `v0.10.0`" paragraph carries one sentence: from `35fd839` DeepSeek made up to 4 calls and Anthropic up to 3 on `RateLimitExceeded` (for `max_retries = 3`), now 0 retries unless Cadence is composed. Counts re-verified against `git show 35fd839:` adapter sources and `deepseek/adapter.rs:989-999`.
- **Freeze (commit `fc48c010`):** section 10 opens with the v0.11.0 paragraph and "Frozen 2026-10-24 on the downstream consumer's dogfood report against `e28e43b5` (Phase 46, D-03). Phase 47 re-reads this part only if a code change after `e28e43b5` moves the consumer surface." 10.7 states nothing is `#[deprecated]` and names ADR-0060, ADR-0061, ADR-0062 (D-00e). 10.8 `### Operators: v0.10.1 → v0.11.0` is six numbered bold-lead steps: back up; migrations `007` to `014` run at adapter construction via `sqlx::migrate!`; config (TENANT-01 note kept, M-B-06 removed-key refusal, M-B-15 `llm.<provider>`, M-B-05 Cadence default and `treasurer.cadence.enabled: false`, `treasurer` optional); allowance-webhook rollout (`docs/src/deployment-topologies/http-service-host.md`); deploy (the three shipped `k8s` manifests); verify (`paladin-cli setup-check --verbose`, an agent-route request, a cross-tenant run read answering 404). The Library consumers lead now reads "a consumer-shaped index over register rows, not a new register".

### 9.x citations left in part 10 (each names a v0.10 row or the v0.10 register)

| Location in part 10 | Citation | v0.10 row / target it names |
|---------------------|----------|-----------------------------|
| scope line of 10.2 intro | `§9.2` | the v0.10 register as a whole ("§9.2 is the v0.10 one") |
| 10.2 `PaladinResult` row (Mitigation, 2 links) | `§9.2` + `#92-` anchor, twice | the v0.10 `PaladinResult` row (`served_by`, Phase 31 `usage` rename; present in `v0.10.1`) |
| 10.2 `Settings` row (2 links) | `§9.2` + anchor, twice | the v0.10 `Settings` row and its allowlist entry (present in `v0.10.1`) |
| 10.2 `ExecuteResponse` row (1 link) | `§9.2` + anchor | the v0.10 `ExecuteResponse` row (present in `v0.10.1`) |
| 10.8 DR-11 Register column | `9.2 StopReason row (CallLimit, TokenBudget)` | the v0.10 `StopReason` row (present in `v0.10.1`); `AllowanceHalted` is cited at 10.2 |
| 10.8 Library consumers lead | `section 9.2` | the pre-v0.10.0 rows named in the "Starting before v0.10.0" paragraph |
| 10.8 "No total-only `TokenUsage` constructor" | `section 9.2` | the `TokenUsage` row (Phase 31; present in `v0.10.1`) |
| 10.8 "Starting before `v0.10.0`" (4 cites) | `section 9.2` | `TokenUsage`, `PaladinResult` (Phase 31) and `StreamingResponse` rows; `Commissary`, `TokenCounter`, `TokenCounterFactory` rows (Phase 32); `RagRetrievalService` row (Phase 33); all present in `v0.10.1` |

No kept 9.x citation names a row added since the tag. No `M-B-01..M-B-04` citation exists in part 10.

## Task Commits

1. Task 1 (tracer): `2d9d3a17` docs(46-11): section 10 cross-references point inside section 10; DR-3 facade path
2. Task 2: `1d3d6f93` docs(46-11): fold web3sec's guide corrections G-2..G-7 into section 10
3. Task 3: `fc48c010` docs(46-11): freeze section 10 -- opening, deprecations, operator checklist

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Consistency] Two other cells restated the one-sided Phalanx timeout claim or the weaker decorator wording**
- **Found during:** Task 2
- **Issue:** the 10.2 `BattalionError` register row still said a Phalanx timeout is "inside `AggregationError`" (prohibition 1 forbids describing it as `Err`-only), and DR-21 and the `LlmPort` register row said a decorator "should" forward `check_reasoning`, contradicting the G-6 bullet and the trait docs ("must forward").
- **Fix:** qualified the `BattalionError` Change cell (`Err(AggregationError)` with `on_error: None`; under `Absorb` a `node_errors` entry, see M-B-06); changed "should" to "must" in DR-21 and the `LlmPort` row's Mitigation cell. No key column, Deliberate-breaking cell or allowlist `migration_row` touched; `make check-migration-allowlist` still exits 0.
- **Files modified:** MIGRATION.md
- **Commit:** `1d3d6f93`

**2. [Wording] G-2 prefix list and G-7 phrasing**
- M-B-08 names the `function error: ` prefix as well as the two the handoff listed, because `NodeErrorSource`'s `Display` adds all three. The G-7 sentence says "the first 429 is returned to the caller" and does not state that nothing is logged, so the guide claims no signal it cannot prove (D-06).

**3. [Wording] Library consumers lead**
- Its "Every item below ... section 10.2" line gained "except the pre-`v0.10.0` rows ... in section 9.2", because the "Starting before `v0.10.0`" paragraph under it cites v0.10 rows.

**Total deviations:** 1 auto-fixed (Rule 2), 2 wording judgement calls. **Impact:** none on scope; all inside `MIGRATION.md` section 10.

## Deferred Items

- **D-06 (deferred to the next milestone):** the optional one-time `warn!` the first time an adapter returns `RateLimitExceeded` with no Cadence layer was NOT added. No source file changed in this plan.

## Issues Encountered

- `make test-shell-guards` is red while `MIGRATION.md` is uncommitted (its closing no-mutation assertion); it was run after each commit's tree was clean and passed (54 assertions).
- The worked example in 10.1 is not compiled (`MIGRATION.md` is not); it mirrors the muster's compiled `node_error_read_pin`.

## Verification

- All three task verify commands pass (Task 3's `make test-shell-guards` and the treasurer vocabulary guard run on the committed tree).
- `make check-migration-allowlist`, `make check-gates`, `make check-doc-config` exit 0; `cargo test --offline --test legacy_removal_guard --test entity_rename_guard --test treasurer_vocabulary_guard` passes (8 + 3 + 3). `grep -c 'TBD' MIGRATION.md` prints 0. Every section 10.1-10.8 has content. Part 9 diff against the `v0.10.1` tag is empty.
- `git diff --stat` across the three task commits lists only `MIGRATION.md`.
- mdbook is not installed here; no mdbook leg applies (`MIGRATION.md` is not an mdbook page).

## Known Stubs

None.

## Threat Flags

None. T-46-22 (timeout outcomes) mitigated by G-3 in M-B-06, DR-9 and the `BattalionError` row, with the line range re-checked; T-46-23 mitigated by the `35fd839`-cited counts and the G-4 wording; T-46-20 mitigated (placeholder token count 0 after each commit).

## Last Phase 46 commit to `MIGRATION.md`

`fc48c010` is the last Phase 46 commit that writes `MIGRATION.md` (plans 46-12 to 46-14 retarget citations elsewhere). The 50.1 WR-01..03 bullet (`aa48314d`, the base-URL scheme tightening) is inside 10.8 ("A base URL must start with `http://` or `https://`"), per the CONTEXT discretion check.

## Next Phase Readiness

Ready for 46-12 (retarget live `§9.x` citations in the CHANGELOG `[Unreleased]` sections and TOML comment blocks to the section 10 headings now frozen).

## Self-Check: PASSED

- Commits found: `2d9d3a17`, `1d3d6f93`, `fc48c010`.
- `MIGRATION.md` exists; "### Operators: v0.10.1 → v0.11.0" present; `Frozen 2026-10-24` and `e28e43b5` present; `TBD` count 0.
