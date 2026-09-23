---
phase: 37
slug: v0-10-0-crate-release
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-09-18
validated: 2026-09-23
---

# Phase 37 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
>
> This phase's "tests" are deterministic evidence checks over release-mechanics artifacts, not
> unit tests over application code: no file under `src/` or `crates/*/src/` is modified. The
> checks below are `grep`/`jq`/`curl`/`git`/`gh` assertions plus the existing workspace gates.
>
> **Shorthand used in the map:** `export EV=.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`
>
> **Exit-code honesty (applies to every command below):** `grep -c` exits 1 when its count is
> zero, which is the *passing* case for the no-placeholder and no-unshipped-section checks — so
> every count check captures the value (`C=$(grep -c PATTERN FILE; true)`) and compares it, never
> relying on the exit code. A `cargo test` filter that selects zero tests still exits 0, so the
> two frozen compatibility targets assert their printed `N passed` counts.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | None — evidence checks over files, git objects, the GitHub API and the crates.io index |
| **Config file** | none |
| **Quick run command** | The individual gate command named in the task's own `<verify><automated>` |
| **Full suite command** | `make check-gates` then the six D-06 gate commands then `make publish-dry-run` |
| **Estimated runtime** | Seconds for the grep-shaped checks; ~1-2 min each for the compat targets and semver-checks; ~3-5 min for the MSRV check; several minutes for `make publish-dry-run` (it runs the whole `release-check` chain first) |

---

## Sampling Rate

- **After every task commit:** the task's own `<verify><automated>` command
- **After every plan wave:** the assertions of that plan's tasks, re-run against the committed tree
- **Before `/gsd-verify-work`:** the full D-06 seven-gate set must be green on the recorded head SHA, and the registry table's row count must equal the live `cargo metadata`-derived crate count
- **Max feedback latency:** seconds for the grep-shaped checks; up to ~90 minutes for anything gated on a CI run conclusion (the reason waiting is a hard stop with a resume file rather than a parked watcher)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 37-01-01 | 01 | 1 | SHIP-05 | T-37-04 | Evidence rows carry command, SHA and verbatim result; pre-flight numbers recorded before any gate | integration | `test -f $EV && grep -q '^## Local sweep' $EV && C=$(grep -c TBD MIGRATION.md; true); test "$C" = "0"` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-01-02 | 01 | 1 | SHIP-05 | T-37-01 | No crates.io credential ever reaches the agent, the repository or CI secrets | manual | — (manual-only; see table below) | n/a | ✅ green (manual, observed 2026-09-23) |
| 37-01-03 | 01 | 1 | SHIP-05 | T-37-01 | Registry state, the D-13 non-dispatch and the carried findings are recorded, not fixed | integration | `grep -q 'Findings carried forward' $EV && grep -q 'D-13' $EV` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-02-01 | 02 | 2 | SHIP-05 | T-37-08 | Register guards green and every count-shaped gate asserted on the captured value | gate | `make check-gates && U=$(grep -c '^## \[Unreleased\]' CHANGELOG.md; true); test "$U" = "0"` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-02-02 | 02 | 2 | SHIP-05 | T-37-08 | Frozen compatibility proofs pass with selected-test counts asserted, not exit codes | integration | `cargo test --features web-server --test v0_9_config_boot > /tmp/g2.txt 2>&1 && grep -q '9 passed' /tmp/g2.txt && cargo test -p paladin-web --test openapi_golden_v0_9 > /tmp/g3.txt 2>&1 && grep -q '7 passed' /tmp/g3.txt` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-02-03 | 02 | 2 | SHIP-05 | T-37-09 | No unrecorded API break against the published baseline; MSRV floor clean | gate | `env RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets && for p in paladin-ai paladin-ai-core paladin-ports paladin-battalion paladin-herald paladin-llm paladin-memory paladin-storage paladin-notifications paladin-content paladin-web; do cargo semver-checks check-release --package "$p" --default-features --baseline-version 0.9.0; done` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-03-01 | 03 | 3 | SHIP-05 | T-37-10 | Twelve crates package in dependency order from a clean tree with zero test failures | gate | `test -z "$(git status --porcelain)" && make publish-dry-run > /tmp/g6.txt 2>&1 && C=$(grep -c 'aborting upload due to dry run' /tmp/g6.txt; true); test "$C" -ge 12` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-03-02 | 03 | 3 | SHIP-05 | T-37-11 | Dependencies and licences clean, API surface unmoved, coverage labelled CI-attributed | gate | `make security && make api-surface && test -z "$(git status --porcelain -- .project/current-exports.txt)" && grep -q 'CI-attributed' $EV` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-04-01 | 04 | 4 | SHIP-05 | T-37-13 | `SHIP-05` minted in both places; `SHIP-04` byte-identical | integration | `S=$(grep -c 'SHIP-05' .planning/REQUIREMENTS.md; true); test "$S" -ge 2 && grep -q 'Requirements\*\*: SHIP-05' .planning/ROADMAP.md` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-04-02 | 04 | 4 | SHIP-05 | T-37-14 | Evidence pointer corrected; prior sealed evidence files additions-only | integration | `grep -q "37-CI-EVIDENCE" .planning/STATE.md && A=$(grep -c '37-CI-EVIDENCE' .planning/phases/29-program-gates-release/29-CI-EVIDENCE.md; true); test "$A" = "1"` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-05-01 | 05 | 5 | SHIP-05 | T-37-05 | Section 12 appended and every sign-off box state unchanged by the agent, asserted diff-scoped so an independent maintainer tick cannot invalidate it | integration | `H=$(grep -c '^## 12\. Re-seal' .project/v0.10.0/09-program-acceptance-audit.md; true); test "$H" = "1" && git diff 284c6683~1 284c6683 -- .project/v0.10.0/09-program-acceptance-audit.md > /tmp/d5.txt && B=$(grep -c '^[+-]- \[' /tmp/d5.txt; true); test "$B" = "0"` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-05-02 | 05 | 5 | SHIP-05 | T-37-14 | Pointer paragraph dated and additive only | integration | `grep -q '## 12. Re-seal' .planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md && grep -q '2026-09-18' .planning/phases/29-program-gates-release/29-ACCEPTANCE-AUDIT.md` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-06-01 | 06 | 6 | SHIP-05 | T-37-16 | Full pre-push hook stage run, API-surface baseline unmoved, PR carries the merge-method instruction | integration | `M=$(gh pr view 55 --json state,mergeCommit --jq '"\(.state) \(.mergeCommit.oid)"'); test "$M" = "MERGED 1d4a9724cc219b85856a23012543458d62559e47" && git merge-base --is-ancestor 1d4a9724cc219b85856a23012543458d62559e47 origin/main` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-06-02 | 06 | 6 | SHIP-05 | T-37-17 | Resume condition written as a checkable fact; no watcher parked; the orchestrator's tracking-commit placement rules recorded where a resuming orchestrator reads them | integration | `test -f .planning/phases/37-v0-10-0-crate-release/.continue-here.md && grep -qi 'tracking commit' .planning/phases/37-v0-10-0-crate-release/.continue-here.md && node -e 'JSON.parse(require("fs").readFileSync(".planning/HANDOFF.json","utf8"))' && ! pgrep -f '[g]h run watch'` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-06-03 | 06 | 6 | SHIP-05 | T-37-03 | Maintainer reads the PR body before the CI wait | manual | — (manual-only; see table below) | n/a | ✅ green (manual, observed 2026-09-23) |
| 37-07-01 | 07 | 7 | SHIP-05 | T-37-09 | Every run on the PR head recorded; a red check stops the phase rather than being repaired | integration | `gh run list --branch feature/phase-33 --limit 20 --json workflowName,headSha,event,conclusion > /tmp/runs.json && test -s /tmp/runs.json && grep -q 'CI-run table' $EV` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-07-02 | 07 | 7 | SHIP-05 | T-37-11 | Coverage read from the CI job only, with CI's own digits | integration | `grep -qi 'fail-under-lines' $EV && grep -qi 'CI-attributed' $EV` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-07-03 | 07 | 7 | SHIP-05 | T-37-18 | The commit boundary is declared before the maintainer's tick, in the precise form that covers orchestrator tracking commits too | integration | `grep -q 'N/A-by-deviation' $EV && grep -q 'chore/37-close' $EV` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-08-01 | 08 | 8 | SHIP-05 | T-37-05 | The §11 sign-off box is ticked by the maintainer and by nobody else, over a branch already at parity with `origin` | manual | — (manual-only; see table below) | n/a | ✅ green (manual, observed 2026-09-23) |
| 37-08-02 | 08 | 8 | SHIP-05 | T-37-20 | Tick observed read-only and line-scoped; the `paladin-eval` pre-tag gate resolves down one of its three named branches | integration | `grep -q 'I sign §11' $EV && X=$(grep -c '^- \[[ x]\] \*\*The .v0\.10\.0. tag may be cut\*\*' .project/v0.10.0/09-program-acceptance-audit.md; true); test "$X" = "1" && { CODE=$(curl -s -o /tmp/pe.json -w '%{http_code}' -H 'User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)' https://index.crates.io/pa/la/paladin-eval; true); NN=$(grep -c 'not needed' $EV; true); OK=0; if test "$CODE" = "200" && test -s /tmp/pe.json; then OK=1; fi; if test "$CODE" = "404" && test "$NN" -ge 1; then OK=1; fi; test "$OK" = "1"; }` | ✅ | ✅ green (re-run 2026-09-23) |
| 37-08-03 | 08 | 8 | SHIP-05 | T-37-02 | Merge and tag push performed by the maintainer, never the agent | manual | — (manual-only; see table below) | n/a | ✅ green (manual, observed 2026-09-23) |
| 37-09-01 | 09 | 9 | SHIP-05 | T-37-21 | The tagged commit EQUALS the merge commit, on a two-parent merge, with containment also proven | integration | `git fetch --tags origin && TC=$(git rev-parse v0.10.0^{commit}); MC=$(gh pr list --head feature/phase-33 --state merged --json mergeCommit --jq '.[0].mergeCommit.oid'); test "$TC" = "$MC" && git merge-base --is-ancestor "$TC" origin/main && test "$(git cat-file -t v0.10.0)" = "tag"` | ✅ | ⏭ superseded (never executed; Phase 37.1, D-04) |
| 37-09-02 | 09 | 9 | SHIP-05 | T-37-18 | Post-tag records land on a branch cut from the tagged `main`, never on the feature branch, and any stray commit left on that branch is reconciled rather than stranded | integration | `test "$(git branch --show-current)" = "chore/37-close" && git merge-base --is-ancestor v0.10.0^{commit} HEAD && { S=0; if git rev-parse --verify --quiet refs/heads/feature/phase-33 > /dev/null; then S=$(git rev-list --count HEAD..refs/heads/feature/phase-33); fi; OK=0; if test "$S" = "0"; then OK=1; fi; if grep -q 'stray' $EV; then OK=1; fi; test "$OK" = "1"; }` | ✅ | ⏭ superseded (never executed; Phase 37.1, D-04) |
| 37-09-03 | 09 | 9 | SHIP-05 | T-37-22 | Publish health read from the publish job's own per-crate table in emitted dependency order | integration | `grep -q 'publish-crates' $EV && C=$(grep -c 'paladin-' $EV; true); test "$C" -ge 12` | ✅ | ⏭ superseded (never executed; Phase 37.1, D-04) |
| 37-10-01 | 10 | 10 | SHIP-05 | T-37-24 | Registry row count equals the live derived crate count and the derived count is non-zero | integration | `cargo metadata --no-deps --format-version 1 > /tmp/md.json && node -e 'const m=require("/tmp/md.json");const n=m.packages.filter(p=>p.publish===null).length;if(n===0)process.exit(1);console.log(n)'` plus the per-crate index loop in 37-10 Task 1's own `<verify>` | ✅ | ⏭ superseded (never executed; Phase 37.1, D-04) |
| 37-10-02 | 10 | 10 | SHIP-05 | T-37-25 | The release publish is provably OIDC-based, not credential-based | integration | `curl -sf -o /tmp/pe010.json -H 'User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)' https://crates.io/api/v1/crates/paladin-eval/0.10.0 && grep -q 'trustpub_data' $EV` | ✅ | ⏭ superseded (never executed; Phase 37.1, D-04) |
| 37-11-01 | 11 | 11 | SHIP-05 | T-37-27 | One v0.10.0 release record, carrying the live derived crate count and the registry pointer | integration | `H=$(grep -c '^## v0.10.0' .planning/MILESTONES.md; true); test "$H" = "1" && grep -q 'registry-verified' .planning/MILESTONES.md && grep -q '37-CI-EVIDENCE' .planning/MILESTONES.md` | ✅ | ⏭ superseded (never executed; Phase 37.1, D-04) |
| 37-11-02 | 11 | 11 | SHIP-05 | T-37-28 | Both milestone commands' preconditions enumerated; close-out PR open, not merged by the agent | integration | `test -f .planning/phases/37-v0-10-0-crate-release/37-READY-TO-CLOSE.md && grep -q 'gsd-complete-milestone' .planning/phases/37-v0-10-0-crate-release/37-READY-TO-CLOSE.md && N=$(gh pr list --head chore/37-close --state open --json number --jq 'length'); test "$N" = "1"` | ✅ | ⏭ superseded (never executed; Phase 37.1, D-04) |
| 37-11-03 | 11 | 11 | SHIP-05 | T-37-02 | The milestone close and the close-out merge are the maintainer's acts | manual | — (manual-only; see table below) | n/a | ⏭ superseded (never executed; Phase 37.1, D-04) |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky · ⏭ superseded. Rows for plans 37-09 to 37-11 were
never executed: tag `v0.10.0`'s pipeline published 3 of 12 crates and could not complete forward, so
Phase 37.1 absorbed those waves (plans 37.1-12, 37.1-13, 37.1-15); their properties are validated by
Phase 37.1's own VALIDATION map, not here.*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No test framework, fixture or harness needs
to be created: every command in the map above is either an existing workspace gate (`make
check-gates`, `make publish-dry-run`, `make security`, `make api-surface`, the two frozen
compatibility test targets, `cargo semver-checks`, the MSRV check) that Phase 29 or Phase 33
already exercised on this tree shape, or a read-only `git`/`gh`/`curl` query demonstrated working
during this phase's research.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The §11 sign-off box is ticked | SHIP-05 (SC1) | Human-only by decision (D-00a): the box exists so a person reads the evidence and accepts it; an agent tick would manufacture the authority it represents | Read corpus audit section 12, then §11, then the evidence file's tables; tick the box at roughly line 1549 of `.project/v0.10.0/09-program-acceptance-audit.md`; commit and push it yourself as its own commit touching only that file, and as the branch's last content commit (plan 37-08 Task 1). The file's seven other maintainer sign-off boxes are your own call and outside this phase's criteria — no check here counts them |
| The pull request is merged to `main` | SHIP-05 (SC3) | Maintainer-reserved irreversible act (D-02); the method matters and the repository accepts all three (D-04) | `gh pr merge <PR-number> --merge` — merge-commit method, not squash, not rebase; then capture the merge SHA with `gh pr view <PR-number> --json mergeCommit` |
| The `v0.10.0` tag is created and pushed | SHIP-05 (SC3) | One-way act: the tag push triggers the publish pipeline (D-02); a published version can be yanked but never deleted | Wait for `ci.yml` to conclude success on the merge commit (~60-90 min), then `git tag -a v0.10.0 <merge-sha> -m "v0.10.0 Durable Agent Execution Runtime"` and `git push origin v0.10.0` |
| `paladin-eval`'s first publish is bootstrapped | SHIP-05 (SC4) | Trusted Publishing cannot perform a crate's first publish, and D-00g forbids any agent-held or standing credential | Publish a dependency-free placeholder from a scratch directory outside the repository with a short-lived token, then revoke it (plan 37-01 Task 2); the agent verifies read-only that the sparse index resolves. Replying "not needed — crates.io supports pending publishers" is a third valid path: the continuing 404 is then recorded as a finding and plan 37-08 Task 3's tag hand-off carries a warning block above the tag command naming that the crate does not resolve and that position 11 of 12 rests on your pending-publisher configuration |
| `paladin-eval`'s Trusted Publisher link | SHIP-05 (SC4) | The link is not publicly queryable — the eleven existing rows read "linked (reported)" for the same reason | Configure it in the crates.io UI with repository `DF3NDR/paladin-dev-env`, workflow `release.yml`, environment `crates-io`, and report it; the post-release `trustpub_data` reading (37-10-02) is the observable proof |
| The release PR body is read before the CI wait | SHIP-05 (SC2) | Judgment: whether the gate summary and merge instructions read correctly is not mechanically checkable | Plan 37-06 Task 3's checkpoint — confirm base, head, pointers, the merge-method section and the do-not-tag-yet note |
| The milestone close sequence | SHIP-05 (SC5 preconditions) | D-11 keeps `/gsd-audit-milestone` and `/gsd-complete-milestone` with the maintainer; a plan must not archive the directory it executes from | Run `/gsd-verify-work 37`, `/gsd-validate-phase 37`, `/gsd-audit-milestone`, then `/gsd-complete-milestone v0.10.0` on `chore/37-close`, then merge that PR (plan 37-11 Task 3) |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or are listed in the Manual-Only table above
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (the longest manual run is one task)
- [x] Wave 0 covers all MISSING references (none — existing infrastructure suffices)
- [x] No watch-mode flags
- [x] Feedback latency < 90 s for every non-CI-gated check
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-09-23 by `/gsd-validate-phase 37`

---

## Validation Audit 2026-09-23

| Metric | Count |
|--------|-------|
| Gaps found | 5 (all command drift, none a missing test) |
| Resolved | 5 (command corrections in place, no auditor spawn) |
| Escalated | 0 |

Every executed map row (plans 37-01 to 37-08: 17 automated rows, 4 manual-only) was re-run verbatim on
`chore/37.1-post-close` at `6a2d25ea`. The six cargo gates ran serialized: `make check-gates`,
the two frozen compatibility targets (`9 passed` / `7 passed` asserted), `make security` +
`make api-surface` (baseline unmoved), `make publish-dry-run` (12 dry-run aborts), the MSRV
check on Rust 1.88, and `cargo semver-checks` for all eleven baseline crates (`v0.9.0 -> v0.10.1`,
"no semver update required" each). All green.

Five seeds were red for reasons not visible from the SUMMARYs, and were corrected in place:

- **37-05-01** used `git diff HEAD~1`, which only holds on the commit that appended section 12.
  Pinned to that commit (`284c6683`) so the no-checkbox-change assertion stays diff-scoped forever.
- **37-06-01** asserted branch parity and one *open* PR; PR #55 is merged. Rescoped to the merged
  state and merge SHA (`1d4a9724`) being an ancestor of `origin/main`.
- **37-06-02** self-matched: `pgrep -f 'gh run watch'` finds the checking shell's own command
  line. Seeded `'[g]h run watch'`.
- **37-07-03** grepped "after the tick commit"; no tick commit ever existed because the
  maintainer merged before the plan's tick checkpoint. The evidence records this as
  `N/A-by-deviation` with the boundary declared as `chore/37-close`; the seed now asserts that.
- **37-08-02** required the physical §11 `- [x]` tick. Per D-00a the tick was deferred to the
  maintainer's own hand and the in-session statement ("I sign §11: the v0.10.0 tag may be cut on
  1d4a9724…") is the sign-off of record at `37-CI-EVIDENCE.md` line 890. The seed now asserts
  that statement plus the box line existing exactly once in either state, and the `paladin-eval`
  index resolving (HTTP 200).

Manual-only rows observed 2026-09-23: no crates.io credential entered the repository or CI secrets
(37-01-02, D-00g); the PR body was read at the plan 37-06 checkpoint (37-06-03); the §11 sign-off
of record was the maintainer's in-session statement, never an agent tick (37-08-01); merge of PR
#55 and the `v0.10.0` tag push were the maintainer's acts (37-08-03). The milestone close
(37-11-03) is superseded to Phase 37.1 and remains the maintainer's act.
