# Phase 58: Crate Release - Research

**Researched:** 2026-11-05
**Domain:** Cargo-workspace lockstep patch release (`v0.11.1`) through a PR-merge + annotated-tag + `release.yml` + crates.io Trusted Publishing pipeline, with a maintainer-gated irreversible leg
**Confidence:** HIGH on the bump/changelog/gate mechanics (reproduced in a scratch clone), HIGH on the registry/CI state (read live), MEDIUM on the SDK-smoke root cause (log-read, not reproducible without a Docker daemon)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Carried forward (locked by Phases 37 / 37.1 / 47 / 52 / 53 / 54 — not re-asked)**

- **D-00a:** Only the maintainer ticks a sign-off box — never an agent, under any mode (Phase 47 D-00a).
- **D-00b:** The maintainer alone performs the irreversible acts: merging the release PR, pushing the `v*` tag, applying the `release` label, any `cargo yank`, any key rotation or history purge. The agent hands over exact commands and stops (Phase 47 D-00b).
- **D-00c:** Tag-cut mechanism: the release PR merged to `main` with a **merge commit** (not squash, not rebase), then an **annotated tag pushed directly to that merge commit** after `ci.yml` is green on it. `make release` is not the path (its trailing push is blocked by the "Protect main branch" ruleset); its component steps are (Phase 47 D-00c, Operator Guide).
- **D-00d:** Findings are recorded, never fixed silently; amend-at-source with dated additions; `WINDOWS.md` rows are never deleted (Phase 47 D-00d).
- **D-00e:** The 82 % workspace line-coverage floor (ADR-0006) is measured by CI's `Coverage` job only; `cargo-llvm-cov` is absent here and the sandbox has no Docker (Phase 47 D-00e).
- **D-00f:** Shipped tree outranks any document. The publishable-crate list is whatever `cargo metadata --no-deps` says (`publish != false`), never hard-coded (Phase 47 D-00f).
- **D-00g:** Publishing is via crates.io Trusted Publishing; no standing registry credential exists and nothing in this phase may introduce or request one (Phase 47 D-00g).
- **D-00h:** Registry proof is a sparse-index (`index.crates.io`, mandatory `User-Agent`) read per publishable crate — crate, version, checksum, `yanked` — recorded as a table. The workflow's own log is not proof on its own (Phase 47 D-00h).
- **D-00i:** Waiting on the maintainer is a hard stop with a resume file naming the exact resume condition — never a parked watcher or a long-lived `gh run watch` (Phase 47 D-00i).
- **D-00j:** Any red gate from the start of the re-seal onward — locally or in PR CI — is recorded (command, SHA, failing output) and the agent halts at a blocking checkpoint and fixes **nothing**. The one exception is a pure infrastructure flake, re-run **exactly once** with both run IDs and the justifying log line recorded (Phase 47 D-00j).
- **D-00k:** `release.yml` cannot be dry-run-dispatched before a ref literally named `v0.11.1` exists. No dispatch rehearsal, no rc tag, no shadow tag (Phase 47 D-00k).
- **D-00l:** Post-tag commits reach the protected `main` through a `chore/58-close` branch cut from the tagged `main` and a small docs-only PR the maintainer merges; the tagged commit stays exactly what was released; `/gsd-complete-milestone` is not run by any plan (Phase 47 D-00l).
- **D-00m:** The phase stays open across the merge: a **build wave** (pre-bump fixes, bump set, anchors, MIGRATION subsection, audit register), a **pre-merge wave** (re-seal, PR, evidence, sign-off), the maintainer's **merge + tag checkpoint**, and a **post-tag wave** (registry verification, smoke, records, hand-offs). Verified only on what is observably true, never by promise (Phase 47 D-00m).
- **D-00n:** `CHANGELOG.md` heading dates equal the tag date (Phase 46 D-13). `MIGRATION.md` §9 and §10 are frozen records; only §11 is written (Phase 52 D-00d).
- **D-00o:** `v0.11.1` is a patch and every Phase 52-57 change is additive, decided in Phase 53 on measured evidence (`53-VERSION-DECISION.md`). That file itself says the pre-tag audit must re-run `scripts/check-semver.sh` on its own head rather than rely on the record (see D-10).
- **D-00p:** The version gate goes red at the bump by design: `scripts/check-version-strings.sh` holds every copyable string to the workspace manifest version, so the bump commit must carry the string edits for the gate to be green on the release commit (Phase 54 carry-forward, 54-FIRST-EVIDENCE "Carry-forward to Phase 58"; 54 CR-01: `make release` runs no version-string gate before the tag).
- **D-00q:** `scripts/finalize-crate-changelogs.sh` leaves a changelog with no `[Unreleased]` anchor and no target-version section untouched and records a named failure; `check-release-consistency.sh` independently requires a `## [X.Y.Z]` heading in all twelve publishable changelogs (Phase 54 carry-forward rule 1).
- **D-00r:** The Phase 52 release gates are on: `Publish Dry Run` and `Semver Checks (vs v0.11.0)` run on every PR; `End-to-End Tests` runs under the maintainer's `release` label; `scripts/release-body-fit.sh` and the create/finalize guards see their first live run at this tag — read that release's body size and footer at the tag (Phase 52 D-05, D-11, D-12; STATE "Release-body size guard has never run live").
- **D-00s:** Partial-publish recovery is the maintainer's option A: a patch release through the same pipeline, orphaned versions yanked only after the patch is registry-verified, one yank-register row per crate; the agent diagnoses read-only per `docs/src/appendix/release-recovery.md` §1-§2 and hard-stops (Phase 47 D-18, Phase 37.1 D-02).

**Branch & PR shape**

- **D-01:** **One PR.** `claude/laughing-dirac-e0h2ax` (Phases 54-57, 359 commits ahead of `main`) is the release PR; the build wave and the bump commit set land as its last commits; it is retitled `release: v0.11.1 — Dogfood Cleanup` (wording at the planner's discretion) with a curated body; its single merge commit carries the whole remainder of the milestone and is the tag target. The Phase 47 precedent (PR #59). Rejected: merging Phases 54-57 first and cutting a small bump PR off `main`; rejected: an audit run between two such merges. — **Reversibility:** one-way — the merge commit on the protected `main` becomes the tag target and cannot be rewritten once the tag points at it.
- **D-02:** **`main` is frozen** at `8b23cb4c5` until the release PR merges; no other merge before the tag, so the PR's diff is exactly "everything since the PR #62 merge". If `main` moves anyway, the branch merges `main` in (merge commit, never rebase), the re-seal restarts from the top on the new head, and the maintainer re-confirms the sign-off against the new SHA (Phase 47 D-02).
- **D-03:** **The `release` label is applied once, after the bump push.** The agent opens or retitles the PR only after the bump commit set is on `origin`, then hands the maintainer the label step as one checkpoint so the full `ci.yml` re-run (about 63 minutes of Docker build) happens on the candidate final head and `End-to-End Tests` reports before the irreversible merge. Rejected: labelling early; rejected: skipping the label for a patch (breaks Phase 52 D-05).
- **D-04:** **Sign-off home and order as Phase 47 D-10..D-14.** The re-seal table and the single tick box live in a new `58-ACCEPTANCE-AUDIT.md` beside `58-CI-EVIDENCE.md`; the box reads `- [ ] **The \`v0.11.1\` tag may be cut**`; the signed table is the eight 37.1 rows re-read at `0.11.1` (MIGRATION §11.2 ↔ allowlist set-equality via `make check-migration-allowlist`; `v0_9_config_boot`; OpenAPI golden; `make check-semver` vs `0.11.0` at patch, twelve crates, now a real check (Phase 52); MSRV; `make publish-dry-run`; `[0.11.1]` completeness via `make check-gates` plus the hard-assertion greps; `make check-publish-order`), each with command, SHA, result and CI twin named or "none" stated. `make api-surface`, `make check-consumer-surface`, `make check-version-strings` (inside `check-gates`) and the CI `Coverage` job are CI-evidence entries. Order is structurally enforced: local re-seal → push → PR CI green including `Coverage` → evidence appended → the maintainer ticks the box and the tick commit is on `origin` at the PR head → CI green on that SHA → the agent verifies the tick on `origin` → only then the merge + tag hand-off is written. No tick on `origin`, no hand-off. — **Reversibility:** one-way — the merge and tag that follow trigger a crates.io publish.
- **D-05:** **The red `Generated SDK Clients (Python + TypeScript) smoke` job is diagnosed first, in the build wave.** A plan reads the job log (run `41927940508`, job `126917997570`, and the latest run on the branch) and whether the context is required by the ruleset. A CI or script defect gets a minimal fix in the release PR; anything that would touch `src/` or `crates/*/src/` is recorded as a finding in `58-CI-EVIDENCE.md` and `58-AUDIT-DISPOSITIONS.md` and the maintainer decides (fix, accept a red non-required job at the merge, or hold the tag) at a blocking checkpoint. Rejected: fixing it whatever it takes; rejected: recording only.

**Pre-tag audit dispositions (SHIP-10)**

- **D-06:** **One disposition register, three verdicts.** A new `58-AUDIT-DISPOSITIONS.md` in the phase directory lists every carried item with exactly one of `fixed` (pointer to the commit or evidence), `waived` (reason), or `re-homed` (owner, date, pointer to the todo). The milestone audit's `tech_debt` list may cite only rows of that register, so a `passed` status is backed by a visible disposition per row. The carried set, to be completed by the plan from the sources named in canonical refs: 57-REVIEW CR-01, WR-01, WR-02 and IN-01…05; the two orphaned Phase 55 deferrals (News API key in the URL `D-52-03-01`; `${…}` placeholder in the Treasurer webhook `secret`, 54 WR-04); the red SDK-smoke job; the tracked `sk-proj-…` literal; OSV alerts 23-25 (`D-52-03-02`); the stale CodeQL anchors (`D-52-03-03`); FIXES-04 `RunStatusValue::Cancelled` (55 deferred-items); 55-08 `EmbeddingPort` rustdoc; the 56 deferred-items entry; 54 WR-01/WR-04/WR-05/IN-02; the Phase 47 carried release-tooling findings not closed by Phase 52; the seven pending todos (three folded, four acknowledged); the `Generated SDK Clients` and Codecov maintainer-only items from STATE; and anything else the plan's sweep of `.planning/phases/5[2-7]-*/` (deferred-items, REVIEW, VERIFICATION `deferred`, SUMMARY `deviations`) and `.planning/todos/pending/` surfaces. The manual `scripts/check-review-dispositions.sh` run (not in `check-gates` by design) and `make check-findings-log` are part of the audit sweep and recorded with their result lines.
- **D-07:** **Only WR-02 is fixed in `v0.11.1`; CR-01 and WR-01 are re-homed.** The `check-review-dispositions.sh` acceptance of non-evidence as a `closed` pointer is gate tooling, not crate code, and lands in the build wave with its fixture. CR-01 (worker nack-and-redeliver loses the run outcome when only the second write fails) and WR-01 (an empty `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` wipes a YAML secret) are behaviour changes in published crates and go to `v0.12.0` with the maintainer as owner. Rejected: all three as pre-bump commits; rejected: re-homing WR-02 too.
- **D-08:** **The `sk-proj-…` literal is scrubbed in the release PR; everything else about it is the maintainer's.** A docs-only build-wave commit replaces the literal in `.project/Milestone_3-Completion/Post-Epic_24-cleanup/LIVE_API_TESTS_FIX.md` with a redacted placeholder, and the 56-08 key-shape scanner reads clean on the tracked tree. Whether the key was ever live, its rotation, and any history rewrite are maintainer checkpoints recorded as rows in `58-AUDIT-DISPOSITIONS.md` (owner maintainer, date before the tag for rotation; the purge decision may be re-homed). The literal itself is never quoted in any planning document or commit message. Rejected: leaving the file and recording only; rejected: deleting the whole file.
- **D-09:** **Re-home form: one pending todo per item, owner named, date fixed.** Each re-homed item becomes (or already is) a `.planning/todos/pending/` file with an `owner` (the maintainer, or `v0.12.0 Phase 1` for code items), `recheck_by` set to the next milestone's first planning day, and a pointer back to its source finding; the register row cites the todo path. `WINDOWS.md` stays at `open_count: 0` and gains no row. The four acknowledged todos (OpenAI Responses API, setup-check output, treasurer price digits, DeepSeek reasoning hint) keep their files and get the dated owner line if they lack one. Rejected: new `WINDOWS.md` rows closed as waived; rejected: the register as the only home.

**Consumer statement & web3sec (SHIP-09)**

- **D-10:** **A `### Library consumers: v0.11.0 → v0.11.1` subsection under MIGRATION §11.8**, mirroring §10.8's. One paragraph on measured evidence: no Rust signature, type or serialized schema changed; §11.2 has no row and `.cargo/semver-checks-allowlist.toml` has no entry; `scripts/check-semver.sh` on the release head reads `0 failure(s)` for all twelve packages at `--release-type patch` (head SHA and date named); the only consumer-visible note is `get_job` unrouted-but-not-deprecated, already in §11.7. It is written in the bump commit set (D-15, commit 3) and re-read on the candidate final head; if the final head differs from the SHA it names, the SHA is corrected in a docs-only commit before the tick. Rejected: one sentence in the §11 intro only; rejected: both places.
- **D-11:** **A red `check-semver` on the release head is a hard stop before any bump.** The pre-tag audit re-runs `scripts/check-semver.sh` on its own head (D-00o). A red line stops the phase; the agent reports the lint and the offending item and proposes an additive reshape that removes the break without changing behaviour (Phase 53 PD-05-1 pattern) first. If none exists, the maintainer chooses at a blocking checkpoint: register the one break in §11.2 plus the allowlist and still ship `0.11.1` (SHIP-09's "records the one deliberate break" branch) or move every version string to `0.12.0` per the roadmap note. Rejected: registering any break and shipping `0.11.1` regardless; rejected: an automatic flip to `v0.12.0`. — **Reversibility:** one-way — a published version number is permanent.
- **D-12:** **A short outbound `58-HANDOFF-web3sec.md`, no return contract.** Written in the post-tag wave: tag, merge SHA, release run id, the twelve-crate registry table, pointers to MIGRATION §11.8's consumer subsection and to the operator steps that touch a consumer's deployment (`M-B-36` key rotation, `M-B-38` environment variables), and the Cadence muster pin (folded todo). No return fields are requested and no todo stays pending on a reply. Rejected: the full Phase 47 hand-off with a return contract; rejected: no hand-off.
- **D-13:** **A keyless scratch-crate smoke on the published `0.11.1` is part of the post-tag proof.** After the registry table, the agent runs `cargo add paladin-ai@0.11.1` in a scratch crate outside the workspace (no `[patch]`, no path dependencies, no workspace inheritance), builds the README quick example, and records resolve and compile results in `58-CI-EVIDENCE.md`. No provider key is used; a failure is recorded and put to the maintainer, never fixed in this phase. Rejected: registry table only; rejected: the full keyed first-run walk.

**Bump commit set & changelogs (SHIP-08)**

- **D-14:** **Three bump commits, the last three on the branch.** Commit 1 (`chore(release)`): `cargo release version 0.11.1 --execute --no-confirm --workspace` on a clean tree, then the 52 user-facing strings the gate lists (42 class-1 Cargo pins in docs, the README `Current version` line, 8 `k8s/` labels and image tags, `openapi.json` `info.version` via `make openapi`), so `make check-gates` including `check-version-strings` is green on that commit, plus a grep-zero check for `0.11.0` across manifests, pins and the gate's file classes (historical CHANGELOG/MIGRATION/ADR text excluded; `crates/doc-examples` hand-touched if the tool skips it). Commit 2 (`chore(release)`): `make finalize-crate-changelogs VERSION=0.11.1`, root included, followed by `make check-changelogs` and `make check-release-consistency RELEASE_TAG=v0.11.1`. Commit 3 (`docs(release)`): the MIGRATION §11.8 consumer subsection (D-10). The expected `openapi.json` diff is the version string only; any other change means a route moved and is recorded as a finding and the agent stops. Rejected: one consolidated commit; rejected: four commits (manifests alone read red on the version gate).
- **D-15:** **Crate changelog entries are derived from the git log, never a stock line.** For `paladin-battalion`, `paladin-eval` and `paladin-herald` (no anchor today) a build-wave plan reads `git log v0.11.0..HEAD -- crates/<crate>/` (unshallowed or via GitHub where the shallow clone cannot reach `v0.11.0`), adds the `## [Unreleased]` anchor and the real entries (Phase 55's rustdoc-allow removals and lint fixes count), and uses a "no functional change" line only when that log is genuinely empty. The other eight crate changelogs are sanity-checked the same way so no `[Unreleased]` section contradicts the root (47-REVIEW WR-01). Rejected: bare anchors stamped empty; rejected: copying root lines.
- **D-16:** **Stamp date = the day the bump is pushed; redated before the tick if the tag slips.** At the sign-off checkpoint the maintainer names the tag day; if it differs, one `docs(release)` commit corrects the twelve headings before the box is ticked and re-seal row 7 is re-read on that head (D-00n). Rejected: a blocking date question before stamping; rejected: stamping after the merge (the tagged tree would lack the `[0.11.1]` heading `release.yml` and `check-release-consistency` require).
- **D-17:** **All non-bump work lands before the bump, in the build wave.** The Cadence muster pin (`test`), the `gpt-4` literal sweep (`docs`), the devcontainer `cargo-llvm-cov` install (`chore`), the WR-02 checker fix (`fix`), the `sk-proj-…` scrub (`docs`), the three changelog anchors (`docs`), the audit register and re-home todos (`docs`), and a CI-side SDK-smoke fix if diagnosis finds one (`ci`) each land as their own conventional commit, so the bump set is the last three commits and the re-seal runs once on the candidate final head. A red gate after the re-seal starts fixes nothing (D-00j); this ordering keeps every fix inside the window where fixing is allowed. Rejected: bump first; rejected: a separate pre-release PR (breaks D-02).

### Claude's Discretion

The maintainer did not select these; the defaults below were stated and accepted. Research may overturn a default only with evidence, and must say so in RESEARCH.md. (**Research overturns none of them** — see "Discretion defaults: confirmed" below.)

- **Order of operations inside commit 1** — default: run `cargo release version` on a clean tree first, then edit the strings, then `make openapi`, then stage and commit. This sidesteps the untested dirty-tree behaviour of `cargo-release` (54-FIRST-EVIDENCE rule 3); if the tool refuses anyway, record what it printed.
- **Root `CHANGELOG.md` `[Unreleased]` curation** — default: the section (about 60 lines today) is stamped as written by Phases 52-57; a hand-curation only if the stamp leaves it unreadable, with the release-body size read at the tag (D-00r). No condensation step exists in this release.
- **PR title and body wording**, within the required contents (re-seal summary, pointers to `CHANGELOG.md` `[0.11.1]`, MIGRATION §11, `58-CI-EVIDENCE.md`, `58-ACCEPTANCE-AUDIT.md`, `58-AUDIT-DISPOSITIONS.md`, the twelve-crate list from `cargo metadata`), written via `gh api` REST with the "hand the exact command over" fallback for a read-only token.
- **Re-seal command order and output capture**, provided every signed row has a command, a SHA and a result, and `58-CI-EVIDENCE.md` follows the `47-CI-EVIDENCE.md` house form (local sweep table, CI-run table, registry table, registry probe command with the `User-Agent` header).
- **Hand-off checkpoint texts** for the label, the merge + tag and any recovery — reuse Phase 47's with `0.11.1`; the tag command is `git tag -a v0.11.1 <merge-sha> -m "v0.11.1"` then `git push origin v0.11.1`.
- **`58-ACCEPTANCE-AUDIT.md` preamble** — default: two or three sentences on what `v0.11.1` is, drawn from MIGRATION §11's opening paragraph, no new claims.
- **Plan count and wave layout**, provided D-00m's four-wave split around the D-04 tick gate and the maintainer's merge + tag checkpoint holds, no plan both executes the bump and starts the re-seal in the same task, and the SDK-smoke diagnosis and the audit sweep run before the bump.
- **The `v0.11.1` GitHub Release object** — created by `release.yml` from the `[0.11.1]` section through the Phase 52 body builder; its size and footer are read and recorded; a `gh release edit` is the maintainer's.
- **`chore/58-close` contents** — as Phase 47 D-17: post-merge and release-run CI evidence, the registry table and smoke, the MILESTONES.md `v0.11.1` entry (tag, merge SHA, release run id, crate count, "registry-verified", Trusted Publishing, release-body size), the SHIP-08/09/10 ticks through `gsd-tools` handlers in the post-tag wave only after the registry table shows all twelve crates at `0.11.1` `yanked = false`, `58-HANDOFF-web3sec.md`, and the "ready to close" hand-off listing the `/gsd-audit-milestone` / `/gsd-complete-milestone v0.11.1` prerequisites for the maintainer.
- **Devcontainer `cargo-llvm-cov` install shape** (folded todo) — version `0.8.7` as the todo says, in the image definition only; no coverage run is claimed from this sandbox.
- **`gpt-4` sweep model literal** — the replacement model name follows the todo's own recommendation and the pricing table's current rows; the plan reads the todo first.

### Deferred Ideas (OUT OF SCOPE)

- **57-REVIEW CR-01** (worker nack-and-redeliver loses the run outcome on a second-write failure) and **WR-01** (empty `APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET` wipes a YAML secret) — `v0.12.0`, maintainer-owned, as dated todos (D-07, D-09).
- **News API key in the URL** (`D-52-03-01`) and the **Treasurer webhook `secret` placeholder not refused** (54 WR-04) — the two orphaned Phase 55 deferrals, re-homed with owner and date (D-09); the first is a `v0.12.0` security-hardening candidate.
- **FIXES-04 `RunStatusValue::Cancelled`** — the next minor, as Phase 55 D-05 recorded.
- **OSV alerts 23-25 without a governing exception row** (`D-52-03-02`) — maintainer decision, dated.
- **A history purge of the `sk-proj-…` literal** — maintainer decision (D-08).
- **Making `release.yml` dry-runnable against a bare SHA before a tag exists** — carried from Phases 37 / 37.1 / 47.
- **A standing post-release consumer smoke with a real key** — D-13 ships the keyless half; the keyed walk stays with the maintainer's own runs.
- **The `get_job` `#[deprecated]` attribute** — the next minor, per MIGRATION §11.7.
- **Codecov token and `fail_ci_if_error`**, the seven §11 sign-off boxes in `.project/v0.10.0/09-program-acceptance-audit.md` — maintainer-only items observed at the v0.11.0 close, unchanged.
- Reviewed todos not folded (acknowledged by the audit with owner and date): OpenAI Responses API (`2026-10-20-openai-responses-api-for-pro-tier.md`), setup-check section order (`2026-11-02-setup-check-first-run-output.md`), unquoted treasurer price digits (`2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md`), DeepSeek reasoning-budget hint (`2026-11-04-reasoning-budget-hint-for-deepseek-empty-completion.md`).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SHIP-08 | `v0.11.1` is tagged on `main` and ships through `release.yml` end to end with no manual step; all 12 publishable crates are on crates.io at `0.11.1` | Bump + changelog + gate sequence reproduced end to end in a scratch clone (§"Verified release mechanics"); `release.yml` job graph and the whole-run `ci.yml` conclusion clause read (§"Release pipeline facts"); two merge/tag blockers found that CONTEXT does not mention (required `pre-commit` context red; SDK-smoke red blocks the publish gate); sparse-index + `trustpub_data` probes confirmed reachable (§"Registry proof") |
| SHIP-09 | `MIGRATION.md` states a `0.11.0` consumer takes `0.11.1` without adaptation, or records the one deliberate break | Phase 53 decision read; `check-semver` re-run on the current head: 12 packages × 223 checks, 0 failures; the bumped-manifest case re-measured for one crate; §11.1 holds 25 behavioural rows (M-B-23..47) that the D-10 wording must not hide (§"SHIP-09 wording") |
| SHIP-10 | The milestone audit reads `passed`, not `tech_debt`: every carried item fixed, waived with a reason, or re-homed with an owner and a date | Audit tooling/format read (`audit-milestone.md` §6, v0.11.0 audit precedent); scoping trap for the pre-tag audit (SHIP-08..10 are themselves pending); carried-item source map; `check-review-dispositions.sh`, `make check-findings-log`, `windows status` all run green today (§"Audit inputs") |
</phase_requirements>

## Summary

Phase 58 is an operations phase, not a code phase: every mechanic it needs already exists and was exercised by Phase 47 (`v0.11.0`). I reproduced the whole mechanical chain in a throwaway clone (`cargo release version 0.11.1 --execute --no-confirm --workspace` on a clean tree, the gate-driven string edits, the three missing changelog anchors, `make finalize-crate-changelogs`, `make check-changelogs`, `make check-release-consistency RELEASE_TAG=v0.11.1`) and it works exactly as CONTEXT D-14/D-15 describe: 13 manifests plus `Cargo.lock` move, the gate then lists exactly 52 strings (42 + 1 + 8 + 1), and after editing those lines it reads `OK: 128 file(s) scanned ... '0.11.1'`. The decisions in CONTEXT are sound; research found no reason to overturn a discretion default.

Research did find **four things CONTEXT does not know**, all of which can stop the merge or the tag if they surface after the re-seal starts (D-00j) instead of in the build wave: (1) the required status context `pre-commit run --all-files` is **red on the branch today** because of one trailing blank line in `.planning/phases/57-register-hygiene-close-out/57-VALIDATION.md` (reproduced locally; no other hook fails); (2) the red SDK-smoke job is diagnosable from the public job log (`tsc: not found` inside the generated TypeScript package's `prepare` build, coincident with `ubuntu-latest` rolling from `ubuntu-24.04` to `ubuntu-26.04`) and, while it is **not** a required ruleset context, `release.yml`'s `check-release-consistency` clause 3 reads the **whole-run** conclusion of `ci.yml` on the tagged SHA, so D-05's "accept a red non-required job at the merge" option would leave the tag unpublishable; (3) a one-off failure of the unit test `treasury_spend_prints_one_row_per_currency` in the required `Build & Test (cli)` context on `3b0d095a2` (passed on the next head) is a test-level flake, not an infrastructure flake, so D-00j's single re-run exception does not obviously cover it; (4) a pre-tag milestone audit run through the stock workflow would return `gaps_found` because SHIP-08/09/10 are still pending, so the audit must be explicitly scoped to Phases 52-57 (v0.11.0 precedent: first audit `gaps_found`, then a post-release re-audit).

Everything else the planner needs is measured below: the semver gate is green on the current head and on a bumped manifest, the sparse index and the crates.io `trustpub_data` API are reachable from the sandbox (so the registry proof and the Trusted Publishing proof are agent-runnable), `gh` write capability is untested but reads work, there is no Docker daemon and ~5.9 GB of free disk.

**Primary recommendation:** Add three build-wave items CONTEXT lacks (fix the `57-VALIDATION.md` end-of-file, diagnose and fix the SDK-smoke script, characterize the `treasury_spend` flake), push the pre-bump head early (no label) to get a real CI read on all three before the bump, then follow CONTEXT's bump → re-seal → tick → merge/tag → registry-read order unchanged. Scope the pre-tag audit explicitly to Phases 52-57.

## Project Constraints (from CLAUDE.md)

- **TDD / coverage:** Red-Green-Refactor; 82 % workspace line-coverage floor (ADR-0006), enforced only by CI's `Coverage` job (`cargo llvm-cov --fail-under-lines`); all public APIs need doc tests. This phase changes no public API.
- **Before committing a parent task:** `cargo test` → `cargo fmt --check` → `cargo clippy` → `make api-surface` (an intentional surface change needs `make api-surface-update` plus a CHANGELOG entry), then a conventional-commit message. Stop after each major task and wait for go-ahead. The Phase 58 bump must leave `.project/current-exports.txt` untouched (it contains no version string, measured: 0 matches for `0.11.0`).
- **Security:** `make security` (cargo-audit + cargo-deny) and `cargo clippy -- -D warnings` on new/modified code, plus the manual credential-handling review in `.github/instructions/security.instructions.md`. CodeQL is advisory-only; Snyk must not be reintroduced and no phase may record itself blocked on a Snyk scan.
- **Dependencies flow inward only; Medieval Military terminology** in code, docs and comments (Cadence, Herald, Garrison, Citadel, ...). `Entity<T>` is generic by design; "node" means only a WarGraph/Campaign vertex (ADR-0060). New docs and examples written in this phase (the `gpt-4` sweep prose, MIGRATION text) must keep this vocabulary.
- **No `unwrap()`/`expect()`/`panic!` in library code**; irrelevant to this phase except that the Cadence muster pin and the WR-02 checker fix must not introduce them in non-test code.
- **Conventional commits**; commit/PR attribution lines per the harness reminder. Project workflow rule: `pre-commit` hooks (`end-of-file-fixer`, `trailing-whitespace`, `gitleaks`, `shellcheck`, `cargo fmt/clippy`, `check-api-surface`, `doc-check`) run on every commit and in the required CI context `pre-commit run --all-files`; every `.planning/` file this phase writes must therefore end in exactly one newline and carry no stray trailing whitespace.
- **Never quote the `sk-proj-…` literal** anywhere (CONTEXT D-08, specifics); commit messages name the file and line only.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Lockstep version bump (manifests, `Cargo.lock`) | Local workspace tooling (`cargo release version`) | Gate (`check-version-strings`) | `release.toml` is configured for `version` only; no commit, tag or push happens (measured: the tool left a modified, uncommitted tree) |
| User-facing version strings (README, `docs/src`, `k8s/`, `openapi.json`) | Repo files edited by hand/sed | `scripts/check-version-strings.sh` in `make check-gates` and CI | The gate lists `file:line` for every stale string; `openapi.json` is generated from `CARGO_PKG_VERSION` and guarded by the `openapi_matches_committed_baseline` test |
| Changelog stamping | `scripts/finalize-crate-changelogs.sh` (via `make finalize-crate-changelogs`) | `check-changelogs`, `check-release-consistency` | Needs a `## [Unreleased]` anchor per file; one invocation stamps all twelve |
| Consumer-compat statement | `MIGRATION.md` §11.8 | `check-semver` (CI `semver` job) + allowlist gate | The statement is prose; its evidence is the semver run on the release head |
| Pre-merge gating | GitHub required status checks on the PR (46 contexts) | Local re-seal | Live ruleset read: includes `End-to-End Tests`, `Publish Dry Run`, `Semver Checks (vs v0.11.0)`, `pre-commit run --all-files`; does **not** include the SDK-smoke job |
| Tag cut | Maintainer (annotated tag pushed to the merge commit) | `release.yml` `verify-tag-source` (tag SHA must be an ancestor of `origin/main`) | Irreversible; D-00b |
| Publish | `release.yml` `publish-crates` under the `crates-io` environment (OIDC Trusted Publishing) | `check-release-consistency` (tag ↔ manifests ↔ changelogs ↔ `ci.yml` conclusion on the SHA) | `scripts/publish-crates.sh` owns the order, registry pre-check and outcome table |
| Registry proof | crates.io sparse index + crates.io web API (`trustpub_data`) | `cargo add` scratch-crate smoke | Independent of the workflow log (D-00h) |
| Milestone audit | `/gsd-audit-milestone` document + `58-AUDIT-DISPOSITIONS.md` register | `check-review-dispositions.sh`, `check-findings-log`, `windows status` | `passed` is a human-judged status over the register; no script computes it |

## Standard Stack

### Core (tooling this phase uses; nothing new is added to the shipped dependency graph)

| Tool | Version (measured) | Purpose | Why Standard |
|------|--------------------|---------|--------------|
| `cargo-release` | 1.1.7 (installed, `--locked`, pinned by plan 47-01) | `cargo release version <v> --execute --no-confirm --workspace` lockstep bump | `release.toml` is written for it; reproduced: bumps 13 manifests + `Cargo.lock`, no commit/tag/push |
| `cargo-semver-checks` | installed (version printed per-run by the script) | `scripts/check-semver.sh` | Same script as CI's `semver` job; pins `--baseline-version 0.11.0 --release-type patch` |
| `scripts/check-version-strings.sh` | repo | 52-string gate | Offline, python3 ≥ 3.11 (`tomllib`); sandbox has 3.13 |
| `scripts/finalize-crate-changelogs.sh` | repo | Stamp `## [0.11.1] - <date>` in all 12 changelogs | Enumerates from `cargo metadata`, not a glob |
| `scripts/check-release-consistency.sh` | repo | Tag ↔ manifest ↔ changelog ↔ CI-conclusion agreement | Runs inside `release.yml` before any publish |
| `rust-lang/crates-io-auth-action@v1` | pinned in `release.yml` | OIDC token mint for Trusted Publishing | No standing credential (D-00g) |
| `pre-commit` | present at `/root/.local/bin/pre-commit`, hook repos cached | Reproduce the required `pre-commit run --all-files` context locally | Works offline here (reproduced the failing hook) |

### Supporting

| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| `cargo-llvm-cov` | pin **0.8.7** (crates.io latest is 0.9.1) | Devcontainer install only (folded todo) | Keep the pin equal to `ci.yml:1489` `tool: cargo-llvm-cov@0.8.7`; do not "upgrade" it in this phase |
| `curl` + `python3` | system | Sparse-index and `crates.io/api/v1` probes | Post-tag registry/TP proof |
| `gsd-tools` (`requirements mark-complete`, `windows status`, `audit-open`) | `/workspace/.claude/gsd-core/bin/gsd-tools.cjs` | SHIP tick, WINDOWS read, open-todo listing | Post-tag wave only for the ticks (D-00a: only the maintainer ticks the *sign-off box*; REQUIREMENTS ticks go through the handler) |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `cargo release version` | `sed` over 13 manifests | Rejected by CONTEXT; the tool also updates `Cargo.lock` and intra-workspace pins consistently (measured) |
| One `sed` loop over the gate's own `file:line` output | Hand edits | Measured: replacing `0.11.0` → `0.11.1` on exactly the 52 reported lines turns the gate green with no collateral (the 52 are the only occurrences the gate reads) |

**Installation:** none for the release itself (tools already present). Devcontainer todo: `RUN cargo install --locked --version 0.8.7 cargo-llvm-cov` in `.devcontainer/Dockerfile.dev` (the todo names that file; `.devcontainer/Dockerfile` also exists and carries its own `vhs` pin — the planner must read both before choosing where the line goes).

**Version verification:** `cargo release --version` → `cargo-release 1.1.7`; `rustc 1.97.1`; `python3 3.13.16`; `rust-version = "1.88"` workspace MSRV. [VERIFIED: local execution]

## Package Legitimacy Audit

This phase adds no dependency to any published crate. The only installable it touches is `cargo-llvm-cov` in the devcontainer image.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| cargo-llvm-cov | crates.io | since 2021-01 | ~146k/wk | github.com/taiki-e/cargo-llvm-cov | OK | Approved (pin 0.8.7, matches CI) |
| cargo-release | crates.io | since 2016-03 | ~1.4k/wk | github.com/crate-ci/cargo-release | OK | Approved (already installed, 1.1.7) |
| cargo-semver-checks | crates.io | since 2022-07 | ~13k/wk | github.com/obi1kenobi/cargo-semver-checks | OK | Approved (already installed) |

Verdicts from `gsd-tools query package-legitimacy check --ecosystem crates ...`. [VERIFIED: gsd-tools seam]
**Packages removed due to [SLOP] verdict:** none. **Packages flagged [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```
 maintainer's branch claude/laughing-dirac-e0h2ax (HEAD b2705d7f0, 359+ commits over main 8b23cb4c5)
        |
        |  BUILD WAVE (own conventional commits, all before the bump; fixing allowed)
        |   pre-commit EOF fix --> SDK-smoke diagnosis/fix --> treasury_spend flake characterised
        |   WR-02 checker fix+fixtures --> sk-proj scrub --> gpt-4 sweep(+gate) --> Cadence muster pin
        |   devcontainer llvm-cov --> 3 changelog anchors+entries --> 58-AUDIT-DISPOSITIONS + todos
        |   [early push, NO label]  --> CI read of pre-commit / SDK smoke / flake  (informational)
        |   check-semver on own head (HARD STOP if red, D-11) --------------------------------+
        v                                                                                     |
 BUMP SET (last 3 commits)                                                                    |
   c1 chore(release): cargo release version + 52 strings + Cargo.lock + openapi.json          |
   c2 chore(release): make finalize-crate-changelogs VERSION=0.11.1 (12 files)                |
   c3 docs(release):  MIGRATION 11.8 "Library consumers: v0.11.0 -> v0.11.1"                  |
        |                                                                                     |
        v   PRE-MERGE WAVE (red gate => record + STOP, fix nothing, D-00j)                    |
 local re-seal (8 rows, command+SHA+result) -> push -> retitle PR (gh REST or hand-over)      |
        |                                                                                     |
        v  [MAINTAINER] apply `release` label -> full ci.yml (~63 min) incl. End-to-End Tests  |
 PR CI green on final head (46 required contexts; Coverage; Semver non-zero count) <----------+
        |  evidence appended -> 58-ACCEPTANCE-AUDIT.md -> [MAINTAINER] ticks box -> tick commit on origin
        |  -> CI green on tick SHA -> agent verifies tick on origin -> merge+tag hand-off written
        v
 [MAINTAINER] merge commit to main (frozen) -> push-to-main ci.yml run must conclude `success` (whole run)
        |
        v  [MAINTAINER] git tag -a v0.11.1 <merge-sha> -m "v0.11.1"; git push origin v0.11.1
 release.yml (runs-on ubuntu-latest = Ubuntu 26.04, first time for this workflow)
   verify-tag-source -> { test, create-release, check-release-consistency }
   create-release: extract [0.11.1] body -> release-body-fit (first live run) -> GitHub Release
   publish-crates (needs test + create-release + consistency; env crates-io; OIDC) -> 12 crates in CRATES order
   build-docker / build-binaries / sbom -> finalize-release-body (not gating crates.io)
        |
        v  POST-TAG WAVE (agent, read-only)
 sparse-index table (12 rows) + crates.io API trustpub_data (provider/repository/run_id/sha)
 + GitHub Release assets/body size/footer + keyless scratch-crate `cargo add paladin-ai@0.11.1` smoke
        |
        v  chore/58-close branch from tagged main -> docs-only PR [MAINTAINER merges]
 MILESTONES.md entry, SHIP-08/09/10 ticks via gsd-tools, web3sec hand-off, ready-to-close hand-off
```

### Recommended Project Structure (phase artifacts)

```
.planning/phases/58-crate-release/
├── 58-CONTEXT.md / 58-RESEARCH.md / 58-DISCUSSION-LOG.md      # inputs
├── 58-NN-PLAN.md / 58-NN-SUMMARY.md                            # plans
├── 58-AUDIT-DISPOSITIONS.md     # register: id | source | verdict | pointer/reason/owner+date | todo path
├── 58-CI-EVIDENCE.md            # house form of 47-CI-EVIDENCE.md: local sweep, CI-run table, registry table, probe command
├── 58-ACCEPTANCE-AUDIT.md       # 8-row re-seal + the single tick box (maintainer-ticked only)
├── 58-HANDOFF-web3sec.md        # short outbound hand-off, no return contract
└── 58-READY-TO-CLOSE.md         # /gsd-audit-milestone + /gsd-complete-milestone prerequisites
.planning/v0.11.1-MILESTONE-AUDIT.md   # written by /gsd-audit-milestone (workflow path), archived later by complete-milestone
```

### Suggested plan / wave layout (planner's discretion; satisfies D-00m)

| Wave | Plans (each an atomic conventional commit unless noted) | Gate |
|------|---------------------------------------------------------|------|
| 1 build | (a) pre-commit EOF fix for `57-VALIDATION.md` + local `pre-commit run --all-files` green; (b) SDK-smoke diagnosis + minimal `scripts/sdk-smoke/run.sh` fix; (c) `treasury_spend_*` flake characterisation (read-only loop; maintainer checkpoint if a `src/` test edit would be needed); (d) WR-02 checker fix + near-miss fixtures; (e) `sk-proj` scrub; (f) `gpt-4` sweep (+ `check-doc-models` gate, planner's call); (g) Cadence muster pin; (h) devcontainer llvm-cov; (i) 3 changelog anchors + real entries + sanity-check of 8 others | each commit passes `pre-commit` hooks |
| 1 build (end) | (j) audit sweep → `58-AUDIT-DISPOSITIONS.md` + re-home todos with `recheck_by`; (k) `scripts/check-semver.sh` on own head (D-11 hard stop) and early push of the pre-bump head (no label) | semver `0 failure(s)`; early CI read |
| 2 bump | (l) c1, c2, c3 — three commits, one plan or three, but **no re-seal in the same task** | `make check-gates` green on c1 |
| 3 pre-merge | (m) local re-seal rows; (n) push + retitle PR; **checkpoint: maintainer applies `release` label**; (o) PR CI evidence; (p) `58-ACCEPTANCE-AUDIT.md`; (q) scoped milestone audit; **checkpoint: maintainer ticks box**; (r) verify tick on `origin`, write merge+tag hand-off | structural D-04 order |
| 4 merge/tag | **checkpoint: maintainer merges and tags**; agent stops (resume file names the condition, D-00i) | none by agent |
| 5 post-tag | (s) registry + `trustpub_data` table; (t) release run/body/footer/assets read; (u) keyless smoke; (v) `chore/58-close` content: MILESTONES entry, SHIP ticks, hand-offs | registry table 12/12 |

### Pattern 1: gate-driven string edit (measured, replaces "remember every string")

**What:** Let the gate enumerate the work, edit exactly those lines, re-run the gate.
**When to use:** commit 1, after `cargo release version`.
**Example:**
```bash
# Source: reproduced in a scratch clone of HEAD on 2026-11-05 (52 findings -> OK)
cargo release version 0.11.1 --execute --no-confirm --workspace      # clean tree; leaves it modified, no commit
./scripts/check-version-strings.sh 2>&1 \
  | grep -E "found '0\.11\.0'" \
  | sed -E 's/^([^:]+):([0-9]+): .*/\1 \2/' \
  | while read -r f l; do sed -i "${l}s/0\.11\.0/0.11.1/g" "$f"; done
./scripts/check-version-strings.sh    # -> OK: 128 file(s) scanned ... '0.11.1' (class 1: 42; 2: 1; 3: 8; 4: 1; exempt: 1)
```
Note the sandbox edit covers `crates/paladin-web/openapi.json:14` too; the plan should still run `make openapi` and confirm the diff is that one line (D-14).

### Pattern 2: changelog stamping (measured)

```bash
# Source: reproduced 2026-11-05 after adding `## [Unreleased]` to battalion/eval/herald
make finalize-crate-changelogs VERSION=0.11.1   # -> OK: 12 publishable package(s) ... all "finalized"
make check-changelogs                            # -> 11 publishable crate(s) checked, all have a CHANGELOG.md
make check-release-consistency RELEASE_TAG=v0.11.1   # -> OK: 12 ... match tag version '0.11.1'
```
The script inserts the dated heading *immediately after* `## [Unreleased]`, so the existing Unreleased body ends up under `## [0.11.1] - <date>` and an empty `[Unreleased]` anchor remains above it. The Makefile target does **not** pass `--date`; the date is `date +%Y-%m-%d` of the run. To stamp a different day call `./scripts/finalize-crate-changelogs.sh --version 0.11.1 --date YYYY-MM-DD` directly, or redate the twelve headings by a `docs(release)` commit (D-16).

### Anti-Patterns to Avoid

- **Blanket `0.11.0` → `0.11.1` replacement.** `0.11.0` legitimately remains in: `scripts/check-semver.sh:86` (`SEMVER_BASELINE_VERSION`), `tests/scripts/check-semver_test.sh:67`, `Makefile:271-277`, `.github/workflows/ci.yml:369-437` (job name `Semver Checks (vs v0.11.0)`), `.github/rulesets/protect-main-branch.json:159` (the required context **name**), `docs/src/appendix/{branch-protection,release-checklist,release-automation}.md`, `docs/src/deployment/cicd.md`, `.cargo/semver-checks-allowlist.toml` comments, `release.toml`, `.devcontainer/Dockerfile*` (`vhs=0.11.0`, an unrelated tool), `docs/DEMOS.md` (`vhs 0.11.0`), and frozen history (CHANGELOG, MIGRATION §9/§10, ADR text, `docs/src/api-reference/upgrading.md` v0.10.1→v0.11.0 content). Changing the semver baseline or the job name would break the baseline comparison or orphan the required status context. The D-14 "grep-zero" check must be scoped to manifests, pins and the gate's four classes only. [VERIFIED: repo grep]
- **Running `make release`.** Its trailing push is blocked by the ruleset and it runs no version-string gate (54 CR-01); use its component steps only.
- **Treating the gate as complete.** It does not read prose: `docs/src/api-reference/migration-guide.md:4` still says "leads with the current **v0.11.0** release", and `docs/src/api-reference/upgrading.md` has no `v0.11.0 → v0.11.1` entry. Success criterion 1 says "because the gate demands it", but these two stale statements are invisible to the gate; fix them in the build wave (docs commit) or record them as dispositions.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Lockstep bump of 13 manifests + lockfile | `sed` over `Cargo.toml` | `cargo release version 0.11.1 --execute --no-confirm --workspace` | Also rewrites intra-workspace dependency pins and `Cargo.lock` (measured) |
| Changelog stamping | per-file `perl`/`sed` | `make finalize-crate-changelogs` | Enumerates from `cargo metadata`; idempotent; named failures |
| Release-body size handling | a condensation step | `scripts/release-body-fit.sh` (already wired into `create-or-reuse-release.sh` and `finalize-release-body.sh`) | Root `[Unreleased]` section is 13,488 chars vs a 125,000 limit [VERIFIED: `awk` over `CHANGELOG.md` lines 8-67]; the guard will not truncate |
| Registry/TP proof | scraping the workflow log | sparse index + `GET https://crates.io/api/v1/crates/<crate>/0.11.1` (`version.trustpub_data`) | Independent of the workflow; includes the run id and commit `sha` |
| Required-context check | assuming from the ruleset JSON | `GET /repos/DF3NDR/paladin-dev-env/rules/branches/main` (unauthenticated read works) | The committed JSON and the live ruleset agreed (46 contexts) when read on 2026-11-05 |
| Review-disposition validation | reading prose | `scripts/check-review-dispositions.sh` and `make check-findings-log` | Both run green today |
| Semver evidence from CI | download logs | check-run **annotations** of the `Semver Checks (vs v0.11.0)` job (`GET /repos/.../check-runs/{job_id}/annotations`) | The script emits one `::notice` carrying every package's `N checks:` line; readable without log-download credentials (verified on job 126988790441) |

**Key insight:** every release-day primitive already has a script or gate with a named-failure mode. The phase's risk is sequencing and the handful of un-gated edges (prose, required-context hygiene, runner drift), not missing tooling.

## Runtime State Inventory

A version bump is not a rename, but it leaves registered state behind; each category answered explicitly.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None — `0.11.1` changes no persisted schema (MIGRATION §11.4 reads "No persistence or schema change"); verified by the semver/allowlist evidence only for Rust types, not for stored JSON, but Phase 53's decision and §11.4 are the record | none |
| Live service config | crates.io: all 12 crates currently resolve at `0.11.0` as latest, `yanked=false`, **no `0.11.1` row** (sparse index read 2026-11-05); GitHub `crates-io` Environment and crates.io Trusted Publishing config exist and published `0.11.0` for all 12 (47-CI-EVIDENCE `trustpub_data` table) | none beyond the post-tag read |
| OS-registered state | None | none |
| Secrets/env vars | No credential is added; the publish job mints an OIDC token per run. The `sk-proj-…` literal in a tracked `.project/` markdown is *tracked secret-shaped content*, not runtime state | scrub (D-08); rotation is the maintainer's |
| Build artifacts / installed packages | `k8s/` manifests reference `paladin-server:0.11.0` (two images, moved by the gate); ghcr.io image tags are produced by `release.yml` `build-docker`; a local `target/` of 17 GB exists | none beyond the string edits; mind disk (5.9 GB free) |

## Common Pitfalls

### Pitfall 1: A required status context is already red (`pre-commit run --all-files`)
**What goes wrong:** the PR cannot merge; discovered only after the label/CI cycle.
**Why it happens:** `end-of-file-fixer` rewrites `.planning/phases/57-register-hygiene-close-out/57-VALIDATION.md` (file ends `\n\n`). Reproduced locally with `pre-commit run end-of-file-fixer --all-files`; every other hook (`check-toml/yaml`, `trailing-whitespace`, `gitleaks`, `shellcheck`) passes on the tree. CI runs `41946243555` (3b0d095a2) and `41949135339` (b2705d7f0) both fail the same hook. The live ruleset lists `pre-commit run --all-files` as required.
**How to avoid:** a first build-wave `docs`/`chore` commit that removes the extra newline, plus running `pre-commit run --all-files` (it takes ~4-5 min incl. clippy) before every push. Every new `.planning/` document in this phase must satisfy the same hook.
**Warning signs:** the `pre-commit` workflow shows `Failed - files were modified by this hook`.

### Pitfall 2: A red *non-required* `ci.yml` job still blocks the publish
**What goes wrong:** the maintainer merges with the SDK-smoke job red (D-05's "accept" option), tags, and `release.yml`'s `Pre-Publish Consistency Gate` fails closed with a CI-conclusion error; `publish-crates` never starts.
**Why it happens:** `check-release-consistency.sh` clause 3 takes the deciding `ci.yml` run for the tagged SHA (latest `created_at`, then `id`) and requires `conclusion == "success"` — a whole-run verdict, with no per-job exemption (documented in the script header).
**How to avoid:** treat "SDK smoke green on the push-to-`main` run" as a tag precondition, not an option; the D-05 checkpoint's "accept a red job" choice must be removed or reworded to "accept and re-run until green / hold the tag". A job re-run on the same run id updates the conclusion; confirm via `GET .../actions/workflows/ci.yml/runs?head_sha=<sha>` before the tag.
**Warning signs:** `ci.yml` run on the merge SHA shows `failure` with only the SDK job red.

### Pitfall 3: SDK-smoke root cause is runner drift, not Paladin code
**What goes wrong:** the planner assumes `src/` is involved and escalates to the maintainer.
**Evidence:** log of job 126917997570 (and again 126988790544 on b2705d7f0): Python half passes (`SDK smoke (Python): all checks passed`); TypeScript half fails at `npm install --no-save ../../target/sdk/typescript` → generated package's `prepare` → `npm run build` → `sh: 1: tsc: not found` (exit 127). In the green run for `v0.11.0` (job 121733645764, 2026-10-26) the same step printed `added 1 package ... in 4s` (the generated client built). The only changed variable visible in the logs: runner image `ubuntu-24.04` (20261012.334.1) → `ubuntu-26.04` (20261025.179.1); a CI annotation confirms "The ubuntu-latest label will migrate to Ubuntu 26 beginning October 19, 2026". The generator image is pinned (`openapi-generator-cli:v7.25.0`).
**How to avoid / minimal fix (CI-side, in `scripts/sdk-smoke/run.sh`):** build the generated client explicitly (`(cd ../../target/sdk/typescript && npm install --ignore-scripts && npm run build)` or install its dev dependencies) and install it into the smoke package with `--ignore-scripts`; or pin the npm used. This is a hypothesis about *why* the lifecycle behaviour changed [ASSUMED — the exact npm behaviour difference on 26.04 was not reproduced: a local npm 10.9.4 test of a directory dependency with a `prepare` script did not run `prepare`]. **It cannot be verified in the sandbox** (no Docker daemon, so the generator cannot run); the only proof is a CI run, so push the fix in the build wave and read the job. The fix stays inside `scripts/` (within D-05's "CI or script defect" branch); nothing touches `src/`.
**Warning signs:** `tsc: not found` / exit 127 in the TypeScript half.

### Pitfall 4: The first release run happens on a new runner image
**What goes wrong:** `release.yml` jobs (`test`, `check-release-consistency`, `publish-crates`, `build-docker`, `sbom`) all use `ubuntu-latest` and have never run on Ubuntu 26.04; `build-binaries` also uses `macos-latest`. The latest branch CI run shows 28 jobs green on the new image, so the risk is moderate, but a surprise lands *after* the tag.
**How to avoid:** none possible before the tag (D-00k forbids rehearsal); instead pre-read the release-recovery runbook, keep the agent's role to read-only diagnosis (D-00s), and note that crates.io publication depends only on `test`, `create-release`, `check-release-consistency` (not on docker/binaries/sbom).

### Pitfall 5: A test-level flake in a required context
**What goes wrong:** `Build & Test (cli)` (required) failed once on `3b0d095a2`: `application::cli::commands::treasury::tests::treasury_spend_prints_one_row_per_currency` panicked at `src/application/cli/commands/treasury.rs:496` (`assert!(rendered.contains("EUR"))`); the same job passed on `b2705d7f0` with no relevant change. Two settlements are written for a fresh SQLite ledger and read back through the default look-back window computed from `store_now()`; a sub-second ordering between the settlement timestamps and the window end is the plausible mechanism [ASSUMED — not reproduced].
**Why it matters:** D-00j permits one re-run only for a "pure infrastructure flake"; a flaky unit test is not infrastructure, and the test lives under `src/` (outside this phase's boundary).
**How to avoid:** characterise in the build wave (repeat-run the single test; record the rate); if reproducible, record a finding + dispositions row and put the choice (test-only fix under `src/application/cli/...` vs accept-and-rerun) to the maintainer at a blocking checkpoint **before** the re-seal. The planner should decide the rule for the re-seal now, rather than invent it in the middle of a red run.

### Pitfall 6: A pre-tag milestone audit that cannot read `passed`
**What goes wrong:** run unscoped, `/gsd-audit-milestone` marks SHIP-08/09/10 `unsatisfied`/`partial` (unchecked in REQUIREMENTS, Phase 58 has no VERIFICATION) and the workflow's FAIL rule forces `gaps_found`.
**Evidence:** `audit-milestone.md` §5d/§6: "Any `unsatisfied` requirement MUST force `gaps_found`". Precedent: the v0.11.0 audit was first `gaps_found` ("SHIP-07 unsatisfied, Phase 47 not started"), then superseded by a post-release re-audit (`status: tech_debt`).
**How to avoid:** the audit's frontmatter states its scope (Phases 52-57; requirements 49 satisfied of 50 in scope, FIXES-04 under a `re_homed_requirements:` key mirroring v0.11.0's `superseded_requirements:`), lists SHIP-08/09/10 as "owned by Phase 58, proven by the post-tag registry read, outside this audit's scope", and sets `tech_debt: []` with every re-homed item cited as a register row. If the maintainer wants the tool's literal verdict, a second post-tag run (like v0.11.0's) is the fallback. The scoping choice is a decision for the maintainer/orchestrator — see Open Questions.

### Pitfall 7: `check-review-dispositions.sh` tightened so far it rejects real rows
**What goes wrong:** WR-02's fix (require a commit-shaped token with both a letter and a digit, drop `ADR/UTF/SHA/HTTP` from `REQ_RE`, require known extensions for path pointers) can turn the 51 real archived dispositions red, which would flip the audit sweep from `0 failure(s)` to failures.
**How to avoid:** write the near-miss fixtures first (RED), implement, then run the checker against the real archive (`checked 51 finding(s) across 13 review file(s); 0 failure(s)` today) and `make test-shell-guards` (22 fixtures + new ones). Keep a commit id that is all digits (rare, ~4 % of 7-hex ids) acceptable only when adjacent to "commit"/"sha".

### Pitfall 8: Scrubbing only the line CONTEXT names
**What goes wrong:** the `LIVE_API_TESTS_FIX.md` file holds key-shaped content on **two** lines (49: a partial `sk-proj-…` with a tail fragment; 53: the full literal). Scrubbing line 53 only leaves a fragment. `gitleaks` (the pre-commit secret hook) passes on the current tree, so it is **not** a proof of absence.
**How to avoid:** scrub every key-shaped token in the file; prove with a regex scan of the tracked tree, e.g. `git grep -nE 'sk-proj-[A-Za-z0-9_-]{16,}'` and a scan for the tail-fragment pattern, expecting no match except documented placeholders (`sk-proj-staging-...`, `sk-proj-...` ellipsis forms in `docs/src/deployment/production.md`, `k8s/secret.yaml.example`). Never echo a matched value into evidence; record counts and `file:line` only.

### Pitfall 9: Repeating a stale baseline after the release
**What goes wrong:** after `0.11.1` is published, `check-semver.sh` (`SEMVER_BASELINE_VERSION`), the `semver` job name, the ruleset context name and ~8 docs still say `0.11.0`.
**How to avoid:** not this phase's job (renaming the job would also change the required context). Record it as a re-homed item for `v0.12.0` Phase 1 with the three coupled places (script, `ci.yml` job name, ruleset context) so it is renamed together.

### Pitfall 10: Disk exhaustion mid-gate
Free space is ~5.9 GB with a 17 GB `target/`. Earlier phases hit "No space left on device" during `cargo test`. `check-semver` adds ~100-200 MB per crate unless `SEMVER_PRUNE_LOCAL_CACHE=1`. The facade compile for `make check-consumer-surface` (Cadence pin) needs `llm-anthropic` and `llm-deepseek`. Plan a `target/` prune checkpoint (`CARGO_INCREMENTAL=0`, delete `target/debug/incremental`) before the re-seal.

## Code Examples

### Registry proof (agent-runnable, read-only)
```bash
# Source: 47-CI-EVIDENCE.md house form; both endpoints verified reachable from this sandbox 2026-11-05
UA='paladin-release-verify (github.com/DF3NDR/paladin-dev-env)'
sparse_path() { n="$1"; echo "${n:0:2}/${n:2:2}/$n"; }     # all twelve names are >= 4 chars
for c in $(cargo metadata --format-version 1 --no-deps | jq -r '.packages[]|select(.publish==null)|.name' | LC_ALL=C sort); do
  curl -sS -A "$UA" "https://index.crates.io/$(sparse_path "$c")" \
   | jq -r --arg v 0.11.1 'select(.vers==$v)|[.name,.vers,.cksum,.yanked]|@tsv'
  curl -sS -A "$UA" "https://crates.io/api/v1/crates/$c/0.11.1" \
   | jq -c '.version|{num,yanked,trustpub_data}'            # provider=github, repository, run_id, sha=merge commit
done
```
Assertions to copy from Phase 47: rows = 12 = N; every `vers` = `0.11.1`, `yanked` = `false`, 64-hex `cksum`; exactly one `0.11.1` line per crate; `trustpub_data.sha` equals the tagged merge commit; `trustpub_data.run_id` equals the `release.yml` run id. For `v0.11.0` the API returned `{'provider': 'github', 'repository': 'DF3NDR/paladin-dev-env', 'run_id': '40396108246', 'sha': '70657919…'}` [VERIFIED: live call]. **Pre-tag baseline (read 2026-11-05):** each of the 12 crates' latest sparse-index line is `0.11.0`, `yanked=false`, no `0.11.1` — record it as the "before" table.

### Read the semver evidence from CI without log access
```bash
# Source: verified on job 126988790441 (Semver Checks (vs v0.11.0)), annotation level notice
curl -sS "https://api.github.com/repos/DF3NDR/paladin-dev-env/check-runs/<job_id>/annotations" | jq -r '.[].message'
# expect 12 lines "semver: <pkg>: 223 checks: 223 pass, 31 skip" and the verdict "12 package(s) checked ... 0 failure(s)"
```

### The merge/tag hand-off commands (reuse Phase 47 text with 0.11.1)
```bash
# after the merge commit exists on origin/main and ci.yml on that SHA has concluded `success`:
git fetch origin && git tag -a v0.11.1 <merge-sha> -m "v0.11.1" && git push origin v0.11.1
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Semver job pinned to `0.9.0`, a silent no-op (found in 47) | Pinned `--baseline-version 0.11.0 --release-type patch`, tripwire for zero-check output | Phase 52 | Measured live: `223 checks: 223 pass, 31 skip` ×12 on the current head; the same for a bumped-manifest crate |
| Version strings swept by hand | `check-version-strings` gate in `check-gates` + CI | Phase 54 | Lists the exact lines; red by design at the bump |
| Release body built from commit subjects / hit argv cap | `extract-changelog-section` + `release-body-fit` (125,000-char guard) | Phase 52 | Guard sees its first live run at this tag |
| Publish with a standing registry token | Trusted Publishing via OIDC, `crates-io` environment | Phase 19/47 | `trustpub_data` on the registry is the proof |
| `ubuntu-latest` = 24.04 | `ubuntu-latest` = 26.04 (label migration from 2026-10-19) | Oct 2026 | Probable cause of the SDK-smoke regression; first release run on the new image |

**Deprecated/outdated:** the `make release` single-command path (blocked trailing push); `gpt-4` as a copyable OpenAI model id (deprecated upstream, DF-18).

## Discretion defaults: confirmed (no overturn)

- **Commit-1 order (clean-tree bump first):** confirmed — `cargo release version` ran on a clean tree in ~4 s, did not commit/tag/push, and updated `Cargo.lock`. The dirty-tree case remains untested, as intended.
- **Root `[Unreleased]` stamped as written:** confirmed — the section is 13,488 characters (CONTEXT said ~60 lines; it is 60 lines), far under the 125,000-char guard.
- **Anchors for the three crates:** confirmed necessary; `git log v0.11.0..HEAD -- crates/<crate>/` works in this shallow clone because the `v0.11.0` tag is present locally. Measured counts: `paladin-herald` **0** commits (so the "no functional change" line is legitimate for it), `paladin-battalion` 2 (`79cd06bf7` test-only cross-reference; `2d3cf119a` removal of the crate-wide semver allows), `paladin-eval` 1 (`f70f03deb` docs: cancelled-run note). Note the Phase 55 rustdoc-allow removals touched other crates; derive entries per crate from its own log, and also sanity-check the eight existing `[Unreleased]` sections (content 5 commits, core 13, llm 8, memory 1, notifications 3, ports 5, storage 11, web 19; root crate `src/` 36). `crates/paladin-core` publishes as `paladin-ai-core`.

## Release pipeline facts (read from the tree)

- `release.yml` triggers: `push` on tags `v*.*.*`, plus `workflow_dispatch`. Jobs: `verify-tag-source` (tag SHA must be an ancestor of `origin/main`) → `test` (`cargo test --workspace`), `create-release` (extract `[0.11.1]` section from root `CHANGELOG.md`, fit, create-or-reuse GitHub Release), `check-release-consistency` (tag = every manifest version, `## [X.Y.Z]` heading in all 12 changelogs, `ci.yml` run on the tagged SHA concluded `success`) → `publish-crates` (needs `test`, `create-release`, `check-release-consistency`; environment `crates-io`; `id-token: write`; `scripts/publish-crates.sh --version <tag>`). `build-docker`, `build-binaries`, `sbom` need only `create-release`; `finalize-release-body` runs `if: always()` after them. The `release.yml` `test` job gates crates.io publishing, so a red `cargo test --workspace` there (for example the flake above, or the two network-bound `build_run_api_persists_*` tests Phase 55 TESTS-01 was meant to make hermetic) blocks the publish.
- `publish-crates.sh` `CRATES` order: `paladin-ai-core, paladin-ports, paladin-herald, paladin-battalion, paladin-llm, paladin-memory, paladin-web, paladin-notifications, paladin-content, paladin-storage, paladin-eval, paladin-ai` (12, equals `cargo metadata` publishable set; `paladin-doc-examples` has `publish = []`).
- `ci.yml` runs on every branch push and on PRs to `main`; the `release` label is the only trigger for `End-to-End Tests` on a PR, and E2E runs unconditionally on a push to `main`. `Publish Dry Run` is `cargo publish --workspace --dry-run`, which resolves unpublished sibling versions through `path` entries, so it works at a bump.
- **Live branch state (2026-11-05):** CI on head `b2705d7f0` — 28 jobs `success` (including `Semver Checks (vs v0.11.0)`, `Publish Dry Run`, `Coverage`), 1 `failure` (SDK smoke), others still running when read. Most earlier runs on the branch show `cancelled` (superseded pushes), so **no complete CI run on the whole milestone diff has existed for days**; the first full green read is the re-seal run.
- **Live ruleset (46 required contexts)** includes: `End-to-End Tests`, `Publish Dry Run`, `Semver Checks (vs v0.11.0)`, `Coverage`, `API Surface Tracking`, `pre-commit run --all-files`, `Build & Test (cli)`, `Workflow Lint`, `OSV Scanner`; it does not include the SDK-smoke job, `Docker Build` or `Kubernetes Smoke Test`.

## Audit inputs (SHIP-10) — measured today

| Check | Result (2026-11-05, tree = `3b0d095a2` code + docs) |
|-------|-----------------------------------------------------|
| `scripts/check-review-dispositions.sh` | `checked 51 finding(s) across 13 review file(s); 0 failure(s)` |
| `make check-findings-log` | `OK: every entry's last Disposition is a final form (parsed 36 entries ...)` |
| `gsd-tools windows status` | `open_count: 0`, `waived 53`, `fixed 55`, `total 108` |
| `make check-gates` (11 members) | all `OK`; `check-version-strings`: 128 files scanned at `0.11.0` |
| `make security` | `advisories ok, bans ok, licenses ok, sources ok` (the Phase 56 note about RUSTSEC-2026-0429 on `zip 3.0.0` no longer fails the gate; register the old deferred-items entry as `fixed` with the REG-06 pointer after re-proving it) |
| `scripts/check-semver.sh` (full, `SEMVER_PRUNE_LOCAL_CACHE=1`) | 12 packages, each `223 checks: 223 pass, 31 skip`; `12 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`; exit 0 |
| Same script, bumped manifests (0.11.1), `--package paladin-herald`, scratch clone | `223 checks: 223 pass, 31 skip`; `0 failure(s)` — the explicit `--release-type patch` pin makes the manifest version irrelevant, as the script header promises |
| `gsd-tools audit-open` | lists 5 pending todos (cap shows 7 total) — expected at close; they stay pending by D-09 |

**Carried-item source map for the register (D-06):** 57-REVIEW (`CR-01` L75, `WR-01` L105, `WR-02` L128, `IN-01..05`); `57-REGISTER-EVIDENCE.md` "Orphaned deferrals"; `52-…/deferred-items.md` (D-52-03-01/02/03); `55-…/deferred-items.md` (FIXES-04, `EmbeddingPort` rustdoc, CONTEXT deferrals); `56-…/deferred-items.md` (zip advisory — now resolved by REG-06); `54-REVIEW.md` WR-01/WR-04/WR-05/IN-02; STATE "Blockers/Concerns" (Phases 52-57 close notes); the seven `.planning/todos/pending/*.md`; the **new** items from this research (pre-commit EOF, SDK smoke, `treasury_spend` flake, semver-baseline re-pin for `v0.12.0`, prose gaps in `docs/src/api-reference`). Every row must be re-proven on the tree before dispositioning (prove-then-lock).

**Todo frontmatter gap:** all seven pending todos have `owner` but **none has `recheck_by`** (the field exists in older completed todos, e.g. `recheck_by: 2026-11-02`). D-09 requires a date; the date is "the next milestone's first planning day", which is not yet known (see Open Questions).

## SHIP-09 wording (honest-consumer statement)

Measured facts for the §11.8 subsection: §11.2 is header-only; `.cargo/semver-checks-allowlist.toml` has zero `[[entry]]` tables; `check-migration-allowlist` OK; `check-semver` `0 failure(s)` for all 12 at patch on the current head; `get_job` is unrouted but not deprecated (§11.7). **However** §11.1 carries M-B-23 … M-B-47 — 25 *behavioural* rows, several of which a direct **library** consumer observes without a compile error: M-B-30 (`ContentListRepository` on SQLite/MySQL returns `RepositoryError::QueryError` instead of panicking), M-B-33 (`FileContentFetcher::ingest_content` returns `Err` where it returned `Ok(())`), M-B-34 (two versioning services return `Err(EntityNotFound)` / stop inventing ids), M-B-35 (`PlanningService` leaves `expected_output` empty), M-B-40 (`FileStoragePort::list_files` honours `limit`), M-B-41 (reasoning-check error text), M-B-45/46/47 (Cadence pacing and cache fence semantics). D-10's "the only consumer-visible note is `get_job`" is therefore too narrow if it is read as "no behavioural change a library consumer can see". Recommended wording: *no source change is required to compile or to call any public item; run-time behaviour changes are listed in §11.1, and the ones that reach a direct library consumer are M-B-30, 33, 34, 35, 40, 41, 45, 46, 47*, plus the `get_job` note. That keeps SHIP-09's "without adaptation" claim true in the compile-time sense and honest about the rest. The maintainer should confirm this reading (Open Question 4). Also set the evidence line to the *final* head SHA (D-10 already requires the correction commit).

## Common-failure map for the re-seal rows (D-04)

| Row | Command | Notes measured here |
|-----|---------|---------------------|
| 1 | `make check-migration-allowlist` | green today |
| 2 | `v0_9_config_boot` test | run via `cargo test --test v0_9_config_boot` (not run here; disk) |
| 3 | OpenAPI golden | `cargo test -p paladin-web --lib openapi_matches_committed_baseline`; version comes from `CARGO_PKG_VERSION`, so it is red until `openapi.json` moves (a built-in bump check) |
| 4 | `make check-semver` | needs crates.io (reachable); ~18 min cold; use `SEMVER_PRUNE_LOCAL_CACHE=1`; CI twin `Semver Checks (vs v0.11.0)` |
| 5 | MSRV (1.88) | CI job `MSRV (Rust 1.88)`; local toolchain is 1.97.1 |
| 6 | `make publish-dry-run` | runs `release-check` (clean-code, test, doc tests, audit, release build) then `cargo publish --workspace --dry-run`: heavy; CI twin `Publish Dry Run` |
| 7 | `make check-gates` + `make check-changelogs` + `make check-release-consistency RELEASE_TAG=v0.11.1` | measured green on the scratch bump |
| 8 | `make check-publish-order` | green today (12 crates) |
| CI-evidence | `make api-surface` (needs `nightly-2026-09-20`, installed), `make check-consumer-surface`, `Coverage` | `cargo-llvm-cov` absent locally; Coverage is CI-only (D-00e) |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo-release` | commit 1 | yes | 1.1.7 | — |
| `cargo-semver-checks` | D-11, row 4 | yes | installed | CI `semver` job |
| `cargo-audit`, `cargo-deny`, `shellcheck`, `jq`, `pre-commit` | `make security`, gates | yes | present | — |
| `nightly-2026-09-20` toolchain | `make api-surface` | yes | installed | CI `API Surface Tracking` |
| `cargo-llvm-cov` | coverage (local) | **no** | — | CI `Coverage` job only (D-00e) |
| `mdbook`, `cargo-cyclonedx` | docs build, SBOM | no | — | CI `Build MDBook`; SBOM is release-job only |
| Docker daemon | Docker/E2E/K8s jobs, SDK generator | **no** (CLI present, `docker.sock` absent) | CLI 29.8.2 | CI only |
| crates.io sparse index + crates.io web API | registry/TP proof, semver baselines | yes (HTTP 200) | — | — |
| GitHub REST (unauthenticated GET incl. job logs, annotations, rules) | CI reads | yes | rate limit 15,000/h observed | — |
| `gh` writes (PR retitle, run re-run) | PR/CI actions | **unknown**: `gh auth status` reports `GH_TOKEN` invalid, yet `gh api` reads succeed; no write attempted | — | "hand the exact command over" (CONTEXT) |
| Disk | builds | ~5.9 GB free of 252 GB (target 17 GB) | — | prune `target/debug/incremental`, `CARGO_INCREMENTAL=0` |

**Missing with no fallback:** none blocking agent work. **Maintainer-only:** label, merge, tag, tick, key rotation, history purge.

Side-effect disclosure: while probing toolchains I ran a bare `cargo +nightly`, which installed the generic `nightly` rustup toolchain; I uninstalled it immediately (`rustup toolchain list` is back to `stable`, `nightly-2026-09-20`, `1.97.1`). A throwaway clone and logs live under the scratchpad directory only; the repository working tree was not modified by this research (the main `target/` gained semver-checks caches, pruned by the script).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Bash regression harnesses `tests/scripts/*_test.sh` (run by `make test-shell-guards`, loop over a glob so new files are picked up) + Rust `cargo test` + live CI jobs |
| Config file | none for shell harnesses; Rust per `Cargo.toml` |
| Quick run command | `make check-gates && make check-release-consistency RELEASE_TAG=v0.11.1 && bash tests/scripts/check-review-dispositions_test.sh` |
| Full suite command | `make clean-code && make security && make api-surface && make test-shell-guards && cargo test` (coverage and E2E are CI-only) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SHIP-08 | Workspace, 12 manifests, changelogs read `0.11.1`; all strings gated | gate | `make check-gates` (incl. `check-version-strings`); `make check-changelogs`; `make check-release-consistency RELEASE_TAG=v0.11.1`; grep-zero `0.11.0` over manifests/pins/gate classes | yes |
| SHIP-08 | Release build/lint/security/surface green on the release commit | gate | `make clean-code`; `make security`; `make api-surface` | yes |
| SHIP-08 | OpenAPI baseline equals bumped version | unit | `cargo test -p paladin-web --lib openapi_matches_committed_baseline` | yes |
| SHIP-08 | Semver job read a non-zero count on the release PR | CI read | `GET .../check-runs/<job>/annotations` → 12 lines `223 checks` | yes (CI) |
| SHIP-08 | Required contexts green on the final head, incl. `pre-commit run --all-files`, `Coverage`, `End-to-End Tests` | CI read | `GET .../actions/runs?head_sha=<sha>` + jobs | yes (CI) |
| SHIP-08 | `ci.yml` on the merge SHA concluded `success` (publish precondition) | CI read | `GET .../actions/workflows/ci.yml/runs?head_sha=<merge>` | yes (CI) |
| SHIP-08 | 12 crates at `0.11.1`, unyanked, Trusted Publishing, GitHub Release + assets | external read | registry probe script (Code Examples); `GET /repos/.../releases/tags/v0.11.1` | ❌ Wave 0: small read-only probe script in the phase directory (not a gate) |
| SHIP-08 | A consumer resolves and builds the published crate | smoke | scratch crate: `cargo add paladin-ai@0.11.1 && cargo build` (README quick example, keyless) | ❌ created post-tag |
| SHIP-09 | §11.8 subsection exists, names head SHA/date, states no-source-change + behavioural pointers | doc grep | `grep -n '^### Library consumers: v0.11.0' MIGRATION.md` + SHA appears in `git log` | ❌ Wave 0 grep assertion in the plan |
| SHIP-09 | §11.2 ↔ allowlist set-equal; patch semver clean | gate | `make check-migration-allowlist`; `./scripts/check-semver.sh` | yes |
| SHIP-10 | Every carried item has exactly one verdict; re-homed rows cite an existing todo with `owner` + `recheck_by` | doc check | throwaway verifier over `58-AUDIT-DISPOSITIONS.md` and `.planning/todos/pending/*.md` (rows × {fixed, waived, re-homed}; todo path exists; frontmatter has both keys) | ❌ Wave 0 |
| SHIP-10 | No undispositioned review warning in Phases 38-51 | gate | `./scripts/check-review-dispositions.sh` (`0 failure(s)`) | yes |
| SHIP-10 | Findings log fully dispositioned; WINDOWS at 0 | gate | `make check-findings-log`; `gsd-tools windows status` (`open_count: 0`) | yes |
| SHIP-10 | WR-02 fixed with near-miss fixtures | shell unit | `bash tests/scripts/check-review-dispositions_test.sh` | yes (extend) |
| (folded) | Cadence imports pinned | compile | `make check-consumer-surface` | yes (extend `tests/consumer_surface_muster.rs`) |
| (folded) | `gpt-4` literal gone / gated | gate | `make check-doc-examples`; new `check-doc-models` (if landed) + its `tests/scripts/*_test.sh` | partly ❌ |

### Sampling Rate
- **Per task commit:** the task's own check plus `pre-commit run --all-files` (or at minimum the hooks over staged files) — the required CI context is the strictest per-commit bar.
- **Per wave merge:** `make check-gates`, `make test-shell-guards`, `./scripts/check-review-dispositions.sh`, `make check-findings-log`.
- **Phase gate:** the 8-row re-seal on the candidate final head + PR CI all-green + registry table 12/12 before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] Near-miss fixtures for WR-02 in `tests/scripts/check-review-dispositions_test.sh` (dates, `ADR-0063`/`SHA-256`/`UTF-8`, `and/or/else`, `not_a_thing` must fire `CLOSED_NO_POINTER`).
- [ ] Register-completeness verifier for `58-AUDIT-DISPOSITIONS.md` (verdict per row; todo paths exist; `owner` + `recheck_by` present).
- [ ] Read-only registry/TP probe script (phase directory) producing the 12-row table and the assertions listed above.
- [ ] If the `gpt-4` gate lands: script + Makefile target + `tests/scripts/check-doc-models_test.sh` + allowlist for deliberate deprecated-id pages.
- [ ] Framework install: none (Bash + cargo already present).

## Security Domain

`security_enforcement` is not set to `false` in `.planning/config.json` (absent = enabled).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V1 Architecture / supply chain | yes | Trusted Publishing via OIDC, `crates-io` environment restricted to `v*.*.*` tags, `verify-tag-source` ancestor check, `check-release-consistency` gate; no standing registry credential (D-00g) |
| V2/V3 Authentication, Session | no (no runtime auth change) | — |
| V4 Access Control | yes (release acts) | Ruleset: PR-only merges, zero bypass actors on `main`; tag creation limited to repository admin ("Protect release tags"); maintainer-only irreversible acts (D-00b) |
| V5 Input Validation | yes (shell/script edits) | `shellcheck` hook; scripts take tag/version only via `env:`; `gpt-4` gate regexes fixture-tested |
| V6 Cryptography | no new use | `X-Paladin-Signature` HMAC and SSRF guard are untouched |
| V8 Data Protection | yes | Scrub the tracked `sk-proj-…` literal; never quote key material in evidence, commits or PR body; rotation/purge are maintainer acts |
| V14 Configuration | yes | Required-context hygiene (46 contexts); `.cargo/audit.toml` / `deny.toml` registers unchanged unless a dated exception is added; GitHub Actions pinned by major (`crates-io-auth-action@v1`) |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Tag pushed from a non-`main` commit | Spoofing/Tampering | `verify-tag-source` rejects a tag SHA not contained in `origin/main`; merge commit + annotated tag (D-00c) |
| Partial publish (some crates at `0.11.1`, others not) | Tampering/Repudiation | `publish-crates.sh` registry pre-check and outcome table; option-A recovery (patch + yank after verification), agent read-only (D-00s) |
| Secret echoed into evidence/commits | Information disclosure | Redact-before-truncate rule; evidence records counts and `file:line`, never values; `gitleaks` hook (note: it did **not** flag the existing literal, so a regex scan is the real proof) |
| Version/changelog/tag disagreement publishing the wrong content | Tampering | `check-release-consistency` pre-publish gate; `check-version-strings`; OpenAPI golden test |
| Poisoned runner image/action drift | Tampering | Pin by major; the 26.04 migration is the live example — record, do not rehearse (D-00k) |
| Registry yank/overwrite | Repudiation | crates.io versions are immutable; yanks only by the maintainer after verification |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The SDK-smoke regression is caused by the `ubuntu-latest` → Ubuntu 26.04 migration changing how `npm install <dir>` runs the generated package's `prepare` build; a local npm 10.9.4 did not reproduce it | Pitfall 3 | The proposed `run.sh` fix may not cure it; budget for a second CI cycle (~63 min is for the whole `ci.yml`, the SDK job itself takes ~3 min) or escalate to the maintainer under D-05 |
| A2 | `treasury_spend_prints_one_row_per_currency` is a sub-second timestamp/window race | Pitfall 5 | If it is a real ordering bug in `treasury.rs`, it is a `src/` fix outside this phase's boundary; changes the checkpoint wording, not the plan shape |
| A3 | The `release.yml` jobs will run without surprises on Ubuntu 26.04 because 28 `ci.yml` jobs did | Pitfall 4 | A post-tag failure in a non-publish job (docker/binaries/sbom) leaves a GitHub Release with fewer assets; maintainer-owned recovery |
| A4 | A concrete `recheck_by` date such as the first planning day of `v0.12.0` can be chosen without the maintainer | Open Question 3 | Re-homed rows would carry a placeholder date and fail D-09/SHIP-10's "owner and a date" test |
| A5 | Replacing `gpt-4` with `gpt-4o-mini` (the todo's recommendation, proven by the maintainer's keyed walk on 2026-11-03) is still served | Don't Hand-Roll / gpt-4 sweep | A mock-based or snapshot test that embeds the model name may need the same edit; the walk, not this research, is the evidence |
| A6 | The pre-commit gitleaks default ruleset not flagging the existing literal means the scrub must be proven by regex, not by the hook | Pitfall 8 | Low; a regex scan costs nothing |

## Open Questions

1. **Treat the SDK-smoke job as a tag precondition?**
   - What we know: not a required ruleset context; `check-release-consistency` demands a whole-run `success` for `ci.yml` on the tagged SHA; D-05 currently allows "accept a red non-required job at the merge".
   - What's unclear: whether the maintainer wants that option removed.
   - Recommendation: reword the D-05 checkpoint to "fix in the release PR, or hold the tag"; if the CI-side fix does not work, escalate rather than accept red.
2. **How is the `treasury_spend_*` flake handled at the re-seal?**
   - What we know: one failure in the required `Build & Test (cli)`, pass on the next head; D-00j's exception names only infrastructure flakes; the test is under `src/`.
   - Recommendation: characterise it in the build wave and put the rule to the maintainer once, before the re-seal.
3. **What concrete date is `recheck_by`?**
   - What we know: D-09 says "the next milestone's first planning day"; no `v0.12.0` date exists in ROADMAP/STATE.
   - Recommendation: ask the maintainer for the date at the register checkpoint; do not invent one (an `[ASSUMED]` default would be a short fixed date after the tag, but only with confirmation).
4. **Is "without adaptation" the right sentence given M-B-30/33/34 (Ok→Err, panic→Err)?**
   - Recommendation: the compile-time statement plus a named pointer list (see SHIP-09 wording); confirm with the maintainer when the subsection is drafted.
5. **Audit scope.** Confirm that the pre-tag `/gsd-audit-milestone` is scoped to Phases 52-57 with SHIP-08/09/10 explicitly out of scope and proven post-tag (v0.11.0 precedent), versus accepting a `gaps_found` pre-tag audit and a post-tag re-audit.
6. **Do the two stale prose statements in `docs/src/api-reference/` get fixed in this phase?** The gate does not catch them; the success criterion's wording implies "every user-facing version string reads 0.11.1".

## Sources

### Primary (HIGH confidence — executed or read this session)
- Repository files: `.planning/phases/58-crate-release/58-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/STATE.md`, `53-VERSION-DECISION.md`, `54-FIRST-EVIDENCE.md`, `47-CI-EVIDENCE.md`, `MIGRATION.md` §10.8/§11, `Makefile`, `release.toml`, `.github/workflows/{release,ci}.yml`, `scripts/{check-version-strings,finalize-crate-changelogs,check-release-consistency,check-semver,check-review-dispositions,publish-crates,release-body-fit}.sh`, `.github/rulesets/protect-main-branch.json`, `.pre-commit-config.yaml`, `docs/src/appendix/release-automation.md`, `.claude/gsd-core/workflows/audit-milestone.md`
- Local executions: `cargo metadata`, `cargo release version` (scratch clone), `scripts/check-version-strings.sh`, `make finalize-crate-changelogs`, `make check-changelogs`, `make check-release-consistency`, `make check-gates`, `make security`, `scripts/check-semver.sh` (full and per-package), `scripts/check-review-dispositions.sh`, `make check-findings-log`, `gsd-tools windows status`, `pre-commit run <hook> --all-files`
- GitHub REST (public, read): job logs 126917997570 / 126988790544 / 121733645764 / 126979656952 / 126979605660, runs `41949135307`, `41946243555`, `41927940508`, branch rules `GET /rules/branches/main` (46 contexts), check-run annotations of job 126988790441
- crates.io: sparse index for all 12 crates; `https://crates.io/api/v1/crates/paladin-ai/0.11.0` (`trustpub_data`)
- `gsd-tools query package-legitimacy check --ecosystem crates ...`

### Secondary (MEDIUM confidence)
- Phase 47 precedent documents (house forms) and `docs/src/appendix/release-automation.md` Operator Guide (describes the flow; exercised in Phase 47)

### Tertiary (LOW confidence)
- The npm/Ubuntu 26.04 causal story (A1) and the timestamp-race story (A2): log-supported but not reproduced

## Metadata

**Confidence breakdown:**
- Standard stack / mechanics: HIGH — reproduced end to end in a scratch clone and re-checked against the gates
- Architecture / sequencing: HIGH — read from `release.yml`, `ci.yml`, ruleset and scripts; matches the Phase 47 precedent
- Pitfalls: HIGH for 1, 2, 6-9 (read/measured); MEDIUM for 3-5 (diagnosis from logs, not reproduced)

**Research date:** 2026-11-05
**Valid until:** the earlier of the next push to `main` (D-02 freeze), a `ubuntu-latest` image change that alters the SDK-smoke result, or 7 days — CI state and the runner image are fast-moving
