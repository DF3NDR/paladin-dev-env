# Phase 58: Crate Release - Context

**Gathered:** 2026-11-05
**Status:** Ready for planning

<domain>
## Phase Boundary

`v0.11.1` is **released, not merely releasable**. The `0.11.0 → 0.11.1` lockstep bump lands as
the last commits on the milestone branch with `check-version-strings` green on the release
commit (the Phase 54 gate lists 52 user-facing strings that move with the manifests); all twelve
publishable changelogs carry a dated `[0.11.1]` section; `MIGRATION.md` §11.8 states the
library-consumer upgrade story on measured evidence; the milestone audit over Phases 52-57 reads
`passed` because every carried item is fixed, waived with a reason, or re-homed with an owner and
a date; the re-seal gate set is green on the candidate final head and signed; the maintainer
merges the one release PR to `main` with a merge commit, pushes the annotated `v0.11.1` tag to it,
and `release.yml` runs end to end with no manual step; every publishable crate (the list is
whatever `cargo metadata` says, twelve today) resolves on the crates.io sparse index at `0.11.1`,
`yanked = false`, through Trusted Publishing; the GitHub Release exists with its assets; a keyless
scratch-crate build on the published `0.11.1` is recorded; the release record, the SHIP-08/09/10
ticks, the short web3sec hand-off and a "ready to close" hand-off are written.

Delivered here: a build wave of pre-bump commits (the three folded todos, the WR-02 checker fix,
the `sk-proj-…` literal scrub, a CI-side SDK-smoke fix if it is one), the three-commit bump set,
the crate-changelog anchors and entries, the MIGRATION §11.8 consumer subsection,
`58-AUDIT-DISPOSITIONS.md` and the dated re-home todos, the re-seal and `58-CI-EVIDENCE.md`,
`58-ACCEPTANCE-AUDIT.md` with the maintainer's tick, the retitled release PR, the merge + tag
hand-off, the post-tag registry table and smoke, `58-HANDOFF-web3sec.md`, the MILESTONES.md
entry on `chore/58-close`.

**Not** delivered here: any change under `src/` or `crates/*/src/` (57-REVIEW CR-01 and WR-01
are re-homed, not fixed); any new capability; any fix to a red gate once the re-seal has started
(a red gate stops the phase); any `ci.yml` change beyond a small CI-side fix to the SDK-smoke job
if diagnosis shows one; a return contract from web3sec; the `/gsd-complete-milestone v0.11.1`
run itself; the maintainer's key rotation or history purge; anything belonging to `v0.12.0`.

Observed state at discussion time (2026-11-05, HEAD `3b0d095a2` on `claude/laughing-dirac-e0h2ax`,
359 commits ahead of `origin/main` at `8b23cb4c5`, the PR #62 merge): every manifest reads
`0.11.0`; `cargo metadata` lists twelve publishable crates; the root `CHANGELOG.md` and eight
crate changelogs carry `## [Unreleased]`, while `paladin-battalion`, `paladin-eval` and
`paladin-herald` carry neither an anchor nor a `[0.11.1]` section; `MIGRATION.md` §11 is open
(intro records the Phase 53 patch decision, §11.2 is header-only, §11.8 has an operator
checklist and no library-consumer subsection); `cargo-release 1.1.7` and `cargo-semver-checks`
are installed in the sandbox, `cargo-llvm-cov` is not, and there is no Docker; the
`Generated SDK Clients (Python + TypeScript) smoke` CI job is red on the branch and undiagnosed;
the complete `sk-proj-…` OpenAI key literal is still tracked in
`.project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md`; every Phase 52-57
`VERIFICATION.md` reads `passed`; `WINDOWS.md` reads `open_count: 0`.

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by Phases 37 / 37.1 / 47 / 52 / 53 / 54 — not re-asked)

- **D-00a:** Only the maintainer ticks a sign-off box — never an agent, under any mode (Phase 47
  D-00a).
- **D-00b:** The maintainer alone performs the irreversible acts: merging the release PR, pushing
  the `v*` tag, applying the `release` label, any `cargo yank`, any key rotation or history purge.
  The agent hands over exact commands and stops (Phase 47 D-00b).
- **D-00c:** Tag-cut mechanism: the release PR merged to `main` with a **merge commit** (not
  squash, not rebase), then an **annotated tag pushed directly to that merge commit** after
  `ci.yml` is green on it. `make release` is not the path (its trailing push is blocked by the
  "Protect main branch" ruleset); its component steps are (Phase 47 D-00c, Operator Guide).
- **D-00d:** Findings are recorded, never fixed silently; amend-at-source with dated additions;
  `WINDOWS.md` rows are never deleted (Phase 47 D-00d).
- **D-00e:** The 82 % workspace line-coverage floor (ADR-0006) is measured by CI's `Coverage` job
  only; `cargo-llvm-cov` is absent here and the sandbox has no Docker (Phase 47 D-00e).
- **D-00f:** Shipped tree outranks any document. The publishable-crate list is whatever
  `cargo metadata --no-deps` says (`publish != false`), never hard-coded (Phase 47 D-00f).
- **D-00g:** Publishing is via crates.io Trusted Publishing; no standing registry credential
  exists and nothing in this phase may introduce or request one (Phase 47 D-00g).
- **D-00h:** Registry proof is a sparse-index (`index.crates.io`, mandatory `User-Agent`) read
  per publishable crate — crate, version, checksum, `yanked` — recorded as a table. The
  workflow's own log is not proof on its own (Phase 47 D-00h).
- **D-00i:** Waiting on the maintainer is a hard stop with a resume file naming the exact resume
  condition — never a parked watcher or a long-lived `gh run watch` (Phase 47 D-00i).
- **D-00j:** Any red gate from the start of the re-seal onward — locally or in PR CI — is recorded
  (command, SHA, failing output) and the agent halts at a blocking checkpoint and fixes
  **nothing**. The one exception is a pure infrastructure flake, re-run **exactly once** with both
  run IDs and the justifying log line recorded (Phase 47 D-00j).
- **D-00k:** `release.yml` cannot be dry-run-dispatched before a ref literally named `v0.11.1`
  exists. No dispatch rehearsal, no rc tag, no shadow tag (Phase 47 D-00k).
- **D-00l:** Post-tag commits reach the protected `main` through a `chore/58-close` branch cut
  from the tagged `main` and a small docs-only PR the maintainer merges; the tagged commit stays
  exactly what was released; `/gsd-complete-milestone` is not run by any plan (Phase 47 D-00l).
- **D-00m:** The phase stays open across the merge: a **build wave** (pre-bump fixes, bump set,
  anchors, MIGRATION subsection, audit register), a **pre-merge wave** (re-seal, PR, evidence,
  sign-off), the maintainer's **merge + tag checkpoint**, and a **post-tag wave** (registry
  verification, smoke, records, hand-offs). Verified only on what is observably true, never by
  promise (Phase 47 D-00m).
- **D-00n:** `CHANGELOG.md` heading dates equal the tag date (Phase 46 D-13). `MIGRATION.md` §9
  and §10 are frozen records; only §11 is written (Phase 52 D-00d).
- **D-00o:** `v0.11.1` is a patch and every Phase 52-57 change is additive, decided in Phase 53
  on measured evidence (`53-VERSION-DECISION.md`). That file itself says the pre-tag audit must
  re-run `scripts/check-semver.sh` on its own head rather than rely on the record (see D-10).
- **D-00p:** The version gate goes red at the bump by design: `scripts/check-version-strings.sh`
  holds every copyable string to the workspace manifest version, so the bump commit must carry
  the string edits for the gate to be green on the release commit (Phase 54 carry-forward,
  54-FIRST-EVIDENCE "Carry-forward to Phase 58"; 54 CR-01: `make release` runs no version-string
  gate before the tag).
- **D-00q:** `scripts/finalize-crate-changelogs.sh` leaves a changelog with no `[Unreleased]`
  anchor and no target-version section untouched and records a named failure;
  `check-release-consistency.sh` independently requires a `## [X.Y.Z]` heading in all twelve
  publishable changelogs (Phase 54 carry-forward rule 1).
- **D-00r:** The Phase 52 release gates are on: `Publish Dry Run` and
  `Semver Checks (vs v0.11.0)` run on every PR; `End-to-End Tests` runs under the maintainer's
  `release` label; `scripts/release-body-fit.sh` and the create/finalize guards see their first
  live run at this tag — read that release's body size and footer at the tag (Phase 52 D-05,
  D-11, D-12; STATE "Release-body size guard has never run live").
- **D-00s:** Partial-publish recovery is the maintainer's option A: a patch release through the
  same pipeline, orphaned versions yanked only after the patch is registry-verified, one
  yank-register row per crate; the agent diagnoses read-only per
  `docs/src/appendix/release-recovery.md` §1-§2 and hard-stops (Phase 47 D-18, Phase 37.1 D-02).

### Branch & PR shape

- **D-01:** **One PR.** `claude/laughing-dirac-e0h2ax` (Phases 54-57, 359 commits ahead of
  `main`) is the release PR; the build wave and the bump commit set land as its last commits; it
  is retitled `release: v0.11.1 — Dogfood Cleanup` (wording at the planner's discretion) with a
  curated body; its single merge commit carries the whole remainder of the milestone and is the
  tag target. The Phase 47 precedent (PR #59). Rejected: merging Phases 54-57 first and cutting a
  small bump PR off `main` (two CI cycles, two merges, and `main` briefly holding unreleased
  `0.11.1` code under a `0.11.0` version); rejected: an audit run between two such merges.
  — **Reversibility:** one-way — the merge commit on the protected `main` becomes the tag target
  and cannot be rewritten once the tag points at it.
- **D-02:** **`main` is frozen** at `8b23cb4c5` until the release PR merges; no other merge
  before the tag, so the PR's diff is exactly "everything since the PR #62 merge". If `main`
  moves anyway, the branch merges `main` in (merge commit, never rebase), the re-seal restarts
  from the top on the new head, and the maintainer re-confirms the sign-off against the new SHA
  (Phase 47 D-02).
- **D-03:** **The `release` label is applied once, after the bump push.** The agent opens or
  retitles the PR only after the bump commit set is on `origin`, then hands the maintainer the
  label step as one checkpoint so the full `ci.yml` re-run (about 63 minutes of Docker build)
  happens on the candidate final head and `End-to-End Tests` reports before the irreversible
  merge. Rejected: labelling early (a second full re-run after the bump); rejected: skipping the
  label for a patch (breaks Phase 52 D-05).
- **D-04:** **Sign-off home and order as Phase 47 D-10..D-14.** The re-seal table and the single
  tick box live in a new `58-ACCEPTANCE-AUDIT.md` beside `58-CI-EVIDENCE.md`; the box reads
  `- [ ] **The \`v0.11.1\` tag may be cut**`; the signed table is the eight 37.1 rows re-read at
  `0.11.1` (MIGRATION §11.2 ↔ allowlist set-equality via `make check-migration-allowlist`;
  `v0_9_config_boot`; OpenAPI golden; `make check-semver` vs `0.11.0` at patch, twelve crates,
  now a real check (Phase 52); MSRV; `make publish-dry-run`; `[0.11.1]` completeness via
  `make check-gates` plus the hard-assertion greps; `make check-publish-order`), each with
  command, SHA, result and CI twin named or "none" stated. `make api-surface`,
  `make check-consumer-surface`, `make check-version-strings` (inside `check-gates`) and the CI
  `Coverage` job are CI-evidence entries. Order is structurally enforced: local re-seal → push →
  PR CI green including `Coverage` → evidence appended → the maintainer ticks the box and the
  tick commit is on `origin` at the PR head → CI green on that SHA → the agent verifies the tick on
  `origin` → only then the merge + tag hand-off is written. No tick on `origin`, no hand-off.
  — **Reversibility:** one-way — the merge and tag that follow trigger a crates.io publish.
- **D-05:** **The red `Generated SDK Clients (Python + TypeScript) smoke` job is diagnosed
  first, in the build wave.** A plan reads the job log (run `41927940508`, job `126917997570`,
  and the latest run on the branch) and whether the context is required by the ruleset. A CI or
  script defect gets a minimal fix in the release PR; anything that would touch `src/` or
  `crates/*/src/` is recorded as a finding in `58-CI-EVIDENCE.md` and `58-AUDIT-DISPOSITIONS.md`
  and the maintainer decides (fix, accept a red non-required job at the merge, or hold the tag)
  at a blocking checkpoint. Rejected: fixing it whatever it takes (widens the release PR into
  crate code); rejected: recording only (a required red context would block the merge
  unexplained).

### Pre-tag audit dispositions (SHIP-10)

- **D-06:** **One disposition register, three verdicts.** A new `58-AUDIT-DISPOSITIONS.md` in
  the phase directory lists every carried item with exactly one of `fixed` (pointer to the
  commit or evidence), `waived` (reason), or `re-homed` (owner, date, pointer to the todo). The
  milestone audit's `tech_debt` list may cite only rows of that register, so a `passed` status is
  backed by a visible disposition per row. The carried set, to be completed by the plan from the
  sources named in canonical refs: 57-REVIEW CR-01, WR-01, WR-02 and IN-01…05; the two orphaned
  Phase 55 deferrals (News API key in the URL `D-52-03-01`; `${…}` placeholder in the Treasurer
  webhook `secret`, 54 WR-04); the red SDK-smoke job; the tracked `sk-proj-…` literal; OSV
  alerts 23-25 (`D-52-03-02`); the stale CodeQL anchors (`D-52-03-03`); FIXES-04
  `RunStatusValue::Cancelled` (55 deferred-items); 55-08 `EmbeddingPort` rustdoc; the 56
  deferred-items entry; 54 WR-01/WR-04/WR-05/IN-02; the Phase 47 carried release-tooling
  findings not closed by Phase 52; the seven pending todos (three folded, four acknowledged);
  the `Generated SDK Clients` and Codecov maintainer-only items from STATE; and anything else
  the plan's sweep of `.planning/phases/5[2-7]-*/` (deferred-items, REVIEW, VERIFICATION
  `deferred`, SUMMARY `deviations`) and `.planning/todos/pending/` surfaces. The manual
  `scripts/check-review-dispositions.sh` run (not in `check-gates` by design) and
  `make check-findings-log` are part of the audit sweep and recorded with their result lines.
- **D-07:** **Only WR-02 is fixed in `v0.11.1`; CR-01 and WR-01 are re-homed.** The
  `check-review-dispositions.sh` acceptance of non-evidence as a `closed` pointer is gate
  tooling, not crate code, and lands in the build wave with its fixture. CR-01 (worker
  nack-and-redeliver loses the run outcome when only the second write fails) and WR-01 (an empty
  `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` wipes a YAML secret) are behaviour changes in published
  crates and go to `v0.12.0` with the maintainer as owner. Rejected: all three as pre-bump commits
  (three more crate diffs and MIGRATION entries in the release PR); rejected: re-homing WR-02 too.
- **D-08:** **The `sk-proj-…` literal is scrubbed in the release PR; everything else about it is
  the maintainer's.** A docs-only build-wave commit replaces the literal in
  `.project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md` with a redacted
  placeholder, and the 56-08 key-shape scanner reads clean on the tracked tree. Whether the key
  was ever live, its rotation, and any history rewrite are maintainer checkpoints recorded as
  rows in `58-AUDIT-DISPOSITIONS.md` (owner maintainer, date before the tag for rotation; the
  purge decision may be re-homed). The literal itself is never quoted in any planning document
  or commit message. Rejected: leaving the file and recording only (ships the literal in the
  `v0.11.1` source archive); rejected: deleting the whole file.
- **D-09:** **Re-home form: one pending todo per item, owner named, date fixed.** Each re-homed
  item becomes (or already is) a `.planning/todos/pending/` file with an `owner` (the maintainer,
  or `v0.12.0 Phase 1` for code items), `recheck_by` set to the next milestone's first planning
  day, and a pointer back to its source finding; the register row cites the todo path.
  `WINDOWS.md` stays at `open_count: 0` and gains no row. The four acknowledged todos (OpenAI
  Responses API, setup-check output, treasurer price digits, DeepSeek reasoning hint) keep their
  files and get the dated owner line if they lack one. Rejected: new `WINDOWS.md` rows closed as
  waived; rejected: the register as the only home.

### Consumer statement & web3sec (SHIP-09)

- **D-10:** **A `### Library consumers: v0.11.0 → v0.11.1` subsection under MIGRATION §11.8**,
  mirroring §10.8's. One paragraph on measured evidence: no Rust signature, type or serialized
  schema changed; §11.2 has no row and `.cargo/semver-checks-allowlist.toml` has no entry;
  `scripts/check-semver.sh` on the release head reads `0 failure(s)` for all twelve packages at
  `--release-type patch` (head SHA and date named); the only consumer-visible note is
  `get_job` unrouted-but-not-deprecated, already in §11.7. It is written in the bump commit set
  (D-15, commit 3) and re-read on the candidate final head; if the final head differs from the
  SHA it names, the SHA is corrected in a docs-only commit before the tick. Rejected: one
  sentence in the §11 intro only; rejected: both places.
- **D-11:** **A red `check-semver` on the release head is a hard stop before any bump.** The
  pre-tag audit re-runs `scripts/check-semver.sh` on its own head (D-00o). A red line stops the
  phase; the agent reports the lint and the offending item and proposes an additive reshape that
  removes the break without changing behaviour (Phase 53 PD-05-1 pattern) first. If none exists,
  the maintainer chooses at a blocking checkpoint: register the one break in §11.2 plus the
  allowlist and still ship `0.11.1` (SHIP-09's "records the one deliberate break" branch) or
  move every version string to `0.12.0` per the roadmap note. Rejected: registering any break and
  shipping `0.11.1` regardless; rejected: an automatic flip to `v0.12.0`.
  — **Reversibility:** one-way — a published version number is permanent.
- **D-12:** **A short outbound `58-HANDOFF-web3sec.md`, no return contract.** Written in the
  post-tag wave: tag, merge SHA, release run id, the twelve-crate registry table, pointers to
  MIGRATION §11.8's consumer subsection and to the operator steps that touch a consumer's
  deployment (`M-B-36` key rotation, `M-B-38` environment variables), and the Cadence muster pin
  (folded todo). No return fields are requested and no todo stays pending on a reply. Rejected:
  the full Phase 47 hand-off with a return contract; rejected: no hand-off.
- **D-13:** **A keyless scratch-crate smoke on the published `0.11.1` is part of the post-tag
  proof.** After the registry table, the agent runs `cargo add paladin-ai@0.11.1` in a scratch
  crate outside the workspace (no `[patch]`, no path dependencies, no workspace inheritance),
  builds the README quick example, and records resolve and compile results in
  `58-CI-EVIDENCE.md`. No provider key is used; a failure is recorded and put to the maintainer,
  never fixed in this phase. Rejected: registry table only; rejected: the full keyed first-run
  walk.

### Bump commit set & changelogs (SHIP-08)

- **D-14:** **Three bump commits, the last three on the branch.** Commit 1 (`chore(release)`):
  `cargo release version 0.11.1 --execute --no-confirm --workspace` on a clean tree, then the 52
  user-facing strings the gate lists (42 class-1 Cargo pins in docs, the README `Current version`
  line, 8 `k8s/` labels and image tags, `openapi.json` `info.version` via `make openapi`), so
  `make check-gates` including `check-version-strings` is green on that commit, plus a grep-zero
  check for `0.11.0` across manifests, pins and the gate's file classes (historical
  CHANGELOG/MIGRATION/ADR text excluded; `crates/doc-examples` hand-touched if the tool skips
  it). Commit 2 (`chore(release)`): `make finalize-crate-changelogs VERSION=0.11.1`, root
  included, followed by `make check-changelogs` and `make check-release-consistency
  RELEASE_TAG=v0.11.1`. Commit 3 (`docs(release)`): the MIGRATION §11.8 consumer subsection
  (D-10). The expected `openapi.json` diff is the version string only; any other change means a
  route moved and is recorded as a finding and the agent stops. Rejected: one consolidated
  commit; rejected: four commits (manifests alone read red on the version gate).
- **D-15:** **Crate changelog entries are derived from the git log, never a stock line.** For
  `paladin-battalion`, `paladin-eval` and `paladin-herald` (no anchor today) a build-wave plan
  reads `git log v0.11.0..HEAD -- crates/<crate>/` (unshallowed or via GitHub where the shallow
  clone cannot reach `v0.11.0`), adds the `## [Unreleased]` anchor and the real entries (Phase 55's
  rustdoc-allow removals and lint fixes count), and uses a "no functional change" line only when
  that log is genuinely empty. The other eight crate changelogs are sanity-checked the same way
  so no `[Unreleased]` section contradicts the root (47-REVIEW WR-01). Rejected: bare anchors
  stamped empty; rejected: copying root lines.
- **D-16:** **Stamp date = the day the bump is pushed; redated before the tick if the tag
  slips.** At the sign-off checkpoint the maintainer names the tag day; if it differs, one
  `docs(release)` commit corrects the twelve headings before the box is ticked and re-seal row 7
  is re-read on that head (D-00n). Rejected: a blocking date question before stamping; rejected:
  stamping after the merge (the tagged tree would lack the `[0.11.1]` heading `release.yml` and
  `check-release-consistency` require).
- **D-17:** **All non-bump work lands before the bump, in the build wave.** The Cadence muster
  pin (`test`), the `gpt-4` literal sweep (`docs`), the devcontainer `cargo-llvm-cov` install
  (`chore`), the WR-02 checker fix (`fix`), the `sk-proj-…` scrub (`docs`), the three changelog
  anchors (`docs`), the audit register and re-home todos (`docs`), and a CI-side SDK-smoke fix if
  diagnosis finds one (`ci`) each land as their own conventional commit, so the bump set is the
  last three commits and the re-seal runs once on the candidate final head. A red gate after the
  re-seal starts fixes nothing (D-00j); this ordering keeps every fix inside the window where
  fixing is allowed. Rejected: bump first; rejected: a separate pre-release PR (breaks D-02).

### Claude's Discretion

The maintainer did not select these; the defaults below were stated and accepted. Research may
overturn a default only with evidence, and must say so in RESEARCH.md.

- **Order of operations inside commit 1** — default: run `cargo release version` on a clean tree
  first, then edit the strings, then `make openapi`, then stage and commit. This sidesteps the
  untested dirty-tree behaviour of `cargo-release` (54-FIRST-EVIDENCE rule 3); if the tool
  refuses anyway, record what it printed.
- **Root `CHANGELOG.md` `[Unreleased]` curation** — default: the section (about 60 lines today)
  is stamped as written by Phases 52-57; a hand-curation only if the stamp leaves it unreadable,
  with the release-body size read at the tag (D-00r). No condensation step exists in this
  release.
- **PR title and body wording**, within the required contents (re-seal summary, pointers to
  `CHANGELOG.md` `[0.11.1]`, MIGRATION §11, `58-CI-EVIDENCE.md`, `58-ACCEPTANCE-AUDIT.md`,
  `58-AUDIT-DISPOSITIONS.md`, the twelve-crate list from `cargo metadata`), written via `gh api`
  REST with the "hand the exact command over" fallback for a read-only token.
- **Re-seal command order and output capture**, provided every signed row has a command, a SHA
  and a result, and `58-CI-EVIDENCE.md` follows the `47-CI-EVIDENCE.md` house form (local sweep
  table, CI-run table, registry table, registry probe command with the `User-Agent` header).
- **Hand-off checkpoint texts** for the label, the merge + tag and any recovery — reuse Phase
  47's with `0.11.1`; the tag command is `git tag -a v0.11.1 <merge-sha> -m "v0.11.1"` then
  `git push origin v0.11.1`.
- **`58-ACCEPTANCE-AUDIT.md` preamble** — default: two or three sentences on what `v0.11.1` is,
  drawn from MIGRATION §11's opening paragraph, no new claims.
- **Plan count and wave layout**, provided D-00m's four-wave split around the D-04 tick gate and
  the maintainer's merge + tag checkpoint holds, no plan both executes the bump and starts the
  re-seal in the same task, and the SDK-smoke diagnosis and the audit sweep run before the bump.
- **The `v0.11.1` GitHub Release object** — created by `release.yml` from the `[0.11.1]` section
  through the Phase 52 body builder; its size and footer are read and recorded; a
  `gh release edit` is the maintainer's.
- **`chore/58-close` contents** — as Phase 47 D-17: post-merge and release-run CI evidence, the
  registry table and smoke, the MILESTONES.md `v0.11.1` entry (tag, merge SHA, release run id,
  crate count, "registry-verified", Trusted Publishing, release-body size), the SHIP-08/09/10
  ticks through `gsd-tools` handlers in the post-tag wave only after the registry table shows
  all twelve crates at `0.11.1` `yanked = false`, `58-HANDOFF-web3sec.md`, and the "ready to
  close" hand-off listing the `/gsd-audit-milestone` / `/gsd-complete-milestone v0.11.1`
  prerequisites for the maintainer.
- **Devcontainer `cargo-llvm-cov` install shape** (folded todo) — version `0.8.7` as the todo
  says, in the image definition only; no coverage run is claimed from this sandbox.
- **`gpt-4` sweep model literal** — the replacement model name follows the todo's own
  recommendation and the pricing table's current rows; the plan reads the todo first.

### Folded Todos

- **Pin the three Cadence import paths web3sec takes in `tests/consumer_surface_muster.rs`**
  (`.planning/todos/pending/2026-11-04-muster-pin-cadence-consumer-imports.md`, score 0.6).
  Problem: the consumer's known Cadence imports are not under the muster gate. Fit: a test-only
  build-wave commit (D-17) that holds the consumer surface the hand-off (D-12) points at; no
  public-surface change, so the API-surface baseline does not move.
- **Replace the deprecated `"gpt-4"` model literal across the docs and examples, and gate it**
  (`.planning/todos/pending/2026-11-03-docs-deprecated-gpt-4-model-literal.md`, score 0.9).
  Problem: the Quickstart's `.model("gpt-4")` fails against a real OpenAI key (DF-18). Fit: a
  docs/examples build-wave commit (D-17) so the first keyed run on `0.11.1` does not stumble
  where the `0.11.0` walk did; this reverses the earlier "ship the Quickstart line unchanged"
  decision by the owner's choice here. Whether a gate script lands with it is the planner's
  call within the todo's scope.
- **Install `cargo-llvm-cov` 0.8.7 in the devcontainer image**
  (`.planning/todos/pending/2026-11-03-devcontainer-install-cargo-llvm-cov.md`, score 0.6).
  Problem: no local coverage read is possible here. Fit: a devcontainer-only build-wave commit
  (D-17); coverage evidence for this release still comes from CI (D-00e).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and source decisions
- `.planning/ROADMAP.md` §"Phase 58: Crate Release" (line ~602) — goal, five success criteria,
  the suggested ordering, the `v0.12.0` branch; and the "v0.11.1 exit criteria" block (line
  ~265) re-read at the tag
- `.planning/REQUIREMENTS.md` §SHIP-08, SHIP-09, SHIP-10 (lines ~250-259) and the traceability
  rows (lines ~377-379) — the requirements ticked in the post-tag wave
- `.planning/phases/53-tenant-isolation-closure/53-VERSION-DECISION.md` — the patch decision,
  its semver evidence shape, and the instruction to re-run `check-semver` on the release head
  (D-00o, D-11)
- `.planning/phases/54-first-run-usability/54-FIRST-EVIDENCE.md` §"Carry-forward to Phase 58" —
  the seven (now three) anchor-less changelogs, finalize rule 3, the 52 strings the gate lists
  at the bump, the untested dirty-tree case, `check-semver` left to this phase (D-00p, D-00q,
  D-14, D-15)
- `.planning/phases/52-ci-release-pipeline-repair/52-CONTEXT.md` D-00f, D-05..D-12 and
  `52-VERIFICATION.md` — the armed gates this release exercises live for the first time
- `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-CONTEXT.md` — D-00a..D-18:
  every release-day rule this phase inherits (D-00a..D-00s above)
- `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md`,
  `47-CI-EVIDENCE.md`, `47-HANDOFF-web3sec.md`, `47-READY-TO-CLOSE.md` — the house forms
  `58-ACCEPTANCE-AUDIT.md`, `58-CI-EVIDENCE.md`, `58-HANDOFF-web3sec.md` and the close hand-off
  copy (D-04, D-12, Claude's discretion)
- `.planning/milestones/v0.10.0-phases/37.1-v0-10-1-patch-release/37.1-CONTEXT.md` — D-01
  (option-A recovery), D-10 (enforced sign-off order), D-08 (two-stage red-gate policy)

### Audit inputs (SHIP-10)
- `.planning/phases/57-register-hygiene-close-out/57-REVIEW.md` §CR-01 (line 75), §WR-01 (105),
  §WR-02 (128), IN-01…05 — the carried review findings (D-06, D-07)
- `.planning/phases/57-register-hygiene-close-out/57-REGISTER-EVIDENCE.md` §"Orphaned
  deferrals" — the two Phase 55 deferrals with no owner (D-06, D-09)
- `.planning/phases/57-register-hygiene-close-out/57-REVIEW-DISPOSITIONS.md` and
  `scripts/check-review-dispositions.sh` — the manual checker run in the audit sweep, and the
  WR-02 fix target (D-06, D-07)
- `.planning/phases/52-ci-release-pipeline-repair/deferred-items.md` (D-52-03-01..03),
  `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/deferred-items.md` (FIXES-04,
  `EmbeddingPort` rustdoc, CONTEXT deferrals),
  `.planning/phases/56-dogfood-findings/deferred-items.md` — carried items to disposition (D-06)
- `.planning/phases/56-dogfood-findings/56-SECURITY.md` — the `sk-proj-…` tree-scan finding and
  the 56-08 key-shape scanner (D-08)
- `.planning/phases/54-first-run-usability/54-REVIEW.md` §WR-01, WR-04, WR-05, IN-02 — advisory
  follow-ups to disposition (D-06)
- `.planning/STATE.md` §"Blockers/Concerns" (Phase 57, 56, 54, 53, 52 close notes) and
  §"Pending Todos" — the live carried-item list the audit sweep starts from (D-06)
- `.planning/todos/pending/` (seven files) — three folded, four acknowledged (D-09, Folded Todos)
- `.planning/milestones/v0.11.0-MILESTONE-AUDIT.md` — frontmatter shape and the `tech_debt`
  register the `v0.11.1` audit must not repeat
- `.claude/gsd-core/workflows/audit-milestone.md` §6 — the `passed` / `tech_debt` status rule
- `scripts/check-findings-log.sh` (`make check-findings-log`) — every `DF-nn` dispositioned
  (exit criterion)
- `WINDOWS.md` — must still read `open_count: 0` at the tag (D-09)

### Consumer statement and hand-off (SHIP-09)
- `MIGRATION.md` §11 intro (line 7), §11.2 (line 39, header-only), §11.7 (line 127, `get_job`
  note), §11.8 (line 133, operator checklist; the consumer subsection lands here) and §10.8
  "Library consumers: v0.10.1 → v0.11.0" (line ~523) as the shape to mirror (D-10)
- `.cargo/semver-checks-allowlist.toml` — zero entries; set-equal to §11.2 (D-10, D-11)
- `scripts/check-semver.sh` (`make check-semver`; CI job `Semver Checks (vs v0.11.0)`,
  `.github/workflows/ci.yml` line ~409) — the release-head run D-10 cites and D-11 gates on
- `tests/consumer_surface_muster.rs` + `make check-consumer-surface` — the muster the Cadence
  pin extends (Folded Todos, D-12)
- `.planning/phases/56-dogfood-findings/56-CONTEXT.md` D-13/D-14 — the scratch-crate walk rules
  D-13's smoke reuses (keyless half only)

### Release mechanics
- `.github/workflows/release.yml` — tag trigger (`v*.*.*`), `verify-tag-source` (29), `test`
  (92), `create-release` (112), `check-release-consistency` (405), `finalize-release-body` (531),
  `publish-crates` (602)
- `docs/src/appendix/release-automation.md` §"Operator Guide: Cutting a Release" — steps 1-8,
  the `release` label as a maintainer act, the 63-minute re-run note (D-03)
- `docs/src/appendix/release-checklist.md` §2 — the root `## [VERSION]` section is a hard
  prerequisite for the release run (D-16)
- `docs/src/appendix/release-recovery.md` §1-§2 (agent's read-only scope), §3-§5 (maintainer's)
  (D-00s)
- `docs/src/appendix/branch-protection.md` — main-only tag policy, the two rulesets (D-00c,
  D-00l)
- `Makefile` — `check-gates` (281) and its members incl. `check-version-strings` (225),
  `check-release-consistency` (239), `check-findings-log` (252), `check-semver` (277),
  `openapi` (446), `api-surface` (452), `security` (475), `clean-code` (508), `release-check`
  (653), `publish-dry-run` (663), `finalize-crate-changelogs` (669), `release` (677 — not the
  path, D-00c)
- `release.toml` — lockstep configuration for `cargo release version` (D-14)
- `scripts/check-version-strings.sh`, `scripts/finalize-crate-changelogs.sh`,
  `scripts/check-release-consistency.sh`, `scripts/check-publish-order.sh`,
  `scripts/publish-crates.sh`, `scripts/release-body-fit.sh` — the gates and the first-live
  body guard (D-00p, D-00q, D-00r, D-14)

### Version bump inputs
- `Cargo.toml` `[workspace.package] version` (line 58), `[workspace.dependencies]` pins, the
  root package's own `paladin-*` entries; `crates/*/Cargo.toml`; `crates/doc-examples/Cargo.toml`
  (`publish = false`) — the thirteen manifests (D-14)
- `CHANGELOG.md` (`[Unreleased]` at line 8) and the eleven `crates/*/CHANGELOG.md` — three
  without an anchor (D-15)
- `crates/paladin-web/openapi.json` `info.version` (D-14)
- `README.md`, `docs/src/**`, `k8s/**` — the 52 strings `check-version-strings` lists at the bump
  (D-00p, D-14)
- `.project/current-exports.txt` — the API-surface baseline; must not move (D-04)
- `.planning/decisions/0006-coverage-gate.md` — ADR-0006, the 82 % floor (D-00e)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `release.toml` + `cargo release version 0.11.1 --execute --no-confirm --workspace`: the
  lockstep bump already configured; `cargo-release 1.1.7` is installed in this sandbox (D-14).
- `scripts/check-version-strings.sh` (`make check-version-strings`, inside `make check-gates`):
  the list of every string that must move with the bump is the gate's own red output on the
  bumped tree (D-00p).
- `scripts/finalize-crate-changelogs.sh` (`make finalize-crate-changelogs VERSION=0.11.1`):
  stamps all twelve changelogs in one invocation once every file has an anchor (D-00q, D-15).
- `make check-gates`: the eleven offline guards including `check-migration-allowlist`,
  `check-publish-order` and `check-version-strings`; `make check-semver` builds and needs
  crates.io (D-04, D-11).
- `scripts/check-review-dispositions.sh` with its 22 `make test-shell-guards` fixtures: the
  WR-02 fix target and the manual audit-sweep run (D-06, D-07).
- `scripts/check-findings-log.sh` (`make check-findings-log`): the DF-nn disposition check the
  exit criteria require (D-06).
- `47-ACCEPTANCE-AUDIT.md`, `47-CI-EVIDENCE.md`, `47-HANDOFF-web3sec.md`, `47-READY-TO-CLOSE.md`:
  copy their structure (eight-row table and box; three evidence tables and the sparse-index
  probe command; hand-off and close forms) with `0.11.1` (D-04, D-12).
- Phase 56 D-13's scratch-crate walk commands: reusable for the keyless smoke (D-13).
- `tests/consumer_surface_muster.rs`: the file the Cadence pin extends (Folded Todos).

### Established Patterns
- Amend-at-source with dated additions governs every finding, the todo updates and the audit
  register (D-00d, D-06, D-09).
- `chore/NN-close` branch plus a small docs-only PR for post-tag bookkeeping (D-00l).
- CI-evidence files are sealed at the SHA they verify; all bookkeeping lands before the phase is
  verified.
- Blocking human checkpoints via the interactive question mechanism, reply recorded verbatim;
  owner-gated acts get a short, specific, just-in-time runbook (label, merge + tag, key rotation).
- ROADMAP / STATE / REQUIREMENTS mutations go through `gsd-tools` handlers, never direct edits.
- Prove-then-lock: every carried item is re-checked on the tree before it is dispositioned
  (Phases 53-57); the audit register follows the same rule.
- The clippy pre-commit hook and the GSD commit helper interact; the pre-push `api-surface`
  gate runs on every push and the baseline must not move.

### Integration Points
- `gh api` (REST) for the PR retitle and body, run listing, job logs (the SDK-smoke diagnosis)
  and the one allowed flake re-run; GraphQL is blocked in this sandbox; every write needs the
  "hand the exact command over" fallback for a read-only token.
- crates.io sparse index `index.crates.io`: read-only HTTP with a `User-Agent` header, no
  credential (D-00h, D-13).
- `cargo metadata --format-version 1 --no-deps`: the publishable-crate list (twelve today).
- This sandbox is a shallow clone (5,392 commits reachable, `v0.11.0` tag not necessarily
  present); `git log v0.11.0..HEAD -- crates/<crate>/` for D-15 may need an unshallowed fetch of
  the tag or a GitHub compare, and the evidence must say which.
- `.planning/todos/pending/` frontmatter (`owner`, `recheck_by`, `dispositioned_by`) is the
  re-home record the audit register cites (D-09).

</code_context>

<specifics>
## Specific Ideas

- The maintainer took the recommended, conservative option on every question, as in Phases 37,
  37.1 and 47: one reviewable merge commit (D-01), `main` frozen (D-02), the label once on the
  final head (D-03), nothing fixed past the re-seal (D-17), a hard stop on a red semver line
  (D-11), ticks only on what is true (D-04, Claude's discretion). For any unlisted situation:
  **stop and record, do not improvise.**
- "Released, not merely releasable", "verified on what is true, not by promise", and "a red gate
  from the re-seal onward stops the phase" are the governing phrases; SHIP-08 is ticked by the
  registry table and the smoke, not by the merge.
- The audit must read `passed` by construction: every row in `58-AUDIT-DISPOSITIONS.md` carries
  a verdict, and the `tech_debt` list may cite nothing else. "Re-homed" is a dated todo with a
  named owner, never a bare mention.
- The `sk-proj-…` literal is never reproduced anywhere in planning documents, evidence files or
  commit messages; the scrub commit names the file and the line, not the value.
- The release-body size guard, the version-string gate on a bump, the armed semver gate on a
  release PR and the finalize script on anchor-less changelogs all run live for the first time
  here; each first-live result is recorded in `58-CI-EVIDENCE.md` whether or not it surprises.

</specifics>

<deferred>
## Deferred Ideas

- **57-REVIEW CR-01** (worker nack-and-redeliver loses the run outcome on a second-write
  failure) and **WR-01** (empty `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` wipes a YAML secret) —
  `v0.12.0`, maintainer-owned, as dated todos (D-07, D-09).
- **News API key in the URL** (`D-52-03-01`) and the **Treasurer webhook `secret` placeholder
  not refused** (54 WR-04) — the two orphaned Phase 55 deferrals, re-homed with owner and date
  (D-09); the first is a `v0.12.0` security-hardening candidate.
- **FIXES-04 `RunStatusValue::Cancelled`** — the next minor, as Phase 55 D-05 recorded.
- **OSV alerts 23-25 without a governing exception row** (`D-52-03-02`) — maintainer decision,
  dated.
- **A history purge of the `sk-proj-…` literal** — maintainer decision (D-08).
- **Making `release.yml` dry-runnable against a bare SHA before a tag exists** — carried from
  Phases 37 / 37.1 / 47.
- **A standing post-release consumer smoke with a real key** — D-13 ships the keyless half; the
  keyed walk stays with the maintainer's own runs.
- **The `get_job` `#[deprecated]` attribute** — the next minor, per MIGRATION §11.7.
- **Codecov token and `fail_ci_if_error`**, the seven §11 sign-off boxes in
  `.project/v0.10.0/09-program-acceptance-audit.md` — maintainer-only items observed at the
  v0.11.0 close, unchanged.

### Reviewed Todos (not folded)
- **Add OpenAI Responses API support so the pro tier is usable**
  (`2026-10-20-openai-responses-api-for-pro-tier.md`) — new provider capability; out of scope by
  the milestone's cleanup-only rule; acknowledged by the audit with its owner and date.
- **paladin-cli setup-check section order is unstable, keyless runs show bare headings**
  (`2026-11-02-setup-check-first-run-output.md`) — CLI behaviour change in a published crate;
  acknowledged, dated.
- **An unquoted treasurer price with 19 significant digits loses digits on load**
  (`2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md`) — config-loader behaviour
  change; acknowledged, dated.
- **Budget-aware default or reasoning-share hint for reasoning models**
  (`2026-11-04-reasoning-budget-hint-for-deepseek-empty-completion.md`) — adapter behaviour
  change; acknowledged, dated.

</deferred>

---

*Phase: 58-crate-release*
*Context gathered: 2026-11-05*
