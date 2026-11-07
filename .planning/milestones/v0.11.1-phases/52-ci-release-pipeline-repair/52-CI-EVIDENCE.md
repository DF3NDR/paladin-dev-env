---
phase: 52
slug: ci-release-pipeline-repair
status: verified
created: 2026-10-28
---

# Phase 52: CI and release pipeline repair, evidence record

> Measured facts for the phase's CI gates. Each section records the command, the tool versions, the tree and the
> output that was actually observed, so a later reader can repeat the measurement rather than trust a summary.

## D-02 allow-table measurement

Plan 52-05, Task 1. Purpose: prove that the 28 crate-wide `cargo-semver-checks` allow lines (six manifests) were
redundant at the published `0.11.0` baseline, by removing every table and then running the gate over all twelve
published packages.

- HEAD SHA when measured (manifests edited in the working tree, uncommitted): `95659216ab4a514c3c615194638130f9d9bd9944`
- `cargo semver-checks --version`: `cargo-semver-checks 0.50.0`
- `rustc --version`: `rustc 1.97.1 (8bab26f4f 2026-07-14)`
- Baseline: published `0.11.0` from crates.io, `--release-type patch` (fixed by flag; every manifest equals the baseline version).
- Per-package command, run through `scripts/check-semver.sh --package <pkg>` with `SEMVER_PRUNE_LOCAL_CACHE=1`:

```text
cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.11.0 --release-type patch
```

The twelve packages are the `CRATES` array of `scripts/publish-crates.sh`. The script was invoked once per package
rather than once for all twelve; the loop body, the command and the check-line parsing are the same, and the
per-package form let free disk space be read after every package (the sandbox had about 575 MB free).

| Package | Check line | Exit |
|---|---|---|
| `paladin-ai-core` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-ports` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-herald` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-battalion` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-llm` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-memory` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-web` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-notifications` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-content` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-storage` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-eval` | 223 checks: 223 pass, 31 skip | 0 |
| `paladin-ai` | 223 checks: 223 pass, 31 skip | 0 |

Every package printed `semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`.

Lints that fired: none.

### Removed allow lines

| Manifest | Package | `= "allow"` lines removed |
|---|---|---|
| `Cargo.toml` | `paladin-ai` | 1 |
| `crates/paladin-core/Cargo.toml` | `paladin-ai-core` | 11 |
| `crates/paladin-battalion/Cargo.toml` | `paladin-battalion` | 2 |
| `crates/paladin-llm/Cargo.toml` | `paladin-llm` | 4 |
| `crates/paladin-ports/Cargo.toml` | `paladin-ports` | 6 |
| `crates/paladin-web/Cargo.toml` | `paladin-web` | 4 |

Total 1 + 11 + 2 + 4 + 6 + 4 = 28. The removed text is the `[package.metadata.cargo-semver-checks.lints]` table, any
comment lines inside it, and the contiguous comment block above it that justified the allows. `git diff -U0` showed
only comment lines, the six table headers, six separating blank lines and the 28 allow lines removed, and no added line.

### Disk and one retry

- Free space before the first package and after each of the first eleven: 575 MB (volume `/dev/vda`, 99% used).
- The first attempt at `paladin-web` failed with exit 101. This was not a lint: the rustdoc build of the `0.11.0`
  baseline ran out of space (`No space left on device`, a linker bus error in `rust-embed-impl`, then a failed
  `rmeta` write), so no check line was produced. The stale `0.9.0` baseline directory
  `target/semver-checks/registry-paladin_web-0_9_0-*` (about 571 MB, no longer used by any gate) was removed once,
  which raised free space to 1.2 GB. The retry of `paladin-web` then produced the line recorded above, with no
  `No space left` text in its log. Free space after each of the remaining packages: 1.2 GB.

### Known limit

RESEARCH Pitfall 9 (the coupling of this measurement to the exact toolchain and its rustdoc JSON format) is the
known limit of this measurement: a different `rustc` or `cargo-semver-checks` version can change the lint set that
runs or what it reports, so the result above is exact for `cargo-semver-checks 0.50.0` on `rustc 1.97.1` only.

## Folded todo: local coverage reproduction

Source: `.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md` (folded into Phase 52 by CONTEXT as a
verification note only). Plan 52-08 Task 1, 2026-10-29. The todo file is unchanged and stays pending
(`resolves_phase: 56`).

**Procedure attempted (PD-08-1).** The todo's procedure needs `cargo-llvm-cov` and a running Docker daemon: `make
services-up`, then `make coverage` (which runs `scripts/coverage.sh`). The attempt was the two probes that decide
whether that procedure can start.

| Probe | Command | Result |
|-------|---------|--------|
| `cargo-llvm-cov` | `cargo llvm-cov --version`; `command -v cargo-llvm-cov` | `error: no such command: llvm-cov`; `command -v` exit 1. Not installed. |
| Docker CLI | `docker --version` | `Docker version 29.8.2, build 7fc2dff` (client only) |
| Docker daemon | `docker info` | exit 1; client section prints, then `failed to connect to the docker API at unix:///var/run/docker.sock; check if the path is correct and if the daemon is running: dial unix /var/run/docker.sock: connect: no such file or directory` |

Neither precondition holds, and the disk allowance (about 1 GB free on `/`) could not hold a workspace coverage build
in any case. `make services-up` and `make coverage` were therefore **not run**, and no local `Lines:` figure exists.
Nothing in this section claims one.

**CI comparison figures (read, not measured here).**

- Reference of record: `Lines: 167104/180183 = 92.74%` on merge commit `70657919beadbd341a72be2d0107080017a803f0`
  (run 40396108246, the v0.11.0 release; quoted in STATE.md).
- Latest `Coverage` job on the phase branch, read from the public Actions API (2026-10-29T04:21Z, HTTP 200): job
  123230155496 in run 40807444391 (`ci.yml`, `push`, `head_sha` `0c2b4a2bd56c20566bece912a74644b75e5cc2b6`) is
  `completed`, `success`; steps `Measure coverage`, `Coverage summary` and `Upload coverage to Codecov` all `success`.
  Run 40789100661 on `95659216ab4a514c3c615194638130f9d9bd9944` (job 123170114565) is likewise `completed`, `success`.
  A success means the `--fail-under-lines` floor held. The printed `Lines:` figure is in the job log, which the public
  API does not serve without a token, so no new percentage is quoted for these runs.
- The run on the current head `458d1f46e` (run 40816898656) was `queued` at read time.

**Verdict: not reproducible here; H-06 opened.** The local half of the todo remains unwalked. The CI half is green on
the phase branch (floor held on `0c2b4a2bd`), with 92.74 % on `70657919` as the last figure of record. H-06 in
`52-HANDOVER.md` carries the procedure for a Docker-capable machine. No coverage gate or threshold changed in this
phase (`scripts/coverage.sh` and the `coverage` job in `ci.yml` are untouched by this plan); the todo stays pending
(resolves_phase 56).

## First PR run

H-02 of `52-HANDOVER.md`, observed 2026-10-29 on PR #61 (<https://github.com/DF3NDR/paladin-dev-env/pull/61>, head
`5e993f681cbb9dbf6575ae56674d252d7e80dcf1`, the phase branch into `main`). Reads are from the Actions API with the
maintainer's token; job ids are quoted so each line is re-checkable.

| Step | Run | Job | Conclusion |
|------|-----|-----|------------|
| No label: `Publish Dry Run` | 40928112540 (`pull_request`) | 123647995031 | `success` |
| No label: `End-to-End Tests` | 40928112540 | 123653075412 | `skipped` (job-level `if:` false; reports success to the required-check gate, as on PR #59) |
| `release` label applied (maintainer act; the label did not exist and was created first with `gh label create release`) | run 40928112540 then read `completed cancelled`: the `labeled` event started a new run and `cancel-in-progress` cancelled the older one on the same ref (D-07, accepted) | | |
| Labelled: `Publish Dry Run` | 40932320347 (`pull_request`, `labeled`) | 123658055531 | `success` (4m37s) |
| Labelled: `End-to-End Tests` | 40932320347 | 123663512686 | `success` (12m25s) |
| `Build MDBook` (docs.yml, PR-only) | 40928112482 | 123638982883 | `success` |

No finding: `Publish Dry Run` passed at the already-published `0.11.0` version (RESEARCH A6 did not materialise), and the
`labeled` trigger produced the e2e run without any workflow change. The labelled run's remaining jobs were still
`in_progress` at 2026-10-29T19:10Z; its `End-to-End Tests` conclusion is the H-02 record and does not depend on them.

Executor re-read, plan 52-08 Task 3, 2026-10-29T23:18Z (public API): jobs 123647995031, 123653075412, 123658055531 and 123663512686 read `success`, `skipped`, `success`, `success` as tabled above. Run 40932320347 reads `cancelled` at run level only because `Docker Build` and `Kubernetes Smoke Test` in it were cancelled by a later push; its `End-to-End Tests` job is `success`.
