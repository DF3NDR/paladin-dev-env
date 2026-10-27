# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v0.7.1 — Milestone 1 close-out

**Shipped:** 2026-08-04
**Phases:** 4 | **Plans:** 38 | **Tasks:** 88 | **Commits:** 255

### What Was Built

- **Nine ADRs (`0001`–`0009`)** settling every contested definition in the corpus, each naming its
  chosen variant and the shipped code it was checked against. `.planning/decisions/` and
  `.planning/ledgers/` were stood up as new document classes to hold them.
- **Code changes applying those decisions** — `ProviderCapabilities.temperature_range`, Formation's
  1-Paladin minimum, the `BattalionCheckpointConfig` rename, and Formation's per-Paladin
  times/tokens/`node_errors` on `BattalionResult` rendered through all three Heralds.
- **A real UTF-8 panic fix** in `TableHerald::truncate_text` plus the two adjacent panic paths that
  shared the same defective helper.
- **An offline coverage pipeline** (`rustc -C instrument-coverage` → `llvm-profdata` → `llvm-cov`)
  that works with no `cargo-llvm-cov`, no network and no Docker — 84.79% at Phase 1, reproduced at
  85.56% / 85.92% in Phase 3.
- **62 previously dead tests activated** — 25 in `tests/unit/llm/`, 37 in `tests/cli/` — none
  deleted, plus four real Commander error-path tests replacing `#[ignore]`d stubs.
- **Release coherence** — twelve manifests on version 0.7.0 / edition 2024, advisory posture
  recorded, quickstart fixed and measured at 15 minutes, gate suite at 2,924 tests / 185 doc tests
  / 47 example targets.

### What Worked

- **Recording a decision before applying it.** Phases 1→2 ran as "ADR first, code second", and the
  audit's highest-value check — do the six ADRs match shipped code? — came back clean on all of
  them. The ADR gave the code change an unambiguous target.
- **Refusing to paper over a missing prerequisite.** Plan 01-08 declined to write a ledger row it
  could not source, and 01-04 halted at its own Task 1 precondition rather than fabricating a
  coverage number. Both were later closed properly by gap-closure plans. The halts cost time and
  bought correctness.
- **Measuring instead of restating.** The "22 runnable examples" figure had propagated through five
  documents from a single Milestone-1 validation report. Phase 4 counted the tree (47 `.rs` files,
  4 declared targets) and replaced the count with a property — "every example target builds" —
  that cannot go stale the same way.
- **Re-verification after gap closure.** Three of four phases entered verification as `gaps_found`
  and were re-verified to `passed` only after the gaps were actually closed. No phase was talked
  into passing.

### What Was Inefficient

- **The coverage question was answered three times.** Plan 01-04 halted on it, 01-09 measured it,
  01-10 wrote the ADR, 01-12 flipped the checkbox — and Phase 3 then re-measured it twice more. A
  single decision consumed five plans across two phases because the measurement tooling was
  established only after the requirement was already in flight.
- **REQUIREMENTS.md bookkeeping drifted from reality repeatedly.** Two separate gap-closure plans
  (02-11, 03-07/03-08) existed solely to reconcile checkboxes and traceability rows against work
  that was already done. The record lagged the code by a whole verification cycle each time.
- **A premature checkbox flip had to be reverted.** RECON-07 was marked satisfied before ADR-0006
  existed (commit `799c53f` reverted it). Caught, but only after it shipped into the file.
- **Nyquist validation never ran at all.** All four `VALIDATION.md` files sat at `status: draft`
  through the entire milestone and nobody noticed until the closing audit.

### Patterns Established

- **ADR-first precedence.** `.planning/decisions/` outranks PRDs and ingested docs; the shipped tree
  outranks both. Wired into PROJECT.md as an explicit precedence order.
- **Machine-parseable decision records.** ADRs are validated by `adr-parser.cjs`, and checkbox flips
  can be gated on that parse succeeding — a decision cannot be cited before it is well-formed.
- **Provenance-standard measurement (D-17).** Every recorded figure carries `rustc -vV`,
  `cargo --version`, `git rev-parse HEAD`, `date -u` and raw pasted stdout — not a restated number.
  This is what made the 84.79% figure auditable months later.
- **Deferrals carry a named owner or an explicit "no owner assigned".** Never a silent drop. Ten
  deferrals left this milestone; every one names where it goes.
- **Superseded work is preserved, not deleted.** The 2026-05-27 benchmark run stayed in place marked
  superseded; plan 01-04 got a disposition record rather than removal.

### Key Lessons

1. **Establish the measurement before writing the requirement that depends on it.** The coverage
   gate cost five plans because the requirement (RECON-07) was written before anyone knew whether
   the number could be produced in this sandbox. Cheap fix: a feasibility spike on the measurement
   tool during discuss-phase.
2. **Bookkeeping is not free — schedule it inside the plan that does the work.** Every phase needed
   a dedicated follow-up plan to reconcile REQUIREMENTS.md. Flipping the checkbox should be the
   last task of the plan that earns it, not a separate cleanup pass.
3. **A count copied from a report is a liability; a property proven by a command is not.** Prefer
   success criteria that re-derive themselves ("every example target builds") over criteria that
   restate a number ("22 examples").
4. **Verification timestamps are load-bearing.** A documentation-only commit added after the fact
   invalidated Phase 1's verification and forced this milestone to close as `override_closeout`.
   If a phase is verified, later edits to its directory need either a re-verify or a conscious
   override.
5. **Workflow defaults can be wrong for the project.** The stock milestone-close step deletes
   REQUIREMENTS.md; here that file carries forward scope for twelve unstarted phases. Read what a
   destructive step actually targets before running it.

### Cost Observations

- Sessions: not tracked this milestone.
- Notable: the offline coverage pipeline was the single highest-leverage artifact produced — it
  unblocked QUAL-01/02/03 and is reusable by Phase 15's PIPE-02 without modification.
- Notable: three phases required a second verification pass. Budgeting one re-verification per
  phase as the expected case, rather than the exception, would have made the schedule honest.

---

## Milestone: v0.8.0 — Milestone 2-12 close-out & Provider Expansion

**Shipped:** 2026-08-24
**Phases:** 14 (5-17, incl. inserted 15.1) | **Plans:** 149 | **Commits:** 1,014

### What Was Built

Four ingest-derived milestone blocks closed out, plus the first forward work beyond the ingest.
Five as-shipped ledgers now carry 554 `REQ-*` rows with `file:line` verdicts. The quality gates
Deferred-QA Epic 25 specified and nobody started exist and run. Nine LLM providers ship where three
did. Branch protection went from literally nothing to three applied rulesets with 44 required
contexts.

### What Worked

- **Record-then-apply, split across phases.** Phases 5, 7, 10 and 13 recorded; 6, 8, 9, 12, 14
  changed code. The zero-`.rs` boundary on the recording phases was independently re-measured at
  each close and held every time. Discovery never quietly became implementation.
- **Verifiers that abstain rather than pass.** Four phases came back `gaps_found` or `human_needed`
  on their first pass and were re-verified after real fixes (06, 14, 16, 17). Phase 12 refused to
  mark a clause passing because no CI run existed that could confirm it — and it was right to.
- **Disclosure over silent scope-cutting.** Phase 13 found a route defect it could not fix without
  breaching its own zero-`.rs` boundary, and handed it forward with the exact fix and a named
  owner. Phase 14 fixed it. That is the mechanism working.
- **Measuring a tool instead of trusting it.** The Snyk probe is the single highest-value thing this
  milestone produced: four deliberate vulnerabilities, 0 findings in Rust, 3 in JavaScript. It
  turned six plans' worth of unsatisfiable blocking into a recorded, evidence-backed removal.

### What Was Inefficient

- **A stale requirement blocked verification in six plans before anyone measured it.** The Snyk
  mandate sat in an untracked instructions file from Phase 15.1 through Phase 17, recorded as
  "not run" five times, before being tested. The probe took an afternoon. It should have been the
  first response to the second failure, not the tenth.
- **The ROADMAP's milestone table went stale for four blocks and thirteen phases.** Every phase was
  `[x]` while the table read "Not started". Nothing was unbuilt; the record simply was not
  maintained, in a milestone whose entire purpose was making the record true.
- **`Requirements: TBD` carried into execution once and cost a retroactive settlement.** Phase 15.1
  shipped seven verified success criteria with no identifiers, which the milestone audit then had
  to record as a traceability silence rather than close.
- **Disk exhaustion silently degraded verification twice.** Plans 14-01 and 14-04 could not run
  `cargo test --workspace` at all (99% full). Targeted verifies covered it, but the gap was
  environmental and unflagged until close.

### Patterns Established

- **Probe the scanner, not just the code.** A clean result from a tool that cannot analyse the
  language reads as assurance while meaning nothing. Adopted as a hard precondition in SAST-01.
- **Dated at-source correction banners that retain the original text.** Applied consistently across
  ledgers, ADRs and `.project/` annotations; nothing was silently edited away.
- **Mint requirement IDs at roadmap time.** Direct consequence of the 15.1 experience; Phase 18
  minted `SAST-01`…`SAST-04` before any planning began.
- **Guards that parse the artifact, not prose about it.** `check-advisory-register.sh` and
  `check-workflow-triggers.sh` both enforce relationships earlier phases had only asserted.

### Key Lessons

1. **A gate that cannot fail is worse than no gate.** The duplicate audit job, the Snyk mandate, and
   the path-filter trap in `CLAUSE_CONTEXT` are three instances of one defect class: something that
   reports success without doing work.
2. **Check whether a later phase already closed the finding.** The milestone audit's first pass
   carried two Phase 12 items forward as open; Phase 15.1 had closed both five days later. Reading
   a VERIFICATION.md without asking what came after it produces confidently stale conclusions.
3. **A record's account of itself drifts even when the record is correct.**
   `SECURITY-EXCEPTIONS.md` governed eleven suppressions correctly while its own heading said ten.
4. **Deferring with a named owner and a working fix is not scope-cutting.** Deferring without one
   is.

### Cost Observations

- Sessions: not tracked per-milestone
- Notable: four phases required a second verification pass, and three required `--gaps` replanning
  rounds (17 needed three). The re-verification loop, not first-pass execution, is where the
  quality came from — and it is not free.

## Milestone: v0.9.0 — Security Tooling

**Shipped:** 2026-09-01
**Phases:** 4 (18-21) | **Plans:** 25 | **Commits:** 240

### What Was Built

The supply-chain and release posture, settled by measurement. CodeQL proven to analyse all 385
first-party `.rs` files and then disqualified as a required-check Rust SAST by a five-class probe
(retained advisory-only, version-scoped verdict). crates.io publishing moved to per-run OIDC
tokens with the standing credential revoked and deleted. A pre-publish consistency gate,
idempotent same-tag re-runs, registry-state publish detection, and a rehearsed stuck-halfway
runbook. Release artifacts made real: curated-changelog body, feature-correct binaries, a
digest-bound image, verifiable checksums — proven by the first fully-green release run in this
project's history (`v0.8.1-rc.5`).

### What Worked

- **Pre-registered criteria before every measurement.** Phase 18 wrote and committed promotion
  thresholds, re-probe criteria and confound-test criteria *before* each run produced the numbers
  they judge — no threshold was ever retrofitted to a result, across four measurement rounds
  including a self-correction.
- **Live rehearsal on throwaway rc tags as the proof standard.** rc.2 proved the OIDC exchange,
  rc.3/rc.4 proved the recovery path (and found two real gate bugs the phase then fixed),
  rc.5 proved the artifact path end-to-end. Every pipeline claim in this milestone traces to a
  real run ID, not a re-reading of the workflow.
- **Ratchet ordering as a requirement, not a preference.** The OIDC path was proven with a real
  publish *before* the old token was revoked; the archive files were committed *before* originals
  were deleted. Nothing was ever in a state where the only copy of a capability was the untested
  one.
- **Narrow, named human backstops — which then actually closed.** Verifications held three claims
  open as `human_needed` rather than letting CI-internal corroboration stand in (token revocation,
  out-of-band pull-by-digest, `paladin-cli` execution). All three were closed by recorded UAT
  within days, with evidence including the failure shapes met on the way.
- **Independent re-verification against live state.** Verifiers queried the GitHub API, the
  crates.io registry and cited run IDs directly instead of trusting SUMMARY.md or even the
  phases' own evidence files — and in every case the claims held.

### What Was Inefficient

- **The binary-attachment defect survived every prior release run silently.** `build-binaries`
  produced nothing (Cargo silently skips a binary with unmet `required-features`) on every release
  since the matrix was written, and was known-broken through Phases 19-20 before Phase 21 owned
  it. Four `fail-fast: false` legs reported green around a missing artifact for months.
- **Three phases rewrote the same `release.yml`, serially.** The 19 → 20 → 21 ordering was honored
  and correct — but shared-file blast radius made the milestone strictly sequential; nothing in
  it could parallelize across phases.
- **A plan was written for a branch not taken.** 18-05's observation-window measurement was mooted
  by the disqualified verdict and closed as not-applicable — cheap, but a branch-aware plan would
  have gated it on the 18-03 verdict instead of scheduling it unconditionally.
- **Five rc tags were consumed as rehearsal budget.** Deliberate and worth it, but the tag
  namespace now carries five `v0.8.1-rc.*` entries whose only meaning is "pipeline rehearsal" —
  worth a note in the release docs so a future reader does not mistake them for abandoned
  releases.

### Patterns Established

- **Version-scoped tool verdicts with a written re-probe trigger.** "Disqualified at CodeQL
  2.26.3 / rust-queries 0.1.40" plus a kept fixture and a named re-run condition — a verdict that
  can be revisited without being re-derived.
- **Coverage before findings, recursively.** Every scan re-proved its analysed-file denominator
  before any finding count was interpreted — the Snyk lesson turned into a standing discipline.
- **Registry state over error prose.** Already-published detection reads the crates.io index;
  release existence reads the GitHub API by tag; both replaced string-matching on error output.
- **Honest tested/untested status lines.** The runbook said "untested" until the rehearsal ran;
  COVERAGE.md's overstated digest-pull row was corrected rather than left to read as proof.

### Key Lessons

1. **A green run proves what it exercised, nothing more.** The first fully-green release run
   arrived only after this milestone made every silent-success path (skipped publish, missing
   binary, `::warning::`-then-pass) into a hard failure. Everything green before that was green
   around holes.
2. **Backstops close when they are narrow and named.** All three `human_needed` items specified
   the exact command, the expected output and why only a human could run it — and all three were
   discharged within days. A vague "needs manual verification" would still be open.
3. **Ordering is itself a security control.** Prove-then-revoke and archive-then-delete both
   turned potentially irreversible transitions into checkpointed ones.
4. **A measured disqualification is a deliverable.** CodeQL joins Snyk in the evidence record —
   with the crucial difference that its coverage was proven, so the verdict is a detection gap,
   not an analysis mirage. The next person asking "why no Rust SAST?" gets a dated answer.

### Cost Observations

- Sessions: not tracked per-milestone
- Notable: 240 commits over 8 days for 25 plans — the smallest, fastest milestone yet, and the
  first with zero `gaps_found` verification passes (all four phases passed first time, two with
  named human backstops). The rehearsal budget (five rc tags, three live pipeline runs) replaced
  the re-verification loops that consumed v0.8.0.

## Milestone: v0.10.0 — Durable Agent Execution Runtime

**Shipped:** 2026-09-23 (closed; `v0.10.0` tagged 2026-09-18, `v0.10.1` released 2026-09-21)
**Phases:** 19 (22-37.1, three inserted) | **Plans:** 231 (228 executed, 3 superseded) | **Commits:** 1,678

### What Was Built

The runtime the framework had been promising. A cyclic superstep engine (`WarGraph`/`WarEngine`)
over typed Battlefield state, one Waypoint per superstep across three storage backends under a
single 13-function contract suite, and resume with zero re-execution — including a persisted
frontier and intra-superstep fan-out progress. Node-driven control flow (Directive, Muster, nested
Battalion subgraphs, LLM-evaluated edges behind a fail-closed registry that fixed BUG-01). Parley
pause/resume with total-before-write validation, Chronicle replay/fork, batch-wide graceful
shutdown. Aegis per-node fault tolerance with a table-driven transience taxonomy, provider
fallback and node caching. An execution-middleware agent runtime with twelve built-ins, the Vault,
structured output. A background-run platform API on durable queue and repository ports with a
lease-heartbeating worker pool, versioned assistants, schedules and SSRF-guarded signed webhooks.
A twelve-variant trace stream with log/OTel/SSE sinks, Mermaid/DOT exporters and the new
`paladin-eval` crate. Then, mid-milestone and before the tag: lossless six-field token accounting
end to end, one counting contract and one window resolver, RAG rationed through the Commissary;
and a documentation-currency programme that inventoried the debt read-only before closing every
row by ID (mdBook current, `cargo doc` 65 → 0 warnings and `--all-features` 77 → 0, 14 new
example programs, 101/101 public-API doctests). Released to crates.io as `0.10.1` after the
`v0.10.0` tag published 3 of 12 crates.

### What Worked

- **One contract suite per port, run unchanged against every adapter.** `WaypointPort` (13
  functions, three stores), `RunRepositoryPort` (14 clauses, SQLite + Postgres), `RunQueuePort`
  (in-memory + Redis Lua), `NodeCachePort` (9 cases). New backends were a migration and a
  `use`, never a re-derivation of semantics; the Postgres Tier-2 run confirmed it live.
- **Red committed before green, visible in history.** BUG-01, BUG-03, BUG-04, the `EPIPE`
  regression (a 200,000-character case observed red first), the API-surface pin harness
  (10/15 red → 15/15 green). Every defect fix in this milestone can be checked out at its failing
  commit.
- **Inventory before editing.** Phase 34 measured the documentation debt in one read-only pass —
  and found the carried "14 unresolved links" was really 77 `--all-features` errors across eight
  crates (5.5× undersized) plus 65 default-feature warnings. Phases 35-36 then closed rows by ID
  with set-equality reconciliation both ways, so nothing was fixed twice or skipped once.
- **Inserted phases for discovered scope, each with its own verification.** 22.1 (engine
  readiness defects + measured MSRV), 36.1 (deferred registers to `open_count: 0` before the tag),
  37.1 (the patch release). None was folded into a running phase's tail; all three closed
  `passed`.
- **Gates re-sealed on the final commit, three times** (Phases 33, 37, 37.1), with a corpus
  acceptance audit whose sign-off boxes an agent is forbidden to tick. The Phase 37.1 re-seal was
  what let the maintainer merge a 23-day, 1,678-commit milestone on one PR read.
- **Independent re-derivation against live state, again.** Verifiers for 37 and 37.1 read
  crates.io's sparse index (checksum, `yanked`, `trustpub_data`), the GitHub API and the git
  remote directly; the "12 publishable crates" count was derived from `cargo metadata` every time
  it was used, never copied.

### What Was Inefficient

- **The first real release stalled at crate 4 of 12 on a defect the local gate was structurally
  blind to.** `make publish-dry-run` runs `cargo publish --workspace --dry-run`, which resolves
  sibling crates from the workspace overlay, not the registry — so `paladin-battalion`'s two
  forward-pointing *versioned* dev-dependencies (introduced in Phases 23 and 28) were green
  locally and fatal live. v0.9.0's lesson ("rehearse the pipeline") had been learned for the
  *artifact* path on rc tags; v0.10.0 went to a real tag with no rc rehearsal of the *publish*
  path. A second, unrelated `EPIPE` race in the release script fired only because this release
  body was the first to exceed ~46 KB.
- **The biggest phase needed the most rework.** Phase 27 (26 plans) verified 12/17 and needed
  eight gap-closure plans (27-19..27-26); Phases 24 and 35 also re-verified from `gaps_found`,
  and 22 from `human_needed`. Four re-verification passes against v0.9.0's zero.
- **A PRD acceptance bar written without a baseline.** PRD 07's ≤ 3 % tracing-overhead criterion
  was missed by 6-7× (+22 %/+18 %) and *accepted* rather than met (D-16). The bar predated any
  measurement of the dispatcher it constrains.
- **"Complete" twice before shipped.** Phases 22-29 closed 2026-09-10 with `0.10.0` bumped and
  untagged; the milestone was then extended with Phases 30-33 (2026-09-14) and 34-37
  (2026-09-17). The untagged bump made this legitimate, but eleven more days of scope shipped
  under a version number that had already been declared done — and the ROADMAP's milestone row
  read "Phases 22-33", then "22-37", while the truth was 22-37.1.
- **Planning artefacts on the wrong branch.** `.planning/` commits accumulated on
  `feature/phase-33`; Phase 37.1's first plan was cherry-picking 24 of them onto the release
  branch. Zero conflicts, but pure tax.
- **External breakage with no upstream.** Docker Hub deleted the community MinIO images
  mid-milestone; two quick tasks restored green with a pin to the last quay.io release, which is
  terminal — a frozen third-party image now underpins four CI jobs.
- **Nyquist validation is optional and therefore skipped.** Seven of nineteen phases closed with
  `VALIDATION.md` at `draft`; the step is not on the phase-close critical path, so it does not
  happen.

### Patterns Established

- **Fail-closed registries for anything named in a document** — edge evaluators, retry predicates,
  error handlers: an unknown name is an error at compile-time of the graph, never a silent default.
- **Redact, then bound.** Every external response body that reaches an error or a log passes the
  redaction pass before truncation, on character boundaries (`map_http_status`, `FailRun`).
- **`#[non_exhaustive]` on every public enum a phase touches** (X-10), with a `MIGRATION.md` §9.2
  row and a `cargo semver-checks` allowlist row for each break — no shims, by ADR-0051.
- **Amend at source with a dated note; never rewrite the record.** SHIP-05 stays unticked with its
  supersession note; stale PROJECT.md counts carry a *(Corrected …)* paragraph beside them.
- **`WINDOWS.md` as the cross-phase defect ledger**, `open_count` gating `/gsd-ship`, and a
  dedicated closure phase (36.1) that walks every register before a tag.
- **Human-only acts named as such.** Sign-off boxes, yanks, token revocations and ruleset changes
  are recorded as the maintainer's, performed in-session, and written down verbatim — never done by
  an agent under an assumed order.
- **Offline gates that exercise the real resolution path.** `scripts/check-publish-order.sh`
  derives the per-crate publish order from `cargo metadata` and fails on the exact tree that broke
  `v0.10.0`; wired into `make check-gates` and the PR-time CI job, not the push-to-main-only one.

### Key Lessons

1. **A dry run that resolves from a local overlay is not a rehearsal of publishing.** The gate
   was green and its command genuinely passed; it simply could not see the class of defect. When
   a gate cannot observe a failure mode, build one that can — or rehearse the real path on a
   throwaway tag before the real one.
2. **Acceptance bars need a measured baseline before they are written.** A ≤ 3 % overhead
   criterion authored with no dispatcher to measure became a 6-7× miss that had to be accepted.
   Write the bar after the first measurement, or write it as "measure and record".
3. **Inventory before fixing, and reconcile the inventory both ways.** The 5.5× undercount would
   have become five phases of "done" against a wrong denominator.
4. **Insert a phase instead of stretching one.** Every inserted phase closed with its own
   verification; the scope that was folded into a running phase's tail (Phase 22.1's BUG-04
   promotion, Phase 37's D-16 diagnosis) is where the timeline blurred.
5. **Extending a "complete" milestone is cheap only if the record moves with it.** The ROADMAP
   milestone row lagged the phase list twice; the close had to reconstruct 22-37.1 from three
   different "in progress" sentences.

### Cost Observations

- Sessions: not tracked per-milestone
- Notable: 1,678 commits over 23 days for 231 plans — 7× v0.9.0's commit count and the largest
  milestone by every measure; four re-verification passes (v0.9.0 had none); 16 code reviews and
  17 security passes recorded; three inserted phases; two release runs and three publish attempts
  to reach 12/12 crates. The documentation programme (Phases 34-36.1, 46 plans) was the price of
  shipping twelve phases of runtime work with the mdBook and rustdoc trailing behind.

## Milestone: v0.11.0 — Treasurer Spend Governance

**Shipped:** 2026-10-27 (closed; `v0.11.0` tagged and published 2026-10-26)
**Phases:** 15 (38-51, one inserted, four added mid-milestone) | **Plans:** 174 | **Commits:** 1,126 on the release PR per GitHub (737 reachable in the shallow sandbox clone)

### What Was Built

The spend governor the token economy had been reserving a word for. An output-side Treasurer:
exact i64 nano-unit pricing from an operator-only price table through a `PricingLlmAdapter`
decorator, a durable `TreasuryLedgerPort` (in-memory / SQLite / Postgres under one contract suite)
settled once per superstep attempt and once per priced agent call, attributed to the tenant and API
key that caused it; rolling per-tenant and per-API-key allowances refused at admission with
`429 allowance_exhausted` on every spend-starting route, one durable deduped warn notice per window
delivered as trace event, herald line and signed webhook; a resumable mid-run halt at the superstep
boundary with a typed `halt_reason` and fork-as-resume; and Cadence — per-(provider, model) 429
pacing that honours `Retry-After`, shares state through Redis Lua, degrades through an outage and
coalesces cache stampedes with fenced locks. Around it: the legacy Battalion error/retry/timeout
surfaces and `LlmError(String)` removed for Aegis-only fault handling (ADR-0059); `Node<T>` →
`Entity<T>` with read-aliased persisted JSON (ADR-0060); nine configuration-swappable LLM
providers over a dated per-model profile registry seeded only from scrubbed live probes
(ADR-0061); exact-or-refuse reasoning effort with `LlmPort::check_reasoning` (ADR-0062); the one
downstream consumer's surface pinned by a compile-time muster and nine harmonised `*Config::new`
shapes; RustFS in place of the terminal MinIO pin (ADR-0055); `Runnable::Agent` SSE and webhooks;
MIGRATION section 10 and a Treasurer book page; and a one-tag release of all twelve crates with
both release blockers reproduced offline first.

### What Worked

- **Design decisions recorded before code, as their own plan.** Phase 38's first plan wrote
  ADR-0052 (attachment point) and ADR-0053 (ledger model) with nothing to implement yet; Phases
  41, 42, 43 and 48 each opened or closed with an ADR plan. Every one of the eleven ADRs reads
  `conforms` at close, with per-decision proving tests cited — the v0.10.0 pattern, now the
  default shape of a phase.
- **One contract suite per port, again, and it kept paying.** `TreasuryLedgerPort` (three
  adapters), the eight-clause `balance` contract, the `CadencePort` edge-rule contract run at both
  jitter ends, the 11-case `FileStoragePort` suite with a compiled-in count so it cannot pass
  vacuously, and the 17-case × 9-fixture provider conformance suite (153 generated tests). Adding
  Postgres `balance`, Anthropic to the conformance suite, or a ninth provider was a `use`, not a
  re-derivation.
- **Measured, dated seeds instead of remembered defaults.** Every per-model profile and every
  effort level the adapters ship traces to a scrubbed live probe transcript committed in the phase
  (2026-10-15, 2026-10-20, 2026-10-22; 718 effort rows), with drift tests comparing the docs page
  to the registry cell by cell. The three Phase 50 UAT defects (stale Kimi fallback, `max_tokens`
  on `gpt-5.4-nano`, discarded vendor totals) were all cases where a remembered default had
  outlived the vendor.
- **Red first, on the consumer's behalf.** The consumer-surface muster was proven red by scripted,
  self-restoring mutations before it was trusted; Phase 50.1's six gaps were found by the consumer's
  own pre-measured bump (112 errors across 3 crates) and closed before the tag, so web3sec moves
  once.
- **Both release blockers reproduced offline before the build wave.** The 156,528-byte release
  body that the real payload builder rejects and the `cargo deny` red on RUSTSEC-2026-0386 were
  reproduced in Phase 47's first plan, fixed with a byte-identity ledger and a lockfile pin, and the
  tag published 12/12 on the first run — the first release since 0.9.0 that needed no patch.
- **Overrides written down with their accepter and date.** ALLOW-03's checkpoint wording
  (operator, 2026-10-07), OBS-05's overhead figure (maintainer, 2026-09-30), REASON-06's two
  waivers (owner, 2026-10-23), ADOPT-01…03's supersession (owner, 2026-10-14): each is an
  amend-at-source note beside the requirement, so the archive reads true without a re-audit.
- **A validation sweep before the close, not after.** The audit's nine unreconciled
  `VALIDATION.md` files were closed by `/gsd-validate-phase` the same day (15/15 compliant, 0 gaps
  across 251 mapped tasks), which turned a `tech_debt` audit with a Nyquist rider into a plain one.

### What Was Inefficient

- **The milestone was extended four times after its roadmap was written**, each time from a
  downstream handoff: Phase 48 (code survey, 2026-10-12), Phases 49-51 (three web3sec handoffs,
  2026-10-14) and Phase 50.1 (a fourth, 2026-10-19), all sequenced *before* the already-planned
  Phases 46-47. The ten-phase, 35-requirement roadmap became fifteen phases and 66 requirements;
  Phase 50 alone grew from 18 plans to 32 with a 13-plan gap-closure set. The docs phase (46) that
  had to freeze the migration guide waited five weeks for the surface to stop moving.
- **Provider work found its defects in UAT, not in planning.** Phase 50's live probe (plan 50-17)
  ran *after* the adapters had been rewritten against remembered defaults, so three root-cause
  debug sessions and a 13-plan closure set followed. The probe should have been the first plan.
- **Sandbox disk exhaustion was a recurring tax.** Six Phase 50 summaries and one Phase 50.1
  summary record `No space left on device` during workspace builds, each recovered by deleting
  `target/debug/incremental`; the full workspace test could not be completed locally in several
  plans and was CI-attributed through `WINDOWS.md` unrun-verify rows (68, 69, 73).
- **The pre-release acceptance bar was still missed, and still accepted.** Tracing overhead was
  re-measured at +19.36 % / +16.19 % against the ≤ 3 % PRD 07 bar after the Phase 45 fixes and
  waived (row 61) — the second milestone running in which the bar was accepted rather than met or
  rewritten.
- **`WINDOWS.md` was released around, not through.** Phase 47 shipped with `open_count: 13` by
  recording the rows as "not its own" and skipping `/gsd-ship`; the register's gate was designed
  to block exactly this, and the rows now carry into the next milestone with owners but no closer.
- **Release-process findings carried for a third time.** The cargo-semver-checks baseline still
  pinned to 0.9.0 (a no-op for every checked crate since 2026-09-26), `check-migration-allowlist`
  not in CI, the release-body builder's hard cap, and `cargo test --workspace --all-features` in CI
  were all listed as carried findings in Phase 47, as they were in 37/37.1.
- **Debug sessions scoped to root cause only never closed.** Three `debug/` files stay
  `diagnosed` although their fixes landed in plans 50-19..50-23; the artifact audit counted them as
  open at close and they had to be acknowledged by hand.

### Patterns Established

- **A phase opens with the ADR plan and closes with the `conforms` plan.** Design decisions land
  as a numbered ADR before any code (Phase 38 plan 01, Phase 41 plan 09, Phase 42 plan 01, Phase
  43 plan 13, Phase 48 plan 02); the sealing plan re-reads the ADR against the tree and marks it
  `conforms` with per-decision evidence.
- **One decorator per concern, composed in a fixed order.** `Pricing(Cadence(provider))`, with
  `FallbackLlmAdapter` wrapping each hop; every decorator forwards `model_profile` and
  `check_reasoning` so pre-judging gates see the adapter's own verdict.
- **Fail-closed admission with a named unlocking key.** A refused parameter or effort names the
  exact YAML key that would unlock it (`llm.<provider>.models.<id>.effort.budget`,
  `paladin.params_omitted`); an auto-selected value is omitted visibly, never silently sent.
- **Seed only from a committed, dated, scrubbed probe; pin the docs to the registry with a drift
  test.** No model row without a transcript line; no docs table that can disagree with the code.
- **Clean breaks under a phase-scoped X-03 supersession, each measured.** ADR-0059 (Phase 44) and
  ADR-0060 (Phase 48) follow ADR-0051's shape: every break measured with cargo-semver-checks against
  the *published* baseline, one `MIGRATION.md` row and one allowlist entry per break, set-equality
  gated.
- **The consumer's surface is a compile-time test in the upstream repo.** `consumer_surface_muster`
  turns "will this break web3sec?" into a red build here, before a tag — and the consumer's own
  pre-measured bump is filed verbatim as a handoff document in the phase.
- **Release blockers are reproduced offline in the phase's first plan.** The payload-size and
  advisory failures were made to fail locally before anything was bumped, inverting v0.10.0's
  discover-at-tag sequence.
- **Owner decisions at checkpoints are quoted verbatim in the SUMMARY** (`option-a but we also
  want to record fixing the signature drift as a todo`), so a later reader sees the words, not a
  paraphrase.

### Key Lessons

1. **Probe the vendor before rewriting the adapter.** Three of Phase 50's four UAT defects were
   remembered defaults the vendor had since changed; the live probe that would have caught them ran
   as plan 17 of 18. For any work against an external API, the dated measurement is the first
   plan, and the seed rule ("no row without a transcript line") applies from the start.
2. **Freeze the roadmap or freeze the guide — not both open at once.** Four mid-milestone
   extensions, all sequenced before the docs and release phases, cost the milestone five weeks of
   a frozen migration guide that could not be frozen. Either admit consumer-driven phases as a
   separate milestone, or hold the docs phase open as a living document and freeze it once.
3. **A gate you release around is not a gate.** `WINDOWS.md`'s `open_count` was designed to block
   `/gsd-ship`; shipping without `/gsd-ship` and recording the rows as "not mine" moved 13 rows
   into the next milestone with no closer. Either disposition at release or change the rule.
4. **A root-cause-only investigation needs a closing step.** Three debug sessions whose fixes
   shipped stayed `diagnosed` and surfaced as open at close. When the fix lands, the fixing plan
   moves the session to `resolved/` — it is the one place that knows.
5. **Reproduce the release blocker offline, then build.** Both v0.11.0 blockers were found by
   measuring the candidate end to end before the version bump, and the tag published 12/12 first
   time. This is the mirror of v0.10.0's lesson 1 and should stay the fixed shape of a release
   phase.
6. **Accepting a bar twice is deciding it.** OBS-05's ≤ 3 % overhead bar has now been accepted
   rather than met at two closes. Rewrite the bar to the measured regime (or fund the dispatcher
   work) rather than carry a third waiver.

### Cost Observations

- Model mix: not tracked per-milestone
- Sessions: not tracked per-milestone
- Notable: 1,126 commits over 34 days for 174 plans — two-thirds of v0.10.0's commit count on
  three-quarters of its plans, with 15 phases (five added after the roadmap), 13 code reviews, 11
  security passes and 13 UAT records; eleven ADRs (v0.10.0: 13 across 19 phases). Phase 50 (32
  plans, 158 reachable commits) was the largest phase of the milestone and the one with the most
  rework; Phase 47's release landed on one tag with no patch — the first since 0.9.0. The four
  consumer-driven phases (49, 50, 50.1, 51: 64 plans) were 37 % of the milestone's plans and
  were not in the roadmap when it was written.

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v0.7.1 | — | 4 | First milestone with protected decisions. The corpus had 0 locked ADRs across twelve prior milestones and eighteen months; this one produced 9. |
| v0.8.0 | — | 14 | First milestone to disqualify a tool by measurement (Snyk probe), and the first to apply live branch protection. Four phases needed a second verification pass. |
| v0.9.0 | — | 4 | First milestone proven by live pipeline rehearsals (five throwaway rc tags, three real release runs); first fully-green release run; standing publish credential eliminated. Zero failed verification passes. |
| v0.10.0 | — | 19 | Largest milestone by every measure (231 plans, 1,678 commits, 23 days). First with three inserted phases and a dedicated deferred-items closure phase before the tag; first real release to stall mid-publish (3/12) and be finished by a patch release through the same pipeline; first milestone extended twice after its phases were declared complete. Four re-verification passes. |
| v0.11.0 | — | 15 | First milestone whose every phase opened or closed with an ADR plan (eleven ADRs, all `conforms`); first to seed adapter behaviour only from committed, dated live probes with drift tests; first release since 0.9.0 to publish 12/12 on one tag, with both blockers reproduced offline first. Extended four times after its roadmap from downstream handoffs (five of fifteen phases); 15/15 Nyquist-compliant at close after a same-day validation sweep. Zero failed verification passes on the first attempt for 13 of 15 phases (Phases 50 and 51 needed gap-closure plans). |

### Cumulative Quality

| Milestone | Tests | Coverage | Zero-Dep Additions |
|-----------|-------|----------|-------------------|
| v0.7.1 | 2,924 passing (+185 doc tests) | 85.92% (floor 84%) | 0 new dependencies |
| v0.8.0 | 428 workspace unit + 247 `paladin-llm` crate-scoped; 96/96 doctests | 82.39% (floor 82, ADR-0006) | 6 new LLM providers, no new heavyweight deps |
| v0.9.0 | +294 shell-harness assertions (177 Phase 20 + 117 Phase 21) over the release tooling | unchanged (floor 82, ADR-0006 — no first-party `.rs` changes required it) | 0 new runtime dependencies (workflow + script work) |
| v0.10.0 | 3,708 workspace tests passing (Phase 37.1 sweep, 0 failed); 462 doctests at Phase 34; 101/101 public-API entry points with `# Examples`; `cargo doc` 0 warnings under both ADR-0033 bars | 90.44% (CI, PR #56; floor 82, ADR-0006) | 1 new publishable crate (`paladin-eval`); one new lateral crate edge (`paladin-memory` → `paladin-llm`); external additions (OTel export, Redis cache/queue) feature-gated and recorded in `MIGRATION.md` §9.3 |
| v0.11.0 | Workspace totals CI-attributed (the sandbox could not complete `cargo test --workspace` in several plans, WINDOWS rows 68/69/73); 153 generated provider-conformance tests (17 × 9) plus a 24-case effort suite; every ADR `conforms` with cited tests | 92.74% (CI, merge commit `70657919`; floor 82, ADR-0006) | 0 new publishable crates; new runtime deps feature-gated (Redis Cadence behind the existing `redis` feature); three `zip` RustSec advisories via `utoipa-swagger-ui` carried as dated exceptions, not fixes |

### Top Lessons (Verified Across Milestones)

1. **Record the decision before writing the code, and gate the record on a parser.** Held across
   both milestones — v0.7.1 produced 9 ADRs where twelve prior milestones produced 0; v0.8.0 added
   38 more and wired two mechanical guards that enforce what earlier prose only asserted.
2. **A gate that cannot fail is worse than no gate.** New in v0.8.0, and the milestone's most
   transferable finding: the duplicate audit job, the unsatisfiable Snyk mandate, and the
   path-filter trap all report success without doing work.
3. **Verify against the current tree, not against the last report about it.** All three milestones
   produced findings that were accurate when written and stale when read — v0.9.0's audit itself
   initially carried three human checks as open that recorded UAT had already closed.
4. **Rehearse the pipeline; do not re-read it.** New in v0.9.0: the binary-attachment defect
   survived every prior *reading* of `release.yml` and fell to the first *run* that asserted its
   outputs. Live rehearsal on throwaway tags is now the proof standard for pipeline work.
5. **A gate is only as real as the environment it runs in.** New in v0.10.0, and the direct heir
   of lessons 2 and 4: `make publish-dry-run` was green on the exact tree that failed live because
   it resolved crates from the workspace overlay, not the registry. Every gate now gets asked what
   it *cannot* see — and the answer for publish order became an offline `cargo metadata` gate that
   fails on the broken tree.
6. **Measure the external system before you build against it.** New in v0.11.0: three of four
   Phase 50 UAT defects were vendor behaviour that had changed since the last remembered default,
   and the probe that caught them ran second to last. The dated, committed measurement is the
   first plan of any adapter work — the same rule that made the publish-order gate real in
   v0.10.0, applied to the vendor instead of the registry.

---

*Next milestone: not yet defined — run `/gsd-new-milestone`; new phases start at Phase 52.*
