---
gsd_state_version: 1.0
milestone: v0.10.0
milestone_name: Durable Agent Execution Runtime
status: planning
last_updated: "2026-09-01T19:50:37.762Z"
last_activity: 2026-09-01
progress:
  total_phases: 8
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-09-01 after the v0.9.0 milestone close)

**Core value:** A Rust developer can compose and run multi-agent workflows against any supported
LLM provider through stable port abstractions — without their own domain code depending on a
provider, transport, or storage implementation.
**Current focus:** Planning next milestone (`/gsd-new-milestone` — new phases start at Phase 22;
`.planning/REQUIREMENTS.md` is removed and opened fresh there).

**Progress:** [██████████] 100% — v0.9.0 shipped

**Previous milestone:** v0.9.0 "Security Tooling" shipped 2026-09-01 — 4 phases (18-21), 25
plans, 20/20 requirements, 240 commits (`48ac11a5..3957d701`). Archived to
`milestones/v0.9.0-ROADMAP.md`, `v0.9.0-REQUIREMENTS.md`, `v0.9.0-MILESTONE-AUDIT.md` (status
`tech_debt`, 0 blockers) and `v0.9.0-phases/`. **Released for real post-close (2026-09-01):**
tag `v0.9.0` on merge commit `0b5d4106`, release run `33542459191` fully green, all eleven crates
on crates.io at `0.9.0` — first stable release since 0.5.1; release numbers now track milestone
names. See MILESTONES.md.

**Prior:** v0.8.0 shipped 2026-08-24 — 14 phases, 149 plans, 65/65 requirements,
1,014 commits (`be2ff05..48ac11a5`). Archived to `milestones/v0.8.0-ROADMAP.md`,
`v0.8.0-REQUIREMENTS.md`, `v0.8.0-MILESTONE-AUDIT.md` and `v0.8.0-phases/`.

> ✅ **The milestone-boundary discrepancy flagged at Phase 16 close is resolved.** The ROADMAP
> `## Milestones` table's four stale "Not started" cells — Milestone 4-6 (7-8), Milestone 7-8
> (9-11), Milestone 9-12 + Deferred-QA (12-16) and Provider Expansion (17) — were refreshed to
> "Shipped v0.8.0" as part of this close, and all five blocks were scoped into one milestone. The
> version identity is settled too: the milestone is labelled **v0.8.0**, matching every Cargo
> manifest and the dated CHANGELOG section, rather than the `v0.7.2` the roadmap had carried.

## Current Position

Phase: 22 (not started)
Plan: —
Status: Roadmap created
Last activity: 2026-09-01 — Roadmap created for v0.10.0 (Phases 22-29, 45/45 requirements mapped)

## Performance Metrics

**Velocity:**

- Total plans completed: 187
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 01 | 11 | - | - |
| 02 | 11 | - | - |
| 3 | 8 | - | - |
| 05 | 13 | - | - |
| 06 | 10 | - | - |
| 07 | 13 | - | - |
| 08 | 9 | - | - |
| 10 | 11 | - | - |
| 11 | 5 | - | - |
| 12 | 4 | - | - |
| 13 | 13 | - | - |
| 14 | 8 | - | - |
| 15 | 10 | - | - |
| 17 | 22 | - | - |
| 16 | 14 | - | - |
| 18 | 7 | - | - |
| 19 | 5 | - | - |
| 20 | 7 | - | - |
| 21 | 6 | - | - |

*Updated after each plan completion*

**Recent Trend:**

- Last 5 plans: —
- Trend: —

**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 01 P09 | ~27min + human review cycle | 3 tasks | 2 files |
| Phase 01 P10 | 20min | 2 tasks | 1 files |
| Phase 01 P12 | 40min | 4 tasks | 4 files |
| Phase 03 P01 | 16min | 2 tasks | 1 files |
| Phase 3 P2 | 25min | 2 tasks | 5 files |
| Phase 03 P03 | ~15min | 3 tasks | 2 files |
| Phase 03 P05 | ~25min | 2 tasks | 1 files |
| Phase 03 P06 | ~20min | 2 tasks | 3 files |
| Phase 03 P04 | 35min | 3 tasks | 3 files |
| Phase 03 P07 | 14min | 2 tasks | 2 files |
| Phase 03 P08 | ~30min | 3 tasks | 4 files |
| Phase 06 P07 | ~50min | 3 tasks | 4 files |
| Phase 12 P02 | ~20min | 2 tasks | 3 files |
| Phase 12 P03 | ~20min | 2 tasks | 3 files |
| Phase 12 P04 | ~25min | 2 tasks | 3 files |
| Phase 20 P07 | 35min | 3 tasks | 3 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table — **empty by evidence, and now finally so.**
**All 263 corpus documents are ingested and 0 ADR-typed and 0 SPEC-typed documents exist among
them.** Nothing is locked, and nothing ever was: no LOCKED-vs-LOCKED contradiction was possible in
any run, which is why 69 competing variants produced 0 blockers.

**This is itself a corpus-level finding worth stating.** Twelve milestones, eighteen months and 554
requirements produced **not one protected decision**. Every technical position in this project's
history sits at PRD or DOC precedence and is auto-overridable by the next document that mentions
it — and mechanical precedence has already produced at least one architecturally wrong answer, a
PRD outranking an Approved-status decision record whose rule would reintroduce the exact upward
dependency that decision removed. Everything asserted in the ingested PRDs and DOCs is supersedable,
including by shipped code: run 2 produced eight documented supersessions, run 3 eleven more
(including the whole monolith → workspace path migration), run 4 eleven more plus the corpus's
first document-supersedes-document notice, and run 5 twelve more plus **the first case of a later
run correcting an earlier run's direct code verification.**

First entries expected from Phase 1 (six ADRs, one per competing variant pair), Phase 5 (four
recorded answers), Phase 7 (six more), Phases 9-10 (the RustSec exception set, the licence posture,
the leaf-crate dependency rule, the PDF capability and the `cargo doc` bar), Phase 12 (the advisory
governance schema and the ADR-promotion decision), Phase 13 (the two Milestone 12 seams) and
Phase 14 (the token mechanism).

**Eleven ADR candidates now exist, none entered as a locked decision. The two with a live
operational cost are the same subject from two different milestones:**

0a. **`Milestone_10/Epic_2/prd-dependency-security-license-compliance.md` FR-1 + §8** (run 5) — the
    audit-suppression single-source invariant, with "no inline advisory-ignore flags remain in CI"
    as an explicit success metric. **The tree violates it today** → SUPPLY-01. Promoting this
    together with candidate 0 below would turn the run-5 supply-chain finding from an observation
    into a gate → SUPPLY-03.
0b. **Four further run-5 candidates**, in descending consequence:
    `M9/Epic_5/prd-user-admin-system-completion.md` §6.1 (the opaque-bearer-token decision — **the
    only decision in the corpus a later milestone contradicts in prose while silently preserving in
    code** → WEB-01); `M9/Epic_4/prd-agent-orchestrator-bridge.md` §6.1 (**the cleanest ADR-shaped
    section anywhere** — four-criterion table, `(CHOSEN)` column, rejected option preserved);
    `M12/Epic_1/prd-agent-registry-execution-api.md` §7 + OQ-2 (the `AgentProvisioner` placement,
    recorded as a default rather than a decision → ORCH-04a); and
    `Deferred-QA-CICD-Completion/DEFERRED_COVERAGE.md` (a named sign-off and an unreached review
    trigger, weakened by two stale paths and stale baselines → DEFER-01 … DEFER-03).

0. **`Milestone_7/Epic_4/rustsec-remediation-plan.md`** (run 4) — a formal risk acceptance with
   **owner Platform Security** and **review/expiry target 2026-09-30**. **The only item in all 153
   documents carrying an expiry date.** Nothing else in `.planning/` surfaces that date; SEC-01 is
   what carries it forward. The other three run-4 candidates — `cost-benefit-assessment.md`
   (self-approval block, named approver, 2026-05-25),
   `license-compatibility-decision-checklist.md` (approver `DF3NDR`, 2026-05-28) and
   `facade-cleanup-RECONCILIATION-2026-06-04.md` (an explicit supersession notice that resolved six
   open decisions in execution) — are recorded in PROJECT.md Key Decisions.

1. **`Milestone_5/Epic_1/decisions/battalion-result-upward-dependency-decision.md`** (run 3) — the
   only decision/options pair in all 263 documents. `Status: Approved`, `Decision Date: 2026-05-13`,
   `Chosen Option: Option A`, with a Rationale, a Rejected Options section and an implementation
   checklist. Settles where `PaladinResult`, `StopReason`, `TokenUsage`, `RegistryError` and
   `HandoffError` live; shipped code implements it. Manifest-typed **DOC**, so a PRD published two
   days later outranks it — and that PRD's FR-10 ("types must not be split across crates") would
   undo the fix. **Strongest candidate in the corpus, and the one with real consequences if left
   unprotected.** Two caveats: it settles *location* for five types only, and despite its filename
   it **never mentions `BattalionResult`**.

2. **`Epic_17.5/epic17-5.md`** (run 2) — the CLI belongs in `src/application/cli` because "CLI is an
   input adapter in the application layer, not infrastructure". Already applied in code (`src/cli`
   is absent from the tree), also outranked by a PRD that says otherwise.

Promoting either requires re-tagging the source document via `--manifest` and re-running ingest.
Entering them here would fabricate authority the corpus does not contain.

**Decisions applied by direction, not derived** (ingest run 5 — the final run, from the user):

1. **All 69 competing variants stay unmerged.** No winners picked, in any run. PROJECT.md Key
   Decisions stays **empty with its evidence note**, and the corpus-level finding — 0 locked
   decisions across 263 documents, every technical decision in twelve milestones auto-overridable —
   is recorded prominently in PROJECT.md Context along with all eleven accumulated ADR candidates.

2. **New phases start at Phase 12.** Phases 1-11 unchanged and unrenumbered; `### Phase N:` headers
   preserved verbatim. Six fresh ID prefixes used, as suggested by the synthesizer: `SUPPLY-*`,
   `ORCH-*`, `WEB-*`, `PIPE-*`, `DEFER-*`, `DOCS-*`. Seventeen prefixes are now spent.

3. **Completed work is not re-planned.** M9 100%, M10 100%, M11 92.0%, M12 99.0%. Every shipped
   artefact goes into the 120-row *Milestone 9-12 as-shipped ledger*, not into a phase.

4. **The forward scope is the deferred registers plus the verified defects — not stale checkbox
   counts.** The three deferred registers (M8 `deferred-features`/`deferred-items`, Deferred-QA
   `DEFERRED_COVERAGE` + `prd-deferred-qa-completion` + Epic_25 `prd-cicd-pipeline-enhancement`)
   and the eight verified-open findings are what became Phases 12-16.

5. **Auth is a genuine forward-work item with a security dimension.** No `jsonwebtoken` dependency
   exists anywhere; the only `AuthPort` implementation is M9's in-process hashed store, whose own
   PRD warned a multi-process deployment would need a shared store — and M12 Epic 7 shipped
   `k8s/deployment.yaml`. Recorded as WEB-01 (the mechanism) and WEB-02 (the store), not as a
   ledger note.

6. **The `deny.toml` "out of sync" framing is withdrawn.** `deny.toml` **is** in sync with
   `.cargo/audit.toml` on all five vulnerability advisories. SEC-01 was **corrected in place** with
   a callout; the real gap — 13 of 15 suppressions with no owner and no expiry, against a Milestone
   10 Epic 2 origin policy mandating a single documented exception process — is SUPPLY-02. The
   earlier framing is not repeated anywhere.

7. **Coverage tooling is partially built, not absent.** `.codecov.yml` does not exist and `ci.yml`
   has no coverage gate — but `integration-tests.yml:117-123` does run `cargo llvm-cov` and
   `codecov-action@v3`. PIPE-02 is scoped as *superseding the integration-only path*, and nowhere
   states that coverage tooling is entirely missing.

8. **DEBT-01 was extended in place, not duplicated** — six stale `project/current-exports.txt`
   references became nine, four of them written into Milestone 12 requirements in June 2026. It
   also **shed** the four `actions-rs` references it had absorbed in run 3; those move to PIPE-04,
   which owns the full eight-reference action-modernization sweep.

**Decisions applied by direction, not derived** (ingest run 4, from the user):

1. The RustSec exception sprawl is **genuine forward work**, not a ledger note. Recorded as SEC-01
   with the exact per-file counts read from the tree on 2026-07-30: `rustsec-remediation-plan.md`
   documents 2 risk-accepted advisories (owner Platform Security, expiry 2026-09-30);
   `ci.yml:406` passes exactly those 2 as `--ignore` flags; `.cargo/audit.toml` `[advisories]
   ignore` holds **5**; `deny.toml` `[advisories] ignore` holds **15**. *(The user's brief cited 7
   and 17 — those are the counts of RUSTSEC IDs **mentioned** in each file, which include
   `RUSTSEC-2026-0185` quinn-proto and `-0190` anyhow named in comments as **upgraded rather than
   ignored**. The substance is unchanged and confirmed: four divergent surfaces, 13 suppressions
   with no risk-acceptance record, and a two-month expiry.)* A fifth fact was found during
   verification and added: `ci.yml` runs **two** independent, differently-configured `cargo audit`
   jobs (`:77` bare, `:406` with two inline ignores).

2. The `api-surface` defect was **extended, not duplicated** — DEBT-01 now records the sixth stale
   reference, M8 Epic 7 FR-10, which writes the broken path into an ingested requirement. No new
   requirement was created for it.

3. `facade-cleanup-RECONCILIATION-2026-06-04.md` is the **authoritative account of Milestone 8**,
   superseding the Epic 1 audit and the Epic 3 disposition (HARD-02). Milestone 8 Epic 6 is complete
   despite being recorded "not verified", Epic 3 is complete in substance, and `paladin-herald`
   exists in the tree — which is why the earlier "9 crates" figure was wrong.

4. The v0.1.0-rc.1 release is **history** (HARD-03). No rc.1 artefact is treated as current state,
   and REL-01 must not converge on an rc.1 figure.

5. All 53 competing variants stay **unmerged**. No winners picked. PROJECT.md Key Decisions stays
   empty with its evidence note; the six ADR candidates are named in context only.

**Decisions applied by direction, not derived** (ingest run 3, from the user):

1. The workspace decomposition SHIPPED. All crates exist and are documented in the codebase map.
   No forward phases were created for Milestone 5 extraction work.

2. The Milestone 6 relocations SHIPPED (`application_settings.rs` deleted, orchestration services
   under `src/application/services/`, Maneuver DSL under `crates/paladin-battalion/src/maneuver/`,
   `CircuitBreaker` under `src/infrastructure/resilience/`). Not re-planned.

3. The `battalion-result-upward-dependency-decision.md` pair is recorded accurately and **not**
   overclaimed: it creates no locked decision, it settles the location of five value/error types,
   and it does **not** resolve the run-1 `BattalionResult` field-set variant.

4. The verified open defects from the run-3 verification ARE genuine forward work and became
   DEBT-01 … DEBT-05. Stale open-checkbox counts did not.

**Decisions applied by direction, not derived** (ingest run 2, from the user):

1. Milestone 3 epic numbering — the plan/epic-definition numbering is authoritative
   (19 Herald, 20 Vision, 21 Autonomous, 22 Battalion hardening, 23 CLI/Config, 24 Test hardening).
   The `RELEASE_NOTES_MILESTONE_3.md` mapping is a documentation defect and is not used as a
   provenance key anywhere in `ROADMAP.md` or `REQUIREMENTS.md`.

2. Conclave, Council, Grove, Maneuver, Sentinel vision and the Qdrant Sanctum adapter are verified
   shipped. No forward phases or requirements were created for them; they are in the as-shipped
   ledger.

3. The Epic 13 vs Epic 20 vision API surfaces coexist (`vision_llm_port.rs` and `vision_port.rs`
   both exist). Recorded as coexistence, not as a variant awaiting resolution.

4. Open checkbox counts are not a backlog. Only the six blocks listed under "Not yet verified" in
   `intel/code-verification.md` may be recorded as unverified candidates, explicitly labelled.

- [Phase ?]: Confirmed workspace coverage measurement of record: 84.79% line coverage (61,404 lines, 9,340 missed), human-approved 2026-07-31T15:30:27Z; RECON-07 resolved
- [Phase ?]: Recorded two accepted observations as context for ADR-0006 and VERIFY-05: 84.79% is ~24pts above stale Milestone-1 baselines (delta noted, not explained); function coverage 77.34% is ~7pts below line coverage 84.79%
- [Phase ?]: ADR-0006: coverage gate = 84% hard-fail floor (measured 84.79%, workspace default-feature scope, option-a); 80% target retired as superseded (deviation from D-09); Herald >=95%/autonomous >=90% preserved, handed to VERIFY-05
- [Phase ?]: ADR-0006 wired into PROJECT.md/ROADMAP.md/REQUIREMENTS.md/ledger: 84% workspace coverage floor is the single binding number, 80% retired as superseded per D-09 deviation; RECON-07 satisfied gated on adr-parser.cjs
- [Phase ?]: Phase 3 Plan 01: reproduced ADR-0006 coverage pipeline verbatim at HEAD bb35554d — measured 85.56% workspace line coverage, PASS against 84.00% floor by 1.56pp; zero-coverage set unchanged from Phase 1 (5 files); 9 of QUAL-02's 11 named offenders contradicted; ratchet trigger not met (1.56pt < 2pt), ADR-0006 not amended
- [Phase ?]: FaultyPaladinPort's fail_until_attempt counter is global across every Paladin executed through one port instance, not per-Paladin — tests are designed around this shared-counter semantics
- [Phase ?]: fail_paladin(name) is an additive, chainable builder method so a single FaultyPaladinPort instance can fail more than one named Paladin
- [Phase ?]: Phase 3 Plan 03: extended the existing hermetic rmcp+axum FixtureServer instead of a parallel wiremock harness for the five MCP failure-mode tests (supersedes CONTEXT.md D-11's wiremock framing per Research Pattern 3)
- [Phase ?]: Phase 3 Plan 03: malformed-response fixture returns its truncated tools/list body as text/plain, not application/json -- rmcp's client silently swallows a malformed-but-json-labeled 200 body as an accepted no-op (verified against vendored rmcp-2.1.0 source), which would hang the test instead of failing loud
- [Phase ?]: Phase 3 Plan 03: the 'absent arguments' bad-arguments shape is asserted directly against the fixture's extract_echo_message helper, not through MCPClient::invoke_tool -- rmcp's CallToolRequestParams::with_arguments always wraps its map in Some(..), even when empty, so the public client API cannot construct that wire shape
- [Phase ?]: Phase 3 Plan 05: refactored redis.rs's eight private key/serialization helpers off &self to free functions (private-surface-only, byte-identical key formats), then added its first #[cfg(test)] mod tests -- 11 Docker-free unit tests covering config defaults, all six key builders, priority-key collision-freedom, serialize/deserialize round-trip, error mapping, and get_priority_levels order. Live-server paths recorded deferred with reason, owner Phase 15 (PIPE).
- [Phase ?]: Phase 3 Plan 05: QUAL-02 NOT marked complete -- requirements ready-ids reports it blocked because sibling plans 03-06/03-07/03-08 also carry QUAL-02 and haven't produced SUMMARY.md yet; final adjudication happens in 03-07 against the exit coverage measurement.
- [Phase ?]: Phase 3 Plan 06: closed file_storage_port.rs, arsenal_port.rs and paladin-llm/error.rs zero-coverage entries with #[cfg(test)] unit tests (32 new tests); FileStorageUtils defaults exercised via the trait's existing () implementor rather than a new empty-impl struct (that impl block would not compile, the trait has no default bodies); arsenal_port.rs's 2 missed lines confirmed from 03-coverage-measurement.md as ArsenalRegistry::list's default body; LlmProviderError->LlmError conversion given its first caller across all 9 variants plus an exhaustiveness witness. No pub API or Cargo.toml changed in either crate.
- [Phase ?]: Phase 3 Plan 04: sequential-only cargo bench execution (no concurrent measurement across targets) to avoid CPU-timing contamination; killed an accidentally-queued second bench process by exact PID after pkill self-matched its own invoking shell
- [Phase ?]: Phase 3 Plan 04: P50/P95/P99 derived from criterion's internal SavedSample sample.json (times[i]/iters[i] per-iteration transform, nearest-rank round((n-1)*p), jq round() i.e. round-half-away-from-zero as the tie-break, verified against a real tied index) for all 39 sample.json files this run produced; 2026-05-27 run retained in place under a superseded callout with no cross-run delta computed
- [Phase ?]: Phase 3 Plan 04: examples/muster_baseline.rs added as a new recorded harness for memory-per-Paladin (479 bytes, via /proc/self/status VmRSS delta across 1000 constructed Paladins) and startup time, since criterion produces neither metric family
- [Phase ?]: Phase 3 Plan 07: exit coverage measurement at HEAD 1ad8be5 -- measured 85.92% workspace line coverage, PASS against 84.00% floor by 1.92pp (up from entry's 1.56pp); 4 of 5 entry zero-coverage files closed (redis.rs, file_storage_port.rs, paladin-llm/error.rs, arsenal_port.rs), src/bin/paladin-server.rs deferred to Phase 5/VERIFY-05; ratchet trigger not met (1.92pt < 2pt), ADR-0006 not amended
- [Phase ?]: Phase 3 Plan 07: QUAL-03 critical-path exerciser evidence established at D-19 bar for all three paths (Paladin execution, Battalion orchestration, tool invocation) via 5 verified passing integration tests; percentage clause recorded superseded by ADR-0006 with zero coverage percentages introduced; amendment lands at source in plan 03-08
- [Phase ?]: Phase 3 Plan 08: QUAL-02 and QUAL-03 amended at source in REQUIREMENTS.md with dated provenance (original text retained); ROADMAP criterion 2 and the milestone ledger amended to match; human confirmed both coverage figures (85.56% entry / 85.92% exit) and the ratchet non-trigger decision (1.92pt delta, 0.08pt short of the 2pt trigger, approver declined the offered floor-raise) via blocking checkpoint, recorded in 03-coverage-measurement.md
- [Phase ?]: 06-07: Verdict distribution 118-row total recomputed by counting rather than assumed unchanged after ledger amendments — count only REQ-*-keyed rows, cluster-table verdicts are a separate bookkeeping layer
- [Phase ?]: 06-07: REQ-vision-security-encryption legend verdict stays present, unproven (documentation-only resolution, no new exerciser); recorded disposition is deliberately unimposed, consumer-facing utility (D-16/D-17)
- [Phase ?]: 06-07: CI-job deferral (Epic 24 cluster 8.0) recorded bidirectionally per D-10 — ledger points at PIPE-01/PIPE-02, PIPE-01/PIPE-02 point back at the ledger
- [Phase 11]: FACADE-01 — D5's 17 `println!` occurrences are **not** converted to `log::*`. All 17 across 6 files are `///`/`//!` doc-comment lines inside fenced code blocks (the same grep filtered to non-doc lines returns 0), so conversion would degrade the rustdoc examples the lines exist to illustrate. Closed by a recorded per-occurrence disposition; ROADMAP criterion 1 amended at source with the original text retained
- [Phase 11]: FACADE-02 / ADR-0034 — D1–D4 get verbs, owners and triggers instead of effort ratings. The `user_service.rs` split is **withdrawn** into three non-overlapping owners: the split owned by nobody, full relocation by the run-3 v2 tech-debt item, tests by DEFER-02/Phase 15. ADR-0034 discloses that ADR-0031 was authored under Phase 10 `--auto`, is flagged `⚠ HUMAN REVIEW` and has never been human-ratified — any future phase executing a D3/D4 edge must confirm it with a human first
- [Phase 11]: FACADE-03 / ADR-0035 — the `paladin-ml` leaf-crate placement condition is promoted out of DOC precedence into an ADR **without** creating the crate or reintroducing the adapter (`ls crates/` still 11). The `user.rs` recovery pointer is corrected from a mutable branch name to the immutable SHA `3d48768` in runnable `git show` form
- [Phase 11]: FACADE-04 — the 20-row Milestone 9 candidate triage resolves to **14 done / 6 not a candidate / 0 still open**, superseding two mutually inconsistent RESEARCH.md figures. `paladin-arsenal` and `paladin-sanctum` are recorded as artefacts of a mis-written table, not future crates
- [Phase 11]: ADR allocation — two ADRs (0034 for the D1–D4 set, 0035 for `paladin-ml`) rather than five or one-plus-ledger-rows. Accepted cost: a coarser supersession unit, since a future phase revisiting only D3's verdict must supersede an ADR that also carries D1, D2 and D4. Human-confirmed at UAT
- [Phase ?]: Phase 12 checkpoint (12-01 Task 3) RESOLVED 2026-08-09: a human selected option-a — proceed as planned with ADR-0036 and the D-08 guard. This authorizes plans 12-02, 12-03 and 12-04 to execute as written and ratifies D-01, D-08 and D-00l (previously unconfirmed ⚠ HUMAN REVIEW decisions). Obtained via the runtime's interactive question mechanism (`AskUserQuestion` → `option-a — ADR-0036 + guard`) after the orchestrator declined to auto-select under `--auto`; provenance recorded in 12-01-SUMMARY.md Checkpoint Status.
- [Phase ?]: 12-02: sibling script (not a 4th check-advisory-register.sh clause) chosen for the D-08 guard — TOML-vs-YAML parser separation and the new root-directory override contract
- [Phase ?]: ADR-0036 (Accepted, conforms) promotes PROMOTION.md Part B candidate 7 in the source PRD's §8 two-file framing (.cargo/audit.toml AND deny.toml), citing ADR-0024 as the sibling governing suppression contents without superseding it
- [Phase ?]: Code Locations/Considered Options rewritten as single-physical-line bullets after the structural self-check showed adr-parser.cjs's per-line splitEntries inflating key_files to 29 and options_considered to 19 against 13/4 real bullets; fixed to 12/12 and 4/4 before commit
- [Phase ?]: SUPPLY-03's checkbox and traceability row deliberately left Pending; requirements-completed left empty in 12-03-SUMMARY.md — plan 12-04 owns SUPPLY-03's closure and PROMOTION.md's single writer in the final wave
- [Phase ?]: PROMOTION.md's Part A step 5 procedural text was left untouched despite causing a grep-count-2 vs expected-1 mismatch in the plan's own verify script — the actual Next free ADR number state line is correct and singular; editing Part A prose would be a fourth unauthorized PROMOTION.md edit
- [Phase ?]: Phase 13 / ORCH-01 hand-off written carrying Milestone 10's verdict class in both halves: 100% complete and one acceptance criterion false, and as of 2026-08-08 no longer false
- [Phase ?]: Recorded both v0.8.1-rc.3 and v0.8.1-rc.4 rehearsals in full — rc.4 is the only live proof of Phase 20's own gate and recovery scripts, and it found and fixed two real gate bugs (Findings 5, 6)
- [Phase ?]: Assumption A3 (OIDC token exchange survives a same-tag re-run) proven twice, independently re-verified against crates.io trustpub_data.run_id rather than transcribed from workflow self-reports

### Pending Todos

None yet.

### Blockers/Concerns

**No blockers. 0 across all five ingest runs.** Everything below is a concern with an owning
requirement.

**— Run 5 (final): eight verified-open findings and two corrections —**

- **⚠ CORRECTION to a run-4 finding — do not repeat the earlier framing.** Run 4 recorded
  `deny.toml` as out of sync with `.cargo/audit.toml`, mirroring "only the original two" advisories.
  **That is withdrawn.** Both files carry the same **five** vulnerability advisories
  (`RUSTSEC-2023-0071`, `-2025-0111`, `-2026-0187`, `-2026-0194`, `-2026-0195`); `deny.toml`'s ten
  extra entries are *unmaintained* notices, a different class, labelled as such and authorised by
  M10 Epic 4 FR-1 step 5. **The real gap is that 13 of the 15 suppressions carry documented
  reasoning but no named owner and no expiry**, against an M10 Epic 2 origin policy that mandates a
  single documented exception process — and FR-3's own four-field schema requires neither, so the
  configs comply and the policy is the gap. Separately, the three 2026 **vulnerability** ignores are
  authorised by **no** ingested document (FR-3 and §5 name exactly two). SEC-01 corrected in place;
  SUPPLY-02 carries the corrected scope.

- **🔴 A completed milestone's own acceptance criterion is false.** `ci.yml` has **two jobs with the
  identical display name `Security Audit`**: `:60-77` runs a bare `cargo audit` under a comment
  declaring `.cargo/audit.toml` the single source of truth (compliant), and `:389-406` runs
  `cargo audit --ignore RUSTSEC-2023-0071 --ignore RUSTSEC-2025-0111` — 2 of the 5 advisories.
  (**Corrected by Phase 12 (plan 12-01), dated 2026-08-09, citing `ci.yml:465-482` and commit `cb75b2b`:**
  this citation was already stale — the job actually sat at `ci.yml:465-482`, deleted
  by Phase 9's plan 09-06 in commit `cb75b2b`. SUPPLY-01 is closed; see `REQUIREMENTS.md`'s
  "Verified by Phase 12" block.)
  `cargo audit` scans `Cargo.lock` irrespective of features, so **the two jobs are configured to
  reach different verdicts on the same tree.** Mechanism: the Epic 25 PRD's Appendix B tabulates
  the pre-M10 pipeline as 7 jobs, #4 being `security`; M10 Epic 2 **added** the compliant job
  without removing its predecessor, and Epic 4's non-goals then froze the area. Milestone 10 is
  recorded 100% complete with 0 open checkboxes. **Fix: delete 18 lines** → SUPPLY-01.

- **🔴 The agent API is documented as JWT and implemented as opaque tokens.**
  `grep -rn "jsonwebtoken" Cargo.toml crates/*/Cargo.toml` returns **nothing**. The only `AuthPort`
  implementation is `src/infrastructure/adapters/auth/in_memory_token_auth_adapter.rs` — M9 Epic 5's
  opaque, in-process, hashed store, chosen deliberately with JWT as an explicit non-goal. Yet
  `crates/paladin-web/src/agent_auth.rs` documents its verifier as JWT throughout, and M12 Epic 5's
  **Open Question 4** is unanswered *because it is unanswerable for the shipped adapter*: an opaque
  store has no signing secret and no algorithm. **This is the only variant in five runs that shipped
  code cannot settle** — the tree carries the M12 shape and the M9 mechanism at once → WEB-01,
  variant group 29.

- **🔴 And it has a multi-replica correctness edge.** M9 Epic 5 §6.1 recorded the trade-off in its
  own words — "tokens are validated against an in-process store, so a multi-process deployment would
  later need a shared store" — and M12 Epic 7 then shipped `k8s/deployment.yaml` with liveness and
  readiness probes. **Under more than one replica, a token issued by one pod will not verify on
  another.** Neither document references the other, and no requirement in the 263-document corpus
  covers the shared store → WEB-02. Not a scaling optimisation; a correctness question.

- **`ProviderCapabilities` over-reports.** All three LLM adapters declare tool-calling capability
  and hardcode `function_call: None`; `crates/paladin-ports/src/output/llm_port.rs` has no `tools`
  field, and greps for `struct ToolDefinition`, `struct ToolCall` and `tool_calls` return zero
  across `paladin-ports` and `paladin-llm`. **Correctable today, independent of whether Epic 27 is
  ever built** → WEB-03 (the flag), WEB-04 (the scope).

- **Deferred-QA Epics 25-27 are verified unbuilt, item by item.** No `cli-tests` job, no
  `bench-check` job, no `coverage` job, no `.codecov.yml`, no Makefile coverage targets (the
  `Makefile` has no `llvm-cov` reference at all), eight deprecated GitHub Actions, the architecture
  document frozen at exactly 311 lines with zero of seven newer subsystems and zero Mermaid
  diagrams, `docs/assets/` empty, no `docs/DEMOS.md`, and no `tools`/`ToolDefinition`/`ToolCall`
  symbols. **Only one FR-25.2 item is closed** — the dangling `on: schedule` block is gone
  → Phases 15 and 16. *Note the scoping correction*: coverage tooling is **partially built** —
  `integration-tests.yml:117-123` does run `cargo llvm-cov` and `codecov-action@v3`.

- **The corpus's largest documentation gap was hidden by a relocation.**
  `docs/src/appendix/design-and-architecture.md` is **exactly 311 lines** — the same figure its own
  PRD cites as the *pre-rewrite* state — with Commander 0, Council 0, Conclave 0, Grove 0,
  Maneuver 0, Sanctum 0, Sentinel 0 and zero mermaid blocks. All seven are verified shipped. M11
  Epic 2 relocated the file into `appendix/`, and M11 Epic 3's non-goals exempt exactly that chapter
  from rewriting. **Invisible for two milestones** → DOCS-02.

- **The Epic 28/29 mock prerequisite does not exist.** No `tests/common/` directory; mocks live at
  `tests/helpers/{mock_llm_adapter,mock_arsenal_adapter,mock_paladin_port}.rs` — a different
  location and a disjoint set. None of `MockUserRepository`, `MockLogPort`,
  `MockNotificationService`, `MockEventSource` or `MockTriggerExecutor` exists. ~6-10 of the 35-45
  estimated hours are this infrastructure → DEFER-01.

- **⚠ Two registers propose incompatible next actions on `user_service.rs`.** Deferred-QA Epic 28
  plans to **test** it to ≥ 80%; M8 `deferred-items.md` D2 plans to **split** it. Run 4 established
  `deferred-items.md` as the highest-fidelity document in the corpus. Splitting first is cheaper but
  changes Epic 28's estimate and mock set. **Do not schedule independently** → FACADE-02 ↔ DEFER-02.

- **Epic 29's coverage baseline is stale in both path and number.** `DEFERRED_COVERAGE.md` records
  `listener_service.rs` at 602 LOC / ~57.83% dated 2026-02-14; the module ships as
  `src/application/services/orchestration/listener.rs` after the M6 relocation, and M9 Epic 2 added
  match/no-match/fan-out/rate-limit/dispatch tests against it. **Scope real, arithmetic not**
  → DEFER-03.

- **`project/current-exports.txt` is now at nine stale references** — five in tooling
  (`scripts/check-api-surface.sh:6`, `scripts/extract-public-api.sh:6`, `ci.yml:171,181,186`) and
  five in requirement text (M8 Epic 7 FR-10 plus M12 Epic 1 §7, Epic 5 §7, Epic 6 `cross_refs`,
  Epic 7 FR-4.6). The M12 ones were written in June 2026, months after commit `928c6d5` renamed the
  directory. **Unchanged across three ingest runs; the longest-lived unfixed defect in the corpus
  and the cheapest to close** → DEBT-01, extended in place.

- **Milestone 11's 26 open items are the only genuinely open checkbox count in all 542** — six
  user-guide updates, eight deployment/operations updates, and the linkcheck report review. All
  fourteen target files exist, so **file existence settles nothing**; verify by content → DOCS-01.

- **Two Milestone 12 seams were recorded as defaults rather than decisions.** Where
  `AgentProvisioner` lives (Epic 1 OQ-2 defaults to `paladin-web`, while the shipped queue-worker
  and sidecar topology pages describe would-be second consumers), and whether Garrison/Arsenal
  wiring for HTTP-served agents is planned scope or permanent (stated once, in a non-goal, against a
  decision matrix M11 Epic 6 FR-8 makes "the single source of routing") → ORCH-04.

- **A sixth position on the coverage gate.** The Deferred-QA parent PRD mandates a **78% hard
  gate**; Epic 25 specifies a **phased 70 → 74 → 78 ramp**. The parent PRD's own OQ-3 asks exactly
  this and is recorded Open; the child Epic answered it unilaterally. Measured coverage is 76-77%,
  so 78% fails on day one and 70% passes. Joins 80 / 85 / 75-layered / 80-Epic-24 → variant group
  30, PIPE-02.

- **The predicted fifth milestone-numbering collision did not occur.** Four instances exist from
  runs 2-4; run 5's provenance keys resolve directly against directory numbering → ORCH-05 records
  the prediction closed.

- **✅ Closed by run 5: the last shipped subsystem without an ingested requirement.** Milestone 12's
  Axum HTTP API surface — auth, rate limiting, OpenAPI, SSE streaming — now has 34 requirements
  across seven Epics. **Every shipped subsystem in this workspace now has at least one ingested
  requirement behind it.**

- **✅ Closed by run 5: the ingest itself.** All 263 documents covered. No run 6.

**— Carried forward from runs 1-4 —**

- **🗓 The only deadline in this project is 2026-09-30, and it is a security acceptance.**
  `Milestone_7/Epic_4/rustsec-remediation-plan.md` formally risk-accepts two advisories
  (`RUSTSEC-2023-0071` rsa, `RUSTSEC-2025-0111` tokio-tar) with **owner Platform Security
  (Milestone 7)** and a **review/expiry target of 2026-09-30** — roughly two months from this
  ingest. It is the only dated item in all 263 documents, and nothing in `.planning/` other than
  SEC-01 surfaces it.

- **The RustSec exception set is encoded four different ways.** *(**Run-5 correction:** the
  "`deny.toml` violates its own stated invariant" half of this entry is **withdrawn** — the two
  files are in sync on all five vulnerability advisories. See the run-5 correction above. The
  four-surface count and the 13-without-owner-or-expiry finding stand.)* Verified by direct file
  reads on 2026-07-30: the plan documents **2**;
  `.cargo/audit.toml` `[advisories] ignore` holds **5** (the 2 plus `RUSTSEC-2026-0187` lopdf via
  `pdf-extract`, `-0194` and `-0195` quick-xml via `rust-s3`/`aws-creds`); `deny.toml`
  `[advisories] ignore` holds **15** (those 5 plus 10 unmaintained notices) under a header claiming
  "the same advisory IDs are mirrored here … Keep these two files in sync"; and `ci.yml` runs
  **two independent `cargo audit` jobs** — `:77` bare (reading audit.toml's 5) and `:406` with the
  original 2 passed inline. `make audit` is bare; `cargo deny check` gates at `:105`.
  **Thirteen of `deny.toml`'s fifteen have no entry in the formal risk-acceptance register** — they
  carry inline reasoning but no owner and no expiry, against acceptance criteria that require both.
  Both tools gate CI. Tracked as SEC-01. *(Note for continuity: counting RUSTSEC IDs **mentioned**
  rather than **suppressed** gives 7 and 17, because `RUSTSEC-2026-0185` quinn-proto and `-0190`
  anyhow are named in comments as upgraded rather than ignored. The `ignore` arrays are 5 and 15.)*

- **A defect is now written into a requirement, not just into code.** M8 Epic 7 FR-10
  (`REQ-web-api-baseline-changelog`) mandates
  `./scripts/extract-public-api.sh project/current-exports.txt` — the path that has been stale since
  commit `928c6d5` renamed `project/` to `.project/`. All five original references are unchanged.
  DEBT-01 was **extended** to cover the requirement text as well as the two script defaults and the
  three workflow lines; no duplicate requirement was created.

- **The licence has three answers and one of them is signed.** A decision checklist with approver
  `DF3NDR` (2026-05-28) and a 551-package inventory records `MIT OR Apache-2.0`; the M7 Epic 4 PRD
  and overview say MIT; the shipped root `Cargo.toml` says `license = "MIT"`. The dual-licence rule
  was the stated basis for accepting `r-efi`'s `MIT OR Apache-2.0 OR LGPL-2.1-or-later`. `deny.toml`
  already follows the checklist. SEC-02 — do not resolve by inference.

- **Two open architecture questions from run 4, both worth surfacing rather than assuming.**
  (1) The extracted-crate dependency rule is stated absolutely — "No extracted crate may depend on
  another extracted crate" — and violated once by `crates/paladin-content`'s optional `paladin-llm`
  edge, which the same PRD's §4.4 anticipated without amending the rule → HARD-05, and the strongest
  SPEC candidate in run 4. (2) `paladin-content` declares `pdf = []` gating **nothing** and the
  facade's `content-processing` omits `pdf` entirely, yet `.cargo/audit.toml` suppresses an advisory
  on the grounds that `pdf-extract` **is** in the graph → HARD-06, which SEC-01 depends on.

- **Three small verified defects on a published crate family.** `crates/paladin-herald/` has a
  README but **no `CHANGELOG.md`**, against a criterion the Epic 4 completion summary records as Met
  (the crate was created after Epic 4 closed) → SEC-04. `Dockerfile.chef:25-33` enumerates nine
  crate manifests and omits `paladin-herald`, so the cache-tightness FR-01 exists to deliver is not
  achieved → SEC-05. And the crates.io name-collision guardrail the publish-verification document
  asked for does not exist; collisions cost Epic 4 two package renames and a NO-GO cycle → SEC-03.

- **Milestone 8 shipped beyond its own planning documents, and two of its epics are complete despite
  their records.** The 2026-06-04 reconciliation found the Epic 1 audit and Epic 3 disposition had
  mis-described ~4,400 LOC of orphaned uncompiled duplicates as "active bridges that stay", then
  executed the relocations Epic 3 had deferred to Milestone 9 — 15 commits, ~10,250 net LOC removed,
  and a new `paladin-herald` crate created inside an Epic whose non-goals forbade exactly that.
  Epic 6 is filed "Not verified; low priority" and is complete; Epic 3 is filed "PUNTED" and is
  complete in substance. Milestone 8's three open checkboxes are contradicted by code. HARD-02
  records the reconciliation as authoritative.

- **`infrastructure-adapter-disposition.md` was a live trap for ingest run 5.** *(**Run-5
  outcome:** run 5 read the Milestone 9 documents directly rather than through this record, and did
  **not** re-plan any relocation — `code-verification.md` verified the whole M9 orchestrator
  subsystem shipped. The trap did not spring, but FACADE-04 still stands: the list remains
  uncorrected in the source, and `paladin-arsenal` / `paladin-sanctum` still name crates that do not
  exist.)* The Epic 3 PRD §6
  designates it "the authoritative cross-reference for the §4.3 M9 flags" — the document Milestone 9
  was meant to read — and it records all 20 rows as "Stays", names two crates that do not exist
  (`paladin-arsenal`, `paladin-sanctum`), and disagrees with its own governing PRD on two rows.
  Milestone 9 is recorded 100% complete. **FACADE-04 still stands** — run 5 bypassed this record
  and re-planned nothing, but the list remains uncorrected at source.

- **Checkbox state is the least reliable signal in this project — and it is wrong in both
  directions.** Precedence is **shipped tree > `.planning/codebase/` > `intel/code-verification.md`
  > PRD > DOC > checkbox.** Runs 1 and 2 found checkboxes *understating* shipped reality (Chain of
  Command and Herald wiring; Conclave 129 open and shipped; Sanctum/Qdrant 111 open and shipped).
  Run 3 found the first *accurate* count — Milestone 4's 20 open items, corroborated by zero
  `#[deprecated]` annotations in the tree — **and** the first count that *overstates* completion:
  Milestone 4 Epic 3's CLI-isolation list is fully checked while three CLI-only dependencies remain
  unconditional. Verify each count against the tree before implementing anything.

- **Five verified open defects in Milestone 4-6 scope, all small, all confirmed against the tree
  on 2026-07-30.** (1) The `api-surface` CI job fails on every run: `ci.yml:171,181,186` and both
  `scripts/{check-api-surface,extract-public-api}.sh` defaults point at
  `project/current-exports.txt`, but the directory was renamed in commit `928c6d5` and the baseline
  lives at `.project/current-exports.txt` — so the only automated public-API guard is inert, and
  `check-deprecations.sh` never runs. (2) `grep -rn '#\[deprecated' src crates` returns 0 against
  Milestone 4 Epic 2 FR-8. (3) `crates/paladin-ports/Cargo.toml:18` sets `[lib] doctest = false`
  deferring the fix to an unwritten "Task 7.0", and `ci.yml:225` excludes the crate from `--doc`.
  (4) `structopt`, `colored` and `comfy-table` are still unconditional root dependencies.
  (5) Three `TokenUsage` structs ship (`token_usage.rs:13`, `battalion/mod.rs:497`,
  `llm_analysis_service.rs:51`). Tracked as DEBT-01 … DEBT-05.

- **Two structural questions gate Milestone 4-6 planning rather than its content.** The
  milestone/tier numbering collision (the Milestone 4-6 overviews number themselves "Milestone
  1/2/3" by refactoring tier, and PRDs cross-reference "Milestone 1 / Epic 2" meaning Milestone 4
  Epic 2) → ARCH-02; and the Milestone 6 facade re-export policy, where the overview requires
  backward-compatible re-exports and both PRDs forbid them, which decides whether Milestone 6 was a
  breaking change requiring a major version bump → ARCH-04.

- **Five documented positions would break things if applied literally**: `vision` gating
  `chacha20poly1305`/`zeroize` (would break `cargo build --no-default-features`), the MCP transport
  feature flags, `web-server` gating actix-web, a `paladin-cli` crate, and
  `src/application/use_cases/` as the orchestration home. All five are contradicted by shipped
  code → ARCH-05.

- **One verified open defect in Milestone 2-3 scope.** `grove_service.rs:537` builds its routing
  request with `model: "gpt-4".to_string(), // TODO: Make configurable` in production code
  (`#[cfg(test)]` begins at line 732), so Grove routing ignores the configured provider. This is the
  same defect class Epic 21 removed elsewhere, and it means Epic 22's "all inline TODOs resolved"
  criterion is unmet. Tracked as CLOSE-01.

- **Three open-checkbox blocks still unverified** — Epic 22 hardening (81), Epic 14 autonomous
  (45), Epic 24 test hardening (29). These are the only run-2 blocks `code-verification.md` leaves
  unchecked, and they are *claims*, not work. VERIFY-02 resolves them; CLOSE-02 acts on whatever
  they prove.

- **30 competing variant groups / 60 entries / 69 warnings preserved unmerged** across all five
  runs (6 groups from run 1, 10 from run 2, 4 from run 3, 8 from run 4, 2 from run 5; fourteen of
  run 5's sixteen warnings are not `-v1`/`-v2` pairs and are listed separately). No winners picked —
  deliberately, and at the user's explicit direction, in every run. **Run 5 produced the only
  variant shipped code cannot settle**: group 29's token mechanism, where the tree carries the
  Milestone 12 shape and the Milestone 9 mechanism simultaneously. Recording answers is RECON-02 … RECON-07, VERIFY-03 … VERIFY-06,
  ARCH-03, ARCH-04, SEC-01, SEC-02 and HARD-01 … HARD-07. **Run 4 is the run where shipped code
  settles the most**: six of its eight new groups carry a `settled-by` pointer, which is a fact
  about the tree rather than a decision. The one genuine surprise is group 23 — the two publish
  dry-run forms turned out to **coexist**, per-crate in `release.yml:410` and workspace-wide in
  `ci.yml:644`, which the documents alone could not reveal.
  Highest-consequence now: **ownership of `PaladinResult` / `StopReason` / `TokenUsage`** (group
  19 — the one place where mechanical precedence gives the architecturally wrong answer, because a
  PRD outranks an Approved-status decision record and its FR-10 would reintroduce the upward
  dependency the decision removed), the coverage gate (4 positions), the handoff tool name and
  parameters (3 names / 2 parameter sets), the Grove routing threshold (3 names / 3 defaults), and
  the `paladin-core` dependency allowlist (declared exhaustive at 6, ships 14).

- **Three run-1/run-2 variants were CLOSED by run-3 code verification** — recorded as facts about
  the tree, not decisions. `BattalionResult` resolves to a merged superset at `battalion/mod.rs:549`
  satisfying all three consumers (so RECON-03 became a recording task and GAP-07 lost its code
  change); `BattalionConfig` resolves to the Epic 4 form exactly and `CommanderConfig` does not
  exist anywhere, collapsing the three-owner `metadata_output_dir` warning to one owner; and the
  competing `ErrorStrategy` variant sets turned out to be two distinct enums in two crates, which
  Milestone 6 physically separated. No entry was deleted.

- **Two contradictions are live in shipped code**: `formation.rs:109` rejects fewer than 2 Paladins
  while the Commander's Auto rule routes a single Paladin to Formation; and `require_api_key()` in
  the live-API test harness panics by design, reversing the graceful-skip criterion in both the
  Epic 23 and Epic 24 PRDs.

- **A documentation defect is propagating epic numbers.** `RELEASE_NOTES_MILESTONE_3.md` assigns
  Milestone 3 Epics 19-23 to four Milestone **2** features, and four further documents mislabel
  epics in cross-references. Epic numbers are the corpus's provenance keys, so this misroutes any
  lookup. VERIFY-03 fixes it at the source.

- **Two release-notes claims are verified absent from the tree**:
  `RoutingStrategy::PerformanceBased` with "dynamic learning" (also contradicts Epic 16 non-goal
  NG-3), and the Council/Maneuver API forms that disagree with the shipped surfaces. Do not plan
  against them.

- **A security requirement vanished between PRDs without a recorded decision.** Epic 13 FR-11
  required encryption at rest for temporarily stored image data, memory zeroization and retention
  policies; Epic 20 completed the vision pipeline with none of it and dropped `EncryptionError`
  from the error enum. No artefact for it was found in the tree. VERIFY-04 establishes whether the
  drop was conscious.

- **Quality numbers are below their own gates and the gate has four positions**: 80% (nine
  Milestone-1 PRDs), 85% (unit-test-improvements), 75% overall with a layered per-tier table
  (Milestone 3 plan), 80%/70% re-asserted (Epic 24). Measured: 60.88% unit / 67.79% integration at
  Milestone 1, ~78% overall at Milestone 3. Plus module-scoped gates at 95% (Herald) and 90%
  (autonomous). No performance baseline document exists.

- **Reported test totals are not a monotonic series**: 999 → 1,292 → 1,674 → 1,628 → 853 across
  the corpus. No figure is authoritative; none is used as a gate.

- **All `src/...` paths in the run-1 and run-2 corpus are historical — and several run-3 paths
  are too.** Those PRDs assume a single-crate layout; the workspace was decomposed in Milestone 5
  (run 3) into what is now **ten library crates plus a `doc-examples` crate plus the root
  `paladin-ai` facade** — not the "9-crate workspace" this planning set previously recorded, and not
  the six the Milestone 5/6 overviews assume. Milestone 6 then moved several things Milestone 5 had
  just placed (the Maneuver parser out of `paladin-core`, `CircuitBreaker` into infrastructure), and
  the Milestone 6 Epic 2 PRD's own target directory `src/application/use_cases/` no longer exists.
  Resolve locations through `.planning/codebase/` or the tree, never through a PRD.

- **~~Five shipped crates have no ingested requirement~~ — closed by run 4.** All ten library
  crates now have one: `paladin-storage`, `paladin-notifications`, `paladin-content` and
  `paladin-web` from M7 Epic 1's extraction PRD and its cost-benefit gate, and `paladin-herald` from
  the 2026-06-04 reconciliation rather than from any PRD. What still ships without a requirement is
  Milestone 12's Axum HTTP API surface (auth, rate limiting, OpenAPI, SSE streaming) — run 5.

- **Version metadata disagrees three ways**: branch `release/v0.7.0`, `Cargo.toml` 0.6.0 (root
  package and every workspace crate path dependency), tag v0.5.1. REL-01 converges them, but
  ARCH-04's answer on whether Milestone 6 was a breaking change determines what they converge *to*.

- **Edition is mixed and the documents disagree too**: root plus nine crates on `edition = "2024"`,
  `crates/paladin-ports` and `crates/paladin-notifications` on `"2021"`. Milestone 5 Epics 1-4
  require 2021; Epic 5 and the milestone overview require 2024. ARCH-03(a) records the answer,
  REL-02 applies it.

- **~~Four~~ Eight deprecated GitHub Action references remain**, against Milestone 5 Epic 6's
  "low-risk improvement that should not be deferred" and Deferred-QA FR-25.1. Run 5 completed the
  count: `actions-rs/toolchain@v1` at `ci.yml:147`, `:317`, `:507` and `integration-tests.yml:71`;
  `actions/cache@v3` at `integration-tests.yml:78`, `:84`, `:90`; `codecov/codecov-action@v3` at
  `integration-tests.yml:123`. **Moved out of DEBT-01 into PIPE-04**, which owns the full sweep;
  DEBT-01 keeps only the `project/current-exports.txt` baseline path. Recorded so neither is
  planned twice.

- **No `.planning/config.json`** — granularity `standard` and sequential phase IDs assumed in all
  five runs. Phase IDs are plain (`Phase 12` … `Phase 16`), not milestone-prefixed and not
  project-coded. **No phase in this roadmap is a UI/frontend phase** — Paladin is a Rust library
  and HTTP service with mdbook documentation, so no `UI hint` annotation is carried and
  `/gsd-ui-phase` is not applicable.

- **✅ No ingest runs pending. The ingest is complete.** All five runs are done and all 263
  documents are covered. Any future addition follows the Roadmap Extension Protocol: **new phases
  start at Phase 17**, Phases 1-16 are never renumbered, and **seventeen** ID prefixes are spent
  (`RECON`, `GAP`, `QUAL`, `REL`, `VERIFY`, `CLOSE`, `ARCH`, `DEBT`, `SEC`, `HARD`, `FACADE`,
  `SUPPLY`, `ORCH`, `WEB`, `PIPE`, `DEFER`, `DOCS`).
  **`Milestones-8-11_Dependency-Graph.md` is now spent**: run 5 confirmed every dependency it
  described was honoured and every release gate it named was cut — M9 100% at v0.3.0, M10 100% at
  v0.4.0, M11 92% at v0.5.0, M12 99% at v0.6.0, which is exactly where the tree sits. Keep its
  dependency semantics as a pattern; the schedule is history.

- **Hygiene, not planning**: one ingested source document
  (`Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md`) contains a plaintext OpenAI
  API key in its body. The value was never copied into any `.planning/` file. The user has confirmed
  it is rotated. Redacting the source document and running a repository-wide secret scan is still
  recommended — the same value may appear in `.env` history or coverage artefacts.

### Roadmap Evolution

- Phase 15.1 inserted after Phase 15: Git & CI Governance — branch protection, trigger surface, gitflow model, docs/BRANCH_PROTECTION.md (URGENT)
- Phase 17 added: Additional LLM Provider Adapters — provider-selection study (PROV-01) then feature-gated adapters for survivors (PROV-02..04); first forward phase beyond the ingest
- Phase 18 added: Rust SAST: evaluate and adopt CodeQL — new Security Tooling milestone; SAST-01..04 minted at roadmap time
- Phase 19 added: crates.io Trusted Publishing — replace the long-lived CARGO_REGISTRY_TOKEN with OIDC-issued ephemeral tokens; PUB-01..PUB-05 minted at roadmap time
- Phase 20 added: Release Pipeline Recovery — idempotent re-runs on the same tag, a pre-publish gate over tag/manifest/changelog/CI agreement, and a stuck-halfway runbook with a yank policy; PUBOPS-01..PUBOPS-05 minted at roadmap time
- Phase 21 added: Release Artifacts — Curated Release Notes and Attached Distributables (v0.9.0; ARTIFACT-01..06 minted, twenty-second prefix)

## Deferred Items

### Acknowledged at v0.9.0 milestone close (2026-09-01)

**Verification overrides: 0** — all 4 phases (18-21) report `phase_complete: true` and
`verification_status: passed`. **Open artifacts acknowledged: 1** (the same user-owned pending
todo acknowledged at the v0.8.0 close, unchanged). Closeout type `override_closeout`, on the
strength of that todo rather than any unverified phase.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Testing | `2026-08-13-verify-local-coverage-reproduction` | Open — owner: repo maintainer. Unchanged since the v0.8.0 close: verifies the documented local procedure (`make services-up`, then `make coverage`) reproduces CI's 82.39% on a Docker-capable machine. Deliberately carries no `resolves_phase` tag so a close cannot silently absorb it | v0.8.0 close, re-acknowledged v0.9.0 |
| Testing | Nyquist validation unreconciled for Phases 18-21 | All 4 `VALIDATION.md` files read `status: draft` (NOT-VALIDATED per #2117) — coverage TODO, not a compliance failure. Run `/gsd-validate-phase <N>`. Joins the same open item for archived Phases 05-17 | v0.9.0 close |

Every human-verification backstop the phase verifications declared was closed by recorded UAT
before the close: the Phase 19 crates.io token revocation (operator, 2026-08-28, `19-UAT.md`) and
both Phase 21 checks — out-of-band pull-by-digest and `paladin-cli` execution (user, 2026-09-01,
`21-UAT.md`). The remaining v0.9.0 debt items (CodeQL re-probe trigger, `workflow_dispatch`
publish path, `make publish-dry-run`, dead `upload_url` script output) are inventoried in
`milestones/v0.9.0-MILESTONE-AUDIT.md`, not duplicated here.

### Acknowledged at v0.8.0 milestone close (2026-08-24)

**Verification overrides: 0** — all 14 phases (05-17) report `phase_complete: true` and
`verification_status: passed`. **Open artifacts acknowledged: 1.** Closeout type
`override_closeout`, on the strength of the pending todo below rather than any unverified phase.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Testing | `2026-08-13-verify-local-coverage-reproduction` | Open — owner: repo maintainer. Verifies that the documented local procedure (`make services-up`, then `make coverage`) reproduces CI's measured 82.39% on a Docker-capable machine. The CI half is confirmed (run `31727496744` at commit `e9e3267`); the local half has never been walked end-to-end, because no authoring environment in Phase 15 had Docker or `cargo-llvm-cov`, and none is available at close. **Acknowledged rather than closed by design:** the todo deliberately carries no `resolves_phase` tag precisely so that a phase close cannot silently absorb it | v0.8.0 close |
| Testing | Nyquist validation never reconciled for Phases 05-17 | All 13 `VALIDATION.md` files read `status: draft` — seeded by plan-phase, never promoted by validate-phase, so `nyquist_compliant` is not authoritative. Phase 06 has no `VALIDATION.md` at all. Coverage TODO, not a compliance failure (#2117). Run `/gsd-validate-phase <N>` | v0.8.0 close |
| Security | No static taint analysis for first-party Rust | Snyk was measured and removed 2026-08-18 (0 findings on a four-vulnerability Rust probe vs 3 for identical JavaScript). `cargo-audit`/`cargo-deny` scan dependencies; clippy is a lint. **Not deferred indefinitely — owned by Phase 18 (SAST-01…04) in the v0.9.0 Security Tooling milestone** | v0.8.0 close |

The full debt inventory — 25 recorded items across 10 phases, plus 12 open and 4 waived
`WINDOWS.md` rows — is in `.planning/milestones/v0.8.0-MILESTONE-AUDIT.md`, not duplicated here.

### Acknowledged at v0.7.1 milestone close (2026-08-04)

**Verification overrides: 1.** Closeout type `override_closeout`.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Verification | Phase 1 verification timestamp stale | `01-VERIFICATION.md` records `passed` 5/5 at 2026-07-31T16:46:51Z; commit `be2ff05` (2026-08-03) later added `01-04-SUMMARY.md`, so `init.manager` reports `verification_status: stale` / `phase_complete: false`. The commit is documentation-only ("No ADR, measurement, or source changes"); accepted as an override rather than re-verified | v0.7.1 close |
| Integration | Herald not reachable from Campaign, Chain of Command, or the Commander router (audit WARN-01) | Formation and Phalanx wire Herald; the other three carry zero references. `format_battalion_result` is pattern-agnostic so no requirement's text is falsified, but the composite Chain-of-Command developer flow does not compose without the caller invoking a Herald directly. Unassigned — candidate for Phase 6. **Closed 2026-08-05, plan 06-07: adopted under CLOSE-02 and closed by plan 06-02** — see `.planning/ROADMAP.md`'s Phase 6 WARN-01 outcome note and `.planning/ledgers/milestone-02-03.md`'s CLOSE-02 scope section (Epic 22 cross-reference) for the full record | v0.7.1 close |
| Testing | Nyquist validation never reconciled for Phases 1-4 | All four `VALIDATION.md` files read `status: draft` — seeded by plan-phase, never promoted by validate-phase, so `nyquist_compliant` is not authoritative. Coverage TODO, not a compliance failure (#2117). Run `/gsd-validate-phase 1`…`4` | v0.7.1 close |

### Carried from earlier ingest runs

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| Testing | Live-provider-API integration tests (Epic 6 task 7.0, 18 subtasks) | **Un-deferred by run 2** — suite ships behind `live-api-tests`; only the skip-vs-fail semantics remain open (VERIFY-06) | Ingest run 1, revised run 2 |
| Testing | CLI end-to-end tests (Epic 9 tasks 13.4-13.6) | **Un-deferred by run 2** — the blocking mock provider shipped (REQ-mock-llm-adapter) along with the Tier-1 CLI suites | Ingest run 1, revised run 2 |
| Testing | Garrison large-conversation perf test (Epic 2 task 9.14) | Deferred — marked future enhancement | Ingest run 1 |
| Testing | Vision and RAG latency targets never measured (single image < 5 s; retrieval < 500 ms p95; extraction < 3 s p95) | Deferred to v2 — no baseline document exists | Ingest run 2 |
| Tech debt | Oversized service file decomposition (2,757 / 2,294 / 1,840 lines) | Deferred to v2 — no ingested requirement | Ingest run 1 |
| Tech debt | Clone/lock-contention optimization | Deferred to v2 — blocked on Phase 3 benchmarks | Ingest run 1 |
| Tech debt | Single-threaded orchestration scheduler (`orchestration/scheduler.rs`) | Deferred to v2 — `tokio-cron-scheduler` is already a dependency and already adapted in `paladin-storage` | Ingest run 2 |
| Scope | MCP WebSocket transport | Deferred — recorded as a known limitation by the Epic 23 completion summary | Ingest run 2 |
| Scope | Garrison semantic search / vector context retrieval in the CLI path (recency-based selection only) | Deferred — Epic 23 known limitation; superseded in spirit by Sanctum | Ingest run 2 |
| Scope | Grove learning from past routing decisions | Out of scope — Epic 16 NG-3; the release-notes `PerformanceBased` claim is verified absent from the tree | Ingest run 2 |
| Scope | Automatic Garrison-to-Sanctum migration | Out of scope — Epic 11 explicit non-goal | Ingest run 2 |
| Scope | Batch vision API | Out of scope — Epic 20 NG-6; concurrency is a Battalion concern | Ingest run 2 |
| Scope | Registry multi-tenancy, persistence, distribution | Out of scope — Epic 22 explicit non-goals | Ingest run 2 |
| Scope | ~~Milestones 9-12 feature work~~ | **Closed by run 5** — all four milestones ingested and verified shipped (M9 100%, M10 100%, M11 92.0%, M12 99.0%). Recorded in the 120-row *Milestone 9-12 as-shipped ledger*, not deferred | Ingest run 1, narrowed runs 2-4, closed run 5 |
| Tech debt | **D1 — `src/core/` re-export shims** (6 files, 49 facade importers) | **KEEP, by decision** — removal means rewriting 49 files and preserving `platform/mod.rs`'s maneuver/parser injection, which carries real logic. Becomes debt only if a no-alias policy is adopted (ARCH-04) → FACADE-02 | Ingest run 4 |
| Tech debt | **D2 — mis-layered `src/core/platform/manager/` services** (`content_service`, `event_manager`, `user_service`) | Deferred, medium/medium — partly overtaken: reconciliation commit `6704807` found "no user-service split was needed" because `UserServiceTrait` and the DTOs already live in `paladin-core`. Overlaps the run-3 v2 `user_service` relocation item; do not plan twice → FACADE-02 | Ingest run 4 |
| Tech debt | **D3 — entangled Paladin services** (`planning`/`prompt_generation`/`temperature`/`handoff`, ~2,750 LOC) | **KEEP for now**, high/high — needs the `paladin_builder.rs` / `paladin_execution_service.rs` coupling untangled first, and the targets (`paladin-battalion`, `paladin-llm`) are leaf-to-leaf edges gated on HARD-05 → FACADE-02 | Ingest run 4 |
| Tech debt | **D4 — `content_ingestion_service.rs` placement** (~1,211 LOC) | Deferred, medium/medium — M7 Epic 1's PRD listed it as moving to `paladin-content`; the facade kept its own copy. Needs a dependency-coupling review → FACADE-02 | Ingest run 4 |
| Tech debt | **D5 — residual `println!`/`eprintln!`/`dbg!`** | **Verified exact: 17 occurrences across 6 files**, down from ~435 across 36. The register's own quick win; low/low → FACADE-01 | Ingest run 4 |
| Scope | The `paladin user …` CLI command surface (1,065 LOC, 8 subcommands) | Deferred on purpose — it was declared but **never dispatched**, so it compiled and did nothing. Backend intact; reintroduction is "mostly re-wiring", recoverable verbatim from the M8 removal commit on `chore/facade-cleanup-m8-finish` → FACADE-03(a) | Ingest run 4 |
| Scope | The TensorFlow ML adapter and the `ml` feature flag (636 LOC) | Deferred on purpose — a `#[doc(hidden)]` stub nothing consumed. **Reintroduction condition is the load-bearing part**: a dedicated `paladin-ml` leaf crate, never the facade, with the flag on that crate; `MlPort` stays in the workspace → FACADE-03(b) | Ingest run 4 |
| Scope | A future **content-delivery crate** | Reserved by M7 Epic 1 §4.5.2 as the "correct long-term home" for `file_content_repository.rs`; the file was then deleted and no later document mentions the crate. Carried so the idea is not lost silently | Ingest run 4 |
| Scope | `paladin-arsenal` and `paladin-sanctum` crates | Out of scope — named only by a superseded disposition record that contradicts its own governing PRD. Neither exists; Milestone 9 is 100% complete. Triaging the list is FACADE-04 | Ingest run 4 |
| Tech debt | `paladin-core` / `paladin-ports` dependency allowlists brought back in line with reality (declared 6 and 7; ship 14 and 10) | Deferred to v2 — the architectural invariant holds; this is document-versus-code drift. Needs ARCH-03(b) to choose a direction | Ingest run 3 |
| Tech debt | `retry`, `rate_limiter` and `bulkhead` primitives in `src/infrastructure/resilience/`, plus consolidating the retry logic in `mcp_sse_adapter.rs` and `api_content_deliverer.rs` | Deferred — explicitly scoped out by Milestone 6 Epic 4, which shipped the module scaffold only | Ingest run 3 |
| Tech debt | Full `user_service` relocation out of `src/core/platform/manager/` (with `UserServiceFactory`, `user_config.rs`, user CLI commands, user API controller, `SqliteUserRepository`) | Deferred — Milestone 6 Epic 2 scoped it out and flagged it for "a future Epic" | Ingest run 3 |
| Scope | A `paladin-cli` workspace crate | Out of scope — the Milestone 5 overview's target structure named it, the Epic 6 PRD's non-goal rejected it, and the code agrees with the PRD (a `cli` feature plus `[[bin]] paladin-cli`) | Ingest run 3 |
| Scope | MCP feature flags (`mcp-arsenal` / `mcp-transports` / `mcp-stdio` / `mcp-sse`) | Out of scope — eliminated by a dated 2026-04-15 PRD note; Arsenal and its transports compile unconditionally | Ingest run 3 |
| Scope | A `paladin-infra` crate, and a `CircuitBreakerPort` trait abstraction | Out of scope — both explicitly rejected by Milestone 6 Epic 4, which accepted the resulting layering inversion as a pragmatic trade-off inside the facade crate | Ingest run 3 |
| Tech debt | **Deferred-QA Epic 25 — CI/CD pipeline enhancement** (`cli-tests`, `bench-check` and `coverage` jobs, `.codecov.yml`, four Makefile targets, eight deprecated actions, CONTRIBUTING coverage docs) | **Un-deferred by run 5 — verified unbuilt item by item and promoted to Phase 15** (PIPE-01 … PIPE-05). The register's own recommended first epic: "establishes quality gates that validate all subsequent work" | Ingest run 5 |
| Tech debt | **Deferred-QA Epic 26 — documentation and rustdoc** (architecture doc modernization, zero rustdoc warnings in CI, 100% public-API rustdoc, four asciinema demos) | **Un-deferred by run 5 — promoted to Phase 16** (DOCS-02 … DOCS-04). The architecture document is verified frozen at 311 lines with zero of seven newer subsystems | Ingest run 5 |
| Scope | **Deferred-QA Epic 27 — LLM tool calling** (`tools` on `LlmRequest`, `ToolDefinition`, `ToolCall`, `tool_calls` on `LlmResponse`, all three adapters) | **Decision required, not deferred again** — verified entirely absent; it is a **breaking change to the `LlmPort` trait** by the PRD's own admission, both its open questions are unanswered, and Arsenal/MCP already provides tool execution through a different seam → WEB-04. The separable defect (`ProviderCapabilities` over-reporting) is correctable today → WEB-03 | Ingest run 5 |
| Testing | **Deferred-QA Epics 28-29 — platform-services and event-system coverage** (`user_service.rs` ~4.23% → ≥ 80%; the listener orchestrator ~57.83% → ≥ 80%, with concurrency, deadlock, 1000-event-burst and distributed-tracing scope) | **Partially un-deferred by run 5 — promoted to Phase 15** (DEFER-01 … DEFER-03). **Scope real, numbers not**: both module paths are stale and both baselines predate Milestone 9's tests. Blocked on the shared mock infrastructure that does not exist, and `user_service.rs` must be sequenced against M8 deferred item D2 | Ingest run 5 |
| Tech debt | The shared `Send + Sync` mock and async-test infrastructure (`MockUserRepository`, `MockLogPort`, `MockNotificationService`, `MockEventSource`, `MockTriggerExecutor`, Tokio time control) | **Un-deferred by run 5 → DEFER-01.** Named as an unchecked prerequisite by `DEFERRED_COVERAGE.md` and by both coverage Epics; ~6-10 of the 35-45 estimated hours. Placement (`tests/common/` versus the existing `tests/helpers/`) and `mockall`-versus-hand-written are both unanswered | Ingest run 5 |
| Scope | A shared-store `AuthPort` implementation for multi-process serving | **Never deferred — never requirement-ed at all.** M9 Epic 5 §6.1 anticipated it in prose ("a multi-process deployment would later need a shared store") and M12 Epic 7 then shipped `k8s/deployment.yaml`. **No requirement in the 263-document corpus covers it** → WEB-02 | Ingest run 5 |
| Scope | Garrison (memory) and Arsenal (tools/MCP) wiring for HTTP-served agents | Deferred by M12 Epic 2 and restated by Epic 3 — "agents are LLM + prompt only here". **Whether this is planned scope or a permanent property of the topology is undecided**, and the deployment-topologies decision matrix that routes readers between topologies must say which → ORCH-04(b) | Ingest run 5 |
| Scope | Hot-reloading `config.yml`; TLS termination in `paladin-server`; fine-grained scopes beyond `allowed_roles` + admin gate; encrypting config at rest | Out of scope — all four are explicit Milestone 12 non-goals. TLS is a proxy/ingress concern; secrets management is "the operator's responsibility, as with LLM keys" | Ingest run 5 |
| Scope | Benchmark regression **detection** (`critcmp`, `github-action-benchmark`) | Out of scope — Deferred-QA Epic 25 non-goal. Note the inversion: `benchmark-regression-signal` already ships from M7 Epic 3 while the `bench-check` compile prerequisite does not → PIPE-01 | Ingest run 5 |
| Scope | Rewriting the 35 mdbook appendix files | Out of scope — M11 Epic 3 non-goal ("reference/archive material"). **One exception is under decision**: `design-and-architecture.md`, whose relocation into that exempt chapter is precisely why its gap survived → DOCS-02 | Ingest run 5 |

## Session Continuity

**Stopped at:** Phase 21 complete (UAT 2/2 passed, security threat-secure, verification passed) — v0.9.0 all 4 phases done; ready for /gsd-complete-milestone
Phase 11 closed with UAT 3/3 passed, canonical verification `passed`, and security
`threats_open: 0` (34 threats: 24 mitigate verified closed, 10 accept documented).
Phases 1-4 complete and archived to `.planning/milestones/v0.7.1-phases/`.
See the milestone-boundary note under Project Reference before planning Phase 12.

Last session: 2026-09-01T13:06:14.469Z
Resume file:

Prior session: 2026-07-31T19:27:35.303Z

**Stopped at: ingest run 5 of 5 merged into PROJECT.md, REQUIREMENTS.md, ROADMAP.md and STATE.md.
THE INGEST IS COMPLETE.**

What run 5 produced:

- **120 run-5 requirements** recorded in a new *Milestone 9-12 as-shipped ledger* — the largest of
  the five, with 37 verified-shipped rows and a new verdict class, `Shipped, one acceptance
  criterion false`, earned by Milestone 10. Enumerated and cross-checked against
  `intel/requirements.md`: 120 rows, 120 distinct IDs, zero missing, zero extra.

- **24 forward requirements across Phases 12-16** — SUPPLY-01 … SUPPLY-03, ORCH-01 … ORCH-05,
  WEB-01 … WEB-04, PIPE-01 … PIPE-05, DEFER-01 … DEFER-03, DOCS-01 … DOCS-04. Forward total: **86
  across 16 phases.**

- **4 new variant entries across 2 new groups** (29: token mechanism; 30: coverage threshold),
  plus 14 run-5 warnings recorded as unsettled positions that are not `-v1`/`-v2` pairs. **All 69
  cumulative variants preserved unmerged; no winners picked.**

- **12 supersession chains**, including the first case in five runs of a later run **correcting an
  earlier run's direct code verification**.

- **Two in-place edits, no duplicates**: DEBT-01 extended (six stale `project/current-exports.txt`
  references became nine) and shed its four `actions-rs` references to PIPE-04; SEC-01 corrected
  (the `deny.toml` out-of-sync finding withdrawn, with SUPPLY-01/SUPPLY-02 carrying the corrected
  scope).

- **Phases 1-11 unchanged and unrenumbered.** The Milestone 7-8 detail block was wrapped in
  `<details>` per protocol item 2 with its `### Phase N:` headers intact, and the ROADMAP Overview
  was rewritten so the file reads as one roadmap rather than five appended fragments. All 16
  `### Phase N:` headers verified present and matching the 16 summary checklist entries.

Resume file: .planning/phases/02-functional-gap-closure/02-CONTEXT.md

**Next ingest run: none. There is no run 6.** All 263 documents in `.project/` are covered — 199
classified plus 64 task lists measured deterministically. Every shipped subsystem in the workspace
now has at least one ingested requirement behind it.

**Next action:** plan Phase 1 (`/gsd-plan-phase 1`) and work the roadmap in numeric order, **or**
take the four cheapest verified items first, none of which depends on anything — SUPPLY-01 (delete
`ci.yml:389-406`; 18 lines, and a Milestone 10 acceptance criterion becomes true), DEBT-01 (nine
stale references; the `api-surface` job has been red since commit `928c6d5`), WEB-03
(`ProviderCapabilities` stops over-reporting), and WEB-01/WEB-02 (the token mechanism, which has a
correctness consequence under the shipped Kubernetes Deployment).
(**Corrected by Phase 12 (plan 12-01), dated 2026-08-09, citing `ci.yml:465-482` and commit `cb75b2b`:**
this SUPPLY-01 citation was already stale — the deleted job actually sat at `ci.yml:465-482`, removed by Phase 9's
plan 09-06 in commit `cb75b2b`. SUPPLY-01 is closed, not a live cheap-item candidate; see
`REQUIREMENTS.md`'s "Verified by Phase 12" block.)

**Two things to carry into any planning session:**

1. **Phase 9 carries the only date in the corpus** — a RustSec risk acceptance expiring
   **2026-09-30**, roughly two months out. Numeric order puts it ninth; urgency does not. Phase 12
   should run with or before it.

2. **FACADE-02's D2 and DEFER-02 must be sequenced together.** One splits `user_service.rs`, the
   other tests it to ≥ 80%. Doing them independently means doing the work twice.

## Operator Next Steps

- Start the next milestone with /gsd-new-milestone
