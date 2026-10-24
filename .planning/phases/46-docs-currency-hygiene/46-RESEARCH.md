# Phase 46: Docs Currency & Hygiene - Research

**Researched:** 2026-10-24
**Domain:** Documentation currency, migration-register restructuring, ledger/bookkeeping, one serde load-time guard and one offline CI guard in a Rust workspace (Cargo, mdBook, bash/awk/python gates, GSD ledger tooling)
**Confidence:** HIGH. Every finding below was checked against the live tree at `ac3a52e0` (HEAD; `e28e43b5` plus planning-only commits), against a command run in this session, or against a throw-away prototype. Items that were not run are tagged `[ASSUMED]` and listed in the Assumptions Log.

Provenance tags used here: `[VERIFIED: <how>]` (command, grep, prototype run this session), `[CITED: <file:line>]` (read in the live tree), `[ASSUMED]` (not confirmed this session).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

### Carried forward (locked by earlier phases — cited, not re-asked)

- **D-00a:** Shipped tree outranks any document (Phase 34 D-00g and successors). Gates on every
  commit: `make clean-code`, `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry
  for an intentional public-surface change), `make security`, `make check-api-examples`,
  `make check-migration-allowlist`, `make check-doc-config`, `make check-gates`, 82 % workspace
  line-coverage floor (ADR-0006), MSRV 1.88, and the manual credential-handling review in
  `.github/instructions/security.instructions.md`.
- **D-00b:** X-03 governs public API: every break already has its `MIGRATION.md` §9.2 row and,
  when marked `Y`, a `.cargo/semver-checks-allowlist.toml` entry, written in the same commit as the
  code by Phases 44, 48, 50, 50.1 and 51. **Phase 46 indexes, frames and corrects those rows; it
  does not re-register them.** The one new code change in this phase (D-23) is additive at the
  Rust-signature level and needs no §10.2 row; its behavioural change gets a §10.1 row.
- **D-00c:** Version identity is **v0.11.0** everywhere (CHANGELOG `[Unreleased]`, MIGRATION §10,
  WINDOWS rows). ADRs take the next free number from `.planning/decisions/PROMOTION.md`
  (currently **0063**) and advance that line in the same commit; this phase expects to write **no
  new ADR** — ADR-0059 is amended with a dated note (D-23) and ADR-0015 is cited as-is (D-21).
- **D-00d:** Vocabulary: Medieval-military words for roles, plain words for units and identifiers
  (Phase 30 D-01). Treasurer, Commissary and Cadence are the three officers of the token economy;
  no new officer is minted. "Node" means only a WarGraph/Campaign vertex (ADR-0060).
- **D-00e:** Phase 48 (ADR-0060), Phase 50 (ADR-0061) and Phase 51 (ADR-0062) clean-break policy:
  no deprecated aliases; `MIGRATION.md` §9.7 Deprecations is empty and stays empty.
- **D-00f:** The consumer-surface muster `tests/consumer_surface_muster.rs` (`make
  check-consumer-surface`, Phase 49, extended by 50, 50.1, 51) is kept green on every commit and is
  extended in the same commit as any change to the surface it pins.

### Migration guide shape and freeze (CURR-23)

- **D-01:** `MIGRATION.md` gains a **new top-level part §10 "Upgrading from v0.10.1 to v0.11.0"
  above the existing v0.10 record**, which is kept intact as §9. The file title becomes
  release-neutral ("Upgrading Paladin") with a scope note listing the two parts. §10 mirrors §9's
  sub-structure one-for-one: 10.1 Behavioral changes, 10.2 Rust API changes (the X-10 register),
  10.3 Toolchain & dependencies, 10.4 Persistence & schema migrations, 10.5 Configuration &
  environment, 10.6 HTTP API, 10.7 Deprecations (empty, D-00e), 10.8 Upgrade checklist with the
  "Library consumers" subsection. — **Reversibility:** costly — every later phase appends to §10
  and the CHANGELOG, ADRs and consumer muster cite its section numbers.
- **D-02:** **The v0.11 rows already interleaved in §9.1 / §9.2 / §9.4 / §9.5 / §9.6 / §9.8 move
  into the matching §10.x section.** Row identity is preserved exactly: `M-B-xx` ids, `DR-xx` ids
  and the `crate | type` keys do not change, so `.cargo/semver-checks-allowlist.toml`'s
  `migration_row` keys and `scripts/check-migration-allowlist.sh`'s set-equality keep matching
  with no allowlist edit. Each §9.x section whose rows moved gets a one-line pointer at its top
  ("v0.11 rows that were appended here now live in §10.x"). Citations of "§9.2" in sealed phase
  documents, ADR-0059/0060/0061/0062 and earlier CHANGELOG sections are **left as written**
  (historical). The §9.8 "Library consumers: v0.10.1 → v0.11.0" subsection moves whole into §10.8.
  The planner verifies `make check-migration-allowlist` is green before and after the move.
- **D-03:** **The guide freezes in Phase 46 on the 2026-10-24 web3sec dogfood report**
  (`46-FEEDBACK-web3sec.md`). Its seven guide corrections G-1..G-7 are folded into §10 as
  written there: G-1 facade path on DR-3; G-2 the `message`-field recipe for keeping v0.10
  operator text on M-B-08; G-3 both Phalanx-timeout outcomes (`Err(AggregationError)` with
  `on_error: None`, `Ok` + `node_errors` `source: Timeout(Run)` with `Absorb`) on DR-9 and M-B-06;
  G-4 drop or soften the "two models web3sec runs" parenthetical and add the
  `deepseek-flash`-listed / `deepseek-v4-flash`-unlisted note on the Switching-providers page;
  G-5 full `paladin::` import paths and the pacing-only `with_cadence` in the Cadence recipe;
  G-6 the decorator bullet names both `model_profile` and `check_reasoning` with "must"; G-7 one
  explicit sentence in the pre-v0.10.0 block that DeepSeek previously made up to 4 and Anthropic
  up to 3 calls with back-off on `RateLimitExceeded` and now make 0 retries without Cadence.
  The ROADMAP "Depends on" clause about web3sec's adaptation feedback is recorded as **satisfied
  by this handoff**; Phase 47 re-reads §10 only if a code change after `e28e43b5` moves the
  consumer surface (handoff §5).
- **D-04:** The handoff's four **muster gaps** (§3) are closed in this phase: pin
  `Stockpile::render()`, `BattalionResult.{paladin_success_count, paladin_failure_count,
  per_paladin_times}`, a `paladin::prelude::*` glob import site, and `ModelProfile` through the
  `paladin_ports::output::llm_port::ModelProfile` re-export. Test-only, no public-surface change.
- **D-05:** The mdBook exposes the guide by **rewriting `docs/src/api-reference/upgrading.md` to
  lead with v0.10.1 → v0.11.0** — a condensed §10.1 table, the library-consumer index (DR rows by
  topic), and the operator checklist mirrored from §10.8 — and keeping the existing v0.10
  orientation as a lower section. `docs/src/api-reference/migration-guide.md` gains one pointer
  line. No new `SUMMARY.md` entry for the guide. Both pages link the root `MIGRATION.md` by
  section anchor.
- **D-06:** web3sec's optional **one-time `warn!` on an uncomposed 429** (handoff §4) is
  **deferred**; the G-7 sentence is the v0.11.0 answer. Recorded under Deferred Ideas.

### Treasurer page and configuration reference (CURR-23)

- **D-07:** The Treasurer page is **`docs/src/architecture/treasurer.md`, beside
  `architecture/commissary.md`**, added to `SUMMARY.md` under Architecture. It is the concept page
  for the spend-governance officer: the append-only ledger derived on read (ADR-0053), allowance
  admission in tumbling UTC windows (ADR-0056), mid-run enforcement and the halt contract
  (ADR-0052, ADR-0057), pricing as the cost source, the reservation of the name (ADR-0050), and the
  Rust surface a consumer meets (`Treasurer`, `AllowancePolicy`, `ScopeAllowance`, `Ceiling`,
  `TreasurerSpendGuard`, `OperatorNoticeTarget`, `window_for` in
  `src/application/services/treasurer/`; the ports `AllowanceAdmissionPort`, `TreasuryNoticePort`).
  It follows the Commissary page's shape (concept, responsibilities, type table, worked example,
  links to ADRs and the config reference). The `429 allowance_exhausted` HTTP contract is linked
  to `api-reference/platform-api.md`, not duplicated.
- **D-08:** **Cadence is one section on the Treasurer page** ("Rate pacing (Cadence)"), pointing at
  ADR-0058 and the Cadence table already in `configuration.md`. No separate Cadence page.
- **D-09:** The configuration reference is **one `## Treasurer` block in
  `docs/src/getting-started/configuration.md`**: the existing "Treasurer allowances" and "Treasurer
  rate pacing (Cadence)" sections are regrouped under it, and a new "Pricing" section is added
  (per-model row: `prompt`, `completion`, `cache_read`, `cache_write`, `reasoning`; `currency`; what
  a missing row means at admission and in the ledger). A shared key table (key, type, default,
  env override) covers the whole `treasurer:` subtree. The Treasurer page links this block as its
  reference.
- **D-10:** **Source of truth is the Rust config**: `src/config/treasurer.rs` (`TreasurerConfig`,
  `PriceRowConfig`, `AllowanceConfig`, `AllowanceEntryConfig`, `AllowanceWebhookConfig`,
  `CadenceConfig`, `CadenceBackend`). Every public key appears in the table with its `validate()`
  rule and its `APP_TREASURER_*` override (or "none"); `config.example.yml`'s `treasurer:` block is
  brought into line in the same plan; `make check-doc-config` keeps every fenced YAML snippet
  parsing.

### Audit docs fixes (CURR-22)

- **D-11:** "**twelve publishable crates**" replaces every **present-tense** "eleven" in
  `docs/src/contributing/development-setup.md`, `docs/src/appendix/release-automation.md` and
  `docs/src/appendix/release-recovery.md` (publish order, lockstep versioning, recovery
  enumeration, the Trusted Publishing table's "All eleven crates are covered" line). **Historical**
  statements stay as written — the Credential History rows for `0.8.1-rc.1`/`rc.2` were eleven-crate
  events — with a parenthetical where a reader could misread them as current.
- **D-12:** `paladin-eval` gets a **Trusted Publishing table row** (`paladin-eval`,
  `crates/paladin-eval`, `release.yml`, `crates-io`, link date "on or before 2026-09-21", status
  "linked (proven by publish)") and a **Credential History row** for its first OIDC publish: the
  `v0.10.1` release run `35659477719` on 2026-09-21. The link date is proven by the publish, not
  reported by the operator, and the row says so — the table's own evidence standard.
- **D-13:** The CHANGELOG `[0.10.0]` heading date is corrected from `2026-09-10` to the tag's
  commit date, **`2026-09-18`** (`git log -1 v0.10.0`). The planner verifies the `[0.10.1]` heading
  (`2026-09-20`) against `git log -1 v0.10.1` (2026-09-21) in the same task and corrects it if the
  release-run date does not justify the difference; no other heading moves.

### Nyquist bookkeeping (CURR-24)

- **D-14:** The seven `draft` files (Phases 22, 24, 29, 30, 34, 36, 36.1) are **reconciled against
  the tests that already exist and are green**, via the Nyquist auditor: the per-task map is
  filled from each phase's archived PLAN/SUMMARY/VERIFICATION, `status: validated` is set,
  `nyquist_compliant` is set to whatever the evidence supports, and every uncovered task is listed
  under an explicit Gaps section. **No new test is written in Phase 46** to close a gap.
- **D-15:** Phase 28 is **promoted to `nyquist_compliant: true`**: its two Manual-Only rows cite
  `28-UAT.md` 2/2 passed, the D-37 acceptance and the Phase 45 re-measurement (WINDOWS row 61);
  the accepted overhead deviation stays recorded where it is (WINDOWS rows 35/61, CHANGELOG Known
  limitations) and is not re-litigated.
- **D-16:** The files are **edited in place under `.planning/milestones/v0.10.0-phases/`** — GSD's
  phase resolver already resolves archived phases (`init.phase-op 28` returns the archived dir) —
  each with a dated reconciliation note in its frontmatter. No copies in the Phase 46 directory.
- **D-17:** Mechanism: **eight `/gsd-validate-phase N` runs** (22, 24, 28, 29, 30, 34, 36, 36.1)
  driven as Phase 46 plan tasks, each recording its resulting `status` / `nyquist_compliant` line
  in the plan SUMMARY; ROADMAP success criterion 3 is evidenced by those eight frontmatter lines.
  The STATE.md deferred-items row for these phases is rewritten to the outcome (the open item for
  archived Phases 05–21 is **not** in scope and stays open as written).

### v2 debt lines (CURR-25)

- **D-18:** **Oversized service files — waived**, with a measured re-statement: the waiver
  re-measures today's largest files (`crates/paladin-battalion/src/engine/superstep.rs` ≈ 16k
  lines, `engine/mod.rs` ≈ 14k, `src/application/services/paladin/paladin_execution_service.rs`
  ≈ 10.7k, raw `wc -l` including in-file tests), states that decomposition is a multi-phase
  refactor with no requirement behind it and no consumer-visible effect, and defers it as a
  candidate phase for the next milestone's roadmap. No code moves.
- **D-19:** **Clone/lock contention — waived**: the benchmark blocker is gone
  (`benches/engine_benchmarks.rs`, the Phase 28/45 `bench_superstep_cost_sink_variants`
  measurement) and no bench shows clone or lock contention as a bottleneck; the waiver names those
  benches and the re-open condition (a bench regression attributed to cloning or lock wait).
- **D-20:** **Dependency-allowlist drift — recorded as fixed by ADR-0015** (rebaselined to the
  measured fourteen / eleven dependencies with the purity invariant: no provider SDK, transport
  client, storage driver or web framework in `paladin-core` or `paladin-ports`).
- **D-21:** D-20 gains a **mechanical guard**: a script (`scripts/check-dependency-allowlist.sh`,
  or an equivalent `cargo test` in the root crate) that reads `crates/paladin-core/Cargo.toml` and
  `crates/paladin-ports/Cargo.toml` `[dependencies]` and fails when either gains a crate not
  listed in ADR-0015's measured set, wired into `make check-gates`. If the count has moved since
  ADR-0015 (Phases 38–51 may have added a dependency), the ADR is amended with a dated note naming
  the new entry and its justification **before** the guard is pinned to it — the guard never
  silently blesses drift.
- **D-22:** **Every disposition is a `WINDOWS.md` row** written through the ledger tool
  (`gsd-tools windows ...`): two `waived` rows with the D-18/D-19 reason text and one `fixed` row
  for D-20/D-21. The three "Deferred to v2" rows in STATE.md's deferred-items table are rewritten
  to point at those row ids. No new ADR for the dispositions.

### Folded todos

- **D-23:** **Reject removed v0.10 config keys — option B, in Phase 46**
  (`.planning/todos/pending/2026-10-11-reject-removed-v0-10-battalion-config-keys.md`). A stored
  v0.10 document that still carries `timeout_seconds`, `retry_policy` or `error_strategy` on
  `BattalionConfig`, or `timeout_seconds` on `ConclaveConfig`, fails to deserialize with an error
  naming the removed key and its `aegis.*` replacement and citing the §10.1 row. Constraints from
  the todo hold: reject **only** the named removed keys (never `#[serde(deny_unknown_fields)]`,
  so a v0.11 document with an unrelated unknown key still loads); the CLI's own YAML
  `timeout_seconds` key (a separate, still-supported schema mapping onto `aegis.timeout`) keeps
  working; ADR-0059 is amended with a dated note (it recorded "the removed keys are ignored");
  the §10.1 `M-B-06`/`BattalionConfig` rows, the §10.2 `BattalionConfig` / `ConclaveConfig` rows
  and the CHANGELOG are updated in the same commit; `tests/legacy_removal_guard.rs` and `make
  check-gates` are kept green (the new error strings name removed items, so the guard's
  allowed-context treatment applies). — **Reversibility:** costly — a load-time refusal is
  operator-visible behaviour on a published type; relaxing it later is another §10.1 row.
- **D-24:** **Phase 44 code-review follow-ups**
  (`.planning/todos/pending/2026-10-11-phase-44-review-follow-ups.md`): WR-04 — rustdoc on
  `BattalionConfig.aegis` and the Commander names Maneuver, states that the Commander no longer
  bounds ChainOfCommand or Grove, that `default_maneuver_config` turns the per-attempt
  `run_timeout` into a whole-flow clock, and Maneuver gains the same one-time `warn!` Campaign has
  when `on_error: Absorb` or `retry` is ignored; WR-05 — a dated "remove after 0.11.0 is the
  published baseline" note on the crate-wide semver `allow` lines in `crates/paladin-core/Cargo.toml`
  and `crates/paladin-battalion/Cargo.toml`. The IN-01..IN-05 and IN-N1/IN-N2 info items are
  **optional**: the planner takes the rustdoc ones (IN-05 circuit-breaker Permanent semantics,
  IN-N1 in-flight-attempt-keeps-its-permit) and leaves the test-tightening ones unless cheap.
- **D-25:** **web3sec single bump**
  (`.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md`): folded as an **input**,
  not as work. Its "report divergences before Phase 46 freezes the guide" step is satisfied by
  `46-FEEDBACK-web3sec.md` (D-03). The todo itself stays `pending` — its remaining steps (the
  consumer's re-pin on the `v0.11.0` tag, its gates, its return report) belong to web3sec after
  Phase 47 — and gets a dated update pointing at this phase.

### Claude's Discretion

- **§10 prose and ordering** beyond D-01's sub-structure: how the §10.8 consumer index groups DR
  rows by topic, the wording of each pointer note, and whether §10 opens with a one-paragraph
  "what v0.11.0 is" summary drawn from PROJECT.md.
- **Treasurer page worked example**: which scenario it walks (a ceiling reached at admission, a
  mid-run halt, a ledger read) — one is enough; it must compile as a doc-example or be clearly
  marked illustrative.
- **The guard's form** (shell script vs cargo test) in D-21, and where the ADR-0015 set is read
  from (a literal list in the guard, mirrored in the ADR, is acceptable).
- **Plan slicing**: the natural waves are (1) audit fixes + CHANGELOG date, (2) Nyquist runs,
  (3) debt-line rows + guard, (4) Treasurer page + config reference, (5) MIGRATION §10 restructure
  + row moves + G-1..G-7 + muster pins, (6) removed-keys guard + Phase 44 follow-ups, (7) book
  pages + ROADMAP/STATE bookkeeping. The planner may merge or reorder; the §10 freeze is the last
  docs commit.
- **50.1 and 51 outstanding human items**: 50.1's "disposition of WR-01..03" was already closed by
  the `50.1-REVIEW-FIX.md` commits (`9d0eaa3d`, `ec3fdc92`, `5ebd315b`, plus the §9.8 bullet
  `aa48314d`); the planner only confirms the §10.8 move carries that bullet. Phase 51's three human
  items (CI coverage, Anthropic `Budget` override-only, Gemini 2.5 waiver) are Phase 51 UAT
  matters, not Phase 46 scope; §10 states the shipped behaviour as Phase 51 already wrote it.

### Deferred Ideas (OUT OF SCOPE)

- **One-time `warn!` when an adapter returns `RateLimitExceeded` with no Cadence layer composed**
  (web3sec handoff §4) — next milestone; the G-7 sentence covers v0.11.0.
- **Decomposition of the oversized engine/service files** — candidate phase for the next
  milestone's roadmap (D-18 waiver names the files and today's sizes).
- **Nyquist reconciliation for archived Phases 05–21** — the same open STATE.md item; not part of
  CURR-24.
- **Optional Phase 44 info items** IN-01..IN-04 and IN-N2 (test tightening) if the planner skips
  them (D-24).

### Reviewed Todos (not folded)
- `2026-08-13-verify-local-coverage-reproduction.md` — needs a Docker-capable machine the
  maintainer walks by hand; dispositioned deferred by Phase 36.1; keyword match only.
- `2026-10-13-review-content-data-orchestration-base-items.md` — a design review of the unfinished
  content / general-data orchestration system; owner decided it is unscheduled until the milestone
  after v0.11.0 is roadmapped.
- `2026-10-20-openai-responses-api-for-pro-tier.md` — a new provider capability (OpenAI Responses
  API); its own phase, not docs currency.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CURR-22 | v0.10.0 acceptance-audit docs fixes closed: "twelve publishable crates" in the three docs, `paladin-eval` Trusted Publishing and Credential History rows, CHANGELOG `[0.10.0]` heading date | Exact edit list in "CURR-22 edit list"; all dates and the publish proof verified (git, `gh`, crates.io API); four extra stale count words found beyond D-11's grep |
| CURR-23 | Treasurer mdBook page, Treasurer configuration reference (pricing, `allowance`, warn threshold), v0.10 to v0.11 `MIGRATION.md` guide published and reachable from the book | Full verified key table; `treasurer.md` shape from `commissary.md`; the §9 to §10 move procedure with the three gate files that must change with it; G-1..G-7 locations re-verified; muster pins verified to exist |
| CURR-24 | Nyquist validation for 22, 24, 29, 30, 34, 36, 36.1 out of `draft`; Phase 28 compliant or gaps recorded | State of all eight files, the resolver verified, the interactive and test-writing steps of `validate-phase` that conflict with D-14, and a reconcile-only procedure |
| CURR-25 | Three v2 debt lines each fixed or waived with a written reason | Measured line counts, bench evidence and its limits, ADR-0015 drift finding (`paladin-ports` now has twelve dependencies, not eleven), `gsd-tools windows` real API, guard design and CI wiring |
</phase_requirements>

## Summary

Phase 46 is a documentation and bookkeeping phase with exactly two code changes (D-23 removed-key rejection and D-24 rustdoc plus one `warn!`) and one new offline guard (D-21). Almost everything CONTEXT.md decided is sound and implementable as written. The research found twelve places where the live tree differs from, or adds a precondition to, what CONTEXT assumed; the planner must carry these into the plans because several are gate-breakers that would only surface in CI (see "Findings that change the plan").

The three that matter most. First, D-02's promise that the allowlist set-equality "keeps matching with no allowlist edit" is true of the allowlist file but false of the gate: the register is extracted by an `awk` window hard-coded to the headings `## 9.2 ` and `## 9.3 `, in three places (`scripts/check-migration-allowlist.sh`, `.github/workflows/ci.yml`, and the script's own regression harness). Moving rows under `## 10.2` turns `make check-migration-allowlist` red (proved by simulation) until the window is widened to `(9|10)`. Second, D-23 cannot be done with per-key private fields on the public structs without breaking struct-literal construction, and `tests/legacy_removal_guard.rs` rule 6 forbids the removed key names as whole words in the production code of the two files D-23 names; the verified design is `#[serde(try_from = "<private wire struct>")]` with the wire type in a new sibling module. Third, ADR-0015 has drifted by one: `paladin-ports` now declares twelve dependencies (it gained `log`), so D-21's "amend the ADR before pinning the guard" branch is live, not hypothetical.

**Primary recommendation:** Execute in this order: audit fixes, Nyquist reconciliation, debt rows plus ADR-0015 note plus guard, D-23/D-24 code with the four muster pins, Treasurer page and configuration reference, and last the MIGRATION §10 restructure (which must change the three gate files in the same commit and then retarget the live `§9.x` citations) followed by the book pages and ROADMAP/STATE bookkeeping. No new external package is installed anywhere in this phase.

## Findings That Change the Plan

Read these first. Each row says what CONTEXT assumed, what the live tree shows, and the consequence for planning.

| # | CONTEXT assumed | Live tree shows | Consequence |
|---|---|---|---|
| F1 | D-02: moving rows needs no allowlist edit and `check-migration-allowlist` stays green | The register is read by `awk '/^## 9\.2 /{flag=1; next} /^## 9\.3 /{flag=0} flag'` at `scripts/check-migration-allowlist.sh:91` and `.github/workflows/ci.yml:441`; its regression harness builds fixtures with `## 9.2` / `## 9.3` headers (`tests/scripts/check-migration-allowlist_test.sh:44-56`). `[VERIFIED: renamed the two headings in a scratch copy, script exit 1; widened the awk to (9\|10), exit 0]` | The §10 commit MUST also widen all three to `/^## (9\|10)\.2 /` and `/^## (9\|10)\.3 /`, update the two header comments (`§9.2` wording), and add a harness case that a `Y` row under `## 10.2` is read. The script's own header says "do not improve the parsing without also updating ci.yml", so keep script and ci.yml byte-identical in the awk. Allowlist entries (`migration_row = "crate \| type"`) need no edit. |
| F2 | D-02 lists §9.1/9.2/9.4/9.5/9.6/9.8 as the sections holding v0.11 rows | `git diff -U0 v0.10.1..HEAD -- MIGRATION.md` shows v0.11 additions in **§9.3 as well** (current lines 450, 464, 465: `redis-cadence`, `httpdate`, `rand`), in **§9.7** (lines 946, 948: Phase 48 deprecation sentence) and inside §9.8 step 3 (lines 975-982: the Phase 40 TENANT-01 note) besides the `Library consumers` subsection (1004-1211). `[VERIFIED: git diff ranges]` | The authoritative move-set is the diff against the `v0.10.1` tag, not D-02's list. §10.3 gets the three §9.3 bullets; §10.7 carries the (still-empty) deprecations statement; §10.8 needs a **new** ordered operator checklist (the §9.8 steps are a v0.9 to v0.10 list) that includes the TENANT-01 note. |
| F3 | D-02: rows "interleaved" are movable whole | Four v0.10 rows were edited **in place** with v0.11 text: `PaladinResult` (206), `Settings` (207), `ExecuteResponse` (225), `TraceEvent::NodeFinished/RunFinished` (228); plus two prose lines in §9.7. The pre-edit text is exactly `git show v0.10.1:MIGRATION.md`. `[VERIFIED: 6 deleted lines in the diff]` | For each: restore the v0.10.1 row text in §9.2, and add the v0.11 delta as a new row with the **same `crate \| type` key** in §10.2. The set-equality compares a deduplicated pair set, so duplicate keys across §9.2 and §10.2 are fine (the script's header already documents `paladin-ai \| Settings` appearing twice). Do not leave the combined text in §9.2: it would then describe v0.11 behaviour under a v0.10 heading. |
| F4 | D-02: citations of "§9.2" are historical and left as written | About 39 `§9.x` citations sit in the **`[Unreleased]` section of the root CHANGELOG** (25 `§9.2`, 9 `§9.6`, 2 `§9.5`, 1 `§9.4`, 2 `§9.8`), which is the v0.11.0 release note and not yet historical, plus per-crate `[Unreleased]` entries, `docs/src/user-guides/switching-providers.md:137` (DR-19, "section 9.8"), `crates/paladin-llm/src/registry/seeded.rs` (3 comments, "section 9.8"), `crates/paladin-battalion/src/conclave_execution_service.rs:745` ("9.1 rows") and the muster header (`tests/consumer_surface_muster.rs:23-24`, 542, 554). `[VERIFIED: grep]` | Add a plan task "retarget live citations of moved rows": only citations that point at a **v0.11** row (M-B-05 and later, DR-x, Phase 38-51 rows) are retargeted to §10.x; sealed phase documents, ADRs, `[0.10.x]` and earlier CHANGELOG sections, and citations of v0.10 rows stay. Recipe under "Pitfalls". Use the `git diff` row set to decide which row a citation names. The in-text "section 9.x" references **inside** the moved text (the DR table's Register column, "9.2 `Node` row (Phase 48)", etc.) must be rewritten to 10.x in the same edit. |
| F5 | D-23: guard each removed key; "additive at the Rust-signature level" | `BattalionConfig` and `ConclaveConfig` are all-`pub`-field structs, not `#[non_exhaustive]`; adding a private field per removed key breaks every struct-literal construction in a downstream crate and would need a §10.2 row plus a semver lint. `tests/legacy_removal_guard.rs` rule 6 (lines 230-250) flags the removed names `timeout_seconds`, `retry_policy`, `error_strategy`, `ErrorStrategy` as whole words in any non-comment production line of `battalion/mod.rs` and `battalion/conclave.rs`, even inside a string or serde attribute; rule 5 forbids `timeout(` and `timeout_seconds` in the production code of the five legacy-pattern service files, `commander.rs` among them. | Use `#[serde(try_from = "legacy_keys::BattalionConfigWire")]` (and the same for `ConclaveConfig`) with the wire types and the message text in a **new file** `crates/paladin-core/src/platform/container/battalion/legacy_keys.rs`, which is in neither scoped list. The struct shape, `Serialize` and the public surface do not change. Prototype verified, see Code Examples. The existing test `battalion_config_v0_10_document_deserializes_with_default_aegis` (`battalion/mod.rs:1062-1085`) asserts the opposite of D-23 and must be inverted. |
| F6 | D-22: write `waived` / `fixed` rows with `gsd-tools windows` | The CLI is `windows status`, `append --kind K --phase N [--file F] [--line L] --description D`, `waive <id> "<reason>"`, `fixed <id>`. There is no one-step "write a waived row": append makes an `open` row, then `waive` or `fixed` closes it. Allowed kinds: `stub, todo, fixme, skipped-test, lint-warning, unmet-truth, unrun-verify, deviation`. `fixed` takes no reason text, so the description must carry it. Ids are sequential (the next three are 80, 81, 82) and the writer is read-modify-write (do not run two appends in parallel). `[VERIFIED: handler source + status run]` | Plan each disposition as append then close, strictly sequential, then read `windows status` to confirm counts (waived +2, fixed +1). `/gsd-ship` blocks while `open_count > 0` and 13 rows are already open (56, 58, 62-69, 73, 77, 79); they are not Phase 46's but Phase 47 must know. |
| F7 | D-21: "if the count has moved since ADR-0015" | It has. `paladin-core` `[dependencies]` is the same fourteen names. `paladin-ports` has **twelve**: ADR-0015's eleven plus `log` (`crates/paladin-ports/Cargo.toml`, used by `output/trace_sink_port.rs:180,300` for `log::error!`). `[VERIFIED: manifests read]` | Order matters: (1) amend ADR-0015 with a dated note naming `log` and its justification (a logging facade, not a provider SDK, transport client, storage driver or web framework, so invariant (i) holds), (2) then pin the guard to fourteen / twelve. The git history cannot date the addition (the manifest shows as a new file in `998ae11e`), so state "present by Phase 43; used by the Phase 28 trace sink port" only if confirmed, otherwise say "not in ADR-0015's 2026-08-06 measurement". |
| F8 | D-19: waiver cites benches that show no clone/lock bottleneck | The benches in `benches/engine_benchmarks.rs` (`waypoint_save`, `superstep_cost`, `bench_superstep_cost_sink_variants`) and `config_benchmarks.rs` measure aggregate wall-clock; none attributes time to cloning or lock wait. WINDOWS rows 35 and 61 record an accepted +19.36% / +16.19% tracing overhead whose remaining cost "sits in the dispatcher/sink path", which is exactly where a channel, clone or lock cost would live. | Word the waiver as "no benchmark attributes a regression to cloning or lock wait, and the one standing overhead (WINDOWS 35/61) is in the dispatcher path and is the first place to look", not as "benches prove there is none". The re-open condition is the one CONTEXT already gives. |
| F9 | D-17: eight `/gsd-validate-phase N` runs | The workflow's Step 4 is an interactive `AskUserQuestion` gate and its Step 5 spawns `gsd-nyquist-auditor`, which **writes tests**; D-14 forbids new tests. Seven of the eight files are untouched seeded templates (`| TBD | TBD | TBD |` rows, `wave_0_complete: false`), about 79 plans of source material for the seven plus Phase 28's 45 already-real rows. `[VERIFIED: file reads, plan counts]` | Define the run as **reconcile-only**: executors follow the workflow's Steps 0-3 and 6-7 and edit the file by hand from PLAN/SUMMARY/VERIFICATION, and skip Steps 4-5 (equivalent to the workflow's own "Skip, mark manual-only" option). State this in each plan so an executor does not spawn the auditor. |
| F10 | D-09/D-10: "config.example.yml's treasurer block is brought into line" | `config.example.yml` has `currency`, `pricing`, `allowance` but **no `cadence` block at all** (lines 279-324). It is loaded by in-tree tests (`src/infrastructure/web/agent_host.rs:2293-2336`, `src/config/web_server.rs:145-180`). `docs/src/getting-started/configuration.md` has **no Pricing section** (only two passing mentions). | Add a commented `cadence:` block to the example (commented keeps those tests inert; if uncommented, run them). Pricing section is wholly new text. |
| F11 | D-11: replace present-tense "eleven" | Beyond the grep hits for "eleven", the same three pages also say "ten" where the new count is eleven or twelve: `release-automation.md:104` ("The ten `crates/*` packages"), `:201` ("The ten per-crate changelogs"), `release-recovery.md:223` ("all ten crate changelogs"), and the **canonical publish-order list** (`release-automation.md:76-90`) omits `paladin-eval` although `scripts/publish-crates.sh:155` lists it. `.crate-names.txt` already has twelve names. `[VERIFIED]` | Fix all of these in CURR-22; the order list gains `paladin-eval` in the parallel-safe leaf tier (before `paladin-ai`). Comments in `release.yml`, `publish-crates.sh`, `finalize-release-body.sh`, `check-deprecations.sh` also say "eleven-crate"; they are outside the three named docs, so leave them or fix them as a flagged extra (see Open Questions). |
| F12 | D-13: correct the root `[0.10.0]` heading | `v0.10.0` commit date is 2026-09-18 (`1d4a9724`, 17:21 -04:00); `v0.10.1` commit is 2026-09-21 (`f7dae267`, 16:03 -04:00); release run `35659477719` was created 2026-09-21T21:50:35Z, conclusion success; crates.io `paladin-eval 0.10.1` was created 2026-09-21T23:12:32Z with `trustpub_data` run_id `35659477719`, sha `f7dae267`. The **ten per-crate** `CHANGELOG.md` files and `paladin-eval`'s also carry `## [0.10.0] - 2026-09-10` and `## [0.10.1] - 2026-09-20`. `[VERIFIED: git log, gh run view, crates.io API]` | Root `[0.10.0]` to `2026-09-18`; root `[0.10.1]` to `2026-09-21` (the tag commit date and the run date agree, which satisfies D-13's test). Recommend the same two edits in all eleven crate changelogs (one mechanical `sed` each, same defect); see Open Questions. |

Also verified and unchanged: the web3sec handoff's line references still hold (MIGRATION.md has not changed since `e28e43b5`; only planning files differ), all four muster-gap items exist and are pinnable (`Stockpile::render` at `crates/paladin-llm/src/services/commissary.rs:219`; `BattalionResult.{paladin_success_count, paladin_failure_count, per_paladin_times}` at `battalion/mod.rs:583-595`; `paladin_ports::output::llm_port::ModelProfile` re-export at `llm_port.rs:239-241`), the `paladin-eval` first OIDC publish evidence, and the baseline: `make check-gates` and `make check-doc-config` (160 YAML blocks) are green in this sandbox today.

## Project Constraints (from CLAUDE.md)

- TDD (Red-Green-Refactor): D-23 and D-21 each start with a failing test (inverted v0.10 fixture test; guard regression harness that plants a surplus dependency). Coverage floor 82 % workspace line coverage (ADR-0006) is evidenced by CI only; the sandbox has no `cargo-llvm-cov`. `[VERIFIED: which]`
- Hexagonal rule: dependencies flow inward only. D-23 lives in `paladin-core` (domain) and adds no dependency; the D-24 `warn!` is in `paladin-battalion`, which already uses `log`.
- Ubiquitous language: Medieval-military terms (Treasurer, Commissary, Cadence, Battalion, Aegis, Garrison); plain words for units. "Node" only for a WarGraph/Campaign vertex (ADR-0060); `Entity<T>` is the generic wrapper.
- Before committing a parent task: `cargo test`, `cargo fmt --check`, `cargo clippy`, `make api-surface` (refresh with `make api-surface-update` plus a CHANGELOG entry only for an intentional surface change), conventional commit, stop after each major task for go-ahead.
- Security: `make security` and `cargo clippy -- -D warnings` on new code, plus the manual credential-handling review. CodeQL is advisory only; do not reintroduce Snyk or record a phase as blocked on it.
- No `unwrap()`/`expect()`/`panic!` in library code; return `Result`. Public items need rustdoc; public APIs need doc tests.
- Shell: `make lint-shell` runs shellcheck `--severity=warning` over every tracked `*.sh` outside `.claude/`, so the new guard script and its harness must be shellcheck-clean. `[CITED: Makefile:375-385]`
- Do not use heredocs to create files in agent workflows; use the Write tool.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Removed-key rejection (D-23) | Domain / core (`paladin-core` deserialization of `BattalionConfig`, `ConclaveConfig`) | Docs (MIGRATION §10.1 row, ADR-0059 note) | The refusal is a property of the config types that every host and consumer shares; hosts add nothing. |
| Maneuver ignored-policy `warn!` (D-24) | Application service crate (`paladin-battalion` Commander bridge) | Docs (rustdoc on `BattalionConfig.aegis`, Commander) | The Commander is where `aegis.on_error` and `aegis.retry` are dropped for Maneuver; Maneuver itself never receives an `Aegis`. |
| Dependency purity guard (D-21) | Build/CI tooling (`scripts/`, `Makefile check-gates`, `ci.yml`) | ADR-0015 (the recorded invariant) | Offline manifest check; no runtime component. |
| Treasurer concept page | Docs (mdBook `architecture/`) | Rust source of truth (`src/application/services/treasurer/`) | Mirrors `commissary.md`; links, does not duplicate, the HTTP contract in `platform-api.md`. |
| Treasurer configuration reference | Docs (`getting-started/configuration.md`) | Config source of truth (`src/config/treasurer.rs`, `config.example.yml`) | The Rust config types are the authority (D-10); the table must be derived from them. |
| Migration guide | Repo-root `MIGRATION.md` (authoritative) | mdBook `upgrading.md` / `migration-guide.md` (orientation) | The root file is gated (allowlist, placeholder grep, guards); the book pages condense and link it. |
| Debt dispositions | Planning ledger (`.planning/WINDOWS.md` via `gsd-tools`) | STATE.md deferred table, ADR-0015 note | The ledger is the only sanctioned writer; STATE rows point at its ids. |
| Nyquist bookkeeping | Planning (archived `VALIDATION.md` files) | `ROADMAP.md`/`STATE.md` | Edited in place under `.planning/milestones/v0.10.0-phases/`. |

## Standard Stack

No external package is added, upgraded or removed by this phase. The "stack" is the in-repo tooling the plans must use.

### Core
| Tool | Version | Purpose | Why standard |
|------|---------|---------|--------------|
| serde `try_from` container attribute | serde 1 (workspace `serde = { version = "1", features = ["derive"] }`) | D-23 load-time refusal without changing struct shape | Already the house idiom for validated types: `cost.rs:70`, `principal.rs:39`, `vault.rs:70` use `#[serde(try_from = ...)]` `[CITED: file:line]`. Prototype run this session `[VERIFIED: scratch crate]`. |
| `scripts/check-*.sh` + `tests/scripts/*_test.sh` + `Makefile check-gates` + `ci.yml` step | existing | D-21 guard | Every gate in `check-gates` has all four parts; CI runs the scripts individually (it does not call `make check-gates`), under the `cargo-deny` job (`ci.yml` ~lines 120-161). |
| python3 `tomllib` | python 3 (stdlib) | Parse `[dependencies]` in the guard | Used by `check-crate-names.sh` and `check-changelogs.sh` already; no PyYAML needed for the guard. |
| `gsd-tools windows` | installed `.claude/gsd-core` | Ledger writes (D-22) | The only sanctioned writer. |
| `make check-doc-config` | existing | YAML fences in `docs/src` must parse | Needs PyYAML (present here). Regex only matches a fence that is exactly ```yaml or ```yml. |
| `make check-doc-examples` | existing | Compiles `crates/doc-examples`, rustfmt-scans inline ```rust fences | A Treasurer worked example is either an `{{#include}}` of a compiled anchor in `crates/doc-examples/src/` or a ```rust,ignore fence. |
| mdBook 0.4.40 + `mdbook-linkcheck` 0.7.7 (`warning-policy = "error"`, `follow-web-links = false`) | CI only | Relative links and `SUMMARY.md` entries must resolve | Not installed in this sandbox; web links (GitHub blob URLs) are not followed. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `try_from` wire struct (D-23) | Per-key private fields with `deserialize_with` | Changes constructibility of two public structs; trips legacy guard rule 6 in the two scoped files. Rejected. |
| `try_from` wire struct (D-23) | `#[serde(deny_unknown_fields)]` | Forbidden by D-23 (rejects every unknown key, breaks forward compatibility). |
| Shell guard (D-21) | A `cargo test` in the root crate | Slower (builds the facade), and the house shape for offline gates is a script. Recommend the shell guard. |
| Python `tomllib` in the guard | `cargo metadata` | `cargo metadata` resolves features and renames (`paladin_core = { package = "paladin-ai-core" }`) and does not preserve the manifest key names ADR-0015 lists. Read the manifest keys. |

**Installation:** none.

**Version verification:** serde and serde_json resolved from the workspace; the prototype compiled with `serde 1`, `serde_json 1`, `serde_yaml 0.9`. cargo 1.97.1 / rustc 1.97.1 (MSRV gate is 1.88; `try_from` containers and `IgnoredAny` are old, stable serde features). `[VERIFIED: cargo run in scratch crate]`

## Package Legitimacy Audit

No external package is installed or recommended by this phase, so the `package-legitimacy` seam was not run.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none) | - | - | - | - | - | - |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

The D-21 guard reads two in-repo manifests with the Python standard library. D-23 uses `serde`, already a workspace dependency of `paladin-core`.

## Architecture Patterns

### Work Flow (how the pieces depend on each other)

```
v0.10.1 tag (git) ──► git diff -U0 v0.10.1..HEAD -- MIGRATION.md ──► move-set (rows, in-place edits, prose)
                                                                          │
D-23 code + tests ──► inverted v0.10 fixture test ──► §10.1 row text ─────┤
D-24 rustdoc + warn! ──► Commander/Maneuver docs                          │
muster pins (D-04) ──► red-then-green proof                               ▼
web3sec G-1..G-7 ───────────────────────────────────────────► MIGRATION §10 (last docs commit)
                                                                          │
   same commit ► widen awk (script + ci.yml + harness) ◄─────────────────┤
   same commit ► make check-migration-allowlist green ◄───────────────────┤
   then        ► retarget live §9.x citations ► CHANGELOG [Unreleased], rustdoc comments, switching-providers.md
                                                                          ▼
                      upgrading.md rewrite ► migration-guide.md pointer ► (no SUMMARY entry)
Treasurer page ► SUMMARY.md (Architecture) ► links config reference ► links platform-api.md#allowance-refusals
config.example.yml cadence block ► tests that load it
ADR-0015 dated note ► check-dependency-allowlist.sh ► Makefile check-gates ► ci.yml step ► harness test
windows append/waive/fixed (sequential) ► STATE.md rows 796/797/815/741 rewritten ► ROADMAP/REQUIREMENTS ticks
8 reconcile-only VALIDATION runs ► frontmatter lines ► plan SUMMARY evidence
```

### Recommended file touch map
```
MIGRATION.md                                      # §10 above §9 (D-01/D-02), awk-visible headings
scripts/check-migration-allowlist.sh              # awk window (9|10); header comments
.github/workflows/ci.yml                          # same awk, semver job step (line ~441); new guard step in the gates job
tests/scripts/check-migration-allowlist_test.sh   # fixture header + new case
scripts/check-dependency-allowlist.sh             # new (D-21)
tests/scripts/check-dependency-allowlist_test.sh  # new harness (house shape, run by make test-shell-guards)
Makefile                                          # new target; add to check-gates prerequisites
crates/paladin-core/src/platform/container/battalion/legacy_keys.rs   # new (D-23)
crates/paladin-core/src/platform/container/battalion/{mod.rs,conclave.rs}  # try_from attribute + rustdoc only
tests/consumer_surface_muster.rs                  # four pins (D-04)
docs/src/architecture/treasurer.md, docs/src/SUMMARY.md
docs/src/getting-started/configuration.md, config.example.yml
docs/src/api-reference/{upgrading.md,migration-guide.md}
docs/src/user-guides/switching-providers.md       # G-4 note
docs/src/contributing/development-setup.md, docs/src/appendix/{release-automation.md,release-recovery.md}
CHANGELOG.md (+ 11 crate CHANGELOG.md headings, recommended)
.planning/decisions/{0015,0059}-*.md              # dated notes only; PROMOTION.md unchanged (no new ADR)
.planning/{WINDOWS,STATE,ROADMAP,REQUIREMENTS}.md, 8 archived VALIDATION.md files, web3sec todo (dated update)
```

### CURR-22 edit list (all verified against the live files)

1. `docs/src/contributing/development-setup.md:652` "the eleven workspace crates" to "the twelve workspace crates".
2. `docs/src/appendix/release-automation.md`: line 76 "The eleven-crate order" to "The twelve-crate order" and add `paladin-eval` to the tier-4 list (lines 80-90); line 104 "The ten `crates/*` packages" to eleven; line 201 "The ten per-crate changelogs" to eleven; line 318 "All eleven crates are covered" to twelve (and keep "none is excluded"); line 326 "and all eleven passed" is a statement about the `0.8.1-rc.2` proof run: keep as written with "(all eleven crates existing then)"; line 375 "the whole eleven-crate sequential loop" to twelve.
3. `release-automation.md` Trusted Publishing table (after the `paladin-storage` row, before `paladin-ai`, to follow publish order): `| paladin-eval | crates/paladin-eval | release.yml | crates-io | on or before 2026-09-21 | linked (proven by publish) |`. Extend the paragraph after the table (lines 318-327) with one sentence: this row is proven by the `0.10.1` publish (run `35659477719`, crates.io `trustpub_data` present), not reported by the operator. `[VERIFIED: crates.io API: paladin-eval 0.10.1 created_at 2026-09-21T23:12:32Z, trustpub_data {provider github, repository DF3NDR/paladin-dev-env, run_id 35659477719, sha f7dae267...}]`
4. Credential History table: add `| OIDC publish of paladin-eval 0.10.1 (first publish of this crate, non-null trustpub_data) | 2026-09-21 | (the release workflow; tag pushed by the repository owner) | run [35659477719](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35659477719), .planning/MILESTONES.md Phase 37.1 evidence |`. Add "(eleven crates at the time)" to the two `0.8.1-rc.x` rows' wording, per D-11.
5. `release-recovery.md`: lines 40 and 89 "the eleven crates" to twelve; line 202 "all eleven crates" to twelve; line 223 "all ten crate changelogs" to eleven; line 224 "hand-editing eleven files" to twelve (eleven crate changelogs plus the root).
6. `CHANGELOG.md`: `## [0.10.0] - 2026-09-10` to `2026-09-18`; `## [0.10.1] - 2026-09-20` to `2026-09-21`. The bold `**2026-09-20:**` note at line 1849 and `nightly-2026-09-20` at line 1807 are dated statements about other events: leave them.
7. Proof for the verifier: `git log -1 --format=%cs v0.10.0` prints 2026-09-18 and `git log -1 --format=%cs v0.10.1` prints 2026-09-21.

### CURR-23: MIGRATION §10 procedure (ordered)

1. **Inventory first.** `git diff -U0 v0.10.1..HEAD -- MIGRATION.md` is the move-set (ranges at HEAD: §9.1 30-47; §9.2 206-207, 225-228, 233, 236-361; §9.3 450, 464-465; §9.4 493-505; §9.5 588-631; §9.6 885-943; §9.7 946, 948; §9.8 975-982 and 1004-1211). Save the pre-move identity set: all `M-B-\d+`, `DR-\d+`, and `crate | type` first-column pairs, to compare after (a row must never change identity, D-02).
2. **Add §10 above §9** with the exact heading texts of §9 so GitHub anchors are predictable: `## 10.1 Behavioral changes (user-visible without code changes)`, `## 10.2 Rust API changes (compile-affecting; the X-10 register)`, `## 10.3 Toolchain & dependencies`, `## 10.4 Persistence & schema migrations`, `## 10.5 Configuration & environment`, `## 10.6 HTTP API`, `## 10.7 Deprecations`, `## 10.8 Upgrade checklist`. The row-extraction `awk` needs `## 10.2 ` and `## 10.3 ` to start a line exactly this way.
3. Retitle `# Upgrading from v0.9.0 to v0.10.0` to `# Upgrading Paladin` with a scope note listing the two parts; keep the 2026-09-20 addendum.
4. Move rows; restore the four in-place-edited v0.10 rows in §9.2 from `git show v0.10.1:MIGRATION.md`; add one pointer line at the top of each §9.x that lost rows ("v0.11 rows that were appended here now live in §10.x").
5. Rewrite the in-text references inside moved text from `9.x` to `10.x`, including the DR table's Register column and the "Phase 50 behaviour changes" bullets (they say "section 9.1", "9.5").
6. Apply G-1..G-7 (below). Write §10.8 as: an operator checklist (new, v0.10.1 to v0.11.0: back up, migrations run at start, config changes with the TENANT-01 note and treasurer/cadence defaults, deploy, verify), then the `Library consumers: v0.10.1 → v0.11.0` subsection moved whole.
7. **Same commit:** widen the awk in `scripts/check-migration-allowlist.sh` and `ci.yml`, update `tests/scripts/check-migration-allowlist_test.sh`, then run `make check-migration-allowlist` and `make test-shell-guards`.
8. The placeholder gate greps the literal four-letter token `TBD` anywhere in MIGRATION.md (`ci.yml:483-503`): the new text must not contain it, including in prose about the gate. `[CITED: ci.yml]`
9. Then retarget live citations (F4) and run `cargo test --test legacy_removal_guard --test entity_rename_guard --test treasurer_vocabulary_guard` (MIGRATION.md is in both guards' history sets, the book pages `upgrading.md` and `migration-guide.md` too; new pages are scanned).

**G-1..G-7 (all locations still hold at HEAD):**

| # | Edit |
|---|---|
| G-1 | DR-3 (line 1042): `paladin_core::base::entity::Entity<T>` to `paladin::core::base::entity::Entity<T>`. |
| G-2 | M-B-08 (line 33): add the recipe: to keep the v0.10 text read `message` from the `NodeErrorSource::Paladin`, `::Llm` and `::Function` arms (it equals the old `PaladinError` display; the `Display` of the source adds the `"paladin error: "` / `"llm error: "` prefix, `node_error.rs:113-115`); `node_id` is the Paladin's name. Short code block (CONTEXT specifics). Pattern exists in the muster: `node_error_read_pin` (`consumer_surface_muster.rs:486-506`), including the required wildcard arm because `NodeErrorSource` is `#[non_exhaustive]`. |
| G-3 | M-B-06 (line 31) and DR-9 (line 1048): state both outcomes. With `on_error: None` a Phalanx timeout is inside `Err(AggregationError)`; with `Absorb` the run returns `Ok` and the timeout is a `node_errors` entry with source `Timeout(Run)` (cite `phalanx_service.rs:233-272` as web3sec did; re-check the line range before citing). |
| G-4 | Line 1140: drop "(the two models web3sec runs)" or write "two of the models web3sec runs". On `switching-providers.md` add: DeepSeek's `/models` listing contains `deepseek-flash` (seeded, line 262) and does not list `deepseek-v4-flash`, which is unseeded and runs on the adapter-wide fallback. Facts per web3sec's live check of 2026-10-24: tag `[CITED: 46-FEEDBACK-web3sec.md]`, not re-run here. |
| G-5 | Cadence recipe (lines 1108-1111): `paladin::config::treasurer::CadenceConfig`, `paladin::infrastructure::cadence::{build_cadence, with_cadence, compose_llm}` `[VERIFIED: cadence.rs:128, 203; re-export at :86]`; `compose_llm` takes `&Arc<PriceTable>` (`paladin::core::platform::container::cost::PriceTable`); mention that `with_cadence(llm, wiring.as_ref())` is the pacing-only form. |
| G-6 | Line 1120 decorator bullet: name both `model_profile` and `check_reasoning` with "must", matching DR-21 (line 1060) and the trait docs. |
| G-7 | One explicit sentence in the "Starting before `v0.10.0`" block (line ~1025): from `35fd839`, DeepSeek made up to 4 and Anthropic up to 3 calls with back-off on `RateLimitExceeded` (for `max_retries = 3`); they now make 0 retries without Cadence. `[VERIFIED: git show 35fd839:crates/paladin-llm/src/deepseek/adapter.rs lines 506-511, 533, 556-562 and .../anthropic/adapter.rs lines 386-425; current surfaces the first 429 at deepseek/adapter.rs:989-999]` |

**Muster pins (D-04), all test-only in `tests/consumer_surface_muster.rs`:**
- `Stockpile::render`: `let _render: fn(&Stockpile) -> String = Stockpile::render;`
- `BattalionResult` fields: extend `node_error_read_pin`'s neighbour with a read pin of `paladin_success_count: usize`, `paladin_failure_count: usize`, `per_paladin_times: HashMap<String, u64>`.
- Prelude glob: put `use paladin::prelude::*;` inside a private `mod prelude_glob { ... }` and use at least one item through it (an unused glob import is `unused_imports`, which fails `clippy -D warnings`; the file allows only `dead_code` and `type_complexity`). Pick items the consumer takes from the prelude, for example `PaladinBuilder` and `LlmPort`.
- `ModelProfile` via `paladin_ports::output::llm_port::ModelProfile`: a fn-pointer or type-equality pin beside the existing `output::model_profile::ModelProfile` import (line 82), for example `fn same(a: paladin_ports::output::llm_port::ModelProfile) -> ModelProfile { a }`.
- The red-muster rule applies: show the pin red (temporarily removing the item or mis-typing the path) once, then green; record it in the plan SUMMARY.

**Book pages (D-05):** rewrite `docs/src/api-reference/upgrading.md` to lead with v0.10.1 to v0.11.0 (condensed §10.1 table, the library-consumer index by topic, operator checklist mirrored from §10.8), keep the v0.9 to v0.10 orientation below. Add one pointer line to `migration-guide.md`. Both files are in the legacy and entity guards' history sets, so they may name removed items. Links to the root file use `https://github.com/DF3NDR/paladin-dev-env/blob/main/MIGRATION.md#<anchor>` like the existing ones; the new anchors are `#101-behavioral-changes-user-visible-without-code-changes`, `#102-rust-api-changes-compile-affecting-the-x-10-register`, and so on (GitHub slug rule; `[ASSUMED]` for the §10 anchors until the page renders on `main`, though they follow the pattern of the existing `#92-...` links).

### Treasurer page and configuration reference

`docs/src/architecture/treasurer.md` follows `commissary.md`'s section order: intro with location and ADR pointer; `## Concept` (two responsibilities, "framework owns measurement and enforcement; callers own policy" analogue); `## The model` (type table); `## Flow` (mermaid); `## Usage sketch`; sections for allowance admission, mid-run halt, pricing as cost source, `## Rate pacing (Cadence)` (D-08); `## See also`. All names verified to exist: `Treasurer` (`mod.rs:150`), `OperatorNoticeTarget` (`:105`), `TreasurerSpendGuard` (`guard.rs:102`, re-exported), `AllowancePolicy`, `ScopeAllowance`, `Ceiling` (`policy.rs`, structs), `window_for(now, period_secs) -> Option<(DateTime<Utc>, DateTime<Utc>)>` (`window.rs:32`), ports `AllowanceAdmissionPort` (`crates/paladin-ports/src/input/allowance_admission_port.rs:98`) and `TreasuryNoticePort` (`.../output/treasury_notice_port.rs:136`). `[VERIFIED: grep]`

Constraints on the page:
- Worked example: ```rust,ignore with the same honesty note `commissary.md` uses, or a compiled `{{#include}}` from `crates/doc-examples/src/` (then `make check-doc-examples` compiles it). Do not write an inline plain ```rust fence unless it passes the rustfmt scan.
- The downstream-fixture vocabulary guard (`tests/treasurer_vocabulary_guard.rs` rule 2) forbids the downstream application's fixture term anywhere outside `.planning/`, `.project/`, the ledger module docs and the guard. Do not quote ADR-0050's guardrail sentence that names it. `Treasurer` itself is allowed in docs (rule 1 covers only `examples/`, `benches/`, fixtures).
- ADR-0050's row in `docs/src/contributing/adr-index.md` still says "not built this cycle" and the index stops at 0051 (no rows for 0052-0062). The page states that the name was reserved by ADR-0050 and built in Phases 38-43 (ledger, pricing, admission, mid-run halt, pacing), with Phases 50-51 extending what it meters; consider refreshing that row and adding 0052-0062 (Open Questions).
- Link ADRs by GitHub blob URL as `adr-index.md` does, not by repo-relative path (mdBook cannot resolve `.planning/`).
- `429 allowance_exhausted`: link `../api-reference/platform-api.md#allowance-refusals` (heading at `platform-api.md:652`) and `#operator-allowance-notices` (`:500`), do not duplicate the payload.

**Configuration reference key table (verified against `src/config/treasurer.rs`).** One shared table, columns key, type, default, validation, env override. Every public key:

| Key | Type | Default | `validate()` / load rule | Env override |
|---|---|---|---|---|
| `treasurer.currency` | string | `"USD"` | exactly three ASCII uppercase letters, never case-folded (`price_table`, `allowance_policy`) | `APP_TREASURER_CURRENCY` (applied verbatim; lowercase is rejected like a file value) |
| `treasurer.pricing.<model>` | map of rows | empty map (inert; nothing priced) | key non-empty, matched exactly and case-sensitively against the model string an `LlmResponse` reports | none (config file only) |
| row `prompt` (required) | decimal string, nano-units per 1M tokens | none | digits with optional fraction of at most 9 places, non-negative, at most `9223372036.854775807` | none |
| row `completion` (required) | same | none | same | none |
| row `cache_read` | same | omitted bills at `prompt` | same | none |
| row `cache_write` | same | omitted bills at `prompt` | same | none |
| row `reasoning` | same | omitted bills at `completion` | same | none |
| `treasurer.allowance.warn_at` | integer percent | `80` (`DEFAULT_ALLOWANCE_WARN_AT`) | `0..=100`, `0` disables, `>100` rejected | `APP_TREASURER_ALLOWANCE_WARN_AT` |
| `treasurer.allowance.webhook.url` | non-empty `http(s)` URL | none | non-empty; SSRF guard at wiring, `webhooks.allow_private` for private addresses | none |
| `treasurer.allowance.webhook.secret` | string, never rendered or serialised | none | not echoed in errors | `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` (applied only when a `webhook` exists; `${VAR}` in YAML is not expanded) |
| `treasurer.allowance.tenants.<id>` / `api_keys.<name>` | entry | none | key must be a valid id; must name a configured tenant / API key (`validate_against`); needs a run store | none (config file only) |
| entry `period` (required) | `<integer><m\|h\|d>` | none | `1m` to `366d` (60 s to 31,622,400 s) | none |
| entry `amount` (required) | decimal string, whole currency units | none | positive, at most 9 places, within `i64` nano-units | none |
| entry `lifetime` | decimal string | none | positive, same format | none |
| entry `warn_at` | integer percent | the global `warn_at` | `0..=100` | none |
| `treasurer.cadence.*` | existing seven scalar rows plus `backend` | as in the existing table (`enabled` true, 500, 30000, 300, 2.0, 60, 120) | as in `CadenceConfig::validate` | seven `APP_TREASURER_CADENCE_*`; `backend` has none |

Semantics for the Pricing section `[CITED: configuration.md:490-496, crates/paladin-llm/src/pricing.rs:29-46, WINDOWS.md row 67]`: an unpriced model keeps `cost: None` and logs one `warn!` per model name; a principal with an allowance ceiling calling an agent whose model has no row is refused `422 model_unpriced` at admission; a workflow-node call on an unpriced model settles nothing (no price is fabricated) so an allowance cannot bound it. Every struct under `treasurer:` uses `deny_unknown_fields`, so a typo is a load error (this is the house precedent that D-23 deliberately does **not** copy for `BattalionConfig`).

### CURR-24: Nyquist reconciliation (reconcile-only)

State of the eight files `[VERIFIED: frontmatter read]`:

| Phase | Plans | status | nyquist_compliant | wave_0_complete | Note |
|---|---|---|---|---|---|
| 22 | 17 | draft | false | false | seeded template rows (`TBD`), ENG-01..08 |
| 24 | 14 | draft | false | false | template |
| 28 | 17 | validated | false | true | real map (45 rows), 2 Manual-Only rows, `validated: 2026-09-09`; `28-UAT.md` shows 2 results `pass`; stale text "No WINDOWS.md row exists yet" (row 35 exists now, row 61 amends it) |
| 29 | 9 | draft | false | false | template |
| 30 | 3 | draft | **true** | true | already marked compliant while `draft`; reconcile the status |
| 34 | 9 | draft | false | false | template |
| 36 | 13 | draft | false | false | template |
| 36.1 | 14 | draft | false | false | template, no table rows |

Procedure per phase (`init.phase-op N` resolves every archived dir `[VERIFIED: all eight returned phase_found true under .planning/milestones/v0.10.0-phases/]`; the Nyquist step hook is active `[VERIFIED: render-hooks verify:post]`):
1. Read the phase's PLAN/SUMMARY/VERIFICATION. Build the per-task map with real task ids, requirement ids and the command that proves each (docs and gate phases 29, 34, 36, 36.1 map to `make`/`grep`/script commands, not unit tests).
2. Re-run a representative subset on HEAD where feasible (sandbox has cargo 1.97 and a `target/` cache; heavy feature legs are CI evidence). Record the command and result.
3. Set `status: validated`, set `nyquist_compliant` to what the map supports, `wave_0_complete` consistently, add `validated: <date>` and a dated reconciliation note in the frontmatter (D-16), add a `## Validation Audit <date>` table (gaps found / resolved / escalated, as the workflow's Step 6 specifies) and an explicit `## Gaps` section. Do not write tests. A phase whose gaps remain is `validated` plus `nyquist_compliant: false`, which `audit-milestone` reads as PARTIAL, not NOT-VALIDATED; that satisfies ROADMAP criterion 3.
4. Phase 28 (D-15): flip `nyquist_compliant: true`, check the last Validation Sign-Off box, rewrite the two Manual-Only rows to cite `28-UAT.md` 2/2 passed, the D-37 acceptance and WINDOWS rows 35/61, fix the stale "no WINDOWS row" sentence, and leave the accepted deviation as recorded.
5. Do **not** run the workflow's Step 4/5 (interactive gap menu, auditor that generates tests). The Step 7 commit helper `gsd-tools query commit` is fine.
6. Evidence for ROADMAP criterion 3: `grep -H '^status:\|^nyquist_compliant:' .planning/milestones/v0.10.0-phases/{22,24,28,29,30,34,36,36.1}-*/*-VALIDATION.md` (no `draft` left).
7. Rewrite STATE.md row at line 741 to the outcome; the archived 05-21 rows (line 754 and the v0.8.0 table) stay open as written (D-17).

### CURR-25: debt lines

**Measured today (`wc -l`)** `[VERIFIED]`: `crates/paladin-battalion/src/engine/superstep.rs` 16,018 (production code ends at the `#[cfg(test)] mod tests` on line 5364, so about 5.4k production lines); `engine/mod.rs` 14,324 (several `#[cfg(test)]` module declarations from line 90, so production versus test needs a separate count); `src/application/services/paladin/paladin_execution_service.rs` 10,858 (tests from line 4382, about 4.4k production lines). CONTEXT's "about 16k / 14k / 10.7k" are the raw figures; the waiver should state both raw and production-only so it is not read as overstating the debt. The audit's original figures (2,757 / 2,294 / 1,840 lines) are for different, since-reshaped files.

**D-20/D-21 guard (design).** `scripts/check-dependency-allowlist.sh`, house shape of `check-crate-names.sh`: `set -euo pipefail`, workspace root from `BASH_SOURCE`, python3 `tomllib` reads `[dependencies]` key names (not package names) of `crates/paladin-core/Cargo.toml` and `crates/paladin-ports/Cargo.toml`, compares against literal sets pinned in the script, bidirectional (an added crate fails; a removed crate is a stale entry and fails), accumulates all violations, exits non-zero, prints a one-line OK. Also reject `[build-dependencies]` and `[target.*.dependencies]` additions (neither exists today). The ADR-0015 sets, with the F7 amendment:
- `paladin-core` (14): `serde`, `serde_json`, `uuid`, `chrono`, `thiserror`, `async-trait`, `tokio`, `sha2`, `blake3`, `petgraph`, `murmur3`, `url`, `regex`, `futures` `[VERIFIED: manifest lines 18-32]`.
- `paladin-ports` (12): `paladin_core`, `async-trait`, `serde`, `thiserror`, `uuid`, `chrono`, `tokio`, `serde_json`, `futures`, `log`, `md5`, `mime_guess`.
Wiring (four parts, all required by house convention): the script; `tests/scripts/check-dependency-allowlist_test.sh` (fixtures with a planted surplus and a planted removal, plus the green case; run by `make test-shell-guards`, which CI invokes as one step); a `Makefile` target `check-dependency-allowlist` added to the `check-gates` prerequisite list (currently eight members); a `ci.yml` step beside "Check publish order" because CI does not call `make check-gates`. shellcheck must be clean (`make lint-shell`).

**ADR-0015 amendment** follows the precedent shape (`**Amended ...**` line near the top or a dated `## Amendment` section; see ADR-0004 and ADR-0006). Name `log`, the date, why invariant (i) holds, and that the guard now pins fourteen / twelve. `docs/src/contributing/adr-index.md` has no ADR-0015 row change needed.

**Ledger rows (D-22)**, sequential, each append then close; suggested kinds `todo` (oversized files, clone/lock) and `deviation` (allowlist drift, fixed). Waive reasons carry the measured text from F8 and the production-versus-raw counts; the fixed row's description carries the guard path and ADR note since `fixed` takes no text. Then rewrite STATE.md lines 796, 797 and 815 to point at the three ids, and ROADMAP "Tech debt tracked as v2" bullets at lines 956-965 get a dated note. Confirm with `gsd-tools windows status`.

### D-23 design (removed v0.10 config keys)

- Container attribute `#[serde(try_from = "legacy_keys::BattalionConfigWire")]` on `BattalionConfig`; the wire struct mirrors the four current fields (`aegis` keeps `#[serde(default)]`) and adds three removed-key members using a `deserialize_with = "present"` helper that consumes any value (null included) and returns `Some(())`. `TryFrom` collects every present removed key and returns one `String` error listing each key with its replacement and the §10.1 row (`M-B-06`), so a document carrying all three names all three. `ConclaveConfig` gets the same treatment for its one removed key; its nested `battalion_config` is covered automatically. Messages never echo values.
- Replacements to name: `timeout_seconds` to `aegis.timeout.run_timeout`, `retry_policy` to `aegis.retry`, `error_strategy` to `aegis.on_error` (ADR-0059 D-01/D-02/D-04, M-B-06/M-B-07/M-B-11; confirm against the final row text).
- Unknown unrelated keys still load (prototype: `{"name":"a","novel":1}` is Ok). `Serialize` and the struct literal are untouched.
- The CLI's own `timeout_seconds` is a different type (`src/application/cli/config/battalion_config.rs:284`, `Option<u64>` mapped at `:614` via `per_attempt_timeout(...)` onto `aegis`); add a regression assertion that the CLI YAML keeps loading.
- Tests (write first): each key alone and all three together fail naming each key; `null` value fails; v0.11 document and a document with an unrelated unknown key load; round-trip unchanged; nested `ConclaveConfig` document with a removed key inside `battalion_config` fails; invert `battalion_config_v0_10_document_deserializes_with_default_aegis` (it lives after `mod tests`, so rule 6 does not scan it).
- Same commit: ADR-0059 dated amendment (it recorded "the removed keys are ignored"; the sentence to amend is D-01's closing "a serialized `BattalionConfig` document loses three keys and gains one" and the todo's quote of D-00c), MIGRATION §10.1 `M-B-06` and §10.2 `BattalionConfig` / `ConclaveConfig` rows, CHANGELOG `[Unreleased]` line, rustdoc on the two types describing the refusal (`make doc-check` zero warnings). Confirm with `make api-surface`: the Deserialize impl exists both before and after, so no diff is expected `[ASSUMED: not run]`.
- No new dependency and no §10.2 `Y` row (struct shape unchanged), consistent with D-00b.

### D-24 items

- WR-04 rustdoc: `BattalionConfig.aegis` (the bullet list at `battalion/mod.rs:63-99`) gains a **Maneuver** bullet (Maneuver reads `aegis.timeout.run_timeout` only, as one whole-flow clock; it ignores `retry` and `on_error`) and the Commander docs (`commander.rs:1473`, `:1739`) state that the Commander no longer bounds ChainOfCommand or Grove, and that `default_maneuver_config` (`commander.rs:532-547`) turns the per-attempt `run_timeout` into a whole-flow clock.
- WR-04 `warn!`: the ignoring happens in the Commander's Maneuver branch where `default_maneuver_config()` drops `on_error` and `retry` (`commander.rs:936-946`). Mirror the Campaign message shape (`campaign_service.rs:241-249`, once per execution). Put the decision in a small pure function (for example listing which of `aegis.on_error` / `aegis.retry` are set) and unit-test that; the repository does not capture log output for the Campaign warn either. Keep the production lines free of `timeout(` and `timeout_seconds` (guard rule 5 covers `commander.rs`); words in `//` comment lines and in `run_timeout` (identifier boundary) are fine.
- WR-05: a dated "remove after 0.11.0 is the published baseline" comment above the crate-wide `allow` lines in `crates/paladin-core/Cargo.toml` (`struct_missing`, `enum_missing`, `enum_variant_missing`, at lines ~129-140) and `crates/paladin-battalion/Cargo.toml` (`function_missing`, `module_missing`, line ~106). Comment only; `check-migration-allowlist` also requires each crate-wide allowed lint to stay mirrored by an allowlist entry, so do not remove a line.
- Optional rustdoc-only info items: IN-05 (circuit breaker rustdoc: a Permanent failure neither resets the count nor re-opens a HalfOpen probe, `src/infrastructure/resilience/circuit_breaker.rs`) and IN-N1 (`execute_with_cancellation`: an in-flight attempt finishes and keeps its permit; FirstSuccess ignores `max_concurrency`). Leave IN-01..IN-04, IN-N2 (tests) deferred as CONTEXT allows.
- D-25: dated update on `.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md` pointing at `46-FEEDBACK-web3sec.md` and the resulting §10 commit; status stays `pending`.

### Recommended plan/wave order (refinement of CONTEXT's discretion list)

1. CURR-22 audit fixes and CHANGELOG dates. (independent)
2. Nyquist reconciliation, eight files. (independent, large; split in 2-3 plans)
3. Debt lines: ADR-0015 note, guard + harness + Makefile + ci.yml, three ledger rows, STATE rewrite.
4. D-23 and D-24 code with tests, and the D-04 muster pins. **Before** the §10 freeze, so §10.1/§10.2 text for `M-B-06`, DR-8 and the `BattalionConfig` row is written once against shipped behaviour.
5. Treasurer page and configuration reference, `config.example.yml`.
6. MIGRATION §10 restructure (with gate edits) then citation retargeting; G-1..G-7; the last docs commit.
7. Book pages (`upgrading.md`, `migration-guide.md`, `switching-providers.md` note), ROADMAP/STATE/REQUIREMENTS bookkeeping, web3sec todo update.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Rejecting specific legacy keys | A hand-written `Visitor`/`Deserialize` impl or a `serde_json::Value` pre-scan | `#[serde(try_from = ...)]` with `IgnoredAny` presence members | Works for JSON and YAML, nested types, and `null`; prototype verified; house idiom. |
| Writing ledger rows | Editing `.planning/WINDOWS.md` by hand | `gsd-tools windows append/waive/fixed/status` | The frontmatter counts are validated on every read; a hand edit that drifts the counts makes the ledger unreadable. |
| Parsing manifests in the guard | Regex over `Cargo.toml` | `python3` `tomllib` | Same reason `check-crate-names.sh` uses it: sections, inline tables and renames. |
| Row identity in the migration move | Eyeballing | Extract and diff the `M-B-xx` / `DR-xx` / `crate \| type` key sets before and after | D-02's identity-preservation promise is mechanically checkable. |
| Finding what is "v0.11" in MIGRATION.md | Judging by phase number | `git diff -U0 v0.10.1..HEAD -- MIGRATION.md` | Exact, includes in-place edits and prose. |
| Release-date truth | Memory | `git log -1 --format=%cs <tag>`, `gh run view`, crates.io API `trustpub_data` | All three were used here and agree. |

**Key insight:** every gate in this repo reads text by position or heading. Restructuring documents is therefore a gate change, not just an edit; plan the gate edit and the document edit as one commit.

## Runtime State Inventory

The §9 to §10 move and D-23 are refactor/migration-like, so the five categories are answered explicitly.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None in this repository's datastores embeds a section number. The only population affected by D-23 is a **consumer-stored `BattalionConfig` / `ConclaveConfig` JSON or YAML document** carrying a removed key; in-tree, `BattalionConfig` is built in code (docs state `config.yml` has no `battalion:` section, `configuration.md:355-363`). Owner reports one consumer (web3sec). `[VERIFIED: grep, doc read; consumer data not inspectable]` | Code edit (new refusal) plus the §10.1 row telling the consumer to delete or convert the keys. No data migration. |
| Live service config | The `semver` job in `.github/workflows/ci.yml:441` carries an inline copy of the register-extraction awk. | Code edit in the same commit as the script (F1). |
| OS-registered state | None. `[VERIFIED: nothing in scope registers tasks, units or services]` | None. |
| Secrets/env vars | No env-var name changes. `APP_TREASURER_*` are documented as they are. The webhook secret stays env-only. | None. |
| Build artifacts / installed packages | `MIGRATION.md` and the crate `CHANGELOG.md` files ship inside published crate tarballs from `v0.11.0`; nothing already published changes. `target/` and `docs/book/` are regenerated. | None. |

## Common Pitfalls

### Pitfall 1: Moving rows under `## 10.2` and trusting the allowlist
**What goes wrong:** `make check-migration-allowlist` and CI's `semver` job print the whole allowlist as "extra" pairs and fail.
**Why it happens:** the register window is the literal `## 9.2 ` to `## 9.3 ` span (F1).
**How to avoid:** widen the awk in script, ci.yml and harness in the same commit; run `make check-migration-allowlist` and `make test-shell-guards`.
**Warning signs:** a diff whose "+" side lists every v0.11 `crate | type` pair.

### Pitfall 2: The removed key names trip the legacy guard
**What goes wrong:** `cargo test --test legacy_removal_guard` fails rule 6 (core files) or rule 5 (service files).
**Why it happens:** whole-word match of the removed names on non-comment lines before `mod tests`; string literals and serde attributes count.
**How to avoid:** keep names and messages in `legacy_keys.rs`; in `commander.rs` avoid `timeout(`/`timeout_seconds` outside comments.
**Warning signs:** violation lines `battalion/mod.rs:N: rule 6`.

### Pitfall 3: A literal `TBD` in MIGRATION.md
**What goes wrong:** CI step "Verify MIGRATION.md carries no unfilled placeholder marker" fails.
**How to avoid:** never write the token, even quoting the rule. `grep -c 'TBD' MIGRATION.md` must print 0 before each commit.

### Pitfall 4: `rust` fences and YAML fences in the book
**What goes wrong:** `check-doc-examples` rustfmt-scans plain ```rust blocks and fails on non-compiling snippets; `check-doc-config` fails on a YAML fence that does not parse, and only matches fences that are exactly ```yaml / ```yml.
**How to avoid:** ```rust,ignore for illustrations or `{{#include}}` of a compiled anchor; keep YAML snippets real (they are parsed, not schema-checked). Use ```text or ```yaml with real YAML for shell/env tables.

### Pitfall 5: Citation drift after the move
**What goes wrong:** `[Unreleased]` and rustdoc point readers at §9.2 for rows that now live in §10.2.
**How to avoid (recipe):** list candidates with `grep -rnE '§ ?9\.[0-9]|[Ss]ection 9\.[0-9]|MIGRATION(\.md)? 9\.[0-9]' --include=*.rs --include=*.toml --include=*.md --include=*.yml . | grep -v '^./target\|^./.planning\|^./.project\|^./.claude'`, then retarget only those whose cited row is in the v0.11 move-set. Links of the form `MIGRATION.md#92-...` in `[Unreleased]` become `#102-...`.

### Pitfall 6: Parallel or repeated ledger writes
**What goes wrong:** two concurrent `windows append` calls lose one entry (documented in the handler); a repeated run adds duplicate rows.
**How to avoid:** strictly sequential; check `windows status` before and after; record the resulting ids in the plan SUMMARY.

### Pitfall 7: Treating `validate-phase` as written
**What goes wrong:** an executor spawns `gsd-nyquist-auditor`, which generates tests (forbidden by D-14) or blocks on `AskUserQuestion`.
**How to avoid:** the plan text says "reconcile-only: edit the VALIDATION.md by hand from the archived artifacts; do not spawn the auditor".

### Pitfall 8: The example config is test input
**What goes wrong:** uncommenting a `cadence:` block in `config.example.yml` changes what `agent_host.rs:2293` and `web_server.rs:145` tests load.
**How to avoid:** add the block commented, as the `allowance:` block is; run those tests if anything is uncommented.

### Pitfall 9: Overstating the clone/lock waiver
See F8. Waive on "no attribution evidence", name the dispatcher-path overhead (WINDOWS 35/61) as the first re-open candidate.

### Pitfall 10: Guards that scan the new pages
`entity_rename_guard` forbids old wrapper identifiers and module paths in any non-history `.md`; `treasurer_vocabulary_guard` rule 2 forbids the downstream fixture term. New pages (`treasurer.md`) and edited pages (`configuration.md`, `switching-providers.md`) are scanned; `MIGRATION.md`, `upgrading.md` and `migration-guide.md` are history files for both entity and legacy guards.

## Code Examples

### Removed-key refusal via `try_from` (verified in a scratch crate this session)
```rust
// Source: prototype compiled and run 2026-10-24 with serde 1 / serde_json 1 / serde_yaml 0.9;
// idiom precedent crates/paladin-core/src/platform/container/cost.rs:70 (try_from = "String").
use serde::de::{Deserializer, IgnoredAny};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "BattalionWire")]
pub struct Battalion {
    pub name: String,
    #[serde(default)]
    pub aegis: u32,
}

/// Marks a key as present for any value, `null` included.
fn present<'de, D: Deserializer<'de>>(d: D) -> Result<Option<()>, D::Error> {
    IgnoredAny::deserialize(d)?;
    Ok(Some(()))
}

#[derive(Deserialize)]
struct BattalionWire {
    name: String,
    #[serde(default)]
    aegis: u32,
    #[serde(default, deserialize_with = "present")]
    timeout_seconds: Option<()>,
    #[serde(default, deserialize_with = "present")]
    retry_policy: Option<()>,
    #[serde(default, deserialize_with = "present")]
    error_strategy: Option<()>,
}

impl TryFrom<BattalionWire> for Battalion {
    type Error = String;
    fn try_from(w: BattalionWire) -> Result<Self, String> {
        let mut found = Vec::new();
        if w.timeout_seconds.is_some() { found.push("`timeout_seconds` (use `aegis.timeout.run_timeout`)"); }
        if w.retry_policy.is_some() { found.push("`retry_policy` (use `aegis.retry`)"); }
        if w.error_strategy.is_some() { found.push("`error_strategy` (use `aegis.on_error`)"); }
        if found.is_empty() { Ok(Battalion { name: w.name, aegis: w.aegis }) }
        else { Err(format!("removed in v0.11: {}; see MIGRATION.md 10.1 M-B-06", found.join(", "))) }
    }
}
```
Observed output: absent key and an unrelated unknown key load; `timeout_seconds: null` fails; all three keys together fail naming all three; the same wire works through `serde_yaml` and nested inside an outer struct; `Serialize` output has no removed keys; a struct literal `Battalion { name, aegis }` still compiles. In the real code the wire type and strings live in `legacy_keys.rs`, `Result<Self, String>` may instead use a small `thiserror` error, and the `see MIGRATION.md` text cites the final §10.1 row id.

### Widened register window (script and ci.yml)
```bash
# Source: simulated this session on a scratch copy; exit 0 with rows under "## 10.2".
awk '/^## (9|10)\.2 /{flag=1; next} /^## (9|10)\.3 /{flag=0} flag' "${MIGRATION_FILE}" \
```

### Ledger writes (sequential)
```bash
G="node .claude/gsd-core/bin/gsd-tools.cjs"
$G windows status                                   # note total_count (79 today)
$G windows append --kind todo --phase 46 \
  --file crates/paladin-battalion/src/engine/superstep.rs \
  --description "v2 debt: oversized service files ... (measured wc -l ...)"
$G windows waive 80 "Waived at Phase 46 (D-18): ..."  # reason text is the measured waiver
# repeat for the clone/lock line (waive) and the allowlist drift (fixed 82, no reason argument)
$G windows status                                   # expect waived +2, fixed +1
```

### Dependency guard core (shape, not final)
```python
# Source: shape of scripts/check-crate-names.sh (python3 tomllib, bidirectional set check).
import sys, tomllib
CORE = {"serde","serde_json","uuid","chrono","thiserror","async-trait","tokio","sha2",
        "blake3","petgraph","murmur3","url","regex","futures"}
PORTS = {"paladin_core","async-trait","serde","thiserror","uuid","chrono","tokio",
         "serde_json","futures","log","md5","mime_guess"}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| MIGRATION.md titled for one release, rows appended per phase | Release-neutral title, a part per upgrade (§10 over §9) | This phase (D-01) | Every gate keyed to `## 9.x` headings must learn `10.x`. |
| Removed config keys silently ignored (ADR-0059 D-00c) | Named refusal at load (D-23) | This phase | Operator-visible; relaxing later is another §10.1 row. |
| Dependency allowlist as a prose figure (6 / 7, then 14 / 11) | Mechanical guard against ADR-0015's measured set (14 / 12) | This phase (D-21) | Drift fails CI instead of ageing. |
| "Reserved" Treasurer name (ADR-0050) | A built officer with ledger, admission, halt and pacing (ADRs 0052-0058) | Phases 38-43 | `adr-index.md` row 0050 is stale. |

**Deprecated/outdated:** the "ten/eleven crates" wording across the release docs; `ConclaveConfig.timeout_seconds` and the three `BattalionConfig` keys (removed in Phase 44).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | GitHub renders `## 10.2 Rust API changes (compile-affecting; the X-10 register)` with anchor `#102-rust-api-changes-compile-affecting-the-x-10-register`, same slug rule as the existing `#92-...` links | CURR-23 book pages | Dead anchors from `upgrading.md` and CHANGELOG links; web links are not link-checked, so it would ship silently. Verify once the file is on `main`. |
| A2 | `make api-surface` shows no diff for D-23 because a `try_from` container keeps the `Deserialize` impl | D-23 | A surprise diff means `make api-surface-update` plus a CHANGELOG entry. Run it in the D-23 plan. |
| A3 | Recommended root `[0.10.1]` heading `2026-09-21` is what the owner wants (D-13 leaves it to the planner's check) | F12 | Cosmetic; evidence (tag commit, run, crates.io timestamps) supports it. |
| A4 | Fixing the same two heading dates in the eleven crate changelogs is in scope | F12 | Scope creep if the owner wants only the root; trivial to drop. |
| A5 | `log` entered `paladin-ports` after ADR-0015's 2026-08-06 measurement (the ADR lists eleven without it) | F7 | The ADR text alone proves "not in the measurement"; the commit that added it cannot be dated from history (manifest shows as a new file in `998ae11e`). State only what is proven. |
| A6 | New relative links in `treasurer.md`/`SUMMARY.md` resolve under `mdbook-linkcheck` (not installed here) | CURR-23 | CI `docs` workflow fails; mitigate by checking each relative target exists with `test -f`. |
| A7 | web3sec's live fact in G-4 (DeepSeek `/models` lists `deepseek-flash`, not `deepseek-v4-flash`, as of 2026-10-24) is taken from the handoff, not re-run (needs a DeepSeek credential). G-7's call counts and G-2's `Display` prefixes were re-verified in source. | G-4 | A wrong statement on the Switching-providers page. Date the note ("as of 2026-10-24") and attribute it to the live listing. |
| A8 | The Phase 30 `VALIDATION.md` (`draft`, `nyquist_compliant: true`) is truly compliant | CURR-24 | Reconciliation may downgrade it; that is allowed, record it. |

## Open Questions

1. **Per-crate changelog headings.** Should the eleven crate `CHANGELOG.md` files get the same `[0.10.0]` and `[0.10.1]` date corrections as the root?
   - Known: all eleven carry `2026-09-10` and `2026-09-20`; CURR-22 and D-13 name only the root.
   - Recommendation: yes, one mechanical edit, same defect, ships in the `0.11.0` tarballs; flag it as an extra in the plan.
2. **"eleven-crate" in non-doc files** (`release.yml` comments, `publish-crates.sh:114`, `finalize-release-body.sh:243`, `check-deprecations.sh:49`). Outside the three named docs.
   - Recommendation: fix the comments only if the plan already touches those files; otherwise list them in the SUMMARY as noticed.
3. **`adr-index.md`** stops at ADR-0051 and its 0050 row says "not built this cycle".
   - Recommendation: add one task to refresh row 0050 and add 0052-0062 so the Treasurer page's ADR links have a navigable index; if declined, the page links blob URLs directly.
4. **Phase 28 stale sentence** "No WINDOWS.md row exists yet for the FAIL" in `28-VALIDATION.md`: correct it as part of D-15 (recommended).
5. **Open WINDOWS rows (13)** block `/gsd-ship`. Not Phase 46's, but Phase 47 will meet them; record in the Phase 46 SUMMARY.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | D-23, D-24, muster, guards | yes | 1.97.1 (MSRV 1.88 is a CI gate) | CI |
| cargo-audit, cargo-deny | `make security` | binaries present | not run (advisory DB needs network) | CI jobs |
| cargo-semver-checks | semver evidence | yes | 0.50.0 | CI |
| cargo-public-api + nightly-2026-09-20 | `make api-surface` | yes | 0.52.0 | CI |
| cargo-llvm-cov | 82 % coverage floor | no | - | CI `coverage` job is the evidence (as Phases 45-51) |
| shellcheck | `make lint-shell` | yes | system | none needed |
| python3 + PyYAML | `check-doc-config`, guard | yes | PyYAML importable | none needed |
| mdbook, mdbook-linkcheck | book build | no | CI pins 0.4.40 / 0.7.7 | CI `docs` workflow; locally `test -f` every relative link target |
| Docker | Docker-tier tests | `docker` binary exists; daemon not verified | - | Not needed for this phase (docs, serde, scripts) |
| `gh` CLI | release-run evidence | yes (authenticated through the proxy) | - | none needed after the evidence is recorded |
| crates.io API | publish proof | reachable through the proxy | - | recorded in the plan |

**Missing dependencies with no fallback:** none. **With fallback:** coverage, mdBook link check (CI).

## Validation Architecture

> Nyquist validation is enabled (`workflow.nyquist_validation` is absent in `.planning/config.json`, which means enabled).

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` (libtest, serde_json for fixtures), bash harnesses under `tests/scripts/*_test.sh`, repo guards as `cargo test --test <guard>`, script gates run by `make` |
| Config file | workspace `Cargo.toml` (`[[test]] consumer_surface_muster` has `required-features = ["llm-anthropic", "llm-deepseek"]`); `Makefile`; none dedicated for docs |
| Quick run command | `make check-migration-allowlist check-doc-config check-changelogs check-crate-names check-publish-order` and `make test-shell-guards` |
| Full suite command | `make clean-code && make api-surface && make check-gates && make check-doc-config && make check-doc-examples && make check-api-examples && make check-consumer-surface && cargo test --test legacy_removal_guard --test entity_rename_guard --test treasurer_vocabulary_guard && cargo test -p paladin-ai-core battalion && cargo test -p paladin-battalion --lib commander`; `make security` where the advisory DB is reachable |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CURR-22 | No present-tense "eleven"/"ten" crate count left in the three docs; "twelve" present; `paladin-eval` rows present in both tables | grep gate (plan verify) | `! grep -nwi 'eleven' docs/src/contributing/development-setup.md docs/src/appendix/release-recovery.md` (release-automation allows the dated historical lines only) and `grep -c 'paladin-eval' docs/src/appendix/release-automation.md` is at least 3 | Wave 0 (write the one-liners in the plan) |
| CURR-22 | CHANGELOG heading dates equal tag commit dates | script | `test "$(git log -1 --format=%cs v0.10.0)" = "$(grep -m1 '^## \[0.10.0\]' CHANGELOG.md \| grep -o '[0-9-]\{10\}')"` and the same for 0.10.1 | Wave 0 |
| CURR-23 | Treasurer page exists, in `SUMMARY.md`, relative links resolve | script | `test -f docs/src/architecture/treasurer.md && grep -n 'architecture/treasurer.md' docs/src/SUMMARY.md`; for each `](...)` relative target `test -f` | Wave 0 |
| CURR-23 | Every public `treasurer:` key appears in the reference table | script (one-off, in plan verify) | extract field names from `src/config/treasurer.rs` public structs and grep each in `configuration.md`; all env names from `read_env::<...>("APP_TREASURER_...")` present | Wave 0 |
| CURR-23 | Doc YAML parses; example config still loads | gate + tests | `make check-doc-config`; `cargo test -p paladin-ai --lib config::web_server` and the `agent_host` config.example test (names to confirm) | exists |
| CURR-23 | MIGRATION §10 rows keep identity; gates green | script + gate | key-set diff of `M-B-\d+`, `DR-\d+`, first-column `crate \| type` before versus after; `make check-migration-allowlist`; `make test-shell-guards`; `grep -c TBD MIGRATION.md` equals 0 | Wave 0 (diff script in plan) |
| CURR-23 | Allowlist harness reads a `Y` row under `## 10.2` | shell harness | `bash tests/scripts/check-migration-allowlist_test.sh` (new case) | extend existing |
| CURR-23 | Muster pins hold (D-04) and are red-then-green | compile-only test | `make check-consumer-surface` (`cargo test --test consumer_surface_muster`) | extend existing |
| CURR-23 | New pages obey vocabulary and rename guards | tests | `cargo test --test treasurer_vocabulary_guard --test entity_rename_guard --test legacy_removal_guard` | exists |
| CURR-23 | Book builds, links resolve | CI only | `mdbook build docs/` (CI `docs` workflow) | CI |
| D-23 | Each removed key, alone and combined, fails naming the key and replacement; `null` fails; v0.11 and unknown-key documents load; CLI `timeout_seconds` YAML still loads | unit | `cargo test -p paladin-ai-core battalion::` (names set by the plan), CLI test in `src/application/cli/config` | Wave 0: new tests written first (red) |
| D-23 | No new public-surface diff | gate | `make api-surface` | exists |
| D-24 | Maneuver ignored-policy helper; rustdoc zero warnings | unit + gate | `cargo test -p paladin-battalion --lib commander::`; `make doc-check` | Wave 0 for the helper test |
| CURR-24 | No `draft` among the eight files; Phase 28 compliant; each file has audit table and Gaps section | grep | `grep -H '^status:\|^nyquist_compliant:' .planning/milestones/v0.10.0-phases/{22,24,28,29,30,34,36,36.1}-*/*-VALIDATION.md` and `grep -L 'Validation Audit' ...` is empty | Wave 0 |
| CURR-25 | Guard passes on the tree, fails on a planted surplus and on a removal | script + harness | `./scripts/check-dependency-allowlist.sh`; `bash tests/scripts/check-dependency-allowlist_test.sh`; `make check-gates` | Wave 0 (new files) |
| CURR-25 | Three ledger rows exist and counts agree | tool | `node .claude/gsd-core/bin/gsd-tools.cjs windows status` (waived 39, fixed 30, open unchanged at 13, total 82) | tool exists |
| CURR-25 | STATE and ROADMAP rows point at the ledger ids | grep | `grep -n 'WINDOWS' .planning/STATE.md` on lines 796, 797, 815 | Wave 0 |

### Sampling Rate
- **Per task commit:** the quick command above plus the guard or test the task touched; shellcheck (`make lint-shell`) on any script change.
- **Per wave merge:** the full suite command (without `make security` if the sandbox has no advisory DB; CI runs it).
- **Phase gate:** full suite green, then CI for `coverage` (82 % floor, ADR-0006), `semver`, `docs` and `consumer-surface` before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `scripts/check-dependency-allowlist.sh` and `tests/scripts/check-dependency-allowlist_test.sh` (CURR-25)
- [ ] The new `legacy_keys.rs` tests, written red first (D-23)
- [ ] The MIGRATION identity-diff helper and the CURR-22 grep one-liners (kept in plan verify blocks, not committed unless a plan chooses to)
- [ ] The extra case in `tests/scripts/check-migration-allowlist_test.sh` for a `## 10.2` register
- Framework install: none.

### Manual-only items (with justification)
| Item | Why manual |
|------|------------|
| Rendered mdBook navigation and link check | Needs mdbook and mdbook-linkcheck (CI `docs` workflow); not installed in the sandbox. |
| CI coverage figure after D-23 | No `cargo-llvm-cov` locally; CI `coverage` job is the evidence. |
| web3sec's re-pin and return report | Belongs to the consumer after Phase 47 (D-25). |
| Owner call on Open Questions 1-3 | Scope decisions. |

## Security Domain

`security_enforcement` is absent from `.planning/config.json`, so it is treated as enabled.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | No auth surface is added; the docs describe existing API-key/tenant behaviour. |
| V3 Session Management | no | - |
| V4 Access Control | documentation only | The Treasurer page and config reference restate that no role bypasses an allowance and that tenant scoping applies (existing behaviour, `configuration.md`). |
| V5 Input Validation | yes | D-23 is input validation of stored configuration: explicit allow of unknown keys, explicit refusal of named removed keys, no value echoed. Existing `treasurer:` structs already `deny_unknown_fields`. |
| V6 Cryptography | no | No new cryptography. The webhook HMAC signing description is existing and unchanged. |
| V14 Configuration | yes | Treasurer reference must state the secret is env-only, `${VAR}` is not expanded, and the secret is redacted from `Debug` and `Serialize`; `config.example.yml` must keep placeholders, never real keys. |
| Supply chain | yes | D-21 guard blocks a new dependency entering the two inward-facing crates; no package added. |

### Known Threat Patterns for this phase

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Silently unbounded run after a v0.10 config is loaded under v0.11 (a removed timeout key ignored) | Denial of service / Tampering (by omission) | D-23 refusal with a message naming the key and replacement. |
| Credential or value leaking into an error message | Information disclosure | D-23 messages name keys only; never format the offending value. Review per `security.instructions.md`. |
| Provider SDK, transport client, storage driver or web framework creeping into `paladin-core` / `paladin-ports` | Tampering with the architecture invariant | D-21 guard plus the dated ADR-0015 note; CI step. |
| Documentation instructing an operator to put a webhook secret in a YAML file | Information disclosure | Page and table say env-only, `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET`. |
| SSRF through the operator webhook URL | SSRF | Unchanged: the existing guard at write and send time; the docs only restate it (private addresses need `webhooks.allow_private: true`). DNS rebinding remains the documented limitation; do not claim otherwise. |

The manual credential-handling review applies to D-23 (error text) and to the new pages' examples. CodeQL is advisory only.

## Sources

### Primary (HIGH confidence)
- Live tree at `ac3a52e0`: `MIGRATION.md`, `CHANGELOG.md`, `Makefile`, `scripts/check-migration-allowlist.sh`, `scripts/check-crate-names.sh`, `scripts/publish-crates.sh`, `.github/workflows/{ci,docs}.yml`, `tests/{consumer_surface_muster,legacy_removal_guard,entity_rename_guard,treasurer_vocabulary_guard}.rs`, `tests/scripts/check-migration-allowlist_test.sh`, `src/config/treasurer.rs`, `config.example.yml`, `crates/paladin-core/src/platform/container/battalion/{mod,conclave}.rs`, `crates/paladin-battalion/src/{commander,campaign_service}.rs`, both guarded manifests, `docs/src/**` pages named above, ADRs 0015/0050/0059/0004, `.planning/{STATE,WINDOWS,ROADMAP,REQUIREMENTS}.md`, the eight archived VALIDATION files, `.claude/gsd-core/workflows/validate-phase.md`, `.claude/gsd-core/bin/lib/broken-windows.cjs`.
- Commands run: `git log`/`git diff -U0 v0.10.1..HEAD`, `gh run view 35659477719`, crates.io API `paladin-eval/0.10.1`, `gsd-tools query init.phase-op`, `gsd-tools windows status`, `make check-gates`, `make check-doc-config`, the awk simulation, and the serde prototype (scratch crate).

### Secondary (MEDIUM confidence)
- `46-FEEDBACK-web3sec.md` (the consumer's own report; line references re-verified, runtime facts cited not re-run).

### Tertiary (LOW confidence)
- None used for recommendations.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, no new packages; the serde pattern was compiled and run.
- Architecture: HIGH, the gate-coupling findings (F1, F5, F7) were reproduced or read at source.
- Pitfalls: HIGH for the guard and gate behaviours; MEDIUM for mdBook link behaviour (not runnable here).

**Research date:** 2026-10-24
**Valid until:** until the next commit that touches `MIGRATION.md`, `src/config/treasurer.rs`, either guarded manifest, or the guard tests (line numbers above are at `ac3a52e0`); otherwise 30 days.
