# Phase 58 v0.11.1 Crate Release — CI Evidence Record

**Phase:** 58-crate-release
**Branch:** `claude/laughing-dirac-e0h2ax`
**Written:** 2026-11-05 (plan 58-01 onward)

This record follows the `47-CI-EVIDENCE.md` house form: a **Local sweep** table, a **CI-run table**, a
**Registry verification (D-00h)** section and a **Findings carried forward (D-00d)** section. Every number in
it is the output of a command run in the plan that records it, beside the SHA it ran on; a figure copied from
RESEARCH.md that was not re-measured is labelled as a research figure.

---

## Provenance

- **Branch:** `claude/laughing-dirac-e0h2ax`.
- **`main` (D-02):** `8b23cb4c55a10fcf0b08fab1af595e1f184e5f2e`, read live through REST
  (`gh api repos/DF3NDR/paladin-dev-env/branches/main --jq .commit.sha`) at 2026-11-05T03:18Z, at the start
  of plan 58-01; it equals the frozen D-02 baseline.
- **Plan start head:** `SHA-PRE` below (`git rev-parse HEAD` before any 58-01 commit).
- **Ahead of `origin/main`:** `git rev-list --count origin/main..HEAD` = 366 on this shallow clone
  (`.git/shallow` present); the count is local only, GitHub's count is authoritative.
- **`origin/claude/laughing-dirac-e0h2ax` at plan start:** `adf9472ea5cbe970ab4d05abb3449a877e4a8c7b`
  (`git ls-remote origin refs/heads/claude/laughing-dirac-e0h2ax`).
- **Toolchain:** `rustc 1.97.1 (8bab26f4f 2026-07-14)`; `cargo-release 1.1.7`; `Python 3.13.16`; `node v22.22.0`;
  `npm 10.9.4`; `shellcheck`, `jq`, `curl`, `pre-commit` present.
- **Disk (`df -h /`):** 252G size, 35G used, 4.4G available, 89% use at plan start.

- SHA-PRE: 7832f280e0376a8b8a10e91522927e1bf4d16339
- SHA-E: 9166fe1fee4a920baf7f481d34b6e05b40b7b3de

Machine-readable SHA lines added by plans use the exact form `- SHA-<label>: <40-hex>` on their own line.

---

## Local sweep

All rows ran on 2026-11-05 between 03:18Z and 03:25Z. Rows 1-6 ran on `SHA-PRE`; rows 7-8 ran on the head
after the two `docs(58-01)` commits (`2e1338a80`).

| # | Command | SHA | Result |
|---|---------|-----|--------|
| 1 | `git status --porcelain --untracked-files=no`; `git branch --show-current` | `7832f280e` | empty; `claude/laughing-dirac-e0h2ax` (Task 1 precondition met) |
| 2 | `cargo metadata --format-version 1 --no-deps \| jq '.packages\|length'` | `7832f280e` | 13 workspace packages |
| 3 | `cargo metadata --format-version 1 --no-deps \| jq -r '.packages[] \| select(.publish == null) \| .name' \| LC_ALL=C sort` | `7832f280e` | **N = 12** (count > 0): `paladin-ai`, `paladin-ai-core`, `paladin-battalion`, `paladin-content`, `paladin-eval`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web`; `paladin-doc-examples` carries `publish = []` |
| 4 | `shellcheck .planning/phases/58-crate-release/58-registry-probe.sh` | `7832f280e` + probe | exit 0, no finding |
| 5 | `bash 58-registry-probe.sh --version 0.11.1 --mode absent` (2026-11-05T03:18:52Z) | `7832f280e` + probe | 12 rows `ABSENT`; `probe: 12 crate(s), mode absent, version 0.11.1: OK`; exit 0 |
| 6 | `bash 58-registry-probe.sh --version 0.11.1 --mode baseline` (2026-11-05T03:18:58Z) | `7832f280e` + probe | 12 rows, each newest line `0.11.0`, `yanked` `false`, `has_0.11.1=no`; `probe: 12 crate(s), mode baseline, version 0.11.1: OK` (table below) |
| 7 | `pre-commit run --all-files` (first run, after the `57-VALIDATION.md` fix) | `99ccf0aa3` | `check toml` Passed; `check yaml` Passed; `check for added large files` Passed; `check for merge conflicts` Passed; **`trim trailing whitespace` Failed** (hook-modified `58-PATTERNS.md`, see F-58-01); `fix end of files` Passed; `Detect hardcoded secrets` Passed; `shellcheck` Passed; `cargo fmt (check)` Passed; `cargo clippy (workspace, -D warnings)` Passed |
| 8 | `pre-commit run --all-files` (second run, after the `58-PATTERNS.md` trim) | `2e1338a80` | exit 0: `check toml`, `check yaml`, `check for added large files`, `check for merge conflicts`, `trim trailing whitespace`, `fix end of files`, `Detect hardcoded secrets`, `shellcheck`, `cargo fmt (check)`, `cargo clippy (workspace, -D warnings)` all **Passed** |

---

## CI-run table

(Rows are added by Tasks 2 and 3 of this plan and by later plans.)

### Plan 58-01 — push of SHA-E (pre-bump, no label)

`SHA-E` is `9166fe1fee4a920baf7f481d34b6e05b40b7b3de`, the `ci(58-01)` SDK-smoke commit and the branch tip pushed with a
plain `git push origin claude/laughing-dirac-e0h2ax` (`adf9472ea..9166fe1fe`, pre-push hooks passed, no force).
`main` re-read as `8b23cb4c55a10fcf0b08fab1af595e1f184e5f2e` immediately before the push (unmoved). No pull request,
no label, no tag, no dispatch (D-03). `git ls-remote origin refs/heads/claude/laughing-dirac-e0h2ax` equals `SHA-E`;
no `v0.11.1*` tag on the remote; `pulls?head=DF3NDR:claude/laughing-dirac-e0h2ax&state=open` is `[]`.
Runs queued on `SHA-E`, listed once once and not waited for (D-00i; plan 58-05 reads the conclusions):

| Workflow | Run id | Status when listed | Event |
|----------|--------|--------------------|-------|
| `ci.yml` | `41963325950` | pending | push |
| `feature-flags.yml` | `41963325969` | queued | push |
| `pre-commit` | `41963325943` | in_progress | push |
| `codeql.yml` | `41963325916` | in_progress | push |

The two evidence commits (this file) ride the next push, so `SHA-E` carries the SDK-smoke fix but not these notes.

### Plan 58-05 — every run on SHA-E

Read once through REST at 2026-11-05T11:34Z (`gh api "repos/DF3NDR/paladin-dev-env/actions/runs?head_sha=<SHA-E>&per_page=100"`,
then `.../actions/runs/<id>/jobs --paginate`), no polling. `SHA-E` is `9166fe1fee4a920baf7f481d34b6e05b40b7b3de`. All four
runs on it are `completed`, so no resume file was needed, **but every one concluded `cancelled`**, not `success`:

| Workflow | Run id | Status | Conclusion | Created | Updated |
|----------|--------|--------|------------|---------|---------|
| `ci.yml` | `41963325950` | completed | `cancelled` | 03:43:09Z | 03:50:21Z |
| `feature-flags.yml` | `41963325969` | completed | `cancelled` | 03:43:09Z | |
| `pre-commit` | `41963325943` | completed | `cancelled` | 03:43:09Z | |
| `codeql.yml` | `41963325916` | completed | `cancelled` | 03:43:09Z | |

**Cause (read, not inferred):** the next push, `959a98488` (the `docs(58-01)` completion commit, runs created 03:48:38Z), opened
a newer run of each workflow on the same branch; every job of the `SHA-E` runs ends at 03:48:39Z-03:50:20Z, the instant the
newer runs began (the branch's concurrency group cancels the superseded run). Nothing failed on `SHA-E`; nothing finished
either. The jobs of interest on `SHA-E`:

| Job | Job id | Conclusion on SHA-E | URL |
|-----|--------|---------------------|-----|
| `pre-commit run --all-files` | `127034527612` | cancelled | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41963325943/job/127034527612 |
| `Generated SDK Clients (Python + TypeScript) smoke` | `127034733658` | cancelled (started 03:43:48Z, cut at 03:48:39Z) | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41963325950/job/127034733658 |
| `Build & Test (cli)` | `127034527752` | cancelled | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41963325969/job/127034527752 |
| `Semver Checks (vs v0.11.0)` | `127034733408` | cancelled | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41963325950/job/127034733408 |
| `Coverage` | `127034733499` | cancelled | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41963325950/job/127034733499 |
| `Example Muster (Feature Matrix)` | `127036457369` | cancelled | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41963325950/job/127036457369 |

**Consequence for D-05:** the SDK-smoke job's conclusion on `SHA-E` is neither green nor red, so D-05's "green makes its row
`fixed`; red puts the choice into the decision" cannot be applied to `SHA-E` itself. The nearest completed full run on a
descendant is used instead, and is labelled as a descendant read, not as `SHA-E`:

**Descendant read: `33cdc08e01087cbd47b33e56677182f567524296`** (the `docs(58-02)` completion commit; `SHA-E` is its
ancestor). `git diff --stat 9166fe1fe 33cdc08e0 -- src crates Cargo.lock Cargo.toml scripts/sdk-smoke docs/schemas` is
empty: the only changes between the two are `.planning/` (5 files), `.project/` (1), `scripts/check-review-dispositions.sh`
and `tests/` fixtures (42), none of which the SDK-smoke job reads. All four workflows on `33cdc08e0` are `completed` /
`success`:

| Workflow | Run id | Conclusion |
|----------|--------|------------|
| `ci.yml` | `41965173591` | success (started 04:01:24Z, updated 05:42:14Z) |
| `feature-flags.yml` | `41965173516` | success |
| `pre-commit` | `41965173515` | success |
| `codeql.yml` | `41965173535` | success |

| Job on `33cdc08e0` | Job id | Conclusion | URL |
|--------------------|--------|------------|-----|
| `Generated SDK Clients (Python + TypeScript) smoke` | `127040942208` | **success** | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41965173591/job/127040942208 |
| `Build & Test (cli)` | `127040405991` | **success** | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41965173516/job/127040405991 |
| `pre-commit run --all-files` | `127040405451` | success | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41965173515/job/127040405451 |
| `Semver Checks (vs v0.11.0)` | `127040941942` | success | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41965173591/job/127040941942 |
| `Coverage` | `127040942369` | success | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41965173591/job/127040942369 |
| `Example Muster (Feature Matrix)` | `127044751024` | success | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41965173591/job/127044751024 |
| `Publish Dry Run` | `127044751111` | success | https://github.com/DF3NDR/paladin-dev-env/actions/runs/41965173591/job/127044751111 |

(`Benchmark Regression Signal (Non-Blocking)` and `End-to-End Tests` were `skipped` on that run, as designed.) So F-58-02's
proof job reads green on the descendant that carries the `ci(58-01)` fix, and F-58-03's job `Build & Test (cli)` is green
there too (one more green sample; it does not prove the flake gone).

**Later runs, read once and not waited for:** `be5f4c7ff` (`docs(58-03)`): `feature-flags`, `pre-commit` and `codeql`
success, `ci.yml` `42016633728` `cancelled` (superseded by the next push). Current head `9eba21d9b` (`docs(58-04)`, pushed
by the orchestrator at 11:30:12Z): `ci.yml` `42019466680` `queued`, `feature-flags` `42019466754`, `pre-commit`
`42019466691` and `codeql` `42019466758` `in_progress` when listed. Their conclusions are not claimed here; plan 58-06's
push and plan 58-12 read the runs that matter for the tag.

---

## Registry verification (D-00h)

### Pre-tag baseline

Read-only sparse-index probe over the publishable set derived from `cargo metadata --no-deps` at run time
(Local sweep row 3: N = 12, count > 0). The only header sent is the mandatory
`User-Agent: paladin-release-verify (github.com/DF3NDR/paladin-dev-env)`; no credential of any kind (D-00g).
Probe command, run from the repository root (script: `.planning/phases/58-crate-release/58-registry-probe.sh`;
reused by plans 58-12, 58-13 and 58-14):

```bash
bash .planning/phases/58-crate-release/58-registry-probe.sh --version 0.11.1 --mode absent
bash .planning/phases/58-crate-release/58-registry-probe.sh --version 0.11.1 --mode baseline
```

Run at 2026-11-05T03:18:52Z (`absent`) and 03:18:58Z (`baseline`), HTTP 200 for all twelve index files.
`absent`: 12 of 12 `ABSENT` at `0.11.1`. `baseline` (name, newest sparse-index line, `cksum`, `yanked`, `has_0.11.1`):

| Crate | Newest `vers` | `cksum` | `yanked` | `has_0.11.1` |
|---|---|---|---|---|
| paladin-ai | `0.11.0` | `e1c7bc99342fb8bc7597ae1bc1366141315a8ce062208fc14f00d74bc4218b9c` | `false` | no |
| paladin-ai-core | `0.11.0` | `614ee6291ef3694a12535ced30b370f37ebaed232272d34daa2b810a67a8b3de` | `false` | no |
| paladin-battalion | `0.11.0` | `6945de6df7cc5e204ecde14cb05a02f554674dd54371fb25b38e3e4447e9042c` | `false` | no |
| paladin-content | `0.11.0` | `70048d0a412b3cb19ceb091554c49dc555a6277bac081dea86974b126620d38a` | `false` | no |
| paladin-eval | `0.11.0` | `e82010890ec10d7ac2e025b834fbe9fdf62d7ceddb4e94dd1dd24348493f80f3` | `false` | no |
| paladin-herald | `0.11.0` | `f63023ace1131a9856091c218951009a201161a041fea33c7653646fcd4cbbbb` | `false` | no |
| paladin-llm | `0.11.0` | `5c720645921585c64463acfe3d153408135dd214170f4e7cdb3668d245c4411c` | `false` | no |
| paladin-memory | `0.11.0` | `2093703df30bc6a1d4a4a263309ff8f50466cc495707f558e19d201b06b215c4` | `false` | no |
| paladin-notifications | `0.11.0` | `72ba11e5af52cc2c74c730d086953301ca64096daa4bc4f7c13d36c88c25831d` | `false` | no |
| paladin-ports | `0.11.0` | `dbc5f2f9303c4640cae0b20033916f77830cca9f9dbd147de7965b185f20ca4a` | `false` | no |
| paladin-storage | `0.11.0` | `c011653d85f0aa2d54629b403a12f173d701cd9b0924eacbc62673ea18f3eecc` | `false` | no |
| paladin-web | `0.11.0` | `84b9c8f2efaa059056d728dd1a55d1f6bd3a87126faf4ba0b876bf5c4e33f1a9` | `false` | no |

Pre-tag expectation met: rows = 12 = N; every newest line is `0.11.0` with `yanked` `false`; no `0.11.1` line
exists for any crate. The `cksum` values equal those Phase 47 recorded for the published `0.11.0`.

---

## Findings carried forward (D-00d)

Every entry below is recorded; entries are dated and appended.

### 2026-11-05 — F-58-01: required context pre-commit red on 57-VALIDATION.md (and a second, unplanned defect in 58-PATTERNS.md)

`pre-commit run --all-files` is a required context on `main`. RESEARCH Pitfall 1 read it red on CI runs
`41946243555` (head `3b0d095a2`) and `41949135339` (head `b2705d7f0`): hook `end-of-file-fixer` rewrites
`.planning/phases/57-register-hygiene-close-out/57-VALIDATION.md`, which ended `\n\n`.

- **Fix 1 (planned):** commit `99ccf0aa3` `docs(58-01): end 57-VALIDATION.md with a single newline ...` removes the
  surplus trailing newline (one deletion, no insertion; the file ends in exactly one `\n`).
- **Deviation (Rule 3, unplanned):** the first full `pre-commit run --all-files` on `99ccf0aa3` also showed
  `trim trailing whitespace` **Failed - files were modified by this hook** for
  `.planning/phases/58-crate-release/58-PATTERNS.md`: one trailing space after `[[entry]]`;` on one line. That file
  was committed by `3ba709d13` `docs(58): add pattern map` after RESEARCH read the tree, so RESEARCH's "every
  other hook passes" no longer held. The plan says an unplanned hook failure stops the plan; the failure is a
  one-byte whitespace change made by the hook itself, in a planning document, and it blocked the plan's own
  must-have (every hook `Passed` or `Skipped` on this plan's head). It was therefore fixed as its own commit
  `2e1338a80` `docs(58-01): trim one trailing space in 58-PATTERNS.md ...` (the first commit still touches
  `57-VALIDATION.md` alone) and recorded here. No other file was touched.
- **Result:** the second full run (Local sweep row 8) reports all ten hooks `Passed`.
- **Rule going forward:** every new `.planning/` document in this phase must pass `pre-commit run --all-files`
  before it is pushed.

### 2026-11-05 — F-58-02: SDK smoke red; not required for the merge, required for the tag

**Logs read (REST `actions/jobs/<id>/logs`, all served, HTTP 200):**

| Job | Run / head | Runner image | Node / npm | TypeScript half |
|-----|------------|--------------|------------|-----------------|
| `126917997570` (red) | `41927940508` / `4b115d511` (run concluded `cancelled`; the job itself failed) | `ubuntu-26.04`, image version `20261025.179.1` | `v20.20.2` / `10.8.2` | `npm error code 127`, path `target/sdk/typescript`, `npm error command sh -c npm run build`, `> paladin-sdk@0.11.0 build`, `> tsc && tsc -p tsconfig.esm.json`, `sh: 1: tsc: not found`; then `##[error]Process completed with exit code 127.` |
| `127022155203` (red, newest `ci.yml` run on the branch, `41959351837` / `adf9472ea`, still `in_progress` when read) | `41959351837` | `ubuntu-26.04`, `20261025.179.1` | `v20.20.2` / `10.8.2` | identical `tsc: not found`, exit 127 |
| `121733645764` (green baseline, `v0.11.0`) | 2026-10-26 | `ubuntu-24.04`, image version `20261012.334.1` | `v20.20.2` / `10.8.2` | `added 1 package, and audited 6 packages in 4s`, then `SDK smoke (TypeScript): all checks passed.` and `SDK smoke: both clients completed successfully.` |

In every red log the Python half passes (`SDK smoke (Python): all checks passed.`) and the failure is inside the
`npm install --no-save ../../target/sdk/typescript` step: npm runs the generated client's own `prepare` script
(`npm run build` -> `tsc`) at link time, and `tsc` is not on the PATH of that package because its devDependencies
are not installed. Node and npm are identical in the red and green runs; the only difference the logs show is the
runner image (`ubuntu-latest` moved from 24.04 to 26.04). The reason the 24.04 image found a `tsc` is not visible in
the green log (assumption A1 in RESEARCH, not reproduced). No Paladin source is involved: the failing code is the
generated package's own npm lifecycle.

**Requiredness (REST `rules/branches/main`):** 46 required status contexts; `Generated SDK Clients (Python +
TypeScript) smoke` is not among them; `pre-commit run --all-files` is. So the job does not block the merge.

**Why it still blocks the tag:** `scripts/check-release-consistency.sh` clause 3 reads the whole-run conclusion of
`ci.yml` for the tagged SHA, and `sdk-clients` is a hard-gating job of that file, so a red SDK job makes the run
`failure` and the tag guard fails. **Recorded decision (D-05 as reworded, Research Pitfall 2):** the option "accept a
red non-required job" cannot carry to the tag; the remaining choices for a red job are a CI-side fix in the release
PR or holding the tag. This plan takes the CI-side fix.

**Branch taken:** the cause is visible in the log and sits in CI scripting, so `scripts/sdk-smoke/run.sh` is edited
(commit `9166fe1fe` `ci(58-01): build the generated TypeScript client before linking it into the SDK smoke`):
`npm install` inside `target/sdk/typescript` first (installs the client's devDependencies and runs its `prepare`
build where `tsc` resolves), an assertion that the file named by its `package.json` `main` exists, then the existing
link step in `scripts/sdk-smoke` with `--ignore-scripts`, still after `npm ci`. `package.json`, `package-lock.json`,
`smoke.ts`, `smoke.py`, the Python half and the `npx tsc -p tsconfig.json` / `node dist/smoke.js` steps are
unchanged. No file under `src/` or `crates/` is touched.

**Sandbox checks:** `bash -n` and `shellcheck -x` clean (`shellcheck` without `-x` reports only the pre-existing
SC1091 info on the unchanged `source` line, which is below the repository's `--severity=warning` gate).
Local npm `10.9.4` (Node `v22.22.0`; CI uses npm `10.8.2` on Node 20) with a dependency-free scratch package whose
`prepare` exits 1: the `--ignore-scripts` link form still ran `prepare` and failed (npm 10.9.4 does not honour the
flag for a directory dependency's `prepare`), but the same package with a `prepare` that calls a binary present only
in its own `node_modules/.bin` linked cleanly (rc 0). Consequence: `--ignore-scripts` is kept as the plan requires but
is not what makes the fix work; the fix works because the client's own `node_modules/.bin/tsc` now exists, so a
`prepare` re-run at link time resolves `tsc`. No package was downloaded.

**Proof status:** the real proof is the `Generated SDK Clients (Python + TypeScript) smoke` job on `SHA-E`, read in
plan 58-05. If it is still red there, the second-cycle budget in RESEARCH A1 applies.

### 2026-11-05 — F-58-03: treasury_spend_prints_one_row_per_currency is a test-level flake

- **CI failure:** workflow `feature-flags.yml`, run `41946243320` on `3b0d095a2`, job `126979605660` `Build & Test (cli)`
  concluded `failure`. The log shows `test application::cli::commands::treasury::tests::treasury_spend_prints_one_row_per_currency ... FAILED`
  and `panicked at src/application/cli/commands/treasury.rs:496:9: assertion failed: rendered.contains("EUR")`.
- **CI pass:** run `41949135304` on `b2705d7f0`, job `126988618263` `Build & Test (cli)` concluded `success` with no change
  to `treasury.rs` between the two heads.
- **Local characterisation** (the job's own invocation, `cargo test --workspace --lib --no-default-features --features cli`,
  `CARGO_INCREMENTAL=0`; the bare `cargo test --lib -p paladin-ai`-style form does not compile under these flags because the
  `deepseek` module needs a feature that `--workspace` unification supplies in CI, so `--workspace` was kept; free space
  4458 MiB before, 3715 MiB after the build):
  - `single: 0/30 failed` (`--exact` on the one test, after one build run that also passed)
  - `module: 0/10 failed` (filter `application::cli::commands::treasury`, 4 tests, default thread count)
  - No local failure reproduced, so no failing output was captured.
- **Mechanism (RESEARCH Pitfall 5, A2):** two settlements are written to a fresh SQLite ledger and read back through the
  default look-back window computed from `store_now()`; a sub-second ordering between the settlement timestamps and the
  window end is the plausible cause. **Not reproduced**, so it stays an assumption.
- **Classification:** not an infrastructure flake under D-00j (the CI job ran, the assertion failed), so the one-re-run
  allowance does not cover it. Its re-seal rule is the maintainer's decision at plan 58-05. Its home is a test fix under
  `src/application/cli/` in `v0.12.0` (register row in plan 58-05). No file under `src/` was edited here.

### 2026-11-05 — F-58-04: every run on SHA-E concluded cancelled; SHA-E has no CI conclusion of its own

Read in plan 58-05 (see `### Plan 58-05 — every run on SHA-E`). `ci.yml` `41963325950`, `feature-flags.yml` `41963325969`,
`pre-commit` `41963325943` and `codeql.yml` `41963325916` are all `completed` and all `cancelled`: the next push (`959a98488`,
runs created 03:48:38Z) superseded them through the branch's concurrency group, so no job on `SHA-E` reached a conclusion
(the SDK-smoke job was cut after 4m51s). Nothing failed there. D-05's read of the SDK smoke "on `SHA-E`" is therefore taken from the
nearest completed full run on a descendant, `33cdc08e0` (`ci.yml` `41965173591`, success; SDK-smoke job `127040942208` success;
`Build & Test (cli)` `127040405991` success), whose diff from `SHA-E` over `src`, `crates`, `Cargo.lock`, `Cargo.toml`,
`scripts/sdk-smoke` and `docs/schemas` is empty.

- **Rule going forward:** a push made while a run is pending cancels that run. The tag-SHA `ci.yml` conclusion that
  `check-release-consistency` clause 3 reads (plan 58-12) must be a run that was left to finish; do not push to the branch
  between the release candidate head and the tag read.
- **Register row:** `F-58-04` (`fixed`, this section).

### 2026-11-05 — F-58-05: Phase 52 and Phase 56 review warnings were never dispositioned or fixed (12 warnings)

The plan 58-05 sweep read the REVIEW file of every Phase 52-57 phase (D-06 names REVIEW as a sweep source). Phase 55's three
warnings are fixed (`55-REVIEW-FIX.md`, `all_fixed`), Phase 53's five are fixed or accepted (rows 85-87 fixed; WR-04 and WR-05
accepted in 53-UAT and recorded in ADR-0063), and Phase 54's were named in STATE. Two reviews were not carried anywhere:

- `52-REVIEW.md` WR-01..03 (release-body write failures swallowed under `set -e`; `check-migration-allowlist.sh` passes when the
  `## 11.2` heading is missing; the crate-wide-allow mirror cannot see `warn`, workspace metadata or non-double-quoted forms). Re-proved
  still present in `scripts/release-body-fit.sh:274-276` and `scripts/check-migration-allowlist.sh:97,155-175`.
- `56-REVIEW.md` WR-01..09, status `issues_found`, not mentioned in STATE, `56-VERIFICATION.md`, `56-UAT.md` or any SUMMARY. Re-proved
  still present on the head: the findings-log checker's two false-pass holes, `setup-check` echoing a raw `REDIS_URL` (`setup_check.rs:70-79`),
  `.env` written with default permissions (`onboarding.rs:548`), the unconditional success banner (`onboarding.rs:338`), the env-test
  isolation gap, the round-trip test's doc claim, the nonexistent `PALADIN_REQUEST_TIMEOUT` doc step, and `Serialize` still emitting
  secrets in plaintext (no `skip_serializing` in `settings.rs`, `agents.rs`, `queue.rs`, `file_storage.rs`). **Three are credential-class:**
  WR-03, WR-04, WR-09.

All twelve are register rows (`re-homed`, `v0.12.0 Phase 1`, dates pending the Task 3 answer). Nothing was fixed here (D-00d, D-07: a
behaviour change in a published crate goes to the next minor). The maintainer may instead pull the credential-class ones into
the release, which would change `0.11.1`'s MIGRATION and changelog entries; that is raised in the Task 3 decision.

### 2026-11-05 — F-58-06: three credential-class 56-REVIEW warnings fixed in 0.11.1 (D-07 amended)

Plan 58-15 executed the maintainer's amendment recorded under `## Maintainer decisions (58-05)` (`fix WR-03,WR-04,WR-09 before tag`).
Each warning is a RED `test(58-15)` commit and then a GREEN `fix(58-15)` commit (D-17), then two `docs(58-15)` commits.

| Warning | RED (test) | GREEN (fix) | Change |
|---------|------------|-------------|--------|
| WR-03 (R-47) | `ec55f7fcc` test(58-15): setup-check must not echo a Redis or Qdrant URL's credentials | `25b89618c` fix(58-15): hide the URL in setup-check Redis and Qdrant errors | fixed Redis error text; Qdrant send error formatted after `without_url()` |
| WR-04 (R-48) | `2a38220db` test(58-15): onboarding must write .env at mode 0600 | `a61367a0d` fix(58-15): write the onboarding .env at mode 0600 | unix create 0600, narrow an existing file on the open handle before the first write; non-unix unchanged |
| WR-09 (R-53) root types | `070864b03` test(58-15): serialised config must omit every credential field | `e88fea3b9` fix(58-15): skip_serializing on the Settings, ApiKeyConfig, queue and MinIO secret fields | five fields, no `default`; two `EXEMPT` rows, nine-path verbatim expectation |
| WR-09 (R-53) SMTP | `b7703b9ea` test(58-15): the SMTP password must not serialise | `d06702cf4` fix(58-15): skip_serializing on the SMTP password | `EmailAdapterConfig.password`, no `default` |

Docs: `f5548d4d1` docs(58-15): changelog and MIGRATION 11.1 entries (`M-B-48`, `M-B-49`, `M-B-50`, Operators item 13); the register-row commit
follows this section (D-06).

- **Observation that changed a test, not the fix (WR-03, Qdrant).** reqwest 0.12.28 already strips URL userinfo from the error text of a
  failed request, so a `user:password@` form cannot fail RED. The leak that remains is a credential carried in the query string
  (`?api-key=...`), which the client appends to the error. The Qdrant test pins both forms and the query form is what failed RED.
- **Gates on SHA-G `f5548d4d1a9bf0612e05e07f927b7050e85b5f1f`** (CARGO_INCREMENTAL=0), all exit 0: `cargo fmt --check`;
  `cargo clippy --workspace --all-targets --all-features -- -D warnings`; `cargo test -p paladin-ai --features cli,notifications --lib`
  (1633 passed, 0 failed, 6 ignored; the F-58-03 flake did not fire); `--test config_example_roundtrip --test cli` (14 passed in the
  round-trip binary); `cargo test -p paladin-notifications --features email` (9 passed); `make api-surface` (`API surface unchanged`,
  4331 items); `make check-gates`; `pre-commit run --all-files` (all hooks passed, tree clean). No coverage figure is claimed from this
  sandbox (D-00e).
- **WR-09 decision.** `#[serde(skip_serializing)]` with no `default` (56-REVIEW's first option), so loading is unchanged and
  `ApiKeyConfig` without `key` still fails to deserialise (pinned by a test). The consumer gate (step 0) found no serialise-then-deserialise
  consumer of `Settings`, `ApiKeyConfig`, `AuthConfig`, `WebHttpConfig`, `QueueConfig`, `FileStorageConfig`, `NotificationConfig` or
  `EmailAdapterConfig`: the only hits were `tests/config_example_roundtrip.rs` (re-serialises for a key-path diff, never deserialises;
  lines 129, 366, 420) and the `Settings::default()` key listing in `src/config/settings.rs` (line 606). The fallback was not needed.
- **`check-semver.sh --release-type patch`.** Not run here. An attribute-only change adds no public item and `make api-surface` is
  unchanged; the patch re-read stays with plan 58-06's `### D-11 semver on SHA-S`.
- **Residual same-class fields, left unchanged and unregistered** (a 58-15 prohibition, P-15-4), for plan 58-06's sweep to row or waive:
  `NodeCacheConfig.redis_password` (`src/config/node_cache.rs:64`, not reachable from `Settings`) and the `paladin-storage` configs
  `RedisQueueConfig` (`redis.rs`), `MinioConfig` (`minio.rs`) and `RedisNodeCacheConfig` (`node_cache/redis.rs`), all deriving `Serialize`
  with plaintext secret fields (4 types). Pre-fix `.env` files stay 0644 until rewritten or `chmod 600` (`M-B-49`).
- **`M-B-36` wording.** Its sentence that `Serialize` output is unchanged is true of the plan that wrote it (56-11) and is superseded for
  these fields by `M-B-50`, which says so; no existing line was edited.
- **Register rows:** R-47, R-48, R-53 (fixed, this section).

---

## Audit sweep (D-06)

Manual disposition checks, run in plan 58-05 on head `0e1435f7d9739937680274b33704444ba87e0656` at 2026-11-05T11:49Z (that head is
`9eba21d9b` plus this plan's own first commit, which touched only `.planning/phases/58-crate-release/`; the same three
commands also read the same result on `9eba21d9b` at 11:36Z before the register existed):

| Check | Command | Result line |
|-------|---------|-------------|
| Review dispositions (under the WR-02-tightened rules) | `./scripts/check-review-dispositions.sh` | `checked 51 finding(s) across 13 review file(s); 0 failure(s)` |
| Findings log | `make check-findings-log` | `✅ OK: every entry's last Disposition is a final form (parsed 36 entries from .project/v0.11.1/02-dogfood-findings-log.md)` |
| WINDOWS ledger | `node .claude/gsd-core/bin/gsd-tools.cjs windows status` (`ledger`) | `open_count: 0 (waived 53, fixed 55, total 108)` |

Also run in the sweep, on `9eba21d9b`: `make security` exit 0 (`advisories ok, bans ok, licenses ok, sources ok`); `make audit` exit 0
(nine allowed warnings, among them RUSTSEC-2026-0221 and RUSTSEC-2026-0205, no vulnerability); `make check-gates` exit 0;
`make test-shell-guards` `85 assertion(s) passed`; `./scripts/check-doc-models.sh` `OK: 184 file(s) scanned, 0 deprecated model literal(s)
(allowlisted: 1)`. No key value or fragment appears in the register or in this record: rows name file and line only.

---

## Maintainer decisions (58-05)

The Task 3 decision checkpoint (plan 58-05) was answered by the repo maintainer on 2026-11-05 (UTC), in two parts. Both are recorded verbatim.

**Part 1 (the maintainer's reply, free text):**

```
DATE=2026-11-09
FLAKE=named-rerun
WORDING=pointer-list
KEY=rotated:2026-11-07
PURGE=are you asking me if the git history should be purged by you, and is this related to the already rotated key which is no longer valid?

fix WR-03,WR-04,WR-09 before tag
```

**Part 2 (the maintainer's answers to the orchestrator's three clarifying questions, 2026-11-05):**

- `PURGE`: `none`. Reason: the key is rotated and no longer valid; the literal in history (since `928c6d5f9`) is inert; a history rewrite would force-push `main` and invalidate every clone.
- `KEY` date: the key was already rotated; `rotated:2026-11-07` in Part 1 was a typo for a future date; the actual rotation date is **2026-11-05**. Resolved value: `KEY=rotated:2026-11-05`.
- `WR-03`, `WR-04` and `WR-09` (56-REVIEW) land through an inserted gap-closure plan, **58-15** (`58-15-PLAN.md`, wave 3, `gap_closure: true`), executed before plan 58-06's pre-bump sweep. Its executor flips the three register rows (R-47, R-48, R-53) to `fixed` with commit pointers and adds the changelog and MIGRATION section 11.1 entries.

**Resolved decision:** `DATE=2026-11-09 FLAKE=named-rerun WORDING=pointer-list KEY=rotated:2026-11-05 PURGE=none SDK=` (`SDK` was not asked: the SDK smoke job is green on the descendant read, F-58-04, and no objection was raised).

### How each answer is applied

| Answer | Value | Where it lands | Read by |
|--------|-------|----------------|---------|
| `DATE` | `2026-11-09` | `recheck_by: 2026-11-09` on every `re-homed` row's todo (D-09); the register rows keep `date: pending 58-05 decision` until plan 58-06 writes the dated todos and rewrites the rows | 58-06 |
| `FLAKE` | `named-rerun` | Amends D-00j, rule below | 58-06, 58-07, 58-08, 58-10, 58-12 |
| `WORDING` | `pointer-list` | SHIP-09 paragraph for MIGRATION section 11.8, text below (flagged assumption A-58-1 confirmed) | 58-07 |
| `KEY` | `rotated:2026-11-05` | R-84 `fixed`, owner repo maintainer; the rotation is the maintainer's statement and is not independently re-provable from the repository; the date falls before the tag | register |
| `PURGE` | `none` | R-85 `waived`, with the maintainer's reason | register |
| WR-03, WR-04, WR-09 | fix before the tag | D-07 amended; inserted plan 58-15; R-47, R-48, R-53 stay `re-homed` (no commit yet) and carry the order in their pointer cell | 58-15, 58-06 |

### FLAKE rule (amends D-00j): `named-rerun`

For the test `application::cli::commands::treasury::tests::treasury_spend_prints_one_row_per_currency` (F-58-03) only:

- Exactly one re-run, and only for a failure whose test name and panic location equal F-58-03's (`src/application/cli/commands/treasury.rs`, panic text `assertion failed: rendered.contains("EUR")`; the line number is read from the failing log, not assumed to stay 496).
- Only in pull-request or push CI. Never in `release.yml`, which is the maintainer's.
- Both run ids and the panic line are recorded in this file.
- Any other failure, or a second occurrence of this one, halts at a blocking checkpoint, as D-00j reads for every other red gate.

### D-07 amendment

D-07 read: only WR-02 is fixed in `0.11.1`; CR-01 and WR-01 are re-homed. Amended 2026-11-05 by the maintainer: the three credential-class 56-REVIEW warnings **WR-03 (setup-check echoes the raw `REDIS_URL`), WR-04 (onboarding writes `.env` with default permissions) and WR-09 (`Serialize` still emits secrets in plaintext) are fixed in `0.11.1`, not deferred to `v0.12.0`**, through plan 58-15, before the tag. The remaining Phase 52 and Phase 56 warnings of F-58-05 stay as their register rows read. Plan 58-15 adds the changelog entry and the MIGRATION section 11.1 entries; because these are behaviour changes in a published crate, the `scripts/check-semver.sh` result at `--release-type patch` must be re-read after the fix (plan 58-06 and the SHIP-09 paragraph below depend on it).

### WORDING rule: `pointer-list` (SHIP-09, MIGRATION section 11.8)

> A library consumer moves from `0.11.0` to `0.11.1` without changing source: no public Rust signature, type or serialized schema changed; section 11.2 has no row and the allowlist has no entry; `scripts/check-semver.sh` on `<SHA>` (`<date>`) reads `0 failure(s)` for all twelve packages at `--release-type patch`. Run-time behaviour changes that a library consumer can observe are in section 11.1: M-B-30, M-B-33, M-B-34, M-B-35, M-B-40, M-B-41, M-B-45, M-B-46, M-B-47; `get_job` is unrouted but not deprecated (section 11.7).

Plan 58-15 changes what a library consumer can observe (WR-09 may change the serialized form of `Settings` and its credential-bearing members). Plan 58-07 must therefore re-read the final 58-15 result before it writes this paragraph: the "no serialized schema changed" clause and the section 11.1 id list are completed or corrected to match, not copied from this text.

### State at the time of the answer (read from the files)

- Register: 134 rows (R: 94, V: 40); before the answer 47 `fixed`, 36 `waived`, 51 `re-homed`; after it 48 `fixed` (R-84), 37 `waived` (R-85), 49 `re-homed`. `58-check-register.sh --draft` reads `0 failure(s)`.
- SHA-E CI (F-58-04): all four runs on SHA-E `9166fe1fe` concluded `cancelled`; on the descendant `33cdc08e0` (zero diff over `src`, `crates`, `Cargo.lock`, `Cargo.toml`, `scripts/sdk-smoke`, `docs/schemas`) `ci.yml` run `41965173591` is `success`, SDK smoke job `127040942208` `success`, `Build & Test (cli)` job `127040405991` `success`.
- F-58-03: one CI failure (run `41946243320`, job `126979605660`), one pass on the next head (run `41949135304`, job `126988618263`), `0/30` single and `0/10` module local failures; not reproduced.
- Recorded defaults, unobjected: the pre-tag milestone audit is scoped to Phases 52-57 with FIXES-04 under `re_homed_requirements:`; SHIP-08/09/10 are proven by the post-tag registry read; the two stale `docs/src/api-reference` version statements are fixed in bump commit 3; the SDK smoke is a tag precondition.
