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

**Addendum — 2026-09-18, post-Task-1 environment change (append-only, D-00d; nothing above this
addendum is edited):** between Task 1 and this continuation, the maintainer reported acting on
the 18G figure above. Obtained via the runtime's interactive question mechanism
(`AskUserQuestion`), the maintainer's reply, verbatim:

> "I ran `cargo clean` and now there is plenty of space."

Re-measured live by this continuation, after that report:

| Measurement | Command | Result |
|---|---|---|
| Free space on `/workspace` | `df -BG --output=avail,pcent /workspace` | **135G** available, 84% used |
| Working tree cleanliness | `git status --porcelain` | Empty — clean tree |

This satisfies plan 37-02's precondition text ("the figure recorded by plan 37-01 Task 1"), now
current at 135G, well above both plan 37-02's (>= 20 GiB) and plan 37-03's (>= 40 GiB) thresholds.
`target/` is confirmed cold (near-empty directory, no build artifacts) as a direct consequence of
the `cargo clean` reported above — this is stated plainly so that plan 37-02/37-03 gate timings
are read as cold-build timings, not as a regression against Task 1's or any prior phase's warm-cache
figures. A separate, out-of-band `cargo clippy --workspace --all-targets --all-features -- -D
warnings` cache warm-up was run by the orchestrator ahead of this continuation (finished clean, no
warnings, 6m 23s) — that run was cache preparation for the commit hooks below, not a gate, and is
not recorded as a Local sweep row.

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

### Plan 37-02 head SHA (Task 1, dispatch)

`git rev-parse HEAD` → `028e9726c2388da43d237af06926506bdd8760bf` — the tip of `feature/phase-33`
at dispatch of plan 37-02 (tip of `docs(37-01): complete release-evidence tracer plan`), measured
live, `git status --porcelain` empty, `df -BG --output=avail /workspace` → `135G`. Every row below
through Task 3 ran against this exact SHA's source tree (`crates/`, `src/`, `tests/`,
`Cargo.toml`/`Cargo.lock`) — the three task commits between rows only ever touch this evidence
file itself, never a source file, so the code under test does not change between tasks even though
`HEAD` advances with each task's own commit. Each row below is cross-referenced against this same
head SHA rather than re-stating it per row.

| 2 | `make check-migration-allowlist` (D-06 gate row 1, second half — head `028e9726`) | 15 `crate\|type` pairs in both the MIGRATION.md §9.2 register and `.cargo/semver-checks-allowlist.toml`, set-equal in both directions — identical pair count to `33-CI-EVIDENCE.md` row 4 and §11's own 15 | ✅ PASS |
| 3 | `make check-gates` (D-06 gate row 1 bundle — head `028e9726`) | Per-crate CHANGELOG coverage 11/11; package-name allow-list 12/12; advisory-exception register 11 rows vs 11 `deny.toml` + 5 `.cargo/audit.toml` ignore entries, all satisfied; workflow inline-suppression scan: 7 files, 165 steps, 1 `cargo audit` invocation, 0 inline suppressions; workflow trigger-policy table 7/7; CodeQL dismissal register 6/6; plus row 2's set-equality check — all seven sub-targets exit 0 | ✅ PASS |
| 4 | `U=$(grep -c '^## \[Unreleased\]' CHANGELOG.md; true); echo "$U"` (D-06 gate row 7, hard assertion — head `028e9726`) | `0` | ✅ PASS |
| 5 | `H=$(grep -c '^## \[0.10.0\]' CHANGELOG.md; true); echo "$H"` (D-06 gate row 7, hard assertion — head `028e9726`) | `1` | ✅ PASS |
| 6 | `N=$(cat CHANGELOG.md MIGRATION.md \| grep -c 'v0[.]11[.]0'; true); echo "$N"` (D-06 gate row 7, hard assertion, the withheld next-version string — head `028e9726`) | `0` | ✅ PASS |
| 7 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'rag'` (D-06 gate row 7, recorded reading, §11 topic 1 of 3 — head `028e9726`) | `22` (up from `33-CI-EVIDENCE.md` row 28's `17`; both non-zero, no regression) | ✅ PASS |
| 8 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -c 'TokenUsage'` (D-06 gate row 7, recorded reading, §11 topic 2 of 3 — head `028e9726`) | `4` (same as `33-CI-EVIDENCE.md` row 29) | ✅ PASS |
| 9 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -c 'Commissary'` (D-06 gate row 7, recorded reading, §11 topic 3 of 3 — head `028e9726`) | `15` (same as `33-CI-EVIDENCE.md` row 30) | ✅ PASS |
| 10 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'mdBook'` (D-06 gate row 7, recorded reading, documentation-phase topic 1 of 4, Phases 34-36.1 — head `028e9726`) | `1` | ✅ PASS |
| 11 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'rustdoc'` (D-06 gate row 7, recorded reading, documentation-phase topic 2 of 4 — head `028e9726`) | `0` — carried as a finding below, not a gate failure (see this task's action text and `## Findings carried forward (D-00d)`) | ⚠️ RECORDED, not a gate |
| 12 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'examples'` (D-06 gate row 7, recorded reading, documentation-phase topic 3 of 4 — head `028e9726`) | `7` | ✅ PASS |
| 13 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'intra-doc'` (D-06 gate row 7, recorded reading, documentation-phase topic 4 of 4 — head `028e9726`) | `0` — carried as a finding below, not a gate failure (see this task's action text and `## Findings carried forward (D-00d)`) | ⚠️ RECORDED, not a gate |

Rows 4-13 are the split gate-row-7 assertion the plan's action text prescribes: rows 4-6 plus
row 3's `check-changelogs` sub-target are the four hard assertions (all green, D-14 would stop the
plan on any one of them going red); rows 7-13 are seven recorded topic-count readings, none of
which is itself a red-gate condition per the plan's own text — a zero on a documentation-phase
topic (rows 11 and 13) is a carried finding, not a fix, and `CHANGELOG.md`/`MIGRATION.md` were not
edited either way. `git status --porcelain CHANGELOG.md MIGRATION.md` confirmed empty after this
task.

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

**Post-checkpoint registry state for `paladin-eval` (this plan, Task 3):**

```
curl -s -o /tmp/pe_post.json -w '%{http_code}' \
  -H 'User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)' \
  https://index.crates.io/pa/la/paladin-eval
```

**HTTP status: `404`** (same `NoSuchKey` body shape as the pre-bootstrap baseline above —
re-verified 2026-09-18, after Task 2's checkpoint resolved). No `vers` field present.

**Maintainer's Task 2 reply, recorded verbatim** (obtained via the runtime's interactive question
mechanism, `AskUserQuestion`, presented against the three listed options "Bootstrapped 0.0.1" /
"Deferred" / "Not needed"; the maintainer answered in free text instead of selecting one):

> "You'll provide specific instructions (short runbook) for the Owner Gated  requirement when the
> requirement is needed and we will together make sure it is done properly."

**Orchestrator's classification of that reply (the orchestrator's reading, not the maintainer's
words): `deferred`.** Reasoning: the reply reports no publish and no placeholder version, so it is
not "bootstrapped `<version>`"; it makes no claim that crates.io now supports a pending publisher,
so it is not "not needed"; it postpones the act to the point of need and asks the agent to supply
a short runbook then, to be worked through together. This is also the fail-safe branch of the
three: it leaves plan 37-08's gate fully in force. The orchestrator stated this reading to the
maintainer in-session.

**Open obligation carried from this reply:** the agent owes the maintainer a short, specific D-17
runbook at the point of need. First natural opportunity: the PR CI wait in plans 37-06/37-07. Hard
deadline: before plan 37-08 Task 3's tag hand-off — per D-17's own instructions text, a `404` under
a "deferred" reply withholds the tag command and halts the phase there until the bootstrap is
actually done.

**Reading this 404 correctly:** the continuing 404 is the expected state under "deferred" — it is
**not** a failure of this plan (per Task 1's own note, and per this task's action text). The gate
that actually consumes this fact sits in plan 37-08, immediately before the tag hand-off, and
branches on the three-way reply captured verbatim above.

---

## Findings carried forward (D-00d)

Not populated by this plan. Task 3 appends the D-13 non-dispatch record and the first set of
carried documentation findings here; later plans append further findings as they are observed.
Nothing above this heading is edited by any later addition — additions are dated and appended
only.

### D-13 — dry-run dispatch not attempted (this plan, Task 3, 2026-09-18)

No `workflow_dispatch` of `release.yml` was attempted, and no rc tag or non-`v` shadow tag was
created.

**Traced reason:** the dispatch's `tag` input is used both as the ref to resolve (`verify-tag-source`,
`git rev-list -n 1 "$RELEASE_TAG"`) and as the literal version string matched against the
CHANGELOG heading (`create-release`) and against every publishable crate's manifest
(`check-release-consistency`). No single value satisfies all three constraints before a ref
literally named `v0.10.0` exists:
- `v0.10.0` / `0.10.0` (no such ref pre-tag) fails step 1 outright — `verify-tag-source` cannot
  resolve it as a revision.
- The exact 40-char merge-commit SHA resolves in step 1 but fails the changelog-heading match
  (`create-release`) — no `## [<40-hex-chars>]` heading exists.
- An existing older tag (e.g. `v0.9.0`) resolves and matches the changelog heading, but fails the
  manifest match (`check-release-consistency` clause 1) — the manifest is `0.10.0`, the tag strips
  to `0.9.0`.

`37-RESEARCH.md` Q1 carries the full row-by-row trace over these four candidates. D-13's own
fallback sentence — "if a dry run cannot be dispatched without a real tag, fall back to going
straight to the tag and record why; do not substitute an rc tag" — is the authority applied here.

**The theoretical non-`v`-prefixed shadow-tag escape hatch** (a lightweight tag literally named
`0.10.0`, no leading `v`, which would resolve step 1 and match steps 3-6 without matching the
`push: tags: v*.*.*` trigger glob) **was considered and rejected.** `37-RESEARCH.md` Q1 records it
as traced but untested by design, and explicitly not recommended: it adds an extra pushed tag
object outside the documented flow, for a low-value rehearsal, given the seven local gates already
prove packaging validity (Q5). No such tag was created.

### Carried documentation findings (this plan, Task 3, 2026-09-18)

Recorded only; nothing below is fixed by this plan (D-14 — this phase does not edit docs pages
under CONTEXT `<deferred>`; currency fixes are v0.11.0 scope):

- `docs/src/appendix/release-automation.md`'s per-crate Trusted Publishing table and Credential
  History ledger have no `paladin-eval` row — D-17's bootstrap (deferred, per the reply recorded
  above) is not yet reflected there, and won't be until the bootstrap actually happens.
- `docs/src/appendix/release-automation.md`'s "Canonical Publish Order" section still describes
  the pre-`paladin-eval` eleven-crate order; `scripts/publish-crates.sh`'s `CRATES` array is the
  live authority (twelve crates, `paladin-eval` at position 11 of 12).
- `docs/src/contributing/development-setup.md` still states eleven publishable crates; the tree
  (`cargo metadata`, live) says twelve — per D-00f, the shipped tree outranks any document.
- `docs/src/appendix/release-recovery.md` §1's example loop enumerates eleven crate names, not
  twelve.
- `CHANGELOG.md`'s `[0.10.0]` heading carries the date `2026-09-10`, earlier than the actual
  release date. Recorded, not edited: neither the D-06 gate set nor
  `scripts/check-release-consistency.sh` reads the date (clause 2 matches only the version
  heading), so this is a currency finding, not a gate failure.

### Plan 37-02, Task 1 — zero documentation-phase topic counts (2026-09-18)

Local sweep rows 11 and 13 recorded `0` for the `rustdoc` and `intra-doc` topic-count readings
inside the `[0.10.0]` CHANGELOG section, against the plan's own four documentation-phase topics
(`mdBook`, `rustdoc`, `examples`, `intra-doc`). **Per the plan's action text this is explicitly not
a red gate** — D-06 gate row 7's binding machine checks are `make check-changelogs` and
`scripts/check-release-consistency.sh` clause 2 (both hard-asserted green in rows 3-4 above), not
these topic-count readings. Recorded for the maintainer's awareness only; `CHANGELOG.md` was not
edited. A spot grep of the `[0.10.0]` section's own `### Documentation` subsection (see rows 458,
476-499 for line references) shows substantial rustdoc/intra-doc-adjacent prose (e.g. "The
generated API documentation now builds warning-free," "a new code-quality gate keeps every future
[doctest] from shipping without one") that does not literally contain the strings `rustdoc` or
`intra-doc` — a wording gap, not a missing-content gap; left as-is per D-14 (this phase does not
edit CHANGELOG.md prose to make a topic grep pass).

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
