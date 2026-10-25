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
  is not installed at the start of plan 47-01; Task 2 installs exactly 1.1.7 (`--locked`) and records it.
- **Tick commit:** (filled by plan 47-08).
- **Merge commit:** (filled by plan 47-08).
- **Tagged commit:** (filled by plan 47-08).

Machine-readable SHA lines added by later plans use the exact form `- SHA-<label>: <40-hex>` on their own
line (labels H, R, A0, A1, A2, B).

---

## Local sweep

Rows 1-12 are plan 47-01 Task 1's own record: a read-only pass over the tree, the toolchain, the advisory
databases and the sandbox. All rows ran on local HEAD `f72fe6acd6a56eab11f5d72944ee8cf8858cace2` on
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

