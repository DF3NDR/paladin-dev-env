# Phase 47: v0.11.0 Crate Release - Research

**Researched:** 2026-10-25
**Domain:** Rust workspace lockstep release engineering (cargo-release bump, Keep-a-Changelog stamping, GitHub PR/CI gating, tag-triggered crates.io Trusted Publishing, sparse-index verification)
**Confidence:** HIGH (every mechanism below was exercised against the real tree in this session; the two release blockers were reproduced, not inferred)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

**Carried forward (locked by Phases 29 / 37 / 37.1 — not re-asked)**

- **D-00a:** Only the maintainer ticks a sign-off box — never an agent, under any mode including `--auto` (Phase 29 D-17, Phase 37 D-00a).
- **D-00b:** The maintainer alone performs the irreversible acts — merging the release PR, pushing the `v*` tag, and any `cargo yank`. The agent hands over exact commands and stops; it never merges to `main`, never pushes a `v*` tag, never publishes, never yanks (Phase 37 D-02/D-16, Phase 37.1 D-00b, runbook §5).
- **D-00c:** Tag-cut mechanism: the release PR merged to `main` with a **merge commit** (not squash, not rebase), then an **annotated tag pushed directly to that merge commit** after `ci.yml` is green on it. `make release` is not the path (its trailing push is blocked by the "Protect main branch" ruleset); its component steps are (Phase 37 D-00c/D-04, Operator Guide).
- **D-00d:** Findings are recorded, never fixed silently; amend-at-source — original text retained, additions dated; `WINDOWS.md` rows are never deleted (Phase 29 D-12/D-24).
- **D-00e:** The 82 % workspace line-coverage floor (ADR-0006) is measured by CI's `coverage` job only.
- **D-00f:** Shipped tree outranks any document. The publishable-crate list is whatever `cargo metadata` says (`publish != false`), never hard-coded (Phase 37 D-00f/D-08).
- **D-00g:** Publishing is via crates.io Trusted Publishing; no standing registry credential exists and nothing in this phase may introduce or request one. All twelve crates, `paladin-eval` included, are linked — `0.10.1` was their last pipeline publish.
- **D-00h:** Registry proof is a sparse-index (`index.crates.io`, mandatory `User-Agent`) read per publishable crate — crate, version, checksum, `yanked` — recorded as a table. The workflow's own log is not accepted as proof on its own (Phase 37 D-08).
- **D-00i:** Waiting is a hard stop with a resume file naming the exact resume condition — never a parked watcher or a long-lived `gh run watch` (Phase 37 D-12).
- **D-00j:** Any red gate from the start of the re-seal onward — locally or in PR CI — is recorded (command, SHA, failing output) and the agent halts at a blocking checkpoint and fixes **nothing** (Phase 37 D-14). The one exception is a pure infrastructure flake, re-run **exactly once** with both run IDs and the justifying log line recorded (Phase 37 D-15).
- **D-00k:** `release.yml` cannot be dry-run-dispatched before a ref literally named `v0.11.0` exists (Phase 37 research Q1). No dispatch rehearsal, no rc tag, no shadow tag.
- **D-00l:** Post-tag commits reach the protected `main` through a `chore/47-close` branch cut from the tagged `main` and a small docs-only PR the maintainer merges; the tagged commit stays exactly what was released; `/gsd-complete-milestone` is not run by any plan (Phase 37 D-10/D-11).
- **D-00m:** The phase stays open across the merge: a **build wave** (bump, stamps, regen), a **pre-merge wave** (re-seal, PR, evidence, sign-off), the maintainer's **merge + tag checkpoint**, and a **post-tag wave** (registry verification, records, hand-offs). Verified only on what is observably true, never by promise (Phase 37 D-09).
- **D-00n:** `CHANGELOG.md` heading dates equal the tag date (Phase 46 D-13). MIGRATION §10 is frozen at `e28e43b5` and is re-read only if a code change after that SHA moves the consumer surface (Phase 46 D-03).

**Branch & PR shape**

- **D-01:** **One PR.** The bump commit set lands as the last commits on `claude/laughing-dirac-e0h2ax`; the already-open **PR #59 is the release PR** — retitled `release: v0.11.0 — Treasurer Spend Governance` (wording at the planner's discretion) with a curated body; its single merge commit carries the whole milestone and is the tag target. This is the v0.10.0 precedent (PR #55 from `feature/phase-33`). Rejected: a second `release/v0.11.0` PR off `main` (Operator Guide pattern) — it would put unreleased 0.11 code on `main` under a `0.10.1` version and cost a second sign-off cycle. — **Reversibility:** one-way — the merge commit on a protected `main` becomes the tag target and cannot be rewritten after the tag points at it.
- **D-02:** **`main` is frozen** at `df24689c` until the release PR merges; #59 is the only merge before the tag, so its diff is exactly "everything since `v0.10.1`". If `main` moves anyway, the branch merges `main` in (merge commit, never rebase), the re-seal restarts from the top on the new head, and the maintainer re-confirms the sign-off against the new SHA.
- **D-03:** The **agent retitles and rewrites the PR #59 body via `gh api` (REST only — GraphQL is blocked in this sandbox)**: re-seal summary, pointer to `CHANGELOG.md` `[0.11.0]`, `MIGRATION.md` §10, `47-CI-EVIDENCE.md` and `47-ACCEPTANCE-AUDIT.md`, and the twelve-crate list derived from `cargo metadata`. Fallback for a read-only token: hand the exact command to the maintainer.
- **D-04:** The branch **keeps its name** `claude/laughing-dirac-e0h2ax`. No `release/*` branch is created (renaming would close #59 and lose its thread; every STATE.md "pushed on `claude/laughing-dirac-e0h2ax`" citation stays true; the `release/*` one-push ruleset is not engaged).

**Bump mechanics & gate scope**

- **D-05:** The bump is applied with **`cargo release version 0.11.0 --execute --no-confirm --workspace`** (install `cargo-release` first; `release.toml` already configures lockstep and pin updates — Operator Guide step 2). Then: diff every `Cargo.toml`, and a **grep-zero check for `0.10.1`** across manifests and the workspace-dependency pins (historical CHANGELOG/MIGRATION/ADR text excluded). `crates/doc-examples` is `publish = false` — if the tool skips it, its `version` and its nine `paladin-*` pins are hand-touched in the same commit. Thirteen manifests plus the root's own `paladin-llm`/`paladin-eval` entries carry the version today (`Cargo.toml` lines 34-43, 58, 136, 229, 344). — **Reversibility:** costly — a missed pin reproduces the v0.10.0 class of defect; the publish-order and release-consistency gates catch it only on the final head.
- **D-06:** The twelve changelogs are stamped with **`make finalize-crate-changelogs VERSION=0.11.0`** (`scripts/finalize-crate-changelogs.sh`, root included), dated to the tag date (D-00n). `check-changelogs` and `check-release-consistency` are re-run locally afterwards. The root `[0.11.0]` section is hand-curated only if the stamp leaves it unreadable; its content is what Phases 38-51 and 46 wrote under `[Unreleased]`.
- **D-07:** **`make openapi`** regenerates `crates/paladin-web/openapi.json` after the bump; the **expected diff is the version string only**. Any other change means a route moved after the Phase 46 freeze — it is recorded as a finding and the agent stops (D-00j); it is not absorbed into the release.
- **D-08:** The **build wave fixes nothing outside the bump commit set itself** (manifest pins, the stamp output, the OpenAPI version string). Red here is handled as follows: a defect in the bump itself is corrected; anything else — a failing test, a lint, a drifted golden — is recorded and put to the maintainer. From the moment the re-seal starts on the candidate final head, D-00j applies unchanged.
- **D-09:** CI's `cargo-semver-checks` job stays pinned to **`--baseline-version 0.9.0`**; its known no-op state (Phase 38 plan 38-09 finding: `0 checks, 254 skip` for every package; `paladin-eval` excluded) is **recorded as a deferred release-tooling finding in `47-CI-EVIDENCE.md`**, with the recommendation to re-pin to `0.11.0` and add `paladin-eval` in the next milestone's first phase. No `ci.yml` edit on release day. Rejected: re-pinning to `0.10.1` in the build wave (fires on every registered 0.11 break and widens the release PR into CI and allowlist work).

**Sign-off home & re-seal table**

- **D-10:** The v0.11.0 re-seal table and the sign-off box live in a **new `47-ACCEPTANCE-AUDIT.md` in this phase's directory**, beside `47-CI-EVIDENCE.md`. The v0.10.0 corpus audit (`.project/v0.10.0/09-program-acceptance-audit.md`, §11-§13) and the archived `29-ACCEPTANCE-AUDIT.md` pointer are **not edited** — a closed milestone's document does not govern a v0.11.0 release. The box reads `- [ ] **The \`v0.11.0\` tag may be cut**` with the evidence sentence in the §13 form.
- **D-11:** The signed table is the **eight 37.1 rows, re-read at `0.11.0`** on the candidate final head, each with command, SHA, result and a pointer to `47-CI-EVIDENCE.md`: (1) `MIGRATION.md` no-TBD + register ↔ `semver-checks` allowlist set-equality — now reading **both §9.2 and §10.2** (`make check-migration-allowlist`, Phase 46); (2) `v0_9_config_boot`; (3) OpenAPI golden (`openapi_golden_v0_9`); (4) `cargo semver-checks` vs `0.9.0`, eleven crates, recorded as the known no-op per D-09; (5) MSRV 1.88; (6) `make publish-dry-run`, twelve crates in dependency order; (7) `CHANGELOG.md` `[0.11.0]` completeness via `make check-gates` plus the hard-assertion greps (`[Unreleased]` count 0, `[0.11.0]` count 1 per changelog); (8) `make check-publish-order`.
- **D-12:** `make api-surface` (no baseline move after the bump), `make check-consumer-surface` (the muster — the roadmap's Phase 49 dependency) and the CI `coverage` job (≥ 82 %) are **CI-evidence-only entries in `47-CI-EVIDENCE.md`, not signed rows**; the signed table keeps the eight-row shape every re-seal since Phase 29 has used.
- **D-13:** **Sign-off order is structurally enforced (Phase 37.1 D-10):** local re-seal → push → PR CI green including `coverage` → evidence appended → the **maintainer ticks the box in `47-ACCEPTANCE-AUDIT.md` and the tick commit is pushed on the branch** (or the maintainer tells the agent to commit it, with the reply quoted verbatim in the SUMMARY) → CI green on that true final SHA → the agent **verifies the ticked box is on `origin` at the PR head** → only then the merge + tag hand-off is written. No tick on `origin`, no hand-off. — **Reversibility:** one-way — the merge and tag that follow trigger a crates.io publish; published versions can be yanked but never deleted or re-used.
- **D-14:** CI corroboration is **recorded plainly per row**: each row names the PR-run job that reconfirms it where one exists; rows with no CI twin (`check-migration-allowlist` is not wired into `ci.yml`; `Publish Dry Run` runs only on `main` pushes) say so in the Evidence column rather than being assumed covered. No `ci.yml` change to close that gap on release day (recorded as a finding beside D-09).

**Post-tag wave & hand-offs**

- **D-15:** Phase 47 writes an **outbound `47-HANDOFF-web3sec.md`** in the phase directory (the mirror of the inbound `49-`/`50-`/`51-HANDOFF-*.md` and `46-FEEDBACK-web3sec.md`): tag, merge SHA, release run ID, the twelve crates at `0.11.0` registry-verified, pointers to `MIGRATION.md` §10.8 (the DR rows) and `tests/consumer_surface_muster.rs`, and the return contract (consumer commit SHA, D-2 choice, gate totals, divergences — the folded todo's step 5). The folded todo `2026-10-14-web3sec-single-bump-to-v0-11-0.md` gets a **dated "tag cut, hand-off written" update and stays `pending`** until web3sec returns.
- **D-16:** **SHIP-07, the ROADMAP checkbox, the REQUIREMENTS traceability row and STATE.md are ticked only after the sparse-index table shows all twelve crates at `0.11.0`, `yanked = false`**, through `gsd-tools` handlers (never direct edits), in the post-tag wave, riding `chore/47-close`. Rejected: ticking on the release PR before the merge — it ticks a claim not yet true.
- **D-17:** **`chore/47-close` carries:** the post-merge and release-run CI evidence, the registry table, the MILESTONES.md v0.11.0 entry (v0.10.0-entry shape: tag, merge SHA, release run ID, crate count, "registry-verified", Trusted Publishing), the D-16 ticks, `47-HANDOFF-web3sec.md`, the todo update, and the D-09/D-14 findings. The last plan ends at **"ready to close"** with the `/gsd-audit-milestone` / `/gsd-complete-milestone` prerequisites listed; the maintainer runs `/gsd-complete-milestone v0.11.0` on the same branch so the archive moves ride the same PR.
- **D-18:** If the **real release run goes red after the tag**, Phase 37 D-16 applies unchanged: the agent diagnoses **read-only** per `docs/src/appendix/release-recovery.md` §1-§2 (sparse index for what reached crates.io, the run log, the matching runbook section and failure code), writes it to `47-CI-EVIDENCE.md`, and hard-stops. Completing forward (§3), any re-dispatch, any yank (§5) and any `47.1` patch phase are the maintainer's. **Option A** (a patch release through the same pipeline; orphaned versions yanked only after the patch is registry-verified; one yank-register row per crate) is the recorded default if the maintainer asks for a recommendation. — **Reversibility:** one-way — a half-published dependency chain on crates.io is permanent state.

### Claude's Discretion

The maintainer did not select these; the defaults below were stated and accepted. Research may overturn a default only with evidence, and must say so in RESEARCH.md.

- **Bump commit granularity** — default: three commits on the branch (manifests via `cargo release version`; changelog stamps; `openapi.json` regen), each conventional-commit (`chore(release): …`), so a defect in one is isolated. One commit is acceptable if the tooling makes the split awkward.
- **Stamp date** — default: the day the maintainer is expected to tag; if the tag slips past that date, the heading date is corrected in a docs-only commit *before* the tick (D-00n), and the re-seal row 7 is re-read on the new head.
- **PR title and body wording**, within D-03's required contents and the v0.10.0 PR #55 shape.
- **Re-seal command order and output capture**, provided every one of D-11's eight rows has a command, a SHA and a result, and `47-CI-EVIDENCE.md` follows the `37.1-CI-EVIDENCE.md` house form (local sweep table + CI-run table + registry table; the registry probe command shape at its line ~1735 is reusable verbatim).
- **Hand-off checkpoint texts** for merge + tag — reuse Phase 37.1's with `0.11.0`; the tag command is `git tag -a v0.11.0 <merge-sha> -m "v0.11.0"` + `git push origin v0.11.0`.
- **Whether `47-ACCEPTANCE-AUDIT.md` also carries a short "what v0.11.0 is" preamble** drawn from `MIGRATION.md` §10's opening paragraph — default yes, two or three sentences, no new claims.
- **Plan count and wave layout**, provided D-00m's four-wave split around the D-13 tick gate and the maintainer's merge + tag checkpoint holds, and no plan both executes the bump and starts the re-seal in the same task.
- **The v0.11.0 GitHub Release object** — created by `release.yml` from the curated `[0.11.0]` section; no manual edit is planned. If the body is wrong, record it; a `gh release edit` is the maintainer's.
- **Whether `claude/laughing-dirac-e0h2ax` is deleted after the merge** — the maintainer's GitHub setting; no plan depends on it.

### Deferred Ideas (OUT OF SCOPE)

- **Re-pin CI's `cargo-semver-checks` baseline to `0.11.0` and add `paladin-eval`** — the job is a no-op at `0.9.0` (Phase 38 plan 38-09); a `ci.yml` change for the next milestone's first phase, recorded as a finding here (D-09).
- **Wire `check-migration-allowlist` into `ci.yml` and run `Publish Dry Run` on pull requests** — so every signed re-seal row has a CI twin (D-14). Release-tooling hardening.
- **Making `release.yml` dry-runnable against a bare SHA before a tag exists** — carried from Phases 37 / 37.1.
- **A consumer smoke test** (`cargo add paladin-ai@0.11.0` in a scratch project, cold build) as a standing post-release check — carried from Phases 37 / 37.1; web3sec's own re-pin (D-15) is the de-facto smoke test this time.
- **Path-only as a workspace-wide rule for workspace dev-dependencies** and a lint enforcing it — carried from Phase 37.1.
- **Carried from Phase 36.1 / 37 / 37.1 `<deferred>`, unchanged:** `cargo test --workspace --all-features` in CI; removing the eight crate-level rustdoc allows; regenerating `doc-coverage-report.md`.
- Reviewed, not folded: content/data orchestration base-item review; OpenAI Responses API for the pro tier; verify local `make coverage` reproduces CI's 82.39 % figure.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SHIP-07 | v0.11.0 is released: all 12 publishable crates on crates.io at `0.11.0`, the tag cut on a `main` merge commit, CHANGELOG and MIGRATION complete, and the publish-order gate green | Bump mechanics proven in a scratch worktree (cargo-release 1.1.7: 13 manifests + `Cargo.lock`, incl. the `=0.10.1` exact pin). Stamp script proven (12/12). Gates `check-gates`, `check-release-consistency`, `check-publish-order` run green post-bump. Registry probe verified read-only against the live index (12/12 `0.10.1` present, 12/12 `0.11.0` ABSENT today). **Two release blockers found that CONTEXT does not know about** (see Summary): root `[0.11.0]` body 156,528 chars breaks `create-release`; `cargo-deny` red on RUSTSEC-2026-0386 blocks the required "License & Dependency Policy" check. |
</phase_requirements>

## Summary

This is a release-operations phase: no `src/` change, one irreversible chain (merge, tag, crates.io publish). The mechanisms CONTEXT chose all work as written, and I proved that by running them in a throwaway `git worktree` (removed afterwards): `cargo release version 0.11.0 --execute --no-confirm --workspace` (cargo-release 1.1.7, built from crates.io in this sandbox in about 5 minutes) rewrote all 13 manifests, **including the unusual exact pin `version = "=0.10.1"` in `crates/paladin-ports/Cargo.toml:16`** and `crates/doc-examples` (`publish = false`), and also updated 13 `paladin-*` package entries in `Cargo.lock`. `scripts/finalize-crate-changelogs.sh --version 0.11.0` stamped 12/12 changelogs, and `make check-gates`, `check-release-consistency.sh --tag v0.11.0` and `check-publish-order.sh` were green afterwards. `index.crates.io` is reachable and confirms the pre-state: all twelve crates at `0.10.1` (`yanked=false`, all with Trusted Publishing metadata), `0.11.0` absent for all twelve.

**Two defects would stop the release and are not in CONTEXT.** (B-1) The root `CHANGELOG.md` `[Unreleased]` body is **156,528 characters / bytes (pure ASCII)**. `release.yml` `create-release` feeds exactly that section to `scripts/create-or-reuse-release.sh`, which passes it as a single `jq --arg body` argv element. I reproduced `jq: Argument list too long` (Linux single-argument cap 131,072 bytes), and GitHub's release-body API cap is 125,000 characters in any case. `create-release` is a `needs:` of `publish-crates`, so the tag run would fail before publishing anything, and the tag cannot move afterwards (Protect release tags ruleset). The v0.10.0 section was 42,708 chars and v0.10.1's 3,732, so this is the first release to hit it. (B-2) `cargo deny check advisories` fails on **RUSTSEC-2026-0386** (tokio 1.52.3 `AsyncFd::new` unsound, published 2026-10-13). CI job "License & Dependency Policy", a *required* status context of the "Protect main branch" ruleset, is red on the current PR head `88b3b9d2` and on the previous push run for `e241c348`; every other job on the head is green or still running. A lockfile-only `cargo update -p tokio --precise 1.53.3` turns `cargo deny check` fully green (verified in the worktree, with the bump applied). Both need a maintainer decision before the build wave (D-08 forbids absorbing them silently), so the plan must open with a short preflight plan and a decision checkpoint.

Smaller corrections to CONTEXT are tabulated under "CONTEXT Drift Report". The most consequential: D-11 row 7 asserts `## [Unreleased]` count 0, but the stamp script deliberately preserves the anchor (count becomes 1, empty), so the literal assertion cannot pass without hand-deleting the anchor line.

**Primary recommendation:** Plan five waves (preflight and decision checkpoint; build; pre-merge re-seal and sign-off; maintainer merge and tag; post-tag verification and records). Put B-1 (condense the root `[0.11.0]` section to at most about 110,000 bytes and add a size gate) and B-2 (lockfile-only tokio bump, maintainer-approved) into the preflight checkpoint, install `cargo-release` 1.1.7 with `cargo install --locked cargo-release --version 1.1.7`, and treat disk space (2.5 GB free) as a first-class constraint on the local re-seal.

## Architectural Responsibility Map

Release engineering has no browser/SSR tiers; the tiers here are *who is allowed to act where*.

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Version bump, changelog stamp, OpenAPI regen, evidence files | Agent in sandbox (branch `claude/laughing-dirac-e0h2ax`) | — | D-05..D-08; only commit-set the agent may author on the release branch |
| PR retitle/body, CI-run reads, registry reads | Agent via GitHub REST (`gh api`) and `index.crates.io` | Maintainer (fallback command if the write is refused) | D-03, D-00h; GraphQL blocked, REST confirmed readable, write untested |
| Sign-off tick | Maintainer only | — | D-00a |
| Merge to `main`, annotated tag push, any yank | Maintainer only (own machine; `gh pr merge` is GraphQL) | — | D-00b; "Protect release tags" has an admin-role bypass only for the maintainer |
| Gate on the tagged commit (source-in-main, consistency, CI conclusion) | GitHub Actions `release.yml` (`verify-tag-source`, `check-release-consistency`) | — | Authoritative; local runs cannot evaluate clause 3 (CI conclusion) |
| Publish in dependency order | GitHub Actions `publish-crates` via OIDC Trusted Publishing (`crates-io` environment) | crates.io | D-00g; no credential exists anywhere |
| Proof of publication | crates.io sparse index read by the agent | crates.io API (`trustpub_data`) | D-00h; workflow log alone is not proof |
| Post-tag bookkeeping | `chore/47-close` branch + docs-only PR | Maintainer merges | D-00l; `main` is ruleset-protected |

## Standard Stack

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| cargo-release | 1.1.7 (published 2026-10-21 by `epage`) | Lockstep `cargo release version` bump incl. dependency-pin rewrite and `Cargo.lock` | Already the project's selected tool (`release.toml`, `docs/src/appendix/release-automation.md`, `make install-tools` at Makefile:101). Behaviour verified this session. [VERIFIED: worktree run] |
| `scripts/finalize-crate-changelogs.sh` | in tree | Stamps `## [0.11.0] - <date>` into 12 changelogs (root included) | Enumerates via `cargo metadata`, never a glob; idempotent. [VERIFIED: ran 12/12] |
| `scripts/check-release-consistency.sh` | in tree | Tag ↔ manifest ↔ changelog agreement (+ CI-conclusion clause when `--sha` given) | The same script `release.yml` runs. [VERIFIED] |
| `scripts/check-publish-order.sh`, `make check-gates` | in tree | Publish-order gate (row 8) and the nine offline guards (row 7) | [VERIFIED: green pre- and post-bump] |
| `scripts/extract-changelog-section.sh` | in tree | Builds the GitHub Release body from `## [0.11.0]` | Used by `release.yml` `create-release`; reproduces B-1. [VERIFIED] |
| `gh api` (REST) | gh in sandbox | PR #59 retitle/body, run listing, branch/ruleset reads | GraphQL is blocked with an explicit 403 naming the REST/CCR routes. [VERIFIED] |
| `index.crates.io` sparse index + `crates.io/api/v1` | live | Registry verification | Read-only, needs `User-Agent`. [VERIFIED: probe script ran] |

### Supporting
| Tool | Version | Purpose | When to Use |
|------|---------|---------|-------------|
| cargo-semver-checks | 0.50.0 (installed; equals CI pin) | Row 4 sweep vs `0.9.0` | Local row 4; downloads baselines from crates.io |
| cargo-deny / cargo-audit | 0.20.2 / 0.22.2 (installed) | `make security` (`security.instructions.md`: must pass before a phase is sealed) | B-2 detection and fix proof |
| nightly-2026-09-20 | installed | `make api-surface` (`PUBLIC_API_TOOLCHAIN`) | Optional local confirmation of audit seam 8; CI `API Surface Tracking` is already green on the head |
| `gsd-tools` handlers | `requirements.mark-complete`, `roadmap.*`, `state.*`, `phase.complete` exist | D-16 ticks | Post-tag wave only. [VERIFIED: command-aliases.cjs lists them] |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Condensing the root `[0.11.0]` section (B-1) | Patch `create-or-reuse-release.sh` to `jq --rawfile` and add truncation | Touches release tooling on release day (against the spirit of D-08/D-09), needs script tests and a re-seal, and the 125,000-char API cap still forces truncation. Not recommended. |
| Lockfile-only tokio bump (B-2) | Allowlist RUSTSEC-2026-0386 in `deny.toml` + `.cargo/audit.toml` + `SECURITY-EXCEPTIONS.md` (the register is kept in sync by `check-advisory-register.sh`) | Adds a governed exception for a fixable advisory; the project's own precedent upgraded the directly-fixable ones "instead of ignoring them" (`.cargo/audit.toml` header). Allowlist only if the maintainer will not take a lockfile change. |
| `make release` | — | Rejected by D-00c (trailing push blocked by ruleset). |

**Installation (sandbox, verified):**
```bash
cargo install --locked cargo-release --version 1.1.7 --root "$SCRATCH/cr-root"   # about 5 min; needs rustc >= 1.97 (sandbox has 1.97.1)
export PATH="$SCRATCH/cr-root/bin:$PATH"
```
`release.toml` records `cargo-release 1.1.2` from Phase 20; the bump behaviour was verified on 1.1.7, so pin 1.1.7 rather than "latest".

**Version verification:** `crates.io/api/v1/crates/cargo-release` → `max_stable_version 1.1.7`, repo `crate-ci/cargo-release`; sparse index lists `rust_version 1.97` for 1.1.7 [VERIFIED: registry].

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| cargo-release | crates.io | 10 yrs (2016-03) | 1,341/wk recent per seam; 548,725 total | github.com/crate-ci/cargo-release | OK (`gsd-tools query package-legitimacy check --ecosystem crates`) | Approved; named in project docs and Makefile. Version 1.1.7 is 4 days old; behaviour verified empirically, so pin it. |
| cargo-semver-checks, cargo-deny, cargo-audit | crates.io | 4-10 yrs | 13k-313k/wk | obi1kenobi / EmbarkStudios / rustsec | OK | Already installed and CI-pinned; no new install. |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
No postinstall concept exists on crates.io; `cargo install --locked` compiles from source (no `build.rs` flagged: `has_build_rs: null` for 1.1.7).

## Architecture Patterns

### System Architecture Diagram

```
 sandbox (agent)                        GitHub                               crates.io
 ───────────────                        ──────                               ─────────
 W0 preflight ──► decisions (tokio, body size, stamp date)  [maintainer checkpoint]
        │
 W1 build:  cargo release version 0.11.0 ─► commit 1 (13 manifests + Cargo.lock)
            finalize-crate-changelogs --date ─► commit 2 (12 changelogs, body-size gate)
            make openapi ─► commit 3 (openapi.json line 14 only)
        │
 W2 pre-merge: local re-seal (8 rows) ─► push ─► PR #59 head ─► CI (44 required contexts)
            │                                      │  (agent: REST retitle/body; reads runs)
            ▼                                      ▼
   47-CI-EVIDENCE.md  ◄───────────────────── PR-run job twins (SHA "A" = PR head)
            │
   47-ACCEPTANCE-AUDIT.md ─► MAINTAINER ticks box + pushes tick commit ─► CI green on final SHA
            │                      agent verifies tick on origin/PR head  (D-13) ── no tick, no hand-off
            ▼
 W3 hand-off ─────────────► MAINTAINER: merge commit (not squash) ─► main push triggers ci.yml
                                   │                     (first run of E2E + Publish Dry Run)
                                   ▼                     wait for ci.yml success on merge SHA "B"
                            MAINTAINER: git tag -a v0.11.0 B ; git push origin v0.11.0
                                   │
                                   ▼  release.yml: verify-tag-source ► test ► create-release (BODY!)
                                      ► check-release-consistency (needs ci.yml success on B)
                                      ► publish-crates (OIDC) ─ 12 crates in CRATES order ───►  0.11.0 x12
 W4 post-tag: agent read-only: tag object, release run, sparse-index table (12 x 0.11.0, yanked=false)
            ► chore/47-close: MILESTONES, gsd-tools ticks, 47-HANDOFF-web3sec.md, todo update ► "ready to close"
```
Two-SHA rule (Phase 29 D-21): evidence is recorded for **A** (the PR head carrying the bump and the tick) and **B** (the merge commit that gets tagged). [CITED: 29-CONTEXT.md D-21]

### Recommended Project Structure (files this phase creates; no `src/` change)
```
.planning/phases/47-v0-11-0-crate-release/
├── 47-RESEARCH.md              # this file
├── 47-NN-PLAN.md / -SUMMARY.md
├── 47-CI-EVIDENCE.md           # local sweep table + CI-run table + registry table + Findings (house form: 37.1)
├── 47-ACCEPTANCE-AUDIT.md      # preamble + 8-row table + the sign-off box (D-10/D-11)
├── 47-HANDOFF-web3sec.md       # outbound, mirrors 49-HANDOFF §9 return contract (D-15)
└── .continue-here.md           # resume file at the merge+tag stop (D-00i)
```
Tracked-file changes on the branch: `Cargo.toml`, 12 `crates/*/Cargo.toml`, `Cargo.lock`, 12 changelogs, `crates/paladin-web/openapi.json`, plus any B-1/B-2 remedy the maintainer approves.

### Pattern 1: Bump with proof (commit 1)
```bash
# Source: verified in a scratch worktree, 2026-10-25
cargo release version 0.11.0 --execute --no-confirm --workspace
git diff --stat            # expect exactly 14 files: Cargo.toml, 12 crates/*/Cargo.toml (incl. doc-examples), Cargo.lock
# grep-zero, comment lines excluded (manifests carry historical "0.10.1" in # comments on purpose):
grep -nE '0\.10\.1' Cargo.toml crates/*/Cargo.toml | grep -vE '^[^:]+:[0-9]+:[[:space:]]*#'   # expect no output
# structural proof, independent of text: every workspace version is 0.11.0 and every versioned path dep points at it
cargo metadata --format-version 1 --no-deps --offline | jq -e '
  ([.packages[].version]|unique)==["0.11.0"] and
  ([.packages[].dependencies[]|select(.path!=null and .req!="*")|.req|test("0\\.11\\.0")|not]|any|not)'
cargo metadata --locked --offline --format-version 1 --no-deps >/dev/null   # Cargo.lock in sync
# Cargo.lock: only third-party crates may still read 0.10.1 (chacha20poly1305, core-foundation, rand, rand_core)
awk '/^name = /{n=$3} /^version = "0.10.1"/{print n}' Cargo.lock
```
Three path-only dev-dependencies (`paladin-battalion` → `paladin-llm`, `paladin-storage`; `paladin-llm` → `paladin-storage`) carry `req "*"`; that is the Phase 37.1 publish-order design, not a missed pin (hence `.req!="*"` in the jq).

### Pattern 2: Stamp with an explicit date (commit 2)
```bash
./scripts/finalize-crate-changelogs.sh --version 0.11.0 --date 2026-MM-DD   # the tag date (D-00n)
make check-gates && ./scripts/check-release-consistency.sh --tag v0.11.0
./scripts/extract-changelog-section.sh --version 0.11.0 --output "$SCRATCH/body.md"
test "$(wc -c < "$SCRATCH/body.md")" -le 110000 || echo "BODY TOO LARGE"       # B-1 gate (see Pitfall 1)
```
`make finalize-crate-changelogs` takes only `VERSION`; it stamps **today's** date (`date +%Y-%m-%d`). Use the script with `--date` whenever the stamp day differs from the tag day.

### Pattern 3: Registry probe (verbatim from 37.1, tested live)
```bash
cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.publish == null) | .name' | sort | while read -r c; do
  n=${#c}; if [ $n -eq 1 ]; then p="1/$c"; elif [ $n -eq 2 ]; then p="2/$c"; elif [ $n -eq 3 ]; then p="3/${c:0:1}/$c"; else p="${c:0:2}/${c:2:2}/$c"; fi
  row=$(curl -sf -H 'User-Agent: paladin-release-verify (github.com/DF3NDR/paladin-dev-env)' "https://index.crates.io/$p" \
        | jq -c --arg v "$V" 'select(.vers==$v) | {vers,yanked,cksum}' || true)
  echo "$c | ${row:-ABSENT}"
done        # V=0.11.0 ; today: 12 x ABSENT. Package names come from cargo metadata (crates/paladin-core publishes as paladin-ai-core).
```

### Pattern 4: PR retitle/body via REST (D-03)
```bash
gh api -X PATCH repos/DF3NDR/paladin-dev-env/pulls/59 -f title='release: v0.11.0 — Treasurer Spend Governance' -F body=@"$SCRATCH/pr-body.md"
```
Reads work and the token reports `permissions: admin, push` on the repo; a write has not been attempted [ASSUMED: PATCH succeeds]. `gh auth status` prints "token invalid" yet REST calls succeed, so do not gate on it. Fallback per D-03: print this exact command for the maintainer.

### Anti-Patterns to Avoid
- **Trusting `grep 0.10.1` across the manifests:** produces false positives from comments; use the filtered grep plus the jq proof.
- **Using directory names for registry probes or semver-checks:** `crates/paladin-core` is `paladin-ai-core`.
- **Running `make release`:** rejected (D-00c); also would commit and push.
- **Pushing the manifest-only commit alone:** the bump invalidates the OpenAPI baseline (`openapi_matches_committed_baseline`) and the pre-push hook would reject it (Makefile `release` target comment, Phase 20-07 finding). Push the three commits together; local hooks are not installed in this sandbox, CI is the enforcer.
- **Tagging before `ci.yml` has *completed successfully* on the merge SHA:** `check-release-consistency` clause 3 fails closed (`CI_MISMATCH`) and `publish-crates` never starts.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Lockstep version bump + pin rewrite + lockfile | sed over manifests | `cargo release version … --workspace` | Handled the `=0.10.1` exact pin and `publish=false` doc-examples correctly (verified) |
| Changelog stamping | perl/sed per file | `scripts/finalize-crate-changelogs.sh` | Idempotent, `cargo metadata`-driven, named failures |
| Publishable-crate list | a hard-coded twelve | `cargo metadata … select(.publish==null)` | D-00f; matches `publish-crates.sh` CRATES (gate-enforced) |
| Publish order | manual ordering | `scripts/publish-crates.sh` + `check-publish-order.sh` | Registry-state-based skip and bounded polling |
| Release-body assembly | hand-edited GitHub Release | `extract-changelog-section.sh` → `create-or-reuse-release.sh` → `finalize-release-body.sh` | Existing pipeline; hence the size gate, not a bypass |
| Registry proof | the run log | sparse-index read per crate | D-00h |
| Tick of SHIP-07/ROADMAP/STATE | direct file edits | `gsd-tools` handlers | D-16 |

**Key insight:** every control already exists; the risk is not missing tooling but two unmeasured *inputs* (changelog size, advisory DB drift) feeding tools that were last exercised on small, clean inputs.

## Runtime State Inventory

A version bump is not a rename, but the canonical question applies: after every tracked file reads `0.11.0`, what still says `0.10.1`?

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | None. No datastore keys embed the crate version. `crates/paladin-web/openapi.json:14` embeds it as a *committed file*, generated from `env!("CARGO_PKG_VERSION")` (`crates/paladin-web/src/openapi.rs:39`). | Regenerate via `make openapi`; expected diff = that one line |
| Live service config | GitHub: PR #59 title/body (`feat(43-02)…`), 3 active rulesets (`Protect main branch` with 44 required contexts and strict up-to-date policy, `Protect release branches`, `Protect release tags`). crates.io Trusted Publisher config: all 12 crates' `0.10.1` carry `trustpub_data.repository = DF3NDR/paladin-dev-env`, so every crate is linked. | Retitle PR; no ruleset change; no crates.io config change |
| OS-registered state | None expected: this is a Rust library workspace with no OS registrations in the tree (a `git grep` for systemd/launchd/schtasks/pm2/crontab outside docs hits only application cron and notification code, nothing version-bearing); not probed on the maintainer's machine | None |
| Secrets/env vars | None. No registry credential exists (D-00g); `release.yml` mints a short-lived OIDC token under the `crates-io` environment. | None; do not introduce one |
| Build artifacts / installed packages | `Cargo.lock` (13 workspace-package entries, fixed by cargo-release); `target/` (20 GB of 0.10.1 artifacts; see Pitfall 9); `k8s/server/worker-deployment.yaml:20,35` label `version: v0.10.0` (already stale at 0.10.1); historical `0.10.1` strings in manifest comments, `.cargo/semver-checks-allowlist.toml`, MIGRATION/CHANGELOG (baseline references, correct as written) | Lockfile: part of commit 1. `k8s` label: **record as a finding (D-00d), do not fix**. Comments: leave. |

## Common Pitfalls

### Pitfall 1 (BLOCKER B-1): The release body is larger than GitHub and Linux allow
**What goes wrong:** `create-release` runs `extract-changelog-section.sh --version v0.11.0` then `create-or-reuse-release.sh --body-file`; the script does `jq -n --arg body "$(cat file)"`. The root `[0.11.0]` section is **156,528 chars** (120 bullets; Added 84.5k, Changed 36.4k, Fixed 11.3k, Breaking 10.8k, Removed 5.2k, Known limitations 4.0k, Security 2.5k, Documentation 1.7k). Reproduced here: `jq: Argument list too long` for a 156,528-byte arg (single argv element cap 131,072 bytes). Independently, GitHub's release API rejects bodies over 125,000 characters ("body is too long (maximum is 125000 characters)") [CITED: github.com/intuit/auto/issues/2418 via search; the cap itself is not in GitHub's own docs, treat as MEDIUM].
**Why it happens:** Phases 38-51 accumulated entries under one `[Unreleased]`; v0.10.0's section was 42,708 and v0.10.1's 3,732, so no previous release approached the limit. v0.10.1's live release body is 3,702 chars of which about 1,000 is the artifact block `finalize-release-body.sh` appends after a marker, so budget about 1 KB for that.
**Consequence:** `create-release` fails → `publish-crates` (`needs: create-release`) never runs → tag already pushed and cannot be moved ("Protect release tags": no non-fast-forward, no deletion except admin bypass) → re-running failed jobs reads the same CHANGELOG at the same tag → same failure. Nothing is published, but the v0.10.0 lesson (a tag that cannot complete forward) repeats.
**How to avoid:** Preflight checkpoint: ask the maintainer to approve condensing the root `[0.11.0]` to **at most 110,000 bytes** (gives about 15k headroom for the artifact block and multibyte characters; the current text is pure ASCII so chars = bytes) — e.g., keep Breaking Changes, Security, Known limitations, Removed and a per-phase summary of Added/Changed in the root file and rely on the per-crate changelogs and `MIGRATION.md` §10 for detail. Add the size gate (Pattern 2) as a row-7 hard assertion, and rehearse the real payload builder offline: source `create-or-reuse-release.sh` with `CREATE_OR_REUSE_RELEASE_LIB_ONLY=1` and call `_cor_build_payload` on the extracted file (no network). Do not edit scripts or workflows (D-08/D-09).
**Warning signs:** `wc -c body.md` > 120000 on the extracted section.

### Pitfall 2 (BLOCKER B-2): `cargo-deny` is red on the PR head before any bump
**What goes wrong:** RUSTSEC-2026-0386 (tokio `AsyncFd::new` I/O-safety, unsound, date 2026-10-13) hits `tokio 1.52.3` in `Cargo.lock`. `cargo deny check` fails on it ("advisories FAILED"), so CI "License & Dependency Policy" (required context) fails; `cargo audit` only warns (10 allowed warnings), which is why "Security Audit" is green. Failed on run `40130860018` (head `e241c348`) and on `40148572118` (head `88b3b9d2`); the other 30+ jobs on the head are green or running (Docker Build was still running at read time).
**Why it happens:** advisory database drift after Phase 46 sealed; tokio is a direct and transitive dependency of almost every crate.
**How to avoid:** Put to the maintainer (D-08). Recommended remedy, verified: `cargo update -p tokio --precise 1.53.3` (lockfile only; one package; tokio MSRV 1.71) — afterwards `cargo deny check` reports `advisories ok, bans ok, licenses ok, sources ok`, `cargo audit` shows 9 allowed warnings, and all of `make check-gates` stays green. Land it as its own `fix(deps)` commit *before* the bump commit so the bump set stays pure. A `cargo update -p tokio` without `--precise` moves to 1.54.1 (a wider jump; 1.54.0 is yanked) — do not use. `make security` "must pass before a phase is sealed" (`security.instructions.md`), so this is not optional.
**Warning signs:** `cargo deny check advisories` non-zero locally in under a minute (network to the advisory DB works through the proxy).

### Pitfall 3: D-11 row 7 "`[Unreleased]` count 0" cannot be met by the stamp script
**What goes wrong:** `finalize-crate-changelogs.sh` inserts the dated heading *after* the `## [Unreleased]` anchor and keeps the anchor (its own test pins this, `finalize-crate-changelogs_test.sh:171`). Verified post-stamp: every one of 12 changelogs has `[Unreleased]` = 1 (empty) and `[0.11.0]` = 1. The 0.10.1 precedent (count 0) came from a patch release written without an anchor.
**How to avoid:** Choose before execution and record it: (a) **recommended, matches D-11's literal text and the 0.10.1 precedent:** after stamping, delete the now-empty `## [Unreleased]` line in the same stamp commit and assert `[Unreleased]`=0, `[0.11.0]`=1 per file; the next milestone re-adds the anchor when it starts (as Phase 38 did). Or (b) amend the assertion to "anchor present and empty (the next non-blank line is `## [0.11.0] - <date>`)". Either satisfies `check-changelogs`, `check-release-consistency` and `extract-changelog-section.sh` (verified the section boundary logic stops at the next `## [`). Flag to the maintainer as a D-11 wording correction.

### Pitfall 4: Registry probes and semver-checks keyed on directory names
`crates/paladin-core` publishes as **`paladin-ai-core`**; the facade is `paladin-ai`. Use `cargo metadata` names (Pattern 3). [VERIFIED]

### Pitfall 5: `=0.10.1` exact pin and comment false-positives
`crates/paladin-ports/Cargo.toml:16` pins `paladin-ai-core` as `=0.10.1`; cargo-release 1.1.7 rewrites it to `=0.11.0` (verified). An older cargo-release or a hand edit might not; the jq proof in Pattern 1 catches any survivor. Eight manifests also carry `0.10.1` inside `#` comments (semver-diagnostic provenance); they are correct history and must not be "fixed".

### Pitfall 6: Empty `[0.11.0]` sections for three crates
`paladin-content`, `paladin-memory` and `paladin-notifications` have an empty `[Unreleased]`, so the stamp yields a heading with no body (gate passes: presence is the signal). Record as a finding and, if the maintainer wants, add one line ("No functional change; version bumped in lockstep with the workspace.") in the stamp commit. Do not invent content.

### Pitfall 7: `check-release-consistency` is not in `check-gates` and cannot test CI conclusion locally
Run it explicitly with `--tag v0.11.0`; without `--sha` it prints "clause 3 not checked". Clause 3 is only decided in `release.yml` on the merge SHA. [VERIFIED]

### Pitfall 8: Two CI jobs run for the first time *after* the merge
`End-to-End Tests` and `Publish Dry Run` are `if: push && ref == main`; they were `skipped` on PR runs (including the current head). They run on the merge commit **B** and are part of the whole-run conclusion clause 3 reads. If either is red on B, `main` already contains the release commit, no tag may be cut, and recovery is a maintainer fix-forward PR. Mitigate by running the dry-run equivalent locally (row 6) and by recording this exposure in the hand-off text. [VERIFIED: ci.yml 2010-2016, 2152-2156]

### Pitfall 9: Sandbox capacity (2.5 GB free; target dir is 20 GB)
`df`: `/` 252 GB size, 37 GB used, **2.5 GB available**; `target/debug` is 17 GB `deps` + 2 GB `incremental`. A version bump changes package identity, so every workspace crate and every test binary rebuilds under new hashes (old ones stay until cleaned). Rows 2, 3, 5, 6 and `make openapi` compile; row 6 (`make publish-dry-run` → `clean-code`, `test`, doc tests, `audit`, `build-release`) is the heaviest. Plan a `df -h /` preflight with an explicit threshold, and an explicit, recorded space-recovery step (e.g., remove `target/debug/incremental`, or `cargo clean` with the "warm the clippy hook after `cargo clean`" caveat), or run the compile-heavy rows on the maintainer's machine and record the CI twin instead [ASSUMED: size of new artifacts exceeds 2.5 GB; measured only the current sizes].

### Pitfall 10: Sign-off/evidence staleness and ordering
Editing any SUMMARY/evidence after VERIFICATION stales it; the tick commit changes the PR head, so CI must be re-read on the *tick* SHA, not the earlier one; the agent must verify the ticked box exists on `origin` at the PR head before writing the hand-off (D-13). If `main` moves, restart from the top (D-02): `gh api repos/DF3NDR/paladin-dev-env/branches/main --jq .commit.sha` must still be `df24689c3d1fd52b0e730105c15b22c2c4ad1404` (it is, today).

### Pitfall 11: Shallow clone
`.git/shallow` present; `rev-list --count HEAD` = 655; `git merge-base` with `origin/main` is empty. Take commit counts from GitHub (`GET /pulls/59` → `commits: 1083, changed_files: 1082` at research time; CONTEXT's 1,081/1,080 predates two docs commits) and say so in the evidence.

## Code Examples

### Read-only PR/CI state (REST)
```bash
gh api repos/DF3NDR/paladin-dev-env/pulls/59 --jq '{title,head:.head.sha,commits,changed_files,mergeable_state}'
gh api "repos/DF3NDR/paladin-dev-env/actions/workflows/ci.yml/runs?head_sha=$SHA" --jq '.workflow_runs[]|{event,status,conclusion,id}'
gh api repos/DF3NDR/paladin-dev-env/actions/runs/$RUN/jobs --paginate --jq '.jobs[]|select(.conclusion!="success")|{name,status,conclusion}'
```
Job *logs* are served from `*.blob.core.windows.net`, which the proxy blocks; use job step conclusions from `/jobs` and reproduce failures locally.

### Post-merge, pre-tag read-only checks (hand-off Step 2)
```bash
git fetch origin --tags; B=$(git rev-parse origin/main)
git rev-list --parents -n 1 "$B"            # commit + two parents = a merge commit
gh api "repos/DF3NDR/paladin-dev-env/actions/workflows/ci.yml/runs?head_sha=$B&status=completed" --jq '.workflow_runs[]|{conclusion,id,event}'
```
Tag (maintainer): `git tag -a v0.11.0 "$B" -m "v0.11.0"` then `git push origin v0.11.0`. Post-tag (agent, read-only): `git cat-file -t v0.11.0` is `tag`; `git rev-parse v0.11.0^{commit}` equals `B`; release run via `gh api repos/DF3NDR/paladin-dev-env/actions/workflows/release.yml/runs?head_sha=$B`.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Per-crate `cargo publish -p` dry runs | `cargo publish --workspace --dry-run` (local overlay) + registry-aware `check-publish-order.sh` | Phases 20 / 37.1 | Dry run alone is blind to publish order; row 8 is the guard |
| Registry token secret | crates.io Trusted Publishing (OIDC, `crates-io` environment) | Phase 19 | No credential to rotate or leak; first publish of a crate still cannot use it (37-RESEARCH Q3) |
| `semver` job against moving "latest" | Pinned `--baseline-version 0.9.0` (no-op for 0.x majors) | Phase 22 | Known no-op at 0.9.0 → 0.11.0 (D-09, recorded finding) |

**Deprecated/outdated in the tree (record, do not fix):** `k8s/server/worker-deployment.yaml` `version: v0.10.0` labels; `Makefile`/`release.toml` comments saying "ten crates"/"eleven" where the tree says twelve; `ci.yml` job display name "Semver Checks (vs v0.9.0)".

## CONTEXT Drift Report (citations verified against the tree, HEAD `88b3b9d2`)

| CONTEXT claim | Finding |
|---|---|
| `Cargo.toml` 34-43, 136, 229, 344 pins | Correct. |
| `Cargo.toml` line 58 = `[workspace.package] version` | **Drift:** line 58 is the root `[package] version` (`paladin-ai`). `[workspace.package]` (line 12) holds only `rust-version`; members use literal versions, so there is no single-source version. |
| "Thirteen manifests" | Correct (root + 12 `crates/*` dirs, of which `doc-examples` is `publish=false`). **Omitted:** `Cargo.lock` also changes (13 package entries) and must be in commit 1; `crates/paladin-ports/Cargo.toml:16` carries an exact `=0.10.1` pin. |
| Makefile lines 189, 217, 248, 252, 417, 423, 618, 628, 634, 642 | All correct. `check-release-consistency` is at 231 and is *not* in `check-gates`. |
| `release.yml` lines 3-17, 29, 92, 112, 405, 602 | All correct. |
| `ci.yml` 313-395 semver job | Comment starts 313, job 345, loop ends ~401. Substance correct. |
| ROADMAP ~792, REQUIREMENTS 434 / 515 | Correct. |
| `[Unreleased]` count 0 after the stamp (D-11 row 7) | **Drift:** stamp preserves the anchor (count 1). See Pitfall 3. |
| `make finalize-crate-changelogs VERSION=0.11.0` "dated to the tag date" | The make target has no date argument; it stamps *today*. Use the script with `--date`. |
| Sandbox: no Docker | Confirmed in effect: `docker` and compose CLIs exist, **no daemon** (`/var/run/docker.sock` absent). |
| Sandbox: shallow clone (653 commits) | Confirmed (655 now, +2 context commits). |
| Sandbox: `cargo-release` not installed | Confirmed; **installable** (1.1.7, about 5 min, rustc 1.97.1 satisfies its `rust_version 1.97`). |
| Sandbox: GraphQL `gh` blocked | Confirmed (HTTP 403 with a message naming the REST and CCR routes). REST reads work; `gh auth status` misleadingly says the token is invalid. Repo permissions show admin/push; write untested. |
| PR #59: 1,081 commits / 1,080 files, CI queued on `e241c348` | Now 1,083 / 1,082, head `88b3b9d2`; CI on `e241c348` completed **failure** (License & Dependency Policy), see B-2. |
| `main` tip `df24689c` | Confirmed (`branches/main`). |
| Other sandbox facts not in CONTEXT | MSRV 1.88 toolchain not installed (only stable 1.97.1 and nightly-2026-09-20); local git hooks and `pre-commit` not installed; only 2.5 GB free disk. |
| MIGRATION §10 frozen at `e28e43b5`; re-read only if consumer surface moves | `src/` and `crates/*/src` **did** change after `e28e43b5` (Phase 46 WR-01..WR-03 fixes: `commander.rs`, `conclave_execution_service.rs`, `phalanx_service.rs`, `battalion_config.rs`, `conclave.rs`, `legacy_keys.rs`, `circuit_breaker.rs`, `seeded.rs`). CI `API Surface Tracking` is green on the head, so the public surface did not move; include a bounded "diff `e28e43b5..HEAD -- src crates/*/src`, api-surface green, §10 stays frozen" task and record the answer. |
| Audit seam 8 (API-surface baseline currency) PARTIAL | Resolved by evidence available now: CI `API Surface Tracking` concluded success on `88b3b9d2`; nightly-2026-09-20 is installed locally if a local confirmation is wanted. |
| Canonical refs exist | All files named in `<canonical_refs>` exist (checked 12 paths). `37.1-CI-EVIDENCE.md` registry table (~1419) and probe command (~1738) match. |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `gh api -X PATCH …/pulls/59` with the sandbox token succeeds (permissions show admin/push; no write attempted) | Pattern 4 | Agent must fall back to handing the command to the maintainer (already in D-03) |
| A2 | Local rebuild artifacts after the version bump exceed the 2.5 GB free space | Pitfall 9 | If they fit, the space-recovery step is unnecessary; harmless to include a preflight |
| A3 | GitHub release-body cap is 125,000 characters | Pitfall 1 | The argv cap (131,072 bytes) is independently verified and gives the same remedy; 110,000 bytes is safe under both |
| A4 | `cargo semver-checks` vs `0.9.0` remains `0 checks, 254 skip` at 0.11.0 (0.x minor bump treated as major) | D-11 row 4 | If it fires, record per D-00j; CI's `Semver Checks` job on the final head is the twin |
| A5 | `make openapi` yields a one-line diff (version string) | Runtime State | Verified indirectly (`env!("CARGO_PKG_VERSION")`, committed line 14); not run (disk). Any other change is a D-07 stop |
| A6 | `ci.yml` on the merge commit will be green incl. `End-to-End Tests` and `Publish Dry Run` (never run on this tree) | Pitfall 8 | A red main-push run blocks the tag after the irreversible merge |

## Open Questions

1. **B-1: how to shrink the root `[0.11.0]` section (maintainer decision, blocks the build wave).**
   - Known: 156,528 chars now; hard limits 131,072 bytes (argv) and 125,000 chars (API); budget at most about 110,000 bytes.
   - Unclear: which content the maintainer wants to keep in the root file versus per-crate changelogs and MIGRATION §10.
   - Recommendation: a preflight checkpoint presenting the section-size table above; the agent condenses only what the maintainer approves, then the size gate and offline payload rehearsal must pass.
2. **B-2: tokio advisory remedy (maintainer decision).** Recommend lockfile-only `--precise 1.53.3` as a separate `fix(deps)` commit before the bump; alternative is a governed exception. Both leave `cargo audit` unchanged.
3. **D-11 row 7 wording:** delete the empty `[Unreleased]` anchor (recommended) or amend the assertion (Pitfall 3).
4. **Stamp date:** the day the maintainer expects to tag; if it slips, a docs-only commit corrects headings before the tick (D-00n).
5. **Empty crate sections:** add the one-line lockstep note or leave heading-only (Pitfall 6).
6. **Space strategy for the local re-seal:** clean-and-rebuild here, or run rows 2, 3, 5, 6 on the maintainer's machine and rely on CI twins (rows 2-5 each have a PR-run twin; row 6 has none).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| cargo / rustc | all | ✓ | 1.97.1 | — |
| cargo-release | D-05 bump | ✗ → installable | 1.1.7 after `cargo install --locked` (about 5 min) | hand-edit is *not* a fallback (missed-pin risk) |
| cargo-semver-checks | row 4 | ✓ | 0.50.0 (= CI pin) | CI `Semver Checks` job |
| cargo-deny / cargo-audit | `make security`, B-2 | ✓ | 0.20.2 / 0.22.2 | — |
| Rust 1.88 toolchain | row 5 (MSRV) | ✗ | only stable 1.97.1 + nightly-2026-09-20 | CI `MSRV (Rust 1.88)` job (green on head); `rustup toolchain install 1.88` needs about a full extra build tree |
| nightly-2026-09-20 | `make api-surface` | ✓ | installed | CI `API Surface Tracking` |
| Docker daemon | `make coverage`, integration suites | ✗ (CLI and compose present, no daemon) | — | CI `Coverage` job (D-00e) |
| `gh` REST | PR/CI reads, retitle | ✓ reads; writes untested | gh present; GraphQL blocked | maintainer runs the printed command |
| `index.crates.io` / `crates.io/api` | registry proof | ✓ | live | — |
| Advisory DB (cargo-deny/audit) | B-2 | ✓ | reachable | — |
| Disk space | any compile | ⚠ 2.5 GB free | — | see Pitfall 9 |
| local git hooks / `pre-commit` | n/a | ✗ not installed | CI `pre-commit run --all-files` is a required context | CI |
| python3, jq, curl, shellcheck | scripts | ✓ | present | — |

**Missing with no fallback:** none, provided B-1 and B-2 are decided.
**Missing with fallback:** cargo-release (install), MSRV 1.88 (CI twin), Docker (CI), space (clean or offload).

## Validation Architecture

(`workflow.nyquist_validation` is absent from `.planning/config.json`, so it is enabled.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | bash guard scripts + `tests/scripts/*_test.sh` regression harnesses; `cargo test`; GitHub Actions as the authoritative gate |
| Config file | `Makefile` (`check-gates`, `test-shell-guards`), `.github/workflows/{ci,release}.yml` |
| Quick run command | `make check-gates && ./scripts/check-release-consistency.sh --tag v0.11.0` (about 10 s; verified green on the bumped scratch tree) |
| Full suite command | `make test-shell-guards` (54 assertions, verified green) plus the eight re-seal rows (Wave 0 gaps below) |

### Phase Requirements → Test Map (SHIP-07)

| Behavior | Test Type | Automated Command | Exists? |
|----------|-----------|-------------------|---------|
| All manifests at 0.11.0, pins consistent, lockfile in sync | structural | Pattern 1 jq proof + `cargo metadata --locked --offline` | ✅ (verified) |
| Filtered grep-zero for `0.10.1` | static | `grep -nE '0\.10\.1' Cargo.toml crates/*/Cargo.toml \| grep -vE ':[0-9]+:[[:space:]]*#'` → empty | ✅ |
| 12 changelogs carry exactly one `## [0.11.0] - <date>`; `[Unreleased]` per Pitfall 3 | static | `for f in CHANGELOG.md crates/*/CHANGELOG.md; do grep -c '^## \[0\.11\.0\]' $f; done` (all 1) + `make check-changelogs` | ✅ |
| Release body fits (B-1) | static + offline | `wc -c` ≤ 110000 on `extract-changelog-section.sh` output; payload build via the sourced `_cor_build_payload` | ❌ Wave 0 (new check) |
| Row 1 | static | `grep -c TBD MIGRATION.md` → 0; `make check-migration-allowlist` | ✅ (verified; no CI twin) |
| Row 2 `v0_9_config_boot` | integration | `cargo test --features web-server --test v0_9_config_boot` | ✅ file `tests/integration/v0_9_config_boot_test.rs`; CI twin: E2E Platform API job |
| Row 3 OpenAPI golden | integration | `cargo test -p paladin-web --test openapi_golden_v0_9`; baseline: `cargo test -p paladin-web --lib openapi_matches_committed_baseline` | ✅; CI twin: Integration Tests |
| Row 4 semver vs 0.9.0 (11 crates) | tool | loop `cargo semver-checks check-release --package <p> --default-features --baseline-version 0.9.0` | ✅; CI twin: Semver Checks |
| Row 5 MSRV | compile | `RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets` | ⚠ toolchain absent; CI twin: MSRV job |
| Row 6 publish dry run (12, dependency order) | tool | `make publish-dry-run` | ✅ heavy; **no CI twin on PRs** |
| Row 7 completeness | static | `make check-gates` + the changelog greps | ✅ (verified) |
| Row 8 publish order | static | `make check-publish-order` | ✅ (verified); CI twin: License & Dependency Policy step "Check publish order" (step is green even while the job is red) |
| Security gate | tool | `cargo deny check` and `cargo audit` (`make security`) | ✅ currently RED until B-2 |
| Evidence-only: api-surface, muster, coverage | CI | `make api-surface`; `make check-consumer-surface`; CI `Coverage` ≥ 82 % | ✅ CI-only per D-12 |
| PR title/body updated | REST read | `gh api repos/DF3NDR/paladin-dev-env/pulls/59 --jq .title` | ✅ |
| Sign-off ordering (D-13) | REST + git | tick present in `47-ACCEPTANCE-AUDIT.md` at `git show origin/claude/laughing-dirac-e0h2ax:<path>` and CI success on that SHA | ✅ |
| Merge is a merge commit; tag annotated on it | git | `git rev-list --parents -n 1 B` (two parents); `git cat-file -t v0.11.0` = `tag`; `git rev-parse v0.11.0^{commit}` = B | ✅ |
| Release run green; publish-order gate green on the release run | REST | `release.yml` run for B: jobs `Verify Tag From Main`, `Pre-Publish Consistency Gate`, `Create Release`, `Publish to crates.io` all `success` | ✅ |
| 12 crates at 0.11.0, `yanked=false` | registry | Pattern 3 with `V=0.11.0` → 12 rows, none ABSENT | ✅ (script verified against 0.10.1) |
| Pre-flight: 0.11.0 absent everywhere before the tag | registry | Pattern 3 → 12 × ABSENT | ✅ (verified today) |

### Sampling Rate
- **Per task commit:** `make check-gates` plus the task-specific structural check (seconds).
- **Per wave merge:** `make test-shell-guards`, `cargo deny check`, `cargo metadata --locked --offline`.
- **Phase gate:** all eight rows green on the final head, CI green on the tick SHA (A) and on the merge SHA (B), registry table 12/12 before any D-16 tick.

### Wave 0 Gaps
- [ ] Body-size gate and offline payload rehearsal (B-1) — new check, no existing script.
- [ ] Decide B-2 (tokio) and the row-7 wording, and record both as findings before the build wave.
- [ ] `cargo install --locked cargo-release --version 1.1.7` and `df -h /` threshold check.
- [ ] Empty evidence skeletons: `47-CI-EVIDENCE.md` (three tables), `47-ACCEPTANCE-AUDIT.md` (8 rows + box).
- [ ] Space-recovery decision for rows 2/3/5/6 (Pitfall 9).

## Security Domain

(`security_enforcement` is absent from config, so it is enabled.)

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (publish identity) | crates.io Trusted Publishing: OIDC token from the `crates-io` GitHub Environment (admits only `v*.*.*` tags), minted per run by `rust-lang/crates-io-auth-action@v1`; no stored registry credential (D-00g) |
| V3 Session Management | no | — |
| V4 Access Control | yes | Rulesets: "Protect main branch" (44 required checks, strict), "Protect release tags" (admin bypass only); human-only merge, tag, yank (D-00a/b) |
| V5 Input Validation | yes | Release body and tag derive from tainted inputs; the workflow routes them through `env:` (CR-01) and `jq --arg`; size gate guards the body |
| V6 Cryptography | no new surface | none hand-rolled; registry checksums are read, not computed |
| V10/V14 Supply chain & config | yes | `make security` (cargo-audit + cargo-deny) must pass (currently red, B-2); `cargo install --locked` for the one new tool; package-legitimacy OK |

### Known Threat Patterns for this release flow

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Tag pushed to a non-main or unreviewed commit | Spoofing/Tampering | `verify-tag-source` (`merge-base --is-ancestor` vs `origin/main`), annotated tag on the merge commit, release-tags ruleset |
| Publishing from a half-verified tree | Tampering | `check-release-consistency` + whole-run CI conclusion gate before `publish-crates` |
| Credential leakage in release automation | Information disclosure | No token exists; agent never requests or writes one; read-only registry probes send only a `User-Agent`; no `-L` redirect following with credential headers (house rule) |
| Agent performing an irreversible act | Elevation of privilege | Hand-off commands only; resume file and hard stop; verify ticked box on `origin` before hand-off |
| Vulnerable dependency shipped (RUSTSEC-2026-0386) | Tampering/DoS | Lockfile-only tokio bump verified clean by `cargo deny` |
| Slopsquatted tooling | Tampering | cargo-release verified against official repo and the project's own docs; version pinned with `--locked` |

## Sources

### Primary (HIGH confidence)
- Codebase, read and executed this session: `Cargo.toml`, `crates/*/Cargo.toml`, `release.toml`, `Makefile`, `.github/workflows/{ci,release}.yml`, `scripts/{finalize-crate-changelogs,check-release-consistency,check-publish-order,publish-crates,extract-changelog-section,create-or-reuse-release,finalize-release-body}.sh`, `deny.toml`, `.cargo/audit.toml`, `MIGRATION.md`, root and per-crate changelogs, `.project/v0.10.0/09-program-acceptance-audit.md` §13, `37.1-CI-EVIDENCE.md`, `37.1-11-PLAN.md` (hand-off text), STATE/ROADMAP/REQUIREMENTS/MILESTONES.
- Live reads: `index.crates.io` (12 crates × 0.10.1 and 0.11.0), `crates.io/api/v1/crates/<crate>/0.10.1` (`trustpub_data`), GitHub REST (`pulls/59`, `branches/main`, `rulesets`, `actions/runs`, `releases/tags`).
- Experiments: scratch `git worktree` with cargo-release 1.1.7 bump, changelog stamp, `make check-gates`, `cargo update -p tokio --precise 1.53.3`, `cargo deny check`, `cargo audit`; `jq --arg` E2BIG reproduction; worktree removed afterwards (repository state unchanged).
- `gsd-tools query package-legitimacy check --ecosystem crates` (all OK).

### Secondary (MEDIUM confidence)
- WebFetch of cargo-release `src/ops/cargo.rs` (confirms `upgrade_dependency_req` mechanism; the `=` handling itself was settled empirically, not from the doc).
- WebSearch: GitHub release-body 125,000-character limit reports (intuit/auto issue 2418; objectstack PR 5290).

### Tertiary (LOW confidence)
- None used for recommendations beyond the Assumptions Log.

## Metadata

**Confidence breakdown:**
- Standard stack / mechanics: HIGH — each tool run against the real tree.
- Architecture (wave layout, two-SHA, hand-off ordering): HIGH — taken from locked decisions plus 37.1's executed pattern.
- Pitfalls: HIGH for B-1 and B-2 (reproduced); MEDIUM for Pitfall 8-9 (inferred from CI config and disk measurements).

**Research date:** 2026-10-25
**Valid until:** about 7 days or until `main` moves, the advisory DB changes, or the `[Unreleased]` text is edited again (fast-moving: B-1 size and B-2 state change with every edit).
