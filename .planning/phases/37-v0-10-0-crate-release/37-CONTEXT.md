# Phase 37: v0.10.0 Crate Release - Context

**Gathered:** 2026-09-18
**Status:** Ready for planning

<domain>
## Phase Boundary

v0.10.0 is **released, not merely releasable**. The Phase 29 gate set is re-run green on the
final post-documentation commit and the evidence is recorded; the release PR is merged to `main`
with a merge commit; the annotated `v0.10.0` tag sits on that merge commit; `release.yml` runs
green; every publishable crate (the `publish = false` `doc-examples` crate excluded) resolves on
crates.io at `0.10.0`; and the release record is written into MILESTONES.md so the milestone can
close.

This phase is almost entirely **outward-facing, irreversible acts and the evidence for them** —
it is not a code phase. Delivered here: the gate re-seal, the PR, the pre-merge and post-merge
CI evidence, the dry-run rehearsal, the registry verification, the MILESTONES.md release entry,
`SHIP-05`, and a "ready to close" hand-off. **Not** delivered here: any change under `src/` or
`crates/*/src/`, any new capability, any fix to a red gate (D-14 — a red gate stops the phase),
the `/gsd-complete-milestone` run itself (D-11), or anything belonging to v0.11.0.

Observed state at discussion time (2026-09-18, HEAD `555c5285`): branch `feature/phase-33` is
488 commits ahead of `main`, 12 commits unpushed, no open PR; workspace version already reads
`0.10.0` (`Cargo.toml:58`, Phase 29 D-18).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by earlier phases — not re-asked)

- **D-00a:** Only the maintainer ticks the §11 sign-off box in
  `29-ACCEPTANCE-AUDIT.md` — never an agent, under any mode including `--auto` (Phase 29 D-17).
- **D-00b:** The `0.10.0` bump already landed on the feature branch without a tag; the tag is
  cut on the `main` merge commit and `release.yml`'s `verify-tag-source` enforces it. "The
  release commit" means both the PR head and the merge commit; evidence is recorded for both
  (Phase 29 D-18 / D-21).
- **D-00c:** The tag-cut mechanism is the documented one: a version-bump PR merged to `main`,
  then an **annotated tag pushed directly to the merge commit**
  (`docs/src/appendix/release-automation.md` Operator Guide). `make release VERSION=0.10.0` is
  **not** used — the "Protect main branch" ruleset blocks its direct push, and it would re-run a
  bump that has already landed.
- **D-00d:** Findings are recorded, never fixed silently (Phase 29 D-12). `WINDOWS.md` rows are
  never deleted (Phase 29 D-24). Amend-at-source: original text is retained, additions are dated.
- **D-00e:** The 82 % workspace line-coverage floor (ADR-0006) is measured by CI's `coverage`
  job only — this devcontainer has no Docker and cannot measure it locally.
- **D-00f:** Shipped tree outranks any document (Phase 34 D-00g, 35 D-00c, 36 D-00c, 36.1 D-00a).
  In particular the publishable-crate count is whatever the tree says, not the "eleven" the docs
  and the v0.9.0 record state (see D-08).
- **D-00g:** Publishing is via crates.io Trusted Publishing — no standing registry credential
  exists (v0.9.0 milestone). Nothing in this phase may introduce or request one.

### Agent / human act boundary

- **D-01:** The agent pushes `feature/phase-33` and opens the release PR to `main` via `gh`,
  with a curated body (gate re-seal summary, pointer to `CHANGELOG.md` `[0.10.0]`, pointer to
  `37-CI-EVIDENCE.md`). This is what unlocks the CI `coverage` run SC2 needs.
- **D-02:** The **maintainer** performs both irreversible acts — merging the PR and pushing the
  `v0.10.0` tag — at one blocking checkpoint. The agent hands over the exact merge method (D-04)
  and the exact `git tag -a v0.10.0 <merge-sha>` + push commands, then stops. The agent never
  merges to `main` and never pushes a `v*` tag. — **Reversibility:** one-way — the tag push
  triggers `release.yml`, which publishes to crates.io; a published version can be yanked but
  never deleted or re-used.
- **D-03:** §11 is ticked **after** the PR's CI is green. Order: local re-seal → push → PR CI
  green including `coverage` ≥ 82 % → evidence appended → maintainer ticks §11 → that tick commit
  is pushed → CI re-runs on the true final SHA → merge. The tick is the last content commit on
  the branch, so the maintainer signs what is visible; the accepted cost is one extra CI cycle on
  a docs-only commit.
- **D-04:** The PR is merged with a **merge commit** — not squash, not rebase. This is the
  v0.9.0 precedent (tag on merge commit `0b5d4106`), the literal wording of SC3 and Phase 29
  D-21, and it keeps every SHA cited across 204 plans' SUMMARYs and audits (e.g. BUG-01's
  `b2d05045` → `8d5ef333`) reachable from `main`. — **Reversibility:** one-way — history on a
  protected `main` cannot be rewritten after the tag points at it.

### Evidence homes & requirement ID

- **D-05:** Phase 37's CI evidence lives in a **new `37-CI-EVIDENCE.md`** in this phase's
  directory, in the house form of `27/28/29/33-CI-EVIDENCE.md` (local sweep table + CI-run
  table). It carries: the pre-merge runs on the PR head, the runs on the post-§11-tick final SHA,
  the dry-run dispatch (D-13), the post-merge runs on the merge commit, the real release run, and
  the registry table (D-08). `29-CI-EVIDENCE.md` and `33-CI-EVIDENCE.md` each get **one dated
  forward-pointer line** and are otherwise untouched — editing a verified phase's table would
  stale its VERIFICATION. The STATE.md sentence that currently names `33-CI-EVIDENCE.md` as the
  destination for the pre-merge run is corrected to name `37-CI-EVIDENCE.md`.
- **D-06:** SC1's "appended to the Phase 29 acceptance audit" is one **new dated re-seal
  section** appended to `29-ACCEPTANCE-AUDIT.md`: a seven-row gate table — (1) `MIGRATION.md` no
  "TBD" and §9.2 ↔ semver-checks allowlist row-for-row, (2) `v0_9_config_boot`, (3) OpenAPI
  golden diff, (4) `cargo semver-checks`, (5) MSRV job, (6) `make publish-dry-run` in dependency
  order, (7) `CHANGELOG.md` `[0.10.0]` complete — each row with command, SHA, result, and a
  pointer to `37-CI-EVIDENCE.md` for run IDs. Existing rows (the Phase 29 and Phase 33 readings)
  are not edited. The §11 box the maintainer ticks therefore sits in the same file that shows
  what was re-verified.
- **D-07:** The requirement is a newly minted **`SHIP-05`** — "v0.10.0 is released": gates
  re-sealed on the final commit, tag on the `main` merge commit, every publishable crate on
  crates.io at `0.10.0`, release evidence in MILESTONES.md. `SHIP-04` ("releasable", `[x]`
  Complete against Phase 29) is **not** extended, re-opened or edited. `SHIP` is an existing
  prefix, so no new prefix is spent (ROADMAP extension protocol item 3). The traceability table
  gains `SHIP-05 | Phase 37`, and ROADMAP Phase 37's `**Requirements**: TBD` line is replaced.
- **D-08:** SC4's registry proof is a **sparse-index query per publishable crate**
  (`index.crates.io`), recorded as a table in `37-CI-EVIDENCE.md` with crate name, version
  `0.10.0`, checksum and `yanked = false`. The crate list is **derived from `cargo metadata`**
  (`publish != false`), never hard-coded — the docs still say "eleven" and `paladin-eval`
  (Phase 28, ADR-0048) may make it twelve. MILESTONES.md gets a v0.9.0-style prose entry (tag,
  merge SHA, release run ID, crate count, "registry-verified") pointing at that table. No
  consumer smoke-build, and the workflow's own log is not accepted as proof on its own.

### Post-merge bookkeeping

- **D-09:** The GSD phase **stays open across the merge**. Plans split into a **pre-merge wave**
  (re-seal, PR, pre-merge evidence, §11 hand-off) and a **post-tag wave** (post-merge evidence,
  registry verification, MILESTONES.md entry), with the maintainer's merge + tag checkpoint
  (D-02) between them. The phase is verified only when its criteria are observably true. This is
  a **deliberate departure from Phase 29 D-21's "close on pre-merge evidence"**: Phase 29's goal
  was "releasable", this phase's goal is "released", and a VERIFICATION that passes SC3-4 by
  promise is exactly what the goal line rules out.
- **D-10:** Post-tag commits reach the protected `main` through a **`chore/37-close` branch cut
  from the tagged `main`** and a second, small, docs-only PR that the maintainer merges (house
  precedent: `chore/15.1-close-out`, `chore/20-close`, `chore/21-close`). No further commits go
  onto `feature/phase-33` after the merge, and the ruleset is never bypassed. The tagged commit
  stays exactly what was released.
- **D-11:** `/gsd-complete-milestone v0.10.0` is **not** run by any plan in this phase. Phase
  37's last plan ends at "ready to close": the MILESTONES.md release record is written and the
  `/gsd-audit-milestone` prerequisites are listed. The phase verifies on SC1-SC4 plus SC5's
  **preconditions**; the maintainer then runs `/gsd-complete-milestone` on `chore/37-close` so
  the archive moves ride the same PR. Reason: a plan must not archive the phase directory it is
  executing from, and verify-work must find its artifacts where they were written.
- **D-12:** Waiting is a **hard stop with a resume file**, never a parked watcher. At the merge
  + tag checkpoint the agent writes a `.continue-here.md` / pause hand-off naming the exact
  resume condition — *tag `v0.10.0` exists on `origin` and points at a commit contained in
  `main`* — and stops. The first task of the post-tag wave re-verifies that condition before
  doing anything else. The same rule covers the ~tens-of-minutes `release.yml` run: no
  long-lived `gh run watch` agent is left parked.

### Rehearsal & red-gate policy

- **D-13:** The pipeline is **rehearsed by a `workflow_dispatch` of `release.yml` with
  `dry_run=true`, against the merge commit, after the merge and before the real tag** — test
  suite, pre-publish consistency gate and `cargo publish --dry-run` in dependency order all run
  with nothing published and no version burned. The run ID goes in `37-CI-EVIDENCE.md`. **Open
  for the researcher:** the dispatch's `tag` input is `required` and `verify-tag-source` insists
  the ref is contained in `main` — establish what that input accepts before `v0.10.0` exists. If
  a dry run cannot be dispatched without a real tag, fall back to going straight to the tag and
  record why; do **not** substitute an rc tag (an rc burns a crates.io version and needs the
  twelve-manifest re-bump Phase 29 D-18 rates "costly").
- **D-14:** **Any red gate is a stop.** If a re-sealed gate fails — locally or in PR CI — the
  agent records the finding (D-00d) with the command, SHA and failing output, then halts at a
  blocking checkpoint. It fixes **nothing**, not even a one-line doc or citation fix; the
  maintainer decides what happens next (including whether it deserves a 36.2). This is stricter
  than Phase 29 D-12's bounded fix set, by explicit maintainer choice for release day.
- **D-15:** The single exception to D-14 is a **pure infrastructure flake** — runner timeout,
  registry 5xx, image-pull failure — where the log shows no test or gate assertion failed. The
  agent may `gh run rerun --failed` **exactly once**; both the red run ID and the re-run ID go in
  `37-CI-EVIDENCE.md` with the log line that justified the classification. A second red, or any
  assertion failure, is a D-14 stop. A red caused by the retired Docker Hub MinIO image on a
  non-rebased tree is a **real failure, not a flake**.
- **D-16:** If the **real** release run goes red after the tag — the pre-publish gate blocks, or
  publishing stops part-way through the dependency order — the agent **diagnoses read-only and
  the maintainer acts**. The agent follows `docs/src/appendix/release-recovery.md` §1-§2: queries
  the sparse index to establish exactly which crates reached crates.io, reads the run, names the
  matching runbook section and failure code, writes that to `37-CI-EVIDENCE.md`, then hard-stops.
  Completing forward (§3), any re-dispatch, and any yank (§5 — maintainer-only by the runbook)
  belong to the maintainer. The agent never triggers a publish. — **Reversibility:** one-way — a
  half-published dependency chain on crates.io is permanent state; only forward completion or a
  yank changes it.

### Planning-time addenda (added 2026-09-18 at `/gsd-plan-phase 37`, after research)

Dated additions under D-00d's amend-at-source rule — nothing above this heading was edited.

- **D-17:** `paladin-eval`'s first publish is bootstrapped by the **maintainer only**, with a
  **dependency-free placeholder version**, at a blocking human-action checkpoint in the
  **pre-merge wave**. Research Q3 (`37-RESEARCH.md`) found `paladin-eval` has never been published
  (sparse index `404`, re-verified by the orchestrator 2026-09-18) and that crates.io Trusted
  Publishing cannot perform a crate's first publish — so an unassisted release run would publish
  ten of twelve crates, fail at `paladin-eval` (position 11) and skip the `paladin-ai` facade. A
  pre-tag publish of the real `0.10.0` is impossible: it depends on five workspace crates at
  `^0.10.0` that reach the registry only during the release run. The maintainer therefore
  publishes a placeholder `paladin-eval` (e.g. `0.0.1`, no dependencies) from a scratch directory
  **outside the repository** using a short-lived crates.io token, links the crate's Trusted
  Publisher with the same triple as the other eleven crates (repository
  `DF3NDR/paladin-dev-env`, workflow `release.yml`, environment `crates-io`), then revokes the
  token. The agent never sees, requests, stores or handles the credential: it hands over the
  steps, then verifies read-only that `paladin-eval` resolves on the sparse index **before** the
  D-02 tag command is handed over. The Trusted Publisher link itself is attested by the maintainer
  (it is not publicly queryable — the existing eleven rows read "linked (reported)" for the same
  reason); after the release, a non-null `trustpub_data` on `paladin-eval` `0.10.0` is the
  observable proof. The real `0.10.0` publish of all twelve crates stays with `release.yml` via
  OIDC, and no tree change is made for the placeholder. This is consistent with D-00g — the token
  is short-lived, maintainer-held and revoked, never a standing credential and never in the
  repository, CI secrets or the agent's environment. The crates.io policy claim is MEDIUM
  confidence (blog post plus search synthesis; the primary docs page is JS-rendered), and the
  checkpoint text must tell the maintainer so. Chosen by the maintainer over a planned
  complete-forward at release time (a deliberately red release run and a half-published chain).
  — **Reversibility:** one-way — a published placeholder version can be yanked but never deleted.

Orchestrator-verified tree facts the planner applies under D-00f (not new maintainer decisions):

- D-06's target file: `29-ACCEPTANCE-AUDIT.md` is a 29-line **pointer**; the audit body and the
  literal sign-off box SC1 names (`- [ ] **The v0.10.0 tag may be cut**`, line ~1549) live in the
  corpus document `.project/v0.10.0/09-program-acceptance-audit.md`. D-06's own rationale — the box
  sits in the same file that shows what was re-verified — is met by following the Phase 33
  precedent exactly: the dated re-seal section is appended to the **corpus document** as a new
  `## 12.` section, **and** the pointer file gains one dated re-seal paragraph naming it. The
  existing §11 text and box wording are not edited.
- D-13's open question is answered (research Q1): `release.yml` cannot be dry-run-dispatched before
  a ref literally named `v0.10.0` exists, so D-13's own documented fallback applies — no dispatch,
  no rc tag, no shadow tag; the reason is recorded in `37-CI-EVIDENCE.md`.

### Claude's Discretion

- Who dispatches the D-13 dry run (it publishes nothing). Default: the agent may dispatch it
  once the maintainer reports the merge is done; if the token lacks `workflow` scope, hand the
  exact `gh workflow run` command to the maintainer instead.
- The local re-seal's command order and how outputs are captured, provided every one of D-06's
  seven rows has a command, a SHA and a result.
- The release PR's title and body wording, and the MILESTONES.md entry's prose, within the
  v0.9.0 entry's shape.
- Whether `feature/phase-33` is deleted after the merge — leave it to the maintainer's GitHub
  setting; no plan task depends on it either way.
- Plan count and wave layout, provided D-09's two-wave split around the D-02 checkpoint holds.
- If `main` moves before the merge: "the final commit" is re-established by updating the branch
  and re-running the re-seal from the top; §11 is re-confirmed by the maintainer against the new
  SHA (D-03's order applies again).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and source decisions
- `.planning/ROADMAP.md` §"Phase 37: v0.10.0 Crate Release" — goal, five success criteria,
  dependencies, Source line; §extension protocol item 3 (requirement-prefix rule, D-07)
- `.planning/phases/29-program-gates-release/29-CONTEXT.md` — D-12, D-17, D-18, D-19, D-20,
  D-21, D-23, D-24: the gate set, the human-only sign-off, the bump-without-tag, the two-SHA
  rule, the CI-evidence form
- `.planning/REQUIREMENTS.md` §SHIP-01…SHIP-04 (lines ~300-321) and the traceability table
  (~670-673) — where `SHIP-05` is minted (D-07)

### Evidence files this phase writes to or points from
- `.planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md` — gets the dated re-seal
  section (D-06); holds the §11 box only the maintainer ticks (D-00a)
- `.planning/phases/29-program-gates-release/29-CI-EVIDENCE.md` — form to mirror; gets one
  forward-pointer line (D-05)
- `.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md` — the previous re-seal;
  form to mirror; gets one forward-pointer line (D-05)
- `.planning/MILESTONES.md` §"v0.9.0 Security Tooling" (lines ~3-34) — the release-record shape
  the v0.10.0 entry follows (D-08)
- `.planning/STATE.md` §Project Reference (lines ~28-39) — carries the sentence D-05 corrects

### Release mechanics
- `.github/workflows/release.yml` — tag trigger, `workflow_dispatch` `tag` + `dry_run` inputs
  (lines 3-17), `verify-tag-source` (line 29), `check-release-consistency` (line 405), the
  dry-run mode switch (lines ~584-648) — the D-13 research question lives here
- `docs/src/appendix/release-automation.md` — Operator Guide (the authoritative step-by-step for
  cutting a release, D-00c) and the canonical publish order
- `docs/src/appendix/release-checklist.md` — the manual release checklist
- `docs/src/appendix/release-recovery.md` — §1-§2 (what reached crates.io, reading the run) are
  the agent's read-only scope under D-16; §3, §5 and §6 are the maintainer's
- `docs/src/appendix/branch-protection.md` — main-only tag policy and the "Protect main branch"
  ruleset (why D-10 uses a chore PR)
- `docs/src/contributing/development-setup.md` §"Cutting a release" (lines ~640-655) — why
  `make release` is not the path (D-00c)
- `Makefile` — `release-check` (577), `publish-dry-run` (587), `release` (601),
  `api-surface` (383)
- `scripts/publish-crates.sh` — the real publish carrier (Phase 29 D-20)

### Gate inputs
- `MIGRATION.md` §9.2 and the `cargo semver-checks` allowlist — row-level set-equality
  (Phase 29 D-04)
- `CHANGELOG.md` `[0.10.0]` — completeness gate; Phases 30-36.1 appended to it
- `.planning/decisions/0006-coverage-gate.md` — ADR-0006, the 82 % floor (D-00e)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `make publish-dry-run` (depends on `release-check`): the local half of gate row 6 — already
  fixed in Phase 29 to match `cargo publish --workspace --dry-run`.
- `release.yml`'s `dry_run` input: a complete, already-built rehearsal path (D-13) — nothing
  new needs writing to rehearse.
- `docs/src/appendix/release-recovery.md`: a named section per pre-publish failure code
  (`MISMATCH`, `ZERO_PACKAGES`, `MISSING_TAG`, `CHANGELOG_MISMATCH`, `CI_MISMATCH`,
  `CI_LOOKUP_FAILED`, `MISSING_SHA`, combined) — D-16's diagnosis is a lookup, not an
  investigation.
- `33-CI-EVIDENCE.md` / `29-CI-EVIDENCE.md`: copy the table structure verbatim for
  `37-CI-EVIDENCE.md`.
- Pre-push API-surface gate + `make api-surface` (added 2026-09-16): will run on D-01's push;
  the baseline must not move in this phase.

### Established Patterns
- Amend-at-source with dated additions, original text retained — governs D-05, D-06, D-07.
- `chore/NN-close` branch + small PR for post-close bookkeeping — governs D-10.
- CI-evidence files are per-phase and sealed at the SHA they were verified on; SUMMARY or
  evidence edits after VERIFICATION make the verification stale — governs D-05 and D-09.
- Blocking human checkpoints obtained through the runtime's interactive question mechanism, with
  provenance recorded in the SUMMARY (Phase 12 precedent) — the form D-02 and D-14 stops take.

### Integration Points
- `gh` CLI: push, PR open, run listing, `run rerun --failed` (D-15), `workflow run` (D-13). The
  devcontainer's `GH_TOKEN` has known read-only-token failure modes — every `gh` write needs a
  fallback of "hand the exact command to the maintainer".
- crates.io sparse index (`index.crates.io`): read-only HTTP, no credential (D-08, D-16).
- `cargo metadata`: the single source for the publishable-crate list (D-08).

</code_context>

<specifics>
## Specific Ideas

- The maintainer wants release day to be **conservative past the point of convenience**: the
  non-recommended "always stop" was chosen over a bounded fix set (D-14). Planner and executor
  should read that as the governing temperament for every unlisted situation — when unsure, stop
  and record, do not improvise.
- "Released, not merely releasable" is the phase's own contrast with SHIP-04 and the reason for
  both D-07 (a new ID rather than a re-opened one) and D-09 (no passing-by-promise).
- The v0.9.0 MILESTONES.md entry is the model for the v0.10.0 one: tag, merge SHA, release run
  ID, crate count, "registry-verified", Trusted Publishing.

</specifics>

<deferred>
## Deferred Ideas

- **Correcting the stale "eleven crates" count** in `docs/src/contributing/development-setup.md`
  and anywhere else it appears, if `cargo metadata` shows a different number — a docs-currency
  fix for v0.11.0; under D-14 this phase does not edit docs pages. Record it as a finding if
  observed.
- **A consumer smoke test** (`cargo add paladin@0.10.0` in a scratch project, cold build) as a
  standing post-release check — considered for D-08 and not chosen; a candidate for a future
  release-tooling phase.
- **Resolve-then-rehearse without a tag**: if D-13's research shows `release.yml` cannot be
  dry-run before a tag exists, making the workflow accept a bare SHA is a CI change for a later
  milestone, not this phase.
- **Carried from Phase 36.1 `<deferred>`, unchanged:** `cargo test --workspace --all-features`
  in CI; removing the eight crate-level rustdoc allows (309 hidden diagnostics); regenerating
  `doc-coverage-report.md`; the Phase 25 / Phase 28 register question, which is
  `/gsd-audit-milestone`'s at close.

### Reviewed Todos (not folded)
- **Verify local `make coverage` reproduces CI's 82.39 % figure**
  (`todos/pending/2026-08-13-verify-local-coverage-reproduction.md`, match score 0.6) — not
  folded: Phase 36.1 D-23 re-homed it to the maintainer with a re-check date because this
  devcontainer has no Docker. SC2 here uses CI's own `coverage` job instead.
- **Evaluate replacing MinIO with RustFS in the dev/test stack**
  (`todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`, match score 0.6) — not
  folded: Phase 36.1 D-24 re-homed it to v0.11.0 as an adapter build; keyword match only, no
  bearing on cutting a release.

</deferred>

---

*Phase: 37-v0.10.0 Crate Release*
*Context gathered: 2026-09-18*
