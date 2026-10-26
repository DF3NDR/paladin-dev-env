# Phase 47: v0.11.0 Crate Release - Pattern Map

**Mapped:** 2026-10-25
**Files analyzed:** 14 (4 created documents + resume file, 10 modified manifests/changelogs/records)
**Analogs found:** 14 / 14 (no Rust source involved; this is a release-documents phase)

All analog paths below were opened and verified this session. The 37.1 phase directory has been archived: the analogs live under `.planning/milestones/v0.10.0-phases/`, not `.planning/phases/`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `.planning/phases/47-.../47-CI-EVIDENCE.md` (new) | evidence record | batch (append-only log) | `.planning/milestones/v0.10.0-phases/37.1-v0-10-1-patch-release/37.1-CI-EVIDENCE.md` | exact |
| `.planning/phases/47-.../47-ACCEPTANCE-AUDIT.md` (new) | sign-off document | request-response (human gate) | `.project/v0.10.0/09-program-acceptance-audit.md` §13 (lines 1627-1759) | exact |
| `.planning/phases/47-.../47-HANDOFF-web3sec.md` (new, outbound) | hand-off document | request-response | `.planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-HANDOFF-web3sec.md` (inbound; mirror) | role-match (direction reversed) |
| `.planning/phases/47-.../.continue-here.md` (new, resume file) | resume file | event-driven (hard stop) | `.planning/milestones/v0.10.0-phases/37-v0-10-0-crate-release/.continue-here.md` | exact |
| `.planning/phases/47-.../47-READY-TO-CLOSE.md` (new, last plan, D-17) | hand-off document | batch | `.planning/milestones/v0.10.0-phases/37.1-v0-10-1-patch-release/37.1-READY-TO-CLOSE.md` | exact |
| `.planning/MILESTONES.md` (modify) | release record | CRUD (prepend entry) | v0.10.0 entry, lines 3-20 | exact |
| `.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md` (modify) | todo | CRUD (dated append) | same file's `## Update 2026-10-16/19/24` blocks | exact |
| `Cargo.toml` + 12 `crates/*/Cargo.toml` (modify) | config/manifest | transform (tool-driven) | `cargo release version` output; no prior diff to copy, mechanics in RESEARCH Pattern 1 | tool-defined |
| `Cargo.lock` (modify) | config/lock | transform | produced by `cargo release version` (13 workspace entries) | tool-defined |
| `CHANGELOG.md` + 11 `crates/*/CHANGELOG.md` (modify) | changelog | transform | `scripts/finalize-crate-changelogs.sh` output; root v0.10.1 section as shape reference | tool-defined |
| `crates/paladin-web/openapi.json` (modify) | generated artifact | transform | `make openapi` output; line 14 only | tool-defined |
| PR #59 title/body (REST write, no file) | release PR | request-response | PR #55 / #56 shape; PATCH command in RESEARCH Pattern 4 | role-match |
| ROADMAP / REQUIREMENTS / STATE ticks | planning state | CRUD | `gsd-tools` handlers only (D-16), never direct edits | n/a |

## Pattern Assignments

### `47-CI-EVIDENCE.md` (evidence record, append-only log)

**Analog:** `.planning/milestones/v0.10.0-phases/37.1-v0-10-1-patch-release/37.1-CI-EVIDENCE.md` (2092 lines; copy structure, not volume)

**Top-level section order** (verified via `grep -n '^#'`; keep exactly this order, with dated `###` entries appended beneath):
1. Title `# Phase 47 v0.11.0 Crate Release — CI Evidence Record (plans 47-01 onward)` plus `**Phase:** / **Branch:** / **Written:**` lines (37.1 lines 1-5; here Branch is `claude/laughing-dirac-e0h2ax`).
2. House-form preamble paragraph (lines 7-14): "follows the 29/33/37 house form: a Local sweep table, a CI-run table, a Registry verification (D-00h) table and a Findings carried forward (D-00d) section". Add 37.1 to the list. State that the older evidence files are sealed and receive dated additions only.
3. `## Provenance` (lines 16-57): bullets for base commit (`df24689c`, main tip), branch head SHAs, toolchain line (rustc `1.97.1`, MSRV `1.88`, `cargo-deny`, `cargo-semver-checks 0.50.0`, `cargo-release 1.1.7`), tick commit, merge commit with two parents, tagged commit **equality note** (tag object peels to the merge commit AND `GET pulls/N .merge_commit_sha` is identical). Fill the tick/merge/tag bullets post-tag.
4. `## Local sweep` (line 59). Table header, verbatim:
```markdown
| # | Command | Result (verbatim/summarized) | Verdict |
|---|---------|-------------------------------|---------|
```
5. `## CI-run table` (line 128) with per-plan `###` subsections ("Plan 37.1-09 — every workflow run on H1, the required-context tally...", line 300). Run-listing table shape (lines 1500-1511):
```markdown
| Workflow | Run ID (URL) | Event | Conclusion |
|---|---|---|---|
| `ci.yml` | [35636377861](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35636377861) | `push` | `success` |
```
Record required-context tally against the live ruleset (RESEARCH says 44 contexts), skipped jobs by name (`End-to-End Tests`, `Publish Dry Run` skip on PRs), and the `Coverage` job's own printed `Lines: N/M = X%` line against the 82 % floor. **Deviation required:** 37.1 used `gh pr checks` (GraphQL); GraphQL is blocked here, so read per-job conclusions with `gh api repos/DF3NDR/paladin-dev-env/actions/runs/$RUN/jobs --paginate --jq '.jobs[]|{name,conclusion}'` (RESEARCH "Code Examples").
6. `## Registry verification (D-00h)` (line 426), empty skeleton before the tag:
```markdown
| Crate | Version | Checksum | `yanked` | Notes |
|-------|---------|----------|----------|-------|
```
Fill with the 37.1 formal form (lines 1769-1800): derived count block, then
```markdown
| Crate | `vers` | `cksum` | `yanked` |
|---|---|---|---|
| paladin-ai | `0.11.0` | `<64 hex>` | `false` |
```
plus the **Assertions** paragraph (row count equals derived count; derived count greater than zero; every row `vers == "0.11.0"` and `yanked == false`) and an elapsed-time note between publish completion and the read. Also copy the "Publish outcome" per-crate table (lines 1702-1715: `| Crate | Outcome |` with `published-now` / `already-at-this-version`) and the "**This order matches `scripts/publish-crates.sh`'s own `CRATES` array exactly**" cross-check.
7. `## Findings carried forward (D-00d)` (line 436): dated `### YYYY-MM-DD — title` entries, amend-at-source. Phase 47 entries to seed (from CONTEXT/RESEARCH): D-09 semver baseline `0.9.0` is a no-op (`0 checks, 254 skip`, `paladin-eval` excluded); D-14 `check-migration-allowlist` and `Publish Dry Run` have no PR twin; B-1 release-body size; B-2 RUSTSEC-2026-0386; row-7 `[Unreleased]` anchor wording; empty `[0.11.0]` sections for `paladin-content`/`paladin-memory`/`paladin-notifications`; `k8s/server/worker-deployment.yaml` `version: v0.10.0` labels; shallow-clone commit counts taken from GitHub; MIGRATION §10 freeze check (`e28e43b5..HEAD -- src crates/*/src`).

**Registry probe command (reuse verbatim; 37.1 lines 1735-1737, tested live in RESEARCH Pattern 3):**
```bash
cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.publish == null) | .name' | sort | while read -r c; do
  n=${#c}; if [ $n -eq 1 ]; then p="1/$c"; elif [ $n -eq 2 ]; then p="2/$c"; elif [ $n -eq 3 ]; then p="3/${c:0:1}/$c"; else p="${c:0:2}/${c:2:2}/$c"; fi
  row=$(curl -sf -H 'User-Agent: paladin-release-verify (github.com/DF3NDR/paladin-dev-env)' "https://index.crates.io/$p" \
        | jq -c --arg v "$V" 'select(.vers==$v) | {vers,yanked,cksum}' || true)
  echo "$c | ${row:-ABSENT}"
done   # V=0.11.0; pre-tag expectation: 12 x ABSENT
```
Note: `crates/paladin-core` publishes as `paladin-ai-core`; always use `cargo metadata` names.

**Post-event entry style to copy** (37.1 lines 1398-1414, "Tag pushed by the maintainer; verified from the remote"): bullet "The tag push is the maintainer's act (D-00b)", `git ls-remote --tags origin 'v0.11.0*'` giving tag object and peeled SHA, annotated type `tag`, release run ID with `event push`, jobs already concluded, ending "Not waited on (D-00i)."

**Red-gate entry style** (37.1 line 961 "RED required check on the tick commit", D-08 stage 2): record command, SHA, failing output; "nothing fixed, nothing re-run". Apply D-00j.

---

### `47-ACCEPTANCE-AUDIT.md` (sign-off document)

**Analog:** `.project/v0.10.0/09-program-acceptance-audit.md` §13 "Re-seal for the v0.10.1 release" (lines 1627-1759) and its box at 1747. Read-only; do not edit it (D-10).

**Shape to copy:**
1. Short preamble (CONTEXT discretion: yes, 2-3 sentences drawn from `MIGRATION.md` §10 opening paragraph, no new claims).
2. "**Head SHA this section's gates were measured on:** `<40-hex>`" paragraph, stating `git rev-parse HEAD` re-confirmed before the first gate and `git status --porcelain` empty; give the `git diff --stat <prev>..<head> -- . ':!.planning' ':!.project'` restricted to files a gate could read (§13 lines ~1645-1650, §14 line ~1780).
3. The gate table, header verbatim from line ~1656 (same 6 columns):
```markdown
| # | Gate | Command | SHA | Result | Evidence |
|---|------|---------|-----|--------|----------|
```
Rows to carry (D-11), with the exact commands §13 used, re-read at `0.11.0`:
- 1: `grep -c TBD MIGRATION.md` then `make check-migration-allowlist` (now reads §9.2 AND §10.2); Evidence column must say "No PR run reconfirms this row" (D-14).
- 2: `cargo test --features web-server --test v0_9_config_boot`; CI twin `E2E Platform API ...` job.
- 3: `cargo test -p paladin-web --test openapi_golden_v0_9`; CI twin `Integration Tests`.
- 4: `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` once per crate, list from `cargo metadata` minus `paladin-eval`, asserted at exactly 11 before looping; result records the known no-op `0 checks: 0 pass, 254 skip`.
- 5: `env RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets`; RESEARCH says toolchain 1.88 is absent here, so the Evidence column must name the CI `MSRV (Rust 1.88)` job as the twin and say the local row was not run (never claim a local pass).
- 6: `make publish-dry-run` (twelve crates, dependency order); no PR twin.
- 7: `make check-gates` plus hard-assertion greps. **Wording correction required (RESEARCH Pitfall 3):** after the stamp, `## [Unreleased]` count is 1 unless the empty anchor is deleted in the stamp commit; decide and state which. Add the body-size assertion (`extract-changelog-section.sh` output at most 110000 bytes, RESEARCH B-1).
- 8: `make check-publish-order`, with the full `✅ OK: 12 crate(s) checked in publish order; ...` line as the Result.
4. Paragraph after the table on gates outside the numbered table (§13 lines ~1672-1695): coverage floor CI-attributed (record the `Lines: a/b = x%` figure), house sweep (`make clean-code`, `make security`, `make api-surface`), and the standing "no static taint analysis of first-party Rust gates a merge" statement from `security.instructions.md`. Per D-12, `api-surface`, `check-consumer-surface`, and `coverage` are CI-evidence entries, not signed rows.
5. `**Findings:**` list in `- *(date, plan, owner: ...)* — text` form (§13 lines ~1697-1740).
6. **Sign-off box, wording to copy (§13 line 1747), set to `v0.11.0`, unticked:**
```markdown
- [ ] **The `v0.11.0` tag may be cut** — evidence: this section in full, plus
  `.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md`'s complete Local sweep table
  and its CI-run table (including the `coverage` job this sandbox cannot measure).
  Unticked, as designed — this box belongs to this release alone. It is closed by a human — the
  maintainer, and nobody else — at this phase's own sign-off checkpoint, never by this audit or
  any agent, under any mode (Phase 29 D-17, Phase 37 D-00a, this phase's D-00a).
```
Lead-in sentence (line ~1547 form): "**The `v0.11.0` tag may be cut once a human reads this section and the linked evidence:**". An agent must never write `[x]`.

**Re-seal restart rule** (§14 lines 1760-1775): if a red gate forces a return to the build wave, do not edit the earlier box; append a new dated section mirroring the shape at the new head, saying the new box is the one the merge waits on.

---

### `47-HANDOFF-web3sec.md` (outbound hand-off)

**Analog:** `.planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-HANDOFF-web3sec.md` (220 lines; inbound from web3sec). Mirror the structure with directions swapped; also consult `.planning/phases/46-docs-currency-hygiene/46-FEEDBACK-web3sec.md` for the inbound form (not read in detail this session).

**Header pattern (lines 1-8):**
```markdown
# Handoff: Adopt Paladin `v0.11.0` in the Web3 Security Paladin consumer

**From:** Paladin (`DF3NDR/paladin-dev-env`), the framework maintainer
**To:** the Web3 Security Paladin ("web3sec") consumer agent
**Date:** <tag date>
**Purpose:** enough context to re-pin and gate the consumer **without access to Paladin's planning files**.
```
Section skeleton to copy (49's headings): `## 1. TL;DR`, `## 2. Background you need`, `## 3. Current state (verified <date>)`, then the breaking-change/adaptation section (here: pointers only, not restated: `MIGRATION.md` §10.8 "Library consumers: v0.10.1 -> v0.11.0", DR rows, and `tests/consumer_surface_muster.rs` with `make check-consumer-surface`), `## Decisions to make`, `## Return contract`, `## Constraints and conventions`, `## Out of scope`.

**TL;DR content (D-15):** tag `v0.11.0`, merge SHA, release run ID, the twelve crates registry-verified at `0.11.0` (cite the `47-CI-EVIDENCE.md` registry table), the single-bump rule (one coordinated pointer bump; detach the `paladin` submodule on the tag, one commit carrying pointer + adaptation + `Cargo.lock`).

**Return contract (49 lines 165-174; copy the numbered form and adapt, fields per the folded todo step 5):**
```markdown
## 9. Return contract (what Paladin needs back)
1. The consumer commit SHA containing the pointer bump to the `v0.11.0` tag.
2. The final D-2 choice (lossless `TokenUsage` carried through `RoutineReport`, not total-only), and which web3sec types changed.
3. Gate output: fmt, clippy, check, and test totals (pass/fail/ignored) against the 2,592/0/2 baseline.
4. Any divergence from this handoff, including real `use` lines not covered by the muster.
```
Constraints section (49 lines 176-182) carries the Medieval Military ubiquitous-language and hexagonal rules; reuse as written. Out-of-scope: web3sec feature work, any new Paladin release unless the return surfaces a library defect.

**Todo update (modify, do not rewrite):** append to `.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md` a dated block in the existing style, heading form `## Update 2026-MM-DD (Phase 47 tag cut, hand-off written)`; keep frontmatter `pending`. Existing update blocks are at "Update 2026-10-16 (Phase 50 closeout, plan 50-16)", "Update 2026-10-19 (Phase 50.1 inserted)", "Update 2026-10-24 (Phase 46)", and the last one already ends "Status stays pending (D-25)."

---

### `.continue-here.md` (resume file, hard stop at merge + tag checkpoint)

**Analog:** `.planning/milestones/v0.10.0-phases/37-v0-10-0-crate-release/.continue-here.md`

**Frontmatter and sections to copy:**
```markdown
---
context: phase
phase: 47-v0-11-0-crate-release
plan: NN
task: N
total_tasks: N
status: paused
last_updated: "<ISO-8601 Z>"
---

# BLOCKING CONSTRAINTS — Read Before Anything Else
> These are not suggestions. Each constraint below was discovered through failure.
- [ ] CONSTRAINT: D-00j red-gate discipline — ... record (command, SHA, failing output) and halt; fix nothing.
- [ ] CONSTRAINT: D-00b — merge and tag are exclusively the maintainer's acts.
**Do not proceed until all boxes are checked.**

## Critical Anti-Patterns
| Pattern | Description | Severity | Prevention Mechanism |
```
Then `<current_state>`, `<completed_work>`, and the exact resume condition (D-00i: a named condition, e.g. "`origin/main` is a two-parent merge commit with `ci.yml` completed success on it", never a parked `gh run watch`). Anti-pattern row to carry: "Auto-approving the merge+tag checkpoint" mitigated by `gate="blocking-human"`. Note the 37 file also had a `.planning/HANDOFF.json` pair; check whether that is still the current convention before planning it.

---

### `47-READY-TO-CLOSE.md` (last plan output, D-17)

**Analog:** `.planning/milestones/v0.10.0-phases/37.1-v0-10-1-patch-release/37.1-READY-TO-CLOSE.md`. Header: `# Phase 47 — Ready to Close`, `**Status:** Ready to close. No plan in this phase runs /gsd-audit-milestone or /gsd-complete-milestone — this document is the hand-off`. Then `## Success Criteria — restated against what was observed`, one `### SCn — ...` with `**Observed: TRUE.**` plus evidence pointer per criterion (the three ROADMAP criteria), then prerequisites for `/gsd-audit-milestone` and `/gsd-complete-milestone v0.11.0`. Observed-state must come from the registry table, not from the merge.

---

### `.planning/MILESTONES.md` (release record, modify)

**Analog:** v0.10.0 entry, lines 3-20. Prepend a new `## v0.11.0 Treasurer Spend Governance (Shipped: <date>)` block above it (confirm exact milestone title from ROADMAP). Field list to reproduce, in order:
```markdown
**Phases completed:** ...
**Requirements:** N/M satisfied (...)
**Timeline:** <start> → <end> (N days, M commits since tag `v0.10.1`)
**Git range:** `<sha>` (milestone opened) → `<sha>` (close)
**Closeout type:** ...
**Audit:** `milestones/v0.11.0-MILESTONE-AUDIT.md` (status ...)
**Tags:** `v0.11.0` on merge commit `<sha>` (<date>; release run `<id>` green, all 12 publishable crates on crates.io at `0.11.0` via Trusted Publishing).
```
(The v0.10.0 Tags line reads: "`v0.10.1` on merge commit `f7dae267` (2026-09-21; release run `35659477719` green, all 12 publishable crates on crates.io at `0.10.1` via Trusted Publishing).") D-17 requires: tag, merge SHA, release run ID, crate count, "registry-verified", Trusted Publishing. The commit count must come from GitHub (shallow clone), say so. Closeout fields (`Closeout type`, `Delivered:`) belong to `/gsd-complete-milestone`; this phase writes only the release-record fields D-17 lists. The audit currently reads `gaps_found` with SHIP-07 the only gap, so do not claim `tech_debt`/clean until SHIP-07 is ticked.

---

### Manifests, changelogs, OpenAPI (modified, tool-driven; no prose analog)

No existing file to copy; the mechanics are the pattern. Planner should copy from RESEARCH verbatim:
- **Commit 1 (`chore(release): bump to 0.11.0`):** `cargo release version 0.11.0 --execute --no-confirm --workspace`; expect 14 files (root + 12 `crates/*/Cargo.toml` including `doc-examples`, plus `Cargo.lock`). Includes the exact pin `crates/paladin-ports/Cargo.toml:16` (`=0.10.1` -> `=0.11.0`). Verification: filtered grep-zero (comment lines excluded), the `jq -e` structural proof (path deps with `req != "*"` must match `0.11.0`; three path-only dev-deps carry `req "*"` by design), `cargo metadata --locked --offline`. Install: `cargo install --locked cargo-release --version 1.1.7`.
- **Commit 2 (`chore(release): stamp 0.11.0 changelogs`):** `./scripts/finalize-crate-changelogs.sh --version 0.11.0 --date <tag-date>` (the `make` target stamps today only). Then `make check-gates && ./scripts/check-release-consistency.sh --tag v0.11.0`.
- **Commit 3 (`chore(release): regenerate OpenAPI baseline`):** `make openapi`; expected diff is `crates/paladin-web/openapi.json` line 14 only; anything else is a D-07 stop.
- Optional preceding `fix(deps)` commit for B-2 (`cargo update -p tokio --precise 1.53.3`, lockfile only) and the B-1 root-changelog condensation, both only after the maintainer decision checkpoint. D-08 forbids absorbing them silently.

**Commit message style:** conventional commits per CLAUDE.md; append the Co-Authored-By and Claude-Session trailers from the attribution reminder.

---

### PR #59 retitle/body (REST write)

**Analog:** v0.10.1's PR #56 (opened with `gh pr create --title "release: v0.10.1 — ..."`, 37.1-CI-EVIDENCE lines 130-195) and PR #55. GraphQL (`gh pr ...`) is blocked here, so use the REST form:
```bash
gh api -X PATCH repos/DF3NDR/paladin-dev-env/pulls/59 -f title='release: v0.11.0 — Treasurer Spend Governance' -F body=@"$SCRATCH/pr-body.md"
```
Body contents per D-03: re-seal summary, pointers to `CHANGELOG.md` `[0.11.0]`, `MIGRATION.md` §10, `47-CI-EVIDENCE.md`, `47-ACCEPTANCE-AUDIT.md`, and the twelve-crate list from `cargo metadata`. Write is untested (RESEARCH A1); if refused, print the exact command for the maintainer.

---

## Shared Patterns

### Amend-at-source, dated additions (D-00d)
**Apply to:** every evidence file, the todo, MILESTONES.
Sealed records are never edited; additions are new dated `### YYYY-MM-DD — title` entries. Example: the 37.1 preamble (lines 7-14) says older evidence files "receive dated additions only, never an edit".

### Human-only acts (D-00a, D-00b)
**Apply to:** `47-ACCEPTANCE-AUDIT.md`, the merge+tag hand-off, `.continue-here.md`.
Box text always includes "closed by a human — the maintainer, and nobody else ... never by this audit or any agent, under any mode". Merge: merge commit only. Tag command to hand over: `git tag -a v0.11.0 <merge-sha> -m "v0.11.0"` then `git push origin v0.11.0`. The 37.1 tag-push record notes GitHub logs `Bypassed rule violations for refs/tags/...` (admin bypass of "Protect release tags"); tell the maintainer this is expected.

### Sign-off ordering enforced on `origin` (D-13)
**Apply to:** hand-off plan.
Read-only verification before writing the merge+tag hand-off: `git show origin/claude/laughing-dirac-e0h2ax:.planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md | grep -n '\[x\] \*\*The `v0.11.0` tag may be cut'` at the PR head SHA, plus CI success on that SHA. 37.1 precedent: lines 853-960 ("read-only verification that the ticked box is present on the remote at PR #56's head").

### Two-SHA rule (Phase 29 D-21)
**Apply to:** `47-CI-EVIDENCE.md`. Evidence is recorded for A (PR head with bump and tick) and B (merge commit that gets tagged). The tick commit changes the PR head, so CI must be re-read on the tick SHA.

### Post-merge pre-tag read-only checks
**Apply to:** hand-off Step 2 (from RESEARCH "Code Examples"):
```bash
git fetch origin --tags; B=$(git rev-parse origin/main)
git rev-list --parents -n 1 "$B"            # commit + two parents = merge commit
gh api "repos/DF3NDR/paladin-dev-env/actions/workflows/ci.yml/runs?head_sha=$B&status=completed" --jq '.workflow_runs[]|{conclusion,id,event}'
```
Warn in the hand-off text that `End-to-End Tests` and `Publish Dry Run` run for the first time on B (RESEARCH Pitfall 8); a red B means no tag.

### Registry-as-proof, derived crate list (D-00f, D-00h)
**Apply to:** `47-CI-EVIDENCE.md`, `47-HANDOFF-web3sec.md`, MILESTONES, D-16 ticks. Crate list always from `cargo metadata ... select(.publish == null)`; the workflow log is never proof. D-16 ticks (SHIP-07, ROADMAP, REQUIREMENTS, STATE) happen only after 12/12 rows show `0.11.0`, `yanked=false`, through `gsd-tools` handlers.

### Red-gate policy (D-00j)
**Apply to:** all execution plans. Record command, SHA, failing output; stop at a blocking checkpoint; fix nothing; one infra-flake re-run max with both run IDs and the justifying log line.

### Waiting is a stop (D-00i)
**Apply to:** CI waits and the merge/tag wait. Write `.continue-here.md` with the exact resume condition; never a parked `gh run watch`.

### Sandbox constraints
**Apply to:** every plan. GraphQL blocked (use `gh api` REST; `gh auth status` misleadingly reports an invalid token). Shallow clone (commit counts from GitHub). No Docker (coverage is CI-only). 2.5 GB free disk (preflight `df -h /`, RESEARCH Pitfall 9). No MSRV 1.88 toolchain locally. Job logs are on a proxy-blocked host; use `/jobs` step conclusions.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| Body-size gate and offline payload rehearsal (RESEARCH B-1, Wave 0) | check | batch | No existing script; RESEARCH proposes `wc -c` on `extract-changelog-section.sh` output plus sourcing `create-or-reuse-release.sh` with `CREATE_OR_REUSE_RELEASE_LIB_ONLY=1` and calling `_cor_build_payload`. Implement as an inline command in the evidence, not a script edit (D-08/D-09). |
| Condensed root `CHANGELOG.md` `[0.11.0]` section | changelog | transform | Needs a maintainer-approved condensation (156,528 chars now; at most about 110,000 bytes). No prior release needed it (v0.10.0: 42,708; v0.10.1: 3,732). |
| tokio advisory remedy (RUSTSEC-2026-0386) | dependency | transform | Lockfile-only `cargo update -p tokio --precise 1.53.3` as its own `fix(deps)` commit before the bump; fallback is the governed-exception route (`deny.toml`, `.cargo/audit.toml`, `SECURITY-EXCEPTIONS.md`, kept in sync by `check-advisory-register.sh`). Maintainer decision first. |
| `chore/47-close` branch + docs-only PR | process | event-driven | Precedent exists (`chore/37.1-close`, `chore/37-close`, 37.1-CI-EVIDENCE line 2060 "close-out branch pushed, documentation-only pull request opened"); no file to copy beyond that evidence entry. |

## Notes for the Planner

- Archive path correction: CONTEXT.md names `.planning/milestones/v0.10.0-phases/37.1-.../37.1-CI-EVIDENCE.md`, which is correct; SUMMARY/PATTERNS text elsewhere may cite `.planning/phases/37.1-...`, which no longer exists (the audit §13 text itself cites that old path).
- The CONTEXT-named line numbers verified: 37.1 registry table ~1419 is actually the first table at line 426 (empty skeleton) and the formal filled table is at lines 1769-1800; the probe command is at lines 1735-1737 and 1780-1783. Audit §13 starts at line 1627 and the box at line 1747.
- 49-HANDOFF's `## 9. Return contract` is at lines 165-174 (CONTEXT cites §9; correct).
- `46-FEEDBACK-web3sec.md` exists in `.planning/phases/46-docs-currency-hygiene/`; I did not read it in detail. The planner should skim it before writing the hand-off if the inbound form matters beyond 49-HANDOFF.
- The phase directory currently holds `47-CONTEXT.md`, `47-DISCUSSION-LOG.md`, `47-RESEARCH.md`, `47-VALIDATION.md`; no evidence skeletons exist yet (RESEARCH Wave 0 gap).

## Metadata

**Analog search scope:** `.planning/milestones/v0.10.0-phases/37-*`, `37.1-*`; `.project/v0.10.0/`; `.planning/phases/49-*`, `46-*`; `.planning/MILESTONES.md`; `.planning/todos/pending/`.
**Files read (full or targeted ranges):** 47-CONTEXT.md, 47-RESEARCH.md, 37.1-CI-EVIDENCE.md (ranges), 09-program-acceptance-audit.md (§13/§14 ranges), 49-HANDOFF-web3sec.md (header and §9-§11), MILESTONES.md (lines 1-30), 37 `.continue-here.md`, 37.1-READY-TO-CLOSE.md (head), the folded todo.
**Pattern extraction date:** 2026-10-25
