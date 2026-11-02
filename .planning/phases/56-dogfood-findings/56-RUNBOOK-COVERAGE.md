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
```
```

### C2 — services

<!-- paste below this line; redacted -->
```
```

### C3 — first coverage run

<!-- paste below this line; redacted -->
```
```

### C4 — workaround run (or "none")

<!-- paste below this line; redacted -->
```
```

### C5 — snippet output line

<!-- paste below this line; redacted -->
```
```

### C6 — stumbles, one bullet per step that did not read as written

<!-- paste below this line; redacted -->
```
```

## Triage checklist

For plan 56-09. Start only from the filled slots.

- [ ] Fetch the CI figure for the commit in `C1`:

  ```bash
  gh run list --workflow=ci.yml --commit <C1 sha> --json databaseId,conclusion,headSha,name --limit 5
  gh run download <id> -n coverage-summary -D <scratch>
  ```

  Read the `Lines:` line. If the `gh` call fails, ask the maintainer to paste the figure from the
  CI job summary.
- [ ] Apply the pass rule: equal whole percents, both >= 82.
- [ ] Log one S3 DF per `C6` stumble and per trap that bit, each with a contained docs fix to
  `docs/src/contributing/testing-guide.md` (D-05). Append entries that parse under
  `make check-findings-log` (next free id is the one after the last `### DF-nn` in
  `.project/v0.11.1/02-dogfood-findings-log.md`).
- [ ] On pass, `git mv` the todo to `.planning/todos/completed/` and add a
  `## Resolution (YYYY-MM-DD)` section with: local figure, CI figure, CI run id, commit, and the
  `cargo-llvm-cov` versions (local and CI).
- [ ] On fail, log a DF with both figures. The todo
  `2026-08-13-verify-local-coverage-reproduction.md` stays pending with a dated update.
