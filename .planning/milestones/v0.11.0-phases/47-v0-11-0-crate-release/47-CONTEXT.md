# Phase 47: v0.11.0 Crate Release - Context

**Gathered:** 2026-10-24
**Status:** Ready for planning

<domain>
## Phase Boundary

v0.11.0 is **released, not merely releasable**. The `0.10.1 → 0.11.0` lockstep bump, the dated
`[0.11.0]` changelog sections and the OpenAPI baseline regeneration land as the last commits on the
milestone branch; the re-seal gate set is run green on the candidate final head and recorded; the
existing PR #59 is retitled and merged to `main` with a merge commit; the annotated `v0.11.0` tag
sits on that merge commit; `release.yml` runs green; every publishable crate (the list is whatever
`cargo metadata` says — twelve today, `doc-examples` is `publish = false`) resolves on the crates.io
sparse index at `0.11.0`, `yanked = false`; the publish-order gate is green on the release run; the
release record is written into MILESTONES.md; SHIP-07 is ticked on what is observably true; and
web3sec receives its outbound hand-off so the milestone can close.

Delivered here: the bump commit set (manifests, changelog stamps, `openapi.json`), the re-seal and
its evidence, the retitled release PR, the sign-off document and the maintainer's tick, the merge +
tag hand-off, the post-tag registry verification, the MILESTONES.md entry, the SHIP-07 /
ROADMAP / STATE ticks, `47-HANDOFF-web3sec.md`, and a "ready to close" hand-off.

**Not** delivered here: any change under `src/` or `crates/*/src/`; any new capability; any fix to
a red gate once the re-seal has started (D-14 — a red gate stops the phase); any `ci.yml` change
(the stale `0.9.0` semver baseline is a recorded finding, D-09); any re-read of the frozen
MIGRATION §10 unless a code change after `e28e43b5` moves the consumer surface (Phase 46 D-03);
the `/gsd-complete-milestone` run itself; web3sec's own re-pin and gates; anything belonging to
the next milestone.

Observed state at discussion time (2026-10-24, HEAD `e241c348` on `claude/laughing-dirac-e0h2ax`):
every manifest reads `0.10.1`; all twelve changelogs sit at `## [Unreleased]`;
`crates/paladin-web/openapi.json` embeds `"version": "0.10.1"`; PR #59 is open against `main`
(1,081 commits, 1,080 files, title still `feat(43-02): …`, `mergeable_state: blocked`, CI queued
on `e241c348`); `origin/main` tip is `df24689c` (the `chore/37.1-post-close` merge); every
dependency phase (46, 48, 49, 50, 50.1, 51) is sealed; the v0.11.0 milestone audit reads
`gaps_found` with SHIP-07 the only unsatisfied requirement; `cargo-release` is not installed in
this sandbox; the sandbox has no Docker and cannot measure coverage locally.

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by Phases 29 / 37 / 37.1 — not re-asked)

- **D-00a:** Only the maintainer ticks a sign-off box — never an agent, under any mode including
  `--auto` (Phase 29 D-17, Phase 37 D-00a).
- **D-00b:** The maintainer alone performs the irreversible acts — merging the release PR, pushing
  the `v*` tag, and any `cargo yank`. The agent hands over exact commands and stops; it never merges
  to `main`, never pushes a `v*` tag, never publishes, never yanks (Phase 37 D-02/D-16, Phase 37.1
  D-00b, runbook §5).
- **D-00c:** Tag-cut mechanism: the release PR merged to `main` with a **merge commit** (not squash,
  not rebase), then an **annotated tag pushed directly to that merge commit** after `ci.yml` is green
  on it. `make release` is not the path (its trailing push is blocked by the "Protect main branch"
  ruleset); its component steps are (Phase 37 D-00c/D-04, Operator Guide).
- **D-00d:** Findings are recorded, never fixed silently; amend-at-source — original text retained,
  additions dated; `WINDOWS.md` rows are never deleted (Phase 29 D-12/D-24).
- **D-00e:** The 82 % workspace line-coverage floor (ADR-0006) is measured by CI's `coverage` job
  only.
- **D-00f:** Shipped tree outranks any document. The publishable-crate list is whatever
  `cargo metadata` says (`publish != false`), never hard-coded (Phase 37 D-00f/D-08).
- **D-00g:** Publishing is via crates.io Trusted Publishing; no standing registry credential exists
  and nothing in this phase may introduce or request one. All twelve crates, `paladin-eval`
  included, are linked — `0.10.1` was their last pipeline publish.
- **D-00h:** Registry proof is a sparse-index (`index.crates.io`, mandatory `User-Agent`) read per
  publishable crate — crate, version, checksum, `yanked` — recorded as a table. The workflow's own
  log is not accepted as proof on its own (Phase 37 D-08).
- **D-00i:** Waiting is a hard stop with a resume file naming the exact resume condition — never a
  parked watcher or a long-lived `gh run watch` (Phase 37 D-12).
- **D-00j:** Any red gate from the start of the re-seal onward — locally or in PR CI — is recorded
  (command, SHA, failing output) and the agent halts at a blocking checkpoint and fixes **nothing**
  (Phase 37 D-14). The one exception is a pure infrastructure flake, re-run **exactly once** with both
  run IDs and the justifying log line recorded (Phase 37 D-15).
- **D-00k:** `release.yml` cannot be dry-run-dispatched before a ref literally named `v0.11.0`
  exists (Phase 37 research Q1). No dispatch rehearsal, no rc tag, no shadow tag.
- **D-00l:** Post-tag commits reach the protected `main` through a `chore/47-close` branch cut from
  the tagged `main` and a small docs-only PR the maintainer merges; the tagged commit stays exactly
  what was released; `/gsd-complete-milestone` is not run by any plan (Phase 37 D-10/D-11).
- **D-00m:** The phase stays open across the merge: a **build wave** (bump, stamps, regen), a
  **pre-merge wave** (re-seal, PR, evidence, sign-off), the maintainer's **merge + tag checkpoint**,
  and a **post-tag wave** (registry verification, records, hand-offs). Verified only on what is
  observably true, never by promise (Phase 37 D-09).
- **D-00n:** `CHANGELOG.md` heading dates equal the tag date (Phase 46 D-13). MIGRATION §10 is
  frozen at `e28e43b5` and is re-read only if a code change after that SHA moves the consumer
  surface (Phase 46 D-03).

### Branch & PR shape

- **D-01:** **One PR.** The bump commit set lands as the last commits on
  `claude/laughing-dirac-e0h2ax`; the already-open **PR #59 is the release PR** — retitled
  `release: v0.11.0 — Treasurer Spend Governance` (wording at the planner's discretion) with a
  curated body; its single merge commit carries the whole milestone and is the tag target. This is
  the v0.10.0 precedent (PR #55 from `feature/phase-33`). Rejected: a second `release/v0.11.0` PR
  off `main` (Operator Guide pattern) — it would put unreleased 0.11 code on `main` under a `0.10.1`
  version and cost a second sign-off cycle. — **Reversibility:** one-way — the merge commit on a
  protected `main` becomes the tag target and cannot be rewritten after the tag points at it.
- **D-02:** **`main` is frozen** at `df24689c` until the release PR merges; #59 is the only merge
  before the tag, so its diff is exactly "everything since `v0.10.1`". If `main` moves anyway, the
  branch merges `main` in (merge commit, never rebase), the re-seal restarts from the top on the new
  head, and the maintainer re-confirms the sign-off against the new SHA.
- **D-03:** The **agent retitles and rewrites the PR #59 body via `gh api` (REST only — GraphQL is
  blocked in this sandbox)**: re-seal summary, pointer to `CHANGELOG.md` `[0.11.0]`, `MIGRATION.md`
  §10, `47-CI-EVIDENCE.md` and `47-ACCEPTANCE-AUDIT.md`, and the twelve-crate list derived from
  `cargo metadata`. Fallback for a read-only token: hand the exact command to the maintainer.
- **D-04:** The branch **keeps its name** `claude/laughing-dirac-e0h2ax`. No `release/*` branch is
  created (renaming would close #59 and lose its thread; every STATE.md "pushed on
  `claude/laughing-dirac-e0h2ax`" citation stays true; the `release/*` one-push ruleset is not
  engaged).

### Bump mechanics & gate scope

- **D-05:** The bump is applied with **`cargo release version 0.11.0 --execute --no-confirm
  --workspace`** (install `cargo-release` first; `release.toml` already configures lockstep and pin
  updates — Operator Guide step 2). Then: diff every `Cargo.toml`, and a **grep-zero check for
  `0.10.1`** across manifests and the workspace-dependency pins (historical CHANGELOG/MIGRATION/ADR
  text excluded). `crates/doc-examples` is `publish = false` — if the tool skips it, its `version`
  and its nine `paladin-*` pins are hand-touched in the same commit. Thirteen manifests plus the
  root's own `paladin-llm`/`paladin-eval` entries carry the version today (`Cargo.toml` lines 34-43,
  58, 136, 229, 344). — **Reversibility:** costly — a missed pin reproduces the v0.10.0 class of
  defect; the publish-order and release-consistency gates catch it only on the final head.
- **D-06:** The twelve changelogs are stamped with **`make finalize-crate-changelogs
  VERSION=0.11.0`** (`scripts/finalize-crate-changelogs.sh`, root included), dated to the tag date
  (D-00n). `check-changelogs` and `check-release-consistency` are re-run locally afterwards. The root
  `[0.11.0]` section is hand-curated only if the stamp leaves it unreadable; its content is what
  Phases 38-51 and 46 wrote under `[Unreleased]`.
- **D-07:** **`make openapi`** regenerates `crates/paladin-web/openapi.json` after the bump; the
  **expected diff is the version string only**. Any other change means a route moved after the Phase
  46 freeze — it is recorded as a finding and the agent stops (D-00j); it is not absorbed into the
  release.
- **D-08:** The **build wave fixes nothing outside the bump commit set itself** (manifest pins, the
  stamp output, the OpenAPI version string). Red here is handled as follows: a defect in the bump
  itself is corrected; anything else — a failing test, a lint, a drifted golden — is recorded and put
  to the maintainer. From the moment the re-seal starts on the candidate final head, D-00j applies
  unchanged.
- **D-09:** CI's `cargo-semver-checks` job stays pinned to **`--baseline-version 0.9.0`**; its
  known no-op state (Phase 38 plan 38-09 finding: `0 checks, 254 skip` for every package; `paladin-eval`
  excluded) is **recorded as a deferred release-tooling finding in `47-CI-EVIDENCE.md`**, with the
  recommendation to re-pin to `0.11.0` and add `paladin-eval` in the next milestone's first phase.
  No `ci.yml` edit on release day. Rejected: re-pinning to `0.10.1` in the build wave (fires on every
  registered 0.11 break and widens the release PR into CI and allowlist work).

### Sign-off home & re-seal table

- **D-10:** The v0.11.0 re-seal table and the sign-off box live in a **new
  `47-ACCEPTANCE-AUDIT.md` in this phase's directory**, beside `47-CI-EVIDENCE.md`. The v0.10.0
  corpus audit (`.project/v0.10.0/09-program-acceptance-audit.md`, §11-§13) and the archived
  `29-ACCEPTANCE-AUDIT.md` pointer are **not edited** — a closed milestone's document does not
  govern a v0.11.0 release. The box reads `- [ ] **The \`v0.11.0\` tag may be cut**` with the
  evidence sentence in the §13 form.
- **D-11:** The signed table is the **eight 37.1 rows, re-read at `0.11.0`** on the candidate final
  head, each with command, SHA, result and a pointer to `47-CI-EVIDENCE.md`: (1) `MIGRATION.md`
  no-TBD + register ↔ `semver-checks` allowlist set-equality — now reading **both §9.2 and §10.2**
  (`make check-migration-allowlist`, Phase 46); (2) `v0_9_config_boot`; (3) OpenAPI golden
  (`openapi_golden_v0_9`); (4) `cargo semver-checks` vs `0.9.0`, eleven crates, recorded as the
  known no-op per D-09; (5) MSRV 1.88; (6) `make publish-dry-run`, twelve crates in dependency
  order; (7) `CHANGELOG.md` `[0.11.0]` completeness via `make check-gates` plus the hard-assertion
  greps (`[Unreleased]` count 0, `[0.11.0]` count 1 per changelog); (8) `make check-publish-order`.
- **D-12:** `make api-surface` (no baseline move after the bump), `make check-consumer-surface`
  (the muster — the roadmap's Phase 49 dependency) and the CI `coverage` job (≥ 82 %) are
  **CI-evidence-only entries in `47-CI-EVIDENCE.md`, not signed rows**; the signed table keeps the
  eight-row shape every re-seal since Phase 29 has used.
- **D-13:** **Sign-off order is structurally enforced (Phase 37.1 D-10):** local re-seal → push →
  PR CI green including `coverage` → evidence appended → the **maintainer ticks the box in
  `47-ACCEPTANCE-AUDIT.md` and the tick commit is pushed on the branch** (or the maintainer tells
  the agent to commit it, with the reply quoted verbatim in the SUMMARY) → CI green on that true
  final SHA → the agent **verifies the ticked box is on `origin` at the PR head** → only then the
  merge + tag hand-off is written. No tick on `origin`, no hand-off. — **Reversibility:** one-way —
  the merge and tag that follow trigger a crates.io publish; published versions can be yanked but
  never deleted or re-used.
- **D-14:** CI corroboration is **recorded plainly per row**: each row names the PR-run job that
  reconfirms it where one exists; rows with no CI twin (`check-migration-allowlist` is not wired into
  `ci.yml`; `Publish Dry Run` runs only on `main` pushes) say so in the Evidence column rather than
  being assumed covered. No `ci.yml` change to close that gap on release day (recorded as a finding
  beside D-09).

### Post-tag wave & hand-offs

- **D-15:** Phase 47 writes an **outbound `47-HANDOFF-web3sec.md`** in the phase directory (the
  mirror of the inbound `49-`/`50-`/`51-HANDOFF-*.md` and `46-FEEDBACK-web3sec.md`): tag, merge SHA,
  release run ID, the twelve crates at `0.11.0` registry-verified, pointers to `MIGRATION.md` §10.8
  (the DR rows) and `tests/consumer_surface_muster.rs`, and the return contract (consumer commit
  SHA, D-2 choice, gate totals, divergences — the folded todo's step 5). The folded todo
  `2026-10-14-web3sec-single-bump-to-v0-11-0.md` gets a **dated "tag cut, hand-off written"
  update and stays `pending`** until web3sec returns.
- **D-16:** **SHIP-07, the ROADMAP checkbox, the REQUIREMENTS traceability row and STATE.md are
  ticked only after the sparse-index table shows all twelve crates at `0.11.0`, `yanked = false`**,
  through `gsd-tools` handlers (never direct edits), in the post-tag wave, riding `chore/47-close`.
  Rejected: ticking on the release PR before the merge — it ticks a claim not yet true.
- **D-17:** **`chore/47-close` carries:** the post-merge and release-run CI evidence, the registry
  table, the MILESTONES.md v0.11.0 entry (v0.10.0-entry shape: tag, merge SHA, release run ID, crate
  count, "registry-verified", Trusted Publishing), the D-16 ticks, `47-HANDOFF-web3sec.md`, the
  todo update, and the D-09/D-14 findings. The last plan ends at **"ready to close"** with the
  `/gsd-audit-milestone` / `/gsd-complete-milestone` prerequisites listed; the maintainer runs
  `/gsd-complete-milestone v0.11.0` on the same branch so the archive moves ride the same PR.
- **D-18:** If the **real release run goes red after the tag**, Phase 37 D-16 applies unchanged:
  the agent diagnoses **read-only** per `docs/src/appendix/release-recovery.md` §1-§2 (sparse index
  for what reached crates.io, the run log, the matching runbook section and failure code), writes it
  to `47-CI-EVIDENCE.md`, and hard-stops. Completing forward (§3), any re-dispatch, any yank (§5) and
  any `47.1` patch phase are the maintainer's. **Option A** (a patch release through the same
  pipeline; orphaned versions yanked only after the patch is registry-verified; one yank-register row
  per crate) is the recorded default if the maintainer asks for a recommendation. — **Reversibility:**
  one-way — a half-published dependency chain on crates.io is permanent state.

### Claude's Discretion

The maintainer did not select these; the defaults below were stated and accepted. Research may
overturn a default only with evidence, and must say so in RESEARCH.md.

- **Bump commit granularity** — default: three commits on the branch (manifests via
  `cargo release version`; changelog stamps; `openapi.json` regen), each conventional-commit
  (`chore(release): …`), so a defect in one is isolated. One commit is acceptable if the tooling
  makes the split awkward.
- **Stamp date** — default: the day the maintainer is expected to tag; if the tag slips past that
  date, the heading date is corrected in a docs-only commit *before* the tick (D-00n), and the
  re-seal row 7 is re-read on the new head.
- **PR title and body wording**, within D-03's required contents and the v0.10.0 PR #55 shape.
- **Re-seal command order and output capture**, provided every one of D-11's eight rows has a
  command, a SHA and a result, and `47-CI-EVIDENCE.md` follows the `37.1-CI-EVIDENCE.md` house form
  (local sweep table + CI-run table + registry table; the registry probe command shape at its line
  ~1735 is reusable verbatim).
- **Hand-off checkpoint texts** for merge + tag — reuse Phase 37.1's with `0.11.0`; the tag command
  is `git tag -a v0.11.0 <merge-sha> -m "v0.11.0"` + `git push origin v0.11.0`.
- **Whether `47-ACCEPTANCE-AUDIT.md` also carries a short "what v0.11.0 is" preamble** drawn from
  `MIGRATION.md` §10's opening paragraph — default yes, two or three sentences, no new claims.
- **Plan count and wave layout**, provided D-00m's four-wave split around the D-13 tick gate and
  the maintainer's merge + tag checkpoint holds, and no plan both executes the bump and starts the
  re-seal in the same task.
- **The v0.11.0 GitHub Release object** — created by `release.yml` from the curated `[0.11.0]`
  section; no manual edit is planned. If the body is wrong, record it; a `gh release edit` is the
  maintainer's.
- **Whether `claude/laughing-dirac-e0h2ax` is deleted after the merge** — the maintainer's GitHub
  setting; no plan depends on it.

### Folded Todos

- **web3sec moves once, straight to v0.11.0, after Phases 50 and 51**
  (`.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md`, score 0.6, `recheck_by:
  Phase 47 close`). Problem: the consumer must not bump twice; the tree it meets next is `v0.11.0`.
  Fit: Phase 47 produces the tag and, per D-15, the outbound hand-off that gives web3sec the tag,
  the merge SHA, the registry proof and the §10.8 pointers; the consumer's own re-pin, gates and
  return report stay web3sec's, so the todo stays `pending` with a dated update.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and source decisions
- `.planning/ROADMAP.md` §"Phase 47: v0.11.0 Crate Release" (line ~792) — goal, three success
  criteria, five dependency phases and what each must have landed before the tag
- `.planning/REQUIREMENTS.md` §SHIP-07 (line ~434) and the traceability row (line ~515) — the
  requirement D-16 ticks
- `.planning/v0.11.0-MILESTONE-AUDIT.md` — `gaps_found`; SHIP-07 the only unsatisfied requirement;
  seam 8 (API-surface baseline currency after 51-08) PARTIAL — the audit the post-tag wave must
  turn green
- `.planning/milestones/v0.10.0-phases/37-v0-10-0-crate-release/37-CONTEXT.md` — D-00a..D-17:
  every release-day rule this phase inherits (act boundary, merge commit, evidence form, hard stops,
  red-gate policy, `paladin-eval` bootstrap history)
- `.planning/milestones/v0.10.0-phases/37.1-v0-10-1-patch-release/37.1-CONTEXT.md` — D-00a..D-12:
  the structurally enforced sign-off order (D-10), the two-stage red-gate policy (D-08), the
  yank/option-A recovery (D-01), the record shape (D-05)
- `.planning/milestones/v0.10.0-phases/37-v0-10-0-crate-release/37-RESEARCH.md` — Q1 (why no
  dry-run dispatch before a tag), Q3 (Trusted Publishing cannot do a first publish)
- `.planning/phases/46-docs-currency-hygiene/46-CONTEXT.md` — D-00c (version identity v0.11.0
  everywhere), D-03 (MIGRATION §10 freeze at `e28e43b5`), D-11/D-12 (twelve crates,
  `paladin-eval` Trusted Publishing row), D-13 (heading date = tag date), D-25 (web3sec single bump
  folded as input)
- `.planning/phases/46-docs-currency-hygiene/46-FEEDBACK-web3sec.md` — the inbound hand-off form
  D-15's outbound hand-off mirrors; the consumer's return-contract expectations
- `.planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-HANDOFF-web3sec.md`
  §9 "Return contract" — the fields `47-HANDOFF-web3sec.md` must ask for (D-15)
- `.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md` — the folded todo; its
  "What to do" steps 1-5 and dated updates (D-15)

### Evidence files this phase writes to or mirrors
- `.planning/milestones/v0.10.0-phases/37.1-v0-10-1-patch-release/37.1-CI-EVIDENCE.md` — house
  form for `47-CI-EVIDENCE.md`: local sweep table, CI-run table, registry table (line ~1419) and the
  `index.crates.io` probe command with the mandatory `User-Agent` header (line ~1735)
- `.project/v0.10.0/09-program-acceptance-audit.md` §13 (line ~1627) — the eight-row gate table
  and box wording `47-ACCEPTANCE-AUDIT.md` copies (D-10/D-11); **read-only, not edited**
- `.planning/MILESTONES.md` §"v0.10.0 Durable Agent Execution Runtime" (lines 1-20) — the
  release-record shape the v0.11.0 entry follows (D-17)
- `.planning/STATE.md` §Accumulated Context "Release mechanics" and "Partial-publish recovery"
  bullets — the locked release rules in their current wording

### Release mechanics
- `.github/workflows/release.yml` — tag trigger (`v*.*.*`), `workflow_dispatch` inputs (lines 3-17),
  `verify-tag-source` (29), `test` (92), `create-release` (112, body from the curated `[X.Y.Z]`
  section via `scripts/extract-changelog-section.sh`), `check-release-consistency` (405),
  `publish-crates` (602)
- `.github/workflows/ci.yml` lines 313-395 — the `cargo-semver-checks` job, its `0.9.0` baseline
  pin and the `paladin-eval` exclusion (D-09's finding)
- `docs/src/appendix/release-automation.md` §"Operator Guide: Cutting a Release" — steps 1-8 (the
  `cargo release version` command, D-05; the `make openapi` requirement, D-07; the tag push with the
  release-tags ruleset bypass)
- `docs/src/appendix/release-checklist.md` — §2 "the root `## [VERSION]` section is a hard
  prerequisite for the release run"
- `docs/src/appendix/release-recovery.md` — §1-§2 the agent's read-only scope under D-18; §3-§5
  the maintainer's; §5 yank register (one row per crate)
- `docs/src/appendix/branch-protection.md` — main-only tag policy, "Protect main branch" ruleset
  (why D-00l uses a chore PR), "Protect release tags" ruleset
- `Makefile` — `check-changelogs` (189), `check-publish-order` (217), `check-consumer-surface`
  (248), `check-gates` (252), `openapi` (417), `api-surface` (423), `release-check` (618),
  `publish-dry-run` (628), `finalize-crate-changelogs` (634), `release` (642 — not used, D-00c)
- `release.toml` — `cargo-release` lockstep/pin configuration (D-05)
- `scripts/publish-crates.sh` — the `CRATES` order the real publish runs in;
  `scripts/check-publish-order.sh` — the gate (row 8); `scripts/check-release-consistency.sh` —
  tag ↔ manifest ↔ changelog agreement; `scripts/finalize-crate-changelogs.sh` — the stamp (D-06)

### Version bump and gate inputs
- `Cargo.toml` — `[workspace.package] version` (58), `[workspace.dependencies]` pins (34-43), the
  root package's own `paladin-llm` (136) and `paladin-eval` (229, 344) entries;
  `crates/*/Cargo.toml` and `crates/doc-examples/Cargo.toml` (`publish = false`, nine pins) — the
  thirteen manifests D-05 bumps
- `CHANGELOG.md` and the eleven `crates/*/CHANGELOG.md` — all at `## [Unreleased]` (D-06)
- `MIGRATION.md` §10 (frozen) and §10.2 / §9.2 registers; `.cargo/semver-checks-allowlist.toml`
  — row-level set-equality (re-seal row 1)
- `crates/paladin-web/openapi.json` — embeds `"version": "0.10.1"` (D-07)
- `tests/consumer_surface_muster.rs` + `make check-consumer-surface` — CI-evidence entry (D-12)
- `.project/current-exports.txt` — the API-surface baseline, last refreshed at plan 51-08 (D-12)
- `.planning/decisions/0006-coverage-gate.md` — ADR-0006, the 82 % floor (D-00e)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `release.toml` + `cargo release version … --workspace`: the lockstep bump already configured;
  nothing new to write (D-05). `cargo-release` must be installed in the sandbox first.
- `scripts/finalize-crate-changelogs.sh` (`make finalize-crate-changelogs VERSION=`): stamps all
  twelve changelogs in one invocation (D-06).
- `make check-gates`: bundles the nine offline guards including `check-publish-order` and
  `check-changelogs`; `make check-migration-allowlist` reads both §9.2 and §10.2 since Phase 46.
- `37.1-CI-EVIDENCE.md`: copy the three-table structure and the registry-probe command verbatim
  for `47-CI-EVIDENCE.md`.
- `.project/v0.10.0/09-program-acceptance-audit.md` §13: copy the eight-row table header, the
  row wording and the box sentence into `47-ACCEPTANCE-AUDIT.md`, re-read at `0.11.0`.
- Phase 37.1's merge + tag and yank hand-off checkpoint texts: reuse with `0.11.0`.
- `46-FEEDBACK-web3sec.md` / `49-HANDOFF-web3sec.md` §9: the hand-off form and return contract
  for `47-HANDOFF-web3sec.md`.

### Established Patterns
- Amend-at-source with dated additions — governs the todo update (D-15) and every finding.
- `chore/NN-close` branch + small PR for post-tag bookkeeping — governs D-17.
- CI-evidence files are sealed at the SHA they verify; editing a SUMMARY or evidence after
  VERIFICATION stales it — do all bookkeeping before the phase is verified.
- Blocking human checkpoints via the interactive question mechanism, reply recorded verbatim;
  owner-gated acts get a short, specific, just-in-time runbook at the point of need.
- ROADMAP/STATE/REQUIREMENTS mutations go through `gsd-tools` handlers, never direct edits (D-16).
- The clippy pre-commit hook and the GSD commit helper interact; warm the hook after any
  `cargo clean`. The pre-push `api-surface` gate runs on every push — the baseline must not move.

### Integration Points
- `gh api` (REST) for the PR retitle/body (D-03), run listing, and run re-run (D-00j). GraphQL
  (`gh pr …`) is blocked in this sandbox; every write needs the "hand the exact command over"
  fallback for a read-only token.
- crates.io sparse index `index.crates.io`: read-only HTTP with a `User-Agent` header, no
  credential (D-00h, D-18).
- `cargo metadata --format-version 1 --no-deps`: the single source for the publishable-crate
  list (twelve today).
- This sandbox is a shallow clone (653 commits); `git merge-base` with `origin/main` is empty.
  Any history-walking evidence (commit counts since `v0.10.1`) must be taken from GitHub (PR #59
  reports 1,081 commits) or an unshallowed fetch, and say which.

</code_context>

<specifics>
## Specific Ideas

- The maintainer took the recommended, conservative option on every question, as in Phases 37
  and 37.1: one reviewable merge commit (D-01), `main` frozen (D-02), nothing fixed past the
  re-seal (D-08), no `ci.yml` edit on release day (D-09/D-14), a sign-off enforced on `origin`
  rather than stated (D-13), ticks only on what is true (D-16), the agent never publishes or
  re-dispatches (D-18). For any unlisted situation: **stop and record, do not improvise.**
- "Released, not merely releasable" and "verified on what is true, not by promise" (Phase 37
  D-09) are the governing phrases; SHIP-07 is ticked by the registry table, not by the merge.
- The release-tooling findings this phase deliberately leaves on the table — the `0.9.0` semver
  baseline, `check-migration-allowlist` not in `ci.yml`, `Publish Dry Run` never running on a PR —
  are recorded in one place (`47-CI-EVIDENCE.md`) so the next milestone's first phase can pick
  them up as a set.

</specifics>

<deferred>
## Deferred Ideas

- **Re-pin CI's `cargo-semver-checks` baseline to `0.11.0` and add `paladin-eval`** — the job is
  a no-op at `0.9.0` (Phase 38 plan 38-09); a `ci.yml` change for the next milestone's first
  phase, recorded as a finding here (D-09).
- **Wire `check-migration-allowlist` into `ci.yml` and run `Publish Dry Run` on pull requests** —
  so every signed re-seal row has a CI twin (D-14). Release-tooling hardening.
- **Making `release.yml` dry-runnable against a bare SHA before a tag exists** — carried from
  Phases 37 / 37.1.
- **A consumer smoke test** (`cargo add paladin-ai@0.11.0` in a scratch project, cold build) as a
  standing post-release check — carried from Phases 37 / 37.1; web3sec's own re-pin (D-15) is the
  de-facto smoke test this time.
- **Path-only as a workspace-wide rule for workspace dev-dependencies** and a lint enforcing it —
  carried from Phase 37.1.
- **Carried from Phase 36.1 / 37 / 37.1 `<deferred>`, unchanged:** `cargo test --workspace
  --all-features` in CI; removing the eight crate-level rustdoc allows; regenerating
  `doc-coverage-report.md`.

### Reviewed Todos (not folded)
- **Review the unfinished content / general data orchestration system before removing any
  base-layer item** (`todos/pending/2026-10-13-review-content-data-orchestration-base-items.md`,
  score 0.6) — keyword match only (`crates`); a core-layer review whose recheck is "before the
  milestone after v0.11.0 is roadmapped". No bearing on cutting a release.
- **Add OpenAI Responses API support for the pro tier**
  (`todos/pending/2026-10-20-openai-responses-api-for-pro-tier.md`, score 0.6) — a new adapter
  wire, a new capability; its own phase in a later milestone.
- **Verify local `make coverage` reproduces CI's 82.39 % figure**
  (`todos/pending/2026-08-13-verify-local-coverage-reproduction.md`, score 0.4) — re-homed to the
  maintainer by Phase 36.1 D-23 (no Docker here); SC uses CI's `coverage` job. Not folded in
  Phases 37 or 37.1 either.

</deferred>

---

*Phase: 47-v0.11.0 Crate Release*
*Context gathered: 2026-10-24*
