---
phase: 47
slug: v0-11-0-crate-release
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-10-25
validated: 2026-10-27
---

# Phase 47 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | bash guard scripts + `tests/scripts/*_test.sh` regression harnesses; `cargo test`; GitHub Actions as the authoritative gate |
| **Config file** | `Makefile` (`check-gates`, `test-shell-guards`), `.github/workflows/{ci,release}.yml` |
| **Quick run command** | `make check-gates && ./scripts/check-release-consistency.sh --tag v0.11.0` |
| **Full suite command** | `make test-shell-guards` plus the eight re-seal rows (see `47-RESEARCH.md` §Validation Architecture) |
| **Estimated runtime** | ~10 seconds (quick run); the heavy re-seal rows (2, 3, 5, 6) are wave-level and take minutes |

---

## Sampling Rate

- **After every task commit:** Run `make check-gates` plus the task-specific structural check
- **After every plan wave:** Run `make test-shell-guards && cargo deny check && cargo metadata --locked --offline`
- **Before `/gsd-verify-work`:** Full suite must be green — all eight re-seal rows green on the final head, CI green on the tick SHA and on the merge SHA, registry table 12/12 at `0.11.0` before any D-16 tick
- **Max feedback latency:** 30 seconds for per-task checks (heavy re-seal rows run per wave, not per task)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 47-01-01 | 01 | 1 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 1: Tracer -- open 47-CI-EVIDENCE.md and record the preflight end  | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; N=$(cargo metadata --format-version 1 --no-deps --offline \| jq '[.packages[]\|select(. …` | ✅ | ✅ green (re-run 2026-10-27) |
| 47-01-02 | 01 | 1 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 2: Install cargo-release 1.1.7, reproduce B-1 offline, answer the | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; cargo release --version \| grep -q '1\.1\.7' && grep -q '_cor_build_payload' "$E" && gr …` | ✅ | ✅ green (re-run 2026-10-27) |
| 47-01-03 | 01 | 1 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | checkpoint — Task 3: The maintainer decides the six pre-build items before any trac | n/a (checkpoint:decision) | n/a | ✅ manual (six-part pre-build decision recorded in 47-01-SUMMARY.md and 47-CI-EVIDENCE.md) |
| 47-02-01 | 02 | 2 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 1: Tracer -- record the maintainer's decision and clear B-2 end t | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; grep -q "Maintainer's pre-build decision (47-01)" "$E" && cargo deny check >/dev/null 2 …` | ✅ | ✅ state-at-the-time — asserts the B-2 commit is *not yet* on origin (`! git merge-base --is-ancestor`); it has been pushed and merged since |
| 47-02-02 | 02 | 2 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 2: Clear B-1, part 1 -- record SHA-PRE-B1, write the pointer para | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; PRE=$(sed -n 's/^- SHA-PRE-B1: \([0-9a-f]\{40\}\)$/\1/p' "$E" \| tail -1); M=$(sed -n ' …` | ✅ | ✅ state-at-the-time — asserts exactly one `## [Unreleased]` heading (pre-stamp tree); the 0.11.0 stamp replaced it in plan 47-03 |
| 47-02-03 | 02 | 2 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 3: Clear B-1, part 2 -- condense `### Changed` the same way, then | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; PRE=$(sed -n 's/^- SHA-PRE-B1: \([0-9a-f]\{40\}\)$/\1/p' "$E" \| tail -1); M=$(sed -n ' …` | ✅ | ✅ state-at-the-time — same pre-stamp `[Unreleased]` assertion as 47-02-02 |
| 47-03-01 | 03 | 3 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 1: Tracer -- commit 1, the lockstep bump with structural proof (1 | `test "$(cargo metadata --format-version 1 --no-deps --offline \| jq -c '[.packages[].version]\|unique')" = '["0.11.0"]' && cargo metadata --format-ver …` | ✅ | ✅ state-at-the-time — asserts HEAD is the `chore(release): bump workspace to 0.11.0` commit; HEAD has moved on |
| 47-03-02 | 03 | 3 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 2: Commit 2 -- stamp the twelve changelogs at DATE, apply R7 and  | `for f in CHANGELOG.md crates/*/CHANGELOG.md; do test "$(grep -c '^## \[0\.11\.0\] - [0-9]\{4\}-[0-9]\{2\}-[0-9]\{2\}$' "$f")" = 1 \|\| exit 1; done && …` | ✅ | ✅ green (re-run 2026-10-27) |
| 47-03-03 | 03 | 3 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 3: Commit 3 -- regenerate the OpenAPI baseline after clearing sta | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; H=$(sed -n 's/^- SHA-H: \([0-9a-f]\{40\}\)$/\1/p' "$E" \| tail -1); PB=$(sed -n 's/^- S …` | ✅ | ✅ green (re-run 2026-10-27) |
| 47-04-01 | 04 | 4 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 1: Tracer -- mint 47-ACCEPTANCE-AUDIT.md at the measured head and | `A=.planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md; E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; R=$(sed -n 's/^- SH …` | ✅ | ✅ state-at-the-time — asserts the sign-off box is still unticked (`- [ ]`); the maintainer ticked it in 47-06 |
| 47-04-02 | 04 | 4 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 2: Run the compile-heavy rows 2, 3, 4 and 6 under the SPACE rule, | `A=.planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md; ! grep -q 'not yet run' "$A" && grep -qE '^\\| 5 \\|.*MSRV \(Rust 1\.88\)' "$A" && …` | ✅ | ✅ state-at-the-time — asserts no `- [x]` in the acceptance audit (unticked by design before 47-06) |
| 47-04-03 | 04 | 4 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 3: House sweep and security gate, the CI-evidence-only entries, a | `A=.planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md; E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; make security >/dev …` | ✅ | ✅ state-at-the-time — same unticked-box assertion as 47-04-02 |
| 47-05-01 | 05 | 5 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 1: Tracer -- push the candidate head to PR #59 and retitle it wit | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; A0=$(sed -n 's/^- SHA-A0: \([0-9a-f]\{40\}\)$/\1/p' "$E" \| tail -1); test -n "$A0" &&  …` | ✅ | ✅ state-at-the-time — asserts PR #59 head equals SHA-A0 (`d4e66b47`); the final head became `72e13f95` after the tick and redate |
| 47-05-02 | 05 | 5 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 2: Write the first resume file and hard-stop until every run on S | `C=.planning/phases/47-v0-11-0-crate-release/.continue-here.md; A0=$(sed -n 's/^- SHA-A0: \([0-9a-f]\{40\}\)$/\1/p' "$C" \| tail -1); test -n "$A0" &&  …` | ✅ | ✅ state-at-the-time — reads the resume file `.continue-here.md`, deleted at phase close |
| 47-06-01 | 06 | 6 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 1: Tracer -- record every run on SHA-A0 per job, tally required c | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; A=.planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md; A0=$(sed -n 's/^- S …` | ✅ | ✅ state-at-the-time — reads `.continue-here.md` (deleted at close) |
| 47-06-02 | 06 | 6 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | checkpoint:human-action — Task 2: The maintainer ticks the sign-off box and pushes the tick comm | n/a (checkpoint:human-action) | n/a | ✅ manual (tick commits `6ef9207a` / `72e13f95` authored by the maintainer; `[x]` at the tag (re-verified 2026-10-27)) |
| 47-06-03 | 06 | 6 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 3: Verify read-only that the ticked box is on origin at PR #59's  | `C=.planning/phases/47-v0-11-0-crate-release/.continue-here.md; P=.planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md; A1=$(sed -n 's/^- S …` | ✅ | ✅ state-at-the-time — reads `.continue-here.md` (deleted at close); the durable truth (tick on origin at the PR head) re-verified below |
| 47-07-01 | 07 | 7 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 1: Tracer -- record the checks on the tick commit SHA-A2 and writ | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; C=.planning/phases/47-v0-11-0-crate-release/.continue-here.md; A2=$(sed -n 's/^- SHA-A2 …` | ✅ | ✅ state-at-the-time — reads `.continue-here.md` (deleted at close) |
| 47-07-02 | 07 | 7 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | checkpoint — Task 2: One-way door -- confirm that the merge and tag may proceed | n/a (checkpoint:decision) | n/a | ✅ manual (one-way decision recorded in 47-07-SUMMARY.md / 47-CI-EVIDENCE.md) |
| 47-07-03 | 07 | 7 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | checkpoint:human-action — Task 3: Merge with a merge commit, then tag the merge commit -- mainta | n/a (checkpoint:human-action) | n/a | ✅ manual (merge commit `70657919` (two parents) tagged `v0.11.0` (annotated); re-verified 2026-10-27) |
| 47-08-01 | 08 | 8 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 1: Tracer -- prove the tag sits on PR #59's merge commit, cut cho | `git fetch -q origin --tags; TC=$(git rev-parse 'v0.11.0^{commit}'); C=.planning/phases/47-v0-11-0-crate-release/.continue-here.md; E=.planning/phases/ …` | ✅ | ✅ state-at-the-time — reads `.continue-here.md` (deleted at close); the durable truth (tag on the merge commit) re-verified below |
| 47-08-02 | 08 | 8 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 2: Record the post-merge ci.yml run and the release.yml run per j | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; TC=$(git rev-parse 'v0.11.0^{commit}'); R=$(gh api "repos/DF3NDR/paladin-dev-env/action …` | ✅ | ✅ state-at-the-time — asserts the current branch is `chore/47-close` (merged as PR #60; the audit runs on `claude/laughing-dirac-e0h2ax`) |
| 47-08-03 | 08 | 8 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 3: Registry verification -- one sparse-index row per derived publ | `E=.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md; L=$(cargo metadata --format-version 1 --no-deps --offline \| jq -r '.packages[]\|select …` | ✅ | ✅ green (re-run 2026-10-27) |
| 47-09-01 | 09 | 9 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 1: Tracer -- re-read the registry live, then write the MILESTONES | `M=.planning/MILESTONES.md; Q=.planning/REQUIREMENTS.md; TC=$(git rev-parse 'v0.11.0^{commit}'); L=$(cargo metadata --format-version 1 --no-deps --offl …` | ✅ | ✅ state-at-the-time — asserts the ROADMAP Phase 47 box is still unticked; `phase.complete 47` ticked it on 2026-10-26 |
| 47-09-02 | 09 | 9 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 2: Write the outbound 47-HANDOFF-web3sec.md and the folded todo's | `H=.planning/phases/47-v0-11-0-crate-release/47-HANDOFF-web3sec.md; T=.planning/todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md; TC=$(git re …` | ✅ | ✅ green (re-run 2026-10-27) |
| 47-10-01 | 10 | 10 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 1: Tracer -- write 47-READY-TO-CLOSE.md against the three success | `R=.planning/phases/47-v0-11-0-crate-release/47-READY-TO-CLOSE.md; test -f "$R" && grep -q '^## Success Criteria — restated against what was observed'  …` | ✅ | ✅ state-at-the-time — asserts the current branch is `chore/47-close` |
| 47-10-02 | 10 | 10 | SHIP-07 | — | N/A (release bookkeeping; no secret handled) | structural gate — Task 2: Push chore/47-close and open the documentation-only pull reque | `R=.planning/phases/47-v0-11-0-crate-release/47-READY-TO-CLOSE.md; test "$(git branch --show-current)" = chore/47-close && test -z "$(git diff --name-o …` | ✅ | ✅ state-at-the-time — asserts the current branch is `chore/47-close` |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

*Populated from the 27 `<task>` blocks of plans 47-01..47-10 by `/gsd-validate-phase 47` on 2026-10-27; commands are abbreviated, the authoritative text is each task's `<automated>` block.*

---

## Wave 0 Requirements

(from `47-RESEARCH.md` §Validation Architecture → Wave 0 Gaps)

- [x] Body-size gate and offline payload rehearsal for the root `[0.11.0]` changelog section (research blocker B-1) — new check, no existing script
- [x] Decide B-2 (tokio RUSTSEC-2026-0386, `cargo deny` red on the PR head) and the D-11 row-7 `[Unreleased]` wording; record both as findings before the build wave
- [x] `cargo install --locked cargo-release --version 1.1.7` and a `df -h /` free-space threshold check
- [x] Empty evidence skeletons: `47-CI-EVIDENCE.md` (three tables), `47-ACCEPTANCE-AUDIT.md` (8 rows + sign-off box)
- [x] Space-recovery decision for re-seal rows 2/3/5/6 (research Pitfall 9: 2.5 GB free against a 20 GB `target/`)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Maintainer ticks the sign-off box in `47-ACCEPTANCE-AUDIT.md` | SHIP-07 | D-00a: only the maintainer ticks; the agent only verifies the tick is on `origin` at the PR head | `git fetch origin && git show origin/claude/laughing-dirac-e0h2ax:.planning/phases/47-v0-11-0-crate-release/47-ACCEPTANCE-AUDIT.md \| grep -c '\[x\].*v0.11.0'` → `1` |
| Release PR #59 merged to `main` with a merge commit | SHIP-07 | D-00b: maintainer-only irreversible act | `git rev-list --parents -n 1 <merge-sha>` shows two parents |
| Annotated `v0.11.0` tag pushed on the merge commit | SHIP-07 | D-00b/D-00c: maintainer-only | `git cat-file -t v0.11.0` = `tag`; `git rev-parse 'v0.11.0^{commit}'` = merge SHA |
| MSRV 1.88 check (re-seal row 5) | SHIP-07 | The 1.88 toolchain is absent in the sandbox; the CI `MSRV` job is the twin | Read the CI `MSRV` job conclusion on the final head via `gh api` (REST) |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (23 automated gates; the 4 without one are the maintainer checkpoints 47-01-03, 47-06-02, 47-07-02, 47-07-03)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify (the longest gap is the two consecutive checkpoints 47-07-02 and 47-07-03, followed by the automated 47-08-01)
- [x] Wave 0 covers all MISSING references (release-body gate `scripts/extract-changelog-section.sh`, evidence skeletons, `cargo-release 1.1.7` all present)
- [x] No watch-mode flags
- [x] Feedback latency < 30s (measured 2026-10-27: 0-4 s per evidence gate; the two gates that run `make security` / `cargo deny` take ~33 s, as the Sampling Rate anticipated for wave-level rows)
- [x] `nyquist_compliant: true` set in frontmatter — set by `/gsd-validate-phase 47` on 2026-10-27

**Approval:** approved 2026-10-27 (`/gsd-validate-phase 47`)

---

## Validation Audit 2026-10-27

| Metric | Count |
|--------|-------|
| Tasks in map | 27 (23 automated evidence gates + 4 maintainer checkpoints) |
| Wave 0 items | 5 of 5 satisfied (recorded in 47-01/47-02 SUMMARYs; tools and scripts present on the tree) |
| Automated gates re-run literally | 23: **6 green** on today's tree; **17 state-at-the-time** (each asserts a condition that was true at its moment in the release sequence and has been deliberately superseded by a later step — an unticked box, a pre-stamp heading count, a not-yet-pushed commit, the `chore/47-close` branch, or the resume file deleted at close). The durable truth behind every one of the 17 was re-verified live today (below) and by 47-VERIFICATION.md on 2026-10-26. |
| Gaps found | 0 |
| Resolved | 0 (none needed) |
| Escalated | 0 |

**Green on today's tree:** 47-01-01 (publishable-crate count and evidence headings), 47-01-02 (`cargo-release 1.1.7`, B-1 reproduction records), 47-03-02 (exactly one `## [0.11.0] - 2026-10-26` heading in each of the 12 changelogs; `make check-gates`; release-consistency version and changelog clauses), 47-03-03 (OpenAPI regeneration records and SHA-H), 47-08-03 (**live sparse-index re-read: one `0.11.0` row per derived publishable crate, 12 of 12**), 47-09-02 (web3sec hand-off and folded todo).

**Durable release truths re-verified 2026-10-27 (git + REST, read-only):** `v0.11.0` is an annotated tag whose commit `70657919…` has two parents, equals PR #59's merge commit and is an ancestor of `origin/main`; the acceptance-audit sign-off box reads `[x]` at the tag; the ROADMAP Phase 47 box and the `SHIP-07` checkbox are ticked; `ci.yml` run 40368801098 and `release.yml` run 40396108246 on that SHA both concluded `success`; 12 of 12 crates read `0.11.0` on crates.io; `cargo metadata` gives 13 workspace packages, all `0.11.0`. Nothing under `src/` or `crates/*/src/` changed in the phase, so there is no code-level test sampling to audit: the phase's Nyquist contract is the evidence-gate chain above, and it held at every task.

**Manual-only items:** the four maintainer checkpoints are recorded verbatim in their SUMMARYs; MSRV 1.88 (re-seal row 5) rests on the CI `MSRV` job, `success` on the final head; the Coverage `Lines:` figure (92.74 %) and the B-1 condensation read were closed by 47-UAT (2/2 pass).
