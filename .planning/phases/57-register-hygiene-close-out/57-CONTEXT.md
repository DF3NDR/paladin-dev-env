# Phase 57: Register Hygiene Close-Out - Context

**Gathered:** 2026-11-04
**Status:** Ready for planning

<domain>
## Phase Boundary

Reconcile every planning register the milestone audit reads with the tree, so that
`/gsd-audit-milestone` for v0.11.1 can read `passed` rather than `tech_debt`:

- `.planning/WINDOWS.md` at `open_count: 0` (REG-02) — every open row fixed on evidence or waived
  with a structured owner/revisit reason.
- Zero undispositioned review warnings across the archived Phase 38-51 `NN-REVIEW.md` files, plus
  Phase 40's IN-01 and IN-04 (REG-05).
- SUMMARY `requirements-completed` frontmatter carrying STORE-02 (Phase 45) and REASON-02/03/05/06
  (Phase 51) (REG-01).
- The three `diagnosed` sessions in `.planning/debug/` archived as resolved (REG-03).
- A bounded human read of the Phase 40 judgment-tier prohibitions recorded, and the Phase 42
  subset recorded as accepted as-is (REG-04).
- The three `zip 3.0.0` RustSec exceptions re-checked against upstream with the check recorded
  and `make security` green (REG-06).

**The one code deliverable the discussion added:** WINDOWS rows 85-87 (the Phase 53 review's
three tenant-isolation defects in `thread_controller.rs` / `principal.rs`) are **fixed in this
phase**, not waived — the roadmap's "little code" scope note bends so the `open_count: 0` exit
criterion is honest. Any further live defect the review sweep or the judgment read surfaces is
handled by the same severity rule (D-14, D-21).

**Measured state at discuss time (2026-11-04), which differs from the roadmap text:**
`windows status` reads `open_count: 13` — rows **56, 64, 65, 66, 68, 69, 77, 79, 83, 84, 85, 86,
87**. Rows 58, 62, 63, 67 and 73 named in the roadmap are already closed by Phases 53-55; rows 83-87
were minted by Phases 52-53 and are in scope under success criterion 1's "any row minted by
Phases 52-56".

Out of scope (deferred, see `<deferred>`): the `check-gates` tracked-tree key-shape scan, the
Phase 54 advisory review follow-ups, the utoipa 5 → 6 migration, and all seven keyword-matched
pending todos.

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by the milestone open and Phases 52-56 — not re-asked)

- **D-00a: Gates.** As Phase 55/56 D-00a: 82 % workspace line coverage (ADR-0006); `make clean-code`
  (includes `make doc-check`), `make check-gates`, `make test-shell-guards`; `make api-surface`
  (an intentional public-surface change also gets `make api-surface-update` plus a CHANGELOG
  entry); `make security`, `make check-migration-allowlist`, `make check-version-strings`;
  `shellcheck`/`actionlint` for every script and workflow edit; the manual credential-handling
  review in `.github/instructions/security.instructions.md`; no `unwrap`/`expect`/`panic!` in
  library code; TDD — every fix is watched failing first and a defect's reproduction becomes its
  regression test.
- **D-00b: Semver.** As Phase 53 D-02 / Phase 55 D-00b: every public change is judged at
  `--release-type patch` against `0.11.0` for all twelve packages; the milestone stays `v0.11.1`,
  the allowlist stays at zero entries, §11.2 stays header-only; a change the gate flags is
  reshaped to additive or dropped; no `#[deprecated]` is added. The rows 85-87 fixes must be
  additive (new port method, read-side rule change, no persisted-schema break).
- **D-00c: Frozen records.** `MIGRATION.md` §9/§10 and every `[0.11.0]` changelog section are
  byte-frozen; behavioural changes go in `MIGRATION.md` §11.1 and `CHANGELOG.md` `[Unreleased]`.
- **D-00d: F5 bound (milestone open 2026-10-27).** The judgment-tier read covers the Phase 40
  subset only; Phase 42 is accepted as-is. Not widened here.
- **D-00e: Owner-accepted deferrals stand.** Rows 64-66 are accepted ADR-0057 contracts; row 77
  is the owner's Grok vendor-cost deferral (milestone decision B9); row 79's probe stays as run.
  None is re-litigated — they are waived under D-03.
- **D-00f: Branch state.** Phase 56 work sits on `claude/laughing-dirac-e0h2ax` unmerged; the
  maintainer's PR precedes building on it. Plans merge `main` back in before this phase's PR.

### WINDOWS rows and re-homing (REG-02)

- **D-01: Rows 85, 86 and 87 are fixed in this phase.** One code plan (or two) carries the three
  Phase 53 review defects with TDD regression tests, additive shape under the patch gate:
  - Row 85 (53-REVIEW WR-01): `visible_thread_page` in `crates/paladin-web/src/thread_controller.rs`
    skips-and-logs an `UnknownSchemaVersion` row in the list walk the way it already treats
    `Serialization`, so one newer-schema run row no longer turns every non-admin `GET /v1/threads`
    into a 500; consider making it hideable in `run_repo_error_for_scope`.
  - Row 86 (53-REVIEW WR-02): a new additive run-repository port method that reads exactly the
    newest run on a thread (`LIMIT 1`, no `limit + 1` decode), used by both the thread controller's
    latest-run lookup (`thread_controller.rs:746`) and `RunSubmissionService::ensure_thread_visible`
    (`src/application/services/run/submission.rs:277-293`), with contract tests across the
    in-memory, SQLite and Postgres adapters.
  - Row 87 (53-REVIEW WR-03): see D-02.
  Rows close as `fixed` via `windows fixed <id>` with the test names in the evidence file.
  — **Reversibility:** costly — the port method is public surface on `paladin-ports`; removing
  it later is a semver break.
- **D-02: Thread ownership derives from the earliest attributed run.** `RunReadScope::permits_thread`
  (`crates/paladin-core/src/platform/container/principal.rs:285-326`) and its callers stop treating
  "the tenant of the latest run" as the owner: ownership is the first run on the thread that
  carries `submitted_by`; an Admin fork or an unattributed (schedule-fired, `submitted_by: None`)
  run never moves it. ADR-0063 (`.planning/decisions/0063-thread-and-job-tenant-visibility.md`) and
  `docs/src/user-guides/parley-and-chronicle.md` are amended to state the new rule; the Phase 53
  SPEC AC-04 wording that described the latest-run rule is superseded by this decision.
  — **Reversibility:** one-way — this is an observable authorization semantic: once v0.11.1 ships,
  a thread a creator can see because of this rule would become hidden again if reverted, and
  ADR-0063 is the published contract downstream consumers (web3sec) read.
- **D-03: Open rows with no fix in this milestone are waived with one structured reason shape.**
  Rows 64, 65, 66, 77 and 79 (and any row D-14/D-21 mint and do not fix) are closed with
  `windows waive <id> "<reason>"` where the reason follows exactly:
  `Re-homed: owner=<person or phase>, revisit=<date or condition>, basis=<ADR / evidence pointer>. <one-line rationale>`.
  No tooling change; the audit can grep `Re-homed:` to find every re-homed row. Owners: rows 64-66
  → the Treasurer maintainers / ADR-0057 closing conditions (next feature milestone roadmap);
  row 77 → owner (DF3NDR), revisit when the Grok cost fields are parsed (ADR-0061 visibility rule);
  row 79 → owner, revisit = an operator-machine seed-probe run before the next seed change.
- **D-04: Rows 83 and 84 close as `fixed` on the CI `Workflow Lint` run id.** The plan reads the
  `Workflow Lint` (job id `actionlint`, `ci.yml:92-102`) result on the PR #61 merge commit
  `84686acfb` (and, for the 55-01 step, the latest `main` run) through the GitHub API, records the
  run id(s) in `57-REGISTER-EVIDENCE.md`, and marks both rows fixed on that pointer. No local
  actionlint install is required.
- **D-05: Rows 56, 68 and 69 close as `fixed` on already-recorded evidence.** Row 56 on the
  55-CORRECTNESS-EVIDENCE rustdoc measurement (`cargo doc -p paladin-ai-core --no-deps` prints 0
  warnings; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` exits 0),
  re-run on this phase's head and recorded; rows 68/69 on Phase 47's `make security` exit 0 and
  the `Coverage` job's 92.74 % on `70657919` (plus Phase 56's local 92.79 % / CI run `41703614636`),
  cited by run id in the evidence file.

### Review-warning disposition (REG-05)

- **D-10: One register plus an archive pointer.** `57-REVIEW-DISPOSITIONS.md` lists every warning
  (phase, id, title, disposition, evidence pointer) across the fourteen archived
  `.planning/milestones/v0.11.0-phases/*/NN-REVIEW.md` files. Each archived REVIEW.md gains one
  appended `## Dispositions (Phase 57)` section that points at the register; everything above
  that section stays byte-stable.
- **D-11: Vocabulary `closed` / `waived` / `open`, evidence required.** `closed` needs a pointer to
  the commit, test or requirement that fixed it (e.g. Phase 40 WR-01 → TENANT-04,
  `53-TENANT-EVIDENCE.md`; WR-02 → TENANT-05; WR-03/WR-04 → TENANT-06; IN-04 → TENANT-07;
  IN-01 → TENANT-09; the REVIEW-FIX reports for Phases 40, 42, 44, 46, 48, 50, 50.1). `waived`
  needs a one-line reason and an owner. `open` is not allowed at seal. Warnings that were fixed
  but never recorded (the 22 warnings in Phases 38, 41, 43, 45, 47 and 51, which have no
  REVIEW-FIX report) get their evidence hunted in the tree, never asserted.
- **D-12: Sweep scope.** All 47 warnings plus Phase 40's IN-01 and IN-04. The other 72 info items
  across the fourteen files are not swept (the audit does not read them; the roadmap does not
  name them). Phases 52-56's own REVIEW files are not in this sweep (their phases dispositioned
  them at their own seals).
- **D-13: Evidence hunting is read-only on the archive.** Prior-phase SUMMARYs, evidence files,
  requirement ids and `git log -S` are the sources; no archived VERIFICATION/SUMMARY is rewritten
  to manufacture a closure.
- **D-14: A warning found still live in the tree mints a WINDOWS row, then fix or waive by severity.**
  A correctness or security defect is fixed in this phase beside rows 85-87 (TDD,
  additive, D-00a/D-00b); anything else is waived under D-03's reason shape. Both outcomes keep
  `open_count` at 0 at seal. The register row then reads `closed` (pointer to the fix) or
  `waived` (pointer to the WINDOWS row).

### Phase 40 judgment-tier read (REG-04)

- **D-20: Claude prepares the worksheet; the owner confirms at UAT.** A plan produces
  `57-JUDGMENT-READ.md`: one row per prohibition (20, from the `### Prohibitions` table in
  `40-VERIFICATION.md:165-197`) with the LLM-judge verdict, the evidence pointer (named test, or
  `file:line` from a fresh read), and a confirm/reject box. The 11 test-backed rows are pre-filled
  from the named tests (re-run in this phase to confirm they still pass); the 9 code-reading rows
  are re-read by Claude on the Phase 40 commit range with fresh `file:line` evidence. The UAT step
  is the owner's confirm-or-reject per row — that is the "bounded human read".
- **D-21: The credential-handling review is repeated over Phase 40's own diff range.** Claude
  re-runs the `.github/instructions/security.instructions.md` checklist (redaction before
  truncation, no `Debug` on key-carrying config, no redirect-following on credential clients, no
  key interpolation in logs, no key value in any response body, persisted row, doc or example)
  over the Phase 40 commits only (the range `40-REVIEW.md` names as `629ef660..` through the 40-06
  seal), records `file:line` evidence in the worksheet, and the owner confirms that row. A rejected
  verdict (a MUST NOT found violated) mints a WINDOWS row; a credential or tenant-isolation
  violation is fixed in this phase (TDD, additive), anything else is waived under D-03.
- **D-22: Recording.** `57-JUDGMENT-READ.md` holds the full table. The archived
  `40-VALIDATION.md` `## Gaps` row for the "~20 judgment-tier prohibitions" is amended in place to
  "human read recorded <date>, see `.planning/phases/57-register-hygiene-close-out/57-JUDGMENT-READ.md`";
  `42-VALIDATION.md` gets a matching gap-row note: "accepted as-is by owner decision (milestone
  open 2026-10-27, inventory F5)". `40-VERIFICATION.md` and `42-VERIFICATION.md` are **not**
  rewritten (their `human_needed` status and verdict tables stay as the historical record).

### `zip 3.0.0` RustSec exceptions (REG-06)

- **D-30: Re-date, record the check, route the lift to the next milestone.** Measured 2026-11-04
  (crates.io dependency listing): `utoipa-swagger-ui` 10.0.2 depends on `zip ^9.0` (build) — the
  rows' `revisit_condition` is **met** — but also on `utoipa ^6.0.0`, while the tree is on `utoipa`
  5.5.0, `utoipa-axum` 0.2 and `utoipa-swagger-ui` 9.0.2 (`crates/paladin-web/Cargo.toml:38-44`).
  Lifting the rows is therefore a utoipa 5 → 6 major migration (derive macros, `utoipa-axum`, the
  committed `crates/paladin-web/openapi.json` snapshot), not a build-dependency bump, and it is
  not taken in a cleanup patch. The three rows stay, with their build-time-only scope and
  never-calls-`extract` compensating control unchanged.
  — **Reversibility:** reversible — the migration can be taken in any later minor release.
- **D-31: New review date and revisit condition.** `review_date` moves to the first planning day
  of the next milestone, expressed as a condition plus a fallback calendar date no later than
  `2027-03-31`. `revisit_condition` changes from "upstream ships zip >= 9" (now true) to "the
  utoipa 6 migration (utoipa, utoipa-axum, utoipa-swagger-ui 10.x) lands in a minor release".
  `why_not_fixable` is rewritten to record that the upstream condition was met on 2026-11-04
  (`utoipa-swagger-ui 10.0.2 → zip ^9.0, utoipa ^6.0.0`) and why it was not taken in v0.11.1.
- **D-32: Where the check is recorded.** In the three rows' fields (D-31) and as a dated entry in
  the register's running history paragraph (`SECURITY-EXCEPTIONS.md:61`, the paragraph that records
  the 2026-10-19 addition). The `.cargo/audit.toml` and `deny.toml` comment blocks for
  RUSTSEC-2026-0367/0368/0369 are mirrored to the same wording; `scripts/check-advisory-register.sh`
  and `make security` must pass on the head. No separate evidence file for REG-06 beyond a line in
  `57-REGISTER-EVIDENCE.md` citing the `make security` run.

### Evidence, SUMMARY frontmatter and debug sessions (REG-01, REG-03, cross-cutting)

- **D-40: Three phase files.** `57-REGISTER-EVIDENCE.md` carries every WINDOWS disposition with
  its evidence pointer (CI run ids, test names, measurements), the debug-archive moves, the SUMMARY
  frontmatter edits and the `make security` run. `57-REVIEW-DISPOSITIONS.md` (D-10) and
  `57-JUDGMENT-READ.md` (D-20) stay separate files because the audit reads them by name.
- **D-41: REG-01 — add each id to the plan that finished it.** STORE-02 goes on
  `45-01-SUMMARY.md` `requirements-completed` (the plan whose frontmatter claims it; closed by
  45-UAT test 1 and the CI contract-suite run). For Phase 51, each of REASON-02/03/05/06 is added
  to `requirements-completed` of the **last** SUMMARY that progressed it per the
  `51-VERIFICATION.md` requirements table (`:139-143`: REASON-02 → plans 01, 03-06, 11-13;
  REASON-03 → 02-05, 10-12, 14; REASON-05 → 03, 05, 07, 09, 12, 15; REASON-06 → 01, 03, 05,
  11-14). The existing `requirements-progress` lines are left untouched. The audit's frontmatter
  scan then finds every id.
- **D-42: REG-03 — archive, do not delete.** The three sessions (`kimi-fallback-profile-stale.md`,
  `openai-max-completion-tokens.md`, `vendor-token-usage-fidelity.md`) move to
  `.planning/debug/resolved/` with frontmatter `status: resolved` and a `resolved:` line citing the
  WINDOWS row that closed each (76, 75, 74 respectively, all `fixed`), following the debug
  workflow's `.planning/debug/resolved/{SLUG}.md` convention.

### Claude's Discretion

- Plan ordering and count; whether rows 85-87 are one plan or two (85+86 mechanical, 87 semantic).
- The exact `windows waive` reason text per row, within D-03's shape.
- The layout of the three Phase 57 files, provided the audit can find each warning, prohibition
  and WINDOWS row by id.
- How the `Workflow Lint` run id is retrieved (GitHub MCP tools; a check-run read on the merge
  commit) and what to do if the API refuses — record the refusal and the PR #61 required-context
  evidence instead.
- Which `run_repo_error_for_scope` arm handles `UnknownSchemaVersion` and whether it becomes
  hideable (row 85).
- UAT shape for rows 85-87 beyond the regression tests (an HTTP tracer against SQLite is enough;
  no live Postgres is required in the sandbox).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and requirements
- `.planning/ROADMAP.md` — Phase 57 entry (goal, five success criteria, scope notes; note the
  open-row list there is stale, see `<domain>`)
- `.planning/REQUIREMENTS.md` — REG-01…REG-06 (lines 228-246) and the v0.11.1 decisions at open
- `.project/v0.11.1/01-carried-gaps-inventory.md` — rows E6, F2, F3, F4, F5, D8 (the inventory
  rows each requirement cites)
- `.project/v0.11.1/03-suggestions-and-scoping.md` — exit criteria and the F5 "bounded read" decision
- `.planning/PROJECT.md` §Current Milestone — decisions at milestone open (F5, B9, D8)

### WINDOWS register and its evidence
- `.planning/WINDOWS.md` — the register; read via `gsd-tools windows status` (13 open rows)
- `.planning/milestones/v0.11.0-MILESTONE-AUDIT.md` — tech_debt inventory with owners for rows
  56, 64-66, 68/69, 77, 79; the F2 hygiene finding; integration warnings G1/G2
- `.planning/phases/53-tenant-isolation-closure/53-REVIEW.md` — WR-01…WR-03 (rows 85-87), with the
  proposed fixes
- `.planning/phases/53-tenant-isolation-closure/53-TENANT-EVIDENCE.md` — closes Phase 40 WR-01…04
- `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-CORRECTNESS-EVIDENCE.md`
  §Carry-forward — row 56 rustdoc measurement; rows 83/84 and 85-87 hand-off text
- `.planning/phases/52-ci-release-pipeline-repair/52-REVIEW.md` and the 52 SUMMARYs — rows 83/84
  provenance (A-52-03; 52-01 and 52-06 `ci.yml` edits)
- `.planning/phases/56-dogfood-findings/56-CONTEXT.md` §deferred and the 56 SUMMARYs — any row
  minted or closed by Phase 56
- `.planning/decisions/0057-mid-run-halt-contract.md` — basis for waiving rows 64-66
- `.planning/decisions/0061-per-model-parameter-support-policy.md` — basis for row 77's revisit
- `.planning/decisions/0063-thread-and-job-tenant-visibility.md` — the thread-ownership contract
  D-02 amends
- `docs/src/user-guides/parley-and-chronicle.md` — user-facing thread ownership text D-02 amends

### Review-warning sweep
- `.planning/milestones/v0.11.0-phases/*/NN-REVIEW.md` — the fourteen review reports (Phases 38,
  40, 41, 42, 43, 44, 45, 46, 47, 48, 50, 50.1, 51; 47 warnings, 74 info items)
- `.planning/milestones/v0.11.0-phases/*/NN-REVIEW-FIX.md` — fix reports for 40, 42, 44, 46, 48,
  50, 50.1 (evidence for `closed`)
- `.planning/milestones/v0.11.0-phases/40-tenant-identity-run-read-scoping/40-REVIEW.md` — WR-01…04,
  IN-01, IN-04
- `.planning/milestones/v0.11.0-phases/40-tenant-identity-run-read-scoping/40-REVIEW-FIX.md` — the
  four WR fixes with commits
- `.planning/phases/56-dogfood-findings/56-REVIEW.md` — the disposition-checker lessons (vacuous
  dispositions, duplicate cycles) that `57-REVIEW-DISPOSITIONS.md` must not repeat

### Judgment-tier read
- `.planning/milestones/v0.11.0-phases/40-tenant-identity-run-read-scoping/40-VERIFICATION.md` —
  §Prohibitions table (lines 165-197), the `human_verification` list, the diff base
- `.planning/milestones/v0.11.0-phases/40-tenant-identity-run-read-scoping/40-VALIDATION.md` —
  the `## Gaps` row D-22 amends
- `.planning/milestones/v0.11.0-phases/40-tenant-identity-run-read-scoping/40-06-SUMMARY.md` — the
  original credential-handling review record
- `.planning/milestones/v0.11.0-phases/42-mid-run-halt-sse-terminal-status/42-VERIFICATION.md` and
  `42-VALIDATION.md` — the Phase 42 subset recorded as accepted as-is
- `.github/instructions/security.instructions.md` — the manual credential-handling checklist D-21
  re-runs

### Security exceptions
- `SECURITY-EXCEPTIONS.md` — rows RUSTSEC-2026-0367/0368/0369 (lines 136-173) and the history
  paragraph (line 61)
- `.cargo/audit.toml` and `deny.toml` — mechanical mirrors whose comments D-32 updates
- `scripts/check-advisory-register.sh` — enforces register / audit.toml / deny.toml agreement
- `crates/paladin-web/Cargo.toml` — `utoipa = "5"`, `utoipa-axum = "0.2"`,
  `utoipa-swagger-ui = { version = "9", features = ["axum", "vendored"] }`

### SUMMARY frontmatter and debug sessions
- `.planning/milestones/v0.11.0-phases/45-rustfs-swap-platform-observability-deviations/45-01-SUMMARY.md`
  and `45-VERIFICATION.md` — STORE-02 target and evidence
- `.planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/51-VERIFICATION.md` — the
  requirements table (lines 139-143) that names which plans progressed each REASON id
- `.planning/debug/kimi-fallback-profile-stale.md`, `.planning/debug/openai-max-completion-tokens.md`,
  `.planning/debug/vendor-token-usage-fidelity.md` — the three sessions to archive
- `.claude/gsd-core/workflows/debug.md` — the `.planning/debug/resolved/` convention

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `gsd-tools windows status | waive <id> <reason> | fixed <id>` — the only register mutation path;
  no direct edits to `.planning/WINDOWS.md`.
- `RunReadScope::permits` / `permits_thread` (`crates/paladin-core/src/platform/container/principal.rs`)
  with its unit tests — the single place thread visibility is decided; D-02 changes the input it
  is given (earliest attributed run) rather than the predicate.
- `run_repo_error_for_scope` and the `Serialization` skip-and-log arm in
  `crates/paladin-web/src/thread_controller.rs::visible_thread_page` — the pattern row 85 extends
  to `UnknownSchemaVersion`.
- `RunSubmissionService::ensure_thread_visible` (`src/application/services/run/submission.rs:277`)
  — the second caller of the latest-run lookup that row 86's port method replaces.
- Run-repository contract tests (`crates/paladin-storage/src/run/contract_tests.rs`) — the home
  for the new newest-run method's tests across in-memory, SQLite and Postgres.
- `53-TENANT-EVIDENCE.md`, `55-CORRECTNESS-EVIDENCE.md` — evidence-file shape to copy for
  `57-REGISTER-EVIDENCE.md`.
- `scripts/check-dogfood-dispositions.py` (Phase 56) — a checker pattern (and its reviewed
  weaknesses) if a never-vacuous check over `57-REVIEW-DISPOSITIONS.md` is wanted.

### Established Patterns
- Evidence pointers are CI run ids, commit hashes, named tests or `file:line`; a disposition
  without one is not accepted at seal (Phase 56 DOGF-01 checker lesson).
- Waivers carry the phase, the decision id and the basis in the reason text (rows 61, 67, 80, 81).
- Archived milestone files are appended to, not rewritten (Phase 46/55 precedent); D-10 and D-22
  follow this.
- Additive-only public surface under the patch gate (`cargo semver-checks` at `--release-type
  patch` vs `0.11.0`, 223 checks pass expected).

### Integration Points
- `paladin-ports` `RunRepositoryPort` (new newest-run method) → `paladin-storage` adapters →
  `paladin-web` thread controller and the application `RunSubmissionService`.
- `principal.rs` thread-ownership rule → `thread_controller.rs` gates → ADR-0063 and
  `parley-and-chronicle.md` text.
- `SECURITY-EXCEPTIONS.md` ↔ `.cargo/audit.toml` ↔ `deny.toml` via `check-advisory-register.sh`
  (part of `make security`).
- The milestone audit (`/gsd-audit-milestone`) reads WINDOWS `open_count`, REVIEW warnings,
  SUMMARY `requirements-completed`, VALIDATION gaps — every Phase 57 artifact targets one of those
  readers.

</code_context>

<specifics>
## Specific Ideas

- The structured waiver reason, verbatim shape:
  `Re-homed: owner=<who/phase>, revisit=<date or condition>, basis=<ADR/evidence>. <rationale>`.
- `57-JUDGMENT-READ.md` rows: `| # | Prohibition (plan) | LLM verdict | Evidence (test or file:line) | Human: confirm / reject | Note |`,
  pre-filled for the 11 test-backed items, fresh evidence for the 9 code-reading items, the
  credential-handling checklist as its own sub-table.
- `57-REVIEW-DISPOSITIONS.md` rows: `| Phase | Id | Title | Disposition | Evidence | Owner (if waived) |`,
  47 warning rows plus Phase 40 IN-01 and IN-04; totals line at the bottom must read `open: 0`.
- The `zip` rows' new `why_not_fixable` must name the measured facts: "utoipa-swagger-ui 10.0.2
  depends on zip ^9.0 (build) and utoipa ^6.0.0 (crates.io, checked 2026-11-04); the tree is on
  utoipa 5.5.0 / utoipa-axum 0.2; the lift is a utoipa 6 migration deferred to the next minor".

</specifics>

<deferred>
## Deferred Ideas

- **Tracked-tree API-key-shape scan in `check-gates`** — proposed at the Phase 56 close
  (STATE.md "Blockers/Concerns", after the 56-SECURITY key-literal find in
  `.project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md:53`). Deferred by
  owner decision; candidate for Phase 58's pre-tag hardening or the next milestone. The key
  rotation / scrub / history-purge decision stays a maintainer action.
- **Phase 54 advisory review follow-ups** (STATE.md Phase 54 close): WR-01 copy-paste Cargo
  snippets name non-existent features (`storage-sqlite`, `llm-openai`); WR-04 the Treasurer
  webhook `secret` placeholder is not refused; WR-05 Python < 3.11 traceback from the gate;
  IN-02 worker pods mount admin API keys they never use. Deferred by owner decision to the next
  doc pass / milestone.
- **utoipa 5 → 6 migration** (`utoipa`, `utoipa-axum`, `utoipa-swagger-ui` 10.x, regenerate
  `openapi.json`) — lifts RUSTSEC-2026-0367/0368/0369 for good. Next feature milestone, minor
  release (D-30).
- **A first-class `rehomed` WINDOWS status / `windows rehome` command** in gsd-tools — not needed
  while the structured reason (D-03) carries owner and date; revisit if the audit wants to count
  re-homed rows separately.
- **Phase 52-56 REVIEW warnings and the audit's integration warnings G1/G2** — not in this sweep;
  Phase 58's pre-tag audit reads them.

### Reviewed Todos (not folded)
Seven pending todos matched on keywords only; none is register work, all left pending:
- `2026-10-20-openai-responses-api-for-pro-tier.md` — out of v0.11.1 by milestone decision (G2).
- `2026-11-02-setup-check-first-run-output.md` — CLI output; DF-re-homed finding, next doc/CLI pass.
- `2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md` — config correctness; not a register item.
- `2026-11-03-devcontainer-install-cargo-llvm-cov.md` — devcontainer image; maintainer task.
- `2026-11-03-docs-deprecated-gpt-4-model-literal.md` — 109-site docs sweep; v0.11.1 ships the
  Quickstart line unchanged by owner decision.
- `2026-11-04-muster-pin-cadence-consumer-imports.md` — downstream-consumer test pins; Phase 58 or next.
- `2026-11-04-reasoning-budget-hint-for-deepseek-empty-completion.md` — adapter behaviour; next milestone.

</deferred>

---

*Phase: 57-register-hygiene-close-out*
*Context gathered: 2026-11-04*
