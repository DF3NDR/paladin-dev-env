---
gsd_state_version: 1.0
milestone: v0.11.0
milestone_name: Crate Release
current_phase: 38
current_phase_name: design-seams-pricing-cost-producer
status: executing
stopped_at: Completed 38-03-PLAN.md
last_updated: "2026-09-26T01:05:04.527Z"
last_activity: 2026-09-26
last_activity_desc: Completed 38-03-PLAN.md
progress:
  total_phases: 1
  completed_phases: 0
  total_plans: 9
  completed_plans: 3
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-09-24 at the start of milestone v0.11.0)

**Core value:** A Rust developer can compose and run multi-agent workflows against any supported
LLM provider through stable port abstractions — without their own domain code depending on a
provider, transport, or storage implementation.
**Current focus:** Phase 38 — design-seams-pricing-cost-producer
(`/gsd-new-project` roadmapper), 10 phases (38-47), 35/35 requirements mapped. Source of truth:
`.project/Milestone_14-Treasurer/` plus the supporting scope in PROJECT.md *Current Milestone*.
Awaiting operator approval before `/gsd-plan-phase 38`.

**Progress:** [███░░░░░░░] 33%
requirements mapped, 100% coverage); no phase planned yet.

**Previous milestone:** v0.10.0 "Durable Agent Execution Runtime" closed 2026-09-23 — 19 phases
(22-37.1), 231 plans, 574 tasks, 88/89 requirements (SHIP-05 superseded by SHIP-06), 1,678 commits
(`495483ef..6a08c293`, 2026-09-01 → 2026-09-23). Archived to `milestones/v0.10.0-ROADMAP.md`,
`v0.10.0-REQUIREMENTS.md`, `v0.10.0-MILESTONE-AUDIT.md` (status `tech_debt`, 0 gaps) and
`v0.10.0-phases/`. Released as two tags: `v0.10.0` on merge commit `1d4a9724` (2026-09-18;
published 3 of 12 crates, since yanked) and `v0.10.1` on merge commit `f7dae267` (2026-09-21; run
`35659477719`, all 12 publishable crates on crates.io at `0.10.1`). Closeout type
`override_closeout` — see *Deferred Items*. Full record: MILESTONES.md.

**Prior:** v0.9.0 "Security Tooling" shipped 2026-09-01 (4 phases, 25 plans, 20/20 requirements,
240 commits; tag `v0.9.0` on `0b5d4106`, all eleven crates at `0.9.0`); v0.8.0 shipped 2026-08-24
(14 phases, 149 plans, 65/65); v0.7.1 shipped 2026-08-04 (4 phases, 38 plans, 25/25). All archived
under `milestones/`.

## Current Position

Phase: 38 (design-seams-pricing-cost-producer) — EXECUTING
Plan: 4 of 9
Status: Executing Phase 38 (plans 01-03 complete)
Last activity: 2026-09-26 — Completed 38-03-PLAN.md

## Performance Metrics

**Velocity:**

- Total plans completed: 415
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
| 22 | 17 | - | - |
| 22.1 | 7 | - | - |
| 23 | 12 | - | - |
| 24 | 14 | - | - |
| 25 | 14 | - | - |
| 26 | 21 | - | - |
| 27 | 26 | - | - |
| 28 | 17 | - | - |
| 29 | 9 | - | - |
| 30 | 3 | - | - |
| 31 | 7 | - | - |
| 32 | 5 | - | - |
| 33 | 6 | - | - |
| 34 | 9 | - | - |
| 35 | 10 | - | - |
| 36 | 13 | - | - |
| 36.1 | 14 | - | - |
| 37 | 8 | - | - |
| 37.1 | 16 | - | - |

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
| Phase 34 P01 | 17min | 2 tasks | 9 files |
| Phase 34 P02 | 15min | 1 tasks | 3 files |
| Phase 34 P03 | 18min | 2 tasks | 3 files |
| Phase 34 P04 | 28min | 2 tasks | 3 files |
| Phase 34 P05 | ~60min | 2 tasks | 2 files |
| Phase 34 P06 | ~18min | 2 tasks | 5 files |
| Phase 34 P07 | 16min | 2 tasks | 16 files |
| Phase 34 P08 | ~90min | 2 tasks | 3 files |
| Phase 34 P09 | ~75min | 2 tasks | 4 files |
| Phase 36 P01 | ~45min | 2 tasks | 9 files |
| Phase 36 P02 | ~55min | 3 tasks | 10 files |
| Phase 36 P03 | ~20min | 2 tasks | 5 files |
| Phase 36 P04 | ~25min | 2 tasks | 8 files |
| Phase 36 P05 | 35min | 2 tasks | 8 files |
| Phase 36 P06 | 30min | 2 tasks | 3 files |
| Phase 36 P07 | ~55min | 2 tasks | 3 files |
| Phase 36 P08 | ~2h10min | 3 tasks | 4 files |
| Phase 36 P09 | ~1h30min | 3 tasks | 6 files |
| Phase 36 P10 | ~1h50min | 3 tasks | 6 files |
| Phase 36 P11 | ~1h10min | 3 tasks | 2 files |
| Phase 36 P12 | ~1h | 2 tasks | 8 files |
| Phase 36 P13 | 25min | 3 tasks | 4 files |
| Phase 36.1 P01 | 20min | 2 tasks | 3 files |
| Phase 36.1 P02 | 15min | 2 tasks | 2 files |
| Phase 36.1 P03 | 20min | 3 tasks | 4 files |
| Phase 36.1 P04 | ~35min | 3 tasks | 9 files |
| Phase 36.1 P05 | ~15min | 3 tasks | 7 files |
| Phase 36.1 P06 | 6min | 3 tasks | 4 files |
| Phase 36.1 P07 | ~15min | 3 tasks | 4 files |
| Phase 36.1 P08 | ~20min | 2 tasks | 3 files |
| Phase 36.1 P09 | ~35min | 2 tasks | 3 files |
| Phase 36.1 P10 | ~15min | 2 tasks | 3 files |
| Phase 36.1 P11 | ~11min | 2 tasks | 7 files |
| Phase 36.1 P12 | ~20min | 3 tasks | 6 files |
| Phase 36.1 P13 | ~28min | 3 tasks | 3 files |
| Phase 36.1 P14 | ~20min | 2 tasks | 1 files |
| Phase 37 P01 | 35min | 3 tasks | 2 files |
| Phase 37 P02 | 50min | 1 tasks | 2 files |
| Phase 37 P03 | 53min | 2 tasks | 1 files |
| Phase 37 P04 | 7min | 2 tasks | 5 files |
| Phase 37 P05 | 14min | 2 tasks | 2 files |
| Phase 37 P07 | 25min | 2 tasks | 1 files |
| Phase 37.1 P01 | ~20min | 2 tasks | 2 files |
| Phase 37.1 P02 | ~40min | 3 tasks | 5 files |
| Phase 37.1 P03 | ~25min | 3 tasks | 3 files |
| Phase 37.1 P04 | ~16min | 3 tasks | 2 files |
| Phase 37.1 P05 | ~9min | 3 tasks | 3 files |
| Phase 37.1 P06 | ~28min | 3 tasks | 29 files |
| Phase 37.1 P07 | ~2h30min | 3 tasks | 3 files |
| Phase 37.1 P08 | ~35min | 2 tasks | 3 files |
| Phase 37.1 P09 | ~1h10min | 2 tasks | 2 files |
| Phase 37.1 P10 | ~20min | 2 tasks | 2 files (uncommitted) |
| Phase 38 P01 | ~5min | 2 tasks | 7 files |
| Phase 38 P02 | 57min | 2 tasks | 10 files |
| Phase 38 P03 | 23min | 2 tasks | 10 files |

## Accumulated Context

### Decisions

**Cleared at the v0.10.0 close (2026-09-23).** Every phase-level decision this section carried for
Phases 22-37.1 is recorded in its own `NN-CONTEXT.md` (and the SUMMARY decision lists) under
`milestones/v0.10.0-phases/`; the locked ones are ADRs in `.planning/decisions/` and the
`## Key Decisions` table in PROJECT.md. Only what a later milestone must honour is kept here:

- **Two-officer token model** — `Commissary` (input-side rationing) is kept and anchored;
  `Treasurer` (output-side spend governance) is reserved for Milestone 14, not built (ADR-0049,
  ADR-0050). `TokenBudget`, `TokenCounterPort`, `TokenUsage`, `max_tokens` are not renamed.

- **X-03 supersession is scoped** — clean breaks were allowed for Phases 31-33 only (ADR-0051); any
  further removal before v0.11.0 needs its own recorded supersession.

- **`paladin-memory` → `paladin-llm`** (`default-features = false`) is the workspace's one lateral
  adapter-to-adapter edge (Phase 33 D-01/D-03); `reqwest` never enters the normal graph through it.

- **MSRV 1.88, measured** from the `time` ≥ 0.3.47 / `rmcp` `process-wrap` chain, single-sourced in
  `workspace.package.rust-version` and enforced by the `msrv` CI job (Phase 22.1; `MIGRATION.md`
  §9.3).

- **Accepted deviations, to revisit** — tracing overhead +22.18 % (log sink) / +18.46 % (composite)
  against PRD 07's ≤ 3 % bar (Phase 28 D-16/D-37, `WINDOWS.md` row 35); run-inspection routes
  authenticated but single-tenant (Phase 27 WR-03, row 32); legacy `Runnable::Agent` runs emit no
  SSE/webhook events (WR-02, row 31); SSE `done` reports `halted` for a caller-cancelled run whose
  persisted status is `Cancelled` (Phase 27 D-14).

- **Release mechanics** — tags are cut on `main` merge commits per the Phase 29 two-SHA rule; the
  `.project/v0.10.0/09-program-acceptance-audit.md` sign-off boxes are ticked by a human, never an
  agent (Phase 29 D-17, Phase 37 D-00a); `make publish-dry-run` resolves from the local workspace
  overlay and is blind to registry publish order — `scripts/check-publish-order.sh` in
  `make check-gates` and CI is the gate for that class (Phase 37.1).

- **Partial-publish recovery** — maintainer's option A: a patch release through the same pipeline,
  the failed tag left in place and its GitHub Release bannered/pre-release, orphaned crate
  versions yanked by the maintainer only after the patch is registry-verified, one register row
  per crate in `docs/src/appendix/release-recovery.md` §5 (Phase 37.1 D-02).

The pre-close text of this section (Phase 33/32/29/28/23 decision digests and the tool-appended
`[Phase ?]` bullets back to Phase 3) is in this file's git history at commit `6a08c293`.

- [Phase ?]: ADR-0052: mid-run Treasurer enforcement — metering at LlmPort pricing decorator both paths, halt at WarEngine superstep boundary + agent-loop TokenBudget cutoff
- [Phase ?]: ADR-0053: append-only derive-on-read ledger; superstep-aggregate settlement selected at plan 38-01 checkpoint, D-15 key (run_id, superstep, attempt) kept unamended
- [Phase ?]: 38-02: CostTally (Empty/Priced/Unknown poisoning accumulator) added for run-level cost totals shared by TraceDispatcher::total_cost (38-06) and the agent loop (38-07); Task 1's cost_of_call arithmetic needed no correction under the 17-test contract.
- [Phase 38]: Hand-rolled exact-integer decimal parser instead of rust_decimal for treasurer.pricing (no new dependency). — Narrow grammar (digits + optional 1-9-digit fraction) is simple to prove exact with checked_mul/checked_add; keeps dependency count flat (no Cargo.lock/deny/audit/MSRV change, no package-legitimacy checkpoint).

### Pending Todos

Both acknowledged as deferred at the v0.10.0 close (see *Deferred Items*); neither is resolved.

- `todos/pending/2026-08-13-verify-local-coverage-reproduction.md` — user-owned; walk the
  documented `make services-up` → `make coverage` procedure on a Docker-capable machine and confirm
  it reproduces the CI figure (now 90.44 %, not the 82.39 % the todo quotes). `recheck_by:
  2026-10-16`.

- `todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md` — evaluate RustFS as the
  dev/test object store (FUT-10); the quay.io MinIO pin from quick task 260913-15w is terminal.
  `recheck_by: 2026-10-16`.

### Blockers/Concerns

**No blockers at the v0.10.0 close (2026-09-23).** All 19 phases `passed`; `WINDOWS.md`
`open_count: 0`; audit `tech_debt` with 0 gaps. The per-phase close notes this section carried for
Phases 23-33 are resolved and archived with their phases (`milestones/v0.10.0-phases/`); the
five-run ingest concern register (run 5's eight verified-open findings and the runs 1-4
carry-forward) was disposed by Phases 5-16 and is preserved in this file's git history at commit
`6a08c293`. Open concerns carried into the next milestone:

- **Coverage is CI-attributed, not locally measurable** — this devcontainer has no Docker; the
  82 % floor (ADR-0006) is read from the CI `coverage` job (90.44 % at PR #56). The local
  reproduction walkthrough is the pending user-owned todo above.

- **Tracing overhead** accepted at 6-7× the PRD bar (D-16); `TraceDispatcher::emit` /
  `LogTraceSink` serialisation is the named optimisation target.

- **Webhook SSRF guard does not pin the resolved address** between check and connect — DNS
  rebinding is a documented limitation (`src/application/services/run/webhook/ssrf.rs` module docs,
  `security.instructions.md`).

- **Terminal MinIO pin** — no newer community `minio/minio` or `mc` tag will ever exist; the
  dev/test stack and the Kubernetes smoke test depend on a frozen third-party image (FUT-10 todo).

- **`cargo-semver-checks` 0.50.0 coverage gap** for inherent-method return-type and tool-coverage
  classes — covered by `MIGRATION.md` §9.2 rows instead (Phases 32/33).

- **Nyquist validation** — seven v0.10.0 phases at `VALIDATION.md` `status: draft` (22, 24, 29,
  30, 34, 36, 36.1) and Phase 28 at `nyquist_compliant: false`; archived phases 05-21 likewise
  unreconciled. `/gsd-validate-phase <N>` each; coverage TODO, not a compliance failure.

- **Bookkeeping drift in the corpus acceptance audit** — the seven §11 judgment-tier sign-off boxes
  and the `v0.10.0` tag box in `.project/v0.10.0/09-program-acceptance-audit.md` are still `- [ ]`
  on disk although 29-UAT recorded the pass and the tag was cut; tick by hand or annotate as
  superseded by §13 (v0.10.1, ticked).

- ~~**`release/*` ruleset bypass** granted temporarily on 2026-09-21 for the v0.10.1 push must be
  removed now that PR #56 is merged.~~ **Removed 2026-09-23** — ruleset `20868128` is back to the
  checked-in shape (`bypass_actors: []`, `current_user_can_bypass: never`); `release/*` branches
  accept one push again, so plan repeat pushes as PRs into the branch.

- **Phase 37 reads incomplete to `init.manager`** (8 of 11 plans with SUMMARYs; 37-09..37-11
  superseded) — expected, documented in MILESTONES.md *Known Gaps*; do not "fix" by fabricating
  SUMMARYs.

### Quick Tasks Completed

| # | Description | Date | Commit | Directory |
|---|-------------|------|--------|-----------|
| 260912-whj | Regenerate API surface snapshot so ci.yml API Surface Tracking check passes on feature/v0.10.0-web3sec-dogfooding | 2026-09-12 | 786a3ba5 | [260912-whj-regenerate-api-surface-snapshot-so-ci-ym](./quick/260912-whj-regenerate-api-surface-snapshot-so-ci-ym/) |
| 260913-15w | Pin MinIO service image to quay.io last known-good release after Docker Hub minio/minio removal | 2026-09-13 | 06765765 | [260913-15w-pin-minio-service-image-to-quay-io-last-](./quick/260913-15w-pin-minio-service-image-to-quay-io-last-/) |
| 260913-h7l | Replace dl.min.io mc download in ci.yml with checksum-verified pinned GitHub release asset after MinIO retired community downloads | 2026-09-13 | 9d0aa7a0 | [260913-h7l-replace-dl-min-io-mc-download-in-ci-yml-](./quick/260913-h7l-replace-dl-min-io-mc-download-in-ci-yml-/) |
| 260916-h40 | Regenerate API surface baseline for Phase 32 window exports and add a local pre-push API surface gate | 2026-09-16 | cd185c9b | [260916-h40-regenerate-api-surface-baseline-for-phas](./quick/260916-h40-regenerate-api-surface-baseline-for-phas/) |

### Roadmap Evolution

- Phase 15.1 inserted after Phase 15: Git & CI Governance — branch protection, trigger surface, gitflow model, docs/BRANCH_PROTECTION.md (URGENT)
- Phase 17 added: Additional LLM Provider Adapters — provider-selection study (PROV-01) then feature-gated adapters for survivors (PROV-02..04); first forward phase beyond the ingest
- Phase 18 added: Rust SAST: evaluate and adopt CodeQL — new Security Tooling milestone; SAST-01..04 minted at roadmap time
- Phase 19 added: crates.io Trusted Publishing — replace the long-lived CARGO_REGISTRY_TOKEN with OIDC-issued ephemeral tokens; PUB-01..PUB-05 minted at roadmap time
- Phase 20 added: Release Pipeline Recovery — idempotent re-runs on the same tag, a pre-publish gate over tag/manifest/changelog/CI agreement, and a stuck-halfway runbook with a yank policy; PUBOPS-01..PUBOPS-05 minted at roadmap time
- Phase 21 added: Release Artifacts — Curated Release Notes and Attached Distributables (v0.9.0; ARTIFACT-01..06 minted, twenty-second prefix)
- Phase 22.1 inserted after Phase 22: Engine readiness defect and MSRV follow-up (readiness defect from 22-16 audit; MSRV 1.85 vs rmcp-pinned process-wrap decision; green postgres-integration run confirmation) (URGENT)
- Phase ? changed: Phase 22.1 scope grew on 2026-09-03: BUG-04 (resume rebuilds the Frontier from scratch, losing pre-crash edge resolutions) promoted from CONTEXT.md deferred ideas into the phase at the 22.1-05 checkpoint by developer decision; ENG-04 re-opened for the phase; CI evidence to be re-captured on the final head.
- Phase 30 added: Token-Economy Vocabulary & Commissary Anchoring — vocabulary rule, Commissary ADR + mdBook page, Treasurer reservation ADR, `max_tokens` table, Quartermaster purge, clean-break versioning ADR (VOCAB-01..07 minted, twenty-seventh prefix; docs only; source `.project/Milestone_13-Token-Economy/Epic_1`)
- Phase 31 added: Lossless Token Accounting — full `TokenUsage` carried port→`RunFinished`→herald, optional cache/reasoning fields, `from_total` removed from the battalion path, streaming parity (ACCT-01..05, twenty-eighth prefix; keystone; clean break under the X-03 supersession)
- Phase 32 added: Unified Token Primitives — `TokenCounterPort::is_exact`, `Commissary::new` without `is_exact_counter`, legacy `TokenCounter`/`TokenCounterFactory` retired, one shared window resolver with strict mode (PRIM-01..05, twenty-ninth prefix; clean break)
- Phase 33 added: Commissary In-Tree Adoption — RAG through `Commissary::dispense` with shed records + marker, integration-tested production caller, Phase 29 release gates re-sealed (COMM-01..04, thirtieth prefix). Milestone 14 Treasurer (`.project/Milestone_14-Treasurer/`) reserved, not roadmapped.
- Phase 34 added: Documentation Currency Audit — read-only inventory of mdBook / rustdoc / examples gaps against Phases 22-33 (and any v0.9.0 leftovers); scopes Phases 35-36 (2026-09-17, pre-tag release readiness)
- Phase 35 added: mdBook Currency — close every Phase 34 mdBook gap; `mdbook build` + linkcheck green (2026-09-17)
- Phase 36 added: Rustdoc Zero-Warning Bar & Examples Currency — `cargo doc` 73→0 warnings so CI "Check documentation" is green, 14 `--all-features` intra-doc links resolved, `examples/` + `doc-examples` current (2026-09-17)
- Phase 37 added: v0.10.0 Crate Release — re-seal the Phase 29 gates on the final commit, merge to `main`, `release.yml` tags `v0.10.0`, all publishable crates on crates.io at `0.10.0` (2026-09-17)
- Phase 36.1 inserted after Phase 36: Deferred Items Closure — walk the Phase 31/32/34/35 deferred-items registers, WINDOWS.md #36-37 and the two pending todos; fix, waive with reason, or re-home each; bring WINDOWS.md back into agreement with the registers before Phase 37 tags v0.10.0 (URGENT)
- Phase 37.1 inserted after Phase 37: v0.10.1 Patch Release — tag v0.10.0 published 3/12 crates (battalion versioned dev-dep vs CRATES order); maintainer chose recovery option A (URGENT)

## Deferred Items

### Acknowledged at v0.10.0 milestone close (2026-09-23)

**Verification overrides: 0** — all 19 phases (22-37.1) report `verification_status: passed` with
`behavior_unverified: 0`. **Phase-completion override: 1** — Phase 37 reads `phase_complete: false`
because plans 37-09, 37-10 and 37-11 were superseded by Phase 37.1 (dated notes in the plan files,
no SUMMARY) after the `v0.10.0` tag published 3 of 12 crates; its `37-VERIFICATION.md` is `passed`
5/5 with SC4 stated as superseded. **Requirement gap acknowledged: 1** — SHIP-05 superseded by
SHIP-06, deliberately unticked (MILESTONES.md *Known Gaps*). **Open artifacts acknowledged: 2** —
both pending todos, each already dispositioned as deferred past v0.10.0 by Phase 36.1 (CURR-20)
with `recheck_by: 2026-10-16`. Closeout type `override_closeout`.

| Category | Item | Status | Deferred At |
|----------|------|--------|-------------|
| todo | `2026-08-13-verify-local-coverage-reproduction` | Open — owner: repo maintainer. Walk `make services-up` → `make coverage` on a Docker-capable machine and confirm the CI figure reproduces (CI now 90.44 %). Carries no `resolves_phase` tag by design | v0.8.0 close; re-acknowledged v0.9.0 and v0.10.0 |
| todo | `2026-09-13-evaluate-rustfs-replacement-for-minio` | Open — owner: repo maintainer; FUT-10 in the archived requirements. The quay.io MinIO pin (quick task 260913-15w) is terminal | Phase 36.1 (2026-09-18); acknowledged v0.10.0 close |
| phase | Phase 37 plans 37-09..37-11 | Superseded by Phase 37.1 — never executed; scope (post-publish verification, milestone-close recording for `v0.10.0`) delivered under `v0.10.1` | Phase 37.1 (2026-09-19) |
| requirement | SHIP-05 | Superseded by SHIP-06; amend-at-source note dated 2026-09-22 in `milestones/v0.10.0-REQUIREMENTS.md` | Phase 37.1 |
| testing | Nyquist validation unreconciled for Phases 22, 24, 29, 30, 34, 36, 36.1 (`draft`) and 28 (`nyquist_compliant: false`) | Coverage TODO, not a compliance failure — `/gsd-validate-phase <N>`. Joins the same open item for archived Phases 05-21 | v0.10.0 close |
| debt | Audit `tech_debt` register (tracing overhead D-16; `WINDOWS.md` waived rows 23-25, 29-35, 45, 51-55; §11 sign-off boxes; IN-01/IN-02; SSE `done` collapse; three roadmap-level v2 lines; Milestone 14 reserved) | Inventoried with owners in `milestones/v0.10.0-MILESTONE-AUDIT.md`, not duplicated here | v0.10.0 close |

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

**Last session:** 2026-09-26T01:04:27.248Z
**Stopped at:** Completed 38-03-PLAN.md
next milestone; ROADMAP collapsed to milestone groupings with no planned phase; PROJECT.md evolved;
RETROSPECTIVE.md extended. Tag `v0.10.0` pre-existed (2026-09-18, merge commit `1d4a9724`) and was
not re-cut; `v0.10.1` (2026-09-21) is the release consumers install.
**Resume file:** None
**Branch state:** the close commits sit on `chore/37.1-post-close`; open a PR to `main` (the
repository enforces PR-only merges to `main`, ADR-0043/0044) — `.planning/`-only, no crate change,
no tag.

**Next action:** `/gsd-new-milestone` — new phases start at Phase 38. Candidate scope is listed
under *Next Milestone Goals* in PROJECT.md (Milestone 14 Treasurer, the FUT-01…10 v2 list, the
accepted deviations D-16 / rows 31-32, the Nyquist backfill, RustFS). Housekeeping that needs no
milestone: tick or annotate the corpus audit §11 boxes (the `release/*` ruleset bypass was removed
2026-09-23); `/gsd-validate-phase` for the seven `draft` phases.

## Operator Next Steps

- **Milestone v0.10.0 closed 2026-09-23** (`override_closeout`; two todos acknowledged; Phase 37's
  three superseded plans and SHIP-05 recorded as known gaps). Archives under `milestones/v0.10.0-*`;
  record in MILESTONES.md; lessons in RETROSPECTIVE.md.

- **Next:** open a PR for `chore/37.1-post-close` → `main` (planning-only), then `/clear` and
  `/gsd-new-milestone`. New phases start at Phase 38; `.planning/REQUIREMENTS.md` is recreated by
  that command.

- **Housekeeping, no milestone needed:** ~~remove the temporary `release/*` ruleset bypass~~ (done
  2026-09-23, ruleset `20868128` matches `.github/rulesets/protect-release-branches.json`); tick or annotate the seven §11 sign-off boxes and the `v0.10.0`
  tag box in `.project/v0.10.0/09-program-acceptance-audit.md` (§13 for v0.10.1 is ticked);
  `/gsd-validate-phase` 22, 24, 28, 29, 30, 34, 36, 36.1 (advisory).

- **Recheck by 2026-10-16:** the two pending todos (`todos/pending/`).
