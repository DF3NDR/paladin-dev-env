# Phase 57: Register Hygiene Close-Out - Research

**Researched:** 2026-11-04
**Domain:** Planning-register reconciliation (WINDOWS ledger, review-warning dispositions, SUMMARY frontmatter, debug archive, human-read worksheet, RustSec exception register) plus one small additive Rust code deliverable (WINDOWS rows 85-87, tenant-isolation hardening in `paladin-web` / `paladin-core` / `paladin-ports` / `paladin-storage`).
**Confidence:** HIGH on registers, tooling and measured state; MEDIUM on how many archived warnings are still live (only a spot-check was possible).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

Carried forward (not re-asked):

- **D-00a: Gates.** As Phase 55/56 D-00a: 82 % workspace line coverage (ADR-0006); `make clean-code` (includes `make doc-check`), `make check-gates`, `make test-shell-guards`; `make api-surface` (an intentional public-surface change also gets `make api-surface-update` plus a CHANGELOG entry); `make security`, `make check-migration-allowlist`, `make check-version-strings`; `shellcheck`/`actionlint` for every script and workflow edit; the manual credential-handling review in `.github/instructions/security.instructions.md`; no `unwrap`/`expect`/`panic!` in library code; TDD — every fix is watched failing first and a defect's reproduction becomes its regression test.
- **D-00b: Semver.** As Phase 53 D-02 / Phase 55 D-00b: every public change is judged at `--release-type patch` against `0.11.0` for all twelve packages; the milestone stays `v0.11.1`, the allowlist stays at zero entries, §11.2 stays header-only; a change the gate flags is reshaped to additive or dropped; no `#[deprecated]` is added. The rows 85-87 fixes must be additive (new port method, read-side rule change, no persisted-schema break).
- **D-00c: Frozen records.** `MIGRATION.md` §9/§10 and every `[0.11.0]` changelog section are byte-frozen; behavioural changes go in `MIGRATION.md` §11.1 and `CHANGELOG.md` `[Unreleased]`.
- **D-00d: F5 bound (milestone open 2026-10-27).** The judgment-tier read covers the Phase 40 subset only; Phase 42 is accepted as-is. Not widened here.
- **D-00e: Owner-accepted deferrals stand.** Rows 64-66 are accepted ADR-0057 contracts; row 77 is the owner's Grok vendor-cost deferral (milestone decision B9); row 79's probe stays as run. None is re-litigated — they are waived under D-03.
- **D-00f: Branch state.** Phase 56 work sits on `claude/laughing-dirac-e0h2ax` unmerged; the maintainer's PR precedes building on it. Plans merge `main` back in before this phase's PR.

WINDOWS rows and re-homing (REG-02):

- **D-01: Rows 85, 86 and 87 are fixed in this phase.** One code plan (or two) carries the three Phase 53 review defects with TDD regression tests, additive shape under the patch gate:
  - Row 85 (53-REVIEW WR-01): `visible_thread_page` in `crates/paladin-web/src/thread_controller.rs` skips-and-logs an `UnknownSchemaVersion` row in the list walk the way it already treats `Serialization`, so one newer-schema run row no longer turns every non-admin `GET /v1/threads` into a 500; consider making it hideable in `run_repo_error_for_scope`.
  - Row 86 (53-REVIEW WR-02): a new additive run-repository port method that reads exactly the newest run on a thread (`LIMIT 1`, no `limit + 1` decode), used by both the thread controller's latest-run lookup (`thread_controller.rs:746`) and `RunSubmissionService::ensure_thread_visible` (`src/application/services/run/submission.rs:277-293`), with contract tests across the in-memory, SQLite and Postgres adapters.
  - Row 87 (53-REVIEW WR-03): see D-02.
  Rows close as `fixed` via `windows fixed <id>` with the test names in the evidence file. — Reversibility: costly — the port method is public surface on `paladin-ports`; removing it later is a semver break.
- **D-02: Thread ownership derives from the earliest attributed run.** `RunReadScope::permits_thread` (`crates/paladin-core/src/platform/container/principal.rs:285-326`) and its callers stop treating "the tenant of the latest run" as the owner: ownership is the first run on the thread that carries `submitted_by`; an Admin fork or an unattributed (schedule-fired, `submitted_by: None`) run never moves it. ADR-0063 (`.planning/decisions/0063-thread-and-job-tenant-visibility.md`) and `docs/src/user-guides/parley-and-chronicle.md` are amended to state the new rule; the Phase 53 SPEC AC-04 wording that described the latest-run rule is superseded by this decision. — Reversibility: one-way — this is an observable authorization semantic: once v0.11.1 ships, a thread a creator can see because of this rule would become hidden again if reverted, and ADR-0063 is the published contract downstream consumers (web3sec) read.
- **D-03: Open rows with no fix in this milestone are waived with one structured reason shape.** Rows 64, 65, 66, 77 and 79 (and any row D-14/D-21 mint and do not fix) are closed with `windows waive <id> "<reason>"` where the reason follows exactly: `Re-homed: owner=<person or phase>, revisit=<date or condition>, basis=<ADR / evidence pointer>. <one-line rationale>`. No tooling change; the audit can grep `Re-homed:` to find every re-homed row. Owners: rows 64-66 → the Treasurer maintainers / ADR-0057 closing conditions (next feature milestone roadmap); row 77 → owner (DF3NDR), revisit when the Grok cost fields are parsed (ADR-0061 visibility rule); row 79 → owner, revisit = an operator-machine seed-probe run before the next seed change.
- **D-04: Rows 83 and 84 close as `fixed` on the CI `Workflow Lint` run id.** The plan reads the `Workflow Lint` (job id `actionlint`, `ci.yml:92-102`) result on the PR #61 merge commit `84686acfb` (and, for the 55-01 step, the latest `main` run) through the GitHub API, records the run id(s) in `57-REGISTER-EVIDENCE.md`, and marks both rows fixed on that pointer. No local actionlint install is required.
- **D-05: Rows 56, 68 and 69 close as `fixed` on already-recorded evidence.** Row 56 on the 55-CORRECTNESS-EVIDENCE rustdoc measurement (`cargo doc -p paladin-ai-core --no-deps` prints 0 warnings; `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` exits 0), re-run on this phase's head and recorded; rows 68/69 on Phase 47's `make security` exit 0 and the `Coverage` job's 92.74 % on `70657919` (plus Phase 56's local 92.79 % / CI run `41703614636`), cited by run id in the evidence file.

Review-warning disposition (REG-05):

- **D-10: One register plus an archive pointer.** `57-REVIEW-DISPOSITIONS.md` lists every warning (phase, id, title, disposition, evidence pointer) across the archived `.planning/milestones/v0.11.0-phases/*/NN-REVIEW.md` files. Each archived REVIEW.md gains one appended `## Dispositions (Phase 57)` section that points at the register; everything above that section stays byte-stable.
- **D-11: Vocabulary `closed` / `waived` / `open`, evidence required.** `closed` needs a pointer to the commit, test or requirement that fixed it (e.g. Phase 40 WR-01 → TENANT-04, `53-TENANT-EVIDENCE.md`; WR-02 → TENANT-05; WR-03/WR-04 → TENANT-06; IN-04 → TENANT-07; IN-01 → TENANT-09; the REVIEW-FIX reports for Phases 40, 42, 44, 46, 48, 50, 50.1). `waived` needs a one-line reason and an owner. `open` is not allowed at seal. Warnings that were fixed but never recorded (the 22 warnings in Phases 38, 41, 43, 45, 47 and 51, which have no REVIEW-FIX report) get their evidence hunted in the tree, never asserted.
- **D-12: Sweep scope.** All 47 warnings plus Phase 40's IN-01 and IN-04. The other 72 info items across the fourteen files are not swept. Phases 52-56's own REVIEW files are not in this sweep.
- **D-13: Evidence hunting is read-only on the archive.** Prior-phase SUMMARYs, evidence files, requirement ids and `git log -S` are the sources; no archived VERIFICATION/SUMMARY is rewritten to manufacture a closure.
- **D-14: A warning found still live in the tree mints a WINDOWS row, then fix or waive by severity.** A correctness or security defect is fixed in this phase beside rows 85-87 (TDD, additive, D-00a/D-00b); anything else is waived under D-03's reason shape. Both outcomes keep `open_count` at 0 at seal. The register row then reads `closed` (pointer to the fix) or `waived` (pointer to the WINDOWS row).

Phase 40 judgment-tier read (REG-04):

- **D-20: Claude prepares the worksheet; the owner confirms at UAT.** A plan produces `57-JUDGMENT-READ.md`: one row per prohibition (20, from the `### Prohibitions` table in `40-VERIFICATION.md:165-197`) with the LLM-judge verdict, the evidence pointer (named test, or `file:line` from a fresh read), and a confirm/reject box. The 11 test-backed rows are pre-filled from the named tests (re-run in this phase to confirm they still pass); the 9 code-reading rows are re-read by Claude on the Phase 40 commit range with fresh `file:line` evidence. The UAT step is the owner's confirm-or-reject per row — that is the "bounded human read".
- **D-21: The credential-handling review is repeated over Phase 40's own diff range.** Claude re-runs the `.github/instructions/security.instructions.md` checklist (redaction before truncation, no `Debug` on key-carrying config, no redirect-following on credential clients, no key interpolation in logs, no key value in any response body, persisted row, doc or example) over the Phase 40 commits only (the range `40-REVIEW.md` names as `629ef660..` through the 40-06 seal), records `file:line` evidence in the worksheet, and the owner confirms that row. A rejected verdict (a MUST NOT found violated) mints a WINDOWS row; a credential or tenant-isolation violation is fixed in this phase (TDD, additive), anything else is waived under D-03.
- **D-22: Recording.** `57-JUDGMENT-READ.md` holds the full table. The archived `40-VALIDATION.md` `## Gaps` row for the "~20 judgment-tier prohibitions" is amended in place to "human read recorded <date>, see `.planning/phases/57-register-hygiene-close-out/57-JUDGMENT-READ.md`"; `42-VALIDATION.md` gets a matching gap-row note: "accepted as-is by owner decision (milestone open 2026-10-27, inventory F5)". `40-VERIFICATION.md` and `42-VERIFICATION.md` are **not** rewritten (their `human_needed` status and verdict tables stay as the historical record).

`zip 3.0.0` RustSec exceptions (REG-06):

- **D-30: Re-date, record the check, route the lift to the next milestone.** Measured 2026-11-04 (crates.io dependency listing): `utoipa-swagger-ui` 10.0.2 depends on `zip ^9.0` (build) — the rows' `revisit_condition` is **met** — but also on `utoipa ^6.0.0`, while the tree is on `utoipa` 5.5.0, `utoipa-axum` 0.2 and `utoipa-swagger-ui` 9.0.2 (`crates/paladin-web/Cargo.toml:38-44`). Lifting the rows is therefore a utoipa 5 → 6 major migration (derive macros, `utoipa-axum`, the committed `crates/paladin-web/openapi.json` snapshot), not a build-dependency bump, and it is not taken in a cleanup patch. The three rows stay, with their build-time-only scope and never-calls-`extract` compensating control unchanged. — Reversibility: reversible.
- **D-31: New review date and revisit condition.** `review_date` moves to the first planning day of the next milestone, expressed as a condition plus a fallback calendar date no later than `2027-03-31`. `revisit_condition` changes from "upstream ships zip >= 9" (now true) to "the utoipa 6 migration (utoipa, utoipa-axum, utoipa-swagger-ui 10.x) lands in a minor release". `why_not_fixable` is rewritten to record that the upstream condition was met on 2026-11-04 (`utoipa-swagger-ui 10.0.2 → zip ^9.0, utoipa ^6.0.0`) and why it was not taken in v0.11.1.
- **D-32: Where the check is recorded.** In the three rows' fields (D-31) and as a dated entry in the register's running history paragraph (`SECURITY-EXCEPTIONS.md:61`, the paragraph that records the 2026-10-19 addition). The `.cargo/audit.toml` and `deny.toml` comment blocks for RUSTSEC-2026-0367/0368/0369 are mirrored to the same wording; `scripts/check-advisory-register.sh` and `make security` must pass on the head. No separate evidence file for REG-06 beyond a line in `57-REGISTER-EVIDENCE.md` citing the `make security` run.

Evidence, SUMMARY frontmatter and debug sessions (REG-01, REG-03, cross-cutting):

- **D-40: Three phase files.** `57-REGISTER-EVIDENCE.md` carries every WINDOWS disposition with its evidence pointer (CI run ids, test names, measurements), the debug-archive moves, the SUMMARY frontmatter edits and the `make security` run. `57-REVIEW-DISPOSITIONS.md` (D-10) and `57-JUDGMENT-READ.md` (D-20) stay separate files because the audit reads them by name.
- **D-41: REG-01 — add each id to the plan that finished it.** STORE-02 goes on `45-01-SUMMARY.md` `requirements-completed`. For Phase 51, each of REASON-02/03/05/06 is added to `requirements-completed` of the **last** SUMMARY that progressed it per the `51-VERIFICATION.md` requirements table (`:139-143`: REASON-02 → plans 01, 03-06, 11-13; REASON-03 → 02-05, 10-12, 14; REASON-05 → 03, 05, 07, 09, 12, 15; REASON-06 → 01, 03, 05, 11-14). The existing `requirements-progress` lines are left untouched.
- **D-42: REG-03 — archive, do not delete.** The three sessions (`kimi-fallback-profile-stale.md`, `openai-max-completion-tokens.md`, `vendor-token-usage-fidelity.md`) move to `.planning/debug/resolved/` with frontmatter `status: resolved` and a `resolved:` line citing the WINDOWS row that closed each (76, 75, 74 respectively, all `fixed`), following the debug workflow's `.planning/debug/resolved/{SLUG}.md` convention.

### Claude's Discretion

- Plan ordering and count; whether rows 85-87 are one plan or two (85+86 mechanical, 87 semantic).
- The exact `windows waive` reason text per row, within D-03's shape.
- The layout of the three Phase 57 files, provided the audit can find each warning, prohibition and WINDOWS row by id.
- How the `Workflow Lint` run id is retrieved (GitHub MCP tools; a check-run read on the merge commit) and what to do if the API refuses — record the refusal and the PR #61 required-context evidence instead.
- Which `run_repo_error_for_scope` arm handles `UnknownSchemaVersion` and whether it becomes hideable (row 85).
- UAT shape for rows 85-87 beyond the regression tests (an HTTP tracer against SQLite is enough; no live Postgres is required in the sandbox).

### Deferred Ideas (OUT OF SCOPE)

- Tracked-tree API-key-shape scan in `check-gates` (candidate for Phase 58 or the next milestone; the key rotation / scrub / history-purge decision stays a maintainer action).
- Phase 54 advisory review follow-ups (WR-01 copy-paste Cargo snippets, WR-04 Treasurer webhook `secret` placeholder not refused, WR-05 Python < 3.11 traceback, IN-02 worker pods mount admin API keys).
- utoipa 5 → 6 migration (lifts RUSTSEC-2026-0367/0368/0369 for good; next feature milestone, minor release).
- A first-class `rehomed` WINDOWS status / `windows rehome` command.
- Phase 52-56 REVIEW warnings and the audit's integration warnings G1/G2 (Phase 58's pre-tag audit reads them).
- Seven keyword-matched pending todos (none is register work).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| REG-01 | Phase 45 and 51 SUMMARY `requirements-completed` carry STORE-02 and REASON-02/03/05/06 | Exact files, current values and the read-back command in "REG-01 findings"; 51-15 has no `requirements-completed` key at all |
| REG-02 | WINDOWS reads `open_count: 0`; rows dispositioned with evidence | Measured 13 open rows; per-row disposition table; `windows` CLI semantics (no reason arg on `fixed`); Workflow Lint run ids; row 68 evidence caveat |
| REG-03 | Three `diagnosed` debug sessions archived as resolved | `audit.cjs` scan semantics; rows 74/75/76 confirmed `fixed`; no `resolved/` dir yet |
| REG-04 | Phase 40 judgment-tier human read recorded; Phase 42 accepted as-is | The 20 prohibitions live in six PLAN files (not in the 10-row VERIFICATION table); 40-UAT test 3 is a bare `pass`; 42 already operator-confirmed; GitHub compare gives the diff range |
| REG-05 | Zero undispositioned review warnings across Phases 38-51 incl. Phase 40 WR-01..04, IN-04 | Per-file counts (47 WR confirmed), plus 2 CR findings the CONTEXT does not mention, one still live; liveness spot-check |
| REG-06 | `zip 3.0.0` exceptions re-checked and re-dated, `make security` green | utoipa-swagger-ui 10.0.2 verified; a FOURTH zip advisory (RUSTSEC-2026-0429) already turns `make security` red |
</phase_requirements>

## Summary

This is mostly a register phase, but three measured facts change the plan the CONTEXT assumes. **(1) `make security` is red on the current head** and CI `Security Audit` / `License & Dependency Policy` are already red on the branch (run `41703614636`): RUSTSEC-2026-0429 (`zip 3.0.0`, `ZipArchive::extract` aborts on a symlink entry with a large declared size, published 2026-10-22, entered the advisory-db 2026-11-02) is a fourth `zip` advisory with no register row. REG-06's "three exceptions" is really four, and "`make security` green" cannot be reached by re-dating three rows. Phase 56 already recorded this as owned by Phase 57. **(2) The Phase 40 "20 prohibitions" are not a 20-row table.** `40-VERIFICATION.md` carries a 10-row condensed table; the 20 `must_haves.prohibitions` live in the six `40-0N-PLAN.md` files (6+3+2+3+3+3), many are duplicates of each other across plans, and the "11 test-backed / 9 code-reading" split in D-20 is not derivable from either source. The base commit `629ef660` is not in this shallow clone (history boundary is 2026-10-08), but is reachable through the GitHub API. **(3) Two review findings graded Critical (38 CR-01, 45 CR-01) sit outside the "47 warnings" count; 38 CR-01 was fixed in-phase, but 45 CR-01 (`list_files` passes `limit` as the S3 delimiter) is still present in `crates/paladin-storage/src/minio.rs:550-556` and is tracked nowhere.** Spot checks also suggest 41 WR-05, 45 WR-02 and 45 WR-05 are still live, so D-14's "fix correctness/security defects" rule can mint more code work than the roadmap's "little code" note expects.

The remaining work is mechanical and low-risk. The `windows` CLI is `status | append | waive | fixed`; `fixed` takes no reason, and the house practice (rows 58, 62, 63, 73) is to hand-edit the reason cell in BOTH the table row and the JSON entry afterwards and prove the file round-trips through the tool's own `parseLedger`/`renderLedger`. Rows 83/84 have clean evidence: `Workflow Lint` succeeded on the PR #61 merge commit (run `41045889697`, job `124029754795`) and on the current branch head including the 55-01 step (run `41765477891`, job `126363067412`). Rows 85-87 are one coherent change if the port method is chosen carefully (see Architecture Patterns): D-01 and D-02 are in tension, and the recommended resolution is a single additive, defaulted `RunRepositoryPort` method that returns the thread's earliest attributed run with `LIMIT 1`.

**Primary recommendation:** Sequence the phase as (a) register for the four `zip` rows first so `make security` is green for every later gate, (b) mechanical REG-01/REG-03 edits, (c) the rows 85-87 code plan with one defaulted port method, (d) a read-only evidence hunt that classifies all 47 warnings plus the 2 Criticals before anything else is minted, (e) the judgment worksheet with an owner UAT checkpoint, and (f) a single final plan that performs every `windows fixed/waive` call last, so rows minted by (c)-(e) are included before `open_count: 0` is asserted.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Thread-ownership rule (`permits_thread`) | Core domain (`paladin-core::principal`) | — | The single place thread visibility is decided; D-02 changes its input, not its predicate [VERIFIED: codebase `principal.rs:285-326`] |
| Owner-run lookup (new port method) | Application port (`paladin-ports::RunRepositoryPort`) | Storage adapters (in-memory, SQLite, Postgres) | Query lives in the adapter; the port declares the contract; adapters apply it inside their own SQL like the scope predicate [VERIFIED: codebase `run_repository_port.rs` module docs] |
| Thread visibility gate and list walk | API / Backend (`paladin-web::thread_controller`) | Application service (`RunSubmissionService::ensure_thread_visible`) | Two callers of the same owner read; both must use the same method [VERIFIED: codebase] |
| Error hiding (`UnknownSchemaVersion`) | API / Backend (`run_repo_error_for_scope`) | — | Role-aware redaction is a web-layer decision (ADR-0063 rule 6) |
| WINDOWS ledger mutation | Tooling (`gsd-tools windows`) | Hand edit of reason cell | Only mutation path; `fixed` takes no reason [VERIFIED: `broken-windows.cjs`] |
| Advisory exceptions | Governance register (`SECURITY-EXCEPTIONS.md`) | `.cargo/audit.toml`, `deny.toml` mirrors | Register is authoritative; script enforces agreement [VERIFIED: `check-advisory-register.sh`] |
| Review dispositions / judgment read | Planning artifacts (`.planning/phases/57-*`) | Archived REVIEW.md (append-only pointer) | Audit reads these by name |

## Standard Stack

No new external packages are installed by this phase. All work uses tools already in the workspace.

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| `gsd-tools windows` (`node /workspace/.claude/gsd-core/bin/gsd-tools.cjs windows ...`) | gsd-core installed | The only WINDOWS mutation path: `status`, `append`, `waive <id> "<reason>"`, `fixed <id>` | CONTEXT locks it; counts are recomputed and a mismatch is refused [VERIFIED: `broken-windows.cjs`] |
| `gsd-tools query summary-extract <path> --fields requirements_completed` | installed | Read-back of exactly what the audit reads for REG-01 | `audit-milestone.md` step 5c uses this command [VERIFIED: workflow + ran it] |
| `cargo-audit`, `cargo-deny` | installed (`~/.cargo/bin`) | `make security` | Present locally [VERIFIED: `command -v`] |
| `cargo-semver-checks`, `cargo-public-api` + `nightly-2026-09-20` | installed | Patch-gate for the new port method; `make api-surface` | Present locally [VERIFIED] |
| `gh api repos/DF3NDR/paladin-dev-env/...` | gh preconfigured | Workflow Lint run ids; Phase 40 diff via compare API | Works for repo `DF3NDR/paladin-dev-env` only; `DF3NDR/paladin` returns 403 [VERIFIED: ran both] |
| `shellcheck` | installed | Any script edit | [VERIFIED] |

### Supporting
| Tool | Purpose | When to Use |
|------|---------|-------------|
| `scripts/check-advisory-register.sh` | Register / audit.toml / deny.toml / Cargo.lock agreement | After the zip-row edits; currently reports "15 register row(s) checked against 15 deny.toml and 8 .cargo/audit.toml ignore entries" [VERIFIED: ran] |
| `scripts/check-findings-log.sh` + `tests/scripts/check-findings-log_test.sh` | Pattern for a never-vacuous disposition checker | Only if the plan adds a checker for `57-REVIEW-DISPOSITIONS.md` |
| `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh` | Patch-gate over all 12 packages | Once, after the port method lands; takes about 12 minutes [CITED: 55-CORRECTNESS-EVIDENCE.md Semver (D-13)] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Hand-edit reason cell after `windows fixed` | Add a reason arg to gsd-tools | Tooling change is explicitly out of scope; the hand-edit has four precedents and a byte-identical round-trip proof |
| Bump `utoipa-swagger-ui` to 10.0.2 to lift the zip rows | — | Pulls `utoipa ^6.0.0`; a major migration; locked out by D-30 |

**Installation:** none.

**Version verification:** `utoipa-swagger-ui` 10.0.2 (published 2026-10-20) depends on `utoipa ^6.0.0`, `zip ^9.0` (build), `utoipa-swagger-ui-vendored ^0.2` (build, optional); 9.0.2 depends on `utoipa ^5.0.0`, `zip ^3.0`. `utoipa` latest 6.0.0 (2026-09-22), `utoipa-axum` latest 0.3.0 (2026-09-22). The tree is on utoipa 5.5.0, utoipa-axum 0.2.0, utoipa-swagger-ui 9.0.2, utoipa-swagger-ui-vendored 0.1.2, zip 3.0.0 [VERIFIED: crates.io API on 2026-11-04 and `Cargo.lock`].

## Package Legitimacy Audit

This phase installs no external packages (the `zip` / `utoipa*` crates are existing lockfile dependencies and are only re-checked, not added). Audit not applicable.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
Phase 57 work, ordered by dependency (WINDOWS mutation LAST)

 [A] zip register (4 rows)  --->  make security green  --->  gate for every later plan
        |
 [B] REG-01 frontmatter, REG-03 debug archive          (independent, mechanical)
        |
 [C] Rows 85-87 code  (TDD)
        port method (defaulted) -> in-memory / SQLite / Postgres adapters
              -> thread_controller (gate + list walk)  \
              -> RunSubmissionService::ensure_thread_visible  > permits_thread(owner_run)
        + ADR-0063 / parley-and-chronicle / MIGRATION 11.1 / CHANGELOGs / openapi.json
        |
 [D] Evidence hunt (read-only) over 13 REVIEW.md: 47 WR + 2 CR + Phase 40 IN-01/IN-04
        -> 57-REVIEW-DISPOSITIONS.md  --live defect-->  windows append  --> fix (TDD) or waive
        |
 [E] 57-JUDGMENT-READ.md  (20 prohibitions + credential checklist)  --> owner UAT confirm/reject
        |                                                  reject --> windows append --> fix/waive
        v
 [F] FINAL: windows fixed/waive for 56, 64-66, 68, 69, 77, 79, 83-87 (+ rows minted by C-E)
        -> hand-edit reason cells (table + JSON) -> parseLedger/renderLedger byte-identical
        -> windows status == open_count 0 -> 57-REGISTER-EVIDENCE.md -> merge main -> CI run ids
```

### Recommended Project Structure
```
.planning/phases/57-register-hygiene-close-out/
├── 57-CONTEXT.md                 # exists
├── 57-RESEARCH.md                # this file
├── 57-REGISTER-EVIDENCE.md       # every WINDOWS disposition + debug moves + frontmatter edits + make security run
├── 57-REVIEW-DISPOSITIONS.md     # | Phase | Id | Title | Disposition | Evidence | Owner (if waived) |
└── 57-JUDGMENT-READ.md           # 20 prohibitions + credential sub-table + confirm/reject column
.planning/debug/resolved/         # new dir; three archived sessions
```

### Pattern 1: One defaulted port method that serves rows 86 and 87 together
**What:** D-01 asks for a "newest run" method used by both callers; D-02 then makes ownership the *earliest attributed* run. Taken literally, the newest-run method would be dead for authorization (the only two callers use the run to decide ownership). Recommended resolution: add ONE method, e.g. `RunRepositoryPort::thread_owner_run(&self, thread_id: &ThreadId) -> Result<Option<Run>, RunRepositoryError>`, returning the earliest run on the thread whose `submitted_by` is `Some`, ordered `(submitted_at ASC, run_id ASC)` with `WHERE thread_id = ? AND tenant_id IS NOT NULL ... LIMIT 1` (no `limit + 1` sentinel). It closes row 86 (exactly one row is decoded; a corrupt row elsewhere on the thread can no longer lock the owner out) and row 87 (Admin forks and unattributed runs never move ownership) with one public-surface addition instead of two. Keep `RunReadScope::permits_thread(&self, Option<&Run>)` unchanged in signature and pass it the owner run; rename the parameter and rewrite the docs.
**When to use:** This is the recommendation; the planner must confirm it against D-01's wording (see Open Question 1). If the owner wants the literal newest-run method as well, it is an additional, otherwise-unused public method (costly to remove).
**Semver:** the method MUST carry a default body. A new required method on a non-sealed public trait is a patch-gate failure; the precedent is `insert_with_latest` ("adding this method does not break any pre-existing implementor of this trait (X-10.4)") and `RunTracePort::max_seq` ("defaulted") [VERIFIED: codebase `run_repository_port.rs`; 42-REVIEW-FIX]. Eleven in-tree test doubles implement the trait (`webhook/tests.rs` x3, `worker_tests.rs`, `cancel.rs`, `run_controller.rs`, `thread_controller.rs` `MockRunRepository`, the port's own mock, plus the three real adapters), so a default is also what keeps them compiling. A correct default can page `list` (walk to the last page, filter `submitted_by.is_some()`); the three real adapters override it with `LIMIT 1`.
**Example (shape only):**
```rust
// Source: pattern of insert_with_latest in crates/paladin-ports/src/output/run_repository_port.rs
/// The thread's owning run: the earliest run that carries `submitted_by`.
async fn thread_owner_run(&self, thread: &ThreadId)
    -> Result<Option<Run>, RunRepositoryError> { /* default: walk `list`, keep earliest attributed */ }
```
No migration is needed: `runs` already has `tenant_id`/`api_key_id` columns (migration 008) and `idx_runs_tenant_submitted`; a thread-scoped `ORDER BY ... LIMIT 1` over a handful of rows per thread does not need a new index, and `check-migration-allowlist` / sqlx checksums stay untouched [VERIFIED: codebase migrations].

### Pattern 2: Skip-and-log in the list walk (row 85)
**What:** In `visible_thread_page` (`thread_controller.rs:777-840`), extend the existing `Err(RunRepositoryError::Serialization { .. }) if !matches!(scope, RunReadScope::All)` arm to `Serialization | UnknownSchemaVersion`. For the single-object gate, widening `run_repo_error_for_scope`'s `hideable` to include `UnknownSchemaVersion` removes the existence oracle WR-01 describes, but that helper is `pub(crate)` and shared with `run_controller` (`GET /runs/{id}` would then answer a tenant 404 instead of 500 for a newer-schema row) and its doc says the 500 is a deliberate recorded residual (D-09, PD-02-2; ADR-0063 "residuals"). Recommended: skip-and-log in the walk (required by D-01) and make `UnknownSchemaVersion` hideable for the thread-owner lookup only, updating the ADR residual line; leave the run routes as they are unless the owner says otherwise.

### Pattern 3: WINDOWS row closure with evidence (house practice)
**What:** `windows fixed <id>` (no reason argument), then set the reason cell by hand in both the table row and the JSON entry (no `|`, no four-backtick run), then prove the file loads with `parseLedger` and re-renders byte-identical, then `windows status` shows the expected counts. `windows waive <id> "<reason>"` takes the reason inline and is used for waivers. Rows 58, 62, 63, 73 were closed this way [CITED: 55-13-SUMMARY.md deviation note; 54-09-PLAN.md].

### Anti-Patterns to Avoid
- **Closing row 68 on a stale Phase 47 `make security` exit 0.** `make security` is red today; closing 68 ("CI Security Audit / License jobs red") without a green `make security` on this phase's head and a green CI `Security Audit` at the final head is dishonest evidence. Cite the Phase 57 run.
- **Renaming `permits_thread_admits_all_and_only_the_latest_runs_tenant`.** It is cited by name in WINDOWS reasons, `56-DOGFOOD-EVIDENCE.md`, `.project/v0.11.1/02-dogfood-findings-log.md` and 56-04's verify commands; add new tests for the new semantics and keep or deliberately re-point this one (edit the evidence in the same plan).
- **Putting `windows waive/fixed` in an early plan.** Sweep and judgment plans can mint rows; counts must be asserted only at the end.
- **A vacuous disposition register.** 56-REVIEW WR-01/WR-02 found a checker that accepted `TBD`, `none`, duplicate cycles and silently dropped a malformed heading. If `57-REVIEW-DISPOSITIONS.md` gets a checker, fail on any warning id found in the archives but absent from the register, reject placeholder evidence (`TBD`, `none`, `todo`), and require `Owner` on `waived` [CITED: 56-REVIEW.md]. Note CONTEXT names `scripts/check-dogfood-dispositions.py`; that file does not exist. The real analogue is `scripts/check-findings-log.sh` with `tests/scripts/check-findings-log_test.sh` [VERIFIED: ls].

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Closing/waiving a ledger row | Direct JSON/table edits of status | `gsd-tools windows fixed/waive` | Counts recomputed and cross-checked; only the reason cell on `fixed` rows is hand-edited, per precedent |
| Minting a new row | Editing WINDOWS.md | `gsd-tools windows append --kind --phase --file --line --description` | Keeps ids, counters and recorded_at consistent |
| Tenant visibility decision | A second ownership predicate in the controller or service | `RunReadScope::permits_thread` fed the owner run | One rule, written once (Phase 53 decision) |
| Moving a debug session | Deleting it | `mv .planning/debug/{slug}.md .planning/debug/resolved/` with `status: resolved` | `audit.cjs` skips `status: resolved` and ignores the `resolved/` subdirectory [VERIFIED: `audit.cjs scanDebugSessions`] |
| Reading SUMMARY frontmatter | grep | `gsd-tools query summary-extract <path> --fields requirements_completed` | Exactly the call the audit makes |
| Advisory exceptions | Editing only `audit.toml`/`deny.toml` | Register row + both mirrors, then `scripts/check-advisory-register.sh` | The script fails on any one-sided change |

**Key insight:** Every deliverable here is verified by an existing tool that the audit also uses; build each edit so the audit's own command produces the expected output, and record that command's output as the evidence.

## Per-Requirement Findings

### REG-02: WINDOWS (measured 2026-11-04)
`windows status`: `open_count: 13, waived: 40, fixed: 34, total: 87`. Phase 56 minted nothing (total unchanged at 87). Open rows: 56, 64, 65, 66, 68, 69, 77, 79, 83, 84, 85, 86, 87 [VERIFIED: ran `windows status`]. Rows 58, 62, 63, 67, 73 are already closed; rows 74, 75, 76 are `fixed` (needed for REG-03).

| Row | Disposition | Evidence to cite |
|-----|-------------|------------------|
| 56 | `fixed` | Re-run on the Phase 57 head: `cargo doc -p paladin-ai-core --no-deps` finished with no warning lines today (7.5 s) [VERIFIED: ran]; also record `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` exit 0 (not re-run in this research; `make doc-check` covers it) |
| 64, 65, 66, 77, 79 | `waive` with the D-03 shape | ADR-0057 (64-66), ADR-0061 (77), `50.1-SEED-PROBE.md` (79) |
| 68 | `fixed` | A green `make security` on the Phase 57 head plus CI `Security Audit` / `License & Dependency Policy` success on the final head (see anti-pattern above). Today those two CI jobs FAIL on the branch (run `41703614636`, jobs `126153521475`, `126153521855`) |
| 69 | `fixed` | `Coverage` job `126153522434` succeeded in run `41703614636` (branch head `d9a48cfc6`); 92.74 % on `70657919` is in `47-CI-EVIDENCE.md:1263`. Note run `41703614636` overall is `failure` (Security Audit, License, SDK smoke), so cite the job, not the run [VERIFIED: gh api] |
| 83 | `fixed` | `Workflow Lint` job `124029754795` success in run `41045889697` on merge commit `84686acfb`; also job `124670697434` in main run `41246424413` (`8b23cb4c5`) [VERIFIED: gh api] |
| 84 | `fixed` | Same as 83. The 55-01 `ci.yml` step ("Check no stub panics in published crates (FIXES-01)", `ci.yml:188`) is NOT on `main` (branch is 258 commits ahead of `origin/main`); the covering evidence is `Workflow Lint` job `126363067412` success in branch run `41765477891` (head `69de7c285`). Cite the run for the final Phase 57 head at seal instead |
| 85, 86, 87 | `fixed` | The new regression tests by name (see Validation Architecture) |

`windows fixed`'s CLI ignores reasons, so budget a hand-edit for every `fixed` row (56, 68, 69, 83-87 = 8 rows).

### REG-01: SUMMARY frontmatter
- `45-01-SUMMARY.md:49` reads `requirements-completed: []`. Its own `key-decisions` says "STORE-01 and STORE-02 are NOT marked complete by this plan". D-41 overrides this on the strength of `45-UAT.md` test 1 (`ci.yml` run `36751442387`, 11 passed against RustFS). Put the evidence beside the edit (a trailing YAML comment or a dated sentence in the register evidence file) rather than leaving the contradiction silent. Confirm a trailing `# comment` still parses with `summary-extract` before relying on it (not tested here) [ASSUMED].
- Phase 51 SUMMARYs 01-14 have `requirements-completed: []` plus `requirements-progress: [...]`; 17-20 carry real values; **51-15 has no `requirements-completed` key** (only `requirements: [REASON-05]` at line 22) and 51-16 has neither; so for REASON-05 the key must be ADDED to 51-15, not edited. Targets per D-41: REASON-02 → `51-13`, REASON-03 → `51-14`, REASON-05 → `51-15`, REASON-06 → `51-14`.
- Verify with `gsd-tools query summary-extract <path> --fields requirements_completed` run from the project root with a relative path; running from another directory returns "File not found" [VERIFIED: ran]. The audit globs `.planning/phases/*-*/*-SUMMARY.md`, which does NOT include `.planning/milestones/v0.11.0-phases/`; archived phases are only reached by the audit agent reading them, which is why CONTEXT edits them in place.

### REG-03: Debug sessions
Three files in `.planning/debug/`, all `status: diagnosed`; `.planning/debug/resolved/` does not exist yet. `gsd-tools audit-open` currently lists them as "Debug Sessions (3 open)" and treats `resolved`/`complete` as closed [VERIFIED: ran]. Rows 74/75/76 are `fixed` (74 and 75 and 76 have empty reasons). Move with `git mv`, set frontmatter `status: resolved` and add the `resolved:` line. Re-run `audit-open` afterwards: debug count should be 0 (it will still list the pending todos, which are out of scope).

### REG-04: Judgment read
- The 20 prohibitions: six PLAN files hold 6+3+2+3+3+3 = 20 `must_haves.prohibitions` (each `{requirement_id, category, statement}`, no `verification`); the condensed 10-row table at `40-VERIFICATION.md:165-197` collapses duplicates (unattributed sentinel, `RUN_SCHEMA_VERSION`, key-value logging, TokenBudget/Commissary rename appear in several plans) [VERIFIED: parsed the PLAN frontmatter]. The worksheet should list all 20 (each plan/statement) and map each to its condensed verdict row; the "11 test-backed / 9 code-reading" split should be derived from the evidence, not copied.
- `40-UAT.md` test 3 already reads `result: pass` (bare, no evidence, UAT `complete` 2026-09-30). `40-VERIFICATION.md` frontmatter says `status: passed` but the body says `human_needed`. The worksheet adds the per-item record the bare pass lacks; do not edit either file.
- `40-VALIDATION.md:76` is a row in `## Manual-Only Verifications`, not `## Gaps` (there is no Gaps section). `42-VALIDATION.md` has NO judgment-tier row in its Manual-Only table (lines 96-101); the note must be added as a new row. `42-VERIFICATION.md` already records `human_verification_resolved: all 15 verdicts confirmed by the operator (2026-10-07T20:58:19Z)`, so "accepted as-is" is consistent and even stronger than the CONTEXT assumes.
- Diff base: `629ef660` is not in the local clone (shallow, boundary 2026-10-08). GitHub has it: `gh api repos/DF3NDR/paladin-dev-env/compare/629ef660...be3a9030` returns 29 commits and 69 files (`be3a9030` is the Phase 40 final code commit per `40-UAT.md`). Either `git fetch --unshallow` (or `--shallow-since=2026-09-24`) before the read, or use the compare API with `Accept: application/vnd.github.diff` [VERIFIED: gh api].
- The credential checklist has the existing pre-seed: 40-06 manual review recorded clean; `ApiKeyConfig` `#[derive(Debug)]` noted pre-existing at `629ef660`. The Phase 56 DF-07 work (Debug redaction) and WR-09 (Serialize still emits secrets) are later and not Phase 40's diff; do not conflate.

### REG-05: Review sweep
- Files: 13 `NN-REVIEW.md` (38, 40, 41, 42, 43, 44, 45, 46, 47, 48, 50, 50.1, 51), not fourteen (39 and 49 have none). Warning counts: 2+4+5+2+7+5+6+3+2+5+0+4+2 = **47** (matches frontmatter) [VERIFIED: grep and frontmatter].
- The CONTEXT says 22 warnings lack a REVIEW-FIX report (Phases 38, 41, 43, 45, 47, 51). Measured: 2+5+7+6+2+2 = **24**. REVIEW-FIX exists for 40, 42, 44, 46, 48, 50, 50.1 [VERIFIED].
- Not every warning in a "fixed" phase is actually covered: `44-REVIEW-FIX.md` fixed only WR-01 and WR-02 and states WR-03, WR-04, WR-05 "were deliberately not in scope", queued as todos at `.planning/todos/pending/2026-10-11-*` (described as untracked; check they exist). `42-REVIEW-FIX.md` has one skipped item (IN-7, not a warning). `42-REVIEW.md` uses ids `WR-5`, `WR-6` and `IN-10..14` (iteration 2); WR-1..4 from iteration 1 are not in the file, so the register should say so.
- **Critical findings outside the "warning" count:** `38-REVIEW.md` CR-01 (resolved in-phase 2026-09-26, noted in the file) and `45-REVIEW.md` CR-01. **45 CR-01 is still live:** `crates/paladin-storage/src/minio.rs:550-556` still does `self.bucket.list(prefix.clone(), max_keys.clone())` where the second argument is rust-s3's delimiter; no WINDOWS row, no later phase cites it [VERIFIED: grep of tree, WINDOWS.md, v0.11.0-MILESTONE-AUDIT.md]. A correctness defect, so D-14 says mint a row and fix with TDD (the review proposes `list_page` and a contract test with keys `a5`, `b5`, `c` and `limit: Some(2)`).
- **Spot-check of likely-live warnings** (a pointer for the hunt, not a verdict): 41 WR-05 (empty operator-webhook secret signs with the empty key; no empty-filter in `with_operator_notice_secret`, `run_api_wiring.rs:1352`); 45 WR-02 (multipart errors embed raw S3 bodies; no `s3_err_summary` helper in `minio.rs`); 45 WR-05 (no redaction in `worker.rs`; `worker_tests.rs:1324` still pins `sk-agent-secret-marker` text as readable). 41 WR-05 overlaps the Phase 54 WR-04 item the owner deferred (webhook `secret` placeholder not refused); consider waiving it with a pointer to that deferral instead of fixing, which is an owner-level call. These three are security-class under D-14, so the "little code" scope can grow.
- Phase 40: WR-01..04 closed by TENANT-04/05/06 per CONTEXT D-11 and `40-REVIEW-FIX.md` (4 fixed); IN-01 and IN-04 are in `40-REVIEW.md` (5 info items). Nothing else needed beyond pointer evidence.
- Mechanics: the register must also append `## Dispositions (Phase 57)` to each of the 13 archived REVIEW.md files, leaving the bytes above untouched (`git diff` shows pure additions).

### REG-06: zip exceptions (and the fourth advisory)
- `make security` today: `cargo audit` exits non-zero ("1 vulnerability found", RUSTSEC-2026-0429); `cargo deny check advisories` fails on the same ID. The `make` target stops at `audit`. Advisory: `zip` 3.0.0, patched `>= 9.0.0`, CVSS 7.5 (DoS: unbounded `Vec::with_capacity` in `ZipArchive::extract` for a symlink entry), advisory-db commit 2026-11-02 [VERIFIED: ran both tools]. It concerns the same `extract` API as 0367-0369, so the same compensating control applies (the utoipa-swagger-ui build script never calls `extract` and only opens its own embedded bundle).
- Phase 56 recorded this in `.planning/phases/56-dogfood-findings/deferred-items.md` and `56-VERIFICATION.md:168` ("owned by Phase 57 REG-05", a typo for REG-06). The CONTEXT does not mention it.
- Required register edit set: (1) a fourth `[[exception]]` row for RUSTSEC-2026-0429 with all eleven fields; (2) the same ID in `.cargo/audit.toml` `ignore` and in `deny.toml` `[advisories].ignore` (vulnerability section); (3) update the prose counts in `SECURITY-EXCEPTIONS.md` ("exactly the eight vulnerability-class rows", "The fifteen live suppressions", "Eight ... and seven ...") to nine / sixteen / nine and seven; (4) apply D-31 re-dating and the D-32 history paragraph entry to all four rows. After the edit `check-advisory-register.sh` should read 16 register rows against 16 deny.toml and 9 audit.toml entries. `review_date` is not date-validated by the script (only presence) [VERIFIED: script read].
- Lifting is not available: `utoipa-swagger-ui` 10.0.2 requires `utoipa ^6.0.0` (and `utoipa-axum` 0.3.0 exists); this is the D-30 migration, and `crates/paladin-web/openapi.json` is a committed snapshot guarded by `openapi_matches_committed_baseline` [VERIFIED: crates.io and codebase].
- CI context: the advisory entered advisory-db 2026-11-02; the last green `Security Audit` on main was 2026-10-31. Expect `Security Audit` and `License & Dependency Policy` to turn red on every branch/PR until the row lands. The branch also has a third red job (`Generated SDK Clients (Python + TypeScript) smoke`, run `41703614636`, pre-existing per Phase 56) that is outside this phase's registers but affects "CI green" claims.

## Common Pitfalls

### Pitfall 1: Counting "three" zip rows
**What goes wrong:** Re-dating three rows leaves `make security` red; REG-06's exit criterion and the roadmap's "with `make security` green" cannot be met.
**How to avoid:** Plan for four rows; run `make security` (both legs: audit then deny) as the plan's verification, not just `check-advisory-register.sh`.
**Warning signs:** `check-advisory-register.sh` passes while `cargo audit` fails (the script never reads the advisory db).

### Pitfall 2: D-01 vs D-02 produce two public methods and a dead one
**What goes wrong:** Shipping both a newest-run method and an earliest-attributed method adds permanent public surface, one of which nothing uses.
**How to avoid:** One defaulted method (Pattern 1); decide explicitly with the owner (Open Question 1).

### Pitfall 3: Default method that silently re-introduces the `limit + 1` decode
**What goes wrong:** A default implemented through `list(limit: 1)` still decodes two rows on SQLite/Postgres.
**How to avoid:** The three real adapters override with `LIMIT 1`; contract tests assert a corrupt *other* row on the thread no longer fails the lookup (the WR-02 reproduction: tenant A's thread with a corrupt older row).

### Pitfall 4: Legacy mixed-tenant threads change owner
**What goes wrong:** `mixed_tenant_thread_belongs_to_the_latest_runs_tenant` (`thread_controller.rs:2887`) encodes the old rule; under D-02 the thread belongs to acme (first attributed) and globex loses it. The test must change deliberately, and `MIGRATION.md` §11.1 M-B-24/M-B-25 (lines 14-15), §11.6 lines 75 and 91, `docs/src/api-reference/platform-api.md:258,663`, `docs/src/deployment-topologies/http-service-host.md:127`, `docs/src/user-guides/parley-and-chronicle.md:313`, ADR-0063 decision 1 and 4, the utoipa `description` strings in `thread_controller.rs` (lines 929, 1053, 1263) and the committed `crates/paladin-web/openapi.json` (five description occurrences) all say "latest run".
**How to avoid:** Grep `latest run` / `latest-run` / `newest run` after the change; regenerate `openapi.json` with `make openapi`; keep `[0.11.0]` changelog sections and MIGRATION §9/§10 byte-frozen (the M-B-24/25 rows are in §11.1 and may be edited since v0.11.1 is unreleased; note the CHANGELOG `[Unreleased]` entries in `paladin-core` and `paladin-web` that also state the latest-run rule).

### Pitfall 5: The `MockRunRepository` in `thread_controller.rs` counts `list` calls
**What goes wrong:** The double bumps `list_calls` and injects failures only through `list`; tests asserting the bounded-walk cost (examined-thread cap, call counts) will change meaning when the controller stops calling `list`.
**How to avoid:** Override the new method in the double, keep failure injection per thread, and re-state the cost assertions against it.

### Pitfall 6: Hand-editing the ledger breaks the round-trip
**What goes wrong:** A `|` or four-backtick run in a reason cell, or editing only the table row or only the JSON entry, corrupts or desynchronises the ledger.
**How to avoid:** Follow Pattern 3 exactly; load with `parseLedger`, re-render, diff byte-identical; reason text without `|`. D-03's reason shape (`Re-homed: owner=..., revisit=..., basis=...`) contains no pipe; keep it that way.

### Pitfall 7: The sweep mints unbounded code work
**What goes wrong:** D-14 sends every live correctness/security warning to a TDD fix in this phase; 45 CR-01, 45 WR-02, 45 WR-05 and 41 WR-05 alone cross subsystems (storage adapter, run worker, webhook wiring).
**How to avoid:** Make the evidence hunt its own plan that outputs a classified list before any fix is started, and put an owner checkpoint on the live-defect list (fix vs waive-with-owner), because the phase's goal is register agreement, not a defect backlog.

### Pitfall 8: Disk and clone state
**What goes wrong:** 3.8 GB free on `/` (earlier phases noted builds dropping the disk to 1.4 GB), a shallow clone missing Phase 40-43 history, no `cargo-llvm-cov`, no `actionlint`.
**How to avoid:** Use `CARGO_INCREMENTAL=0`, targeted `-p` runs, rely on CI for coverage and Postgres legs, and fetch history or use the compare API for the Phase 40 read.

## Code Examples

### Closing a row (house practice)
```bash
# Source: 54-09-PLAN.md / 55-13-PLAN.md precedent; subcommands verified in broken-windows.cjs
G="node /workspace/.claude/gsd-core/bin/gsd-tools.cjs"
$G windows fixed 83                       # no reason argument exists
# then set the reason cell in BOTH the table row and the JSON entry, no '|' characters
$G windows waive 65 "Re-homed: owner=Treasurer maintainers, revisit=next feature milestone roadmap, basis=ADR-0057 closing conditions. Check-only boundary read is an accepted, bounded overshoot."
$G windows status | python3 -c 'import json,sys; d=json.load(sys.stdin)["ledger"]; print(d["open_count"], d["total_count"])'
```

### Minting a row for a live defect found by the sweep
```bash
$G windows append --kind deviation --phase 45 --file crates/paladin-storage/src/minio.rs --line 550 \
  --description "45-REVIEW CR-01: list_files passes limit as the S3 delimiter ..."
```

### Reading what the audit reads for REG-01 and REG-03
```bash
$G query summary-extract .planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/51-15-SUMMARY.md --fields requirements_completed
$G audit-open | sed -n 1,12p     # debug sessions must drop to 0
```

### Phase 40 diff without local history
```bash
gh api repos/DF3NDR/paladin-dev-env/compare/629ef660...be3a9030 --jq '.files[].filename'
gh api -H 'Accept: application/vnd.github.diff' repos/DF3NDR/paladin-dev-env/compare/629ef660...be3a9030 > "$SCRATCH/phase40.diff"
```

### Workflow Lint evidence lookup
```bash
gh api "repos/DF3NDR/paladin-dev-env/commits/84686acfb/check-runs?per_page=100" \
  --jq '.check_runs[] | select(.name=="Workflow Lint") | [.conclusion,.id,.details_url] | @tsv'
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Thread owner = latest run (ADR-0063 as accepted 2026-10-31) | Thread owner = earliest attributed run (D-02) | This phase | Authorization semantic change; amends a published ADR before v0.11.1 ships |
| Three `zip 3.0.0` exceptions | Four (RUSTSEC-2026-0429 added to the advisory-db 2026-11-02) | Advisory published 2026-10-22 | `make security` red until a row lands |
| `utoipa-swagger-ui` 9.x (zip ^3) | 10.0.2 (zip ^9.0, utoipa ^6) | 2026-10-20 | Upstream condition met, lift needs utoipa 6 migration |

**Deprecated/outdated:** the CONTEXT's "22 warnings without a fix report" (measured 24), "fourteen REVIEW files" (13), "20 rows in the Prohibitions table" (10 condensed rows; 20 in PLAN files), "`## Gaps` row" in 40-VALIDATION (it is a Manual-Only row), `scripts/check-dogfood-dispositions.py` (does not exist).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | A single defaulted `thread_owner_run` method satisfies both D-01 (row 86) and D-02 (row 87) and is preferable to adding a separate newest-run method | Pattern 1 | Owner may want the literal newest-run method too; one extra public method |
| A2 | Making `UnknownSchemaVersion` hideable for the thread-owner lookup only (not for run routes) is acceptable | Pattern 2 | ADR-0063 residual wording and a possible owner preference to keep 500 on single objects |
| A3 | A trailing YAML comment on `requirements-completed: [STORE-02]` still parses with `summary-extract` | REG-01 | Fall back to recording the evidence in `57-REGISTER-EVIDENCE.md` only |
| A4 | 41 WR-05, 45 WR-02, 45 WR-05 are still live and security-class (grep-level spot check only) | REG-05 | Fewer code fixes than feared, or more if other warnings are also live |
| A5 | A default implementation of the new method that walks `list` is semantically correct for third-party implementors | Pattern 1 | Slow for large threads only; real adapters override |
| A6 | The two pre-existing red CI jobs besides Security/License (SDK smoke) are out of Phase 57's registers | REG-06 | If Phase 58 expects CI fully green, someone must own the SDK smoke failure |
| A7 | Waiving 41 WR-05 by pointing to the owner-deferred Phase 54 WR-04 is acceptable | REG-05 | Owner may require the fix under D-14 |

## Open Questions (RESOLVED)

All six questions are answered by the Phase 57 plans (plan-checker revision 1, 2026-11-04). Each RESOLVED note names the plan decision or task that owns the answer.

1. **D-01 vs D-02: one port method or two?**
   - What we know: ownership moves to the earliest attributed run; the only two callers of the old latest-run lookup are the authorization gate and `ensure_thread_visible`.
   - What's unclear: whether the owner still wants a literal "newest run" method in the public surface.
   - Recommendation: one defaulted `thread_owner_run`; ask once in plan review.
   - **RESOLVED:** PD-02-1 (`57-02-PLAN.md`) adopts one defaulted `RunRepositoryPort::owner_run_for_thread` (the earliest attributed run, with the earliest-run fallback of PD-02-2), named after the trait's `active_run_for_thread`. The "ask once" is the blocking `checkpoint:decision` at 57-02 Task 1 (option-a recommended; option-b adds D-01's literal newest-run method; option-c keeps the latest-run rule). The door fails closed: 57-02 Tasks 2-3, 57-03 Task 1 and 57-04 Task 1 run only when `57-02-SUMMARY.md` `## Decision` opens with `Selected: option-a`; any other answer stops them for a plan revision.
2. **Fourth zip advisory: lift or except?**
   - What we know: lifting needs utoipa 6 (D-30 defers it); the same compensating control covers 0429.
   - Recommendation: add the fourth exception row, mirror it, re-date all four.
   - **RESOLVED:** PD-01-1 (`57-01-PLAN.md`): RUSTSEC-2026-0429 gets a fourth `zip 3.0.0` row on the D-31 terms, under the same build-time-only compensating control re-measured from the build script, mirrored in `.cargo/audit.toml` and `deny.toml`; all four rows are re-dated (PD-01-2). The lift stays with the utoipa 6 migration (D-30, deferred).
3. **How far does D-14 go for the live warnings (45 CR-01, 45 WR-02, 45 WR-05, 41 WR-05, others the hunt finds)?**
   - Recommendation: classify first, then an owner checkpoint (fix in Phase 57 vs waive under D-03 with a named owner and date); do not start fixes before the classified list exists.
   - **RESOLVED:** Findings whose liveness was verified at plan time are fixed in this phase under D-14 by 57-05 (45 WR-05, 41 WR-05; PD-05-1) and 57-06 (45 CR-01, WR-01, WR-02, WR-03; PD-06-1), each minting its WINDOWS row first. Everything else the 57-08..57-10 sweep or the 57-07 judgment read finds live (plan-time expectation: at least 45 WR-04 and WR-06) is classified first and decided at one owner `checkpoint:decision`, 57-10 Task 3 (PD-10-1). Fix-in-phase items go through the expected `/gsd-plan-phase 57 --gaps` pass before 57-11, whose precondition halts while any of them has no fix commit.
4. **Do Critical findings belong in `57-REVIEW-DISPOSITIONS.md`?** Recommendation: yes (38 CR-01 `closed`, 45 CR-01 `open` until fixed); a register that omits a live Critical does not match the tree.
   - **RESOLVED:** PD-08-1 (`57-08-PLAN.md`): yes. 38 CR-01 and 45 CR-01 are register rows, 51 findings in total (47 WR, 2 CR, Phase 40 IN-01/IN-04). 45 CR-01 closes on the 57-06 fix (57-10 Task 1).
5. **Orphaned Phase 55 deferrals:** "News API key in the URL (`D-52-03-01`, `news_api_fetcher.rs`)" and "`${...}` in the treasurer webhook `secret`" were routed to a `56.x` phase that was never created (Phase 56 became dogfood). They are not WINDOWS rows, so they do not affect `open_count`, but nothing owns them. Recommendation: note in `57-REGISTER-EVIDENCE.md` where they landed (Phase 54 deferral for the second) or re-home them explicitly.
   - **RESOLVED:** PD-11-3 (`57-11-PLAN.md`): `57-REGISTER-EVIDENCE.md` `## Orphaned deferrals` records where each item now sits, with owner and revisit (the second is Phase 54 WR-04, owner-deferred). Neither is a WINDOWS row, and nothing is implemented for them in this phase.
6. **Pending todos files for Phase 44 follow-ups** (`2026-10-11-reject-removed-v0-10-battalion-config-keys.md`, `2026-10-11-phase-44-review-follow-ups.md`) are cited by `44-REVIEW-FIX.md` as untracked; confirm they exist before citing them as waiver evidence for 44 WR-03..05.
   - **RESOLVED:** Confirmed at plan time: both files exist under `.planning/todos/completed/` (not `pending/`), each `completed: 2026-10-24`. 57-09 Task 2 step 2 reads their resolution sections and verifies each fix on the head before citing them for 44 WR-03..05 (D-13: evidence hunted, never asserted).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| node + `gsd-tools.cjs` | all WINDOWS/summary/audit commands | yes | node 22 | — |
| `gh` (repo `DF3NDR/paladin-dev-env`) | Workflow Lint run ids, Phase 40 compare | yes | preconfigured | GitHub MCP tools |
| `cargo-audit` / `cargo-deny` | REG-06, rows 68 | yes | installed | — |
| network to crates.io / advisory-db | `make security` | yes (db fetched on run) | — | — |
| `cargo-semver-checks`, `cargo-public-api`, nightly-2026-09-20 | rows 85-87 gates | yes | installed | CI `Semver Checks` job |
| `shellcheck` | script edits | yes | /usr/bin | — |
| `cargo-llvm-cov` | 82 % coverage floor | no | — | CI `Coverage` job |
| `actionlint` | workflow edits | no | — | CI `Workflow Lint` job (no workflow edits planned) |
| Docker daemon / Postgres server | Postgres contract legs | binary present, daemon not checked | — | SQLite + in-memory locally, CI `postgres-integration` |
| Full git history | D-21 diff `629ef660..` | no (shallow, boundary 2026-10-08) | — | `git fetch --unshallow` or GitHub compare API |
| Disk | cargo builds | 3.8 GB free | — | `CARGO_INCREMENTAL=0`, targeted `-p` runs, avoid full-workspace builds |

**Missing dependencies with no fallback:** none.
**Missing dependencies with fallback:** `cargo-llvm-cov`, `actionlint`, full git history, Postgres legs (CI or API).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (Rust 1.97.1), tokio tests, shell guard tests in `tests/scripts/*_test.sh`, plus tool read-backs (`windows status`, `summary-extract`, `audit-open`) |
| Config file | `Cargo.toml` workspace; `.cargo/audit.toml`; `deny.toml`; `Makefile` |
| Quick run command | `cargo test -p paladin-ai-core --lib principal::` and `cargo test -p paladin-web --lib thread_controller::` |
| Full suite command | `make clean-code && cargo test --workspace && make check-gates && make test-shell-guards && make security && make api-surface` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| REG-01 | Audit-extract returns STORE-02 / REASON-02,03,05,06 | tool read-back | `gsd-tools query summary-extract <45-01 / 51-13 / 51-14 / 51-15 SUMMARY> --fields requirements_completed` | tool exists |
| REG-02 | `open_count: 0`; every `fixed` reason cell present; every waiver matches `Re-homed:` | tool read-back + grep | `windows status` then `python3` check over entries (status != open; `waived` reasons start `Re-homed:` for 64-66, 77, 79) | Wave 0: small check snippet in the evidence plan |
| REG-02 (85) | `UnknownSchemaVersion` row does not 500 `GET /v1/threads` for a Tenant scope | unit (web) | `cargo test -p paladin-web --lib thread_controller::` (new test, name e.g. `list_walk_skips_a_newer_schema_run_row_for_a_tenant_scope`) | no, Wave 0 |
| REG-02 (86) | A corrupt non-owner run row does not lock the owner out; exactly one row decoded | contract test across 3 adapters | `cargo test -p paladin-storage --features sqlite --lib run::` (new clause in `contract_tests.rs`, wired into in-memory, SQLite, Postgres) | no, Wave 0 |
| REG-02 (87) | Admin fork / unattributed run never moves ownership; mixed-tenant legacy thread belongs to the earliest attributed tenant | unit (core) + web | `cargo test -p paladin-ai-core --lib principal::` and `cargo test -p paladin-web --lib thread_controller::` (rewrite `mixed_tenant_thread_belongs_to_...`) plus `ensure_thread_visible` test in `submission.rs` | partly (test to be rewritten) |
| REG-03 | No debug session listed open | tool read-back | `audit-open` debug count 0; `test -f .planning/debug/resolved/*.md` | tool exists |
| REG-04 | 20 prohibitions all have a row, evidence and an owner mark | file check | grep/awk over `57-JUDGMENT-READ.md` (20 rows + checklist rows, no empty Human column at seal) | Wave 0 |
| REG-05 | Every warning id in the 13 REVIEW.md files appears in the register with a non-placeholder disposition; totals line `open: 0` | script or checklist | a never-vacuous checker modelled on `scripts/check-findings-log.sh` (optional) | no, Wave 0 (optional) |
| REG-06 | Four rows agree across register/audit.toml/deny.toml; no advisory fails | script + tools | `./scripts/check-advisory-register.sh` and `make security` | exists |

### Sampling Rate
- **Per task commit:** the task's targeted `cargo test -p ... --lib <module>` plus `cargo fmt --check` / `cargo clippy -p ... -- -D warnings`.
- **Per wave merge:** `make clean-code`, `make check-gates`, `make security`.
- **Phase gate:** full suite above, `SEMVER_PRUNE_LOCAL_CACHE=1 CARGO_INCREMENTAL=0 ./scripts/check-semver.sh` (12 packages, patch, about 12 min), `make api-surface`, `windows status` showing `open_count: 0`, and CI green on the Phase 57 head for `Security Audit`, `License & Dependency Policy`, `Workflow Lint`, `Coverage`, `Semver Checks`, `Postgres Storage Contract Suites`.

### Wave 0 Gaps
- [ ] New failing tests for rows 85-87 (RED before GREEN): the list-walk `UnknownSchemaVersion` skip, the corrupt-other-row contract clause (all three adapters), the Admin-fork / unattributed-run ownership tests in `principal.rs` and `thread_controller.rs`, and the `ensure_thread_visible` case in `submission.rs`.
- [ ] `MockRunRepository` (thread_controller.rs) override of the new method.
- [ ] Optional: a disposition checker for `57-REVIEW-DISPOSITIONS.md` and a `tests/scripts/*_test.sh` regression test, designed against the 56-REVIEW weaknesses.
- [ ] Coverage floor stays 82 %: new code is small but the contract clause must run on all adapters.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (no auth change) | existing `Principal` / API-key config |
| V3 Session Management | no | — |
| V4 Access Control | yes | `RunReadScope::permits_thread` fed the owner run; hidden == missing 404 (ADR-0054/0063); no post-filtering of fetched pages |
| V5 Input Validation | limited | `ThreadId` validation unchanged; no new request fields |
| V7 Error Handling and Logging | yes | role-aware `redact_store_error_for_scope`; the skip-and-log line must not interpolate backend text beyond what the existing `Serialization` arm logs; no API key in any log |
| V6 Cryptography | no | — |
| V14 Configuration / dependencies | yes | RustSec exception register, `cargo-audit` / `cargo-deny` |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Cross-tenant thread takeover via Admin fork or schedule-fired run (row 87) | Elevation of privilege / Tampering | Ownership from earliest attributed run; regression tests for Admin fork and `submitted_by: None` |
| Existence oracle through a distinguishable 500 vs 404 (`UnknownSchemaVersion`) | Information disclosure | Hide for Tenant scope, redact for Admin |
| Cross-tenant availability loss via one corrupt row aborting `GET /v1/threads` | Denial of service | Skip-and-log per thread in the walk |
| Decode of unrelated rows (`limit + 1`) locking out an owner | Denial of service | Single-row `LIMIT 1` owner query |
| Build-time zip `extract` advisories | Tampering / DoS | Build-time-only scope, never-calls-`extract`, input is the crate's embedded bundle; documented exception |
| Credential in error text/logs (Phase 40 re-read; 45 WR-02/05, 41 WR-05 if live) | Information disclosure | Redact before truncate (`crates/paladin-llm/src/redaction.rs`); no key in logs, Debug or responses |

The manual credential-handling review in `.github/instructions/security.instructions.md` is the primary control (no merge-gating Rust SAST; CodeQL is advisory only; Snyk is not to be reintroduced). Run it on the new thread-owner code path.

## Project Constraints (from CLAUDE.md)

- TDD red-green-refactor; every defect reproduction becomes a regression test; 82 % workspace line coverage (ADR-0006); doc tests on public APIs.
- Dependencies flow inward only (core → nothing; ports → core; adapters → core + ports); the new port method lives in `paladin-ports`, adapters in `paladin-storage`, never importing infrastructure from core or application.
- Ubiquitous Medieval Military terms in code, docs and comments; `Entity<T>` is deliberately generic; "node" means only a WarGraph/Campaign vertex.
- No `unwrap()`/`expect()`/`panic!` in library code; prefer borrowing; lazy iterators.
- Before committing a parent task: `cargo test`, `cargo fmt --check`, `cargo clippy`, `make api-surface` (intentional surface change: `make api-surface-update` plus a CHANGELOG entry); conventional commits; stop after each major task and wait for go-ahead.
- Security: `make security`, `cargo clippy -- -D warnings`, manual credential review; do not reintroduce Snyk; CodeQL is advisory.
- Use `Write`/`Edit` for files; commit messages end with the session attribution lines supplied by the harness.
- Edits to planning registers must keep archived files append-only (D-10/D-13/D-22) and `.planning/WINDOWS.md` is mutated only via `gsd-tools windows` plus the documented reason-cell hand edit.

## Sources

### Primary (HIGH confidence)
- Repository measurements this session: `windows status`, `audit-open`, `summary-extract`, `make security` (audit and deny legs), `scripts/check-advisory-register.sh`, `cargo doc -p paladin-ai-core --no-deps`, tree greps, YAML parse of the six Phase 40 PLAN frontmatters.
- `/workspace/.claude/gsd-core/bin/lib/broken-windows.cjs`, `audit.cjs`, `workflows/audit-milestone.md`, `templates/DEBUG.md`, `workflows/execute-phase.md` (resolved convention).
- crates.io API (2026-11-04): `utoipa-swagger-ui` 10.0.2 / 9.0.2 dependency lists, `utoipa` 6.0.0, `utoipa-axum` 0.3.0.
- GitHub API (`DF3NDR/paladin-dev-env`): check-runs on `84686acfb`, jobs of runs `41246424413`, `41703614636`, `41765477891`, compare `629ef660...be3a9030`.
- RustSec advisory-db clone: `crates/zip/RUSTSEC-2026-0429.md`.

### Secondary (MEDIUM confidence)
- Phase artifacts: `53-REVIEW.md`, `55-CORRECTNESS-EVIDENCE.md`, `55-13-SUMMARY.md`, `56-REVIEW.md`, `56-VERIFICATION.md`, `40-VERIFICATION.md`, `40-UAT.md`, `42-VERIFICATION.md`, `42-VALIDATION.md`, the 13 archived REVIEW.md and 7 REVIEW-FIX.md files, `ADR-0063`.

### Tertiary (LOW confidence)
- Liveness of 41 WR-05, 45 WR-02, 45 WR-05 (grep-level only); the other 40+ warnings were not checked individually.

## Metadata

**Confidence breakdown:**
- Standard stack / tooling: HIGH - every command was run or its source read.
- Architecture (rows 85-87 shape): MEDIUM - recommendation resolves a real tension in D-01/D-02 and needs owner confirmation.
- Register facts (counts, ids, run ids): HIGH - measured.
- Warning liveness: MEDIUM/LOW - spot-check only; the evidence hunt is a plan task.
- Pitfalls: HIGH - each is grounded in an observed file or tool behaviour.

**Research date:** 2026-11-04
**Valid until:** 2026-11-11 for CI run states and advisory-db contents (fast-moving); register and tooling facts stable for the phase.
