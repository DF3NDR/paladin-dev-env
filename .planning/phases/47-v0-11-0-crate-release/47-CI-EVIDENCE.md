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
- SHA-PRE-B1: 700a92d86f3d6b2e24bbd2c163daec7c307297e0
- B1-MODE: a
- SHA-PRE-BUMP: 680cf9c294dc22de02c1faae19afc45188470a35
- SHA-H: c945684647c4f9c78fdde2fbe2fc8fc2e8806106
- SHA-R: d82d67cac926412a21cccd72278fe8fe9df672c0
- SHA-A0: d4e66b47628559b175f1958b4f5267058096dd7f
- **B-1 pre-condensation measurements (plan 47-02 Task 2, measured at `SHA-PRE-B1`):** root `[Unreleased]` body 156528 bytes; `### Added` 84557 bytes and 57 top-level bullets; `### Changed` 36407 bytes and 44 top-level bullets (a top-level bullet is a column-0 `- ` line with everything nested under it; the subsection byte counts here exclude the heading line, so they sit 10 bytes under the 47-01 table's 84567 and 36419, which include it). Lead paragraph added by Task 2 names the SHA above.

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
| 19 | B-2 remedy (plan 47-02 Task 1): `cargo update -p tokio --precise 1.53.3`, then `git diff --name-only` (parent `7be8fe48`; committed as `0e1f7958976b84ac92cac7cd0a965b44c1359463`) | `Updating tokio v1.52.3 -> v1.53.3`; `git diff --name-only` printed exactly `Cargo.lock`. Package-level change: **`tokio` 1.52.3 -> 1.53.3** (checksum `8fc7f01b...` -> `ce3335fa...`); no package added or removed. Seven Windows-only dependency edges were re-pointed by the resolver to already-locked `windows-sys` entries (`errno` 0.3.14, `is-terminal` 0.4.17, `quinn-udp` 0.5.16, `rustix` 1.1.4, `rustls-platform-verifier` 0.7.0, `tempfile` 3.27.0 -> `windows-sys 0.52.0`; `nu-ansi-term` 0.50.3 -> `windows-sys 0.59.0`; each was `windows-sys 0.61.2`); none affects a Linux build. | B2=a APPLIED |
| 20 | `cargo metadata --locked --offline --format-version 1 --no-deps` (SHA `0e1f7958976b84ac92cac7cd0a965b44c1359463`) | Exit 0. | PASS |
| 21 | `cargo deny check` (all four checks; cargo-deny 0.20.2; SHA `0e1f7958976b84ac92cac7cd0a965b44c1359463`) | **Exit 0**: `advisories ok, bans ok, licenses ok, sources ok`. Row 6's `RUSTSEC-2026-0386` diagnostic is gone. | PASS (was exit 1, row 6) |
| 22 | `cargo audit` (SHA `0e1f7958976b84ac92cac7cd0a965b44c1359463`) | **Exit 0**; `warning: 9 allowed warnings found` (row 7 had 10); `RUSTSEC-2026-0386` no longer appears in the output. | PASS |
| 23 | `make check-gates` (SHA `0e1f7958976b84ac92cac7cd0a965b44c1359463`) | **Exit 0**; final lines: publish-order check `OK: 12 crate(s) checked`, and the `paladin-core (14)` / `paladin-ports (12)` dependency-set check. | PASS |
| 24 | `cargo release version 0.11.0 --workspace` (dry run, no `--execute`; cargo-release 1.1.7; SHA `680cf9c294dc22de02c1faae19afc45188470a35`) | Printed `Upgrading <crate> from 0.10.1 to 0.11.0` for all 13 packages and the dependency re-pins ("Updating workspace's dependency ...", "Updating paladin-ai's dependency ...", "Updating paladin-doc-examples's dependency ..."); ended `warning: aborting release due to dry run; re-run with --execute`. No tag, publish or push step appears in the output (`release.toml`: `publish = false`, `push = false`). | PASS (verified before executing) |
| 25 | `cargo release version 0.11.0 --execute --no-confirm --workspace`, then `git diff --name-only` (SHA `680cf9c294dc22de02c1faae19afc45188470a35`) | Exit 0. `git diff --name-only` is exactly 14 files: root `Cargo.toml`, the 12 `crates/*/Cargo.toml` (`doc-examples`, `paladin-battalion`, `paladin-content`, `paladin-core`, `paladin-eval`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web`) and `Cargo.lock`. The tool rewrote `crates/doc-examples` itself (its `[package]` version and all eight `paladin-*` dependency pins (`paladin-ai`, `paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-content`, `paladin-storage`; the plan text says nine, the file carries eight, none is left at the old version) read `0.11.0`): **no manifest line was hand-touched**. | PASS, 14 files |
| 26 | `cargo metadata --format-version 1 --no-deps --offline \| jq -e '([.packages[].version]\|unique)==["0.11.0"] and ([.packages[].dependencies[]\|select(.path!=null and .req!="*")\|.req\|test("0\\.11\\.0")\|not]\|any\|not)'` (working tree after the bump) | Printed `true`, exit 0. Unique workspace version `["0.11.0"]`. 45 versioned path-dependency requirements, all naming `0.11.0`. The only path dependencies with `req "*"` are the three Phase 37.1 publish-order dev-dependencies: `paladin-battalion -> paladin-llm`, `paladin-battalion -> paladin-storage`, `paladin-llm -> paladin-storage` (kind `dev`) - by design, not a missed pin. | PASS |
| 27 | `grep -nE '0\.10\.1' Cargo.toml crates/*/Cargo.toml \| grep -vE '^[^:]+:[0-9]+:[[:space:]]*#'` | Empty (filtered grep exit 1). The unfiltered grep still finds 14 comment-only lines (the `cargo-semver-checks 0.50.0 vs the published 0.10.1` history in `Cargo.toml`, `paladin-battalion`, `paladin-core`, `paladin-llm`, `paladin-ports`); they are correct history and were not edited (RESEARCH Pitfall 5). | PASS (empty) |
| 28 | `cargo metadata --locked --offline --format-version 1 --no-deps` | Exit 0: `Cargo.lock` is in sync with the bumped manifests. | PASS |
| 29 | `awk '/^name = /{n=$3} /^version = "0.10.1"/{print n}' Cargo.lock` | Lists only third-party crates still at `0.10.1`: `chacha20poly1305`, `core-foundation`, `rand`, `rand_core`. The 13 workspace packages (`paladin-ai`, `paladin-ai-core`, `paladin-battalion`, `paladin-content`, `paladin-doc-examples`, `paladin-eval`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web`) all read `0.11.0` in the lockfile. | PASS |
| 30 | `sed -n 16p crates/paladin-ports/Cargo.toml` | `paladin_core = { package = "paladin-ai-core", version = "=0.11.0", path = "../paladin-core" }` - the exact pin reads `=0.11.0`. | PASS |
| 31 | `cargo metadata --format-version 1 --no-deps --offline \| jq -r '.packages[]\|select(.publish==null)\|.name' \| sort` (E-2) | 12 names, non-empty, identical in membership to row 3: `paladin-ai`, `paladin-ai-core`, `paladin-battalion`, `paladin-content`, `paladin-eval`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web`. | PASS, 12 unchanged |
| 32 | `make check-gates` (working tree after the bump) | **Exit 0**; `OK: 12 crate(s) checked in publish order; every versioned workspace dependency (normal, dev and build) resolves to an earlier CRATES position, and CRATES exactly matches the cargo-metadata publishable set`; `paladin-core (14)` and `paladin-ports (12)` dependency sets match ADR-0015. `check-publish-order` read the bumped pins. | PASS |
| 33 | `git commit` of the 14 files, then `git diff --name-only HEAD~1 HEAD` | Commit `1a77f7bd39d8c6d3f0cbc3f25ff900ec03e15efb` `chore(release): bump workspace to 0.11.0`; its file list is the 14 files of row 25, no deletion (`--diff-filter=D` empty). Not pushed. | PASS |
| 34 | `./scripts/finalize-crate-changelogs.sh --version 0.11.0 --date 2026-10-25` (DATE from `47-01-SUMMARY.md` `## Pre-build decision`; SHA `dacdbfc0`) | Exit 0: `OK: 12 publishable package(s) checked for version '0.11.0' (date 2026-10-25)`; each of `paladin-ai`, `paladin-ai-core`, `paladin-battalion`, `paladin-content`, `paladin-eval`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web` reported `finalized`. It inserted `## [0.11.0] - 2026-10-25` after the `## [Unreleased]` anchor in 12 files (24 insertions; `paladin-ai` is the root `CHANGELOG.md`, `paladin-ai-core` is `crates/paladin-core`). | PASS |
| 35 | Empty-section re-measure: per file, non-blank lines between `## [0.11.0]` and the next `## [` (EMPTY decision) | Non-blank line counts: root 575, `paladin-battalion` 93, `paladin-core` 137, `paladin-eval` 4, `paladin-herald` 25, `paladin-llm` 301, `paladin-ports` 143, `paladin-storage` 88, `paladin-web` 45; **0 for `paladin-content`, `paladin-memory` and `paladin-notifications`** (E-2). EMPTY=a: under each of those three headings, one blank line then "No functional change; version bumped in lockstep with the workspace." (file layout: heading, blank, that line, blank, `## [0.10.1]`). | HANDLED (3 crates named) |
| 36 | R7=a: delete the `## [Unreleased]` line and the one blank line after it in each of the 12 changelogs, then per file `grep -c '^## \[Unreleased\]'`, `grep -c '^## \[0\.11\.0\] - 2026-10-25$'` and the diff's highest touched pre-image line against the `## [0.10.1]` heading line | All 12 files: `[Unreleased]` count **0**, `[0.11.0] - 2026-10-25` count **1**, the first `## [` heading in the file is `## [0.11.0] - 2026-10-25`. Highest pre-image line touched is 8 in every file, far above each `## [0.10.1]` heading (root 659, battalion 119, content 10, core 164, eval 16, herald 41, llm 328, memory 10, notifications 10, ports 174, storage 113, web 63): nothing at or below a previous release heading changed. | PASS |
| 37 | `make check-gates` and `./scripts/check-release-consistency.sh --tag v0.11.0` (working tree after the stamp) | Both **exit 0**. `OK: 12 publishable package(s) checked, all match tag version '0.11.0' with a changelog section for it.` Clause 3 (the CI conclusion, D-10) is **not checked locally**: the script printed `NOTE: the CI-conclusion clause (D-10) was not checked -- no --sha was supplied and this is not a GitHub Actions run`; it is decided by `release.yml` on the merge commit (RESEARCH Pitfall 7). | PASS (clause 3 not checked) |
| 38 | B-1 boundary rehearsal, offline: `./scripts/extract-changelog-section.sh --changelog CHANGELOG.md --version 0.11.0 --output <scratch>/body.md`; `wc -c`; then in a subshell `CREATE_OR_REUSE_RELEASE_LIB_ONLY=1`, `. ./scripts/create-or-reuse-release.sh`, `_cor_build_payload v0.11.0 "Release v0.11.0" <scratch>/body.md false`, then `jq -e '(.body\|length) <= 125000'` | Extract exit 0; body file **108,926 bytes** (limit 110,000; equals the 47-02 condensed body, so the stamp added nothing to it); first line is the B-1 lead paragraph. `_cor_build_payload` exit 0; payload keys `body`, `draft`, `name`, `prerelease`, `tag_name`; `tag_name` `v0.11.0`, `draft` false; payload `.body` length **108,824 characters** (cap 125,000, a research figure, RESEARCH A3); `jq -e` printed `true`, exit 0. No network call was made. Scratch files are under the session scratchpad, not the tree. | PASS |
| 39 | `git commit` of the 12 changelogs, then `git diff --name-only HEAD~1 HEAD` | Commit `3108297339ccf7e74e75f85ed5f64400aadd710b` `chore(release): stamp 0.11.0 changelogs (2026-10-25)`; its file list is exactly the 12 changelogs (root plus 11 `crates/*/CHANGELOG.md`). Not pushed. | PASS |
| 40 | Capacity before the first post-bump compile (SPACE=a): `df -h /` then `du -sh target` (SHA `fe5d0736`) | `/dev/vda 252G size, 37G used, 2.5G avail, 94% use`; `target` **20G**. Every artifact under `target/` was built at the old workspace identity, so none is reused. | MEASURED |
| 41 | `cargo clean` (the SPACE=a clean; SHA `fe5d0736`) | `Removed 29214 files, 19.8GiB total`. `df -h /` after: `18G used, 22G avail, 45% use`; `target/` no longer exists. | CLEANED (2.5G to 22G free) |
| 42 | `make openapi` (= `UPDATE_OPENAPI=1 cargo test -p paladin-web --lib openapi_matches_committed_baseline -- --quiet`; first compile after the clean; SHA `fe5d0736`) | Exit 0, real 1m2.7s: `test result: ok. 1 passed`, `Wrote crates/paladin-web/openapi.json`. `df -h /` after: `19G used, 21G avail, 48% use`. **No capacity event**: no compile aborted on out-of-space, so no second clean or re-run was needed. | PASS |
| 43 | `git status --porcelain` and `git diff --numstat -- crates/paladin-web/openapi.json` after row 42 | Only ` M crates/paladin-web/openapi.json`; numstat `1  1`; the diff is the single hunk `-    "version": "0.10.1"` / `+    "version": "0.11.0"` at line 14. No route or schema moved after the Phase 46 freeze (D-07). | PASS (version line only) |
| 44 | `env -u UPDATE_OPENAPI cargo test -p paladin-web --lib openapi_matches_committed_baseline` (compares the generated document with the regenerated file) | `test openapi::tests::openapi_matches_committed_baseline ... ok`, `1 passed; 0 failed`; the tree is unchanged by the test. | PASS |
| 45 | `git commit` of `crates/paladin-web/openapi.json`, then `git diff --name-only HEAD~1 HEAD` | Commit `c945684647c4f9c78fdde2fbe2fc8fc2e8806106` `chore(release): regenerate OpenAPI baseline for 0.11.0`; its file list is `crates/paladin-web/openapi.json` only. This is `SHA-H`, the candidate final code head. Not pushed. | PASS |
| 46 | Build-wave summary: `git diff --name-only 680cf9c294dc22de02c1faae19afc45188470a35 c945684647c4f9c78fdde2fbe2fc8fc2e8806106 -- . ':!.planning' \| sort` against `(ls Cargo.toml crates/*/Cargo.toml CHANGELOG.md crates/*/CHANGELOG.md; echo Cargo.lock; echo crates/paladin-web/openapi.json) \| sort` | **Identical, 27 paths**: the 13 manifests (root plus 12 crates), `Cargo.lock`, the 12 changelogs, `crates/paladin-web/openapi.json`. The three build-wave `chore(release)` commits: bump `1a77f7bd39d8c6d3f0cbc3f25ff900ec03e15efb`, stamp `3108297339ccf7e74e75f85ed5f64400aadd710b`, OpenAPI `c945684647c4f9c78fdde2fbe2fc8fc2e8806106`. The 47-02 remedy commits (all before `SHA-PRE-BUMP`, so outside the build-wave diff): `0e1f7958976b84ac92cac7cd0a965b44c1359463` (`fix(deps)` tokio 1.53.3), `adf15d799bd835ea3a0a8792eae8e43e3b343d39` and `677b6ea3655eb11229ed44b19e334b9f12e1ec08` (B-1 condensation, root `CHANGELOG.md`). | PASS (T-47-12) |
| 47 | Re-seal head (plan 47-04 Task 1; D-00j applies from here): `git rev-parse HEAD`, `git status --porcelain`, then `git diff --name-only c945684647c4f9c78fdde2fbe2fc8fc2e8806106 d82d67cac926412a21cccd72278fe8fe9df672c0 -- . ':!.planning' ':!.project'` (read 2026-10-25T19:16Z) | `SHA-R` = `d82d67cac926412a21cccd72278fe8fe9df672c0` (branch `claude/laughing-dirac-e0h2ax`); `git status --porcelain` printed nothing; the restricted diff from `SHA-H` printed nothing (0 paths). The unrestricted diff `SHA-H..SHA-R` is 4 files under `.planning/` (47-03 summary, evidence, state, roadmap). `df -h /` before the re-seal: `252G size, 19G used, 21G avail, 48%`. | CLEAN; `SHA-R` differs from `SHA-H` only under `.planning/` |
| 48 | Plan 47-04 Task 1 precondition (D-02): `gh api repos/DF3NDR/paladin-dev-env/branches/main --jq .commit.sha` (SHA-R; read 2026-10-25T19:15Z) | `df24689c3d1fd52b0e730105c15b22c2c4ad1404`, equal to the D-02 baseline; `main` has not moved. | UNMOVED |
| 49 | Re-seal row 1a: `grep -c TBD MIGRATION.md` (SHA-R) | Prints `0` (grep exit 1, the no-match status). | `COUNT=0` |
| 50 | Re-seal row 1b: `make check-migration-allowlist` (SHA-R; output in the scratchpad) | **Exit 0**. The script reads `## 9.2` and `## 10.2` together: the MIGRATION side printed **56** `crate \| type` pairs and the allowlist side **56**; the closing lines read `Allowlist is set-equal to the MIGRATION.md §9.2 and §10.2 deliberate-breaking registers (crate\|type pairs).` and `Every crate-wide cargo-semver-checks allow is mirrored by an allowlist entry (crate\|lint pairs).` Per section, re-derived read-only with the script's own awk: **15** deliberate-breaking pairs in section 9.2, **47** in section 10.2, 6 present in both, so the deduplicated union is 15 + 47 - 6 = **56**, equal to the allowlist's 56. | PASS (56 = 56, set-equal) |
| 51 | Re-seal row 7a: `make check-gates` (SHA-R; the nine guards of the `check-gates` prerequisite list) | **Exit 0**. One OK line per guard, verbatim in substance: `check-changelogs` "11 publishable crate(s) checked, all have a CHANGELOG.md."; `check-crate-names` "12 publishable crate(s) checked, all match the allow-list exactly."; `check-advisory-register` "15 register row(s) checked against 15 deny.toml and 8 .cargo/audit.toml ignore entries; all clauses satisfied."; `check-workflow-suppressions` "7 workflow file(s) scanned, 174 run step(s) examined, 1 cargo audit invocation(s) found; no inline advisory-ignore suppression detected."; `check-workflow-triggers` "7 workflow file(s) scanned, 7 policy-table row(s) read; coverage, drift, context and reachability clauses all pass."; `check-codeql-dismissals` "6 governed dismissal(s) checked; schema, drift, staleness, uniqueness and reachability clauses all pass."; `check-migration-allowlist` the two set-equal lines of row 50; `check-publish-order` "OK: 12 crate(s) checked in publish order; every versioned workspace dependency (normal, dev and build) resolves to an earlier CRATES position, and CRATES exactly matches the cargo-metadata publishable set."; `check-dependency-allowlist` "paladin-core (14) and paladin-ports (12) [dependencies] match ADR-0015's measured sets." | PASS (9 of 9) |
| 52 | Re-seal row 7b: the R7=a hard-assertion greps, per changelog (root `CHANGELOG.md` plus the 11 `crates/*/CHANGELOG.md`, 12 files; SHA-R): `grep -c '^## \[Unreleased\]'`, `grep -c '^## \[0\.11\.0\] - 2026-10-25$'`, `grep -m1 '^## \['` | All 12 files: `[Unreleased]` count **0**, `[0.11.0] - 2026-10-25` count **1**, the first `## [` heading is `## [0.11.0] - 2026-10-25`. Root `CHANGELOG.md` also: `## [0.11.1]` count 0, `## [0.12.0]` count 0. | PASS (R7=a form) |
| 53 | Re-seal row 7c: B-1 body-size assertion: `./scripts/extract-changelog-section.sh --changelog CHANGELOG.md --version 0.11.0 --output <scratch>/body.md`, then `wc -c` (SHA-R) | Extract **exit 0**; body file **108926 bytes**, at most 110,000 (1074 bytes of margin); identical to the 47-02 condensed body and to row 38. | PASS (108926 <= 110000) |
| 54 | Re-seal row 8: `make check-publish-order` (= `./scripts/check-publish-order.sh`; SHA-R) | **Exit 0**; the full line, verbatim: `✅ OK: 12 crate(s) checked in publish order; every versioned workspace dependency (normal, dev and build) resolves to an earlier CRATES position, and CRATES exactly matches the cargo-metadata publishable set.` | PASS (E-3) |
| 55 | Re-seal row 2 (plan 47-04 Task 2): `df -h /` (`252G size, 19G used, 21G avail, 48%`), `cargo clean` (SPACE=a; `Removed 2697 files, 997.8MiB total`; `22G avail, 45%` after), then `cargo test --features web-server --test v0_9_config_boot` (SHA-R; 2026-10-25T19:17:45Z to 19:19:29Z, real 1m44s) | **Exit 0**. `Running tests/integration/v0_9_config_boot_test.rs`; `running 9 tests`, all `ok` (`graceful_shutdown_defaults_true_by_m_b_02`, `platform_config_defaults_are_inert`, `apply_env_overrides_is_a_no_op_with_no_app_vars`, `v0_9_config_file_loads_at_v0_10`, `v0_10_settings_fields_resolve_to_default`, `v0_9_config_leaves_the_run_surface_unwired`, `dev_ui_route_is_unregistered_404`, `v0_10_route_families_answer_501`, `v0_9_routes_are_mounted`); `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`. Same 9-test count as the v0.10.1 re-seal. `df -h /` after: `21G used, 19G avail, 52%`. No capacity event. | PASS (9 passed, 0 failed) |
| 56 | Re-seal row 3: `cargo test -p paladin-web --test openapi_golden_v0_9`, then the supporting line `cargo test -p paladin-web --lib openapi_matches_committed_baseline` (SHA-R; 2026-10-25T19:19:30Z to 19:20:20Z) | **Exit 0** (both). Golden target: `running 9 tests`, all `ok` (`allowance_429_exception_is_narrowly_scoped`, `openapi_v0_9_paths_match_the_frozen_baseline`, `info_version_is_the_only_normalisation`, `execute_response_exception_is_narrowly_scoped`, `security_schemes_match_the_frozen_baseline`, `ref_closure_schemas_match_the_frozen_baseline`, `phase_42_exception_is_narrowly_scoped`, `ref_closure_is_non_empty_and_fully_resolved`, `v0_9_path_restriction_is_non_empty`); `test result: ok. 9 passed; 0 failed`. The v0.10.1 re-seal read 7 tests on this target: the count is measured here, not carried; it grew after Phase 37.1 (the Phase 42 and allowance exceptions are among the named tests). Lib line: `test openapi::tests::openapi_matches_committed_baseline ... ok`, `1 passed; 0 failed; 294 filtered out`. `git status --porcelain` empty afterwards. | PASS (9 passed, 0 failed; supporting 1 passed) |
| 57 | Re-seal row 4 (plan 47-04 Task 2): derive the list with `cargo metadata --format-version 1 --no-deps --offline \| jq -r '.packages[]\|select(.publish==null)\|.name' \| sort`, remove `paladin-eval`, assert exactly 11 before looping, then per crate `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0` (SHA-R; script in the scratchpad; 2026-10-25T19:20:25Z to 19:28:41Z) | **RED, D-00j halt.** The 11-entry assertion held (`count=11`: `paladin-ai`, `paladin-ai-core`, `paladin-battalion`, `paladin-content`, `paladin-herald`, `paladin-llm`, `paladin-memory`, `paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web`). **Ten of eleven exit 0** with the known no-op shape, each `Checking <pkg> v0.9.0 -> v0.11.0 (major change)`, `Checked [ 0.000s] 0 checks: 0 pass, 254 skip`, `Summary no semver update required`. **`paladin-web` exited 101** with `error: running cargo-doc on crate 'paladin-web' failed`: the baseline v0.9.0 build script of `utoipa-swagger-ui v9.0.2` could not fetch its Swagger UI archive (verbatim excerpt and diagnosis in Findings, entry "Re-seal row 4 red"). No semver verdict was produced for `paladin-web`; nothing was fixed and nothing was re-run. `df -h /` after: `27G used, 13G avail, 68%`. | **RED** (10 of 11 no-op; `paladin-web` errored, environment-side) |
| 58 | Maintainer decision on re-seal row 4 (plan 47-04 continuation; no command run; SHA-R `d82d67cac926412a21cccd72278fe8fe9df672c0`) | Reply received 2026-10-25T19:45:29Z (UTC), verbatim: `1`. Meaning (the checkpoint's option 1): the `paladin-web` line of row 4 is attributed to its CI twin `Semver Checks (vs v0.9.0)` (unrestricted egress) and recorded plainly as **NOT RUN LOCALLY**, with the measured reason (row 57: the 0.9.0 baseline's `utoipa-swagger-ui 9.0.2` build script downloads from `github.com`, which this sandbox's egress proxy answers with a deterministic HTTP 403 policy denial). No local accommodation (`SWAGGER_UI_DOWNLOAD_URL` override, vendored zip, feature flag) is authorised and **no re-run of `paladin-web` semver-checks was made in this sandbox**. Row 4's other 10 crates stay as measured in row 57 (exit 0, no-op shape). The CI-twin conclusion is filled from the PR run in plan 47-06, like row 5's. Row 57's RED verdict is not edited (append-only, D-00d): this row supersedes it for the purpose of the audit. | 10 of 11 measured PASS; `paladin-web` NOT RUN LOCALLY, CI-attributed (maintainer decision) |
| 59 | Re-seal row 5 (plan 47-04 Task 2): MSRV `env RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets` was **not run** (SHA-R). `ls ~/.rustup/toolchains` lists `1.97.1-x86_64-unknown-linux-gnu`, `nightly-2026-09-20-x86_64-unknown-linux-gnu`, `stable-x86_64-unknown-linux-gnu`; no `1.88` toolchain | No output exists because no command ran; installing 1.88 would need a full extra build tree and the plan says not to. Row 5 is **not run locally -- CI-attributed**; its twin is the CI `MSRV (Rust 1.88)` job, read from the PR run in plan 47-06. No local pass is claimed. | NOT RUN LOCALLY (CI-attributed) |
| 60 | Re-seal row 6 (plan 47-04 Task 2): SPACE=a: `df -h /` before (`252G size, 27G used, 13G avail, 68%`), `cargo clean` (`Removed 25948 files, 10.4GiB total`; `22G avail, 45%` after), then `make publish-dry-run` (= `release-check` then `cargo publish --workspace --dry-run`; working-tree HEAD `c3aa399f8487cc789f4d802b17090a891bf58174`, whose tree outside `.planning/` equals SHA-R `d82d67cac926412a21cccd72278fe8fe9df672c0`; 2026-10-25T19:47:44Z to 19:55:33Z, 7m49s; full output in the scratchpad `row6.log`, 3762 lines) | **RED, D-00j halt. Exit status 2** (`make: *** [Makefile:621: release-check] Error 2`, from `make[1]: *** [Makefile:130: test] Error 101`). Stages that completed green inside `release-check`'s `clean-code` prerequisite: formatting, clippy (warnings as errors), `shellcheck clean`, check, `doc-check passed (ADR-0033)` (three steps, including its doc tests, 13 `test result: ok` lines), then the `# Examples` headings gate. The stage that failed is `make test` = `cargo test --workspace --lib --bins`, on `paladin-ai --lib`: `test result: FAILED. 1457 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.83s`. The two failures, verbatim: `infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default` and `infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set`, each `panicked at src/infrastructure/web/run_api_wiring.rs:2369:13: the run never reached a terminal status` (the test polls the run repository every 100 ms for at most 10 s; callers at lines 2423 and 2408). Closing lines verbatim: `error: test failed, to rerun pass `-p paladin-ai --lib``, `make[1]: Leaving directory '/home/user/paladin-dev-env'`, `EXIT=2`. `cargo test` stops at the first failing target, so the `--lib --bins` targets after `paladin-ai --lib` were not run, and `cargo test --workspace --doc`, `make audit`, `make build-release` and `cargo publish --workspace --dry-run` never started: **no test tally for the workspace, no `cargo audit` warning count and no twelve-crate dry-run order exist for this row.** `df -h /` after: `28G used, 12G avail, 70%`; the host has 4 cores, load average 1.59 (1 min) at 19:56Z. **Not a capacity event** (no out-of-space abort; the SPACE one-re-run allowance does not apply) and not a recognised infrastructure failure, so no re-run was made and nothing was fixed (D-00j). Whether the two tests are timing-sensitive under this sandbox's load or a real defect in the tree that ships is **not determined** and is not guessed here. | **RED** (`make test` 2 failed in `paladin-ai --lib`; rows after the failing stage not run) |
| 61 | Maintainer decision on re-seal row 6 and its gate (plan 47-04 second continuation; no command run to obtain the decision). Decision, received 2026-10-25 between 20:00Z and 20:04Z (UTC), verbatim: `agree to your recommendation to option one, gated on the CI result, falling back to 4 if that result is red.` Option 1 as presented: record the `make test` leg of row 6 as NOT RUN LOCALLY (RED in this sandbox), attributed to its CI twin `Unit Tests (stable)`, with (a) WINDOWS.md row 73 as precedent (open since 2026-10-15: the same two tests fail identically in this sandbox, reproduced at the pre-Phase-50 commit `28b6a69b`, "pre-existing and sandbox-specific; CI on the pushed branch is the proof of record") and (b) the CI run as proof of record; then run the legs that never executed directly. **Gate, as observed by the orchestrator 2026-10-25T20:19:42Z (read-only `gh api`):** `ci.yml` push run 40228639490 on head `d900b21b` (tree outside `.planning/` identical to SHA-H), job `Unit Tests (stable)` id 121275766339: `completed`, `success`; also `success` on that run: `License & Dependency Policy`, `Semver Checks (vs v0.9.0)`, `MSRV (Rust 1.88)`. **Re-read by this agent at 2026-10-25T20:21:33Z** with `gh api repos/DF3NDR/paladin-dev-env/actions/runs/40228639490/jobs?per_page=100`: job 121275766339 `completed` / `success` at `2026-10-25T20:18:33Z`; its steps via `gh api repos/DF3NDR/paladin-dev-env/actions/jobs/121275766339`: 5 `Run unit tests` success, 6 `Run doc tests` success (`ci.yml` lines 540-544 run exactly `cargo test --workspace --lib --bins` and `cargo test --workspace --doc`, the two cargo commands of row 6's `test` and doc-test legs). The run itself was still `in_progress` (jobs `Unit Tests (beta)`, `Integration Tests`, `Coverage`, `Code Quality`, `Docker Integration Tests`, `Benchmark Compile Check` and `Redis Cadence Contract Suite` not yet concluded). **Per-test log read failed, recorded as a failed read:** `gh run view 40228639490 --repo DF3NDR/paladin-dev-env --job 121275766339 --log` exits 1 with `run 40228639490 is still in progress; logs will be available when it is complete`, and `gh api .../actions/jobs/121275766339/logs` is redirected to `productionresultssa5.blob.core.windows.net`, which this sandbox's proxy denies; so the two test names could not be grepped in the CI log and the CI twin is cited on the job and step conclusions only. **Gate met on the job conclusion; the fallback to option 4 does not trigger.** The local sandbox result of row 60 stays RED and is never presented as a pass. | decision recorded; **gate met (CI job `Unit Tests (stable)` success)**; local test leg stays RED |
| 62 | Re-seal row 6, leg 2 (plan 47-04 Task 2 second continuation): `cargo test --workspace --doc` (the `release-check` line after `make test`; working-tree HEAD `d900b21b73ba5a06ab9a57c43d6e66d3d4f792cb`, tree outside `.planning/` equal to SHA-R; `df -h /` before `252G size, 28G used, 12G avail, 70%`; 2026-10-25T20:22:12Z to 20:22:37Z; full output in the scratchpad `leg1-doc.log`, 981 lines) | **Exit 0.** 13 `Doc-tests` targets (`paladin`, `paladin_core`, `paladin_battalion`, `paladin_content`, `paladin_doc_examples`, `paladin_eval`, `paladin_herald`, `paladin_llm`, `paladin_memory`, `paladin_notifications`, `paladin_ports`, `paladin_storage`, `paladin_web`), 13 `test result: ok.` lines, 0 failed; tally summed from those lines: **681 passed; 0 failed; 210 ignored**. Closing lines verbatim: `all doctests ran in 0.43s; merged doctests compilation took 0.43s`, `EXIT=0`. Run directly rather than through `make publish-dry-run` because the chain stopped at `make test` (row 60). | PASS (doc-test leg; this does not make `make publish-dry-run` as a whole a pass) |
| 63 | Re-seal row 6, leg 3: `make audit` (= `cargo audit` after the advisory-db self-heal line; 2026-10-25T20:22:44Z to 20:22:46Z; scratchpad `leg2-audit.log`) | **Exit 0.** `Loaded 1352 security advisories (from /root/.cargo/advisory-db)`, `Scanning Cargo.lock for vulnerabilities (691 crate dependencies)`, final line verbatim `warning: 9 allowed warnings found`. The nine, from the report: `dotenv 0.15.0` RUSTSEC-2021-0141, `fxhash 0.2.1` RUSTSEC-2025-0057, `number_prefix 0.4.0` RUSTSEC-2025-0119, `paste 1.0.15` RUSTSEC-2024-0436, `rustls-pemfile 2.2.0` RUSTSEC-2025-0134, `smartstring 1.0.1` RUSTSEC-2026-0249, `unsafe-libyaml 0.2.11` RUSTSEC-2024-0449 (all unmaintained), `event-listener 5.4.1` RUSTSEC-2026-0221 and `scc 2.4.0` RUSTSEC-2026-0205 (both unsound). Pre-bump row 7 read 10 allowed warnings; the one fewer is RUSTSEC-2026-0386, cleared by the B2=a `tokio` 1.53.3 update. | PASS (exit 0, 9 allowed warnings, 0 vulnerabilities) |
| 64 | Re-seal row 6, leg 4: `make build-release` (= `cargo build --release --workspace`). SPACE=a headroom judgement: `df -h /` read `252G size, 28G used, 12G avail, 70%` with `target` at 9.8G of debug artifacts from legs 1-3, so a pre-emptive `cargo clean` was made (SPACE=a; not an out-of-space abort): `Removed 24983 files, 9.9GiB total`, `df -h /` after `18G used, 22G avail, 45%`. Build 2026-10-25T20:23:02Z to 20:27:17Z (clean included); scratchpad `leg3-build.log` | **Exit 0.** Last lines verbatim: `Compiling paladin-notifications v0.11.0 (/home/user/paladin-dev-env/crates/paladin-notifications)`, `Finished `release` profile [optimized] target(s) in 4m 14s`, `EXIT=0`. `df -h /` after: `19G used, 21G avail, 49%` (`target` 1.5G). | PASS |
| 65 | Re-seal row 6, leg 5: `cargo publish --workspace --dry-run` (the `publish-dry-run` target's own command after `release-check`; `df -h /` before `19G used, 21G avail, 49%`, after `24G used, 16G avail, 62%`; 2026-10-25T20:49:45Z to 20:53:52Z; scratchpad `leg4-publish.log`, 796 lines) | **Exit 0.** 12 `Packaging`, 12 `Verifying` (each followed by a green `Finished `dev` profile` build) and 12 `Uploading ... warning: aborting upload due to dry run` pairs; no error. **Twelve-crate order as emitted, `Packaging` pass:** paladin-ai-core, paladin-ports, paladin-battalion, paladin-llm, paladin-content, paladin-storage, paladin-eval, paladin-herald, paladin-memory, paladin-notifications, paladin-web, paladin-ai (all `v0.11.0`). **`Uploading` (dry-run upload) order as emitted:** paladin-ai-core, paladin-ports, paladin-battalion, paladin-herald, paladin-llm, paladin-notifications, paladin-storage, paladin-web, paladin-content, paladin-eval, paladin-memory, paladin-ai. Both are valid topological orders chosen by cargo; neither equals the `CRATES` array of `scripts/publish-crates.sh` (paladin-ai-core, paladin-ports, paladin-herald, paladin-battalion, paladin-llm, paladin-memory, paladin-web, paladin-notifications, paladin-content, paladin-storage, paladin-eval, paladin-ai), whose own order is the one row 8 (`check-publish-order`) proves: every versioned workspace dependency resolves to an earlier `CRATES` position and `CRATES` equals the cargo-metadata publishable set. `paladin-ai` is last in all three. | PASS (exit 0, twelve crates) |
| 66 | Task 3 `make security` (= `make audit` then `make deny`; working-tree HEAD `72af77e2`, tree outside `.planning/` equal to SHA-R; 2026-10-25T20:55:04Z to 20:55:07Z; scratchpad `sec.log`, 9027 lines) | **Exit 0.** `cargo audit` leg: `Loaded 1352 security advisories`, `Scanning Cargo.lock for vulnerabilities (691 crate dependencies)`, `warning: 9 allowed warnings found` (the same nine as row 63; no vulnerability). `cargo deny check` leg final line verbatim: `advisories ok, bans ok, licenses ok, sources ok`; it prints non-fatal `warning[duplicate]` (e.g. `thiserror`, `hashbrown`, `windows-sys`, `rand`, `rand_core`, `getrandom`) and `warning[wildcard]` (`paladin-battalion` 2, `paladin-llm` 1) lines that `deny.toml` does not escalate. Clears B-2: the pre-bump `cargo deny check advisories` failure on RUSTSEC-2026-0386 (row 7) no longer occurs after the `tokio` 1.53.3 update. | PASS |
| 67 | Task 3 `make test-shell-guards` (same HEAD; 2026-10-25T20:55:07Z to 20:55:33Z; scratchpad `sg.log`, 390 lines) | **Exit 0.** 14 guard-script regression harnesses under `tests/scripts/`, each ending `N assertion(s) passed` (11, 15, 16, 9, 32, 21, 11, 21, 16, 15, 25, 84, 17, 54 = **347 assertions**, 347 `PASS:` lines, 0 `FAIL`): `check-codeql-dismissals`, `check-dependency-allowlist`, `check-migration-allowlist`, `check-publish-order`, `check-release-consistency`, `check-workflow-suppressions`, `check-workflow-triggers`, `create-or-reuse-release`, `extract-changelog-section`, `extract-public-api`, `finalize-crate-changelogs`, `finalize-release-body`, `package-release-binaries`, `publish-crates`. Closing line `PASS (no mutation): git status --porcelain -- scripts .github/workflows is unchanged`. | PASS |
| 68 | Task 3 house-sweep bookkeeping: what row 6's `release-check` already ran at SHA-R (read from rows 60 and 62-64, no new command) | `make clean-code` (formatting, clippy with warnings as errors, `shellcheck clean`, check, `doc-check passed (ADR-0033)`, the `# Examples` headings gate) ran green in the first row-6 run on HEAD `c3aa399f` (tree outside `.planning/` equal to SHA-R), row 60. `cargo test --workspace --doc` (row 62), `cargo audit` (row 63) and the release build (row 64) ran green directly on HEAD `d900b21b`. The workspace test leg is the exception: `make test` is RED locally and CI-attributed (rows 60-61). `make api-surface`, `make check-consumer-surface` and the `Coverage` job are not run here; they are CI-evidence-only entries (below, D-12). | house sweep: green except the CI-attributed test leg |

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

### Plan 47-05 — push of SHA-A0 and the PR #59 retitle (2026-10-25)

**D-02 and precondition checks (read 2026-10-25T21:06Z, before any write):** `gh api repos/DF3NDR/paladin-dev-env/branches/main --jq .commit.sha` printed `df24689c3d1fd52b0e730105c15b22c2c4ad1404` (unmoved); `git status --porcelain` printed nothing; `git branch --show-current` printed `claude/laughing-dirac-e0h2ax`; `git ls-remote --tags origin 'refs/tags/v0.11.0*'` printed nothing.

**SHA-A0 is `d4e66b47628559b175f1958b4f5267058096dd7f`** (`git rev-parse HEAD`, `docs(47-04): complete the re-seal and sign-off section plan`). Its tree outside `.planning/` is byte-identical to `SHA-H` `c945684647c4f9c78fdde2fbe2fc8fc2e8806106` and to `SHA-R` `d82d67cac926412a21cccd72278fe8fe9df672c0`: `git diff --name-only <SHA-H> d4e66b47 -- . ':!.planning'` and the same against `<SHA-R>` both printed nothing.

**The push, stated as measured.** SHA-A0 reached `origin` before this plan began: the orchestrator's post-wave-4 push, about 2026-10-25T21:04Z, put it there (the `push` runs below were created at 21:04:25Z, the `pull_request` runs at 21:04:30Z). This plan then ran the plan's explicit `git push origin claude/laughing-dirac-e0h2ax` at 2026-10-25T21:06:49Z (plain, no force flag, no hooks installed in `.git/hooks`); output verbatim: `Everything up-to-date`, exit 0. No empty commit and no commit outside the plan was made to force a push. After it, `git ls-remote origin refs/heads/claude/laughing-dirac-e0h2ax` printed `d4e66b47628559b175f1958b4f5267058096dd7f` and `GET /pulls/59 .head.sha` printed the same 40 hex characters: **PR head equals SHA-A0**.

**PR #59 before the write (read 21:06Z):** `title` "feat(43-02): honour the provider retry delay as a minimum and cap the gate at max_wait", `body` 459 characters, `commits` 1116, `changed_files` 1109, `mergeable_state` `blocked`, `auto_merge` null, `base` `main` at `df24689c3d1fd52b0e730105c15b22c2c4ad1404`.

**The REST write (D-03).** `gh api -X PATCH repos/DF3NDR/paladin-dev-env/pulls/59 -f title='release: v0.11.0 — Treasurer Spend Governance' -F body=@<scratchpad file>` at 2026-10-25T21:07:17Z; exit 0, HTTP success (no refusal, so no maintainer fallback command is needed). Response: `{"title":"release: v0.11.0 — Treasurer Spend Governance","state":"open","head":"d4e66b47628559b175f1958b4f5267058096dd7f","bodylen":4313}`.

**PR #59 read back independently (2026-10-25T21:07:24Z), `GET /pulls/59`:**

| Field | Value |
|---|---|
| `title` | `release: v0.11.0 — Treasurer Spend Governance` |
| `head.sha` | `d4e66b47628559b175f1958b4f5267058096dd7f` (= SHA-A0) |
| `body` length | 4313 characters |
| `commits` | 1116 (per GitHub; the sandbox clone is shallow) |
| `changed_files` | 1109 (per GitHub) |
| `mergeable_state` | `blocked` |
| `state` / `auto_merge` | `open` / `null` |

Body content checked by `grep` on the read-back text: the heading `v0.11.0 — Treasurer Spend Governance`, the eight-row re-seal table carrying the audit's recorded result words (rows 4, 5 and 6 worded as CI-attributed or mixed, not as local passes), the four pointers (`CHANGELOG.md` `[0.11.0]`, `MIGRATION.md` section 10, `47-CI-EVIDENCE.md`, `47-ACCEPTANCE-AUDIT.md`), the twelve derived crate names with the count, `1116 commits and 1109 changed files per GitHub (the sandbox clone is shallow)`, the merge-commit instruction, a statement that CI on the pushed head had not concluded and the sign-off box is unticked, and the session URL line. The body names no model. No tag `v0.11.0*` exists on the remote. Nothing was merged, approved, labelled, closed, commented on or dispatched.

**Attribution-line difference, recorded as a finding (not corrected).** The body sent ended with `🤖 Generated with [Claude Code](https://claude.com/claude-code)` then the session URL line. The body read back from GitHub does not contain the `🤖 Generated with ...` line and ends with the session URL line, a blank line, `---` and `_Generated by [Claude Code](https://claude.ai/code)_`. That footer was not authored by this plan; something between the request and the stored body rewrote the attribution. `diff` of the sent file against the read-back shows exactly those two differences and no other. It was not re-written: a second PATCH would meet the same rewrite, and the substantive body is intact.

**Workflow runs on SHA-A0 at the single read, 2026-10-25T21:06:17Z (re-read 21:07:24Z, unchanged), via `gh api "repos/DF3NDR/paladin-dev-env/actions/runs?head_sha=d4e66b47628559b175f1958b4f5267058096dd7f&per_page=100"`:** 9 runs, none concluded. This is a snapshot, not polled to completion (D-00i). Per-job conclusions and the `coverage` figure are filled by plan 47-06.

| Workflow | Run ID (URL) | Event | Status at read | Conclusion |
|---|---|---|---|---|
| `ci.yml` | [40233738810](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738810) | `push` | `queued` | `(none yet)` |
| `ci.yml` | [40233745953](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233745953) | `pull_request` | `queued` | `(none yet)` |
| `codeql.yml` | [40233738867](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738867) | `push` | `in_progress` | `(none yet)` |
| `codeql.yml` | [40233746019](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233746019) | `pull_request` | `in_progress` | `(none yet)` |
| `pre-commit` | [40233738812](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738812) | `push` | `in_progress` | `(none yet)` |
| `pre-commit` | [40233745937](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233745937) | `pull_request` | `queued` | `(none yet)` |
| `feature-flags.yml` | [40233738811](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738811) | `push` | `queued` | `(none yet)` |
| `feature-flags.yml` | [40233745927](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233745927) | `pull_request` | `queued` | `(none yet)` |
| `Docs` | [40233746031](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233746031) | `pull_request` | `queued` | `(none yet)` |

**The earlier push run on `d900b21b` was cancelled, not failed.** `ci.yml` push run `40228639490` (head `d900b21b73ba5a06ab9a57c43d6e66d3d4f792cb`, created 2026-10-25T19:59:42Z) reads `status: completed`, `conclusion: cancelled`, updated 2026-10-25T21:04:48Z: the workflow's concurrency group (`cancel-in-progress` off `main`) cancelled it when the push of SHA-A0 landed. Its `Unit Tests (stable)`, `Unit Tests (beta)`, `Semver Checks (vs v0.9.0)`, `MSRV (Rust 1.88)` and `License & Dependency Policy` jobs had concluded `success` before the cancellation (plan 47-04 rows cite them); those are mid-plan observations on a different head and are **not** the proof of record. The proof of record is the runs on SHA-A0 above, read once by plan 47-06.

### Plan 47-06 — every run on SHA-A0 (read 2026-10-25T23:10Z)

**Pre-reads (before anything below was recorded).** `git fetch origin`; `git rev-parse origin/claude/laughing-dirac-e0h2ax` printed `d4e66b47628559b175f1958b4f5267058096dd7f` (= SHA-A0); `gh api repos/DF3NDR/paladin-dev-env/branches/main --jq .commit.sha` printed `df24689c3d1fd52b0e730105c15b22c2c4ad1404` (D-02: `main` unmoved); `GET /pulls/59` printed `head.sha` `d4e66b47628559b175f1958b4f5267058096dd7f`, `state` `open`, `mergeable_state` `blocked`. The local branch is three docs commits ahead of `origin` (`2d76e0cc`, `0a4e64e7`, `a8afb720`, all plan 47-05's, unpushed by design).

**Every workflow run on SHA-A0**, once, via `gh api "repos/DF3NDR/paladin-dev-env/actions/runs?head_sha=d4e66b47628559b175f1958b4f5267058096dd7f&per_page=100"`: 9 runs, all `status: completed`, all `run_attempt` 1 (nothing was re-run, re-dispatched or cancelled by this plan; D-00i, D-00j). **7 `success`, 2 `failure`**: the two `failure` rows are both `pre-commit`, and `pre-commit run --all-files` is a **required** context (tally below).

| Workflow | Run ID (URL) | Event | Conclusion | Created / last update (UTC) | Attempt |
|---|---|---|---|---|---|
| `ci.yml` | [40233738810](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738810) | `push` | `success` | 21:04:25 / 22:57:58 | 1 |
| `ci.yml` | [40233745953](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233745953) | `pull_request` | `success` | 21:04:30 / 23:04:55 | 1 |
| `codeql.yml` | [40233738867](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738867) | `push` | `success` | 21:04:25 / 21:17:29 | 1 |
| `codeql.yml` | [40233746019](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233746019) | `pull_request` | `success` | 21:04:30 / 21:21:57 | 1 |
| `pre-commit` | [40233738812](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738812) | `push` | `failure` | 21:04:25 / 21:10:11 | 1 |
| `pre-commit` | [40233745937](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233745937) | `pull_request` | `failure` | 21:04:30 / 21:12:46 | 1 |
| `feature-flags.yml` | [40233738811](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738811) | `push` | `success` | 21:04:25 / 21:16:32 | 1 |
| `feature-flags.yml` | [40233745927](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233745927) | `pull_request` | `success` | 21:04:30 / 21:30:42 | 1 |
| `Docs` | [40233746031](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233746031) | `pull_request` | `success` | 21:04:30 / 21:16:28 | 1 |

`ci.yml` jobs per run, from `gh api repos/DF3NDR/paladin-dev-env/actions/runs/<id>/jobs --paginate` (PR run `40233745953`: 39 jobs, `total_count` 39; push run `40233738810`: 39 jobs, `total_count` 39). Conclusions: PR run 37 `success` and 2 `skipped` (`End-to-End Tests`, `Publish Dry Run`); push run 36 `success` and 3 `skipped` (those two and `Benchmark Regression Signal (Non-Blocking)`); no `failure`, no `cancelled`.

| Job | PR run job id | PR conclusion | push run job id | push conclusion |
|---|---|---|---|---|
| API Surface Tracking | 121290924875 | `success` | 121290851163 | `success` |
| Benchmark Compile Check | 121290925008 | `success` | 121290851675 | `success` |
| Benchmark Regression Signal (Non-Blocking) | 121290924872 | `success` | 121290852880 | `skipped` |
| CLI Snapshot Tests | 121290924795 | `success` | 121290851612 | `success` |
| Code Quality | 121290924938 | `success` | 121290851489 | `success` |
| Consumer-Surface Muster | 121290924739 | `success` | 121290851406 | `success` |
| Coverage | 121290924766 | `success` | 121290851599 | `success` |
| Crate Isolation (paladin-ai) | 121290924916 | `success` | 121290851510 | `success` |
| Crate Isolation (paladin-ai-core) | 121290925049 | `success` | 121290851517 | `success` |
| Crate Isolation (paladin-battalion) | 121290924896 | `success` | 121290851413 | `success` |
| Crate Isolation (paladin-content) | 121290925034 | `success` | 121290851462 | `success` |
| Crate Isolation (paladin-llm) | 121290925058 | `success` | 121290851292 | `success` |
| Crate Isolation (paladin-memory) | 121290924873 | `success` | 121290851426 | `success` |
| Crate Isolation (paladin-notifications) | 121290924936 | `success` | 121290851511 | `success` |
| Crate Isolation (paladin-ports) | 121290924977 | `success` | 121290851354 | `success` |
| Crate Isolation (paladin-storage) | 121290925256 | `success` | 121290851470 | `success` |
| Crate Isolation (paladin-web) | 121290924945 | `success` | 121290851538 | `success` |
| Docker Build | 121297224341 | `success` | 121296447119 | `success` |
| Docker Integration Tests | 121290924772 | `success` | 121290851572 | `success` |
| E2E Platform API (PRD 06 acceptance-1 lifecycle + SHIP-02 boot proof) | 121290924867 | `success` | 121290851325 | `success` |
| End-to-End Tests | 121298528477 | `skipped` | 121296653665 | `skipped` |
| Example Muster (Feature Matrix) | 121297224366 | `success` | 121293745928 | `success` |
| Generated SDK Clients (Python + TypeScript) smoke | 121290925016 | `success` | 121290851366 | `success` |
| Integration Tests | 121290925041 | `success` | 121290851363 | `success` |
| Kubernetes Smoke Test | 121317355941 | `success` | 121316124835 | `success` |
| License & Dependency Policy | 121290924574 | `success` | 121290851549 | `success` |
| MSRV (Rust 1.88) | 121290925039 | `success` | 121290851357 | `success` |
| OSV Scanner | 121290924791 | `success` | 121290851513 | `success` |
| Ollama Integration Tests (live server) | 121290925046 | `success` | 121290851587 | `success` |
| Postgres Storage Contract Suites (live server) | 121290924831 | `success` | 121290851379 | `success` |
| Publish Dry Run | 121297225326 | `skipped` | 121296448144 | `skipped` |
| Redis Cadence Contract Suite (live server) | 121290924941 | `success` | 121290851931 | `success` |
| Redis Node Cache Contract Suite (live server) | 121290924845 | `success` | 121290851542 | `success` |
| Redis Run Queue Contract Suite (live server) | 121290924898 | `success` | 121290851626 | `success` |
| Security Audit | 121290924882 | `success` | 121290851423 | `success` |
| Semver Checks (vs v0.9.0) | 121290924820 | `success` | 121290851454 | `success` |
| Unit Tests (beta) | 121290925014 | `success` | 121290851551 | `success` |
| Unit Tests (stable) | 121290925044 | `success` | 121290851499 | `success` |
| Workflow Lint | 121290924809 | `success` | 121290851516 | `success` |

`End-to-End Tests` and `Publish Dry Run` appear as `skipped` on both events (conditional jobs that do not run off `main`; D-14), named as skipped and not as passed. `Benchmark Regression Signal (Non-Blocking)` is `skipped` on the push run only.

**The other workflows' jobs** (same `runs/<id>/jobs --paginate` read): `codeql.yml` push 40233738867 and pull_request 40233746019 each carry one job, `CodeQL Analysis (Rust)` (ids 121290757056 and 121290778898), both `success`. `feature-flags.yml` push 40233738811 and pull_request 40233745927 each carry 20 jobs (`LLM Registry Unit Tests (llm-all)`, 16 `Build & Test (<feature>)` jobs, `CLI Isolation (library without cli feature)`, `OTel Feature (otel)`, `Feature Matrix Summary`), all 20 `success` in both. `Docs` 40233746031 carries `Build MDBook` (121290778560, `success`) and `Deploy to GitHub Pages` (121293680076, `skipped`). `pre-commit` push 40233738812 and pull_request 40233745937 each carry one job, `pre-commit run --all-files`, **`failure`**:

| Run | Job | Job id (URL) | Job conclusion | Failing step |
|---|---|---|---|---|
| push 40233738812 | `pre-commit run --all-files` | [121290756636](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738812/job/121290756636) | `failure` | step 7 `Run pre-commit` (`failure`; steps 1-6 `success`) |
| pull_request 40233745937 | `pre-commit run --all-files` | [121290778127](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233745937/job/121290778127) | `failure` | step 7 `Run pre-commit` (`failure`; steps 1-6 `success`) |

**License & Dependency Policy steps** (PR run job `121290924574`; the push run job `121290851549` reads identically), every step `success`, the publish-order gate on its own row:

| Step | Conclusion |
|---|---|
| 5 Check per-crate changelogs | `success` |
| 6 Check crates.io package names | `success` |
| 7 Check advisory exception register | `success` |
| 8 Check workflow files for inline advisory suppressions | `success` |
| 9 Check workflow trigger policy | `success` |
| 10 Check CodeQL dismissal register | `success` |
| **11 Check publish order** | **`success`** (PR run job 121290924574 and push run job 121290851549; the gate executed pre-merge on this head, re-seal row 8's CI twin; the step's printed line is not readable here, the log host being blocked, so the proof is the step's own conclusion) |
| 12 Check core/ports dependency allowlist (ADR-0015) | `success` |
| 13 Run shell-guard regression tests | `success` |
| 16 Run cargo-deny check | `success` |

**Required-context tally, derived from the live ruleset, not assumed.** `gh api repos/DF3NDR/paladin-dev-env/rulesets/20868126` (`Protect main branch`, `enforcement` `active`, `strict_required_status_checks_policy` `true`) lists **44** required contexts (counted from the JSON, not carried from any document). The check runs on SHA-A0 were read with `gh api "repos/DF3NDR/paladin-dev-env/commits/d4e66b47628559b175f1958b4f5267058096dd7f/check-runs?per_page=100&page=<n>"` (`total_count` 128, read in two pages because `--paginate` follows a `repositories/<id>` link this proxy refuses) and every required name was matched against them:

- **Absent from the runs: none.** All 44 required names appear (43 of them twice, once per event; `Build MDBook` once, the `Docs` workflow being pull-request-only).
- **42 concluded `success`** on every row they have.
- **1 concluded `skipped`: `End-to-End Tests`** (both rows). Recorded as skipped, not as a pass: the conditional job did not execute; it is the same shape as the v0.10.1 record, and a skipped required job satisfies the ruleset.
- **1 concluded `failure`: `pre-commit run --all-files`** (both rows: check runs 121290756636 on the push event and 121290778127 on the pull_request event). **A required context is red on SHA-A0.** D-00j applies; see the Findings entry "Plan 47-06: pre-commit red on SHA-A0".
- `MSRV (Rust 1.88)`, `Semver Checks (vs v0.9.0)`, `Consumer-Surface Muster`, `E2E Platform API ...` and `Docker Build` are not among the 44 and are all `success` here. The advisory `CodeQL` results check (app `github-advanced-security`, check run 121293439567) is `failure`, also not required; see the Findings entry "Plan 47-06: the advisory `CodeQL` results check".

**`Coverage` job.** PR run job 121290924766: `success` (completed 21:30:47Z; steps `Measure coverage`, `Coverage summary`, `Upload coverage summary`, `Upload coverage to Codecov` all `success`). Push run job 121290851599: `success` (completed 21:34:35Z, same steps). The `Measure coverage` step runs `scripts/coverage.sh`, whose `cargo llvm-cov` call carries `--fail-under-lines "$FLOOR"` with `FLOOR="${COVERAGE_FLOOR:-82}"` (`scripts/coverage.sh` lines 35 and 102), so a `success` conclusion means the measured line coverage was at or above 82 percent **if** `COVERAGE_FLOOR` was unset; that inference is not a figure. **The `Lines:` figure is unreadable from this sandbox.** Sources tried, each by one read, nothing re-run: (1) the job log, `gh api repos/DF3NDR/paladin-dev-env/actions/jobs/121290924766/logs`: redirected to the Azure blob host `productionresultssa12.blob.core.windows.net`, answered `Forbidden`; (2) the `coverage-summary` artifact, PR run artifact 12615295044 (1122584 bytes, not expired) and push run artifact 12615515178: `gh api repos/DF3NDR/paladin-dev-env/actions/artifacts/12615295044/zip` redirected to the same blob host and failed (empty file); (3) the check-run output of the `Coverage` job: `title`, `summary` and `text` all `null`, and its two annotations are the Node.js 20 deprecation warning and the `ubuntu-latest` migration notice, no figure; (4) commit statuses on SHA-A0: `total_count` 0 (no Codecov status); (5) PR #59 comments: one, from `socket-security[bot]`, no coverage figure. **No source was readable. Figure unreadable from this sandbox (blob host blocked); the job concluded `success` under `--fail-under-lines`; figure requested from the maintainer at the tick checkpoint (the `Lines:` line of the `Coverage` summary, job 121290924766).** No figure is expected, carried forward or inferred here (D-00e).

**Resulting position.** Seven of the nine runs and 42 of 44 required contexts are green, one required context skipped by design, and **one required context (`pre-commit run --all-files`) is red on the candidate head**. Per the plan's step 6 and D-00j, the plan stops at a blocking checkpoint here: nothing is fixed, nothing is re-run, and nothing is pushed. This record and the audit's CI-twin pointers are committed locally only.

---

### CI-evidence-only entries (D-12)

Three gates are not signed rows of the audit and have no local twin in this sandbox. Each is **to be read from the PR run on SHA-A0 in plan 47-06**; nothing is recorded here as measured.

- **`API Surface Tracking`** (`make api-surface`; job `API Surface Tracking`): expected signal is that the committed API-surface baseline does not move after the version bump (the bump commit changes no public item; MIGRATION section 10 stays frozen at `e28e43b5`, Findings entry D-00n). The push run 40228639490 on head `d900b21b` shows the job `success` at 2026-10-25T20:09:39Z; that is a mid-plan observation, not the 47-06 reading, and the final reading comes from the run on SHA-A0.
  **Plan 47-06 reading (SHA-A0):** PR run 40233745953 job 121290924875 `success` (21:31:55Z), push run 40233738810 job 121290851163 `success`; steps `Check API surface changes` and `Check deprecation warnings` `success`, `Generate API diff on failure` `skipped`. The committed API-surface baseline did not move.
- **`Consumer-Surface Muster`** (`make check-consumer-surface`, the Phase 49 muster; job `Consumer-Surface Muster`): to be read from the PR run on SHA-A0 in plan 47-06. The push run 40228639490 shows it `success` at 2026-10-25T20:13:33Z (mid-plan observation only).
  **Plan 47-06 reading (SHA-A0):** PR run job 121290924739 `success` (21:19:48Z), push run job 121290851406 `success`; step 5 `Consumer-surface muster (make check-consumer-surface)` `success`.
- **`Coverage`** (the 82 percent workspace line-coverage floor, ADR-0006, D-00e; job `Coverage`): this sandbox has no Docker and cannot run `make coverage`, so no local figure exists and none is expected or carried forward. The figure is the job's own printed `Lines:` summary, read in plan 47-06; at the 20:21Z read the job was `in_progress`.
  **Plan 47-06 reading (SHA-A0):** PR run job 121290924766 `success`, push run job 121290851599 `success`; the printed `Lines:` figure is **unreadable from this sandbox** (job log and `coverage-summary` artifact both sit on the blocked blob host; check-run output, commit statuses and PR comments carry no figure). Requested from the maintainer at the tick checkpoint; see the CI-run subsection "Plan 47-06".

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

## B-1 condensation ledger

Plan 47-02 (B1=a). Source for every row is `git show 700a92d86f3d6b2e24bbd2c163daec7c307297e0:CHANGELOG.md` (`SHA-PRE-B1`), line ranges taken from that file. Chunk ids `A1`, `A2`, ... are `### Added`; `C1`, `C2`, ... are `### Changed`. A chunk is a run of consecutive top-level bullets of about 12,000 source bytes or fewer (one bullet is never split). The re-diff was run after every chunk against the original subsection: (ids) every `M-B-`/`DR-`/`ADR-` id in the chunk's source lines is present in the condensed subsection; (figures) numeric tokens of the condensed subsection, phase-label lines and id tokens removed first, are a subset of the original subsection's; (code spans) backtick spans of the condensed subsection, excluding spans that are exactly a workspace crate name, are a subset of the original's; (untouched) every other subsection is byte-identical. The figure and code-span checks are a mechanical proxy for "no new claim"; see `- B-1 maintainer read:` below.

| Chunk | Subsection | Source lines at SHA-PRE-B1 | Top-level bullets | Source bytes | Phase groups written to | Ids carried | Re-diff |
|-------|------------|----------------------------|-------------------|--------------|-------------------------|-------------|---------|
| A1 | `### Added` | 12-134 | 9 | 10777 | Phase 51 | ADR-0004, ADR-0062, DR-20, DR-21, DR-26 | PASS (ids, figures, code spans; other subsections untouched) |
| A2 | `### Added` | 135-262 | 10 | 11311 | Phase 51, Phase 50.1, Phase 50 | ADR-0053, ADR-0061, DR-1, DR-2, DR-8, DR-9, M-B-05 | PASS (ids, figures, code spans; other subsections untouched) |
| A3 | `### Added` | 263-393 | 4 | 11597 | Phase 50, Phase 49, Phase 43 | ADR-0004, ADR-0058, ADR-0061 | PASS (ids, figures, code spans; other subsections untouched) |
| A4 | `### Added` | 394-503 | 3 | 9353 | Phase 43, Phase 42 | ADR-0050, ADR-0057 | PASS (ids, figures, code spans; other subsections untouched) |
| A5 | `### Added` | 504-561 | 2 | 4961 | Phase 42, Phase 41 | ADR-0056, ADR-0057 | PASS (ids, figures, code spans; other subsections untouched) |
| A6 | `### Added` | 562-698 | 5 | 11836 | Phase 41, Phase 38 | ADR-0052, ADR-0053 | PASS (ids, figures, code spans; other subsections untouched) |
| A7 | `### Added` | 699-838 | 13 | 11846 | Phase 38, Phase 39, Phase 40, Phase 45, Phase 50 | ADR-0054 | PASS (ids, figures, code spans; other subsections untouched) |
| A8 | `### Added` | 839-957 | 9 | 10180 | Phase 50 | ADR-0046, ADR-0061, M-B-16 | PASS (ids, figures, code spans; other subsections untouched) |
| A9 | `### Added` | 958-989 | 2 | 2695 | Phase 50 | ADR-0061 | PASS (ids, figures, code spans; other subsections untouched) |
| C1 | `### Changed` | 992-1129 | 11 | 11448 | Phase 46, Phase 51, Phase 50.1, Phase 50 | DR-21, M-B-11, M-B-20 | PASS (ids, figures, code spans; other subsections untouched) |
| C2 | `### Changed` | 1130-1259 | 11 | 11239 | Phase 50, Phase 48, Phase 43, Phase 42 | ADR-0004, ADR-0060, ADR-0061, M-B-12, M-B-13, M-B-15 | PASS (ids, figures, code spans; other subsections untouched) |
| C3 | `### Changed` | 1260-1396 | 20 | 11741 | Phase 42, Phase 41, Phase 39, Phase 38, Phase 40, Phase 45, Phase 44, Phase 50 | ADR-0055, ADR-0059, M-B-06, M-B-08, M-B-09, M-B-10, M-B-11, M-B-16 | PASS (ids, figures, code spans; other subsections untouched) |
| C4 | `### Changed` | 1397-1419 | 2 | 1978 | Phase 50, Phase 46 | ADR-0015, M-B-17 | PASS (ids, figures, code spans; other subsections untouched) |

Top-level bullet totals: `A1`-`A9` sum to 57 (the original `### Added` has 57); `C1`-`C4` sum to 44 (the original `### Changed` has 44). Condensed top-level bullets: 90 in `Added`, 49 in `Changed` (several source bullets became two or three, because each shipped capability has its own bullet).

**Tightening pass (plan 47-02 Task 3).** After `Changed` was condensed the root body measured 116,534 bytes, over the 110,000 budget, so the condensed bullets of `A2`-`A8` and `C1`-`C3` were shortened (name lists, per-model enumerations and public-surface lists trimmed; the six preserved subsections were not touched) and the re-diff was re-run over every chunk, cumulatively against the original subsection. The `Re-diff` column above is the result of that final run, not of the first pass. The `Added` rewrite is in the same commit as `Changed` (its commit body says so).

**B-1 measurements after condensation (commit `677b6ea3655eb11229ed44b19e334b9f12e1ec08`):**

| Quantity | At `SHA-PRE-B1` | After | Check |
|---|---|---|---|
| root `[Unreleased]` body | 156528 bytes | **108926 bytes** | at most 110000: PASS (1074 bytes of margin) |
| `### Added` (excluding heading) | 84557 | 46648 | smaller, figure and code-span subsets: PASS |
| `### Changed` (excluding heading) | 36407 | 26374 | smaller, figure and code-span subsets: PASS |
| six preserved subsections (`Breaking Changes`, `Security`, `Removed`, `Fixed`, `Known limitations`, `Documentation`) | 35445 | 35445 | `cmp` byte-identical: PASS (all six) |
| lines at or below `## [0.10.1]` | 1852 onward | identical | `diff` empty; highest pre-image line touched by `git diff SHA-PRE-B1` is 1418 (above the heading at 1852): PASS |
| `M-B-`/`DR-`/`ADR-` ids | 43 distinct | 43 distinct | `comm -23` of before against after is empty: PASS |
| `###` headings inside the body | 8 | 8 | none missing: PASS |
| `BREAKING` markers in `Changed` | 4 | 4 | the four struct-literal breaks and their migration lines are kept |

`make check-changelogs`: exit 0 (11 publishable crates, all have a changelog). `make check-gates`: exit 0 after the condensation. The aim of "about 60,000 bytes" for `Added` plus `Changed` was not reached (73,022 bytes, 60 percent of the original 120,964); the hard limit is the 110,000-byte body, which holds, and tightening further would have dropped per-capability detail rather than lists.

- B-1 maintainer read: requested. The figure and code-span checks above are a mechanical proxy: they catch a new number or a new identifier, not a changed meaning. That the condensed `[Unreleased]` adds no claim beyond that proxy is a judgment the maintainer confirms by reading the condensed entry against this ledger (and, where in doubt, against `git show 700a92d86f3d6b2e24bbd2c163daec7c307297e0:CHANGELOG.md`), at the latest at the 47-06 sign-off (47-06 Task 2, instructions step 2). This does not pause plan 47-02.

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

### 2026-10-25 — Maintainer's pre-build decision recorded (plan 47-01 Task 3; D-08)

Reply received 2026-10-25T18:19:21Z (UTC), verbatim:

> DATE=Today
> All (a) choices

Resolved by the orchestrator, one value per item: `B1=a` `B2=a` `R7=a` `EMPTY=a` `SPACE=a` `DATE=2026-10-25`
("Today" is 2026-10-25, the UTC date of the reply). This equals the plan's `recommended` option. Effect on the
findings above: B-1 is cleared by condensing `Added` and `Changed` to at most 110,000 bytes in total body size
(chunk-by-chunk, ledgered); B-2 by `cargo update -p tokio --precise 1.53.3` as its own `fix(deps)` commit before the
bump; R7 by deleting the empty `[Unreleased]` anchor in the stamp commit; EMPTY by one lockstep line under each of
`paladin-content`, `paladin-memory` and `paladin-notifications`; SPACE by `cargo clean` before the first compile and
again before row 6, with one retry on an out-of-space abort; the stamp date equals the tag date (D-00n). No tracked
file outside `.planning/` had changed when the decision was recorded. The authoritative copy is
`47-01-SUMMARY.md` under `## Pre-build decision`.

### 2026-10-25 — Maintainer's pre-build decision (47-01): D-08 authorisation for the B-1 and B-2 remedies (plan 47-02)

This entry is the D-08 authorisation: it is written before any remedy lands. Source: `47-01-SUMMARY.md` under
`## Pre-build decision`, maintainer reply received 2026-10-25T18:19:21Z (UTC), verbatim:

> DATE=Today
> All (a) choices

One line per item, each with its effect in plan 47-02 and after:

- B1=a: condense the root `CHANGELOG.md` `[Unreleased]` `### Added` then `### Changed` chunk by chunk (two commits, one ledger row per chunk) to a body of at most 110,000 bytes; the six other subsections stay byte-identical; every `M-B-`/`DR-`/`ADR-` id is preserved; the full text stays at `SHA-PRE-B1`.
- B2=a: `cargo update -p tokio --precise 1.53.3`, `Cargo.lock` only, as its own `fix(deps)` commit before the bump (Local sweep rows 19-23).
- R7=a: delete the empty `## [Unreleased]` anchor in the stamp commit (plan 47-03); assert `[Unreleased]` count 0 and `[0.11.0]` count 1 per changelog.
- EMPTY=a: one line "No functional change; version bumped in lockstep with the workspace." under each of `paladin-content`, `paladin-memory` and `paladin-notifications` (plan 47-03).
- SPACE=a: `cargo clean` before the first compile and again before row 6; an out-of-space abort is a capacity event, cleaned and re-run once; a second abort goes to the maintainer.
- DATE=2026-10-25: the `## [0.11.0] - <DATE>` stamp equals the tag date (D-00n); merge, the merge-commit `ci.yml` run and the tag land the same calendar day or the date is re-decided.

### 2026-10-25 — Plan 47-03 build wave: bump, stamp and OpenAPI regeneration landed (SHA-H recorded)

- Three isolated `chore(release)` commits on `claude/laughing-dirac-e0h2ax`, nothing pushed, nothing tagged, nothing published: bump `1a77f7bd39d8c6d3f0cbc3f25ff900ec03e15efb` (14 files), stamp `3108297339ccf7e74e75f85ed5f64400aadd710b` (12 files), OpenAPI `c945684647c4f9c78fdde2fbe2fc8fc2e8806106` (1 file). `SHA-PRE-BUMP` is `680cf9c294dc22de02c1faae19afc45188470a35`; `SHA-H` is `c945684647c4f9c78fdde2fbe2fc8fc2e8806106`. Local sweep rows 24-46 hold the proofs. Plan 47-04 re-seals on `SHA-H` (or on a later docs-only head, which the audit must state).
- **Plan text drift (no effect):** the plan says `crates/doc-examples` carries "nine" `paladin-*` pins; the file carries eight (`paladin-ai`, `paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-llm`, `paladin-memory`, `paladin-content`, `paladin-storage`). `cargo release` rewrote all eight and the manifest's own version; no line was hand-touched.
- **EMPTY=a applied** to `paladin-content`, `paladin-memory` and `paladin-notifications`; each `[0.11.0]` section is the single lockstep line. The `paladin-eval` section is four non-blank lines (a real, short entry) and was left as the script produced it.
- **R7=a applied.** Every changelog now opens with `## [0.11.0] - 2026-10-25`; there is no `## [Unreleased]` heading until the next release cycle re-adds one. `make check-changelogs`, `make check-gates` and `check-release-consistency.sh --tag v0.11.0` all exit 0 without it.
- **Not checked locally:** the CI-conclusion clause of `check-release-consistency.sh` (RESEARCH Pitfall 7). The stamp date `2026-10-25` equals the tag date only if the merge, the merge-commit `ci.yml` run and the tag land the same calendar day (D-00n); otherwise the date is re-decided.
- `k8s/server/worker-deployment.yaml` still labels `version: v0.10.0` (finding above); it is outside the declared 27-path set and was not touched (D-08).

### 2026-10-25 — Re-seal row 4 red: the `paladin-web` 0.9.0 baseline cannot be built in this sandbox (plan 47-04, D-00j halt)

**Command (SHA-R `d82d67cac926412a21cccd72278fe8fe9df672c0`, Local sweep row 57):** `cargo semver-checks check-release --package paladin-web --default-features --baseline-version 0.9.0`, the eleventh of the eleven looped checks. **Exit status 101.** The other ten crates exited 0 in the known no-op shape; `paladin-web` produced no semver verdict. Failing output, verbatim (the `Checking ...` progress lines that precede it are elided):

```
=== paladin-web
error: running cargo-doc on crate 'paladin-web' failed with output:
...
error: failed to run custom build command for `utoipa-swagger-ui v9.0.2`

Caused by:
  process didn't exit successfully: `.../target/semver-checks/registry-paladin_web-0_9_0-x86_64_unknown_linux_gnu-ce8d0f4521eb017b/target/debug/build/utoipa-swagger-ui-44c1704b7edf8a1c/build-script-build` (exit status: 101)
  --- stdout
  SWAGGER_UI_DOWNLOAD_URL: https://github.com/swagger-api/swagger-ui/archive/refs/tags/v5.17.14.zip
  cargo:rerun-if-env-changed=SWAGGER_UI_DOWNLOAD_URL
  start download to : ".../utoipa-swagger-ui-e62fed847a7ce16f/out/v5.17.14.zip"
  reqwest feature: Err(NotPresent)
  trying to download using `curl` system package

  --- stderr
  thread 'main' (19784) panicked at /root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/utoipa-swagger-ui-9.0.2/build.rs:219:51:
  failed to open downloaded Swagger UI: InvalidArchive("Could not find EOCD")
error: failed to build rustdoc for crate paladin-web v0.9.0
error: aborting due to failure to build rustdoc for crate paladin-web v0.9.0
EXIT[paladin-web]=101
FAILED_COUNT=1
```

**Diagnosis (read-only; nothing was changed).** The 0.9.0 baseline of `paladin-web`, built from the registry by `cargo-semver-checks`, depends on `utoipa-swagger-ui 9.0.2` **without** the `vendored` feature (the current tree enables it: `crates/paladin-web/Cargo.toml` line 44), so its build script downloads the Swagger UI archive from `github.com` with the system `curl`. In this session the egress proxy answers that URL with HTTP 403 and a 378-byte JSON body, which `curl` saved as `v5.17.14.zip`: the file in the build's `OUT_DIR` is that body (`{"message":"GitHub access to this repository is not enabled for this session. Use add_repo to request access. ...`), hence `Could not find EOCD`. Re-reading the same URL with `curl -sS -L` returned `http=403` with the same body, so the denial is deterministic, not transient. `/root/.ccr/README.md` says a 403 from the proxy is a policy denial and must be reported, not retried or routed around. The same crate's current-tree (0.11.0) build in the same loop **succeeded** (it uses `vendored`), and the plan's own CI twin (`Semver Checks (vs v0.9.0)`) runs on a runner with unrestricted egress. This is an environment-side failure, not a semver result and not a capacity event, so the SPACE one-re-run allowance does not apply; **no re-run was made and no environment variable, vendored zip or `--features` flag was used to get around the denial** (D-00j). The decision whether row 4's `paladin-web` line may be attributed to the CI twin, supplied a locally available copy of the same archive, or run by the maintainer on a machine with GitHub egress is the maintainer's.

**State left behind.** Rows 5 and 6 were not run (the halt precedes them). `df -h /` after the loop: `27G used, 13G avail, 68%`; `target/semver-checks/` holds the per-crate rustdoc build trees. No tracked file outside `.planning/` changed (`git status --porcelain` empty before this entry).

### 2026-10-25 — Re-seal row 4: maintainer decision, `paladin-web` attributed to its CI twin (plan 47-04 continuation)

Reply received 2026-10-25T19:45:29Z (UTC), verbatim:

> 1

This is option 1 of the row-4 checkpoint. Effect, exactly as decided:

- The `paladin-web` line of row 4 is **NOT RUN LOCALLY** and is attributed to the CI job `Semver Checks (vs v0.9.0)` (a runner with unrestricted egress). It is never recorded as a local pass.
- Measured reason (Local sweep row 57, Findings entry "Re-seal row 4 red"): the 0.9.0 baseline's `utoipa-swagger-ui 9.0.2` build script downloads its Swagger UI archive from `github.com`, which this sandbox's egress proxy answers with a deterministic HTTP 403 policy denial (the saved archive is the 378-byte denial body, read as an invalid zip, `Could not find EOCD`).
- No local accommodation was authorised or used: no `SWAGGER_UI_DOWNLOAD_URL` override, no vendored zip, no feature flag; `paladin-web` semver-checks was not run again in this sandbox.
- Row 4's other ten crates stay as measured: exit 0, `0 checks: 0 pass, 254 skip`, `no semver update required`.
- The CI-twin conclusion for `Semver Checks (vs v0.9.0)` is filled from the PR run in plan 47-06, like row 5's `MSRV (Rust 1.88)`.
- Row 57's RED verdict and the entry above it are left as written (append-only, D-00d). The audit row 4 now reads "10 of 11 measured PASS locally; `paladin-web` not run locally" and points at rows 57 and 58. D-00j was honoured: the plan halted with nothing fixed, and resumed only on this decision.

### 2026-10-25 — Re-seal row 6 red: `make publish-dry-run` stops in `make test` on two `run_api_wiring` tests (plan 47-04, D-00j halt)

**Command (working-tree HEAD `c3aa399f8487cc789f4d802b17090a891bf58174`, code-identical to SHA-R `d82d67cac926412a21cccd72278fe8fe9df672c0` outside `.planning/`; Local sweep row 60):** `make publish-dry-run`, after `cargo clean` under SPACE=a. **Exit status 2.** Failing output, verbatim:

```
failures:
    infrastructure::web::run_api_wiring::tests::build_run_api_persists_no_run_traces_by_default
    infrastructure::web::run_api_wiring::tests::build_run_api_persists_run_traces_when_trace_persist_is_set

test result: FAILED. 1457 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 43.83s

error: test failed, to rerun pass `-p paladin-ai --lib`
make[1]: *** [Makefile:130: test] Error 101
make[1]: Leaving directory '/home/user/paladin-dev-env'
make: *** [Makefile:621: release-check] Error 2
```

Each of the two tests panicked at `src/infrastructure/web/run_api_wiring.rs:2369:13` with `the run never reached a terminal status`. Read-only context (nothing was changed): that assertion is the end of a 10-second poll loop (100 ms interval) waiting for a spawned worker to drive a `POST /v1/runs` run to a terminal status in a sqlite-backed run store; the other `run_api_wiring` tests in the same binary passed, including `wired_treasurer_refuses_an_exhausted_key_through_the_run_router`. Whether the failure is a timing bound that this 4-core sandbox misses while 1457 other tests run in the same process, an interaction with the `cargo clean` cold build, or a real defect in code that ships is **undetermined**; this entry does not guess. The two tests are not in the Phase 47 build wave's declared 27-path set (the wave touched manifests, changelogs, `Cargo.lock` and `openapi.json` only), so nothing the build wave changed is a known cause, but that is an observation, not a diagnosis. Note that `Cargo.lock` moved `tokio` to 1.53.3 in plan 47-02 (B2=a) and the panic backtrace shows `tokio-1.53.3`; whether that matters is likewise not established.

**What did not run (so cannot be reported):** the `--lib --bins` targets after `paladin-ai --lib`, `cargo test --workspace --doc`, `make audit` (`cargo audit` warning count), `make build-release`, and `cargo publish --workspace --dry-run` (the twelve-crate order). Row 6 therefore has no PASS evidence of any kind beyond the `clean-code` stage.

**Not a capacity event:** `df -h /` read `12G avail, 70%` after the abort and no step aborted on out-of-space, so the SPACE one-re-run allowance does not apply. **No re-run was made.** Plan 47-04 Task 3 (house sweep and security gate), `47-04-SUMMARY.md` and the STATE and ROADMAP updates were not started: D-00j stops the plan here. The decision on how to proceed (restart the build wave to repair the cause, then re-seal at a new head under the restart rule; or a recorded maintainer-directed handling of the two tests) is the maintainer's.

### 2026-10-25 — Re-seal row 6: maintainer decision, the `make test` leg attributed to its CI twin `Unit Tests (stable)` (plan 47-04 second continuation)

Reply received 2026-10-25 between 20:00Z and 20:04Z (UTC), verbatim:

> agree to your recommendation to option one, gated on the CI result, falling back to 4 if that result is red.

This is option 1 of the row-6 checkpoint, gated. Effect, exactly as decided:

- The `make test` leg of row 6 (`cargo test --workspace --lib --bins`) is **NOT RUN LOCALLY to a pass**: it is **RED locally** in this sandbox (two `run_api_wiring` worker-deadline tests, Local sweep row 60) and is attributed to the CI job `Unit Tests (stable)`. It is never recorded as a local pass.
- Precedent: `.planning/WINDOWS.md` row 73 (kind `unrun-verify`, phase 50, status `open`, recorded 2026-10-15): the same two tests fail identically in this sandbox, reproduced at the pre-Phase-50 commit `28b6a69b`; "pre-existing and sandbox-specific, not a Phase 50 regression; CI on the pushed branch is the proof of record". Row 73 stays open; this plan does not close or edit it.
- Gate: `ci.yml` push run `40228639490` on head `d900b21b` (tree outside `.planning/` identical to SHA-H): job `Unit Tests (stable)` id `121275766339` concluded `success` at 2026-10-25T20:18:33Z (orchestrator read 20:19:42Z; re-read by this agent 20:21:33Z, Local sweep row 61). Steps 5 `Run unit tests` (`cargo test --workspace --lib --bins`) and 6 `Run doc tests` (`cargo test --workspace --doc`) both `success`. The gate is met on the **job conclusion**; the fallback to option 4 does not trigger. The per-test CI log line could not be read (run still `in_progress`; the blob log host is denied by this sandbox's proxy), so the two test names are not confirmed by name in the CI log; the final CI twins are filled in plan 47-06 from the run that concludes.
- The legs that never ran in row 60 were then run directly in the Makefile's order and are all green: doc tests, `make audit`, `make build-release`, `cargo publish --workspace --dry-run` (Local sweep rows 62-65), with the twelve-crate order recorded. They were run as separate commands, after a second `cargo clean` before the release build (SPACE=a headroom judgement, not an out-of-space abort), so **`make publish-dry-run` as one chain was never green locally**; the row reads as a mix of local passes and one CI-attributed leg, and says so.
- Row 60's RED verdict and its Findings entry are left as written (append-only, D-00d). D-00j was honoured: the plan halted with nothing fixed and resumed only on this decision. Cause of the two local failures remains **undetermined**; CI's green `Unit Tests (stable)` on the same tree is consistent with a sandbox-specific timing bound (WINDOWS row 73) but is not a diagnosis.

### 2026-10-25 — Plan 47-05: SHA-A0 pushed by the orchestrator, PR #59 retitled, attribution footer rewritten in transit

- **SHA-A0 provenance.** `SHA-A0` `d4e66b47628559b175f1958b4f5267058096dd7f` reached `origin` through the orchestrator's post-wave-4 push (about 21:04Z), not through a push made by this plan; the plan's own `git push` returned `Everything up-to-date`. Recorded as measured; PR #59's `head.sha` equals it (CI-run table, "Plan 47-05").
- **The earlier run was cancelled.** `ci.yml` push run `40228639490` on `d900b21b` ended `cancelled` by the concurrency group when SHA-A0 landed; its already-concluded `success` jobs are mid-plan observations, not proof of record (plan 47-06 reads the SHA-A0 runs once).
- **PR body attribution differs from what was sent.** The stored body lacks the `🤖 Generated with [Claude Code](https://claude.com/claude-code)` line sent and ends with an appended `---` / `_Generated by [Claude Code](https://claude.ai/code)_` footer. Not authored by this plan and not re-written; owner: none, informational (the maintainer may edit the footer by hand if the exact line matters).
- **PR title and body written through REST on the first attempt** (no refusal, no maintainer fallback command). The body states the audit's rows 4, 5 and 6 as CI-attributed or mixed, says CI on the pushed head had not concluded, and says the box is unticked.

### 2026-10-25 — Plan 47-06: `pre-commit` red on SHA-A0 (required context; D-00j halt; cause diagnosed, nothing fixed)

- **What was read.** Both `pre-commit` runs on SHA-A0 `d4e66b47628559b175f1958b4f5267058096dd7f` concluded `failure`: push run [40233738812](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738812) (job [121290756636](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233738812/job/121290756636), step 7 `Run pre-commit`) and pull_request run [40233745937](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233745937) (job [121290778127](https://github.com/DF3NDR/paladin-dev-env/actions/runs/40233745937/job/121290778127), step 7 `Run pre-commit`). `pre-commit run --all-files` is one of the 44 required contexts of the `Protect main branch` ruleset, so this is a red required check on the candidate head. Both are recorded as `failure` in the CI-run table, never as passes.
- **Failing output, as supplied (not read by this agent).** The job log sits on the blocked blob host. The maintainer pasted the run log to the orchestrator, which relayed it: every hook passed except `end-of-file-fixer`, which reported `Fixing .planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md`, a one-line diff removing a trailing blank line at the end of that file.
- **Independent confirmation from git (read-only).** The last bytes of `47-CI-EVIDENCE.md` at `d4e66b47` (SHA-A0) are `.` `\n` `\n` (a trailing blank line); at `2d76e0cc` and at local HEAD `a8afb720` they are `.` `\n`. Walking the file's history, the trailing blank line is absent at `c3aa399f` and present at `d900b21b`, `72af77e2` and `342aeac2`: **cause commit `d900b21b`** (`docs(47-04): record row 4 decision, row 5 attribution and the red row 6 (D-00j halt)`). **Fixing commit `2d76e0cc`** (`docs(47-05): record the push of SHA-A0 and the PR #59 retitle`) removed it incidentally while appending its own section; it is one of the three unpushed local commits. No release file was involved and no file outside `.planning/` is implicated. No local `pre-commit` binary exists in this sandbox, so the hook itself was not run here.
- **Decision requested, not taken.** D-00j: nothing is fixed. Pushing SHA-A1 would carry `2d76e0cc`'s incidental fix to `origin` and re-run `pre-commit` on SHA-A1, which is the intended proof for the next head; whether that counts as the recorded remedy, or whether the red needs a different decision, is the maintainer's call. Not an infrastructure flake: the cause is a deterministic hook rewrite, so no re-run is justified and none was made. Owner: the maintainer (checkpoint of plan 47-06 Task 1).

### 2026-10-25 — Plan 47-06: the advisory `CodeQL` results check is `failure` on SHA-A0 (not required; carried, not acted on)

- Check run 121293439567 (app `github-advanced-security`, name `CodeQL`): `failure`, title `24 new alerts including 24 high severity security vulnerabilities`, 24 annotations, every one `Cleartext logging of sensitive information` across `src/config/treasurer.rs`, `src/application/services/paladin/*`, `src/bin/paladin-server.rs`, `crates/paladin-llm/src/config/layers.rs`, `crates/paladin-memory/src/*` and `examples/*`. The workflow runs `codeql.yml` (`CodeQL Analysis (Rust)`) are both `success`.
- It is not among the 44 required contexts, and CodeQL is advisory-only in this repository (`.github/instructions/security.instructions.md`; it does not gate a merge). The same check reads `failure` with the same title on the earlier PR head `2e857c6ce7e0d9f37ff5cc74def293d2d0587756`, so it predates this phase's version bump (no file under `src/` or `crates/*/src/` changed between `SHA-PRE-BUMP` and SHA-R). Not triaged here; recorded so the maintainer reads it before ticking. Owner: the maintainer.

### 2026-10-25 — Plan 47-06: the `Coverage` figure and the job logs are unreadable from this sandbox (D-00e)

- The `Lines:` figure of the `Coverage` job (PR run job 121290924766, push run job 121290851599, both `success`) could not be read: job logs and the `coverage-summary` artifact (PR 12615295044, push 12615515178) are served from `productionresultssa12.blob.core.windows.net`, which this sandbox's proxy answers `Forbidden`; check-run output, annotations, commit statuses and PR comments carry no figure. Recorded as unreadable, never as an expected number, and requested from the maintainer at the tick checkpoint. The `Check publish order` step is recorded by its step conclusion for the same reason (its printed line is in the unreadable log).
