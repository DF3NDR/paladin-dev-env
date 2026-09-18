# Phase 37: v0.10.0 Crate Release - Pattern Map

**Mapped:** 2026-09-18
**Files analyzed:** 10 (evidence/bookkeeping documents; no `src/`/`crates/*/src/` files — this is a
release-mechanics phase, not a code phase, per `37-CONTEXT.md` `<domain>`)
**Analogs found:** 10 / 10

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md` (new) | evidence-doc | batch (gate sweep + CI-run capture) | `33-CI-EVIDENCE.md` / `29-CI-EVIDENCE.md` | exact |
| `.project/v0.10.0/09-program-acceptance-audit.md` — new `## 12.` section (append) | evidence-doc | batch | same file's own `## 11.` section | exact |
| `29-ACCEPTANCE-AUDIT.md` (pointer) — one dated re-seal paragraph (append) | evidence-doc | transform (pointer update) | same file's existing "Re-sealed on `69500c9b…`" paragraph | exact |
| `29-CI-EVIDENCE.md` / `33-CI-EVIDENCE.md` — one dated forward-pointer line each | evidence-doc | transform (amend-at-source) | none needed — pattern is the sibling files' own dated-addition convention (see `29-ACCEPTANCE-AUDIT.md`'s re-seal paragraph) | role-match |
| `.planning/MILESTONES.md` — v0.10.0 entry (append) | config/record | CRUD (append record) | the `## v0.9.0 Security Tooling` entry, same file | exact |
| `.planning/REQUIREMENTS.md` — mint `SHIP-05` + traceability row | config/record | CRUD (append row) | `SHIP-04` row (~line 313) + traceability table rows (~670-680) | exact |
| `.planning/ROADMAP.md` — Phase 37 `**Requirements**:` line | config/record | transform (string replace) | any other phase's `**Requirements**:` line | exact |
| `.planning/STATE.md` — one sentence corrected | config/record | transform (verbatim substring replace) | n/a — quote-and-replace, no analog needed | exact |
| `.planning/phases/37-.../.continue-here.md` (D-12 hard-stop hand-off) | evidence-doc | event-driven (pause/resume) | `.claude/gsd-core/workflows/pause-work.md` template; no prior `.continue-here.md` found in git history for this repo (checked: none matched `*continue-here*` in `git log --all --diff-filter=A --name-only`) | role-match |
| `chore/37-close` branch + docs-only PR body | config/record | request-response (PR) | precedent PRs `chore/21-close` (#49), `chore/20-close` (#47), `chore/15.1-close-out` (#32) — structure consistent, naming varies | role-match |

Blocking-checkpoint task shape (D-02, D-03, D-13's dry-run hand-off, D-17's paladin-eval bootstrap)
is its own cross-cutting pattern — see **Shared Patterns** below; it is not a "file" but governs
how every plan in this phase must be authored.

## Pattern Assignments

### `.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`

**Analog:** `.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md` (and
`29-CI-EVIDENCE.md` for the same shape one phase further back)

**Header/provenance pattern** (33-CI-EVIDENCE.md lines 1-19):
```markdown
# Phase 33 Commissary In-Tree Adoption — CI Evidence Record (plan 33-06)

**Phase:** 33-commissary-in-tree-adoption
**Branch:** `feature/phase-33` (...)
**Head SHA at sweep time:** `69500c9b...` — the tip of `feature/phase-33` at dispatch of this
plan, carrying plans 33-01 through 33-05 ... This is the "final commit" D-24 names: no plan after
this one changes source, and this plan's own two commits are docs-only (this file, and the audit
§11 append), so the gate sweep below is valid evidence for the phase's actual shipped tree.
**Written:** 2026-09-16

This record has two parts, in the `29-CI-EVIDENCE.md` shape: a **Local sweep** (every gate this
devcontainer can run without Docker...) and a **CI-run table** for the newest workflow runs
available to this branch.
```
For Phase 37, adapt: SHA/branch will change across the two waves (pre-merge head, post-§11-tick
SHA, merge commit, tagged commit) — do NOT collapse these into one "Head SHA" line; use one
provenance line per wave-relevant SHA (D-05's evidence needs more than two run-table entries per
RESEARCH.md Q7).

**Local sweep table pattern** (33-CI-EVIDENCE.md lines 21-30):
```markdown
## Local sweep

| # | Command | Result (verbatim/summarized) | Verdict |
|---|---------|-------------------------------|---------|
| 3 | `grep -c TBD MIGRATION.md` | `0` | ✅ PASS |
| 4 | `make check-migration-allowlist` | 15 `crate|type` pairs ... set-equal in both directions | ✅ PASS |
| 6 | `cargo test --features web-server --test v0_9_config_boot` | `test result: ok. 9 passed; 0 failed; 0 ignored` — same 9-test count `29-CI-EVIDENCE.md` row 8 recorded | ✅ PASS |
| 8 | `cargo semver-checks check-release --package paladin-ai --default-features --baseline-version 0.9.0` | `Checking paladin-ai v0.9.0 -> v0.10.0 (major change)` / `0 checks: 0 pass, 254 skip` / `Summary no semver update required` | ✅ PASS |
```
Copy this table shape verbatim for D-06's seven gates (RESEARCH.md Q5 gives the exact command per
gate). Note the `grep -c` zero-count exit-code trap called out in RESEARCH.md Pitfall 4 — do not
let a passing gate 1 look like a script failure.

**CI-run table pattern:** not shown in the excerpted range above but is the file's second `##`
section, columns `Workflow | Run ID (URL) | Conclusion | SHA | Notes` per RESEARCH.md Q7 — mirror
that column set exactly; this phase's version needs rows for: pre-merge PR-head run(s), the run on
the post-§11-tick final SHA, the (skipped, documented) dry-run dispatch non-attempt, the post-merge
run on the tagged merge commit, the real release run, and D-08's registry table as its own
sub-section.

**Closing pattern:** every evidence file ends with a "Summary and what remains" prose section —
model Phase 37's on `33-CI-EVIDENCE.md`'s own closing paragraphs (not excerpted above but present
at the file's tail) rather than inventing new structure.

---

### `.project/v0.10.0/09-program-acceptance-audit.md` — new `## 12.` section

**Analog:** the same document's own `## 11. Re-seal after Phases 30-33 (Phase 33, COMM-04)`
section, referenced (not directly read this session — flagged `[CITED]` in RESEARCH.md Q5) via its
pointer file's description:

```markdown
**Re-sealed on `69500c9b51a37f11215037c49318d76ea017dab3`, 2026-09-16.** Phases 31, 32 and 33
changed public API after this audit's ten sections were sealed above, so Phase 33 (COMM-04)
re-ran the full release-gate list on its own final commit and appended the result as a new
`## 11. Re-seal after Phases 30-33 (Phase 33, COMM-04)` section to the corpus document — this
pointer's ten-section scope is otherwise unchanged. Full verbatim evidence lives in
`.../33-CI-EVIDENCE.md`. This is the precondition ADR-0051 sets for cutting the `v0.10.0` tag;
§11 adds one further unticked, human-only sign-off box for that decision, alongside — never in
place of — the seven boxes named above.
```

Phase 37 must: (1) read `.project/v0.10.0/09-program-acceptance-audit.md` sections 10-11 directly
during planning (RESEARCH.md flags this as not yet directly read — an Open Question the planner
must close), (2) append a new `## 12. Re-seal for v0.10.0 release (Phase 37, SHIP-05)` section in
the same seven-row-gate-table shape, each row carrying command + SHA + result + a pointer to
`37-CI-EVIDENCE.md`, (3) leave the existing `## 11.` text and its sign-off box wording untouched
(D-06: "Existing rows are not edited"), (4) add the human-only sign-off box for the maintainer's
§11-or-new-box tick — confirm during planning whether Phase 37 reuses the existing §11 box or a
freshly worded one is warranted (RESEARCH.md Q5 flags this as unresolved without a direct read).

---

### `29-ACCEPTANCE-AUDIT.md` (pointer file) — one dated re-seal paragraph

**Analog:** the file's own prior paragraph (same file, different date) — copy this exact shape,
change the SHA/phase/date/section-number:

```markdown
**Re-sealed on `69500c9b51a37f11215037c49318d76ea017dab3`, 2026-09-16.** Phases 31, 32 and 33
changed public API after this audit's ten sections were sealed above, so Phase 33 (COMM-04)
re-ran the full release-gate list on its own final commit and appended the result as a new
`## 11. Re-seal after Phases 30-33 (Phase 33, COMM-04)` section to the corpus document ...
```
becomes (shape, not literal text — fill in real values at execution time):
```markdown
**Re-sealed on `<final-SHA>`, 2026-09-18.** Phase 37 re-ran the full release-gate list for the
v0.10.0 release itself and appended the result as a new `## 12. Re-seal for v0.10.0 release
(Phase 37, SHIP-05)` section to the corpus document — this pointer's ten-section scope is
otherwise unchanged. Full verbatim evidence lives in `.../37-CI-EVIDENCE.md`.
```

---

### `29-CI-EVIDENCE.md` / `33-CI-EVIDENCE.md` — forward-pointer lines

No dedicated analog file exists for "a dated forward-pointer line appended to an otherwise-frozen
evidence file" as its own pattern, but the mechanism is identical to the re-seal paragraph above:
one short, dated, italicized or plain addendum sentence naming the newer file, appended at the
very end of the file, with nothing above it touched. Example shape:
```markdown
*2026-09-18 addendum: Phase 37's pre-merge/post-tag CI evidence for the v0.10.0 release lives in
`.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`; this file's own rows are unchanged.*
```

---

### `.planning/MILESTONES.md` — v0.10.0 entry

**Analog:** `## v0.9.0 Security Tooling (Shipped: 2026-09-01)`, same file, lines 1-34

**Metadata-line pattern** (lines 3-10):
```markdown
## v0.9.0 Security Tooling (Shipped: 2026-09-01)

**Phases completed:** 4 phases (18-21), 25 plans
**Requirements:** 20/20 satisfied (SAST-01…04, PUB-01…05, PUBOPS-01…05, ARTIFACT-01…06)
**Timeline:** 2026-08-24 → 2026-09-01 (8 days, 240 commits)
**Git range:** `48ac11a5` → `3957d701`
**Closeout type:** override_closeout — 0 verification overrides (all 4 phases `passed`), 1 open
artifact acknowledged ...
**Audit:** `milestones/v0.9.0-MILESTONE-AUDIT.md` (status `tech_debt` ...)
```

**Post-amendment prose block pattern** (lines 11-18) — this is the shape Phase 37's entry should
copy (per RESEARCH.md Q7: model on the *post-amendment* block only, no strikethrough needed since
this release is clean, not retroactive):
```markdown
... **v0.9.0 was released for real** through the documented PR-merge flow: PR #50 bumped all
twelve manifests to `0.9.0` and curated the changelog, tag `v0.9.0` was cut on merge commit
`0b5d4106`, and release run `33542459191` completed fully green — all eleven crates published
to crates.io at `0.9.0` via Trusted Publishing (registry-verified), stable GitHub release with
binaries, digest-bound image and `SHA256SUMS`.
```
For v0.10.0: state tag, merge SHA, release run ID, crate count **twelve** (not eleven —
RESEARCH.md Q3/Q8 stress this is a live, corrected fact), "registry-verified", Trusted Publishing —
same clause order, no strikethrough wrapper.

**Delivered / Key accomplishments pattern** (lines 20-27): narrative prose sections after the
metadata block — reuse the same two-heading shape (`**Delivered:**`, `**Key accomplishments:**`).

---

### `.planning/REQUIREMENTS.md` — mint `SHIP-05`

**Analog:** `SHIP-04` row, same file (~line 313):
```markdown
- [x] **SHIP-04**: v0.10.0 is releasable: all workspace crates at `0.10.0` with changelogs
  updated, `cargo publish --dry-run` green for every publishable crate in dependency order,
  mdBook + rustdoc updated with no new broken intra-doc links, and the semver and MSRV CI jobs
  green on the release commit (overview §5 DoD 1, 3, 6, 7; X-08)
```
Copy this exact `- [ ] **SHIP-05**: <one-line statement> (<parenthetical source pointer>)` shape.
Per D-07 the statement is "v0.10.0 is released": gates re-sealed on the final commit, tag on the
`main` merge commit, every publishable crate on crates.io at `0.10.0`, release evidence in
MILESTONES.md.

**Traceability table row pattern** (~line 673):
```markdown
| SHIP-04 | Phase 29 | Complete |
```
Add `| SHIP-05 | Phase 37 | Complete |` directly after it (do not touch the SHIP-04 row).

---

### `.planning/ROADMAP.md` — Phase 37 Requirements line

**Analog:** any other phase's `**Requirements**:` line (e.g. Phase 29's, which reads
`**Requirements**: SHIP-01, SHIP-02, SHIP-03, SHIP-04`, by the same file's established convention).
Replace Phase 37's current `**Requirements**: TBD` with `**Requirements**: SHIP-05` verbatim
(D-07's own instruction), touching nothing else in that phase's ROADMAP block.

---

### `.planning/STATE.md` — sentence correction

No copy-pattern needed — this is a scoped, exact substring replace. Current sentence (verified
live, STATE.md lines ~37-39):
```
Before that, push `feature/phase-33` (or open the PR) so the CI `coverage` job supplies the one
gate this devcontainer cannot measure and a real pre-merge run is appended to
`33-CI-EVIDENCE.md`'s CI-run table; the tag is cut on the `main` merge commit by `release.yml`
per Phase 29 D-21's two-SHA rule.
```
Replace only the substring `` `33-CI-EVIDENCE.md`'s CI-run table `` with
`` `37-CI-EVIDENCE.md`'s CI-run table ``. Leave every other word — including the now-superseded
"Next: `/gsd-complete-milestone v0.10.0`" line above it, which D-11 says is not run this phase —
alone unless the planner is separately told to also fix that line; D-05 only names the
CI-evidence-table sentence.

---

### `.planning/phases/37-.../.continue-here.md` (D-12 hard stop)

**Analog:** `.claude/gsd-core/workflows/pause-work.md` — the generic template this repo has never
yet instantiated as a phase-level `.continue-here.md` (checked: no historical file matched
`*continue-here*` under `git log --all --diff-filter=A --name-only`, so there is no in-repo
precedent file to excerpt verbatim; the workflow doc is the pattern source).

**Structure to follow** (pause-work.md's own `<gather>` step, ten numbered items — reproduce as
section headings in the written file): current position, work completed, work remaining,
decisions made, blockers/issues, **human actions pending** (this is the load-bearing section for
D-12 — name the exact resume condition here verbatim: *"tag `v0.10.0` exists on `origin` and
points at a commit contained in `main`"*), background processes (none expected — D-12 explicitly
forbids a parked `gh run watch`), files modified, outstanding async jobs, blocking constraints
(severity `blocking`/`advisory`).

Also write the machine-readable sibling `.planning/HANDOFF.json` per the same workflow doc — the
markdown alone is not the complete pattern.

---

### `chore/37-close` branch + docs-only PR

**Analog:** precedent PRs `chore/21-close` (#49), `chore/20-close` (#47),
`chore/15.1-close-out` (#32) — confirmed via `git log --all --grep` (RESEARCH.md Q9). Structure is
consistent (small branch cut from the tagged/closed state, its own PR, merged by the maintainer)
but naming is inconsistent across history (`-close` vs `-closeout` vs `-close-out`). Use
`chore/37-close` exactly as D-10 specifies rather than matching any one historical spelling. Pull
the actual PR bodies read-only via `gh pr view 49 --json title,body` (and `#47`, `#32`) at plan
time for exact wording precedent — this mapper did not fetch them live to stay within the
read-only, no-branch-switch constraint, but the mechanism (`gh pr view <n> --json title,body`) is
the concrete command to run.

## Shared Patterns

### Amend-at-source, dated additions, nothing above the heading edited
**Source:** `29-ACCEPTANCE-AUDIT.md`'s own re-seal paragraph; D-00d; MILESTONES.md's v0.9.0
strikethrough-amendment convention (a stricter variant, not needed here since no reversal is being
recorded).
**Apply to:** `29-CI-EVIDENCE.md`, `33-CI-EVIDENCE.md`, `29-ACCEPTANCE-AUDIT.md`, the corpus
acceptance audit's new `## 12.` section, STATE.md's one-sentence fix. Every one of these is an
*addition*, never a rewrite of existing content.

### House evidence-file shape: Local sweep table + CI-run table + closing prose
**Source:** `29-CI-EVIDENCE.md` / `33-CI-EVIDENCE.md` (both same shape).
**Apply to:** `37-CI-EVIDENCE.md` in full.

### Blocking-human checkpoint task shape (`gate="blocking-human"`, not the default `blocking`)
**Source:** `.planning/phases/36-.../36-13-PLAN.md` and `36.1-.../36.1-14-PLAN.md`'s
`<task type="checkpoint:human-verify" gate="blocking">` tasks, e.g.:
```xml
<task type="checkpoint:human-verify" gate="blocking">
  <name>Task 3: Push the branch and record the real CI run that proves the new required-check step</name>
  <files>.../36-CI-EVIDENCE.md</files>
  <read_first>
    - .../33-CI-EVIDENCE.md (the house `NN-CI-EVIDENCE.md` shape ...)
    - .../36-evidence/36-12-closing-measurement.txt (...)
    - .github/workflows/ci.yml lines 55-70 and 530-590 (...)
  </read_first>
  <what-built> ... </what-built>
  <how-to-verify> ... </how-to-verify>
</task>
```
**Apply to, with one critical deviation:** D-02's merge+tag checkpoint, D-03's §11-tick checkpoint,
D-13's dry-run non-dispatch decision point, and D-17's `paladin-eval` bootstrap checkpoint all need
`gate="blocking-human"` specifically — NOT the plain `gate="blocking"` shown in the 36/36.1
precedent above. RESEARCH.md Q8 is explicit and load-bearing here: this project's
`.planning/config.json` has `_auto_chain_active: true`, so an ordinary `blocking` gate would
auto-approve and silently defeat D-02/D-17's human-only requirement. Every checkpoint task in this
phase's plans touching an irreversible act (merge, tag push, credential-bearing manual publish)
must use `gate="blocking-human"`; only genuinely re-checkable, non-credential, non-merge
checkpoints (if any) may use the plain `blocking` gate the 36-series precedent shows.

### Registry verification one-liner (D-08)
**Source:** `scripts/publish-crates.sh`'s `_pc_index_path` / `_pc_version_in_index` functions,
reproduced as a standalone curl+jq loop in RESEARCH.md Q4:
```bash
for name in $(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.publish == null) | .name'); do
  len=${#name}
  if [ "$len" -le 2 ]; then path="$len/$name"
  elif [ "$len" -eq 3 ]; then path="3/${name:0:1}/$name"
  else path="${name:0:2}/${name:2:2}/$name"; fi
  curl -s -H 'User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)' \
    "https://index.crates.io/${path}" \
  | jq -c --arg v "0.10.0" 'select(.vers == $v) | {name, vers, cksum, yanked}'
done
```
**Apply to:** the D-08 registry table in `37-CI-EVIDENCE.md`. Note the mandatory `User-Agent`
header (crates.io rejects requests without one) and that the crate list must come from live
`cargo metadata`, never a hard-coded twelve-name list (the docs' own stale "eleven" is the cautionary
example).

## No Analog Found

None — every file/section this phase writes has at least a role-match precedent in the existing
tree (see table above). The one near-miss is `.continue-here.md`, which has no *instantiated*
prior example in this repo's history, only the generic workflow template
(`.claude/gsd-core/workflows/pause-work.md`) — recorded above as a role-match, not a gap.

## Metadata

**Analog search scope:** `.planning/phases/{29,33,36,36.1}-*/`, `.planning/{MILESTONES,REQUIREMENTS,ROADMAP,STATE}.md`, `.project/v0.10.0/09-program-acceptance-audit.md` (pointer only — corpus body not re-read here, flagged for planner), `.claude/gsd-core/workflows/pause-work.md`, git history for `chore/*-close*` branches/PRs.
**Files scanned:** ~10 evidence/config files read in full or targeted excerpt; 2 workflow-file greps; 1 git-log precedent search.
**Pattern extraction date:** 2026-09-18
