# Phase 37: v0.10.0 Crate Release - Research

**Researched:** 2026-09-18
**Domain:** Release engineering — GitHub Actions release pipeline, crates.io Trusted Publishing, branch-protection-gated tag cutting, evidence bookkeeping
**Confidence:** HIGH for release-mechanics claims (all traced to file:line or live command output); MEDIUM for crates.io Trusted Publishing policy (confirmed via two independent web sources, not an official crates.io page fetch); LOW/ASSUMED flagged individually below

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- **D-00a:** Only the maintainer ticks the §11 sign-off box in `29-ACCEPTANCE-AUDIT.md` — never an agent, under any mode including `--auto`.
- **D-00b:** The `0.10.0` bump already landed on the feature branch without a tag; the tag is cut on the `main` merge commit and `release.yml`'s `verify-tag-source` enforces it. "The release commit" means both the PR head and the merge commit; evidence is recorded for both.
- **D-00c:** The tag-cut mechanism is the documented one: a version-bump PR merged to `main`, then an annotated tag pushed directly to the merge commit. `make release VERSION=0.10.0` is NOT used.
- **D-00d:** Findings are recorded, never fixed silently. `WINDOWS.md` rows are never deleted. Amend-at-source.
- **D-00e:** The 82% workspace line-coverage floor (ADR-0006) is measured by CI's `coverage` job only.
- **D-00f:** Shipped tree outranks any document. The publishable-crate count is whatever the tree says, not "eleven."
- **D-00g:** Publishing is via crates.io Trusted Publishing — no standing registry credential exists. Nothing in this phase may introduce or request one.
- **D-01:** The agent pushes `feature/phase-33` and opens the release PR to `main` via `gh`, with a curated body. This unlocks the CI `coverage` run SC2 needs.
- **D-02:** The maintainer performs both irreversible acts — merging the PR and pushing the `v0.10.0` tag — at one blocking checkpoint. The agent hands over the exact merge method and the exact `git tag -a v0.10.0 <merge-sha>` + push commands, then stops. Reversibility: one-way.
- **D-03:** §11 is ticked after the PR's CI is green. Order: local re-seal → push → PR CI green including `coverage` ≥ 82% → evidence appended → maintainer ticks §11 → tick commit pushed → CI re-runs on the true final SHA → merge.
- **D-04:** The PR is merged with a merge commit — not squash, not rebase. Reversibility: one-way.
- **D-05:** Phase 37's CI evidence lives in a new `37-CI-EVIDENCE.md`, house form of `27/28/29/33-CI-EVIDENCE.md`. `29-CI-EVIDENCE.md`/`33-CI-EVIDENCE.md` each get one dated forward-pointer line, otherwise untouched. The STATE.md sentence naming `33-CI-EVIDENCE.md` as the pre-merge-run destination is corrected to name `37-CI-EVIDENCE.md`.
- **D-06:** SC1 is one new dated re-seal section appended to `29-ACCEPTANCE-AUDIT.md`: a seven-row gate table (MIGRATION.md no-TBD + §9.2↔allowlist, `v0_9_config_boot`, OpenAPI golden diff, `cargo semver-checks`, MSRV job, `make publish-dry-run` in dependency order, CHANGELOG `[0.10.0]` complete) — each row with command, SHA, result, pointer to `37-CI-EVIDENCE.md`. Existing rows are not edited.
- **D-07:** The requirement is a newly minted `SHIP-05` — "v0.10.0 is released." `SHIP-04` is NOT extended, re-opened or edited. Traceability table gains `SHIP-05 | Phase 37`; ROADMAP Phase 37's `**Requirements**: TBD` line is replaced.
- **D-08:** SC4's registry proof is a sparse-index query per publishable crate, recorded as a table in `37-CI-EVIDENCE.md` (name, version 0.10.0, checksum, yanked=false). Crate list derived from `cargo metadata` (`publish != false`), never hard-coded. MILESTONES.md gets a v0.9.0-style prose entry pointing at that table. No consumer smoke-build; the workflow's own log is not accepted as proof on its own.
- **D-09:** The GSD phase stays open across the merge. Plans split into a pre-merge wave and a post-tag wave, with the maintainer's merge+tag checkpoint (D-02) between them. Deliberate departure from Phase 29 D-21's "close on pre-merge evidence."
- **D-10:** Post-tag commits reach `main` through a `chore/37-close` branch cut from tagged `main` plus a second, small, docs-only PR. No further commits onto `feature/phase-33` after the merge.
- **D-11:** `/gsd-complete-milestone v0.10.0` is NOT run by any plan in this phase. Phase 37's last plan ends at "ready to close." The phase verifies on SC1-SC4 plus SC5's preconditions.
- **D-12:** Waiting is a hard stop with a resume file, never a parked watcher. At the merge+tag checkpoint the agent writes a `.continue-here.md` naming the exact resume condition — tag `v0.10.0` exists on `origin` and points at a commit contained in `main` — and stops. No long-lived `gh run watch` agent is left parked.
- **D-13:** The pipeline is rehearsed by a `workflow_dispatch` of `release.yml` with `dry_run=true`, against the merge commit, after the merge and before the real tag. **Open for the researcher — answered below in Q1.** If a dry run cannot be dispatched without a real tag, fall back to going straight to the tag, record why; do NOT substitute an rc tag.
- **D-14:** Any red gate is a stop. The agent records the finding, halts at a blocking checkpoint, fixes nothing.
- **D-15:** The single exception to D-14 is a pure infrastructure flake (runner timeout, registry 5xx, image-pull failure) — one `gh run rerun --failed` permitted. A red caused by the retired Docker Hub MinIO image on a non-rebased tree is a real failure, not a flake.
- **D-16:** If the real release run goes red after the tag, the agent diagnoses read-only per `release-recovery.md` §1-§2 and hard-stops. Completing forward, re-dispatch, and yank belong to the maintainer.

### Claude's Discretion

- Who dispatches the D-13 dry run. Default: the agent may dispatch it once the maintainer reports the merge is done; if the token lacks `workflow` scope, hand the exact command to the maintainer instead.
- The local re-seal's command order and how outputs are captured, provided every one of D-06's seven rows has a command, a SHA and a result.
- The release PR's title and body wording, and the MILESTONES.md entry's prose, within the v0.9.0 entry's shape.
- Whether `feature/phase-33` is deleted after the merge — leave it to the maintainer's GitHub setting.
- Plan count and wave layout, provided D-09's two-wave split around the D-02 checkpoint holds.
- If `main` moves before the merge: re-establish "the final commit," re-run the re-seal from the top, §11 re-confirmed against the new SHA.

### Deferred Ideas (OUT OF SCOPE)

- Correcting the stale "eleven crates" count in `docs/src/contributing/development-setup.md` — a docs-currency fix for v0.11.0. Record as a finding if observed (it is — see Q3/Q8 below; the true count is **twelve**, and the doc's own release-checklist.md and release-automation.md already say twelve/eleven inconsistently — see State of the Art table).
- A consumer smoke test (`cargo add paladin@0.10.0` cold build) — considered for D-08, not chosen.
- Resolve-then-rehearse without a tag: if `release.yml` cannot be dry-run before a tag exists, making the workflow accept a bare SHA is a CI change for a later milestone, not this phase.
- Carried from Phase 36.1: `cargo test --workspace --all-features` in CI; removing the eight crate-level rustdoc allows; regenerating `doc-coverage-report.md`; the Phase 25/28 register question.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SHIP-05 | "v0.10.0 is released": gates re-sealed on the final commit, tag on the `main` merge commit, every publishable crate on crates.io at `0.10.0`, release evidence in MILESTONES.md. Newly minted per D-07 — NOT an extension of SHIP-04. | This document's entire Q1-Q10 answer set; the ROADMAP/REQUIREMENTS.md mint sites are named in Q7 below. The planner mints `SHIP-05` in `.planning/REQUIREMENTS.md` (new row under the `SHIP` prefix, `~line 321` after SHIP-04, plus the traceability table `~line 673` after `SHIP-04 \| Phase 29 \| Complete`) and replaces ROADMAP Phase 37's `**Requirements**: TBD` line with `**Requirements**: SHIP-05`. |
</phase_requirements>

## Summary

This phase has almost no code to write — it is a release-mechanics execution plan. The two hard technical findings that change the plan's shape are (1) the `release.yml` `workflow_dispatch` `tag` input cannot be satisfied before the real `v0.10.0` tag exists — D-13's dry-run rehearsal is **not dispatchable pre-tag** under the sanctioned reading, so D-13's own documented fallback (go straight to the tag) is the path the plan must take; and (2) **`paladin-eval` has never been published to crates.io** (confirmed 404 on the sparse index) and crates.io Trusted Publishing categorically cannot mint a token for a crate's first-ever publish — a Trusted Publisher configuration can only be created for a crate that already exists on the registry. This is a genuine, D-00g-constrained blocker: the maintainer must manually publish `paladin-eval@0.10.0` with a personal API token (mirroring the exact bootstrap-then-Trusted-Publishing-then-revoke sequence the other eleven crates went through in Phase 19) at some point before or during the real `publish-crates` run reaches it in dependency order (position 11 of 12), or the real release run will go red at that crate per D-16 and the agent must hard-stop and hand this exact finding to the maintainer rather than improvise a workaround.

Every one of the seven D-06 re-seal gates has an exact, already-proven local command — lifted verbatim from `29-CI-EVIDENCE.md` and `33-CI-EVIDENCE.md`, both of which ran the identical gate set on this exact tree shape. The crate count is verified **twelve** (not eleven, not the "twelve publishable + `paladin-doc-examples` excluded" figure Phase 33 already used) via a live, read-only `cargo metadata` run this session. The branch is clean of the MinIO Docker Hub retirement risk (D-15's named real-failure case) — `quay.io/minio` pins and `--retry` curl are already present on `feature/phase-33`. No open PR exists; the branch is 490 commits ahead of `origin/main`, 0 behind, with 14 unpushed local commits beyond its own remote tracking branch.

**Primary recommendation:** Structure the phase as two waves separated by the D-02 human checkpoint, exactly as D-09 specifies. Wave 1 ends with the agent handing the maintainer the merge command and the tag command and writing a `.continue-here.md` naming the resume condition. Wave 2 opens by re-verifying that condition, then (a) surfacing the `paladin-eval` first-publish gap as a `checkpoint:human-action` **before** the real tag push if at all possible (better: have the maintainer manually publish+configure-Trusted-Publishing for `paladin-eval` during the same session as the merge, before the tag is pushed, so the real release run's `publish-crates` job does not hit an unconfigured crate mid-loop), (b) running the D-13 dry run is **not possible pre-tag** so it is skipped with the documented D-13 fallback rationale recorded, and (c) diagnosing read-only per D-16 if the real run still goes red at `paladin-eval`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Gate re-seal (local commands) | Local devcontainer (agent) | — | All seven D-06 gates except `coverage` run offline against the checked-out tree; no network egress beyond `cargo metadata`/registry reads. |
| PR open + push | GitHub API (`gh` CLI, agent-driven) | Local git | D-01 explicitly delegates this to the agent; irreversible only in the trivial sense a push can be force-updated pre-merge. |
| Coverage measurement | CI (GitHub Actions `coverage` job) | — | D-00e: this devcontainer has no Docker; the floor is measured in CI only. |
| Merge to `main` | Human (maintainer via GitHub UI/`gh`) | — | D-02, D-04: agent never merges; ruleset requires PR + all 44 required checks, no bypass actor exists for the branch ruleset. |
| Tag creation/push | Human (maintainer, local git or `gh`) | GitHub tag ruleset (`protect-release-tags`, bypass actor = admin) | D-02: the one-way, publish-triggering act. |
| crates.io publish | CI (GitHub Actions `publish-crates` job, OIDC Trusted Publishing) | Human (only for `paladin-eval`'s first publish, per Q3) | Standard crates already have Trusted Publishing configured; `paladin-eval` categorically cannot use it for its first release — a documented policy fact, not this repo's tooling. |
| Registry verification (SC4) | Local devcontainer (read-only HTTP to `index.crates.io`) | — | No credential needed; D-08 mandates this exact mechanism. |
| Milestone-close bookkeeping | Human (`/gsd-complete-milestone`, on `chore/37-close`) | Agent (writes MILESTONES.md entry, lists preconditions) | D-11: the phase ends at "ready to close," never runs the command itself. |

## Research Question Answers

### Q1 — D-13's dry-run dispatch before the tag exists (traced through the whole file)

**Every use of the `tag` input in `release.yml`, traced job by job:**

1. **`verify-tag-source` / "Resolve release commit"** (`release.yml:45-69`): on `workflow_dispatch`, runs `git fetch --tags --force --quiet origin` then `SHA=$(git rev-list -n 1 "$RELEASE_TAG")` where `RELEASE_TAG = inputs.tag`. `git rev-list -n 1 <ref>` requires `<ref>` to resolve as an existing git revision — a real tag, branch, or SHA. **It does not create or assume anything; it only resolves.** Under `set -euo pipefail`, an unresolvable string kills this step immediately, before the ancestor check ever runs. `[VERIFIED: file read, release.yml:45-69]`
2. **`verify-tag-source` / "Ensure release commit is contained in main"** (`release.yml:71-83`): `git merge-base --is-ancestor "$SHA" origin/main`. This step is unreachable if step 1 failed. `[VERIFIED]`
3. **`create-release` / "Get version"** (`release.yml:126-135`): on `workflow_dispatch`, `echo "version=$RELEASE_TAG"` — **the literal input string, verbatim, with no derivation from the resolved commit and no "v" stripping.** `[VERIFIED]`
4. **`create-release` / "Extract changelog section"** (`release.yml:148-159`): calls `scripts/extract-changelog-section.sh --version "$RELEASE_VERSION"` against root `CHANGELOG.md`. A version string with no matching `## [<version>]` heading is "a hard, named failure with no fallback" per the job's own comment (`release.yml:147`). `[VERIFIED]`
5. **`check-release-consistency` / "Get version"** (`release.yml:427-436`): same literal-echo pattern as step 3, independently derived in a separate job.
6. **`check-release-consistency` script** (`scripts/check-release-consistency.sh:301`): strips **at most one** leading `v` (`TAG_VERSION="${TAG#v}"`), then requires string-equality against every publishable crate's manifest version (clause 1 = `MISMATCH`) and a matching `## [<version>]` CHANGELOG heading (clause 2 = `CHANGELOG_MISMATCH`), plus (clause 3) a green `ci.yml` run on `--sha` (`CI_MISMATCH`/`CI_LOOKUP_FAILED`/`MISSING_SHA`). `[VERIFIED: file read in full]`
7. **`publish-crates` / dry-run mode switch** (`release.yml:617-655`): reads `github.event.inputs.dry_run`; when `"true"`, skips the OIDC `Authenticate with crates.io` step entirely (`if: steps.mode.outputs.dry_run != 'true'`, `release.yml:634`) and passes `--dry-run` to `publish-crates.sh`, which runs `cargo publish --dry-run` per crate and mints no token. `[VERIFIED]`

**Tracing every candidate string through steps 1-6 simultaneously:**

| Candidate `tag` input | Step 1 (resolves as revision?) | Steps 3-4 (changelog heading match?) | Step 6 clause 1 (manifest match?) | Net result |
|---|---|---|---|---|
| `v0.10.0` or `0.10.0` (no such ref exists pre-tag) | **Fails** — `git rev-list -n 1` errors, no such tag/branch/SHA | n/a (job never runs) | n/a | `verify-tag-source` fails outright |
| The exact 40-char merge-commit SHA | Passes (any SHA resolves) | **Fails** — no `## [<40-hex-chars>]` heading exists | Would also fail (SHA ≠ `0.10.0`) | `create-release` hard-fails; `publish-crates` (needs `create-release`) never runs |
| An existing unrelated tag, e.g. `v0.9.0` | Passes | Passes (heading exists) | **Fails** — manifest is `0.10.0`, tag strips to `0.9.0` | `check-release-consistency` reports `MISMATCH`; gate blocks `publish-crates` |
| `main` (branch name) | Passes | **Fails** — no `## [main]` heading | Fails | `create-release` hard-fails |

**No candidate string satisfies all three constraints (resolves as a revision, matches a CHANGELOG heading, matches every crate manifest) except a ref that is *already, literally* named `v0.10.0` or `0.10.0`.** That ref does not exist until the maintainer creates it — which is exactly the act D-13 is trying to rehearse *before*.

**Conclusion: (b) — proof it cannot be dispatched without a real tag under the sanctioned usage.** This is not a guess; it follows deterministically from the four rows in the table above being exhaustive over how the input reaches downstream jobs. **D-13's own documented fallback applies: go straight to the tag, record why, do not substitute an rc tag.** `[VERIFIED — derived from full-file trace, not assumed]`

**A theoretical escape hatch exists and is deliberately NOT recommended:** a lightweight tag literally named `0.10.0` (no leading `v`) would resolve in step 1, match the CHANGELOG heading and the manifest version in steps 3-6, **and would not match the `push: tags: v*.*.*` trigger glob** (`release.yml:5-6`), so pushing it would not itself fire the real tag-push release. This was traced but not tested (creating any tag is forbidden under this research task's hard safety rule, and testing it would itself be the kind of improvisation D-14's "when unsure, stop and record" temperament discourages). It is recorded here as a finding for the planner to consider and explicitly reject in favor of the simpler, sanctioned D-13 fallback — introducing an extra pushed tag object outside the documented flow adds cleanup burden and ambiguity for a low-value rehearsal given the seven local gates already prove packaging validity (Q5 below). **`[ASSUMED]`** — not exercised; do not act on this without the maintainer's explicit sign-off if the planner considers it.

**Token permission needed for the dispatch (if ever attempted post-tag, per D-13's letter — "against the merge commit, after the merge and before the real tag" — which per the trace above is unreachable, but the permission question stands for any future `workflow_dispatch` on this workflow):** `gh workflow run` requires the token to be able to trigger Actions runs — classic PAT `workflow` scope, or fine-grained PAT `Actions: write`. The devcontainer's `GH_TOKEN` is a fine-grained PAT with previously observed read-only failure modes (known environment fact). **Recommendation: do not attempt to test this; if D-13's rehearsal step is retained at all in the plan, gate it behind a `checkpoint:human-action` handing the maintainer the exact `gh workflow run release.yml -f tag=<value> -f dry_run=true` command** — though per the trace above, no value of `<value>` actually works pre-tag, so the more honest plan action is to skip the dispatch entirely and record the reason. `[CITED: release-automation.md "workflow_dispatch-triggered runs minting a Trusted Publishing token is untested" — a related, independently documented gap]`

### Q2 — `check-release-consistency`'s sequencing constraints

`scripts/check-release-consistency.sh` implements three clauses, all in one report (never fail-fast on the first):

1. **Manifest agreement (`MISMATCH`):** every publishable package's `Cargo.toml` version must string-equal the tag version (no semver coercion).
2. **Changelog agreement (`CHANGELOG_MISMATCH`):** every publishable package's own `CHANGELOG.md` (co-located with its manifest) must carry a `## [<version>]` heading.
3. **CI-conclusion agreement (`CI_MISMATCH` / `CI_LOOKUP_FAILED` / `MISSING_SHA`):** **yes — the tagged commit must have a recorded, `success`-concluded `ci.yml` run before the gate passes.** This is queried live via `gh api repos/.../actions/workflows/ci.yml/runs -f head_sha=<sha> -f status=completed`, sorted by `created_at`/`id`, taking the last (most recent) run. `MISSING_SHA` fires if run inside GitHub Actions (`GITHUB_ACTIONS=true`) with no `--sha` — the CI-conclusion clause can never be silently skipped on the CI path. Outside CI (a local run), an absent `--sha` runs clauses 1-2 only and says so explicitly — this is what the local `29`/`33-CI-EVIDENCE.md` sweeps did (row 13 in `29-CI-EVIDENCE.md`: "the CI-conclusion clause is explicitly skipped locally").

**Ordering implication for D-02's checkpoint wording:** because `check-release-consistency` (run as a `release.yml` job on the tag push) checks CI's conclusion for **the tagged SHA itself** — the `main` merge commit — the merge commit must already have a green `ci.yml` run recorded **before** the tag is pushed, or the gate reports `CI_MISMATCH` (a real run existed but didn't conclude success) or `CI_LOOKUP_FAILED`/an implicit "no run" case. Concretely: **push the tag only after `ci.yml` has finished green on the merge commit** — not immediately upon merging. `ci.yml` runs take **~60-90 minutes** on this repo (known environment fact; also cross-checked against `29-CI-EVIDENCE.md`'s own 33-job/37-job run compositions, consistent with a large matrix). The D-02 hand-off to the maintainer should therefore include: "merge the PR, wait for `ci.yml` to conclude success on the merge commit (~60-90 min), *then* push the tag" — not "merge and immediately tag." Recovery path if the tag is pushed too early: `release-recovery.md` §6's `CI_MISMATCH` remedy is either re-running `ci.yml` on that SHA or fixing-and-re-tagging; no re-tag is needed if CI simply hadn't finished yet — re-running the *same* release workflow run once CI catches up is the documented recovery (per `release-recovery.md` §3). `[VERIFIED: scripts/check-release-consistency.sh full read + release-recovery.md §6]`

### Q3 — crates.io Trusted Publishing and first-time crates (BLOCKER)

**Crate list, derived live from `cargo metadata --no-deps --format-version 1` this session (never hard-coded):**

```
PUBLISHABLE COUNT: 12
  paladin-ai (root, 0.10.0)          paladin-battalion (0.10.0)
  paladin-ai-core (0.10.0)           paladin-ports (0.10.0)
  paladin-llm (0.10.0)               paladin-storage (0.10.0)
  paladin-content (0.10.0)           paladin-eval (0.10.0)
  paladin-herald (0.10.0)            paladin-memory (0.10.0)
  paladin-notifications (0.10.0)     paladin-web (0.10.0)
EXCLUDED (publish=false): paladin-doc-examples
```
`[VERIFIED: live cargo metadata run]` — **the count is twelve, confirmed, not "eleven."**

**Sparse-index query per crate (`https://index.crates.io/<prefix>/<name>`), run live this session:**

| Crate | Highest published version | Status |
|---|---|---|
| paladin-ai-core | 0.9.0 | published |
| paladin-ports | 0.9.0 | published |
| paladin-herald | 0.9.0 | published |
| paladin-battalion | 0.9.0 | published |
| paladin-llm | 0.9.0 | published |
| paladin-memory | 0.9.0 | published |
| paladin-web | 0.9.0 | published |
| paladin-notifications | 0.9.0 | published |
| paladin-content | 0.9.0 | published |
| paladin-storage | 0.9.0 | published |
| paladin-ai | 0.9.0 | published |
| **paladin-eval** | **none — HTTP 404** | **never published** |

`[VERIFIED: live curl to index.crates.io, this session]`

**crates.io Trusted Publishing policy for a never-published crate:** confirmed via two independent sources (a WebSearch synthesis citing the crates.io Trusted Publishing docs and RFC, and a direct fetch of the rust-lang blog's 2025-07 crates.io development update): **a crate's first release must be published manually with a personal API token; a Trusted Publisher configuration cannot be created for a crate that does not yet exist on the registry.** Exact quoted language surfaced: *"To get started with Trusted Publishing, you'll need to publish your first release manually. After that, you can set up trusted publishing for future releases."* `[CITED: blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07 via WebFetch; cross-confirmed by WebSearch synthesis of crates.io/docs/trusted-publishing and the RFC 3691 book]` — tag as CITED rather than VERIFIED because the direct fetch of `crates.io/docs/trusted-publishing` itself returned no substantive body (JS-rendered page); the claim rests on the blog post and search synthesis, not the primary docs page's raw HTML.

**This is a real, unresolved gap in this repository's release readiness, not a hypothetical:**
- `release-automation.md`'s "Per-Crate Trust Configuration" table lists Trusted Publishing links for **only the eleven pre-0.10.0 crates** — `paladin-eval` has no row.
- `ci.yml`'s `semver` job explicitly excludes `paladin-eval` from the semver-diff check with the comment "has no published `0.9.0` baseline… would ERROR for a nonexistent baseline" (`ci.yml:341-349`) — this is the same underlying fact (never published) surfacing in a different gate.
- `scripts/publish-crates.sh`'s `CRATES` array places `paladin-eval` at position 11 of 12, immediately before the facade `paladin-ai` (`publish-crates.sh:144-157`).
- ADR-0048 records `paladin-eval` as `publish = true` and registered in the publish script, but **says nothing about how its first Trusted-Publishing-authenticated release is supposed to succeed.**

**What happens on the real release run if this is not resolved first:** `publish-crates.sh`'s per-crate loop authenticates once via OIDC for the *whole job* (`release.yml:632-636`, one token for the entire sequential loop) — the *authentication* step succeeds regardless of per-crate trust configuration, because OIDC token minting is job-scoped, not crate-scoped. The failure would occur at **`cargo publish -p paladin-eval`** itself: crates.io validates the *publish request's* identity against that specific crate's own Trusted Publisher configuration (or lack thereof) at upload time, not at the job's OIDC-exchange time. A crate with no configured Trusted Publisher rejects the upload — this manifests as a `failed` outcome in the per-crate table (`publish-crates.sh`'s `_pc_publish_one`), which (per the script's exit rules) aborts the whole run and marks `paladin-ai` (the facade, the only crate after `paladin-eval` in dependency order) `skipped`. **Net effect: a real, tag-triggered release run would publish 10 of 12 crates and then hard-fail at `paladin-eval`, leaving `paladin-ai` (the facade) never published for v0.10.0.**

**D-00g-compliant path forward (recorded as a finding, NOT a task the agent performs):** the only way to close this gap without introducing a *standing* credential is the exact bootstrap-then-revoke sequence Phase 19 already used for the original eleven crates (documented in `release-automation.md`'s "Credential History" table): the maintainer mints a short-lived personal crates.io API token, manually runs `cargo publish -p paladin-eval` locally (or via a one-off authenticated step) once, configures `paladin-eval`'s Trusted Publisher link via the crates.io web UI (repository `DF3NDR/paladin-dev-env`, workflow `release.yml`, environment `crates-io` — matching the other eleven rows), then revokes the personal token. This must happen **before** the real tag is pushed (so the real `publish-crates` run finds `paladin-eval` already `already-at-this-version`, or — if done between merge and tag exactly as `paladin-ai-core` etc. were bootstrapped originally — before the tag-triggered run reaches it in the dependency loop). **This is a maintainer action, not an agent action** — no agent-executable path exists that both satisfies D-00g and unblocks the crate. The plan must surface this prominently as a `checkpoint:human-action` early in the pre-merge (or immediately post-merge, pre-tag) wave, not discover it mid-release. `[VERIFIED: derived from release.yml + publish-crates.sh + ADR-0048 + live registry query, cross-referenced with CITED crates.io policy]`

**Canonical publish order — cross-checked against three sources, all agree:**
`scripts/publish-crates.sh` `CRATES` array, `docs/src/appendix/release-checklist.md` §5-6, and `29-CI-EVIDENCE.md` row 14's dependency-order confirmation all list: `paladin-ai-core → paladin-ports → paladin-herald → paladin-battalion → paladin-llm → paladin-memory → paladin-web → paladin-notifications → paladin-content → paladin-storage → paladin-eval → paladin-ai`. **`release-automation.md`'s own "Canonical Publish Order" section is stale** — it still describes the pre-`paladin-eval` eleven-crate order and omits `paladin-eval` entirely (a docs-currency defect, out of this phase's scope per D-14, but worth naming in `37-CI-EVIDENCE.md` as a carried finding). `[VERIFIED: three-way cross-check of live files]`

### Q4 — Sparse-index proof format (D-08)

Path-prefix scheme (from `scripts/publish-crates.sh`'s `_pc_index_path`, verified against live query results this session):

| Name length | Path pattern | Example |
|---|---|---|
| 1 | `1/<name>` | — |
| 2 | `2/<name>` | — |
| 3 | `3/<first-char>/<name>` | — |
| 4+ | `<first-2-chars>/<chars-3-4>/<name>` | `paladin-eval` → `pa/la/paladin-eval` |

JSON-lines record fields observed live: `name`, `vers`, `cksum` (present in the raw body though not printed in the greps above — the schema also carries `deps`, `features`, `yanked`; the pipeline's own `_pc_version_in_index` reads `.vers` and `.yanked` only, via `jq`). `yanked` is boolean; a yanked version still returns `200` on the versioned API endpoint and still counts as "published" (never re-uploadable) but should read `yanked: false` for a healthy v0.10.0 release.

**Caching/propagation delay:** `publish-crates.sh`'s own poll (`_pc_wait_for_index_visibility`) bounds this at a default 180s timeout / 5s interval; Phase 19's evidence log recorded ~25-35s typical publish-to-visible latency.

**Concrete one-liner (curl + jq), driven by the `cargo metadata` crate list, that yields `name, version, cksum, yanked` per crate:**

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
`[VERIFIED: path-prefix logic and User-Agent requirement confirmed by live queries this session + scripts/publish-crates.sh read]`

### Q5 — The seven re-seal gates (D-06)

| # | Gate | Exact command | Defined at | Local or CI-only | Runtime | Prior evidence |
|---|------|---------------|------------|-------------------|---------|-----------------|
| 1 | MIGRATION.md no-TBD + §9.2↔allowlist set-equality | `grep -c TBD MIGRATION.md` (want `0`; **note: `grep -c` with a true zero-count exits 1, not 0** — script accordingly, e.g. `grep -c TBD MIGRATION.md; [ "$?" != "2" ]` or capture output and compare to `0` rather than relying on exit code) AND `make check-migration-allowlist` (→ `scripts/check-migration-allowlist.sh`) | `Makefile:201-203` | Local | seconds | `33-CI-EVIDENCE.md` row 3-4 (both green, "0" TBD, 15 pairs set-equal) |
| 2 | `v0_9_config_boot` | `cargo test --features web-server --test v0_9_config_boot` | `tests/integration/v0_9_config_boot_test.rs`; also runs inside CI's `e2e-platform-api` job (`ci.yml:1341-1361`, asserts ≥9 tests selected) | Local AND CI | ~1-2 min | `29-CI-EVIDENCE.md` row 8 (9 passed); `33-CI-EVIDENCE.md` row 6 (same 9) |
| 3 | OpenAPI golden diff | `cargo test -p paladin-web --test openapi_golden_v0_9` | `crates/paladin-web/tests/openapi_golden_v0_9.rs` (not a standalone CI job — rides inside the normal `paladin-web` test sweep in the default/`web-server` feature `Build & Test` jobs) | Local AND CI | seconds | `29-CI-EVIDENCE.md` row 9 (6 passed); `33-CI-EVIDENCE.md` row 7 (7 passed — one more test added since Phase 29, not a regression) |
| 4 | `cargo semver-checks` | `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` for each of the **11** pre-existing publishable crates (paladin-eval deliberately excluded, no 0.9.0 baseline — ADR-0048) | `ci.yml:314-370` (job `semver`, "Semver Checks (vs v0.9.0)") + `.cargo/semver-checks-allowlist.toml` | Local AND CI | ~1-2 min for all 11 | `29-CI-EVIDENCE.md` row 10 (11/11 pass); `33-CI-EVIDENCE.md` rows 8-18 (11/11 pass, identical shape) |
| 5 | MSRV job | `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets --locked` (drop `--locked` to match CI's own `cargo check --workspace --all-features --all-targets` exactly, `ci.yml:279-280`) | `ci.yml:262-280` (job `msrv`, "MSRV (Rust 1.88)") | Local AND CI | ~3-5 min | `29-CI-EVIDENCE.md` row 11 (3m34s); `33-CI-EVIDENCE.md` row 19 (4m50s) |
| 6 | `make publish-dry-run` in dependency order | `make publish-dry-run` (→ `release-check` then `cargo publish --workspace --dry-run`) | `Makefile` `publish-dry-run` target (comment: "all twelve publishable crates") | Local only (CI-analogous job is `Publish Dry Run`, tag/manual-gated, skips on normal pushes) | several minutes (runs full `release-check` first: fmt+clippy+test+doc-test+audit+build-release) | `29-CI-EVIDENCE.md` row 14 (12/12); `33-CI-EVIDENCE.md` row 20 (12/12, 0 test failures) |
| 7 | CHANGELOG.md `[0.10.0]` complete | `grep -c '^## \[Unreleased\]' CHANGELOG.md` (want `0`) + spot-check content grep, e.g. `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci '<topic>'` per major addition since the last re-seal (Phases 34-36.1) | N/A — grep-based judgment, no dedicated script | Local | seconds | `33-CI-EVIDENCE.md` rows 27-31 (0 Unreleased section; topic greps for RAG/TokenUsage/Commissary all non-zero) |

**Bonus finding — `make check-gates` bundles gate 1 plus six siblings** (`check-changelogs`, `check-crate-names`, `check-advisory-register`, `check-workflow-suppressions`, `check-workflow-triggers`, `check-codeql-dismissals`, `check-migration-allowlist` — `Makefile:220`); running it once covers gate 1 plus useful adjacent proof for gate 7's changelog-coverage claim (`33-CI-EVIDENCE.md` row 5).

**`29-ACCEPTANCE-AUDIT.md` structure (critical for D-06):** this file is a **pointer**, not the corpus itself — the real acceptance audit lives at `.project/v0.10.0/09-program-acceptance-audit.md`. The pointer file already records a prior re-seal: "Re-sealed on `69500c9b…`, 2026-09-16… Phase 33 (COMM-04) re-ran the full release-gate list… appended the result as a new `## 11. Re-seal after Phases 30-33 (Phase 33, COMM-04)` section to the corpus document." **D-06's new dated section for this phase must therefore be `## 12. Re-seal for v0.10.0 release (Phase 37, SHIP-05)`**, appended to the **corpus document** `.project/v0.10.0/09-program-acceptance-audit.md` (not to the phase-directory pointer file, which itself gets a one-line update pointing at the new section number). The §11 human sign-off box referenced by D-00a/D-02/D-03 is inside that same corpus document's own `## 11.` section text (the pointer file's line 28 says "§11 adds one further unticked, human-only sign-off box… alongside — never in place of — the seven boxes named above" — i.e., there are already seven pre-existing sign-off checkboxes in section 10, plus one more added at section 11; Phase 37 either reuses that same §11 box (if it is *the* v0.10.0-tag-cut gate, worded generally enough) or needs to confirm with the maintainer whether a fresh checkbox is warranted for the Phase 37 re-seal specifically. **This needs a direct read of `.project/v0.10.0/09-program-acceptance-audit.md` sections 10-11 during planning** — this research call did not read that corpus file directly (out of the phases/*.md tree); flagging as an `Open Question` below rather than asserting its exact content. `[VERIFIED: pointer file read in full; CITED for corpus file's exact §10/§11 wording — not directly read this session]`

### Q6 — Branch and CI reality check (read-only, this session)

- **`git fetch origin` result:** `HEAD` (`feature/phase-33`, local) = `79c6620c`. `origin/main` = `8ed14aea` (`Merge pull request #54 from DF3NDR/feature/phase-26`). `git rev-list --left-right --count origin/main...HEAD` → `0  490`: **0 commits in `origin/main` not in `HEAD`; 490 commits in `HEAD` not in `origin/main`.** CONTEXT.md recorded 488 at discussion time (2026-09-18) — two more commits have landed since (consistent with active work; not a discrepancy).
- **Unpushed commits:** `git status -sb` reports `feature/phase-33...origin/feature/phase-33 [ahead 14]` — 14 local commits not yet on the branch's own remote tracking ref.
- **No commits in `origin/main` not in `HEAD`** (`git log --oneline HEAD..origin/main` returned empty) — **"main moved" risk is currently zero**, but this is a point-in-time fact that must be re-checked at execution time (CONTEXT.md's own Discretion item 6 anticipates this).
- **Open PR:** `gh pr list --head feature/phase-33 --state all` returned empty — **no PR exists yet**, matching CONTEXT.md's "no open PR" observation.
- **MinIO CI-fix presence (D-15's named real-failure risk):** confirmed present on `feature/phase-33` — `ci.yml:681,1393` and `docker/docker-compose.yml:23`, `docker/docker-compose.test.yml:27`, `k8s/minio.yaml:24` all pin `quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772` (not the retired Docker Hub image), and `ci.yml:724,1443` fetch the `mc` client via `curl -fsSL --retry 5 --retry-delay 5 --retry-all-errors` from an archived GitHub release, not `dl.min.io`. **A red caused specifically by the retired MinIO image will NOT occur on this branch** — any MinIO-adjacent red on the pre-merge PR CI run is therefore either a genuine new failure (D-14 stop) or an unrelated transient (D-15's flake path, evaluated on its own log evidence).
- **Required status checks on `main`:** `protect-main-branch.json` (ruleset id `20868126`, confirmed live via `gh api /repos/DF3NDR/paladin-dev-env/rulesets`) requires **44 named status-check contexts** (per `docs/src/appendix/branch-protection.md`, itself verified against the live API at the time it was written) — `Docker Build` and `Kubernetes Smoke Test` are the two deliberately-excluded non-required jobs. No bypass actor exists on this ruleset — every merge, including a maintainer's, must go through a green PR.
- **Allowed merge methods:** live repo settings (`gh api repos/DF3NDR/paladin-dev-env`) report **all three** merge methods enabled (`allow_merge_commit: true, allow_rebase_merge: true, allow_squash_merge: true`) — **the ruleset does not itself force "merge commit" as the only allowed method; D-04's "merge commit, not squash, not rebase" is a project-decision constraint the maintainer must apply manually when clicking merge (or the agent must pass `--merge` explicitly if it opens/merges via `gh pr merge`, though D-02 reserves the actual merge action for the maintainer).** The plan's hand-off text to the maintainer must say "Merge with the merge-commit method (not squash, not rebase)" explicitly, since the repo would otherwise happily accept any of the three.
- **`delete_branch_on_merge: true`** is the live repo setting — consistent with CONTEXT.md's Discretion item 4 ("leave it to the maintainer's GitHub setting").
- **Pre-push hooks that D-01's push triggers** (`.pre-commit-config.yaml`, `stages: [pre-push]`, all `always_run: true`): `cargo-fmt-push` (fmt --check), `cargo-clippy` (`cargo clippy --workspace --all-targets --all-features -- -D warnings` — this is the "always_run, 2-5min" hook named in known-environment-facts), `cargo-build-push` (`cargo build --workspace`), `cargo-test-lib-push` (`cargo test --workspace --lib`), `check-doc-examples`, `check-doc-config`, `check-api-surface` (`./scripts/check-api-surface.sh .project/current-exports.txt`, ~95s per its own inline comment). **Total pre-push wall time is plausibly 10+ minutes** given clippy alone is 2-5 min and a full workspace build/test/api-surface stack follows it — this matches the "known environment facts" note that `git push` here is slow because of pre-push hooks, not a hang. The plan should budget for this, not treat a long-running `git push` as stuck.

### Q7 — Evidence-file house forms

- **`37-CI-EVIDENCE.md`** should mirror `29-CI-EVIDENCE.md`/`33-CI-EVIDENCE.md`'s exact two-part shape: a numbered **Local sweep** table (`#`, `Command`, `Result`, `Verdict`) followed by a **CI-run table** (`Workflow`, `Run ID (URL)`, `Conclusion`, `SHA`, `Notes`), each closing with a "Summary and what remains" prose section. Given D-05/D-09's two-wave structure, this phase's version needs **more** than two run-table entries: pre-merge PR-head run(s), the run on the post-§11-tick final SHA (D-03), the (skipped, per Q1) dry-run dispatch attempt with its documented non-dispatch rationale, the post-merge run on the tagged `main` merge commit, the real release run itself, and D-08's registry table.
- **v0.9.0 MILESTONES.md entry shape** (`.planning/MILESTONES.md:1-34`): a `## v0.9.0 <name> (Shipped: <date>)` heading, `**Phases completed:**`/`**Requirements:**`/`**Timeline:**`/`**Git range:**`/`**Closeout type:**`/`**Audit:**` metadata lines, then a struck-through amendment paragraph recording that "no tag was cut" was later superseded — with the real tag, merge SHA (`0b5d4106`), release run ID (`33542459191`), crate count (eleven, at the time), and "registry-verified" stated in prose — followed by `**Delivered:**` and `**Key accomplishments:**` narrative sections. **The v0.10.0 entry should follow this shape but need not carry a superseded-strikethrough paragraph** (this milestone is being released cleanly, not retroactively) — model it on the *post-amendment* prose block only: tag, merge SHA, release run ID, crate count (**twelve**, not eleven — a live fact this entry must get right where MILESTONES.md's own v0.9.0 entry, written before `paladin-eval` existed, could not), "registry-verified," Trusted Publishing.
- **The exact STATE.md sentence D-05 says to correct** (found live, `.planning/STATE.md:37-39`): *"Before that, push `feature/phase-33` (or open the PR) so the CI `coverage` job supplies the one gate this devcontainer cannot measure and a real pre-merge run is appended to `33-CI-EVIDENCE.md`'s CI-run table; the tag is cut on the `main` merge commit by `release.yml` per Phase 29 D-21's two-SHA rule."* — the phrase `33-CI-EVIDENCE.md`'s CI-run table` must become `37-CI-EVIDENCE.md`'s CI-run table` (verbatim substring replace; the rest of the sentence is otherwise accurate and should be left alone per amend-at-source discipline. Note this sentence sits inside STATE.md's `## Current Position` prose block, not a table — a plain string replace is sufficient, no markdown-table row surgery needed).

### Q8 — Hard-stop / resume mechanics (D-12)

- **`.continue-here.md` / `/gsd-pause-work`:** per `.claude/gsd-core/workflows/pause-work.md`, the hand-off is written to `.planning/phases/XX-name/.continue-here.md` for phase work (this phase qualifies) alongside a machine-readable `.planning/HANDOFF.json` for `/gsd-resume-work`. The file names the exact resume condition — here, "tag `v0.10.0` exists on `origin` and points at a commit contained in `main`," per D-12's own text — as plain, checkable prose.
- **Checkpoint type for the D-02 merge+tag act:** this is a `checkpoint:human-action` (per `checkpoints.md`'s taxonomy: "Action has NO CLI/API and requires human-only interaction" — here the CLI/API exists in principle, but D-02 *explicitly reserves the act* for the human regardless of automatability, which is the documented exception the reference itself allows for "irreversible or trust-establishing steps a human must actually see" — matching the `gate="blocking-human"` attribute, which is "**never** bypassed... even in auto-mode"). The plan must mark this checkpoint `gate="blocking-human"`, not the default `gate="blocking"`, precisely because `_auto_chain_active: true` is set in this project's `.planning/config.json` — an ordinary `blocking` gate **would auto-approve under this project's live config**, defeating D-02 entirely. **This is a load-bearing planning detail**: absent the explicit `gate="blocking-human"` attribute, the merge+tag checkpoint could be silently auto-approved by the orchestrator's own auto-mode rules. `[VERIFIED: checkpoints.md read in full; config.json confirmed `_auto_chain_active: true` live]`
- **Resuming a phase whose wave 1 is complete:** `/gsd-execute-phase 37` (or the orchestrator's resume path) re-reads the phase's plan files and `STATE.md`/`.continue-here.md`; the first task of wave 2 must explicitly re-verify the resume condition (tag exists, points at a commit contained in `main`) before doing anything else, per D-12's own text — this is a plan-authoring requirement, not something the execution harness enforces automatically.

### Q9 — D-10/D-11 mechanics: `chore/NN-close` precedent

Confirmed precedent in git history, four prior instances: `chore/21-close` (PR #49, merged), `chore/20-close` (PR #47, merged), `chore/9-closeout` (PR #33, merged — note the differing exact naming, "closeout" not "close," a real naming variance across history), `chore/15.1-close-out` (PR #32, merged — "close-out" with a hyphen, a third distinct spelling). **The precedent is consistent in structure (a small branch cut from the tagged/closed state, merged via its own PR) but inconsistent in exact naming convention** — the plan should use `chore/37-close` per D-10's own explicit text rather than trying to match one of the three historical spelling variants. `[VERIFIED: git log --all --grep, four matches]`

`/gsd-audit-milestone` and `/gsd-complete-milestone` preconditions: both workflow files exist at `.claude/gsd-core/workflows/audit-milestone.md` and `.claude/gsd-core/workflows/complete-milestone.md`. A targeted grep for explicit "precondition" language returned no hits in this session's pass — **the planner should do a full read of both files during planning** (not just this grep) to extract the literal preconditions list for Phase 37's final plan to enumerate as its "ready to close" hand-off, since this research pass did not have budget to read both files in full. `[ASSUMED — grep-only pass, not a full read; flagged as an Open Question]`

### Q10 — Runbook lookup table (D-16)

`docs/src/appendix/release-recovery.md` §1-§2 (the agent's read-only scope):

- **§1 "Establishing what actually reached crates.io"**: a for-loop over the twelve crate names (**the runbook's own example loop lists only eleven** — it predates `paladin-eval`, a docs-currency gap consistent with the "eleven crates" staleness theme; the plan should extend the loop to include `paladin-eval`), querying `https://crates.io/api/v1/crates/<name>/<version>` with the mandatory `User-Agent` header, mapping `200`→already-published, `404`→not-yet-published, anything else→investigate.
- **§2 "Reading the run"**: `publish-crates`'s own per-crate outcome table (`$GITHUB_STEP_SUMMARY` + job log) is authoritative; four terminal states (`published-now`, `already-at-this-version`, `skipped`, `failed`). Critically: **the workflow's overall run conclusion can be red for reasons unrelated to publishing** (the Build Binaries matrix has a documented history of failing every observed run) — §2 explicitly warns not to infer publish health from the workflow's overall green/red, only from `publish-crates`'s own table.
- **§3 "Completing forward"** (maintainer-only): re-run the *same* tag's existing workflow run (`gh run rerun <id>`, "Re-run failed jobs" first). Never dispatch a second concurrent run against the same tag.
- **§4 "When completing forward is not enough"** (maintainer-only): a landed version is permanent; the fix is a new patch version plus yanking the bad one.
- **§5 "Who may yank"** (maintainer-only, explicitly): only the crate-owner account, never CI, via `cargo yank`.
- **§6 "When the gate blocks the release"** (agent-diagnosable): the seven failure-code table (`MISMATCH`, `ZERO_PACKAGES`, `MISSING_TAG`, `CHANGELOG_MISMATCH` ×2 variants, `CI_MISMATCH`, `CI_LOOKUP_FAILED`, `MISSING_SHA`, plus `_AND_`-joined combinations) with a named remedy per code — this is effectively part of the agent's read-only diagnosis toolkit too, since it explains *why* the pre-publish gate blocked without prescribing a fix action beyond "read the report."

**Agent's D-16 scope is precisely §1-§2 (establish registry state, read the run) plus consulting §6's lookup table to *name* the failure code** — never §3 (re-dispatch), §5 (yank), or acting on §4's "publish a new patch version" remedy.

## Standard Stack

Not applicable in the conventional sense — this phase adds no library dependencies. The "stack" is entirely the existing release-automation tooling already in the repository:

| Tool | Version | Purpose | Why Standard (already adopted) |
|------|---------|---------|-------------------------------|
| `cargo-release` | installed via `cargo install --locked cargo-release` | Lockstep version bump (NOT used this phase — the bump already landed pre-tag per D-00b) | Adopted Milestone 10 Epic 3, documented in `release-automation.md` |
| `cargo semver-checks` | `0.50.0` (pinned in `ci.yml:336`) | API-break detection vs. published `0.9.0` baseline | Already the CI gate; re-run locally for D-06 gate 4 |
| `rust-lang/crates-io-auth-action@v1` | pinned via `@v1` tag | OIDC token minting for Trusted Publishing | Already the CI mechanism; no change needed except resolving the `paladin-eval` gap (Q3) |
| `gh` CLI | whatever is on the devcontainer PATH | PR open/merge query, run inspection | Already the project's standard automation surface for release acts |

No new package installs, so the Package Legitimacy Audit and version-verification protocol in the standard template are **not applicable** to this phase.

## Package Legitimacy Audit

**Not applicable.** This phase installs no new external packages. `cargo-release`, `cargo-semver-checks`, and `crates-io-auth-action` are all pre-existing, already-audited tooling from prior phases (Milestone 10 Epic 3, Phase 22, Phase 19 respectively) — no new registry/marketplace dependency is introduced by Phase 37's own work.

## Architecture Patterns

### System Architecture Diagram

```
[feature/phase-33, 490 commits ahead of main]
        |
        | (D-06) local re-seal: 7 gates, all-green or hard-stop (D-14)
        v
[D-01: agent pushes branch + opens PR to main] --> [PR CI: 44 required checks
        |                                            including `coverage` >= 82%]
        | (D-03) wait for green, append 37-CI-EVIDENCE.md
        v
[D-00a/D-03: maintainer ticks §11 in 09-program-acceptance-audit.md]
        |
        | tick commit pushed --> CI re-runs on true final SHA
        v
==================== D-02 CHECKPOINT (gate="blocking-human") ====================
  maintainer: merge PR with MERGE COMMIT method (not squash/rebase, D-04)
       |
       | wait for ci.yml green on merge commit (~60-90 min) -- Q2's ordering finding
       v
  Q3 BLOCKER: maintainer manually bootstraps `paladin-eval` Trusted Publishing
  (personal token publish -> configure Trusted Publisher -> revoke token)
       |
       v
  maintainer: git tag -a v0.10.0 <merge-sha> && git push origin v0.10.0
==================================================================================
        |
        | agent resumes: re-verify tag exists + is ancestor of main (D-12)
        v
[Q1: D-13 dry-run dispatch is NOT POSSIBLE pre-tag -- already moot, tag now exists;
     record why it was skipped, per D-13's own fallback text]
        |
        v
[release.yml fires on tag push: verify-tag-source -> test -> create-release
        -> check-release-consistency (needs green ci.yml on merge SHA, Q2)
        -> publish-crates (12 crates, dependency order, real OIDC token)
        -> build-docker / build-binaries / sbom / finalize-release-body]
        |
        | D-16: if red, diagnose read-only via release-recovery.md §1-2/6, hard-stop
        v
[D-08: registry verification -- sparse-index query per crate, 12/12 at 0.10.0]
        |
        v
[MILESTONES.md v0.10.0 entry written; phase verifies SC1-4 + SC5 preconditions]
        |
        v
[D-11: phase ends "ready to close" -- maintainer later runs
        /gsd-audit-milestone + /gsd-complete-milestone on chore/37-close]
```

### Recommended Plan/Wave Structure

```
Wave 1 (pre-merge):
  Plan A: local re-seal sweep (D-06's 7 gates) -> 37-CI-EVIDENCE.md Local sweep
  Plan B: mint SHIP-05 (REQUIREMENTS.md + traceability + ROADMAP), STATE.md
          sentence fix, forward-pointer lines in 29/33-CI-EVIDENCE.md
  Plan C: push branch, open PR (D-01), append 37-CI-EVIDENCE.md CI-run table
          entries as they land; surface the paladin-eval Trusted-Publishing
          gap (Q3) as an early checkpoint:human-action, ideally resolved
          during this wave rather than discovered at tag time
  Plan D: append D-06's dated re-seal section to the corpus acceptance audit;
          checkpoint:human-action for the §11 tick (D-00a/D-03)
  --- D-02 checkpoint:human-action, gate="blocking-human" ---
Wave 2 (post-tag):
  Plan E: re-verify resume condition (D-12); confirm real release run outcome
          (D-16 diagnosis path if red); append post-merge/post-tag CI-run
          rows and the D-08 registry table to 37-CI-EVIDENCE.md
  Plan F: write MILESTONES.md v0.10.0 entry; list /gsd-audit-milestone and
          /gsd-complete-milestone preconditions; phase ends "ready to close"
```

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Registry-state checking | A custom crates.io polling script | `scripts/publish-crates.sh`'s existing `_pc_crate_published`/`_pc_version_in_index` functions, or the exact curl+jq one-liner in Q4 | Already handles the `User-Agent` 403 trap, yanked-version semantics, and 429 backoff — reinventing it risks missing the `User-Agent` requirement (a documented incident-time trap per `release-recovery.md`'s own text) |
| Release-gate rehearsal without a tag | A CI workflow modification to accept a bare SHA | Nothing — this is explicitly deferred to a future milestone per CONTEXT.md's own Deferred Ideas list | Q1 already proves the current design cannot do this safely; changing `release.yml` mid-phase would be exactly the kind of "fix nothing" violation D-14 prohibits |
| Publish-order derivation | A hand-maintained crate list in the plan | `cargo metadata --no-deps --format-version 1 \| jq '.packages[] \| select(.publish == null)'` | D-08 explicitly mandates deriving the list this way; the doc's own "eleven" figure is proven stale by this exact mechanism |

**Key insight:** every mechanism this phase needs already exists in the repository, proven working by two prior phases (29, 33) against this exact tree shape. The work is almost entirely evidence capture and human hand-off sequencing, not tool-building.

## Runtime State Inventory

Not applicable — this is not a rename/refactor/migration phase. No code strings, config keys, or identifiers are being renamed.

## Common Pitfalls

### Pitfall 1: Treating the D-13 dry run as a checklist item to "just try"
**What goes wrong:** dispatching `release.yml` with a `tag` input that doesn't resolve as a real ref fails `verify-tag-source` outright; dispatching with a resolvable-but-wrong string (an old tag, a SHA, a branch) fails `create-release` or `check-release-consistency` for unrelated reasons, producing a confusing red run that looks like a release-pipeline bug rather than an input-design constraint.
**Why it happens:** the `tag` input is used, unmodified, as both "which commit to check out" AND "what version string to expect in the CHANGELOG/manifests" — two different concerns collapsed into one string.
**How to avoid:** do not attempt the dispatch at all before the real tag exists (Q1's proof). Record the reasoning once, cite D-13's own fallback text, and move directly to the real tag.
**Warning signs:** any temptation to "just try it and see" — every candidate input string produces a distinct, misleading failure mode rather than a clean "not supported" message.

### Pitfall 2: Tagging immediately after the merge, before `ci.yml` finishes
**What goes wrong:** `check-release-consistency`'s CI-conclusion clause reports `CI_MISMATCH` (a real, not-yet-successful run) if the tag lands before `ci.yml`'s ~60-90 minute run on the merge commit completes.
**Why it happens:** the merge and the tag are two separate human acts (D-02) with no automatic sequencing between them; a maintainer who tags right after clicking "merge" has not waited for CI.
**How to avoid:** the D-02 hand-off text must explicitly instruct "wait for `ci.yml` to conclude success on the merge commit before tagging," not just "merge, then tag."
**Warning signs:** a `check-release-consistency` job reporting `CI_MISMATCH` shortly after a tag push — the recorded recovery (§3, re-run the same release run once CI catches up) is not a failure of the release, just premature tagging.

### Pitfall 3: Assuming the real release run will cleanly publish all twelve crates
**What goes wrong:** `paladin-eval` has never been published; the real run will fail at it unless the maintainer bootstraps Trusted Publishing for it first (Q3).
**Why it happens:** every other crate already has a working Trusted Publisher link from Phase 19's original eleven-crate rollout; `paladin-eval` postdates that rollout and was never carried through the same one-time bootstrap.
**How to avoid:** surface this as an explicit, early checkpoint — do not let it surface for the first time as a red `publish-crates` job on the real, tag-triggered run.
**Warning signs:** `paladin-eval` absent from `release-automation.md`'s Trusted Publishing table (already true today); a 404 on `index.crates.io/pa/la/paladin-eval` (already true today, verified this session).

### Pitfall 4: `grep -c` zero-count exit-code trap (named in the Nyquist requirement, applies directly to gate 1)
**What goes wrong:** `grep -c TBD MIGRATION.md` returning `0` (the desired, passing state) causes `grep` to exit with status `1`, not `0` — a naive `grep -c TBD MIGRATION.md || fail` or a bare `set -e` script would treat the PASSING case as a script failure.
**Why it happens:** `grep -c` reports the match count on stdout regardless of exit code, but its exit code still follows ordinary grep semantics (0 = at least one match, 1 = no matches).
**How to avoid:** capture the count into a variable and compare the *value* to `0`, never rely on the exit code alone, exactly as `check-release-consistency.sh` itself does elsewhere (`{ grep -E ... || test $? -eq 1; }` idiom, used repeatedly in `ci.yml`'s own allowlist-verification step).
**Warning signs:** a locally-green gate 1 that then fails under `set -euo pipefail` when wired into a script — this exact idiom is already documented as a live-discovered trap in `ci.yml`'s own inline comments (`ci.yml:410-415`).

## Code Examples

### Deriving the publishable crate list (never hard-coded, per D-08)
```bash
# Source: scripts/check-release-consistency.sh's own clause-1 logic, verified
# live this session against this workspace's actual Cargo.toml files
cargo metadata --no-deps --format-version 1 | python3 -c "
import json, sys
data = json.load(sys.stdin)
for p in data['packages']:
    if p.get('publish') is None:   # publish=false crates report a non-null list
        print(p['name'], p['version'])
"
```

### The D-06 gate-1 zero-count trap, avoided
```bash
# Source: ci.yml's own idiom (lines ~410-415), applied to gate 1
COUNT=$(grep -c TBD MIGRATION.md || true)
if [ "${COUNT}" != "0" ]; then
  echo "::error::MIGRATION.md still has ${COUNT} TBD marker(s)"
  exit 1
fi
```

### Registry verification one-liner (D-08), reused from Q4

See Q4 above for the full curl+jq loop.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|---------------|--------|
| `release-automation.md`'s "Canonical Publish Order" section (eleven crates, no `paladin-eval`) | `scripts/publish-crates.sh`'s actual `CRATES` array (twelve, `paladin-eval` at position 11) | Phase 28 (ADR-0048) added `paladin-eval`; the doc was never updated | The doc is stale and should not be trusted as the source of publish order — always read the script or `cargo metadata` |
| `docs/src/contributing/development-setup.md`'s "eleven crates" figure | Twelve, confirmed live this session | Same — Phase 28 | Per CONTEXT.md's own Deferred Ideas, correcting this doc page is explicitly out of scope for Phase 37; record as a carried finding, do not fix |
| `release-automation.md`'s Trusted Publishing per-crate table (eleven rows) | Should be twelve once `paladin-eval` is bootstrapped | Not yet happened | This is the Q3 blocker — the table's staleness is itself evidence the gap has never been closed |

**Deprecated/outdated:** none in the conventional sense — every mechanism here is current; the staleness is entirely in prose documentation that lags a code/registry change (`paladin-eval`'s addition), not in tooling that needs replacing.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | crates.io Trusted Publishing categorically cannot be configured for a crate before its first manual publish | Q3 | If wrong (crates.io shipped a "pending publisher" feature since the cited sources), the D-00g-constrained blocker may not exist — but two independent sources agree as of this session, and the repository's own `release-automation.md` table (no `paladin-eval` row) is consistent with the blocker being real. Re-verify against `crates.io/docs/trusted-publishing`'s live rendered page (JS-rendered; this session's fetch could not read its body) before the plan finalizes this as fact. |
| A2 | The theoretical non-`v`-prefixed shadow-tag workaround for Q1 would actually work as traced | Q1 | Untested by design (tag creation is forbidden in this research task). If wrong, it doesn't matter — the recommendation is to NOT use it regardless, in favor of D-13's own documented fallback. |
| A3 | `.project/v0.10.0/09-program-acceptance-audit.md` sections 10-11's exact sign-off-box wording/count | Q5 | If the existing §11 box is scoped narrowly enough to not cover a Phase 37 re-seal, the plan needs a fresh §12 box rather than reusing §11 — this must be confirmed by directly reading the corpus file during planning, which this research pass did not do (out of the `.planning/phases/*` tree this agent's file-reading focused on). |
| A4 | `/gsd-audit-milestone` and `/gsd-complete-milestone`'s exact precondition list | Q9 | A grep-only pass found no explicit "precondition" keyword hits; the planner must read both workflow files in full rather than relying on this research's partial pass. |

## Open Questions

1. **Does `.project/v0.10.0/09-program-acceptance-audit.md`'s existing §11 sign-off box cover a future re-seal, or does Phase 37 need its own fresh checkbox?**
   - What we know: the pointer file (`29-ACCEPTANCE-AUDIT.md`) says §11 was added specifically for "cutting the v0.10.0 tag" and describes it as "alongside — never in place of — the seven boxes" in §10.
   - What's unclear: whether §11's box, once ticked, is a one-time gate for *any* future re-seal-then-tag cycle, or whether Phase 33's re-seal already consumed it and Phase 37 needs a new one.
   - Recommendation: read the corpus file directly during planning (not deferred to execution) and confirm with the maintainer which box gets ticked, before the plan commits to exact wording in its D-02 hand-off.

2. **Exact preconditions `/gsd-audit-milestone` and `/gsd-complete-milestone` require, for the phase's final "ready to close" plan.**
   - What we know: both workflow files exist; four `chore/NN-close` precedents show the branch-and-PR shape.
   - What's unclear: the literal precondition checklist each command enforces (this research pass did a grep-only, not a full read).
   - Recommendation: full-read both files during planning; enumerate the exact preconditions as the final plan's task list.

3. **Whether the maintainer should bootstrap `paladin-eval`'s Trusted Publishing before or after the merge, but definitely before the tag.**
   - What we know: it must happen before the real tag-triggered `publish-crates` job reaches `paladin-eval` in dependency order (position 11 of 12).
   - What's unclear: whether doing it pre-merge (so it's visible in the PR) or post-merge-pre-tag (closer to when it's needed) is better project hygiene; either satisfies the hard constraint.
   - Recommendation: default to surfacing it as early as possible (pre-merge wave) so it isn't a same-day scramble during the D-02 checkpoint window; let the maintainer decide timing.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` / rustc toolchain | All seven D-06 local gates | ✓ | workspace-pinned (`rust-toolchain.toml`) + MSRV 1.88 installable via `dtolnay/rust-toolchain@1.88` | — |
| `gh` CLI | PR open (D-01), run/ruleset inspection | ✓ (used throughout this research session) | whatever is on PATH | — |
| Docker | `coverage` job's Redis/MinIO services, `make coverage` locally | ✗ (`docker: command not found`, reconfirmed by prior phases' own evidence files) | — | D-00e: measured by CI's `coverage` job only, never locally |
| `python3` | `check-release-consistency.sh`'s metadata parsing | ✓ (implied by successful `cargo metadata \| python3` runs in prior evidence and this session) | — | — |
| `jq` | Sparse-index parsing (Q4) | ✓ (used successfully this session) | — | — |
| Network egress to `index.crates.io` / `crates.io` | D-08 registry verification, Q3 investigation | ✓ (used successfully this session, read-only) | — | — |
| Network egress to GitHub API (`gh api`) | Q6's ruleset/repo-settings checks | ✓ (used successfully this session) | — | — |

**Missing dependencies with no fallback:** none — Docker's absence has an explicit, already-adopted fallback (D-00e).

**Missing dependencies with fallback:** Docker (fallback: CI-attributed coverage measurement, per D-00e and every prior phase's identical posture).

## Validation Architecture

> This phase's "tests" are deterministic evidence checks over release-mechanics artifacts, per the Nyquist requirement's own framing — not unit tests over application code.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | None (no test framework) — evidence is grep/jq/curl over files and `gh api`/registry responses |
| Config file | none |
| Quick run command | Individual gate commands from Q5's table |
| Full suite command | `make check-gates` (offline guards) + the seven Q5 commands + `make publish-dry-run` |

### Success Criteria → Evidence Map

| SC / Requirement | Behavior | Proof command | Exit-code trap to avoid |
|---|---|---|---|
| SC1 (seven gates re-sealed) | Every D-06 row green on the final commit | Run each of Q5's seven commands; capture output into `37-CI-EVIDENCE.md` | Gate 1's `grep -c TBD` — see Pitfall 4; compare captured value to `"0"`, not exit code |
| SC1 (§11 ticked) | Maintainer-only; agent cannot prove this, only observe it | `grep -c '\[x\]' <corpus-file-section>` for the specific box, AFTER the maintainer confirms — this is human-only per D-00a (§11 tick observability, not proof of correctness) | — |
| SC2 (coverage ≥82% on PR CI) | CI-only, per D-00e | `gh run view <run-id> --json jobs --jq '.jobs[] | select(.name=="Coverage") | .conclusion'` once the PR CI run exists; cross-check the coverage percentage from the job's `$GITHUB_STEP_SUMMARY` artifact or log | Only observable post-PR-push (Wave 1); cannot be proven before D-01 |
| SC3 (merge commit + tag on it) | `main` fast-forward not used; tag sits on the merge SHA | `git merge-base --is-ancestor <tag-sha> origin/main && git rev-parse <tag>^{commit}` equality check against the recorded merge SHA | Both checks needed — ancestry alone doesn't prove *the tag's own commit* is the merge commit rather than some other ancestor |
| SC3 (`release.yml` green) | The real, tag-triggered run concludes with `publish-crates` green (not the whole-workflow conclusion, per §2's own warning) | `gh run view <run-id> --json jobs --jq '.jobs[] | select(.name=="Publish to crates.io") | .conclusion'` | Do NOT use the workflow's overall conclusion — Build Binaries has a documented history of failing every observed run without affecting publish health |
| SC4 (12/12 crates at 0.10.0) | Sparse-index query per crate | Q4's curl+jq loop, asserting the resulting line count equals `12` (`cargo metadata`-derived count, never hard-coded) | A loop that silently produces fewer than 12 lines (a crate 404s) must be caught by counting output lines, not by "the loop completed without crashing" |
| SC5 preconditions (ready to close) | MILESTONES.md entry written; `/gsd-audit-milestone`/`/gsd-complete-milestone` preconditions listed | Manual read-through against Open Question 2's answer, obtained during planning | — |
| SHIP-05 minted | New row in REQUIREMENTS.md + traceability table + ROADMAP line replaced | `grep -c 'SHIP-05' .planning/REQUIREMENTS.md` (want ≥2: definition row + traceability row) and `grep -c 'Requirements.*TBD' .planning/ROADMAP.md` scoped to the Phase 37 section (want 0 — subject to the same zero-count exit-code trap as Pitfall 4) | Same `grep -c` zero-trap applies to the ROADMAP check |

### Sampling Rate
- **Per plan/task:** the specific gate(s) that plan touches (e.g., the REQUIREMENTS.md-minting plan checks `grep -c SHIP-05`, not the whole D-06 gate set).
- **Per wave merge:** the full D-06 seven-gate sweep, re-run fresh (never reused from an earlier wave — the tree changes between waves).
- **Phase gate:** all of SC1-SC4 plus SC5's preconditions, per D-09's "verified only when its criteria are observably true" standard — no criterion may be marked passed "by promise."

### Wave 0 Gaps
None — no test-framework infrastructure is needed; every proof command in the table above already exists and has been exercised at least once in Phase 29 or Phase 33's own evidence files, or (for SC3/SC4) is a live `gh`/registry query already demonstrated working in this research session.

## Security Domain

> `security_enforcement` is not set to `false` in `.planning/config.json` (the key is absent), so this section is included per the default-enabled rule. This phase is release mechanics, not application code — the applicable threat surface is narrow.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-------------------|
| V2 Authentication | No | This phase touches no application authentication code |
| V3 Session Management | No | — |
| V4 Access Control | Partially | The GitHub branch/tag rulesets ARE the access-control mechanism in scope — already externally enforced (Q6), not something this phase implements |
| V5 Input Validation | Partially | `check-release-consistency.sh`'s tag-string handling (Q1/Q2) is itself an input-validation surface already hardened by prior phases (CR-01's env-indirection convention, applied throughout `release.yml`) — no new validation code needed this phase |
| V6 Cryptography | No | No new cryptographic code; OIDC token exchange is entirely handled by `rust-lang/crates-io-auth-action@v1`, an already-adopted, already-audited dependency |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|----------------------|
| A standing crates.io credential reintroduced to unblock `paladin-eval` (Q3) | Elevation of Privilege / Information Disclosure | D-00g forbids this outright; the only sanctioned path is the maintainer's own manual bootstrap-then-revoke, mirroring Phase 19's exact precedent — never a repository secret, never an agent-held token |
| A tag pushed before CI is green on that commit (Q2) | Tampering (of the release-integrity invariant, not a security exploit per se) | `check-release-consistency`'s CI-conclusion clause already catches this; the mitigation is procedural (wait for CI) not code |
| Command injection via a tainted `tag`/version string flowing into `run:` blocks | Tampering | Already mitigated project-wide via the documented CR-01 convention (every tainted value routed through `env:` indirection, never inlined into a `run:` body) — verified present in every `release.yml` step read during this research |
| A second concurrent release run racing the same tag (`release-recovery.md`'s explicit warning) | Denial of Service / Tampering | Procedural: never dispatch/re-run while another run against the same tag is in flight; check `gh run list` before any re-run action |

## Sources

### Primary (HIGH confidence)
- `.github/workflows/release.yml` — full read, every job and step relevant to `tag`/`dry_run` inputs traced
- `.github/workflows/ci.yml` — `msrv`, `semver`, `e2e-platform-api`, `coverage` job definitions (partial read, targeted sections)
- `scripts/check-release-consistency.sh` — full read
- `scripts/publish-crates.sh` — full read
- `docs/src/appendix/release-automation.md`, `release-recovery.md`, `branch-protection.md`, `release-checklist.md` — full reads
- `.planning/decisions/0048-paladin-eval-composition-crate.md` — full read
- `.planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md`, `29-CI-EVIDENCE.md` — full reads
- `.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md` — full read
- `.planning/MILESTONES.md` (v0.9.0 entry), `.planning/STATE.md`, `.planning/REQUIREMENTS.md`, `.planning/phases/37-v0-10-0-crate-release/37-CONTEXT.md` — full/targeted reads
- `.claude/gsd-core/references/checkpoints.md`, `.claude/gsd-core/workflows/pause-work.md` — full/targeted reads
- Live `cargo metadata --no-deps --format-version 1` run (this session) — twelve publishable crates confirmed
- Live sparse-index `curl` queries against `index.crates.io` for all twelve crate names (this session) — `paladin-eval` 404 confirmed
- Live `git fetch origin` / `git rev-list` / `git log` / `gh pr list` / `gh api` queries (this session) — branch state, rulesets, merge-method settings confirmed
- `.pre-commit-config.yaml` — full read of pre-push stage hooks

### Secondary (MEDIUM confidence)
- `blog.rust-lang.org/2025/07/11/crates-io-development-update-2025-07` (via WebFetch) — Trusted Publishing first-release policy, quoted directly
- WebSearch synthesis citing `crates.io/docs/trusted-publishing` and RFC 3691 — cross-confirms the same policy claim from a second angle

### Tertiary (LOW confidence)
- None — every claim in this document traces to either a live command run this session or a file read this session, except the two crates.io-policy citations above (Secondary tier) and the four items in the Assumptions Log.

## Metadata

**Confidence breakdown:**
- Release-mechanics tracing (Q1, Q2, Q5, Q6): HIGH — every claim traced to file:line or live command output this session
- crates.io Trusted Publishing policy (Q3): MEDIUM — CITED from two independent web sources, not a direct primary-docs-page fetch (the primary page returned no substantive body)
- Evidence-file/GSD-mechanics questions (Q7, Q8, Q9, Q10): mixed HIGH (file reads) / flagged ASSUMED where noted (A3, A4)

**Research date:** 2026-09-18
**Valid until:** This research is tied to a specific, fast-moving point-in-time repository state (branch ahead-count, registry state, open-PR status) — re-verify Q6's branch/PR facts and Q3's registry state immediately before planning executes, not just before this document is trusted. The release-mechanics tracing (Q1, Q2, Q5) is stable until `release.yml`/`check-release-consistency.sh` themselves change.
