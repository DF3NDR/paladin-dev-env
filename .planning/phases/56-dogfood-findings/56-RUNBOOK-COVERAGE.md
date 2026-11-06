# Phase 56 Runbook: local coverage walk (DOGF-05, D-17)

Stage B runbook. It needs a Docker-capable machine and about an hour of wall clock. No agent
environment has a Docker daemon, so this walk is yours. Paste slots `C1`..`C6` at the end are read
by plan 56-09.

## Purpose and decision

- **Requirement:** DOGF-05. A maintainer follows `docs/src/contributing/testing-guide.md`
  section "Test Coverage" on a Docker-capable machine and records the measured line coverage.
- **Decision:** D-17. The walk follows the guide's commands in the guide's order, without
  improving them. If a command is needed that the guide does not give, or the guide is wrong, that
  is a finding (S3, docs fix to `testing-guide.md`), not something to quietly work around.
- **Todo this closes:** `.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`
  (front matter `resolves_phase: 56`).
- **Single source of the measured command:** `scripts/coverage.sh`. `make coverage` and the CI
  `coverage` job both call it. It runs
  `cargo llvm-cov --workspace --features integration-tests,llm-all --lcov --output-path lcov.info --fail-under-lines "$FLOOR" -- --test-threads=1`
  with `FLOOR` defaulting to `82`. Do not substitute another invocation.

## Pass rule

D-17, exactly:

1. Compute the local line-coverage figure from `lcov.info` with the snippet in this runbook (the
   same LF/LH summation CI runs). Truncate toward zero to a whole percent.
2. Take the CI `coverage` job's figure for the **same commit**. Truncate it the same way.
3. **Pass** when the two whole percents are equal **and** both are >= 82.
4. The comparison is against the CI run on the commit you measured, not against the historic
   82.39 % at `e9e3267f9`.

Anything else (different whole percents, or either below 82) is a fail: record both figures and
the suspected cause (scope, service not up, tool or toolchain drift) and do not close the todo.

## Before you start

You need:

- A Docker-capable machine (`docker` and `docker compose` working).
- A clean checkout at a commit that is already pushed to `origin` and whose `ci.yml` run has a
  `coverage` job. Check:

  ```bash
  gh run list --workflow=ci.yml --commit "$(git rev-parse HEAD)" --limit 3
  ```

  If it prints nothing, push the commit (or pick a pushed one) and wait for CI.
- Slot `C1` filled with the environment record from Step 0 below. `git status --porcelain` must be
  empty; a dirty tree measures a different commit from the one CI measured.

## Steps

Run every step from the repository root. After each step, paste what it says into the named slot.
Copy output verbatim; do not tidy it.

0. Record the environment (slot `C1`):

   ```bash
   git rev-parse HEAD
   git status --porcelain
   rustc -V
   cargo -V
   docker version --format '{{.Server.Version}}'
   docker compose version
   ```

1. Install the LLVM tools component (guide, "Prerequisites"):

   ```bash
   rustup component add llvm-tools-preview
   ```

2. Install `cargo-llvm-cov` exactly as the guide says (unpinned), then record the version in `C1`:

   ```bash
   cargo install cargo-llvm-cov --locked
   cargo llvm-cov --version
   ```

3. Start the services (guide, "Local generation" step 1). Paste the command output and the
   container list into slot `C2`:

   ```bash
   make services-up
   docker ps --format '{{.Names}} {{.Ports}}'
   ```

4. Measure coverage (guide step 2). Paste the **last 40 lines** and the `exit=` line into slot
   `C3`:

   ```bash
   make coverage 2>&1 | tee coverage-run.log; echo "exit=${PIPESTATUS[0]}"
   tail -40 coverage-run.log
   ```

   If this fails with `ERROR: Redis unreachable` or `ERROR: object store (RustFS) unreachable`,
   see trap 2 below before doing anything else.

5. Compute the figure (slot `C5`). Run the block below in the checkout root, where `lcov.info`
   was written, and paste its single output line. It is the same LF/LH summation CI runs.

<!-- lcov-snippet:start -->
```bash
python3 - <<'PY'
lf = lh = 0
for line in open("lcov.info"):
    if   line.startswith("LF:"): lf += int(line[3:])
    elif line.startswith("LH:"): lh += int(line[3:])
print(f"Lines: {lh}/{lf} = {100*lh/lf:.2f}%  (truncated: {int(100*lh/lf)})")
PY
```
<!-- lcov-snippet:end -->

   Sanity check for the snippet itself: on a two-record `lcov.info` with `LF:100 LH:90` and
   `LF:100 LH:75` it prints `Lines: 165/200 = 82.50%  (truncated: 82)`.

6. Optional (guide step 3): `make coverage-html`. See trap 4 before reading anything into its
   figure.

7. Stop the services:

   ```bash
   make services-down
   ```

8. Throughout, note in slot `C6` every step that did not read as written (a missing
   prerequisite, an error, a command the guide should have named).

## Traps to record

For each trap: **record what happened before any workaround.** A trap that bites is an S3 DF and a
docs fix (D-17). If it does not bite, write "did not bite" next to its number in `C6`.

1. **No percentage is printed by `make coverage`.** The guide's "Reading the output" says an LCOV
   summary is printed; `--lcov --output-path` writes a file and prints nothing. Record whether
   you saw any percentage in the `make coverage` output.
2. **Port mismatch.** `make services-up` runs `docker/docker-compose.yml`: Redis on host `6379`
   (started with `--requirepass`, research assumption A3) and RustFS on `9000`/`9001`.
   `scripts/coverage.sh` probes, in order, Redis at the preset `TEST_REDIS_HOST`, then
   `redis:6379`, then `localhost:6380`, and the object store at the preset
   `TEST_MINIO_ENDPOINT`, then `rustfs:9000`, then `localhost:9010`. On a bare host that ran only
   `make services-up`, none of those may answer. Also unverified: whether the
   password-protected compose Redis passes the script's unauthenticated probe.
   - If the first run ends `ERROR: Redis unreachable` or `ERROR: object store (RustFS)
     unreachable`, paste it verbatim into `C3`.
   - Then try the workaround
     `docker compose -f docker/docker-compose.test.yml up -d redis-test rustfs-test`
     (host ports 6380 and 9010), or a `.env` copied from `.env.example`. Paste that second run
     into `C4` and say which workaround you used.
   - If there was no failure, write `none` in `C4`.
3. **Tool pin.** CI uses `cargo-llvm-cov@0.8.7` on toolchain `stable`; the guide installs
   unpinned. Record your local `cargo llvm-cov --version` and `rustc -V`. A sub-percent delta
   after a tool bump is a candidate cause, not a code defect.
4. **`make coverage-html` measures a different scope.** The Makefile runs
   `cargo llvm-cov --workspace --features integration-tests --html --output-dir target/coverage`,
   without `llm-all`. Its figure is not comparable to the gated one.
5. **`make services-up` starts more than Redis and RustFS.** It runs `up -d` over every service
   in `docker/docker-compose.yml` that is not behind a compose profile. Reading the file,
   `paladin-app` has no profile and has a `build:` section, so it is built and started;
   `redis-commander`, `redis-insight` and `qdrant` carry `dev`/`debug` profiles. Confirm against
   `docker ps` in `C2`, and record the time taken and anything that was built.
6. **CI's summary label is stale.** CI's summary step prints
   `Scope: --workspace --features integration-tests (the gated measurement)`, but the real scope
   is `integration-tests,llm-all`. You only confirm it when you look at the CI summary; the
   agent logs it (S4).

## Redaction

Never paste `.env` contents, tokens or credentials. Remove local usernames or hostnames from
pasted output if you want to. The compose defaults (`changeme`, `paladin-dev`) are public repo
defaults and need no redaction, but a real `REDIS_PASSWORD` you exported does.

## Cleanup

```bash
make services-down
rm -f coverage-run.log
```

`lcov.info` and `target/` are build outputs and are ignored by git; leave them or delete them.

## Paste slots

Fill these in place. Each has an HTML comment marking where to paste, then an empty fenced
block. Leave a slot empty only if the step was not run, and say so in `C6`.

### C1 — environment

<!-- paste below this line; redacted -->
````
First attempt (before the manual install):

```
git rev-parse HEAD; git status --porcelain; rustc -V; cargo -V; cargo llvm-cov --version
21dacf0d63b55e7a5f14d76eba207c107e4889aa
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo 1.97.1 (c980f4866 2026-06-30)
error: no such command: `llvm-cov`

help: view all installed commands with `cargo --list`
help: find a package to install `llvm-cov` with `cargo search cargo-llvm-cov`
```

After `cargo install cargo-llvm-cov --version 0.8.7 --locked`:

```
git rev-parse HEAD; git status --porcelain; rustc -V; cargo -V; cargo llvm-cov --version
21dacf0d63b55e7a5f14d76eba207c107e4889aa
rustc 1.97.1 (8bab26f4f 2026-07-14)
cargo 1.97.1 (c980f4866 2026-06-30)
cargo-llvm-cov 0.8.7
```

`git status --porcelain` printed nothing (clean checkout). `docker version` / `docker compose
version`: not applicable (no Docker socket inside the devcontainer).

**CI figure for the comparison.** The measured commit `21dacf0d6` had its CI run (`41695288977`)
cancelled when the next push superseded it. The next commit on the branch, `d9a48cfc6`, differs
from `21dacf0d6` only in `.md` files (`git diff --stat 21dacf0d6 d9a48cfc6 -- . ':!*.md'` is
empty), so its `coverage` job measures the same code. From CI run `41703614636`, job
`Coverage` (id `126153522434`, conclusion `success`), summary step, read from the job log by
the orchestrator (the artifact blob host is proxy-blocked from the sandbox):

```
Scope: --workspace --features integration-tests (the gated measurement)
Lines:     170767/183901 = 92.86%
Functions: 17617/20367 = 86.50%
```
````

### C2 — services

<!-- paste below this line; redacted -->
````
`make services-up` was not run: the devcontainer has no Docker socket. The compose siblings are
already up and `scripts/coverage.sh` probes `redis:6379` / `rustfs:9000` by hostname first.

```
$ getent hosts redis rustfs
redis-cli -h redis -p 6379 ping 2>&1 || echo "redis-cli not installed (script falls back to a TCP probe)"
172.19.0.3      redis
172.19.0.4      rustfs
PONG
```

Redis answered `PONG` unauthenticated on 6379 (assumption A3: the compose Redis passes the
script's probe — settled for the devcontainer).
````

### C3 — first coverage run

<!-- paste below this line; redacted -->
````
`make coverage 2>&1 | tee coverage-run.log; echo "exit=${PIPESTATUS[0]}"`, then
`tail -40 coverage-run.log`. All suites passed. Last lines (the `exit=` line printed between the
tee'd output and the tail):

```
    Finished report saved to lcov.info
exit=0
test user_controller::tests::test_login_user_auth_failed ... ok
test user_controller::tests::test_login_user_success ... ok
test user_controller::tests::test_register_user_service_error ... ok
test user_controller::tests::test_register_user_success ... ok
test user_controller::tests::test_user_error_to_response_authentication_failed ... ok
test user_controller::tests::test_user_error_to_response_invalid_email ... ok
test user_controller::tests::test_user_error_to_response_user_not_found ... ok
test user_controller::tests::test_user_to_response ... ok
test user_controller::tests::test_verify_user_success ... ok

test result: ok. 329 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.90s

     Running tests/auth_rbac.rs (target/llvm-cov-target/debug/deps/auth_rbac-dcf7cee72ea5d1ac)

running 5 tests
test admin_route_with_admin_token_succeeds ... ok
test admin_route_with_invalid_token_is_unauthorized ... ok
test admin_route_with_user_token_is_forbidden ... ok
test protected_route_with_valid_admin_token_succeeds ... ok
test protected_route_without_token_is_unauthorized ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.65s

     Running tests/openapi_golden_v0_9.rs (target/llvm-cov-target/debug/deps/openapi_golden_v0_9-d0b2f7e700848e1f)

running 9 tests
test allowance_429_exception_is_narrowly_scoped ... ok
test execute_response_exception_is_narrowly_scoped ... ok
test info_version_is_the_only_normalisation ... ok
test openapi_v0_9_paths_match_the_frozen_baseline ... ok
test phase_42_exception_is_narrowly_scoped ... ok
test ref_closure_is_non_empty_and_fully_resolved ... ok
test ref_closure_schemas_match_the_frozen_baseline ... ok
test security_schemes_match_the_frozen_baseline ... ok
test v0_9_path_restriction_is_non_empty ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s


    Finished report saved to lcov.info
```

No percentage was printed (trap 1 confirmed).
````

### C4 — workaround run (or "none")

<!-- paste below this line; redacted -->
```
none
```

### C5 — snippet output line

<!-- paste below this line; redacted -->
```
Lines: 170650/183901 = 92.79%  (truncated: 92)
```

### C6 — stumbles, one bullet per step that did not read as written

<!-- paste below this line; redacted -->
```
- `cargo-llvm-cov` was missing after a devcontainer rebuild. `.devcontainer/Dockerfile.dev:112`
  installs the `llvm-tools-preview` rustup component but nothing installs `cargo-llvm-cov`
  itself (`post-create.sh` neither). Phase 36.1 had found it at `/usr/local/cargo/bin/`, which
  is not a persisted volume (only `/usr/local/cargo/registry` is), so a hand install was lost
  on rebuild. Installed by hand at CI's pin `0.8.7`.
- `make services-up` (guide step 1) is not runnable inside the devcontainer (no Docker socket).
  The compose siblings `redis`/`rustfs` were used instead; the script's first probe
  (`redis:6379`) found them. The guide step was therefore not exercised on a bare host.
- `make coverage` prints no percentage (trap 1); the LF/LH snippet was needed.
- The measured commit `21dacf0d6` had a cancelled CI run; compared against `d9a48cfc6`
  (md-only delta, see C1).
- CI's summary step prints `Scope: --workspace --features integration-tests` while the real
  scope is `integration-tests,llm-all` (trap 6, confirmed in the job log).
- Trap 2 (port mismatch 6379/9000 vs 6380/9010): did not bite — not exercised in the
  devcontainer. Trap 4 (`coverage-html` scope): not run. Trap 5 (`services-up` builds
  `paladin-app`): not exercised.
- Trap 3 (unpinned install in the guide): bit in spirit — the guide's `cargo install
  cargo-llvm-cov --locked` has no `--version`; the maintainer pinned `0.8.7` by hand to match CI.
- `make services-down`: not applicable. `rm -f coverage-run.log`: done.

**Pass rule (D-17):** local `int(92.79) = 92`, CI `int(92.86) = 92`; equal and both >= 82.
**PASS.** Delta 117 hit lines on an identical 183901-line denominator.
```

## Triage checklist

For plan 56-09. Start only from the filled slots.

- [x] Fetch the CI figure for the commit in `C1`:

  ```bash
  gh run list --workflow=ci.yml --commit <C1 sha> --json databaseId,conclusion,headSha,name --limit 5
  gh run download <id> -n coverage-summary -D <scratch>
  ```

  Read the `Lines:` line. If the `gh` call fails, ask the maintainer to paste the figure from the
  CI job summary.
- [x] Apply the pass rule: equal whole percents, both >= 82.
- [x] Log one S3 DF per `C6` stumble and per trap that bit, each with a contained docs fix to
  `docs/src/contributing/testing-guide.md` (D-05). Append entries that parse under
  `make check-findings-log` (next free id is the one after the last `### DF-nn` in
  `.project/v0.11.1/02-dogfood-findings-log.md`).
- [x] On pass, `git mv` the todo to `.planning/todos/completed/` and add a
  `## Resolution (YYYY-MM-DD)` section with: local figure, CI figure, CI run id, commit, and the
  `cargo-llvm-cov` versions (local and CI).
- [ ] On fail, log a DF with both figures. The todo
  `2026-08-13-verify-local-coverage-reproduction.md` stays pending with a dated update.

CI half unavailable 2026-11-03: `gh run list --workflow=ci.yml --branch main` worked (newest success: run 41246424413, commit 8b23cb4c55a10fcf0b08fab1af595e1f184e5f2e), but `gh run download 41246424413 -n coverage-summary` failed with `error downloading coverage-summary: Get "https://productionresultssa14.blob.core.windows.net/actions-results/...": Forbidden` (the agent egress proxy refused `productionresultssa14.blob.core.windows.net:443`; `gh auth status` also reports the `GH_TOKEN` invalid). Fallback: the maintainer pastes the CI figure (the `Lines:` line from the `coverage-summary` artifact or the `coverage` job summary for C1's commit) into C1 or chat, and Task 3 records its source as maintainer-pasted.

Comparison 2026-11-03: **PASS** (D-17, PD-09-1). Local (C5, maintainer-measured at commit `21dacf0d63b55e7a5f14d76eba207c107e4889aa`): `Lines: 170650/183901 = 92.79%`, truncated 92. CI (run `41703614636`, job `Coverage` id `126153522434`, conclusion `success`, commit `d9a48cfc6`, figure read from the job log by the orchestrator because the artifact blob host is proxy-blocked from the sandbox, so maintainer/orchestrator-relayed, not fetched by `gh run download`): `Lines:     170767/183901 = 92.86%`, truncated 92. The two whole percents are equal and both are >= 82. Same-commit basis: `21dacf0d6`'s own CI run (`41695288977`) was cancelled by the next push, so the comparison uses `d9a48cfc6`, whose delta from `21dacf0d6` is Markdown only (`git diff --stat 21dacf0d6 d9a48cfc6 -- . ':!*.md'` is empty, re-run by the agent), so it measured the same code. Both runs share the denominator 183901; the numerators differ by 117 hit lines (0.07 points), which the walk did not attribute (candidates: nondeterministic hit lines under `--test-threads=1` across environments, or the CI Redis/RustFS stack differing from the compose siblings). Tool: local `cargo-llvm-cov 0.8.7`, rustc 1.97.1; CI pins `cargo-llvm-cov@0.8.7`. Trap 2 (port mismatch, password-protected Redis on a bare host) remains an unverified hypothesis: it did not bite in the devcontainer and was not exercised.
