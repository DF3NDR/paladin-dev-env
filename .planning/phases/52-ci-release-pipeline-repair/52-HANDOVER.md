---
phase: 52
slug: ci-release-pipeline-repair
plan: 08
created: 2026-10-29
---

# Phase 52 -- Maintainer hand-over

This file is the single hand-off for every act only the maintainer can perform (D-08, D-00e) and the record of the phase
gate run (D-00a). The executor printed commands and recorded read-backs; it performed no ruleset write, no label, no
code-scanning dismissal and no remote branch deletion.

Public reads in this file were made unauthenticated against `https://api.github.com` through the sandbox proxy
(the sandbox `gh` token is invalid). Times are UTC.

## Phase gate run

Run 2026-10-29 from the repository root on branch `claude/laughing-dirac-e0h2ax` (head `458d1f46e` when started).
Disk was the constraint: about 1.3 GB free on `/` at the start. Every command below was preceded and followed by
`df -h /`; `No space left on device` appeared in none of them.

| Command | Result | Duration | Notes |
|---------|--------|----------|-------|
| `make check-gates` | exit 0 | 1 s | all offline release-gate guards green (changelogs, crate names, advisory register, workflow suppressions and triggers, CodeQL dismissals, migration allowlist, publish order, dependency allowlist) |
| `make test-shell-guards` | exit 0 | 30 s | 85 assertions passed |
| `make lint-shell` | exit 0 | 7 s | shellcheck clean |
| `make check-migration-allowlist` | exit 0 | under 1 s | allowlist set-equal to the MIGRATION.md section 11.2 register |
| `make check-api-examples` | exit 0 | 2 s | all 105 D-05 public API entry points carry a plural `# Examples` heading |
| `make security` (`cargo audit` then `cargo deny`) | exit 0 | 18 s | advisory database fetched and loaded (1381 advisories, 691 crate dependencies scanned); the existing informational `unmaintained` and `unsound` warnings printed; `advisories ok, bans ok, licenses ok, sources ok` |
| `cargo fmt --all -- --check` (the format step of `make clean-code`) | exit 0 | 3 s | |
| `cargo test --test entity_rename_guard --test legacy_removal_guard` | **not run here: disk allowance** | 36 s, then stopped | compiling the root crate's test dependencies took free space from 1320 MB to 224 MB, under the 250 MB abort line, so the run was stopped before any test executed (exit 143, `aborted-low-disk=1`; the build output is a partial compile, not a test result). About 800 MB of the incremental-compilation files that this attempt created under `target/debug/incremental` were removed afterwards (992 MB free). CI gate: the `Integration Tests` job (`cargo test --workspace --features integration-tests`) compiles and runs both guard tests; the `Coverage` job runs them again. |
| `make clean-code` (clippy, `cargo check`, doc-check) | **not run here: disk allowance** (free space was 992 MB, under the 1 GB line, and the guard-test compile above had already shown a workspace build does not fit) | n/a | the parts that need no build did run and are green: `cargo fmt --check`, `make lint-shell`, `make check-api-examples`. CI gate: `Code Quality` (fmt, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo doc` with zero warnings, all-features `-D warnings`, `# Examples` headings), `MSRV (Rust 1.88)` and `Unit Tests (stable)` / `Unit Tests (beta)` (including `cargo test --workspace --doc`). |
| `make api-surface` | **not run here: disk allowance** | n/a | it builds rustdoc JSON for the whole workspace on the pinned nightly (`nightly-2026-09-20` is installed, `cargo-public-api` is installed); that does not fit in about 1 GB. CI gate: `API Surface Tracking`. This phase did not change any public Rust item, but that claim rests on CI's job, not on a local run. |

The public CI record for the last pushed commit that finished its jobs on the phase branch (`0c2b4a2bd`, run 40807444391)
shows `Code Quality`, `API Surface Tracking`, `Security Audit`, `License & Dependency Policy`, `Workflow Lint`,
`Integration Tests`, `Unit Tests (stable)`, `Unit Tests (beta)`, `MSRV (Rust 1.88)`, `Semver Checks (vs v0.11.0)`,
`Publish Dry Run` and `Coverage` all `completed`, `success`; `End-to-End Tests` `skipped`; `Docker Build` and
`Kubernetes Smoke Test` `cancelled` because a later push superseded the run. Later commits only touched
`.planning/` files. That is evidence for the CI gates named above, not a substitute for them on the final head (H-08).

Sandbox gaps known from earlier plans: `actionlint` and `mdbook` are not installed here (CI's `Workflow Lint` and
`Build MDBook` are the gates).

## Maintainer checklist

Eight items. Each has the decision it closes, the exact commands, the expected read-back, where the evidence goes and a
status line. Reply to the checkpoint with one `H-0N: ...` line per item (see the end of this file).

### H-01 -- Apply the ruleset update for the trunk ruleset (D-08, CIREL-04)

Decision: make `Semver Checks (vs v0.11.0)` a required check (PD-06-1, a planner assumption awaiting user
confirmation) -- confirm or veto. If vetoed, 52-08 Task 3 reverts the ruleset record and docs.

What the PUT does: it replaces the live ruleset `20868126` with the committed record
`.github/rulesets/protect-main-branch.json`, which lists 46 required contexts: the 44 already live, plus `Publish Dry Run`
and `Semver Checks (vs v0.11.0)`. Public read of the live ruleset at 2026-10-29T04:26Z: 44 contexts, neither new name
present, so the PUT has not been applied.

Options:

- `H-01: applied` -- confirms PD-06-1 (semver required) and reports that you ran the PUT and both read-backs.
- `H-01: semver advisory` -- vetoes PD-06-1. Do **not** run the PUT as printed below. Plan 52-08 Task 3 then removes
  `Semver Checks (vs v0.11.0)` from the committed record (45 contexts) and the docs in one commit; you then PUT the
  45-context file.
- `H-01: defer <reason>` -- PD-06-1 stays unconfirmed and is recorded as deferred.

Command (from a checkout of the phase branch, repository root):

```bash
gh api --method PUT \
  -H "Accept: application/vnd.github+json" \
  /repos/DF3NDR/paladin-dev-env/rulesets/20868126 \
  --input .github/rulesets/protect-main-branch.json
```

Never use the create (`POST`) call for this: it adds a second ruleset (see `docs/src/appendix/branch-protection.md`,
"Updating an already-applied ruleset").

Read-back, expected values:

```bash
# exactly three rulesets still exist (a correct update never adds a fourth) -> prints 3
gh api /repos/DF3NDR/paladin-dev-env/rulesets -q 'length'

# the same id now carries 46 required contexts -> prints 46
gh api /repos/DF3NDR/paladin-dev-env/rulesets/20868126 \
  -q '[.rules[] | select(.type=="required_status_checks") | .parameters.required_status_checks[].context] | length'

# both new names are present -> prints 2
gh api /repos/DF3NDR/paladin-dev-env/rulesets/20868126 \
  -q '[.rules[] | select(.type=="required_status_checks") | .parameters.required_status_checks[].context | select(. == "Publish Dry Run" or . == "Semver Checks (vs v0.11.0)")] | length'
```

The same reads work without a token: `curl -fsS https://api.github.com/repos/DF3NDR/paladin-dev-env/rulesets/20868126`.

Timing note: once the PUT is applied, every PR into `main` must report `Semver Checks (vs v0.11.0)` and
`Publish Dry Run`. A PR whose head lacks this phase's `ci.yml` never reports the renamed semver context and stays
blocked until rebased. Apply it when the next PR to merge carries this phase.

Evidence goes: your pasted read-backs in the checkpoint reply; plan 52-08 Task 3 re-reads the public ruleset before it
sets `done`.

Status: done -- PD-06-1 confirmed. The maintainer ran the PUT from their own shell at 2026-10-29T19:33:35Z (response: id 20868126, `enforcement` `active`, `updated_at` `2026-10-29T19:33:35.453Z`) and pasted the read-backs `3` and `46`. Public re-read after the paste: 3 rulesets; 46 contexts; both new names present (count 2); the live context set is identical to `.github/rulesets/protect-main-branch.json` (sorted `diff` empty).

Executor re-read (plan 52-08 Task 3, 2026-10-29T23:18Z, public API, unauthenticated): `GET /repos/DF3NDR/paladin-dev-env/rulesets` returned 3 rulesets; `GET .../rulesets/20868126` HTTP 200, `enforcement` `active`, 46 required contexts, `Publish Dry Run` and `Semver Checks (vs v0.11.0)` both present, and the sorted live context set equals the sorted contexts of the committed `.github/rulesets/protect-main-branch.json`. PD-06-1 is confirmed, so the veto branch of Task 3 did not run and the committed record and docs were left unchanged.

### H-02 -- First-PR observation: `Publish Dry Run` and the `release` label run of `End-to-End Tests` (D-05, D-07, CIREL-04)

On a PR into `main` whose head contains this phase (the PR from the phase branch, once it exists):

1. With no label: confirm `Publish Dry Run` ran and passed, and `End-to-End Tests` was skipped (a skipped job reports
   success to the required-check gate, as on PR #59). A `Publish Dry Run` failure at the already-published `0.11.0`
   version is a finding to triage (RESEARCH A6), not a reason to weaken the gate.
2. Apply the `release` label (this act is yours; the workflow has `labeled` in its `pull_request` types and the e2e job
   runs when the label is present). The full re-run takes roughly 63 minutes (the accepted whole-file re-run).
3. Record the `End-to-End Tests` run URL and conclusion.

Commands (read-only, to observe; no token needed for a public repository):

```bash
PR=<number>
# the PR head's ci.yml run and its two jobs
gh pr checks "$PR" --repo DF3NDR/paladin-dev-env | grep -E 'Publish Dry Run|End-to-End Tests'
# or: curl -fsS "https://api.github.com/repos/DF3NDR/paladin-dev-env/actions/runs?event=pull_request&per_page=5"

# apply the label (maintainer act)
gh pr edit "$PR" --repo DF3NDR/paladin-dev-env --add-label release
```

Expected read-back: before the label, `Publish Dry Run` `success` and `End-to-End Tests` `skipping`/`skipped`; after the
label, a new `ci.yml` run on the same PR in which `End-to-End Tests` is `success` (or a finding if not).

Reply format: `H-02: <PR URL> dry-run <success|failure> e2e-unlabelled <skipped|other> e2e-labelled <run URL> <conclusion>`.

Evidence goes: `52-CI-EVIDENCE.md` `## First PR run` (a failure is written there as a finding for `/gsd-verify-work`).

Status: done -- PR #61 <https://github.com/DF3NDR/paladin-dev-env/pull/61>. Unlabelled `pull_request` run 40928112540: `Publish Dry Run` success (job 123647995031), `End-to-End Tests` skipped (job 123653075412); that run then read `cancelled` when the label event superseded it. Labelled run 40932320347: `End-to-End Tests` success (job 123663512686, 12m25s), `Publish Dry Run` success (job 123658055531). The `release` label did not exist in the repository and was created first (`gh label create release`).

Executor re-read (plan 52-08 Task 3, 2026-10-29T23:18Z, public API, unauthenticated): run 40928112540 job 123647995031 `Publish Dry Run` `success` and job 123653075412 `End-to-End Tests` `skipped`; run 40932320347 job 123663512686 `End-to-End Tests` `success` and job 123658055531 `Publish Dry Run` `success`. Both runs read `cancelled` at run level: 40928112540 was superseded by the label event, and 40932320347 has only `Docker Build` and `Kubernetes Smoke Test` `cancelled` (a later push to the branch superseded it); the H-02 jobs themselves are the `success`/`skipped` conclusions above.

### H-03 -- Semver red-proof close-out (D-04, CIREL-01)

Step 1 (capture the red CI job): **done by the executor**, read-only. Public API read 2026-10-29T04:21:28Z, HTTP 200:
job 123234542712 `Semver Checks (vs v0.11.0)` in run 40808979632 on `scratch/52-semver-red-proof` (head
`6f3d8d7845780d40e6f49b05127b06a60fecce96`) is `completed`, conclusion `failure`; its annotations include
`paladin-storage: lint failure (exit 100)` and the count line
`semver: paladin-storage: 223 checks: 222 pass, 1 fail, 0 warn, 31 skip -- lint failure (exit 100)`. URL:
<https://github.com/DF3NDR/paladin-dev-env/actions/runs/40808979632/job/123234542712>. Recorded under `## Experiment 2 -- CI` in
`52-SEMVER-RED-EVIDENCE.md`, whose frontmatter `status:` is now `ci-proven` (this file was outside plan 52-08's file
list; it closes an H-item with data that became available during this plan, recorded as a Rule 1 deviation).

Step 2 (optional): no pull request exists for the scratch branch (`gh pr create` was refused in this sandbox). Only if you
also want a `pull_request`-event run, open the draft from
<https://github.com/DF3NDR/paladin-dev-env/compare/main...scratch/52-semver-red-proof?expand=1> (title
`DO NOT MERGE: Phase 52 semver red proof`, draft) and close it with `gh pr close <n> --delete-branch`. Skipping this is fine; the
`push` run above is the proof.

Step 3 (remote scratch branch deletion): **yours to run**; the executor did not delete it. It is still on origin
(`git ls-remote` at 2026-10-29T04:27:18Z printed `6f3d8d7845780d40e6f49b05127b06a60fecce96 refs/heads/scratch/52-semver-red-proof`). The
local branch is already deleted.

```bash
git push origin --delete scratch/52-semver-red-proof
```

Expected read-back: this prints nothing (empty output means the branch is gone):

```bash
git ls-remote --heads origin scratch/52-semver-red-proof
```

Evidence goes: your reply (`H-03: done` with the empty read-back, or `n/a`/`defer <reason>`).

Status: done -- `git push origin --delete scratch/52-semver-red-proof` printed `- [deleted]`; `git ls-remote --heads origin scratch/52-semver-red-proof` printed nothing (2026-10-29). Step 2 (draft PR) skipped; the `push` run is the proof.

Executor re-read (plan 52-08 Task 3, 2026-10-29T23:18Z, public API, unauthenticated): `git ls-remote --heads origin scratch/52-semver-red-proof` printed nothing (exit 0).

### H-04 -- CodeQL platform dismissals and reconciliation (D-13, CIREL-06)

The register `CODEQL-DISMISSALS.md` has 37 rows (31 added in this phase plus 6 earlier rows whose alerts are still open
on the platform). The platform side is not done: the sandbox token cannot write code-scanning alerts. The full list of 37
`PATCH` commands is in `52-CARRYOVER-EVIDENCE.md` under `## Platform dismissal commands` (31 new rows, then the 6 earlier
rows). Read the rows first; each command is `-f state=dismissed` with the reason copied from its row. Alert numbers:

- new rows (31): 30, 31, 33, 34, 36, 37, 39, 41, 42, 43, 44, 50, 51, 52, 53, 54, 60, 61, 62, 63, 65, 66, 67, 68, 69, 76,
  77, 78, 79, 80, 81
- earlier rows (6): 28, 40, 45, 46, 47, 49

Shape of each command (the file carries the correct reason per alert; `<reason>` is one of `false positive`, `used in
tests`, `won't fix`):

```bash
gh api --method PATCH /repos/DF3NDR/paladin-dev-env/code-scanning/alerts/<n> \
  -f state=dismissed -f dismissed_reason="<reason>" \
  -f dismissed_comment='Governed in CODEQL-DISMISSALS.md (Phase 52)'
```

To print and run all 37 from the evidence file in one pass (it extracts only the `gh api --method PATCH` lines):

```bash
grep '^gh api --method PATCH /repos/DF3NDR/paladin-dev-env/code-scanning/alerts/' \
  .planning/phases/52-ci-release-pipeline-repair/52-CARRYOVER-EVIDENCE.md | sh -e
```

Reconciliation, after the dismissals (compare the dismissed set with the register's 37 rows):

```bash
gh api "/repos/DF3NDR/paladin-dev-env/code-scanning/alerts?state=dismissed&per_page=100" --paginate
```

Expected read-back: the 37 alert numbers above are all `dismissed`, with the reasons in the file. Out of scope here and
still open on the platform by design: the 20 OSV-Scanner advisory alerts (17 governed by `SECURITY-EXCEPTIONS.md`, 3 ungoverned,
alerts 25, 24, 23; see "Carried deferred items" below).

Evidence goes: your pasted dismissed-alert list in the reply.

Status: done -- all 37 `PATCH` calls returned `dismissed` with the row's reason; `state=dismissed` read-back lists exactly the 37 register rows with identical reasons (`diff` empty: 24 `false positive`, 12 `used in tests`, 1 `won't fix`); 0 CodeQL alerts remain open; the 20 OSV-Scanner alerts remain open by design.

### H-05 -- Issue #41 close (D-15, CIREL-07)

Already closed on the platform, so there is no command to run. Public re-read 2026-10-28T23:21:02Z by plan 52-03 and
again by the executor of plan 52-08 at 2026-10-29T04:21Z (HTTP 200,
`GET https://api.github.com/repos/DF3NDR/paladin-dev-env/issues/41`): `state` `closed`, `state_reason` `completed`,
`closed_at` `2026-10-28T23:09:45Z`, `comments` 1 (the evidence comment, id 6482461863).

Read-back to repeat if wanted:

```bash
curl -fsS https://api.github.com/repos/DF3NDR/paladin-dev-env/issues/41 | python3 -c 'import json,sys; j=json.load(sys.stdin); print(j["state"], j["state_reason"], j["closed_at"], j["comments"])'
```

Expected: `closed completed 2026-10-28T23:09:45Z 1`.

Status: done

Executor re-read (plan 52-08 Task 3, 2026-10-29T23:18Z, public API, unauthenticated): `GET /repos/DF3NDR/paladin-dev-env/issues/41` printed `closed completed 2026-10-28T23:09:45Z 1`. The maintainer replied `H-05: ack`; `done` rests on this read-back, not on the reply word.

### H-06 -- Local `make coverage` on a Docker-capable machine (folded todo 2026-08-13)

The sandbox has no `cargo-llvm-cov` and no reachable Docker daemon (probe output in `52-CI-EVIDENCE.md`
`## Folded todo: local coverage reproduction`), so the local half could not run here and no local figure is claimed.
Procedure (from `.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`, run on a Docker-capable
machine at the phase branch head or a later commit):

```bash
# follow docs/src/contributing/testing-guide.md "Code Coverage" from the top, running only the commands it gives
make services-up
make coverage          # runs scripts/coverage.sh; note the `Lines:` line it prints
git rev-parse HEAD     # record the commit it ran on
```

Expected read-back: a `Lines: <covered>/<total> = NN.NN%` line. Compare with the CI figure at whole-percent precision:
92.74 % on `70657919beadbd341a72be2d0107080017a803f0` (run 40396108246); the `Coverage` job on the phase branch passed
its floor on `0c2b4a2bd` (job 123230155496). Record a mismatch and its cause if identifiable; do not change any gate or
threshold. The todo stays pending (`resolves_phase: 56`).

Reply format: `H-06: <local Lines %> at <SHA>` or `H-06: defer <reason>`.

Evidence goes: the folded-todo section of `52-CI-EVIDENCE.md` (Task 3 adds the figure and verdict).

Status: deferred -- owner: maintainer, on a Docker-capable machine; reason: this sandbox has no Docker daemon and no `cargo-llvm-cov` (re-probed 2026-10-29). The todo stays pending (`resolves_phase: 56`).

### H-07 -- Phase 58 pre-tag audit pointer (D-00f, D-12)

Informational. Phase 58's pre-tag audit reads `52-VERIFICATION.md` and `52-SEMVER-RED-EVIDENCE.md` (now `ci-proven`) from this
phase, and this file's final statuses. The first live exercise of the release-body size guard
(`scripts/release-body-fit.sh`, plan 52-02) is the `v0.11.1` tag. Nothing to run now.

Reply: `H-07: ack`.

Status: done -- acknowledged.

### H-08 -- Phase branch required checks green (D-00a, CIREL-04)

The required checks on the phase branch's latest pushed commit: at least `Workflow Lint`, `License & Dependency Policy`,
`Build MDBook` and `Semver Checks (vs v0.11.0)`.

Read at 2026-10-29T04:21Z (public API, HTTP 200). Latest `ci.yml` run on the branch: run 40816898656, `push`, head
`458d1f46e`, `queued` (not started), so nothing is readable for the head yet. Previous runs, newest first: 40807444391 on
`0c2b4a2bd` `cancelled` (superseded by the next push; `cancel-in-progress`, not a failure), 40802188973 on `5f8e3300b`
`cancelled`, 40801247047 on `4276ce1ad` `cancelled`, 40789100661 on `95659216a` `success`. Job-level reads for the
superseded run 40807444391 (`0c2b4a2bd`): `Workflow Lint`, `License & Dependency Policy` and `Semver Checks (vs v0.11.0)`
`success`. `Build MDBook` belongs to `docs.yml`, which runs on `pull_request` (and `main` pushes) and not on pushes to this
branch, so it has not run and is observable only on the PR.

Commands (read-only, replace `<sha>` with the head you are checking):

```bash
curl -fsS "https://api.github.com/repos/DF3NDR/paladin-dev-env/actions/workflows/ci.yml/runs?branch=claude/laughing-dirac-e0h2ax&per_page=3" \
  | python3 -c 'import json,sys; [print(r["id"], r["status"], r["conclusion"], r["head_sha"][:9]) for r in json.load(sys.stdin)["workflow_runs"]]'
RUN=<run id from above>
curl -fsS "https://api.github.com/repos/DF3NDR/paladin-dev-env/actions/runs/$RUN/jobs?per_page=100" \
  | python3 -c 'import json,sys; [print(j["name"], j["conclusion"]) for j in json.load(sys.stdin)["jobs"] if j["name"] in ("Workflow Lint","License & Dependency Policy","Semver Checks (vs v0.11.0)","Publish Dry Run","Code Quality","API Surface Tracking")]'
gh pr checks <PR> --repo DF3NDR/paladin-dev-env      # includes Build MDBook once the PR exists
```

Expected read-back: every check above `success`; superseded runs may read `cancelled`. The gate commands this sandbox
could not run (clippy, `cargo check`, doc-check, `api-surface`, the two guard tests) are only covered by these CI jobs
(`Code Quality`, `API Surface Tracking`, `Integration Tests`), so a green head run is the evidence for them.

Reply format: `H-08: green` or `H-08: <failing check names>`.

Status: done -- `push` run 40899368946 on the final head `5e993f681`: all 39 jobs `success` except `End-to-End Tests` and `Benchmark Regression Signal (Non-Blocking)` `skipped`; `Workflow Lint`, `License & Dependency Policy`, `Semver Checks (vs v0.11.0)`, `Publish Dry Run`, `Code Quality`, `API Surface Tracking`, `Integration Tests`, `Coverage` all `success`. `Build MDBook` `success` on the PR (`docs.yml` run 40928112482, job 123638982883).

Executor re-read (plan 52-08 Task 3, 2026-10-29T23:18Z, public API, unauthenticated): run 40899368946 (`push`, `5e993f681`) `completed success`, 39 jobs, only `End-to-End Tests` and `Benchmark Regression Signal (Non-Blocking)` `skipped`; docs.yml run 40928112482 job 123638982883 `Build MDBook` `success`. For the later phase-branch head `716923cac`, `push` run 40952344198 is `completed success` (39 jobs, same two `skipped`; `Workflow Lint`, `License & Dependency Policy`, `Semver Checks (vs v0.11.0)`, `Publish Dry Run` `success`), the PR run 40952352062 is `completed success` (39 jobs, none skipped or failed), and `Build MDBook` in docs.yml run 40952352226 is `success`.

## Carried deferred items

From `deferred-items.md` (plan 52-03). None was fixed in Phase 52; each needs an owner outside this phase.

- D-52-03-01: the News API key travels in the request URL (`crates/paladin-content/src/adapters/input/news_api_fetcher.rs`,
  `build_news_api_url`), a `reqwest` error string can echo the URL, and the client follows redirects. Fix is a
  behaviour change with its own test (key to the `X-Api-Key` header, strip the URL from the mapped error,
  `redirect::Policy::none()`).
- D-52-03-02: three advisory alerts with no governing exception: 25 (`RUSTSEC-2026-0221`, `event-listener` 5.4.1), 24
  (`RUSTSEC-2026-0205`, `scc` 2.4.0), 23 (`GHSA-7gcf-g7xr-8hxj`, not identifiable offline). Dismissing them is the
  maintainer's call; the first two are fixed by `cargo update -p event-listener` / `-p scc` (needs network and a build).
- D-52-03-03: stale line anchors in register rows 40 (`rag_retrieval_service.rs:106`, now `:360`) and 49
  (`paladin-server.rs:387`, now `:472`); D-14 keeps those rows byte for byte, so a later sweep must refresh them.

## Maintainer reply

Received 2026-10-29 (the maintainer answered the checklist in a Claude Code session; every act below except the H-01
PUT was executed from that session with the maintainer's repository token, which holds `admin`; the H-01 PUT was run by
the maintainer directly; all were read back live).

```
H-01: applied -- PUT run from the maintainer's shell (updated_at 2026-10-29T19:33:35.453Z); read-backs 3, 46 (pasted), 2 (public re-read)
H-02: https://github.com/DF3NDR/paladin-dev-env/pull/61 dry-run success e2e-unlabelled skipped e2e-labelled https://github.com/DF3NDR/paladin-dev-env/actions/runs/40932320347 success
H-03: done -- `- [deleted] scratch/52-semver-red-proof`; `git ls-remote --heads origin scratch/52-semver-red-proof` -> (empty)
H-04: done -- 37 dismissed; register vs platform diff empty; open CodeQL alerts 0; open OSV alerts 20
H-05: ack
H-06: defer (no Docker-capable machine; resolves in Phase 56)
H-07: ack
H-08: green (push run 40899368946 on 5e993f681; Build MDBook success on docs.yml run 40928112482)
```

Read-backs as pasted:

- H-02, before the label (run 40928112540): `Publish Dry Run ('completed', 'success')`, `End-to-End Tests ('completed',
  'skipped')`; after the label (run 40932320347): `End-to-End Tests ('completed', 'success')`, `Publish Dry Run
  ('completed', 'success')`. The predecessor run then read `completed cancelled` (superseded by the label event, D-07).
- H-03: `To https://github.com/DF3NDR/paladin-dev-env.git` / ` - [deleted]           scratch/52-semver-red-proof`; the
  `ls-remote` read-back printed nothing.
- H-04: the dismissed list (`state=dismissed`, paginated), sorted, was byte-identical to the register's 37
  `<number>|<reason>` rows: 28 used in tests; 30, 31, 33, 34, 36, 37, 39, 40, 41, 42, 43, 44, 45, 46, 47, 49, 50, 51 false
  positive; 52, 53 used in tests; 54, 60, 61 false positive; 62, 63, 65, 66, 67, 68, 69 used in tests; 76, 77 false
  positive; 78 won't fix; 79 false positive; 80, 81 used in tests.
- H-08: run 40899368946 `completed success`, `non-success: []` over all 39 jobs.

- H-01: the maintainer's pasted PUT response carried `"updated_at": "2026-10-29T19:33:35.453Z"` and the 46-entry
  `required_status_checks` list, followed by `3` and `46`. The sandbox permission classifier had refused the same PUT
  earlier in the session, so the maintainer ran it directly; the third read-back (both new names present, `2`) and the
  record-versus-live set comparison (identical) were made by public re-read immediately after the paste.

Intake note (plan 52-08 Task 3, 2026-10-29T23:18Z): this reply reached the plan as the maintainer's own commits `3deb0b829` and `716923cac` on the phase branch, which wrote the statuses above and this block directly into the artifacts; it is kept as the maintainer wrote it. A scan for token-shaped strings (`ghp_`, `github_pat_`, `AKIA`, `sk-`, bearer values) found none, so nothing was redacted.

Nothing remains for the maintainer on this checklist. Merge of PR #61 is the maintainer's act and is outside it.
