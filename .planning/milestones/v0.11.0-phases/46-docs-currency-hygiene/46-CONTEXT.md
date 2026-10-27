# Phase 46: Docs Currency & Hygiene - Context

**Gathered:** 2026-10-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 46 closes the v0.10.0 acceptance audit's docs debt (CURR-22), documents v0.11.0's own
surface — a Treasurer mdBook page, a Treasurer configuration reference and the v0.10.1 → v0.11.0
migration guide (CURR-23) — moves the seven archived v0.10.0 Nyquist `VALIDATION.md` files out of
`draft` and closes Phase 28's compliance flag (CURR-24), and dispositions the three "deferred to v2"
tech-debt lines with a written fix or waiver each (CURR-25). It carries three folded todos: the
Phase 44 code-review follow-ups, the rejection of removed v0.10 Battalion/Conclave config keys
(option B), and the web3sec single-bump input. It is the last phase before Phase 47 (the v0.11.0
crate release); every upstream feature phase (38–45, 48, 49, 50, 50.1, 51) is sealed, and the
downstream consumer's dogfood report against this tree is in hand.

Not in this phase: any new capability, any public-API change beyond the folded removed-keys
guard, any decomposition of oversized files, any new test written to fill a Nyquist gap, and the
release itself (Phase 47).

</domain>

<decisions>
## Implementation Decisions

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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### The downstream consumer's report (freeze input)
- `.planning/phases/46-docs-currency-hygiene/46-FEEDBACK-web3sec.md` — web3sec's 2026-10-24 dogfood
  report against `e28e43b5`: guide corrections G-1..G-7 (§2), muster gaps (§3), the deferred
  `warn!` suggestion (§4), the consumer's next steps (§5). Line numbers are at `e28e43b5`.
- `.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md` — the single-bump rule
  and the consumer's return list; stays pending (D-25).

### Migration register and its gates
- `MIGRATION.md` — §9 (v0.10 record, kept) and the v0.11 rows interleaved in §9.1/9.2/9.4/9.5/
  9.6/9.8 that move into §10 (D-01/D-02); §9.8 "Library consumers: v0.10.1 → v0.11.0" (DR-1..DR-21+).
- `.cargo/semver-checks-allowlist.toml` and `scripts/check-migration-allowlist.sh` — the
  `migration_row` set-equality that the row move must not break.
- `tests/consumer_surface_muster.rs` — the consumer-surface muster to extend per D-04.
- `docs/src/api-reference/upgrading.md`, `docs/src/api-reference/migration-guide.md` — the book
  pages rewritten / pointed per D-05.
- `docs/src/user-guides/switching-providers.md` — receives the G-4 DeepSeek listing note.
- `.planning/decisions/0059-legacy-clean-break-removal.md` — amended with a dated note by D-23.
- `.planning/decisions/0060-base-entity-rename.md`, `0061-per-model-parameter-support-policy.md`,
  `0062-reasoning-effort-fail-loud-policy.md` — the v0.11 clean-break records whose rows §10 indexes.
- `.planning/decisions/PROMOTION.md` — next free ADR number (0063); expected unchanged.

### Audit docs fixes
- `.planning/milestones/v0.10.0-MILESTONE-AUDIT.md` — `tech_debt` frontmatter (the three v2 lines,
  the Phase 28 re-run note) and the docs fixes CURR-22 closes.
- `docs/src/contributing/development-setup.md`, `docs/src/appendix/release-automation.md`
  (Trusted Publishing table, Credential History), `docs/src/appendix/release-recovery.md` — D-11/D-12.
- `CHANGELOG.md` — `[0.10.0]` heading (D-13); `[Unreleased]` for this phase's entries.

### Treasurer
- `docs/src/architecture/commissary.md` — the officer-page shape D-07 mirrors.
- `docs/src/getting-started/configuration.md` — "Treasurer allowances" (§ at ~389) and "Treasurer
  rate pacing (Cadence)" (~516) sections regrouped per D-09.
- `src/config/treasurer.rs` and `config.example.yml` (`treasurer:` block) — D-10 source of truth.
- `src/application/services/treasurer/` (`mod.rs`, `policy.rs`, `guard.rs`, `window.rs`,
  `derive.rs`, `evaluate.rs`) — the Rust surface the page documents.
- `.planning/decisions/0050-treasurer-reservation.md`, `0052-mid-run-treasurer-enforcement.md`,
  `0053-ledger-balance-model.md`, `0056-allowance-admission-model.md`,
  `0057-mid-run-halt-contract.md`, `0058-rate-pacing-cadence.md` — the design records the page links.
- `docs/src/api-reference/platform-api.md` — the `429 allowance_exhausted` contract the page links.

### Nyquist
- `.planning/milestones/v0.10.0-phases/{22,24,28,29,30,34,36,36.1}-*/<N>-VALIDATION.md` — the eight
  files edited in place (D-14..D-17); `28-VALIDATION.md` is the only non-template one.
- `.claude/gsd-core/workflows/validate-phase.md` — the mechanism (`/gsd-validate-phase N`).
- `.planning/WINDOWS.md` rows 35 and 61 — the accepted tracing-overhead deviation Phase 28 cites.

### Debt lines
- `.planning/decisions/0015-core-ports-dependency-allowlist.md` — the fix D-20 records; the set
  D-21's guard pins.
- `crates/paladin-core/Cargo.toml`, `crates/paladin-ports/Cargo.toml` — the guard's inputs.
- `benches/engine_benchmarks.rs` — the bench evidence D-19 cites.
- `.planning/WINDOWS.md` — the ledger the three dispositions are written to (D-22); `gsd-tools
  windows` is the only writer.
- `.planning/STATE.md` deferred-items table — the three "Deferred to v2" rows and the Nyquist row
  rewritten to outcomes.

### Folded todos
- `.planning/todos/pending/2026-10-11-reject-removed-v0-10-battalion-config-keys.md` — D-23's
  constraints and solution sketch.
- `.planning/todos/pending/2026-10-11-phase-44-review-follow-ups.md` and
  `.planning/phases/44-legacy-clean-break-removal/44-REVIEW.md` / `44-REVIEW-FIX.md` — D-24's items.
- `tests/legacy_removal_guard.rs` — must stay green with D-23's new error strings.

### Roadmap and requirements
- `.planning/ROADMAP.md` Phase 46 entry — goal, four success criteria, the "Depends on" clause D-03
  marks satisfied.
- `.planning/REQUIREMENTS.md` CURR-22..CURR-25.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `docs/src/architecture/commissary.md` — the only existing officer concept page; its section
  order (concept, two responsibilities, type table, worked example, ADR links) is the template for
  `treasurer.md`.
- `configuration.md`'s existing key tables (key | type | default | env override) for
  `treasurer.allowance.*` and `treasurer.cadence.*` — the pricing table and the regrouped
  `## Treasurer` block reuse that exact column set.
- `MIGRATION.md` §9's section skeleton and row formats (§9.1 `M-B-xx` table, §9.2 `crate | type |
  change | action | Y/N` register, §9.8 DR table) — §10 copies them verbatim.
- `scripts/check-migration-allowlist.sh` and the other `scripts/check-*.sh` wired into `make
  check-gates` — the house shape for D-21's dependency-allowlist guard.
- `gsd-tools windows waive|fixed` — the only sanctioned writer for WINDOWS.md rows (D-22).
- The Nyquist auditor agent and `/gsd-validate-phase` — produce the per-task map; GSD's resolver
  finds archived phases without moving them.
- `tests/consumer_surface_muster.rs` `[[test]]` table with `required-features` — add the four
  D-04 pins beside the existing ones.

### Established Patterns
- Rows and allowlist entries are written in the same commit as the code they register (Phases
  38–51); Phase 46's only code change (D-23) follows it with a §10.1 row, an ADR-0059 dated note
  and a CHANGELOG line in one commit.
- Historical records are amended with dated notes, never rewritten (PROMOTION.md, ADR
  supersessions, PROJECT.md amendments) — D-02's pointers, D-11's parentheticals and D-16's
  frontmatter notes follow that rule.
- `make check-doc-config` parses every fenced YAML snippet under `docs/src`; new config examples
  must be real, loadable YAML.
- Sandbox has no container runtime and no `cargo-llvm-cov`; CI (coverage job) is the evidence
  for the 82 % floor after D-23 lands, as in Phases 45–51.

### Integration Points
- `docs/src/SUMMARY.md` Architecture section — one new entry (`treasurer.md`); no other nav change.
- `Makefile` `check-gates` target — gains the D-21 guard.
- `crates/paladin-core/src/platform/container/battalion/{mod.rs,conclave.rs}` — D-23's per-key
  deserializer guards on `BattalionConfig` / `ConclaveConfig`.
- `crates/paladin-battalion/src/commander.rs`, `crates/paladin-core/.../battalion/mod.rs`,
  `src/infrastructure/resilience/circuit_breaker.rs` — D-24's rustdoc and one-time `warn!`.
- `.planning/ROADMAP.md` Phase 46 "Depends on" line and `.planning/STATE.md` deferred table —
  bookkeeping commits at phase close.

</code_context>

<specifics>
## Specific Ideas

- The owner wants §10 to read like §9 did at the v0.10.0 close: every section substantive, no
  open marker, the consumer index as "a consumer-shaped index over register rows, not a new
  register" (the §9.8 wording).
- G-2's recipe is worth a short code block: read `message` from the `Paladin` / `Llm` /
  `Function` arms to keep the v0.10 text; `node_id` is the Paladin's name.
- G-3 is the kind of correction the owner cares most about: a consumer that only handles `Err`
  silently stops recording timeouts under `Absorb` — say both outcomes plainly.
- The `paladin-eval` link-date cell should be honest about its evidence class: "on or before
  2026-09-21 (proven by publish run 35659477719)", not a reported date nobody has.
- Waivers are measured statements, not apologies: cite today's line counts and the benches that
  now exist.

</specifics>

<deferred>
## Deferred Ideas

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

</deferred>

---

*Phase: 46-docs-currency-hygiene*
*Context gathered: 2026-10-24*
