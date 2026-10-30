# Phase 52: CI & Release Pipeline Repair - Context

**Gathered:** 2026-10-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Make the CI and release gates every later v0.11.1 phase (53-58) and the `v0.11.1` release itself
run under **actually check something**: the `semver` job compares all twelve publishable crates
against the published `0.11.0` crates.io baseline at the patch release type and reports a non-zero
check count; the crate-wide `cargo-semver-checks` allows that masked removals are gone;
`check-migration-allowlist`, `End-to-End Tests` and `Publish Dry Run` report before the
irreversible merge; `scripts/create-or-reuse-release.sh` never passes the release body through
argv and never needs a manual changelog condensation again; and the three Phase 47 carry-overs
(24 advisory CodeQL alerts, GitHub issue `DF3NDR/paladin-dev-env#41`, the publish-order gate
reading) are closed on the record.

Requirements: CIREL-01, -02, -03, -04, -05, -06, -07, -09 (inventory rows D1-D7). **Not** this
phase: CIREL-08 (the eight `#![allow(rustdoc::…)]`, Phase 55), CIREL-10 (all-features test job,
out by decision D10), the lockstep version bump and the tag (Phase 58), any public-API change.

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by the roadmap, the inventory and earlier phases — not re-asked)

- **D-00a:** Gates as Phase 50/50.1/51 D-00a: 82 % workspace line-coverage floor (ADR-0006);
  `make clean-code`, `make check-gates`, `make test-shell-guards`, `make api-surface`
  (+ `make api-surface-update` and a CHANGELOG entry for an intentional public-surface change),
  `make security`, `make check-api-examples`, `make check-migration-allowlist` and the manual
  credential-handling review in `.github/instructions/security.instructions.md` gate every
  commit. `shellcheck` and `actionlint` already run in CI and bind every script and workflow edit
  here. No `unwrap`/`expect`/`panic!` added to library code (this phase should add no Rust at all
  except a scratch-branch break that is never merged).
- **D-00b:** X-03 / ADR-0060 governs public API: a break needs a `MIGRATION.md` register row
  marked `Y` and a `.cargo/semver-checks-allowlist.toml` entry naming one published crate and one
  **measured** lint, kept set-equal by the gate. This phase changes **which register section the
  gate reads** (D-03 below) and **no public API**; the one known 0.10.1 finding
  (`enum_struct_variant_field_added` on `RunOutcome::Halted.cause`) is expected to vanish at the
  `0.11.0` baseline and is not carried.
- **D-00c:** ADRs take the next free number from `.planning/decisions/PROMOTION.md` (currently
  **0063**) and advance that line in the same commit. No decision below is expected to need a new
  ADR; the advisory-only CodeQL posture (ADR-0036 register shape, Phase 18 verdict) and the
  Trusted Publishing release path stay as recorded.
- **D-00d:** Shipped `MIGRATION.md` sections (§9, §10) are byte-frozen records; `CHANGELOG.md`
  `[0.11.0]` entries are not edited. Workflow trigger changes are registered in the trigger-policy
  register (`check-workflow-triggers`, `docs/src/contributing/branching-model.md`) in the same
  commit, and every new workflow condition is `actionlint`-clean.
- **D-00e:** The semver job addresses crates by **published package name** (`paladin-ai-core`,
  not `paladin-core`); `--default-features` stays (the qdrant rustdoc break is still deferred);
  cargo-semver-checks stays pinned at a tagged release via `taiki-e/install-action`. The
  maintainer-gated acts remain maintainer-gated (branch-protection edits, the Phase 58 tag).
- **D-00f:** Phase 58's release pre-tag audit reads this phase's `VERIFICATION.md`; the first live
  exercise of the repaired release pipeline is the `v0.11.1` tag, not anything in this phase.

### Semver gate re-pin & allow scope (CIREL-01, CIREL-02; inventory D1, D2)

- **D-01:** **Explicit `--release-type patch`** on every package: `cargo semver-checks
  check-release --package <pkg> --default-features --baseline-version 0.11.0 --release-type
  patch`. The `0.9.0` baseline went blind because the manifest delta (0.9 → 0.10/0.11 under 0.x
  rules) was major-equivalent and every lint was skipped; manifests will read `0.11.0 ==
  baseline` until Phase 58's lockstep bump, so the release type is fixed by flag, not inferred.
  Major- and minor-level lints both fire; the gate's verdict cannot drift when the version moves,
  and the same command is the local recipe going forward (supersedes the Phase 44 D-27
  `--release-type minor` diagnostic). Job renamed `Semver Checks (vs v0.11.0)`; `paladin-eval`
  joins the package array (it has a `0.11.0` baseline now), making twelve. The job comment block
  is rewritten for the new baseline (the `paladin-eval` exclusion paragraph goes).
  — **Reversibility:** reversible — one flag and one array edit in `ci.yml`.
- **D-02:** **Measure, then remove every redundant crate-wide allow.** Drop *all* lines under
  `[package.metadata.cargo-semver-checks.lints]` in `crates/paladin-core/Cargo.toml` (eleven
  today) and `crates/paladin-battalion/Cargo.toml` (two), run the D-01 command for every package
  against `0.11.0`, and re-add a crate-wide line **only** for a lint that still fires *and* has a
  registered `Y` row (D-03); anything else that fires is triaged in this phase (fix, or a
  registered break with its `MIGRATION.md` row) — never carried silently. The dated "remove once
  0.11.0 is the baseline" notes are deleted with the lines. The measurement (command, package,
  check count, every lint name that fired) is recorded in the phase evidence file so the next
  phase can repeat it. CIREL-02's six named lines are the minimum; the measured set is the
  deliverable. — **Reversibility:** reversible — manifest metadata only; re-adding a line is one
  commit, but doing so without a register row violates D-00b.
- **D-03:** **Open `MIGRATION.md` section 11 now; the gate reads §11.2 only.** Phase 52 adds a
  `## 11.x` block (v0.11.0 → v0.11.1) above the frozen §10 with an **empty** `## 11.2 Rust API
  changes` register in the §10.2 table shape, and retargets the set-equality window from
  `(9|10).2` to `11.2` in `scripts/check-migration-allowlist.sh` (and therefore CI, see D-06) and
  in `.cargo/semver-checks-allowlist.toml`'s header. The allowlist is emptied to its original
  shipped state (zero `[[entry]]`); §9.2/§10.2 rows keep their `Y` markers untouched. Any break
  Phase 53/55 registers adds a §11.2 row and an entry in the same commit, and if one is needed
  the milestone number question (`v0.11.1` vs `v0.12.0`, scoping note §7) is decided then.
  — **Reversibility:** costly — the window is read by the CI step, the script, its regression
  test fixtures and the register header; widening it back means editing all four and
  re-mirroring frozen rows.
- **D-04:** **Red proof = scratch PR + evidence file.** On a throwaway branch, apply one
  deliberate public break (e.g. remove a `pub fn` from a leaf crate), open a *draft* PR so the
  real `pull_request` `semver` job runs, capture the red job URL and the `N checks: X pass, Y
  fail` log line (plus the green run's non-zero `N checks` line on the phase branch) into
  `52-SEMVER-RED-EVIDENCE.md` (the `49-MUSTER-RED-EVIDENCE.md` pattern), then close the PR
  unmerged and delete the branch. A local run first picks the break and the expected lint name.
  — **Reversibility:** reversible — nothing merges.

### Pre-merge gate triggers (CIREL-03, CIREL-04; inventory D3)

- **D-05:** **`Publish Dry Run` on every PR; `End-to-End Tests` under a `release` label.**
  `publish-dry-run` drops its `main`-push-only condition and runs on every `push` and
  `pull_request` (it stays `needs: [lint, test]`). `e2e-tests` runs on the existing `main` push
  **or** on a pull request carrying the `release` label; `on.pull_request.types` is extended with
  `labeled` (plus the defaults `opened`, `synchronize`, `reopened`) so applying the label
  re-triggers the run. The stale "troubleshooting" comment lines on the e2e job go. The
  concurrency group is unchanged. — **Reversibility:** reversible — job-level `if:` edits.
- **D-06:** **Replace the inline allowlist step with the script.** The duplicated awk step
  "Verify allowlist is set-equal to the MIGRATION.md §9.2 and §10.2 registers" in the `semver`
  job is deleted and replaced by `./scripts/check-migration-allowlist.sh` (placed beside the other
  gate-script steps in the `License & Dependency Policy` job, or kept in the `semver` job — planner's
  call; one implementation either way). The script's "copied VERBATIM from ci.yml / do not improve"
  header is rewritten to say it is the single implementation CI calls. Its regression test
  (`tests/scripts/check-migration-allowlist_test.sh`) and fixtures move to the §11.2 window
  (D-03). The `TBD`-marker step stays. — **Reversibility:** reversible.
- **D-07:** **`release` label is a documented maintainer act.** `docs/src/appendix/release-automation.md`
  and the Phase 58 release checklist state that the release PR carries the `release` label before
  review; the trigger-policy register records the new `labeled` type and the label condition. No
  auto-labeler. — **Reversibility:** reversible.
- **D-08:** **Required where they run.** `Publish Dry Run` becomes a required check on every PR;
  `End-to-End Tests` is required only when it runs (label present — the planner verifies how
  GitHub treats a skipped required check and records the branch-protection wording accordingly);
  the `semver` job stays required under its new name. The branch-protection edit is a maintainer
  act listed in the phase handover checklist with the exact check names.
  — **Reversibility:** reversible — a settings change.

### Release-body builder & its test (CIREL-05; inventory D4)

- **D-09:** **`jq --rawfile body FILE`** inside `_cor_build_payload` replaces `--arg body
  "$(cat FILE)"`; jq reads the file itself so the body never crosses argv and still reaches the
  API as structural JSON data. The function's doc comment is updated to say so.
  — **Reversibility:** reversible.
- **D-10:** **Regression fixture generated at test time.** `tests/scripts/create-or-reuse-release_test.sh`
  gains an assertion that builds a body larger than 131,072 bytes in its `mktemp` scratch dir,
  runs the real script against the stubbed `gh`, and asserts the captured payload's `.body |
  length` exceeds 131,072 and the create call was attempted (and that the pre-fix shape would
  have failed — proven red first). No large fixture file is committed.
- **D-11:** **Handle GitHub's 125,000-character body cap in-script.** When the body exceeds a
  configurable limit (default 125,000 characters; `v0.11.0` shipped at 109,810 after a manual
  condensation of a 156,528-byte section), the script truncates at a line boundary, appends a
  short footer pointing at the full entry (`Full changelog: CHANGELOG.md` + the version anchor),
  and logs the original and final sizes. Tested alongside D-10 (over-cap input → truncated body
  with footer; under-cap input → byte-identical). `finalize-release-body.sh`, which appends
  artifact sections after creation, is checked for the same overflow path and given the same
  guard if it can push a body over the cap. — **Reversibility:** reversible.
- **D-12:** **Verification = shell regression test + shellcheck + actionlint.** No live release
  call, no `workflow_dispatch` dry-run seam in this phase; `make test-shell-guards` is the proof,
  and the first real exercise is Phase 58's tag.

### Carry-over closeouts (CIREL-06, CIREL-07, CIREL-09; inventory D5, D6, D7)

- **D-13:** **CodeQL: triage each of the 24 alerts; fix if cheap and real, else register.** Pull
  the open alerts on the release PR heads with the register's documented `gh api` command (the
  executor needs read access to code-scanning alerts; if the sandbox is refused, the alert list
  is requested from the maintainer and recorded verbatim). A genuine finding with a one-line fix
  is fixed in this phase; test-fixture literals and heuristic false positives get a full
  11-field `[[dismissal]]` row in `CODEQL-DISMISSALS.md` with the "Declared dismissals" count
  updated and `make check-codeql-dismissals` green. The advisory-only posture
  (`codeql.yml` not required) is unchanged. — **Reversibility:** reversible.
- **D-14:** **One shared `review_date` at the next milestone horizon** (about `2027-06-30`) for
  every row Phase 52 adds, noted in the register header so one sweep re-triages them together.
  The six existing rows keep `2026-12-31`.
- **D-15:** **Close issue #41 via the API in-phase with one evidence comment.** The comment cites
  the `v0.11.0` GitHub Release (`https://github.com/DF3NDR/paladin-dev-env/releases/tag/v0.11.0`,
  published 2026-10-26, 10 assets: four platform tarballs with `.sha256`, `SHA256SUMS`, the
  CycloneDX SBOM) and the `release.yml` jobs that produce it (`Create Release`, `Build Binaries`,
  `Generate SBOM`, `Finalize Release Body`), then closes the issue as `completed`. The comment
  text is saved in the phase evidence so the close is reproducible; if the API refuses the
  sandbox token, the text and the close command go to the maintainer instead (Phase 47 D-03
  posture). — **Reversibility:** reversible — an issue can be reopened.
- **D-16:** **Publish-order reading recorded as accepted; no `release.yml` step.** A short dated
  paragraph under "Canonical Publish Order" in `docs/src/appendix/release-automation.md` states
  the two-part gate (`ci.yml` `Check publish order` on every push/PR plus the dependency-ordered
  `publish-crates` job, whose `Pre-Publish Consistency Gate` already requires the whole `ci.yml`
  run on the merge commit to be green), why no separate `release.yml` step is added, and the
  Phase 47 A-SC3 evidence pointer. — **Reversibility:** reversible.

### Claude's Discretion

- Placement of the `check-migration-allowlist.sh` CI step (semver job vs the gate-scripts job),
  D-06.
- The exact wording and anchor of the truncation footer, the line-boundary rule, and the
  environment variable name for the configurable limit, D-11.
- Which throwaway break the red proof uses, D-04.
- Evidence file naming beyond `52-SEMVER-RED-EVIDENCE.md` (e.g. one `52-CI-EVIDENCE.md` for the
  allow measurement, CodeQL triage table and issue-close record is acceptable).

### Folded Todos

- **Verify local `make coverage` reproduces CI's figure** (`.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`):
  folded as a verification note — while this phase is touching `ci.yml`, the planner adds one
  task that runs `make coverage` locally on the phase branch and records the figure next to the
  `Coverage` job's printed figure on the same commit (CI read 92.74 % on `70657919`); a
  mismatch is recorded, not fixed here. No gate change.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase input of record
- `.project/v0.11.1/01-carried-gaps-inventory.md` §D (rows D1-D7) — the verified gap list
  this phase closes, with the Phase 47 evidence pointers and the "in / decide" dispositions.
- `.project/v0.11.1/03-suggestions-and-scoping.md` §1 and §7 — why this phase is first; the
  `v0.11.1`-vs-`v0.12.0` naming rule ("decide at the point the break is known").
- `.planning/ROADMAP.md` "Phase 52" — goal, five success criteria, scope notes (patch release
  type; the one expected-to-vanish finding; CIREL-08 deliberately elsewhere).
- `.planning/REQUIREMENTS.md` "CI and Release Repair (CIREL)" — CIREL-01..09 text.

### Phase 47 evidence (what was measured, where it broke)
- `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md` — D-09
  (semver no-op, `0 checks: 0 pass, 254 skip`), D-14 (gates with no PR twin), B-1 (156,528-byte
  body, 131,072 argv cap, 125,000 GitHub cap), the CI-run job tables.
- `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-01-PLAN.md` — A-SC3 reading
  of the publish-order gate (D-16 records it as accepted).
- `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-08-SUMMARY.md` — the post-merge
  runs where `End-to-End Tests` and `Publish Dry Run` ran for the first time; the `v0.11.0`
  Release object record (body 109,810 chars, 10 assets).
- `.planning/milestones/v0.11.0-MILESTONE-AUDIT.md` — the "24 high alerts, not triaged" line.

### The artifacts this phase edits
- `.github/workflows/ci.yml` — `semver` job (lines ~313-513: comment block, package array,
  inline allowlist step, `TBD` step), `e2e-tests` (~2010), `publish-dry-run` (~2152), the
  `License & Dependency Policy` gate-script steps (~127-166), `on.pull_request` (~22).
- `.github/workflows/release.yml` — read-only context: `create-release` → `finalize-release-body`
  → `check-release-consistency` → `publish-crates` chain (D-16 wording).
- `scripts/create-or-reuse-release.sh` — `_cor_build_payload` (D-09, D-11).
- `scripts/finalize-release-body.sh` — overflow check (D-11).
- `tests/scripts/create-or-reuse-release_test.sh`, `tests/scripts/finalize-release-body_test.sh`,
  `tests/scripts/check-migration-allowlist_test.sh` (+ `tests/scripts/fixtures/`) — the
  stubbed-`gh`, `mktemp`-scratch, accumulate-failures harness pattern every new assertion follows.
- `scripts/check-migration-allowlist.sh` — becomes the single implementation CI calls (D-06),
  window moves to §11.2 (D-03).
- `.cargo/semver-checks-allowlist.toml` — header and entries (D-03).
- `crates/paladin-core/Cargo.toml` (~100-150) and `crates/paladin-battalion/Cargo.toml`
  (~105-116) — the `[package.metadata.cargo-semver-checks.lints]` tables and their dated notes (D-02).
- `MIGRATION.md` — §10.x shape to mirror for the new §11 block (D-03); §9/§10 frozen.
- `CODEQL-DISMISSALS.md` and `scripts/check-codeql-dismissals.sh` — the 11-field row schema,
  count line, staleness rule, and the documented `gh api` reconciliation command (D-13, D-14).
- `docs/src/appendix/release-automation.md` "Canonical Publish Order" — where D-16's paragraph
  and D-07's label note land; `docs/src/contributing/branching-model.md` — trigger-policy register.
- `Makefile` — `check-gates`, `check-migration-allowlist`, `test-shell-guards`, `publish-dry-run`.

### Governing decisions
- `.planning/decisions/PROMOTION.md` — next free ADR number (0063) if one is needed.
- `.planning/milestones/v0.11.0-phases/49-*/49-MUSTER-RED-EVIDENCE.md` — red-proof record shape
  for D-04.
- `.github/instructions/security.instructions.md` — CodeQL advisory-only verdict and the manual
  credential review that still governs `create-or-reuse-release.sh` (it sends a token-bearing
  request).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tests/scripts/*_test.sh` harness pattern (stubbed `gh` on a scratch `PATH` via the `GH_BIN`
  seam, `mktemp -d` + `trap`, `FAILED`/`ASSERTIONS` accumulation, real-tree mutation watch):
  D-10/D-11 assertions slot straight in; `make test-shell-guards` runs them all.
- `scripts/check-migration-allowlist.sh` already exists with the exact CI logic and a
  `MIGRATION_FILE` / `ALLOWLIST_FILE` override seam for fixtures — D-06 is a call-site swap plus a
  window change, not new logic.
- `CODEQL-DISMISSALS.md` register + `check-codeql-dismissals.sh` guard (ADR-0036 shape): D-13
  adds rows to a schema that is already machine-checked (eleven fields, count line, staleness,
  uniqueness, path reachability).
- `taiki-e/install-action` pin of `cargo-semver-checks@0.50.0` and the published-name package
  array: D-01 edits an array and a flag.

### Established Patterns
- Every gate is offline, a named failure never a silent pass, and has a regression test that
  never touches the real tree or the network. New gate logic follows the same rule.
- Workflow triggers are governed: any `on:` change is mirrored in the trigger-policy register and
  checked by `check-workflow-triggers`; inline advisory suppressions in workflows are forbidden
  (`check-workflow-suppressions`).
- Measured, not assumed: the allow table was previously re-derived by dropping the table, running
  the tool, and recording the lints that fired (Phase 44 D-27, Phase 51 plan 51-14). D-02 repeats
  that at the `0.11.0` baseline with `--release-type patch`.
- Maintainer-gated acts (branch protection, tags, merges) are handed over as explicit
  checklists with exact commands/names, never performed by the executor.

### Integration Points
- `ci.yml` `semver` job → `scripts/check-migration-allowlist.sh` → `MIGRATION.md §11.2` ↔
  `.cargo/semver-checks-allowlist.toml` (D-03/D-06 close this loop on one implementation).
- `release.yml` `create-release` step calls `scripts/create-or-reuse-release.sh --body-file`;
  `finalize-release-body` later PATCHes the same body (D-11 guards both ends).
- Phase 53/55 public-signature work and Phase 58's lockstep bump both run under the D-01 gate;
  Phase 58's pre-tag audit reads this phase's `VERIFICATION.md` and the `52-SEMVER-RED-EVIDENCE.md`.
- Branch protection (maintainer) must rename the required `semver` check and add `Publish Dry
  Run` (D-08) for the gates to actually block.

</code_context>

<specifics>
## Specific Ideas

- The semver job must never again report `0 checks`: the green run's own log line (`N checks: X
  pass, 0 fail`, N > 0, for all twelve packages) is recorded as evidence alongside the red proof.
- "Measure, then remove" applies to all thirteen crate-wide allow lines, not only the six named
  in CIREL-02 — a surviving line is one the tool still needs *and* the register still owns.
- The truncation footer is the only acceptable "condensation" from now on: it is automatic,
  logged, and tested.
- Issue #41's closing comment is evidence, not prose: Release URL, asset list, producing jobs.

</specifics>

<deferred>
## Deferred Ideas

- **`workflow_dispatch` dry-run seam for `release.yml`** (build the payload from the current
  Unreleased section without creating a release) — raised while discussing D-12; a Phase 58
  release-prep candidate, not this phase.
- **Auto-labelling `release/**` PRs** — rejected for now in favour of a documented maintainer act
  (D-07); revisit if the manual label is forgotten once.
- **Re-check the three `zip` RustSec exceptions** (inventory D8, review date 2026-12-31) — out of
  this phase by inventory disposition; belongs to whichever phase closes nearest that date.

### Reviewed Todos (not folded)
- `2026-10-13-review-content-data-orchestration-base-items.md` — keyword match only (paladin-core
  removals); not a CI/release item. Stays pending.
- `2026-10-14-web3sec-single-bump-to-v0-11-0.md` — the downstream consumer's own move, already
  satisfied at `v0.11.0`; its remaining act belongs to the consumer, not to CI.
- `2026-10-20-openai-responses-api-for-pro-tier.md` — ruled out at the milestone open ("OpenAI
  Responses API out"); not CI.

</deferred>

---

*Phase: 52-ci-release-pipeline-repair*
*Context gathered: 2026-10-28*
