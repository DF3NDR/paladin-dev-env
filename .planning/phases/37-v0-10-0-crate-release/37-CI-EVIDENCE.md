# Phase 37 v0.10.0 Crate Release — CI Evidence Record (plans 37-01 through 37-11)

**Phase:** 37-v0-10-0-crate-release
**Branch:** `feature/phase-33` (pre-merge wave); `chore/37-close` (post-tag wave, per D-10)
**Written:** 2026-09-18

This record follows the `29-CI-EVIDENCE.md` / `33-CI-EVIDENCE.md` house form: a **Local sweep**
table, a **CI-run table**, a dedicated **Registry verification (D-08)** section, a **Findings
carried forward (D-00d)** section, and a closing **Summary and what remains**. Unlike those two
single-wave records, this phase spans D-09's two-wave split around the D-02 merge+tag checkpoint,
so the **Provenance** block below carries one line per wave-relevant SHA rather than a single
"Head SHA" line — the four not yet known are recorded as `pending` and filled in by later plans in
this phase, never retrofitted into this paragraph's original text (D-00d amend-at-source).

---

## Provenance

- **Local re-seal head SHA (this plan, plan 37-01):** `522ab1d4c4c4b5a62a8bbbbc5e234b0a29edbadd` —
  the tip of `feature/phase-33` at dispatch of this plan (`docs(37): begin phase execution`).
- **PR head at open (D-01):** `pending` — filled in by the plan that pushes the branch and opens
  the release PR.
- **Post-§11-tick final SHA (D-03):** `pending` — filled in after the maintainer ticks §11 in
  `.project/v0.10.0/09-program-acceptance-audit.md` and that tick commit is pushed.
- **`main` merge commit (D-02, D-04):** `pending` — filled in after the maintainer merges the PR
  with the merge-commit method.
- **Tagged commit (D-02):** `pending` — filled in after the maintainer pushes the annotated
  `v0.10.0` tag; per D-00b this is provably the same commit as the merge commit above.

---

## Pre-flight measurements (this plan, plan 37-01)

Measured live at dispatch of this plan — not copied from any orchestrator-reported value:

| Measurement | Command | Result |
|---|---|---|
| Free space on `/workspace` | `df -BG --output=avail /workspace` | **18G** available |
| Working tree cleanliness | `git status --porcelain` | Empty — clean tree |
| `origin/main` vs `HEAD` divergence | `git fetch origin` (read-only) then `git rev-list --left-right --count origin/main...HEAD` | `0  500` — **0** commits in `origin/main` not in `HEAD` (main has not moved), **500** commits in `HEAD` not in `origin/main` |
| Unpushed commit count | `git status -sb` | `feature/phase-33...origin/feature/phase-33 [ahead 24]` — **24** local commits not yet on the branch's own remote tracking ref |
| Local re-seal head SHA | `git rev-parse HEAD` | `522ab1d4c4c4b5a62a8bbbbc5e234b0a29edbadd` |

**Reading these numbers:** the left count of the `origin/main...HEAD` divergence is `0`, so per
CONTEXT Discretion item 6 no "main has moved" re-seal-from-the-top condition applies at this
measurement point — this must be re-checked at each later plan's own dispatch, not assumed to
still hold. 18G free and a clean tree are recorded here specifically so that an ENOSPC or
dirty-tree abort in plan 37-02/37-03 is legible as an environment stop under D-14, never
mistaken for a red gate.

---

## Local sweep

| # | Command | Result (verbatim/summarized) | Verdict |
|---|---------|-------------------------------|---------|
| 1 | `COUNT=$(grep -c TBD MIGRATION.md \|\| true); echo "COUNT=${COUNT}"` (D-06 gate 1, first half — the no-placeholder-marker check; captured into a variable per Pitfall 4, never read from `grep -c`'s exit code, since a zero count makes `grep -c` exit `1`) | `COUNT=0` | ✅ PASS |

**Row 1 is the phase's proof that the whole evidence path works end to end on one thin
end-to-end slice** — the exact command later gate-1 re-seals in plan 37-02/37-03 will reuse, run
here for real, against the real tree, with a real captured count rather than a rehearsal. The
second half of D-06 gate 1 (`make check-migration-allowlist`, the §9.2 ↔ semver-checks allowlist
set-equality check) is deliberately **not** run by this plan — it belongs to the fuller D-06
re-seal sweep plan 37-02/37-03 owns; this task's scope is proving the path with one row, not
running the whole gate set early.

---

## CI-run table

**Opening note — the two read-only GitHub queries this plan ran (Task 1, step 5):**

`gh pr list --head feature/phase-33 --state all --json number,state` returned `[]` — **no PR
exists yet**, the expected pre-D-01 state.

`gh run list --branch feature/phase-33 --limit 5 --json workflowName,headSha,conclusion` did
**not** return an empty list — it returned 5 rows, all `"conclusion":"success"`, all at SHAs
older than this plan's own local re-seal head (`522ab1d4`): four rows at
`6fe5b70a1ff973e088a58184f57d5401c9f97895` (`ci.yml`, `codeql.yml`, `feature-flags.yml`,
`pre-commit`, all success) and one further row at `20195975c1c2665abb169b287fa178353d672bd2`
(`feature-flags.yml`, success). This is the same shape `29-CI-EVIDENCE.md` and
`33-CI-EVIDENCE.md` recorded for their own phases: proof that an earlier point on this branch was
fully green across the pushable workflows, **not** proof of anything about this phase's own
commits — none of the five rows' SHAs match `522ab1d4`. No `gh` command in this plan failed with
an auth or permission error; both queries returned normally.

No further rows are added to this table by this plan — the real pre-merge PR-head run(s), the
run on the post-§11-tick final SHA, the post-merge run on the tagged `main` merge commit, and the
real release run are each supplied by later plans in this phase, per the wave structure `37-PATTERNS.md`
and `37-RESEARCH.md` Q7 describe.

---

## Registry verification (D-08)

**Pre-bootstrap baseline for `paladin-eval` (this plan, plan 37-01, Task 1 step 4):**

```
curl -s -o /tmp/pe_pre.json -w '%{http_code}' \
  -H 'User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)' \
  https://index.crates.io/pa/la/paladin-eval
```

**HTTP status: `404`** (body: an S3-style `NoSuchKey` XML error, the sparse index's un-bootstrapped
response shape). No `vers` field is present, because no version has ever been published. This is
the **expected pre-bootstrap state** per D-17 — recorded here as the D-17 baseline, not as a
failure of this plan or of the release. Task 2 hands this fact, and the reason it matters, to the
maintainer as a blocking-human checkpoint. Task 3 re-runs this identical query after the
checkpoint resolves and records the maintainer's reply verbatim alongside the post-checkpoint
status.

The full twelve-crate registry table (every publishable crate at `0.10.0`, per D-08, derived live
from `cargo metadata`) is **not** run by this plan — v0.10.0 has not been published anywhere yet.
That table is written by the post-tag wave plan once the real release run has completed.

---

## Findings carried forward (D-00d)

Not populated by this plan. Task 3 appends the D-13 non-dispatch record and the first set of
carried documentation findings here; later plans append further findings as they are observed.
Nothing above this heading is edited by any later addition — additions are dated and appended
only.

---

## Summary and what remains

**What this plan (Task 1) proves:** the whole release-evidence path works end to end on one real,
non-rehearsed slice — a real pre-flight measurement block, a real Local sweep row 1 (captured
count, not exit-code-inferred), a real pre-bootstrap registry read for `paladin-eval`, and real
read-only `gh` reads for the PR and run state of `feature/phase-33`. Nothing here was simulated;
Task 2 and Task 3 build on these exact same rows rather than re-measuring from scratch.

**What remains:** Task 2 (this plan) hands the `paladin-eval` first-publish bootstrap to the
maintainer at a blocking-human checkpoint. Task 3 (this plan) records the post-checkpoint registry
state, the D-13 non-dispatch decision, and the first carried findings. The full D-06 seven-gate
re-seal sweep, the D-01 push/PR, the D-02 merge+tag checkpoint, the D-08 full registry table, the
real release run, and the MILESTONES.md entry are all later plans' work, not this plan's.

---

*Phase: 37-v0-10-0-crate-release*
*Written: 2026-09-18*
