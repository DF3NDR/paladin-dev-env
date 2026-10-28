# Phase 52: CI & Release Pipeline Repair - Research

**Researched:** 2026-10-28
**Domain:** GitHub Actions CI gates, `cargo-semver-checks` re-baselining, shell release tooling (`jq`, `gh`), governance registers (MIGRATION.md / CodeQL / ruleset)
**Confidence:** HIGH on everything measured locally in this session (semver counts, argv failure, `jq --rawfile`, publish dry-run warnings, ruleset contents). MEDIUM on GitHub platform semantics that could not be read from official docs here (`docs.github.com` is not reachable from this sandbox) and on the 125,000-character release-body cap (community reports only).

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried forward (locked by the roadmap, the inventory and earlier phases — not re-asked)

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

#### Semver gate re-pin & allow scope (CIREL-01, CIREL-02; inventory D1, D2)

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

#### Pre-merge gate triggers (CIREL-03, CIREL-04; inventory D3)

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

#### Release-body builder & its test (CIREL-05; inventory D4)

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

#### Carry-over closeouts (CIREL-06, CIREL-07, CIREL-09; inventory D5, D6, D7)

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

### Deferred Ideas (OUT OF SCOPE)

- **`workflow_dispatch` dry-run seam for `release.yml`** (build the payload from the current
  Unreleased section without creating a release) — raised while discussing D-12; a Phase 58
  release-prep candidate, not this phase.
- **Auto-labelling `release/**` PRs** — rejected for now in favour of a documented maintainer act
  (D-07); revisit if the manual label is forgotten once.
- **Re-check the three `zip` RustSec exceptions** (inventory D8, review date 2026-12-31) — out of
  this phase by inventory disposition; belongs to whichever phase closes nearest that date.

(Reviewed, not folded: `2026-10-13-review-content-data-orchestration-base-items.md`,
`2026-10-14-web3sec-single-bump-to-v0-11-0.md`, `2026-10-20-openai-responses-api-for-pro-tier.md`.)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| CIREL-01 | Semver job checks all twelve publishable crates vs `0.11.0`, named for the baseline, non-zero check count | Measured: all 12 packages return `223 checks: 223 pass, 31 skip` at `--release-type patch` once allows are removed (table in "Measured Results"). Job-rewrite snippet, zero-check tripwire recommendation, red-proof recipe (§Code Examples 1, §Pattern 1). |
| CIREL-02 | Crate-wide allows in core/battalion and their allowlist entries removed; job stays green | Measured: removing **all** lint tables passes. **Tree has six manifests with tables, 28 lines (not two manifests / 13 lines)**; the gate script's local extension forces all six to go once the allowlist is emptied (Discrepancy 1). |
| CIREL-03 | `check-migration-allowlist` runs in `ci.yml` on every PR | Script is already the single implementation; call-site swap; recommend the `License & Dependency Policy` job (required) not the advisory `semver` job (Discrepancy 2, §Pattern 3). Window retarget to §11.2 and its pitfall (§Pitfall 3). |
| CIREL-04 | `End-to-End Tests` and `Publish Dry Run` run before merge | `if:`/`types:` edits (§Code Examples 2). Verified: dry run at an already-published version only warns (§Measured). GitHub treats a job-level-`if` skip as success (PR #59 merged with E2E required+skipped). |
| CIREL-05 | Release body not passed through argv; regression test > 131,072 bytes | Reproduced exit 126 `Argument list too long` at 140 KB; `jq --rawfile` verified (§Code Examples 3). D-10/D-11 interaction trap (§Pitfall 4). |
| CIREL-06 | 24 CodeQL alerts triaged into `CODEQL-DISMISSALS.md` | Register schema, guard clauses and row template documented; sandbox gets HTTP 403 on code-scanning alerts, so the alert list is a maintainer hand-off (§Environment, §Pattern 6). |
| CIREL-07 | Issue #41 closed with reference to the `v0.11.0` Release | Issue #41 (open, no comments) and the Release (published 2026-10-26T20:29:52Z, 10 assets, body 109,810 chars) verified via public API; evidence-comment text drafted (§Code Examples 6). Close is likely a hand-off (no valid write token). |
| CIREL-09 | Publish-order gate reading recorded as accepted | Insertion point and A-SC3 evidence pointer located (§Pattern 7). |
</phase_requirements>

## Summary

This phase is configuration, shell and register work; it adds no Rust. Almost everything in CONTEXT.md checks out against the tree, and the central mechanism was measured here: `cargo-semver-checks` 0.50.0 (the same version CI pins) runs locally against the published `0.11.0` crates and, with `--release-type patch`, evaluates 217-223 lints per crate instead of skipping all 254. With every crate-wide allow removed, all twelve publishable packages pass with an identical `223 checks: 223 pass, 31 skip`. A deliberate break (making `paladin_ports::output::spend_scope::current_attempt_spend` private) turned the gate red locally: `217 checks: 216 pass, 1 fail, 0 warn, 37 skip`, lint `function_missing`, exit 100. So the re-pin is real, the allows are all redundant at this baseline, and no triage of surprise findings is needed.

Four things in the tree differ from what CONTEXT assumes, and the planner must resolve them: (1) six manifests, not two, carry crate-wide semver allow tables (28 lines), and the gate script's existing "every crate-wide allow must be mirrored" extension will fail CI on the four others the moment the allowlist is emptied; (2) the `semver` job is **not** a required check today (the live ruleset and `docs/src/deployment/cicd.md` both say advisory), so D-08's "stays required" is wrong as stated; (3) `End-to-End Tests` is **already** pinned required in the live ruleset, and its PR skip counts as a pass (PR #59 merged that way), so only `Publish Dry Run` (and, if the maintainer agrees, the semver check) is a new ruleset entry; (4) D-10's "payload body longer than 131,072" cannot coexist with D-11's default 125,000-character cap unless the D-10 test raises the configurable limit.

The release-body fix is small and well understood: the only argv leak is `jq --arg body "$(cat FILE)"`; the rest of the path already uses stdin or files. The 125,000 cap guard must be character-based (use `jq`/`python3`, not `wc -m`, which counts bytes in the C locale), must leave headroom for the artifact sections `finalize-release-body.sh` appends, and must be idempotent across a finalize re-run.

**Primary recommendation:** Re-pin the semver job with `--release-type patch` on a twelve-package array and remove all six crate-wide allow tables together with the 85 allowlist entries and the §9.2/§10.2 window; run `scripts/check-migration-allowlist.sh` from the required `License & Dependency Policy` job against a new empty §11.2 register (with a `## 11.3` terminator); switch the payload builder to `jq --rawfile` with a shared, character-counting, line-boundary truncation helper; and hand the ruleset PUT, the CodeQL alert list and (likely) the issue close to the maintainer with exact commands.

## Architectural Responsibility Map

This phase has no browser/API/database tiers. The relevant tiers are the CI/release control plane:

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Semver verdict (does a public-API change fail?) | `ci.yml` `semver` job (workflow YAML) | Per-crate `Cargo.toml` metadata (suppression), `.cargo/semver-checks-allowlist.toml` (register mirror) | The tool runs only in CI/locally; manifests hold suppression; the register is review traceability, not the mechanism. |
| Allowlist ↔ register set-equality | `scripts/check-migration-allowlist.sh` (single implementation) | `ci.yml` call site, `make check-gates` | One offline script; CI and Make both call it (D-06). |
| Pre-merge gate firing (E2E, dry run) | `ci.yml` job-level `if:` + `on.pull_request.types` | GitHub ruleset (required-context list) | `if:` decides whether it runs; the ruleset decides whether it blocks. They are separate acts (workflow edit vs maintainer settings PUT). |
| Required-vs-advisory classification | GitHub repository ruleset id `20868126` (live) | `.github/rulesets/protect-main-branch.json` (committed record, read by `check-workflow-triggers` clause 3), `docs/src/appendix/branch-protection.md`, `docs/src/deployment/cicd.md` | Live state is authoritative; the repo copy and docs must be edited to match, and applying is a maintainer act. |
| Release-body construction | `scripts/create-or-reuse-release.sh` (payload) + `scripts/finalize-release-body.sh` (artifact sections) | `release.yml` `create-release` / `finalize-release-body` jobs (thin callers) | All logic lives in scripts with a stubbed-`gh` harness; the workflow only passes a file path. |
| Release-body size policy (125,000 cap) | A single shared helper called by both release scripts | Regression harness `tests/scripts/*_test.sh` | Two diverging copies is the failure mode D-06 was written to avoid. |
| CodeQL governance | `CODEQL-DISMISSALS.md` + `scripts/check-codeql-dismissals.sh` (offline) | GitHub code-scanning alert store (enforcement surface, maintainer/API) | Register is governance; the alert store is enforcement; reconciled by a manual `gh api` command. |
| Issue #41 closure | GitHub issue (API) | Phase evidence file (comment text, reproducible) | A platform write; falls back to a maintainer hand-off. |

## Standard Stack

No new external packages are introduced by this phase. Everything below already exists in the tree or CI.

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| `cargo-semver-checks` | 0.50.0 (CI pin via `taiki-e/install-action@v2`; local binary is the same version) | Public-API diff against crates.io `0.11.0` | D-00e pins a tagged release; measured here at 0.50.0. 0.51.0 exists on the registry; do **not** bump in this phase (measurements and the "register-only" allowlist wording are for 0.50.0). [VERIFIED: `cargo semver-checks --version` = 0.50.0; index.crates.io shows 0.51.0 latest] |
| `jq` | local 1.7; `--rawfile` exists since jq 1.5 | Structural JSON payload construction without argv | Already used by `_cor_build_payload`. [VERIFIED: `jq --version`; `--rawfile` run locally] |
| `bash` + `shellcheck` | shellcheck 0.9.0 locally; pre-commit pins `shellcheck-py v0.10.0.1` at `--severity=warning` | Scripts and regression harnesses | House style; new code must be clean at `--severity=warning` (existing files carry info/style findings, not blocking). [VERIFIED: `.pre-commit-config.yaml`] |
| `actionlint` | 1.7.12 (CI downloads it) | Lint every workflow edit | Runs in CI (`Workflow Lint`, a required check). Not runnable here (see Environment). [VERIFIED: ci.yml `actionlint` job] |
| `python3` (+ `tomllib`, `yaml`) | 3.13.16, PyYAML 6.0.1 locally | Text processing inside scripts (house pattern: `python3 - <<'PY'`); `check-codeql-dismissals.sh`, `check-workflow-triggers.sh` | Already required by existing gates. [VERIFIED] |
| `gh` CLI | installed; `GH_TOKEN` in env is **invalid** | Alert pull, issue close, ruleset PUT | See Environment Availability. [VERIFIED: `gh auth status`] |

### Supporting
| Tool | Purpose | When to Use |
|------|---------|-------------|
| `taiki-e/install-action@v2` | Installs the pinned `cargo-semver-checks` in CI | Unchanged. |
| `Swatinem/rust-cache@v2` with `key: semver-checks` | Caches builds for the semver job | Unchanged; a 12th package adds one more rustdoc build per side. |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `jq --rawfile` | `jq -n --slurpfile`, or building the whole payload with `python3` | `--rawfile` is the one-flag change D-09 locks; python is only worthwhile for the truncation helper. |
| Truncating with `wc -m` / `cut -c` | `jq -Rs length` / `python3 len()` | `wc -m` counts **bytes** under the C locale (measured here: `wc -m` = 140039 for a body that is 140035 characters). Use a code-point-counting tool. |
| New CI step in the advisory `semver` job | Step in the required `License & Dependency Policy` job | Placement discussed in Pattern 3; the required job is the only placement that actually blocks a merge. |

**Installation:** nothing to install. `cargo install cargo-semver-checks --version 0.50.0 --locked` reproduces the local tool if needed.

**Version verification:** `cargo semver-checks --version` → `cargo-semver-checks 0.50.0`; crates.io index lists `0.50.0` and `0.51.0` (both unyanked) on 2026-10-28.

## Package Legitimacy Audit

This phase installs **no new external packages** (no `npm`, `pip`, or `cargo add`). The only third-party code touched is the already-pinned `cargo-semver-checks@0.50.0` (installed by `taiki-e/install-action`, unchanged since Phase 22, T-22-SC) and the CI-downloaded `actionlint` 1.7.12 (unchanged). The `package-legitimacy` seam was therefore not run.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| cargo-semver-checks 0.50.0 | crates.io | existing pin, multi-year project | — | github.com/obi1kenobi/cargo-semver-checks (the lint-file `impl:` URL in the tool's own output) | OK (pre-existing pin, no change) | Approved, unchanged |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
*Planner: no `checkpoint:human-verify` install gate is needed for this phase.*

## Measured Results (this session, local, HEAD `3ddf1aa89`)

All runs: `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.11.0 --release-type patch`, toolchain 1.97.1 (the repo's `rust-toolchain.toml` pin), baseline fetched from crates.io. [VERIFIED: local runs]

| Package (published name) | Check line **with current allow tables** | Check line **with that crate's table removed** | Exit |
|---|---|---|---|
| `paladin-ai-core` | not run | `223 checks: 223 pass, 31 skip` | 0 |
| `paladin-battalion` | not run | `223 checks: 223 pass, 31 skip` | 0 |
| `paladin-ports` | `217 checks: 217 pass, 37 skip` | `223 checks: 223 pass, 31 skip` | 0 |
| `paladin-ai` (facade, root manifest) | `222 checks: 222 pass, 32 skip` | `223 checks: 223 pass, 31 skip` | 0 |
| `paladin-llm` | `219 checks: 219 pass, 35 skip` | `223 checks: 223 pass, 31 skip` | 0 |
| `paladin-web` | `219 checks: 219 pass, 35 skip` | `223 checks: 223 pass, 31 skip` | 0 |
| `paladin-herald`, `paladin-memory`, `paladin-storage`, `paladin-notifications`, `paladin-content`, `paladin-eval` (no tables) | `223 checks: 223 pass, 31 skip` | — | 0 |

Conclusions the planner can rely on:
- Every crate-wide `allow` merely **skips** a lint (the skip count rises as allows are added: 31 → 37 for ports). Removing all six tables yields a uniform expected log line `223 checks: 223 pass, 31 skip` for all twelve packages and `Summary no semver update required`. The "31 skip" is the tool's own inapplicable-lint count, not an allow.
- At `--release-type patch` the tool states `v0.11.0 -> v0.11.0 (assume patch change)` and evaluates 196 major + 58 minor lints minus 31 skipped. [VERIFIED: `cargo semver-checks --list` = 196 major, 58 minor]
- The previously known `enum_struct_variant_field_added` finding (`RunOutcome::Halted.cause`) does not occur at the `0.11.0` baseline, as D-00b predicted. **No lint fired anywhere**, so D-02's "triage anything that fires" branch is empty. The measurement is repeatable in roughly 1.5 minutes per crate (about 5 minutes total once builds are cached).
- Red proof, local: in `crates/paladin-ports/src/output/spend_scope.rs` line 230, `pub fn current_attempt_spend` → `fn current_attempt_spend` gave `217 checks: 216 pass, 1 fail, 0 warn, 37 skip`, a `--- failure function_missing: pub fn removed or renamed ---` block naming `paladin_ports::output::spend_scope::current_attempt_spend`, `Summary semver requires new major version: 1 major and 0 minor checks failed`, **exit 100**. (Run against the current ports table, which does not allow `function_missing`; after removal the line would read `223 checks: 222 pass, 1 fail, 0 warn, 31 skip`.) The tree was restored with `git checkout` and is clean. The clean-run line has no `fail`/`warn` fields; the failing line does. Parse with a tolerant `[0-9]+ checks:` pattern, not a fixed field layout.
- **A bad red-proof target exists:** `paladin-herald`'s `MarkdownHerald`/`JsonHerald` are `#[doc(hidden)]`; its rustdoc JSON index holds 3 items, so privatising or renaming a herald method is **not detected** (two attempts, exit 0). The break must be on an item that appears in rustdoc JSON (the ports free function above works).
- `cargo publish --workspace --dry-run` at the already-published `0.11.0` tree prints twelve `warning: crate <name>@0.11.0 already exists on crates.io index` lines and proceeds to packaging (observed through `Packaged` for the first five crates before my 90 s timeout). It does not hard-fail at that step. Completion of the verify/build phase was not observed locally. [VERIFIED up to packaging]
- Live ruleset `20868126` (public read): 44 required contexts, `strict_required_status_checks_policy: true`; contains `End-to-End Tests`; contains **neither** a semver context **nor** `Publish Dry Run`. [VERIFIED: `GET /repos/DF3NDR/paladin-dev-env/rulesets/20868126`]
- `v0.11.0` Release: `published_at` 2026-10-26T20:29:52Z, body length 109,810, 10 assets (`paladin-{linux,macos}-{amd64,arm64}.tar.gz` + `.sha256` each, `paladin-v0.11.0.cdx.json`, `SHA256SUMS`). Issue #41: open, no comments, created 2026-08-28. [VERIFIED: public GitHub API]

## Discrepancies Between CONTEXT.md and the Tree (planner must decide)

1. **Six manifests carry crate-wide semver allow tables, not two.** [VERIFIED: grep]

   | Manifest | Package | Lines | Table starts at |
   |---|---|---|---|
   | `Cargo.toml` (root) | `paladin-ai` | 1 (`constructible_struct_adds_field`) | line 123 |
   | `crates/paladin-core/Cargo.toml` | `paladin-ai-core` | 11 | 136 |
   | `crates/paladin-battalion/Cargo.toml` | `paladin-battalion` | 2 | 113 |
   | `crates/paladin-llm/Cargo.toml` | `paladin-llm` | 4 | 146 |
   | `crates/paladin-ports/Cargo.toml` | `paladin-ports` | 6 | 77 |
   | `crates/paladin-web/Cargo.toml` | `paladin-web` | 4 | 92 |

   CONTEXT/CIREL-02 name core and battalion (13 lines). **Why this is forced:** D-03 empties the allowlist to zero `[[entry]]`, and `scripts/check-migration-allowlist.sh` already fails when any manifest lint in *any* crate has no matching allowlist entry for that crate (the "local-only extension"). Because D-06 makes CI call the script, leaving the root/llm/ports/web tables in place turns CI red. Measured above: removing all six is safe. **Recommendation:** extend D-02's measure-then-remove to all six manifests (28 lines) under D-02's own wording "every redundant crate-wide allow"; delete the dated explanatory comment blocks above each table with the lines (the root `Cargo.toml` block at ~97-122, the ports block at ~58-86, the llm block at ~90-145, the web block, core ~38-135, battalion ~96-112 exist solely to justify the removed lines). Record the 28-line count and the per-package `223 checks` line in the evidence file. Alternative (not recommended): keep entries and rows for the four crates, which contradicts D-03's "zero entries".

2. **The `semver` job is advisory today, not required.** Evidence: `.github/rulesets/protect-main-branch.json` and the live ruleset list no semver context; `docs/src/deployment/cicd.md` line 65 says `Advisory`. D-08 says "the `semver` job stays required under its new name". As the tree stands, a red semver job does not block a merge, which undercuts "gates that actually check something". **Recommendation:** have the maintainer add `Semver Checks (vs v0.11.0)` to the required set (it's an additive PUT), update `cicd.md` row to `Required`, and say so plainly in the hand-over; or, if the maintainer wants it to stay advisory, state that and drop the "required" wording. Either way this needs a one-line user confirmation. (See Open Questions.)

3. **`End-to-End Tests` is already a required context** in the live ruleset (and in the committed JSON), yet today it runs only on `main` push. It has been "required but always skipped" on PRs, and PR #59 merged that way (47-CI-EVIDENCE line 360: both gates "appear as `skipped`" on both events). GitHub reports a job skipped by a job-level `if:` as **success** for required-check purposes, whereas a *workflow* skipped by path/branch filtering leaves checks **Pending** and blocks merge. [CITED: GitHub "Troubleshooting required status checks" via search summary of docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks; corroborated by the PR #59 history] So the D-08 wording is: E2E needs **no ruleset change**; with the `release` label it runs and its failure blocks; without the label its skip passes. `Publish Dry Run` is the only new context (plus semver per item 2). The ruleset's `required_status_checks` list is also read by `check-workflow-triggers.sh` clause 3 (each context must resolve to a job `name:` in some workflow), so a committed JSON edit adding `Publish Dry Run` / `Semver Checks (vs v0.11.0)` is validated for free; the live PUT is the maintainer act in `docs/src/appendix/branch-protection.md` "Updating an already-applied ruleset" (id `20868126`, whole-file `--input`, then read back and check `length == 3`). Docs to update with the count: `branch-protection.md` (currently "all 44 required checks", plus the enumerated list) and `cicd.md` rows for `semver` (line 65), `e2e-tests` (83) and `publish-dry-run` (85).

4. **D-10 vs D-11 collide on size.** A body just over 131,072 bytes is also over the 125,000-character default cap, so with the default limit the guard truncates it and the payload `.body | length` can never exceed 131,072. The D-10 argv regression test must set the configurable limit above the test body (e.g. 500,000) so the payload really carries more than 131,072 characters; the D-11 over-cap test runs at the default limit and asserts truncation. Both are needed.

5. Line numbers in CONTEXT are approximate; current positions: `on.pull_request` 22-23; `cargo-deny` ("License & Dependency Policy") job 114-192 with the gate-script steps 127-166; semver comment block 313-344 and job 345-513 (inline allowlist step 419-490, TBD-marker step 492-513); `e2e-tests` 2010-2070 (`if:` at 2015); `publish-dry-run` 2152 (`if:` at 2156); only three job-level `if:` exist in `ci.yml` (e2e, benchmark-signal at 2075, publish-dry-run).

6. The ci.yml header comment claims push and PR runs "share a concurrency group". They do not: `github.head_ref` is the bare branch name on `pull_request` while `github.ref` is `refs/heads/<branch>` on `push`, so the groups differ. [ASSUMED - standard context semantics, docs unreachable] Consequence: after D-05, `Publish Dry Run` produces two check runs per PR update (one per event). Harmless for required-check satisfaction; costs a few runner minutes.

## Architecture Patterns

### System Architecture Diagram

```
 contributor/maintainer push or PR event ──► ci.yml (on: push '**', pull_request [main, release/**] + types incl. labeled)
                                                  │
        ┌──────────────────┬──────────────────────┼──────────────────────────┬────────────────────────┐
        ▼                  ▼                      ▼                          ▼                        ▼
 License & Dependency   Semver Checks        Integration Tests          Publish Dry Run          End-to-End Tests
 Policy (REQUIRED)      (vs v0.11.0)         (needs for E2E)            (needs lint,test)        (needs integration-tests)
   gate scripts:          for pkg in 12:                                 runs every event         if: main push
   …check-publish-order   semver-checks                                  (D-05)                    OR PR with `release` label
   …check-migration-      --release-type patch                                                        │
     allowlist.sh  ◄──────  reads Cargo.toml tables (now NONE)                                        │
        │  (D-06)            baseline = crates.io 0.11.0                                              │
        ▼                                                                                             │
 MIGRATION.md §11.2 (empty) ⇄ .cargo/semver-checks-allowlist.toml (empty)                             │
                                                                                                      ▼
                                   GitHub ruleset 20868126 (maintainer PUT): contexts by job name; job-level skip == success

 tag v*.*.* ──► release.yml
   create-release ──► extract-changelog-section.sh ──► body file ──► create-or-reuse-release.sh
                                                              (jq --rawfile ──► fit/truncate helper ──► POST releases via stdin)
   build/sbom ──► finalize-release-body.sh (curated section + artifact block ──► same size guard ──► gh release edit --notes-file)
   check-release-consistency (needs whole ci.yml run green) ──► publish-crates (dependency order)
```

### Recommended Project Structure (files this phase touches)
```
.github/workflows/ci.yml              # semver job, e2e-tests/publish-dry-run `if:`, on.pull_request.types, gate-script step
.github/rulesets/protect-main-branch.json   # committed record only; live PUT is the maintainer act
scripts/check-migration-allowlist.sh  # window -> 11.2/11.3, header says "single implementation"
scripts/create-or-reuse-release.sh    # --rawfile + size guard
scripts/finalize-release-body.sh      # same guard on the composed body
scripts/<shared size helper>          # one implementation for both release scripts (recommended)
tests/scripts/check-migration-allowlist_test.sh
tests/scripts/create-or-reuse-release_test.sh
tests/scripts/finalize-release-body_test.sh
.cargo/semver-checks-allowlist.toml   # zero [[entry]]; header rewritten for §11.2
Cargo.toml, crates/{paladin-core,paladin-battalion,paladin-llm,paladin-ports,paladin-web}/Cargo.toml  # tables + justification comments deleted
MIGRATION.md                          # new "# 11." block above §10, scope note updated to three parts
CODEQL-DISMISSALS.md                  # new rows, count line, heading, shared review_date note
docs/src/appendix/release-automation.md, release-checklist.md, branch-protection.md,
docs/src/deployment/cicd.md, docs/src/contributing/branching-model.md   # label act, publish-order paragraph, required set, trigger row, eval no longer excluded
.planning/phases/52-…/52-SEMVER-RED-EVIDENCE.md, 52-CI-EVIDENCE.md      # evidence (names are discretion)
```

### Pattern 1: Semver step (CIREL-01, D-01)
**What:** One loop, patch release type, twelve published names. **Recommended additions** (extending D-01, low risk): run all packages and fail at the end, and fail if any package evaluated zero checks, so a future baseline/version drift can never silently become a no-op again.
**When to use:** the `semver` job's single check step.
**Example:** see Code Examples 1.

### Pattern 2: Gate-script call sites (CIREL-03, D-06)
**What:** Replace the 70-line inline awk step with `./scripts/check-migration-allowlist.sh`; the script file is mode 100755 and the `cargo-deny` job already runs `chmod +x scripts/*.sh`. The script header ("copied VERBATIM … do not improve", "LOCAL-ONLY EXTENSION (not in the ci.yml step)") must be rewritten: the extension is now enforced in CI as well, and the comment on `check-gates` in `Makefile` line 213 (`§9.2 register`) should say `§11.2`.
**Placement recommendation:** put the step in `cargo-deny` ("License & Dependency Policy"), beside `Check publish order` and `Check core/ports dependency allowlist`. That job is a **required** check and runs on every push/PR with no `if:`; the `semver` job is advisory (Discrepancy 2), so a set-equality failure there would not block. The script needs only `awk`, `grep`, `comm`, `sort`, `diff` (no network, no rust cache), so it fits that job's cheap steps. Keep the `TBD`-marker step where it is (semver job) or move it alongside; either is fine, but if the semver job stays advisory, moving the marker step beside the script keeps it blocking.

### Pattern 3: Retargeting the register window (D-03)
**What:** In the script, replace `/^## (9|10)\.2 /` with `/^## 11\.2 /` and `/^## (9|10)\.3 /` with `/^## 11\.3 /`; update the two `echo` lines and the `::error::` text.
**Critical:** the awk window ends only at a `## 11.3 ` heading. The new §11 block **must contain `## 11.3 …`** (give it the full §10.x skeleton: 11.1-11.8 headings, or at least 11.3), otherwise the window runs to end-of-file and swallows every §10.2/§9.2 `Y` row, failing set-equality against the empty allowlist.
**New §11 block shape:** a `# 11. Upgrading from v0.11.0 to v0.11.1` part placed between the scope note and `# 10.`; sections `## 11.1 Behavioral changes`, `## 11.2 Rust API changes (compile-affecting; the X-10 register)` with the same six-column table header as §10.2 (`| Crate | Type | Change | Mitigation | Deliberate breaking? | Req |`, copy the exact header and separator row from line ~67-70 of `MIGRATION.md`) and **no data rows**, then 11.3-11.8. The scope note (line 3, "has two parts, newest first") must become three parts; §9 and §10 section bodies stay byte-frozen (the scope note is outside them). **Never write the four-letter placeholder token anywhere in MIGRATION.md**; the `TBD`-marker step fails the build on any occurrence, including in prose that describes it.

### Pattern 4: `if:` and `types:` edits (CIREL-04, D-05)
See Code Examples 2. Notes: `labeled` fires for **any** label and re-runs the **entire** `ci.yml`; there is no way to filter `types:` by label name, and `github.event.label.name` filtering would have to be added to every job. Accept the cost: this is a single-maintainer repo, label events are rare, and `cancel-in-progress` on non-`main` refs collapses overlapping runs. (Dependabot-style auto-labels at PR creation will cancel-and-restart the first run; noise only.) Removing a label does not re-run anything (`unlabeled` is not in `types`); adding the label after the last push is the intended trigger. `pull_request.labels` is populated on every `pull_request` activity type, so later `synchronize` pushes keep running E2E while the label is present.

### Pattern 5: Release-body size guard (CIREL-05 / D-09, D-11)
- **Builder change (D-09):** `jq -n --arg tag_name … --arg name … --rawfile body "${body_file:-/dev/null}" …`. `--rawfile` preserves trailing newlines that `$(cat FILE)` stripped (measured: file `x\n\n` → length 3 vs 1). Existing case 8 (literal `EOF`, backticks, `$(…)`) must still pass; the payload gains the file's trailing newline(s). This is a harmless one-time difference but state it in the evidence. When `--body-file` is omitted the old code sent an empty body; keep that with `/dev/null`.
- **Where argv is *not* a problem (verified by reading the code):** `printf '%s' "${payload}" | gh api … --input -` uses a builtin and stdin; `PAYLOAD=$(…)`, `RELEASE_JSON`, and function arguments are bash-internal; `finalize-release-body.sh` passes bodies via `--notes-file`. The one argv leak is the `jq --arg body`.
- **Cap guard (D-11) design, prescriptive:**
  1. Measure **characters** with a code-point-aware tool (`python3 len()` in the house `python3 - <<'PY'` style, or `jq -Rs length FILE`). Never `wc -m`/`wc -c`/`${#var}` under an unknown locale.
  2. If length ≤ limit → pass the source file through untouched (so under-cap input is byte-identical to the file).
  3. Else budget = limit − len(footer); cut at the last `\n` at or before the budget; if no newline exists within the budget, hard-cut at the budget (never produce an empty body); if the kept text has an odd number of "```" fence lines, append a closing fence before the footer so the rest of the release page does not render as code.
  4. Append the footer, log `original N chars, limit L, final M chars` to stderr, write the fitted copy to a `mktemp` file (never edit the caller's file), and feed that to `--rawfile`.
  5. Limit variable: read from an environment variable (name is discretion, e.g. `PALADIN_RELEASE_BODY_LIMIT`), default 125000; a non-integer or a value ≤ footer length is a named failure.
  6. **Footer:** a fixed sentinel line (e.g. an HTML comment `<!-- paladin:release-truncated -->`) plus a one-line pointer to `CHANGELOG.md` for the tagged commit (`https://github.com/<repo>/blob/<tag>/CHANGELOG.md`). Avoid computing a heading anchor: GitHub anchors derive from the heading text including the date, which the script does not know.
- **Finalize interplay:** `finalize-release-body.sh` keeps the curated section (text before `<!-- paladin:release-artifacts -->`) and appends ~1-3 KB of artifact sections, so a body truncated to exactly the limit at create time would overflow on finalize and `gh release edit` would be rejected. Two layers: (a) at create time fit to `limit − reserve` (reserve a few thousand characters, documented); (b) in finalize, after `compose_release_body`, measure the composed file and, if over the limit, re-fit the curated part to `limit − artifact_block_length`. The sentinel makes this idempotent: strip an existing footer from the curated text before re-truncating, then re-append one footer. A second finalize run over its own output must stay byte-identical (existing case 12/18 pattern).
- **One implementation:** put the size logic in a single helper that both scripts call (a new `scripts/` file with a `*_LIB_ONLY` sourcing seam is consistent with the house pattern). Do not paste the same 25 lines into two scripts.

### Pattern 6: CodeQL triage (CIREL-06, D-13/D-14)
Row schema (all eleven fields mandatory, non-empty; TOML inside the fenced block between the `BEGIN`/`END MACHINE-READABLE REGISTER` markers): `alert_number, rule_id, path, why_present, why_dismissed, dismissed_reason, owner, review_date, scope, compensating_control, revisit_condition`. Guard clauses: schema, drift (the `Declared dismissals: N` line must equal the row count), staleness (`review_date` ISO and not earlier than today), uniqueness of `alert_number`, reachability (`path` minus an optional `:LINE[:COL]` suffix must exist, or `scope` must contain the word "removed"). Edit three places per addition: the TOML rows, the `Declared dismissals: N` line, and the `## The N live dismissals` heading (prose; not machine-checked but should match). `dismissed_reason` values in use are `false positive` and `used in tests` (GitHub's reasons also include `won't fix`). Use one shared `review_date` (`2027-06-30`) and a header sentence saying so (D-14). Existing rows: alerts 28, 40, 45, 46, 47, 49 (review dates 2026-12-31 and 2027-03-31). A fixed alert needs no row. The documented reconciliation command is `gh api "/repos/DF3NDR/paladin-dev-env/code-scanning/alerts?state=dismissed&per_page=100" --paginate`; the open-alert pull is the same with `state=open` (and `&ref=refs/pull/59/head` or `refs/heads/main` for the release PR heads; after the PR #59 merge the same findings are most likely open alerts on `main`). [ASSUMED: ref semantics]. **The sandbox cannot do this** (HTTP 403, see Environment); the plan must make the alert list a maintainer hand-off with the exact command, and make the register rows a follow-on task that consumes the pasted list. Dismissal on the platform itself (`PATCH /code-scanning/alerts/{n}` with `dismissed_reason`) is also a write that needs a valid token; a register row without the matching platform dismissal is the "governance failure the guard cannot see" the register header warns about, so the hand-over must include both halves.

### Pattern 7: Docs edits (D-07, D-16, D-08)
- `docs/src/appendix/release-automation.md`: add the dated publish-order paragraph under "## Canonical Publish Order" (line 74; insert before "**Packaging note:**" or after the "No package named for a separate CLI binary…" sentence); add the `release` label step to "## Operator Guide: Cutting a Release" (line 108). Evidence pointer for the A-SC3 reading: `.planning/milestones/v0.11.0-phases/47-v0-11-0-crate-release/47-01-PLAN.md` line 107 (row `A-SC3`) and `47-08-SUMMARY.md`. The reading to record: `release.yml` has no publish-order step of its own; its `Pre-Publish Consistency Gate` (`Pre-Publish Consistency Gate` job, line 406 of release.yml) requires the whole `ci.yml` run on the merge commit to have concluded success, and that run's `License & Dependency Policy` job carries `Check publish order`; `publish-crates` runs in the order enforced by `scripts/publish-crates.sh`'s array.
- `docs/src/appendix/release-checklist.md` line ~66-69 still says the `semver` job "excludes" `paladin-eval` for want of a `0.9.0` baseline; update it (ADR-0048 is a historical record and is not edited).
- `docs/src/contributing/branching-model.md`: the `ci.yml` row in the trigger-policy table (a single-line pipe row; keep it one line, no merged cells) gains the `labeled` activity type and the `release`-label condition. `check-workflow-triggers.sh` compares trigger **names** and the push branch filter only, so adding `types:` does not break it; the prose is the D-00d registration.

### Anti-Patterns to Avoid
- **Reading a skipped required check as a failure to engineer around.** Job-level `if:` skip = success; do not add "always run, skip steps" machinery to `e2e-tests`.
- **Putting the allowlist gate step only in the advisory `semver` job.** It would stop blocking merges.
- **Counting characters with `wc -m`/`${#x}`** in the truncation logic (locale-dependent).
- **Editing frozen sections** (§9/§10 bodies, `CHANGELOG.md` `[0.11.0]`).
- **Writing the four-letter placeholder token in `MIGRATION.md`** (CI fails on any occurrence).
- **Picking a `#[doc(hidden)]` item for the red proof** (undetectable).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Passing a large string to jq | Chunking, temp-file shell tricks | `jq --rawfile NAME FILE` | jq reads the file itself, no argv, handles all bytes/escapes (verified at 140 KB). |
| Counting release-body characters | `wc -m`, `cut -c`, byte math | `python3 len()` or `jq -Rs length` | Code-point counts; matches how the API validates length; locale-proof. |
| Set-equality of register vs allowlist | A second awk copy in YAML | `scripts/check-migration-allowlist.sh` | Already tested (16 assertions) and has the local extension; D-06 makes it the one implementation. |
| Reproducing "what GitHub shows for a skipped job" | Workarounds | Job-level `if:` | Documented behaviour; PR #59 is the precedent. |
| Deciding which lints fire | A hand-maintained list | Run the tool with the table removed (D-02) | The tool is the source of truth; the allowlist is only a review register. |
| Register schema validation | New parsing | `scripts/check-codeql-dismissals.sh` | Already enforces the eleven fields, count drift, staleness, uniqueness, reachability. |

**Key insight:** every defect in this phase came from a gate that could be satisfied vacuously (0 checks, a skipped job that was "required", a duplicated awk step that diverged, an argv limit hit only on a real-size input). The fixes should each add a proof that the gate *can* go red (measured check count, scratch break, over-size fixture generated at test time).

## Runtime State Inventory

This phase renames the `semver` job's display name and retargets a register window, so the usual "what outlives a grep" questions apply:

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | None. No database or datastore keys contain `Semver Checks (vs v0.9.0)`. | None. |
| Live service config | GitHub ruleset `20868126` (live, 44 contexts) is **not** in git: it lists `End-to-End Tests`, not the semver job or `Publish Dry Run`. Rulesets `20868128` (release branches) and `20868099` (tags) were not read; `protect-release-branches.json` has no `context` lines (grep found none). | Maintainer PUT of the committed JSON after edit (Discrepancies 2-3), then read back `length == 3`. Branch-protection list in `docs/src/appendix/branch-protection.md` (count 44) updated in the same change. |
| OS-registered state | None. | None. |
| Secrets/env vars | None renamed. `GH_TOKEN`/`GITHUB_TOKEN` in this sandbox is invalid for writes (see Environment). | None. |
| Build artifacts / installed packages | `Swatinem/rust-cache` key `semver-checks` caches the semver job's builds; `target/semver-checks/` locally holds rustdoc caches and registry baselines (`registry-paladin_web-0_9_0-…` is a stale 0.9.0 baseline cache). | No action; a changed baseline re-fetches. The stale 0.9.0 cache directory is harmless. |
| Doc/text references to the old name | `docs/src/deployment/cicd.md:65`, `docs/src/appendix/release-checklist.md:68`, comments in `ci.yml` 313-399, `.cargo/semver-checks-allowlist.toml` header, `scripts/check-migration-allowlist.sh` header, `Makefile:213`, comment blocks in six `Cargo.toml` files. ADR-0048 and `.project/` texts are historical. | Edit the live docs/comments; leave ADRs and `.project/` alone. |

## Common Pitfalls

### Pitfall 1: A re-pin that reads zero checks again
**What goes wrong:** Omitting `--release-type patch` while the manifest still equals the baseline makes the tool infer a patch change today (fine), but the moment Phase 58 bumps the manifest to `0.11.1` or `0.12.0`, 0.x inference makes it major-equivalent and every lint is skipped again.
**Why it happens:** The tool derives the release type from the version delta unless overridden.
**How to avoid:** Fix the type by flag (D-01) and add the zero-check tripwire (Code Examples 1).
**Warning signs:** a log line `0 checks: 0 pass, 254 skip`; a check count below ~215.

### Pitfall 2: Red proof that cannot go red
**What goes wrong:** Breaking an item that rustdoc JSON does not contain (`#[doc(hidden)]`, or compiled out under default features) leaves the job green and the "proof" worthless.
**How to avoid:** Use the verified break (`paladin-ports` `current_attempt_spend`, line 230 of `crates/paladin-ports/src/output/spend_scope.rs`, `pub fn` → `fn`), or any plain `pub fn` outside `doc(hidden)` modules and outside non-default features. Run it locally first and record the lint name. `paladin-herald` is a trap. Note the scratch break only needs to compile `paladin-ports` alone; other CI jobs may go red on the throwaway branch and that is irrelevant.
**Evidence shape:** a push alone already runs the real `semver` job on that branch (the `push: '**'` trigger); the draft PR adds the `pull_request`-event run D-04 asks for.

### Pitfall 3: The register window runs to end of file
See Pattern 3. An empty `## 11.2` table with no `## 11.3` heading reads every later `Y` row.

### Pitfall 4: D-10/D-11 size collision
See Discrepancy 4. Also: bodies between 125,001 characters and 131,072 bytes are now truncated by design; document it.

### Pitfall 5: Counting bytes instead of characters, and a footer that pushes over the cap
`wc -m` counted bytes here. A footer appended after truncating to exactly the limit makes the body too long. Budget the footer, and test with multi-byte text near the cut (the existing finalize case 8 and create case 8 are the multi-byte/metacharacter precedents).

### Pitfall 6: Finalize overflow
See Pattern 5. A create-time body at the cap plus the artifact block exceeds the cap and `gh release edit --notes-file` fails (a red `Finalize Release Body` job on the tag). v0.11.0 had ~1 KB of headroom only because of the manual condensation to 109,810 characters.

### Pitfall 7: `labeled` fan-out
Any label event re-runs the whole pipeline, including the roughly 63-minute Docker build (measured 3762 s in `branch-protection.md`). Mitigated by `cancel-in-progress` and the rarity of label events; accept and state it.

### Pitfall 8: A committed ruleset JSON that disagrees with the live ruleset
The committed payload sat unapplied for months once (`branch-protection.md` "Applied 2026-08-14"). Editing the JSON in this phase changes the record; the live state changes only on the maintainer PUT. The hand-over must say which: either the PR includes the JSON edit and the PUT command, or the JSON stays unchanged until the maintainer applies; never leave the two silently divergent.

### Pitfall 9: Toolchain/rustdoc-JSON coupling
`cargo-semver-checks` 0.50.0 supports a finite range of rustdoc JSON format versions (the toolchain here emits `format_version` 57 and works). A future `rust-toolchain.toml` bump may require a newer `cargo-semver-checks` pin. [ASSUMED] Out of scope; note in the evidence.

### Pitfall 10: Shellcheck severity
The pre-commit hook runs shellcheck at `--severity=warning`; existing files carry info/style findings. New code should be clean at warning level; do not "fix" unrelated info findings.

## Code Examples

### 1. Semver job step (replace lines ~381-400; verified command shape)
```yaml
      - name: Check semver against the published v0.11.0 baseline
        run: |
          set -euo pipefail
          PACKAGES=(
            paladin-ai paladin-ai-core paladin-ports paladin-battalion paladin-herald
            paladin-llm paladin-memory paladin-storage paladin-notifications
            paladin-content paladin-web paladin-eval
          )
          failed=()
          for pkg in "${PACKAGES[@]}"; do
            echo "::group::cargo semver-checks -- ${pkg} vs 0.11.0 (patch)"
            log="$(mktemp)"
            if ! cargo semver-checks check-release --package "${pkg}" --default-features \
                  --baseline-version 0.11.0 --release-type patch 2>&1 | tee "${log}"; then
              failed+=("${pkg}: lint failure")
            fi
            # Zero-check tripwire (recommended): strip ANSI, require a non-zero count.
            if ! sed 's/\x1b\[[0-9;]*m//g' "${log}" | grep -Eq '[1-9][0-9]* checks:'; then
              failed+=("${pkg}: evaluated zero checks")
            fi
            echo "::endgroup::"
          done
          if [ "${#failed[@]}" -ne 0 ]; then
            printf '::error::%s\n' "${failed[@]}"
            exit 1
          fi
```
Source: command verified locally; the expected per-package line is `223 checks: 223 pass, 31 skip` (failing: `N checks: X pass, Y fail, Z warn, W skip`, exit 100). Job `name:` becomes `Semver Checks (vs v0.11.0)`. The comment block above the job is rewritten (delete the `paladin-eval` exclusion paragraph and every `0.9.0` reference; keep the package-name and `--default-features` rationale).

### 2. Trigger and `if:` edits
```yaml
  pull_request:
    branches: [ main, 'release/**' ]
    types: [ opened, synchronize, reopened, labeled ]
...
  e2e-tests:
    name: End-to-End Tests
    needs: [integration-tests]
    if: >-
      (github.event_name == 'push' && github.ref == 'refs/heads/main') ||
      (github.event_name == 'pull_request' &&
       contains(github.event.pull_request.labels.*.name, 'release'))
...
  publish-dry-run:
    name: Publish Dry Run
    needs: [lint, test]
    # no `if:` -- runs on every push and pull_request (D-05)
```
Remove the `# use this under normal behavior` / `# use this for troubleshooting` comment lines at 2013-2016. The `contains(...labels.*.name, 'release')` form is standard and expression-only (no shell interpolation), so it is also safe from workflow-injection. [CITED: GitHub Actions expressions; ASSUMED actionlint-clean until run in CI]

### 3. Payload builder (D-09)
```bash
jq -n \
    --arg tag_name "${tag}" \
    --arg name "${name}" \
    --rawfile body "${body_file:-/dev/null}" \
    --argjson draft false \
    --argjson prerelease "${prerelease}" \
    '{tag_name: $tag_name, name: $name, body: $body, draft: $draft, prerelease: $prerelease}'
```
Verified: with a 140,035-character body the old form exits 126 with `jq: Argument list too long`; this form exits 0 and `.body | length` is 140035 (characters, not the 140039 bytes). Keep the existing "body file not found" error branch.

### 4. Regression-test sketch (D-10/D-11; follow the file's `assert_*` helpers and stub)
```bash
# Generated at test time in ${SCRATCH}; no committed fixture.
BIG="${DIR_BIG}/body.md"
# multi-line, > 131072 bytes, with a multi-byte tail so chars != bytes
awk 'BEGIN{for(i=0;i<3000;i++) printf "line %d %s\n", i, "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"}' > "${BIG}"
# Case A (D-10): raise the limit so the argv-size proof is real
#   PALADIN_RELEASE_BODY_LIMIT=500000 run create ...; assert create attempted (CREATE count 1),
#   captured_payload `.body|length` > 131072, and body cmp-equal to the file (jq -j .body | cmp - file).
# Case B (D-11): default limit; assert `.body|length` <= 125000, last non-empty line is the footer
#   sentinel, body ends on a line boundary, stderr logs "original ... final ..." sizes.
# Case C: under-cap body -> payload body byte-identical to file (cmp).
# Case D: over-cap with NO newline in budget (single huge line, multi-byte) -> non-empty body <= limit.
# Case E: odd fence count in kept text -> closing fence appended before footer.
# Red-first: run Cases A/B against the UNCHANGED script and record exit 126 / "Argument list too long" / zero CREATE calls.
```
`captured_payload` is already written by the stub from `--input -`. The stub's `echo "${BODY}"` for responses is not affected (response bodies are small).

### 5. Lint-table removal check (D-02) and window retarget
```bash
# after removing the six tables and emptying the allowlist:
grep -n 'cargo-semver-checks.lints' Cargo.toml crates/*/Cargo.toml   # expect no output
grep -c '^\[\[entry\]\]' .cargo/semver-checks-allowlist.toml           # expect 0 (currently 85)
./scripts/check-migration-allowlist.sh                                 # expect empty pair lists, exit 0
```

### 6. Issue #41 evidence comment (D-15) — draft text, all facts verified via the public API
```
Closing as completed: `release.yml` now produces the GitHub Release for every version tag.
Release: https://github.com/DF3NDR/paladin-dev-env/releases/tag/v0.11.0 (published 2026-10-26T20:29:52Z,
10 assets: paladin-{linux,macos}-{amd64,arm64}.tar.gz each with a .sha256, SHA256SUMS, paladin-v0.11.0.cdx.json CycloneDX SBOM).
Produced by the `release.yml` jobs `Create Release`, `Build Binaries`, `Generate SBOM` and `Finalize Release Body`
(run 40396108246 on merge commit 70657919), with crates published by `Publish to crates.io`.
```
Commands (maintainer fallback if the sandbox token is refused; neither was attempted by the researcher):
```bash
gh issue comment 41 --repo DF3NDR/paladin-dev-env --body-file <comment.md>
gh issue close 41 --repo DF3NDR/paladin-dev-env --reason completed
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `--baseline-version 0.9.0`, inferred release type | `--baseline-version 0.11.0 --release-type patch` | this phase | 223 evaluated lints per crate instead of 0. |
| Inline awk in `ci.yml` duplicated in a script | One script, CI calls it | this phase | One implementation, local extension now enforced in CI. |
| Gates firing only on `main` push | `Publish Dry Run` every event; E2E on `release` label | this phase | First run on a tree is before the merge. |
| Manual changelog condensation (156,528 → 109,810 bytes) | Automatic, logged, tested line-boundary truncation | this phase | No hand edit before a tag. |

**Deprecated/outdated:** the 0.9.0 baseline wording everywhere (job name, comments, `cicd.md`, `release-checklist.md`); the Phase 44 D-27 `--release-type minor` diagnostic recipe (superseded by D-01); "crate-wide allow until 0.11.0 is the baseline" notes.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | GitHub's release-body validation counts characters (code points) with a 125,000 maximum. Corroborated only by community issue reports (intuit/auto #2418, an objectstack PR), no official docs page reachable here; Phase 47 itself recorded it as an unmeasured research figure. | Pattern 5, Pitfalls 4-6 | If counted in UTF-16 units or bytes, a body at exactly 125,000 characters could be rejected. Mitigation: reserve headroom (a few thousand characters) rather than targeting the exact cap. |
| A2 | A job-level `if:` skip reports success to required-check evaluation while a workflow-level filter skip stays Pending. Source is a search summary of GitHub's docs plus PR #59 precedent (merged with required `End-to-End Tests` skipped); `docs.github.com` was not fetchable. | Discrepancy 3 | If wrong for some event type, label-less release PRs could be blocked; low, because the precedent already merged. |
| A3 | The open CodeQL alerts "on the release PR heads" are retrievable as open alerts on `main` (or `refs/pull/59/head`) after the PR #59 merge. | Pattern 6 | The maintainer pulls from the other ref; no design impact. |
| A4 | The sandbox token can neither read code-scanning alerts (403 seen) nor reliably write issues; a write was not attempted. | Environment, Open Question 3 | If writes work, the executor can close #41 in-phase; plan for both. |
| A5 | `github.head_ref` (bare branch) differs from `github.ref` (`refs/heads/…`), so push and PR runs do not share a concurrency group. | Discrepancy 6 | Only affects run duplication, not correctness. |
| A6 | `cargo publish --workspace --dry-run` completes its verify phase at the already-published `0.11.0` tree (only the warnings and first packaging were observed before a 90 s timeout). | Measured Results, CIREL-04 | If it errors later, `Publish Dry Run` would be red on every PR until the version moves; the first PR run will show it, and the plan should treat a first-run failure as a finding to triage, not a gate to weaken. |
| A7 | cargo-semver-checks output in CI may contain ANSI codes (`CARGO_TERM_COLOR=always`); the tripwire strips them defensively. | Code Examples 1 | None if stripped; a false positive otherwise. |
| A8 | A future toolchain bump may require a newer `cargo-semver-checks` pin. | Pitfall 9 | Gate error rather than silent pass; low. |

## Open Questions

1. **Is the semver check meant to become a required check?** (Discrepancy 2)
   - Known: it is advisory in both the live ruleset and `cicd.md`; D-08 assumes required.
   - Unclear: whether the maintainer wants it blocking.
   - Recommendation: make it required (additive ruleset PUT plus `cicd.md` row); ask the user to confirm in plan review since the phase goal says the gates should "actually check something".

2. **Remove all six crate-wide allow tables (28 lines) or only the two named manifests?** (Discrepancy 1)
   - Known: the script forces all six if the allowlist is emptied; all six are measured redundant.
   - Recommendation: remove all six; treat as covered by D-02 "every redundant crate-wide allow". No user input strictly needed, but the plan should call it out.

3. **Who closes #41 and applies the CodeQL dismissals/ruleset PUT?** The sandbox `GH_TOKEN` is invalid and code-scanning reads return 403. Plan both an "executor tries, falls back" path and a maintainer hand-over checklist with exact commands (Phase 47 D-03 posture).

4. **What exact alerts are the "24"?** Only the maintainer can list them. The CIREL-06 rows cannot be authored before that list exists; make the list intake the first task of that plan and put it on a `checkpoint:human-action`.

5. **Footer link/anchor and limit variable name** are explicitly discretionary; recommended defaults are in Pattern 5.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` / rustc | all measurements | ✓ | 1.97.1 (repo pin) | — |
| `cargo-semver-checks` | D-01/D-02/D-04 local measurement | ✓ | 0.50.0 (same as CI pin) | — |
| crates.io (index + `.crate` download) | semver baselines, publish dry run | ✓ | HTTP 200 via the proxy | — |
| `jq` | CIREL-05 | ✓ | 1.7 | — |
| `shellcheck` | script edits | ✓ | 0.9.0 | CI runs the pinned pre-commit version |
| `python3` + PyYAML + `tomllib` | register guards, truncation helper | ✓ | 3.13.16 / 6.0.1 | — |
| `make` | `make check-gates`, `make test-shell-guards` | ✓ | present | — |
| `actionlint` | D-00d, D-12 | ✗ | — | CI `Workflow Lint` (required) runs 1.7.12. A download of the release binary was attempted; executing it was blocked by the sandbox, so it was **not** run. Do not rely on a local actionlint pass. |
| `docker` (daemon) + `cargo-llvm-cov` | folded todo: local `make coverage` | ✗ (docker client only; `cargo llvm-cov` not installed) | — | Record the folded-todo task as not reproducible in this sandbox; the CI `Coverage` figure and a maintainer-run local figure are the comparison. A mismatch is recorded, not fixed. |
| `gh` authenticated for writes | D-13, D-15, ruleset PUT | ✗ | `GH_TOKEN` invalid | Public reads work (issue, release, rulesets). Code-scanning alerts: 403 `Resource not accessible by integration`. Fall back to maintainer hand-over with exact commands. |
| `docs.github.com` | official-doc citation | ✗ (DNS blocked) | — | Search summaries only; flagged in the Assumptions Log. If official citations are wanted, the host must be allowed in the session's network settings. |
| Git push / draft PR from the sandbox | D-04 scratch PR | not tested | — | If refused, the maintainer pushes the scratch branch (a push alone triggers the real `semver` job; the PR adds the `pull_request` event). |

**Missing dependencies with no fallback:** none block research; the maintainer-gated items (alert list, ruleset PUT, possibly the issue close and the scratch push) are hand-offs by design.

**Disk note for executors:** the sandbox volume has only a few GB free and it fluctuates; each semver-checks run adds ~100-200 MB under `target/semver-checks/local-*`. Delete those directories between runs (they are regenerated), and prefer editing manifests in place and restoring with `git checkout` over creating extra worktrees. This session hit a 0-byte-free condition once; it is an execution risk, not a design one.

## Validation Architecture

> `workflow.nyquist_validation` is `true` in `.planning/config.json`.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Bash regression harnesses (`tests/scripts/*_test.sh`), discovered by glob in `make test-shell-guards`; each builds fixtures in a `mktemp -d` scratch dir with a `trap` cleanup, accumulates `FAILED`/`ASSERTIONS`, uses a stubbed `gh` selected via `GH_BIN`, and ends with a real-tree mutation check. Plus the offline gate scripts via `make check-gates`, plus CI itself and `cargo semver-checks` for the semver proof. |
| Config file | none; `Makefile` target `test-shell-guards` (line ~254) |
| Quick run command | `./tests/scripts/<name>_test.sh` for the touched script, e.g. `./tests/scripts/create-or-reuse-release_test.sh` |
| Full suite command | `make test-shell-guards && make check-gates` |

Baseline captured this session (all green before any edit): create-or-reuse-release 21 assertions, finalize-release-body 84, check-migration-allowlist 16, check-workflow-triggers 11, check-codeql-dismissals 11; `check-migration-allowlist.sh` reports 15 pairs = 15 pairs and the local-extension line OK.

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| CIREL-01 | Each of 12 packages reports a non-zero check count vs 0.11.0 at patch; job named for baseline | integration (tool) + CI | `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.11.0 --release-type patch` (expect `223 checks: 223 pass, 31 skip`); CI job log | tool ✓; CI step is the change |
| CIREL-01 | A public-API break turns the job red (proves lints evaluate) | integration, scratch branch | local: privatise `current_attempt_spend` and rerun (expect exit 100, `function_missing`); CI: scratch branch/draft PR job URL | evidence file ❌ Wave 0 (`52-SEMVER-RED-EVIDENCE.md`) |
| CIREL-01 | Zero-check tripwire fails when count is 0 (if adopted) | shell harness | add a small fixture-driven assertion or extract the parse into a testable function; otherwise prove by inspection + CI | ❌ optional Wave 0 |
| CIREL-02 | No crate-wide allow tables remain; job green | static + tool | `grep -n 'cargo-semver-checks.lints' Cargo.toml crates/*/Cargo.toml` → empty; per-package command above | ✓ (grep) |
| CIREL-02/03 | Allowlist (0 entries) set-equal to empty §11.2; §9.2/§10.2 `Y` rows not read; `TBD` marker absent | shell harness + gate | `./tests/scripts/check-migration-allowlist_test.sh` (retargeted 10a-10d to §11.2/§11.3; new cases: Y rows under §10.2/§9.2 are ignored; Y row under §11.2 with no entry fires; Y row under §11.1 not read; window terminates at §11.3) then `./scripts/check-migration-allowlist.sh`; `grep -c TBD MIGRATION.md` → 0 | test ✓ (edit); new cases ❌ |
| CIREL-03 | Script runs in CI on every PR | CI + workflow lint | `grep -n 'check-migration-allowlist' .github/workflows/ci.yml` shows the call in a required job; CI `Workflow Lint` green | edit |
| CIREL-04 | `Publish Dry Run` runs on PR; E2E runs on PR with `release` label and is skipped without | CI observation | scratch PR without and with the label; record job conclusions; `check-workflow-triggers.sh` and its test stay green | evidence ❌ |
| CIREL-05 | >131,072-byte body reaches the (stubbed) API unmangled; red before fix | shell harness | `./tests/scripts/create-or-reuse-release_test.sh` (new case A with raised limit) | ✓ (extend) |
| CIREL-05 (D-11) | Over-cap body truncated at a line boundary with footer and logged sizes; under-cap byte-identical; no-newline and multibyte edge cases; finalize composed body fits cap and is idempotent | shell harness | `./tests/scripts/create-or-reuse-release_test.sh` and `./tests/scripts/finalize-release-body_test.sh` | ✓ (extend) |
| CIREL-06 | Register has a full row per triaged alert, count matches, nothing stale | gate | `make check-codeql-dismissals`; `./tests/scripts/check-codeql-dismissals_test.sh` | ✓; rows depend on maintainer-supplied list |
| CIREL-07 | #41 closed with Release reference (or restated) | manual-only (platform write) | `gh api repos/DF3NDR/paladin-dev-env/issues/41 -q .state` → `closed`; comment text saved in `52-CI-EVIDENCE.md` | manual |
| CIREL-09 | Publish-order paragraph present with evidence pointer | static | `grep -n 'Check publish order' docs/src/appendix/release-automation.md` and the A-SC3 pointer | ✓ (doc edit) |
| all | Whole gate set still green | aggregate | `make check-gates && make test-shell-guards` ; `shellcheck --severity=warning` on touched scripts; CI `Workflow Lint` | ✓ |

### Sampling Rate
- **Per task commit:** the touched script's own `_test.sh` + `shellcheck --severity=warning <file>` + `make check-gates` (offline, seconds).
- **Per wave merge:** `make test-shell-guards && make check-gates` plus the 12-package semver loop (about 5 minutes with warm caches) when `ci.yml` or any `Cargo.toml` changed.
- **Phase gate:** a full CI run on the phase PR (all required checks green, semver log lines recorded, `Workflow Lint` green), the red-proof evidence file, and the maintainer-act checklist signed off, before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `52-SEMVER-RED-EVIDENCE.md` and `52-CI-EVIDENCE.md` (allow measurement, per-package check lines, CodeQL triage table, issue-close record, first-PR-run findings for E2E/dry run).
- [ ] New/retargeted cases in `tests/scripts/check-migration-allowlist_test.sh` (window moves to §11.2/§11.3; reads-§11.2-only assertions).
- [ ] New cases in `tests/scripts/create-or-reuse-release_test.sh` and `finalize-release-body_test.sh` (argv-size proof with raised limit, truncation, byte-identity, edge cases, idempotency), written red first against the unmodified script.
- [ ] Framework install: none (harness exists).

## Security Domain

> `security_enforcement` is not set to false in `.planning/config.json`, so this applies.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (no auth surface added) | — |
| V3 Session Management | no | — |
| V4 Access Control | yes (CI authority) | Required-check ruleset edits are a maintainer-only PUT; `GITHUB_TOKEN` permissions in `release.yml` stay `contents: write` only where already granted; do not widen. |
| V5 Input Validation | yes | Release body is data: `jq --rawfile` keeps it structural; limit env var validated as an integer; label name only used inside `contains()` expression, never interpolated into `run:`. |
| V6 Cryptography | no (no new crypto; `X-Paladin-Signature` is webhook-side, untouched) | — |
| V10/V14 Config & supply chain | yes | `cargo-semver-checks` stays pinned (`@0.50.0`); no new third-party action; workflow-suppression and trigger-policy guards stay green. |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Command/argument injection via changelog text reaching a shell or argv | Tampering | Body only ever travels as a file path → `jq --rawfile` → stdin to `gh api --input -`; no interpolation (existing house rule, keep the EOF/backtick/`$(…)` test case 8). |
| Workflow expression injection through label or PR-controlled strings | Tampering / Elevation | Use `contains(github.event.pull_request.labels.*.name, 'release')` only in `if:`; never `${{ … }}` inside `run:` (CR-01 convention). |
| Token-bearing request redirect | Information disclosure | Unchanged: `create-or-reuse-release.sh` makes the credentialed `gh api` call; do not add `curl -L`; the manual credential review in `security.instructions.md` still governs this file, and the size guard must not log the body or any header, only sizes. |
| Silent truncation hiding content | Repudiation | Truncation is logged (original/final sizes), marked by a sentinel footer, and pointed at `CHANGELOG.md`; never silent. |
| A gate that cannot fail | Spoofing (assurance) | Red-first tests and the scratch-break evidence; zero-check tripwire. |
| Cost abuse via `labeled` re-runs | Denial of service | Single-maintainer repo; `cancel-in-progress` on non-main; accepted, documented. |
| CodeQL posture drift | Repudiation | Advisory-only posture unchanged; the register is the governance surface; do not reintroduce Snyk or claim CodeQL as merge-gating. |

## Project Constraints (from CLAUDE.md)

- Medieval Military ubiquitous language in code/docs/comments (Paladin, Battalion, Garrison, Arsenal, Citadel, Herald, Quest, Commissary, Cadence). `Entity<T>` is deliberately generic; "node" means only a WarGraph/Campaign vertex (ADR-0060). Script and doc prose in this phase should keep to those terms where they appear.
- TDD red-green-refactor; 82 % workspace line-coverage floor (ADR-0006) via `cargo llvm-cov --fail-under-lines` in the `coverage` job; all public APIs need doc tests (no Rust is added here).
- Dependencies flow inward only (not engaged: no Rust changes).
- Before committing a parent task: `cargo test` → `cargo fmt --check` → `cargo clippy` → `make api-surface` (an intentional surface change needs `make api-surface-update` plus a CHANGELOG entry). Then conventional-commit messages. **Stop after each major task and wait for go-ahead.**
- Security: `make security` (cargo-audit + cargo-deny) and `cargo clippy -- -D warnings`, plus the manual credential-handling review in `security.instructions.md`. CodeQL is advisory-only (CodeQL 2.26.3, disqualified 2026-08-25); Snyk was removed and must not be reintroduced or recorded as a blocker.
- Avoid `unwrap()`/`expect()`/`panic!` in library code (n/a: no Rust added except an unmerged scratch break); prefer borrowing over cloning.
- `.github/instructions/security.instructions.md` credential rules apply to `create-or-reuse-release.sh` (token-bearing request): redact before truncating if a response body is ever embedded in an error; no key in logs; no redirect-following on a credentialed client. The size guard logs sizes only.
- Task-list protocol: mark sub-tasks complete, run `cargo test` / `cargo fmt --check` / `cargo clippy` before committing a parent task, one-line `git commit` with `-m` flags, remove debug prints.
- Git commit attribution line from the session reminder applies to commits made by executors.

## Sources

### Primary (HIGH confidence, measured or read in this session)
- Local runs of `cargo semver-checks` 0.50.0 against crates.io `0.11.0` for all twelve packages, with and without allow tables, plus a scratch break (restored; tree clean).
- Local run of the unmodified `_cor_build_payload` at 140 KB (exit 126) and `jq --rawfile` (exit 0, length 140035); `wc -m` byte behaviour.
- `cargo publish --workspace --dry-run` early output (warnings, packaging).
- Repository files read: `.github/workflows/ci.yml`, `release.yml`, `.github/rulesets/protect-main-branch.json`, `scripts/{check-migration-allowlist,create-or-reuse-release,finalize-release-body,check-codeql-dismissals,check-workflow-triggers}.sh`, `tests/scripts/{create-or-reuse-release,check-migration-allowlist}_test.sh` (+ harness function map for finalize), `.cargo/semver-checks-allowlist.toml`, six `Cargo.toml` manifests, `MIGRATION.md` structure, `CODEQL-DISMISSALS.md`, `docs/src/appendix/{release-automation,release-checklist,branch-protection}.md`, `docs/src/deployment/cicd.md`, `docs/src/contributing/branching-model.md`, `Makefile`, `.pre-commit-config.yaml`, Phase 47 `47-CI-EVIDENCE.md` / `47-01-PLAN.md` / `47-08-SUMMARY.md`.
- Public GitHub API reads (unauthenticated via the proxy): issue #41, `releases/tags/v0.11.0`, ruleset `20868126`, repository metadata.
- crates.io sparse index for all twelve crates and `cargo-semver-checks`.

### Secondary (MEDIUM confidence)
- Search summaries of GitHub docs: required-status-check behaviour for skipped jobs vs filtered workflows (docs.github.com "Troubleshooting required status checks"); `pull_request` default activity types and `types:` overriding them (docs.github.com "Trigger a workflow" and third-party guides).
- Community reports of the 125,000-character release-body cap: github.com/intuit/auto issue 2418; an objectstack-ai PR that truncates to the API limit.

### Tertiary (LOW confidence)
- None used as a basis for a recommendation; unverifiable items are listed in the Assumptions Log.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — no new packages; versions confirmed locally.
- Architecture / mechanisms: HIGH for the semver re-pin, allow removal, argv fix and register window (all executed); MEDIUM for GitHub platform behaviours not readable from official docs here.
- Pitfalls: HIGH for those reproduced (argv failure, byte-vs-char counting, doc(hidden) blind spot, window runs to EOF by reading the awk); MEDIUM for `labeled` fan-out cost and concurrency-group behaviour.
- Carry-overs (CodeQL, issue #41): MEDIUM — the work is understood, but inputs (alert list) and write access are outside this sandbox.

**Research date:** 2026-10-28
**Valid until:** 2026-11-27 (stable tooling; recheck if `cargo-semver-checks`, the pinned Rust toolchain, or the ruleset changes first, and note that Phase 58's lockstep bump is the next event that can move the semver baseline)
