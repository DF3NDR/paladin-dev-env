# Phase 47 v0.11.0 Crate Release — CI Evidence Record (plans 47-01 onward)

**Phase:** 47-v0-11-0-crate-release
**Branch:** `claude/laughing-dirac-e0h2ax` (PR #59 head; never renamed, D-04)
**Written:** 2026-10-25

This record follows the `29-CI-EVIDENCE.md` / `33-CI-EVIDENCE.md` / `37-CI-EVIDENCE.md` /
`37.1-CI-EVIDENCE.md` house form: a **Local sweep** table, a **CI-run table**, a **Registry
verification (D-00h)** table and a **Findings carried forward (D-00d)** section. It is this phase's own
evidence file: the four older evidence files are sealed at the SHAs they verified and receive dated
additions only, never an edit. Every number in this file is the output of a command run in plan 47-01
(recorded beside the SHA it ran on); a figure that appears in RESEARCH.md but was not re-measured here is
labelled as a research figure, never as a measurement.

---

## Provenance

- **Branch:** `claude/laughing-dirac-e0h2ax`.
- **Base commit (D-02):** `df24689c3d1fd52b0e730105c15b22c2c4ad1404` — `main`'s tip, read live via REST
  (`gh api repos/DF3NDR/paladin-dev-env/branches/main --jq .commit.sha`) at 2026-10-25T13:03:59Z; it equals
  the frozen D-02 baseline (unchanged).
- **Local HEAD (sweep SHA):** `f72fe6acd6a56eab11f5d72944ee8cf8858cace2` (`git rev-parse HEAD`,
  `docs(phase-47): begin phase execution`). It is one docs-only commit ahead of the PR head below and has
  not been pushed; the diff between the two is `.planning/` only.
- **PR #59 head (read from GitHub, `GET /pulls/59`, 2026-10-25T13:01Z):** `head.sha`
  `2e857c6ce7e0d9f37ff5cc74def293d2d0587756`; `title` "feat(43-02): honour the provider retry delay as a
  minimum and cap the gate at max_wait" (not yet retitled; plan 47-05 owns the retitle); `state` open;
  `base` `main`; `commits` 1089; `changed_files` 1096; `mergeable_state` `blocked`. These counts come from
  GitHub because the sandbox clone is shallow (`.git/shallow` present, `git rev-list --count HEAD` = 662,
  no merge base with `origin/main`); CONTEXT.md's 1,081/1,080 and RESEARCH.md's 1083/1082 predate later commits.
- **`origin/claude/laughing-dirac-e0h2ax`** (`git ls-remote origin refs/heads/claude/laughing-dirac-e0h2ax`):
  `2e857c6ce7e0d9f37ff5cc74def293d2d0587756`, equal to the PR head.
- **Toolchain this phase measures on:** `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1`;
  `cargo-semver-checks 0.50.0`; `cargo-deny 0.20.2`; `cargo audit` (`cargo-audit-audit 0.22.2`);
  `rustup toolchain list`: `stable-x86_64-unknown-linux-gnu (default)`,
  `nightly-2026-09-20-x86_64-unknown-linux-gnu`, `1.97.1-x86_64-unknown-linux-gnu (active)` — **MSRV 1.88 is
  absent** (so the MSRV gate has no local row; its CI twin is the `MSRV (Rust 1.88)` job). `cargo-release`
  was not installed at the start of plan 47-01; Task 2 installed exactly 1.1.7 (`--locked`, outside the repository): `cargo-release 1.1.7` (Local sweep rows 13-14).
- **Tick commit:** (filled by plan 47-08).
- **Merge commit:** (filled by plan 47-08).
- **Tagged commit:** (filled by plan 47-08).

Machine-readable SHA lines added by later plans use the exact form `- SHA-<label>: <40-hex>` on their own
line (labels H, R, A0, A1, A2, B).

---

## Local sweep

Rows 1-12 are plan 47-01 Task 1's own record: a read-only pass over the tree, the toolchain, the advisory
databases and the sandbox. Rows 13-18 are Task 2's: the one tool install, the B-1 reproduction, the limits and
the MIGRATION section 10 freeze check. All rows ran on local HEAD `f72fe6acd6a56eab11f5d72944ee8cf8858cace2` on
2026-10-25 between 13:01Z and 13:04Z. Nothing outside `.planning/` was modified.

| # | Command | Result (verbatim/summarized) | Verdict |
|---|---------|-------------------------------|---------|
| 1 | `git rev-parse HEAD` then `git status --porcelain` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | HEAD `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`; `git status --porcelain` printed nothing (empty). Branch `claude/laughing-dirac-e0h2ax`. | CLEAN |
| 2 | `cargo metadata --format-version 1 --no-deps --offline \| jq -c '[.packages[].version]\|unique'` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | `["0.10.1"]` — one distinct version across all 13 workspace packages (`jq '.packages\|length'` = 13). | `0.10.1` everywhere, as expected |
| 3 | `cargo metadata --format-version 1 --no-deps --offline \| jq -r '.packages[]\|select(.publish==null)\|.name' \| sort` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | **12** publishable packages (count > 0): `paladin-ai`, `paladin-ai-core`, `paladin-battalion`, `paladin-content`, `paladin-eval`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web`. The 13th package, `paladin-doc-examples`, reports `publish = []`. The directory `crates/paladin-core` publishes as `paladin-ai-core`. This derived list (D-00f), not a hard-coded one, is what the registry probe below iterates. | DERIVED, 12 |
| 4 | `awk '/^## \[Unreleased\]/{f=1;next} /^## \[/{if(f)exit} f' CHANGELOG.md \| wc -c` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | **156528 bytes** (1843 lines) in the root `CHANGELOG.md` `[Unreleased]` body. Over both limits recorded in Task 2 (argv cap 131,072; the 110,000-byte decision budget). | MEASURED: B-1 exposure |
| 5 | `for f in crates/*/CHANGELOG.md; do awk '/^## \[Unreleased\]/{f=1;next} /^## \[/{if(f)exit} f && NF' $f \| wc -l; done` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | Non-blank `[Unreleased]` lines: `paladin-battalion` 93, `paladin-content` **0**, `paladin-core` (publishes as `paladin-ai-core`) 137, `paladin-eval` 4, `paladin-herald` 25, `paladin-llm` 301, `paladin-memory` **0**, `paladin-notifications` **0**, `paladin-ports` 143, `paladin-storage` 88, `paladin-web` 45. 11 crate changelogs plus the root `CHANGELOG.md` (the `paladin-ai` facade) make the 12. **Three are empty: `paladin-content`, `paladin-memory`, `paladin-notifications`** (feeds the EMPTY decision). | MEASURED |
| 6 | `cargo deny check advisories` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`; cargo-deny 0.20.2) | **Exit 1**, `advisories FAILED`. One distinct id: the diagnostic "error[unsound]: AsyncFd::new and related constructors violate I/O safety", **RUSTSEC-2026-0386**, on `tokio 1.52.3` (`Cargo.lock:530`; lockfile entry at lines 6002-6003). Solution reported by the tool: "Upgrade to >=1.51.6, <1.52.0 OR >=1.53.3 (try `cargo update -p tokio`)". Output saved to the scratchpad; the id appears twice in the output (the diagnostic header and the id line). | **RED**: B-2 reproduced |
| 7 | `cargo audit` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`; fetched the advisory DB, scanned 691 crate dependencies) | **Exit 0**. `warning: 10 allowed warnings found`: unmaintained RUSTSEC-2021-0141, RUSTSEC-2025-0057, RUSTSEC-2025-0119, RUSTSEC-2024-0436, RUSTSEC-2025-0134, RUSTSEC-2026-0249, RUSTSEC-2024-0449; unsound RUSTSEC-2026-0221, RUSTSEC-2026-0205, **RUSTSEC-2026-0386** (dated 2026-10-13). `cargo audit` only warns on RUSTSEC-2026-0386 while `cargo deny` fails on it. | exit 0, 10 warnings |
| 8 | `df -h /` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | `/dev/vda 252G size, 37G used, 2.5G avail, 94% use`, mounted on `/`. | 2.5 GB free: tight for a full rebuild (Pitfall 9) |
| 9 | `du -sh target` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | `20G target`. | MEASURED |
| 10 | `test -S /var/run/docker.sock`; `command -v docker`; `docker info` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | `test -S` exit 1 (no socket). `/usr/bin/docker` present (Client: Docker Engine - Community 29.8.2). `docker info`: `failed to connect to the docker API at unix:///var/run/docker.sock; check if the path is correct and if the daemon is running: dial unix /var/run/docker.sock: connect: no such file or directory`. | **No Docker daemon**: Docker-backed integration rows have no local run |
| 11 | `rustup toolchain list` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | `stable-x86_64-unknown-linux-gnu (default)`, `nightly-2026-09-20-x86_64-unknown-linux-gnu`, `1.97.1-x86_64-unknown-linux-gnu (active)`. No `1.88` toolchain. | MSRV 1.88 absent locally |
| 12 | `gh api repos/DF3NDR/paladin-dev-env/branches/main --jq .commit.sha` (read 2026-10-25T13:03:59Z) | `df24689c3d1fd52b0e730105c15b22c2c4ad1404` — equals the D-02 baseline; `main` has not moved. | UNMOVED |
| 13 | `cd <scratchpad> && cargo install --locked cargo-release --version 1.1.7` (outside the repository; SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`; 13:05:25Z to about 13:08Z) | **Exit 0**. 122 `Compiling` lines; `Installing /root/.cargo/bin/cargo-release`; "Installed package cargo-release v1.1.7 (executable cargo-release)". The only tool installed in this plan (RESEARCH Package Legitimacy Audit: crate-ci/cargo-release, OK verdict). `df -h /` before: `252G size, 37G used, 2.5G avail, 94%`; after: `252G size, 37G used, 2.5G avail, 94%` (the build's low-water mark, read mid-build, was `2.2G avail, 95%`, so the install fit). | INSTALLED |
| 14 | `cargo release --version` then `git status --porcelain` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | `cargo-release 1.1.7` (`release.toml` still records 1.1.2, see Findings). `git status --porcelain` empty after the install: no tracked file changed. | 1.1.7 |
| 15 | B-1 reproduction. Body file = `awk '/^## \[Unreleased\]/{f=1;next} /^## \[/{if(f)exit} f' CHANGELOG.md > <mktemp file>` (156528 bytes); then `( export CREATE_OR_REUSE_RELEASE_LIB_ONLY=1; source scripts/create-or-reuse-release.sh; _cor_build_payload v0.11.0 "Release v0.11.0" <file> false >/dev/null )` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | **Exit 126**. stderr verbatim: `scripts/create-or-reuse-release.sh: line 105: /usr/bin/jq: Argument list too long`. Control, same function on a one-line body: exit 0, 122-byte payload. Threshold probe on truncated copies of the same body: 110000 bytes exit 0 (111453-byte payload), 125000 exit 0, 131000 exit 0, **131100 exit 126**, 140000 exit 126 (the same `Argument list too long` text). `getconf ARG_MAX` = 2097152, `getconf PAGE_SIZE` = 4096, so the per-argument cap is 32 pages = 131072 bytes (`MAX_ARG_STRLEN`), matching the observed break between 131000 and 131100. **B-1 reproduced offline by the real payload builder.** | **RED**: B-1 reproduced |
| 16 | Limits recorded beside row 15 (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | Argv cap **131,072 bytes**: measured in row 15. GitHub release-body cap of **125,000 characters**: a **research figure** (RESEARCH A3, MEDIUM confidence), **not measured** in this plan. Decision budget **110,000 bytes** (about 1 KB is appended later by `finalize-release-body.sh`). Measured root body 156528 bytes exceeds all three: by 25,456 bytes over the argv cap, 46,528 bytes over the decision budget. | MEASURED vs RESEARCH figures, labelled |
| 17 | `git diff --stat e28e43b5..HEAD -- src 'crates/*/src'` (SHA `f72fe6acd6a56eab11f5d72944ee8cf8858cace2`) | 2 files, 33 insertions, 0 deletions: `src/application/cli/config/battalion_config.rs` (25 lines, all inside `#[cfg(test)] mod tests`: one new test `cli_conclave_yaml_timeout_seconds_still_loads`) and `src/infrastructure/resilience/circuit_breaker.rs` (8 lines, all `///` rustdoc on `CircuitBreaker`). Two commits touch these paths since `e28e43b5`: `b0429436` (46-08) and `68d03cdb` (46-09). No `crates/*/src` file changed. `git rev-list --count e28e43b5..HEAD` = 98 commits in all. | no signature changed |
| 18 | `gh api repos/DF3NDR/paladin-dev-env/actions/runs/40194317691/jobs --paginate` (the `push` `ci.yml` run on PR #59's head `2e857c6ce7e0d9f37ff5cc74def293d2d0587756`; read 2026-10-25T13:08:35Z) | **`API Surface Tracking`: `completed` / `success`** (job `121172036318`, completed 13:08:09Z): steps `Check API surface changes=success`, `Check deprecation warnings=success`, `Generate API diff on failure=skipped`. The `pull_request` run `40194322581` carried the same job `in_progress` (4 of 11 steps done) at the same read. | `success` on the PR head |

**MIGRATION section 10 freeze answer (D-00n).** Rows 17-18: the public surface is unmoved since `e28e43b5` — the only two source changes are a test and rustdoc, and `API Surface Tracking` concluded `success` on PR #59's head `2e857c6ce7e0d9f37ff5cc74def293d2d0587756`. **MIGRATION section 10 stays frozen at `e28e43b5`**; nothing is put to the maintainer on this point. (Caveat recorded for plan 47-04: the `pull_request` run of the same job had not concluded at the read; the push-run conclusion is the one cited.)

### B-1 reproduction detail: the root `[Unreleased]` body by subsection (plan 47-01 Task 2)

Measured with `awk` over the body file of row 15 (every `### ` heading inside the section, bytes including newlines; the sum equals `wc -c` = 156528):

| Subsection | Bytes |
|---|---|
| `### Added` | 84567 |
| `### Changed` | 36419 |
| `### Removed` | 5231 |
| `### Fixed` | 11327 |
| `### Documentation` | 1683 |
| `### Security` | 2491 |
| `### Breaking Changes` | 10847 |
| `### Known limitations` | 3962 |
| (lead text before the first `###`, one blank line) | 1 |
| **Total** | **156528** |

`### Added` (84567) plus `### Changed` (36419) is 120,986 bytes; the other six subsections plus the one-byte lead are 35,542 bytes
(`Removed`, `Fixed`, `Documentation`, `Security`, `Breaking Changes`, `Known limitations` sum to 35,541). To land at or under the
110,000-byte budget with the six others kept byte-identical, `Added` plus `Changed` must shrink to at most
74,458 bytes (a cut of at least 46,528 bytes, 38.5 percent), before the lead paragraph B1=a adds.


---

## CI-run table

### Pre-build baseline (before the re-seal; D-08 applies, D-00j does not yet)

Read 2026-10-25T13:03Z, via `gh api "repos/DF3NDR/paladin-dev-env/actions/runs?head_sha=2e857c6ce7e0d9f37ff5cc74def293d2d0587756" --jq '.workflow_runs[]|{name,id,event,status,conclusion,html_url}'`
(GraphQL is blocked in this sandbox; job logs sit on a proxy-blocked host, so the per-job and per-step
conclusions below come from `gh api repos/DF3NDR/paladin-dev-env/actions/runs/<id>/jobs --paginate`).
The head the runs were created on is the PR head `2e857c6ce7e0d9f37ff5cc74def293d2d0587756` (the local HEAD `f72fe6acd6a56eab11f5d72944ee8cf8858cace2` is one unpushed docs commit ahead).
This is a snapshot, not polled to completion (D-00i). Nothing was fixed or re-run.

| Workflow | Run ID (URL) | Event | Status | Conclusion |
|---|---|---|---|---|
| `ci.yml` | [40194322581](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40194322581) | `pull_request` | `queued` | `(none yet)` |
| `codeql.yml` | [40194322577](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40194322577) | `pull_request` | `in_progress` | `(none yet)` |
| `pre-commit.yml` | [40194322619](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40194322619) | `pull_request` | `completed` | `success` |
| `docs.yml` | [40194322628](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40194322628) | `pull_request` | `completed` | `success` |
| `feature-flags.yml` | [40194322607](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40194322607) | `pull_request` | `in_progress` | `(none yet)` |
| `ci.yml` | [40194317691](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40194317691) | `push` | `queued` | `(none yet)` |
| `pre-commit.yml` | [40194317688](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40194317688) | `push` | `completed` | `success` |
| `feature-flags.yml` | [40194317692](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40194317692) | `push` | `completed` | `success` |
| `codeql.yml` | [40194317735](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40194317735) | `push` | `completed` | `success` |

**What this snapshot shows.** Both `ci.yml` runs (the `pull_request` run `40194322581` and the `push` run
`40194317691`) had not concluded at read time: 34 jobs each, a mix of `completed`, `in_progress` and `queued`.
The only non-`success` *completed* job in either run is **`License & Dependency Policy` — `failure`** (job
`121172138252` in the PR run, `121172036253` in the push run), and the failing step in both is
**`Run cargo-deny check`** (every other step `success` or `skipped`). That is the B-2 symptom
(RUSTSEC-2026-0386 on tokio 1.52.3, Local sweep row 6), recorded, **not acted on** (D-08 stage 1).

Job tally of the PR run `40194322581` at the read time (`completed/success` 15, `completed/failure` 1,
`in_progress` 7, `queued` 11), and of the push run `40194317691` (`completed/success` 12, `completed/failure`
1, `completed/skipped` 1 = `Benchmark Regression Signal (Non-Blocking)`, `in_progress` 8, `queued` 12).
`End-to-End Tests` and `Publish Dry Run` do not appear in either job list: they are `main`-push-only jobs, so
their first run on this tree is on the merge commit (Findings, D-14).
`API Surface Tracking` was `queued` in the PR run and `in_progress` in the push run — **not concluded**; see
the section 10 freeze entry in Findings.

PR run `40194322581` jobs at read time:

| Job | Status | Conclusion |
|---|---|---|
| Security Audit | completed | success |
| Crate Isolation (paladin-ai) | completed | success |
| Crate Isolation (paladin-web) | completed | success |
| Crate Isolation (paladin-battalion) | completed | success |
| Redis Node Cache Contract Suite (live server) | completed | success |
| Crate Isolation (paladin-storage) | completed | success |
| Crate Isolation (paladin-ports) | completed | success |
| Benchmark Regression Signal (Non-Blocking) | in_progress | (none yet) |
| License & Dependency Policy | completed | failure |
| Workflow Lint | completed | success |
| Crate Isolation (paladin-notifications) | completed | success |
| Crate Isolation (paladin-memory) | completed | success |
| Postgres Storage Contract Suites (live server) | completed | success |
| Semver Checks (vs v0.9.0) | in_progress | (none yet) |
| Generated SDK Clients (Python + TypeScript) smoke | completed | success |
| OSV Scanner | in_progress | (none yet) |
| CLI Snapshot Tests | completed | success |
| Integration Tests | in_progress | (none yet) |
| Docker Integration Tests | queued | (none yet) |
| Ollama Integration Tests (live server) | queued | (none yet) |
| Coverage | queued | (none yet) |
| Benchmark Compile Check | in_progress | (none yet) |
| Redis Cadence Contract Suite (live server) | queued | (none yet) |
| E2E Platform API (PRD 06 acceptance-1 lifecycle + SHIP-02 boot proof) | queued | (none yet) |
| Unit Tests (beta) | completed | success |
| API Surface Tracking | queued | (none yet) |
| MSRV (Rust 1.88) | queued | (none yet) |
| Unit Tests (stable) | queued | (none yet) |
| Code Quality | in_progress | (none yet) |
| Crate Isolation (paladin-llm) | completed | success |
| Consumer-Surface Muster | in_progress | (none yet) |
| Redis Run Queue Contract Suite (live server) | queued | (none yet) |
| Crate Isolation (paladin-content) | queued | (none yet) |
| Crate Isolation (paladin-ai-core) | queued | (none yet) |

---

## Registry verification (D-00h)

### Pre-tag baseline (2026-10-25)

Read-only sparse-index probe, one unauthenticated `GET` per crate, run from the repository root against the
**publishable set derived from `cargo metadata`** (Local sweep row 3: 12 names, count > 0). The only header
sent is the mandatory `User-Agent` (`paladin-release-verify (github.com/DF3NDR/paladin-dev-env)`); no
credential of any kind (D-00g). Probe command (RESEARCH Pattern 3, verbatim from 37.1):

```bash
cargo metadata --format-version 1 --no-deps | jq -r '.packages[] | select(.publish == null) | .name' | sort | while read -r c; do
  n=${#c}; if [ $n -eq 1 ]; then p="1/$c"; elif [ $n -eq 2 ]; then p="2/$c"; elif [ $n -eq 3 ]; then p="3/${c:0:1}/$c"; else p="${c:0:2}/${c:2:2}/$c"; fi
  row=$(curl -sf -H 'User-Agent: paladin-release-verify (github.com/DF3NDR/paladin-dev-env)' "https://index.crates.io/$p" \
        | jq -c --arg v "$V" 'select(.vers==$v) | {vers,yanked,cksum}' || true)
  echo "$c | ${row:-ABSENT}"
done
```

**`V=0.10.1`** (run at 2026-10-25T13:03:42Z) — 12 of 12 present, none yanked:

| Crate | Result |
|---|---|
| paladin-ai | `{"vers":"0.10.1","yanked":false,"cksum":"9ca40b3ecd8d21dca56205af54973e01646703bd37cf83c4a4b2b9505d3009b6"}` |
| paladin-ai-core | `{"vers":"0.10.1","yanked":false,"cksum":"3b2d36cbc223ac9cceb32c6e91a83b63b4f5d6a4a1399fa20fda07a41a317d32"}` |
| paladin-battalion | `{"vers":"0.10.1","yanked":false,"cksum":"7fbcdf4e1e1863ac146006db2fa1199f491d5f5ef747673498a0a4809150e181"}` |
| paladin-content | `{"vers":"0.10.1","yanked":false,"cksum":"2379c7001ee2195c0c9c12a9b8b8c94bd37dd84a141722e687b32806ed5429e1"}` |
| paladin-eval | `{"vers":"0.10.1","yanked":false,"cksum":"8903120815e1ada064780b9f7134fcfbd3df18987f978e84862488cfb2fd7506"}` |
| paladin-herald | `{"vers":"0.10.1","yanked":false,"cksum":"341e015e09f78b9df09877535477cd04591371c8b183299297097e7591faabbc"}` |
| paladin-llm | `{"vers":"0.10.1","yanked":false,"cksum":"cc450e36220149b81275bc1a15ffa0da8a30e6b499db9cfae9ed93a326995aea"}` |
| paladin-memory | `{"vers":"0.10.1","yanked":false,"cksum":"db60eb8e74e00ae009555b01c99f78284ddf8a68c0a75a43cb1762c371f104d2"}` |
| paladin-notifications | `{"vers":"0.10.1","yanked":false,"cksum":"54ab5b7c46428f2a1c2d43b6aab073ea9b1313a95d9e520e921a4717acc88cda"}` |
| paladin-ports | `{"vers":"0.10.1","yanked":false,"cksum":"cb2876ff46b7b506bae9fa6c0f94217deef6a20485b5d12d76187b8b1482b885"}` |
| paladin-storage | `{"vers":"0.10.1","yanked":false,"cksum":"6e912031e15a2c2d252c94202a5fd4950ac432cd139153dc1c57836b71d52078"}` |
| paladin-web | `{"vers":"0.10.1","yanked":false,"cksum":"fdeb5bf5908a36f47ef1a75a946cd59beafa9069b20f487a252ad33d49287073"}` |

**`V=0.11.0`** (run at 2026-10-25T13:03:47Z, five seconds after the `0.10.1` run in the same session, so an
`ABSENT` here is a registry answer and not a network failure: the `0.10.1` run of the same command
returned all twelve rows) — 12 of 12 `ABSENT`:

| Crate | Result |
|---|---|
| paladin-ai | `ABSENT` |
| paladin-ai-core | `ABSENT` |
| paladin-battalion | `ABSENT` |
| paladin-content | `ABSENT` |
| paladin-eval | `ABSENT` |
| paladin-herald | `ABSENT` |
| paladin-llm | `ABSENT` |
| paladin-memory | `ABSENT` |
| paladin-notifications | `ABSENT` |
| paladin-ports | `ABSENT` |
| paladin-storage | `ABSENT` |
| paladin-web | `ABSENT` |

Pre-tag expectation met: every derived publishable crate is present at `0.10.1` with `yanked = false`, and
every one is `ABSENT` at `0.11.0`. The formal table below is filled by plan 47-08, after the publish run.

| Crate | `vers` | `cksum` | `yanked` |
|---|---|---|---|

---

## Findings carried forward (D-00d)

Every entry below is recorded, none is fixed (D-00d). Entries are dated and appended; an earlier entry is amended
at source only when a later plan corrects it, with a dated note.

### 2026-10-25 — D-09: the CI semver job is pinned to `--baseline-version 0.9.0` and is a no-op (deferred)

`.github/workflows/ci.yml` job `Semver Checks (vs v0.9.0)` (display name at line 346) runs
`cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` for eleven packages
(lines 381-401); `paladin-eval` is deliberately excluded (comment at lines 371-379: no published `0.9.0`
baseline). The "known no-op" figure is carried from `37.1-CI-EVIDENCE.md` row 16 (`0 checks: 0 pass, 254 skip`
on `paladin-battalion`), **not re-measured in this plan** (the job was still running on the PR head at the
read). Deferred per D-09. Recommendation: in the next milestone's first phase re-pin the baseline to `0.11.0`
and add `paladin-eval`. If the job fires at `0.11.0`, D-00j applies (A-4).

### 2026-10-25 — D-14: gates with no pull-request twin

(1) `check-migration-allowlist` is **not wired into `ci.yml`**: `grep -n migration .github/workflows/ci.yml`
returns nothing; the `License & Dependency Policy` job runs `check-changelogs`, `check-crate-names`,
`check-advisory-register`, `check-workflow-suppressions`, `check-workflow-triggers`, `check-codeql-dismissals`,
`check-publish-order` and `check-dependency-allowlist` as individual steps (lines 128-165), but not
`check-migration-allowlist`; it runs only through local `make check-gates`. (2) `End-to-End Tests` (`ci.yml`
lines 2009-2013) and `Publish Dry Run` (lines 2152-2156) carry
`if: github.event_name == 'push' && github.ref == 'refs/heads/main'`; neither appears in the job list of either
`ci.yml` run on the PR head (34 jobs each, CI-run table above). Their **first run on this tree is on the merge
commit** (RESEARCH Pitfall 8), after the irreversible merge and before the tag.

### 2026-10-25 — B-1: the release body exceeds the payload builder's argv cap (blocker, reproduced)

The root `CHANGELOG.md` `[Unreleased]` body is **156528 bytes** (Local sweep row 4); its measured subsections are
in the B-1 table below row 18 (`Added` 84567, `Changed` 36419, `Breaking Changes` 10847, `Fixed` 11327,
`Removed` 5231, `Known limitations` 3962, `Security` 2491, `Documentation` 1683). `_cor_build_payload v0.11.0
"Release v0.11.0" <body> false` exited **126** with `/usr/bin/jq: Argument list too long` (row 15); the break
falls between 131000 and 131100 bytes, the 131,072-byte per-argument cap. A failed `create-release` blocks
`publish-crates` after the tag is pushed and immovable. To land at 110,000 bytes with the six non-`Added`/`Changed`
subsections intact, `Added` plus `Changed` must go from 120,986 to at most 74,458 bytes. The 125,000-character
GitHub cap is a research figure (A3), not measured here. Put to the maintainer as B1 in Task 3.

### 2026-10-25 — B-2: RUSTSEC-2026-0386 turns `cargo deny` red; `cargo audit` only warns (blocker, reproduced)

`cargo deny check advisories` exits 1 (`advisories FAILED`) on **RUSTSEC-2026-0386** (tokio `1.52.3`,
`AsyncFd::new` and related constructors violate I/O safety; `Cargo.lock:530`; the lockfile carries
`name = "tokio"` / `version = "1.52.3"` at lines 6002-6003); the tool's own solution line reads "Upgrade to
>=1.51.6, <1.52.0 OR >=1.53.3 (try `cargo update -p tokio`)". `cargo audit` exits 0 and lists the same id among 10
allowed warnings (row 7). On the PR head `2e857c6ce7e0d9f37ff5cc74def293d2d0587756` the required
`License & Dependency Policy` context is **`failure`** in both `ci.yml` runs (jobs `121172138252`, `121172036253`),
failing step `Run cargo-deny check` (CI-run table). `make security` must pass before a phase seals. Put to the
maintainer as B2 in Task 3. Nothing was changed.

### 2026-10-25 — Row-7 wording: the stamp script keeps the `## [Unreleased]` anchor

`scripts/finalize-crate-changelogs.sh` inserts the dated section **immediately after** `## [Unreleased]` and
preserves the anchor (lines 34-38 and 241-245: `replacement = f"{anchor}\n\n{new_heading}"`); the
preservation is pinned by `tests/scripts/finalize-crate-changelogs_test.sh` around lines 170-173
(`grep -qF -- "## [Unreleased]"` must succeed after stamping). So after the stamp an `[Unreleased]` count of 1
survives per changelog, and D-11 row 7's literal "count 0" needs a decision. Put to the maintainer as R7 in Task 3.

### 2026-10-25 — Three crates have an empty `[Unreleased]`

Re-measured (Local sweep row 5): `paladin-content`, `paladin-memory` and `paladin-notifications` each have **0**
non-blank `[Unreleased]` lines, so their stamped `[0.11.0]` section would be a heading with no body (the gate
passes: presence is the signal). Nine others are non-empty (`paladin-eval` has 4). Put to the maintainer as EMPTY
in Task 3; no content is invented.

### 2026-10-25 — `k8s/server/worker-deployment.yaml` still labels `version: v0.10.0`

`k8s/server/worker-deployment.yaml` lines 20 and 35 carry `version: v0.10.0` (metadata label and pod-template
label); `grep -rn v0.10.0 k8s` finds only these two. Stale (the workspace is `0.10.1`). Recorded, **not fixed**: it
is not a gated release input, and a change outside `.planning/` is barred in this plan.

### 2026-10-25 — Shallow clone: commit counts come from GitHub

`.git/shallow` is present (two shallow roots); `git rev-list --count HEAD` = 662 and there is no merge base with
`origin/main`. PR #59's counts are read from `GET /pulls/59` at head `2e857c6c`: **1089 commits, 1096 changed
files** (RESEARCH recorded 1083/1082 and CONTEXT 1,081/1,080; the figures move with every docs commit and are
point-in-time observations, never contracts).

### 2026-10-25 — 13 open WINDOWS rows; none is Phase 47's

`node .claude/gsd-core/bin/gsd-tools.cjs windows status`: 82 entries, **13 open** (ids 56, 58, 62, 63, 64, 65, 66,
67, 68, 69, 73, 77, 79; 39 waived, 30 fixed), `last_updated` 2026-10-24T13:47:20Z. Kinds: `lint-warning` (56),
`deviation` (58, 62-67, 77, 79) and `unrun-verify` (68, 69, 73), from phases 38, 40, 41, 42, 48, 50 and 50.1. None
originates in Phase 47. `/gsd-ship` is not used by this phase because PR #59 already exists. The 13 rows are carried
to the ready-to-close prerequisites for the milestone audit.

### 2026-10-25 — CONTEXT drift (items the build wave must use, not CONTEXT's wording)

(1) `Cargo.toml` line 58 is the root `[package]` version (`version = "0.10.1"`); there is **no
`[workspace.package]` version** (that table, from line 12, carries only `rust-version = "1.88"`).
(2) `Cargo.lock` changes in the bump commit (13 workspace entries). (3) `crates/paladin-ports/Cargo.toml` line 16
carries an exact pin `paladin-ai-core ... version = "=0.10.1"`, and `0.10.1` also appears inside `#` comments of
that file (lines 71, 85), which are correct history and must not be rewritten. (4) `make finalize-crate-changelogs`
takes only `VERSION` (Makefile lines 634-639, no date argument) and stamps today's date; a chosen date needs the
script's `--date` flag. (5) `release.toml` lines 49-50 record `cargo release --version` as `cargo-release 1.1.2`;
this phase uses 1.1.7 (Local sweep row 14).

### 2026-10-25 — Stale count comments and a version-bearing job name

`release.toml` lines 41 and 47 say "the ten per-crate CHANGELOG.md files" (11 crate changelogs plus the root exist,
Local sweep row 5); `scripts/finalize-crate-changelogs.sh:8` says "eleven-file manual chore: today all ten crate";
`.github/workflows/ci.yml:277` says "all ten"; the `ci.yml` job display name is `Semver Checks (vs v0.9.0)` (line
346). `Makefile:629` already says "all twelve publishable crates" (correct). Recorded, not edited.

### 2026-10-25 — MIGRATION section 10 freeze check (D-00n): public surface unmoved

`git diff --stat e28e43b5..HEAD -- src 'crates/*/src'` lists exactly two files, 33 insertions, no deletions:
`src/application/cli/config/battalion_config.rs` (+25, test module only) and
`src/infrastructure/resilience/circuit_breaker.rs` (+8, rustdoc only); no `crates/*/src` file changed (Local sweep
row 17). `API Surface Tracking` concluded `success` on PR #59's head in the `push` `ci.yml` run `40194317691` (row 18;
the `pull_request` run's copy was `in_progress` at the read). **Answer: MIGRATION section 10 stays frozen at
`e28e43b5`.** Plan 47-04 re-reads the `pull_request` run's copy of the job.

### 2026-10-25 — Pre-build CI state: local HEAD is one unpushed docs commit ahead of the PR head, PR not yet retitled

Local HEAD `f72fe6acd6a56eab11f5d72944ee8cf8858cace2` is one docs-only commit ahead of `origin/claude/laughing-dirac-e0h2ax`
= PR #59 head `2e857c6ce7e0d9f37ff5cc74def293d2d0587756`; PR #59's title is still "feat(43-02): honour the provider
retry delay as a minimum and cap the gate at max_wait" (plan 47-05 retitles it). Both `ci.yml` runs on the PR head
were unconcluded at the read (the push run's `API Surface Tracking` had already succeeded; the
`License & Dependency Policy` failure had landed in both). Earlier heads' runs (`22540cd2`, `885ec5ed`, `869091b0`)
read `cancelled`, superseded by later pushes. No run on any head has yet concluded green for the whole workflow.

### 2026-10-25 — Sandbox capacity and tooling limits for the re-seal

`df -h /` reads 2.5 GB available (94 percent used) with `target` at 20 GB (rows 8-9); the `cargo-release` install
fit (low-water mark 2.2 GB), but RESEARCH Pitfall 9 expects the post-bump rebuild not to (A-2): the SPACE decision in
Task 3 governs recovery. There is **no Docker daemon** (the client binary `/usr/bin/docker` 29.8.2 is present, the
socket `/var/run/docker.sock` is absent, row 10), so Docker-backed integration rows have no local run; their CI
twins are the evidence. The MSRV `1.88` toolchain is absent (row 11); the CI `MSRV (Rust 1.88)` job is the twin.
