# Phase 37 v0.10.0 Crate Release — CI Evidence Record (plans 37-01 through 37-11)

**Phase:** 37-v0-10-0-crate-release
**Branch:** `feature/phase-33` (pre-merge wave); `chore/37-close` (post-tag wave, per D-10)
**Written:** 2026-09-18

This record follows the `29-CI-EVIDENCE.md` / `33-CI-EVIDENCE.md` house form: a **Local sweep**
table, a **CI-run table**, a dedicated **Registry verification (D-08)** section, a **Findings
carried forward (D-00d)** section, and a closing **Summary and what remains**. Unlike those two
single-wave records, this phase spans D-09's two-wave split around the D-02 merge+tag checkpoint,
so the **Provenance** block below carries one line per wave-relevant SHA rather than a single
"Head SHA" line — the four not yet known are recorded as `pending` and filled in by later plans in
this phase, never retrofitted into this paragraph's original text (D-00d amend-at-source).

---

## Provenance

- **Local re-seal head SHA (this plan, plan 37-01):** `522ab1d4c4c4b5a62a8bbbbc5e234b0a29edbadd` —
  the tip of `feature/phase-33` at dispatch of this plan (`docs(37): begin phase execution`).
- **PR head at open (D-01):** `1bb9406343b7fb965e0724ac7a689d45e4e5f61c` — the tip of
  `feature/phase-33` when release PR #55 was opened (plan 37-06, 2026-09-18):
  https://github.com/DF3NDR/paladin-dev-env/pull/55.
- **Post-§11-tick final SHA (D-03):** `pending` — filled in after the maintainer ticks §11 in
  `.project/v0.10.0/09-program-acceptance-audit.md` and that tick commit is pushed.
- **`main` merge commit (D-02, D-04):** `1d4a9724cc219b85856a23012543458d62559e47` — the
  maintainer merged PR #55 with the merge-commit method at `2026-09-18T21:21:45Z` (parents
  `8ed14aea05e9e5bca9695211da4c4ad6991e307b` and `1bb9406343b7fb965e0724ac7a689d45e4e5f61c`;
  tree `d5e056d87a5716a7ec82f00805ff44069b6aff2e`, byte-identical to `1bb94063`'s own tree — an
  independent re-verification by this continuation, not merely a repeated claim). **Note (process-
  order deviation from D-03, recorded per D-00d, not edited away):** the "Post-§11-tick final SHA"
  slot immediately above this one is **N/A-by-deviation** — no §11-tick commit preceded this
  merge; the maintainer merged before ticking §11 (see the dated entry below, "Maintainer acts and
  statements, 2026-09-18 (post-CI, pre-tag)," statement (a)). The in-session sign-off of record for
  this head lives in that same entry, statement (d) — not a tick-commit SHA, because none exists.
- **Tagged commit (D-02):** `pending` — filled in after the maintainer pushes the annotated
  `v0.10.0` tag; per D-00b this is provably the same commit as the merge commit above.

---

## Pre-flight measurements (this plan, plan 37-01)

Measured live at dispatch of this plan — not copied from any orchestrator-reported value:

| Measurement | Command | Result |
|---|---|---|
| Free space on `/workspace` | `df -BG --output=avail /workspace` | **18G** available |
| Working tree cleanliness | `git status --porcelain` | Empty — clean tree |
| `origin/main` vs `HEAD` divergence | `git fetch origin` (read-only) then `git rev-list --left-right --count origin/main...HEAD` | `0  500` — **0** commits in `origin/main` not in `HEAD` (main has not moved), **500** commits in `HEAD` not in `origin/main` |
| Unpushed commit count | `git status -sb` | `feature/phase-33...origin/feature/phase-33 [ahead 24]` — **24** local commits not yet on the branch's own remote tracking ref |
| Local re-seal head SHA | `git rev-parse HEAD` | `522ab1d4c4c4b5a62a8bbbbc5e234b0a29edbadd` |

**Reading these numbers:** the left count of the `origin/main...HEAD` divergence is `0`, so per
CONTEXT Discretion item 6 no "main has moved" re-seal-from-the-top condition applies at this
measurement point — this must be re-checked at each later plan's own dispatch, not assumed to
still hold. 18G free and a clean tree are recorded here specifically so that an ENOSPC or
dirty-tree abort in plan 37-02/37-03 is legible as an environment stop under D-14, never
mistaken for a red gate.

**Addendum — 2026-09-18, post-Task-1 environment change (append-only, D-00d; nothing above this
addendum is edited):** between Task 1 and this continuation, the maintainer reported acting on
the 18G figure above. Obtained via the runtime's interactive question mechanism
(`AskUserQuestion`), the maintainer's reply, verbatim:

> "I ran `cargo clean` and now there is plenty of space."

Re-measured live by this continuation, after that report:

| Measurement | Command | Result |
|---|---|---|
| Free space on `/workspace` | `df -BG --output=avail,pcent /workspace` | **135G** available, 84% used |
| Working tree cleanliness | `git status --porcelain` | Empty — clean tree |

This satisfies plan 37-02's precondition text ("the figure recorded by plan 37-01 Task 1"), now
current at 135G, well above both plan 37-02's (>= 20 GiB) and plan 37-03's (>= 40 GiB) thresholds.
`target/` is confirmed cold (near-empty directory, no build artifacts) as a direct consequence of
the `cargo clean` reported above — this is stated plainly so that plan 37-02/37-03 gate timings
are read as cold-build timings, not as a regression against Task 1's or any prior phase's warm-cache
figures. A separate, out-of-band `cargo clippy --workspace --all-targets --all-features -- -D
warnings` cache warm-up was run by the orchestrator ahead of this continuation (finished clean, no
warnings, 6m 23s) — that run was cache preparation for the commit hooks below, not a gate, and is
not recorded as a Local sweep row.

---

## Local sweep

| # | Command | Result (verbatim/summarized) | Verdict |
|---|---------|-------------------------------|---------|
| 1 | `COUNT=$(grep -c TBD MIGRATION.md \|\| true); echo "COUNT=${COUNT}"` (D-06 gate 1, first half — the no-placeholder-marker check; captured into a variable per Pitfall 4, never read from `grep -c`'s exit code, since a zero count makes `grep -c` exit `1`) | `COUNT=0` | ✅ PASS |

**Row 1 is the phase's proof that the whole evidence path works end to end on one thin
end-to-end slice** — the exact command later gate-1 re-seals in plan 37-02/37-03 will reuse, run
here for real, against the real tree, with a real captured count rather than a rehearsal. The
second half of D-06 gate 1 (`make check-migration-allowlist`, the §9.2 ↔ semver-checks allowlist
set-equality check) is deliberately **not** run by this plan — it belongs to the fuller D-06
re-seal sweep plan 37-02/37-03 owns; this task's scope is proving the path with one row, not
running the whole gate set early.

### Plan 37-02 head SHA (Task 1, dispatch)

`git rev-parse HEAD` → `028e9726c2388da43d237af06926506bdd8760bf` — the tip of `feature/phase-33`
at dispatch of plan 37-02 (tip of `docs(37-01): complete release-evidence tracer plan`), measured
live, `git status --porcelain` empty, `df -BG --output=avail /workspace` → `135G`. Every row below
through Task 3 ran against this exact SHA's source tree (`crates/`, `src/`, `tests/`,
`Cargo.toml`/`Cargo.lock`) — the three task commits between rows only ever touch this evidence
file itself, never a source file, so the code under test does not change between tasks even though
`HEAD` advances with each task's own commit. Each row below is cross-referenced against this same
head SHA rather than re-stating it per row.

| 2 | `make check-migration-allowlist` (D-06 gate row 1, second half — head `028e9726`) | 15 `crate\|type` pairs in both the MIGRATION.md §9.2 register and `.cargo/semver-checks-allowlist.toml`, set-equal in both directions — identical pair count to `33-CI-EVIDENCE.md` row 4 and §11's own 15 | ✅ PASS |
| 3 | `make check-gates` (D-06 gate row 1 bundle — head `028e9726`) | Per-crate CHANGELOG coverage 11/11; package-name allow-list 12/12; advisory-exception register 11 rows vs 11 `deny.toml` + 5 `.cargo/audit.toml` ignore entries, all satisfied; workflow inline-suppression scan: 7 files, 165 steps, 1 `cargo audit` invocation, 0 inline suppressions; workflow trigger-policy table 7/7; CodeQL dismissal register 6/6; plus row 2's set-equality check — all seven sub-targets exit 0 | ✅ PASS |
| 4 | `U=$(grep -c '^## \[Unreleased\]' CHANGELOG.md; true); echo "$U"` (D-06 gate row 7, hard assertion — head `028e9726`) | `0` | ✅ PASS |
| 5 | `H=$(grep -c '^## \[0.10.0\]' CHANGELOG.md; true); echo "$H"` (D-06 gate row 7, hard assertion — head `028e9726`) | `1` | ✅ PASS |
| 6 | `N=$(cat CHANGELOG.md MIGRATION.md \| grep -c 'v0[.]11[.]0'; true); echo "$N"` (D-06 gate row 7, hard assertion, the withheld next-version string — head `028e9726`) | `0` | ✅ PASS |
| 7 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'rag'` (D-06 gate row 7, recorded reading, §11 topic 1 of 3 — head `028e9726`) | `22` (up from `33-CI-EVIDENCE.md` row 28's `17`; both non-zero, no regression) | ✅ PASS |
| 8 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -c 'TokenUsage'` (D-06 gate row 7, recorded reading, §11 topic 2 of 3 — head `028e9726`) | `4` (same as `33-CI-EVIDENCE.md` row 29) | ✅ PASS |
| 9 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -c 'Commissary'` (D-06 gate row 7, recorded reading, §11 topic 3 of 3 — head `028e9726`) | `15` (same as `33-CI-EVIDENCE.md` row 30) | ✅ PASS |
| 10 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'mdBook'` (D-06 gate row 7, recorded reading, documentation-phase topic 1 of 4, Phases 34-36.1 — head `028e9726`) | `1` | ✅ PASS |
| 11 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'rustdoc'` (D-06 gate row 7, recorded reading, documentation-phase topic 2 of 4 — head `028e9726`) | `0` — carried as a finding below, not a gate failure (see this task's action text and `## Findings carried forward (D-00d)`) | ⚠️ RECORDED, not a gate |
| 12 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'examples'` (D-06 gate row 7, recorded reading, documentation-phase topic 3 of 4 — head `028e9726`) | `7` | ✅ PASS |
| 13 | `awk '/^## \[0.10.0\]/,/^## \[0.9/' CHANGELOG.md \| grep -ci 'intra-doc'` (D-06 gate row 7, recorded reading, documentation-phase topic 4 of 4 — head `028e9726`) | `0` — carried as a finding below, not a gate failure (see this task's action text and `## Findings carried forward (D-00d)`) | ⚠️ RECORDED, not a gate |

Rows 4-13 are the split gate-row-7 assertion the plan's action text prescribes: rows 4-6 plus
row 3's `check-changelogs` sub-target are the four hard assertions (all green, D-14 would stop the
plan on any one of them going red); rows 7-13 are seven recorded topic-count readings, none of
which is itself a red-gate condition per the plan's own text — a zero on a documentation-phase
topic (rows 11 and 13) is a carried finding, not a fix, and `CHANGELOG.md`/`MIGRATION.md` were not
edited either way. `git status --porcelain CHANGELOG.md MIGRATION.md` confirmed empty after this
task.

### Task 2 head SHA note

`git rev-parse HEAD` at Task 2 dispatch → `cb2ebf3ebb0a54aa25867f6fc58f10046f8e2b85` (Task 1's own
evidence-file commit). The source tree under test (`crates/`, `src/`, `tests/`,
`Cargo.toml`/`Cargo.lock`) is byte-identical to head `028e9726`'s — Task 1's commit touched only
`.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`. Both compiled gates below were
hosted detached per the plan's long-running-command protocol (cold build, `target/` confirmed
cold by plan 37-01's addendum); the command recorded is the plan's exact verbatim command, not the
`target/37-02/run.sh` wrapper used to host it.

| 14 | `cargo test --features web-server --test v0_9_config_boot` (D-06 gate row 2 — head `cb2ebf3e`, source tree == `028e9726`) | `Finished \`test\` profile [unoptimized + debuginfo] target(s) in 2m 00s` (cold build) then `running 9 tests` / all 9 `ok` / `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s` — same 9-test count as `29-CI-EVIDENCE.md` row 8 and `33-CI-EVIDENCE.md` row 6. This target also runs inside CI's `e2e-platform-api` job (`ci.yml:1341-1361`, asserts >= 9 tests selected) — the pre-merge CI run in plan 37-07 re-proves it | ✅ PASS |
| 15 | `cargo test -p paladin-web --test openapi_golden_v0_9` (D-06 gate row 3 — head `cb2ebf3e`, source tree == `028e9726`) | `Finished \`test\` profile [unoptimized + debuginfo] target(s) in 42.42s` (cold build) then `running 7 tests` / all 7 `ok` / `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s` — same 7-test count as `33-CI-EVIDENCE.md` row 7 (the legitimate 6→7 rise since Phase 29 already recorded there). This target also rides inside the normal `paladin-web` test sweep (default/`web-server` feature `Build & Test` jobs), not a standalone CI job — the pre-merge CI run in plan 37-07 re-proves it | ✅ PASS |

`git status --porcelain -- crates src tests` confirmed empty after both commands (no source file
modified).

### Task 3 head SHA note

`git rev-parse HEAD` at Task 3 dispatch (this continuation) → `af21ede92079493e3f965fb66bee2738d51154bc`
(Task 2's own evidence-file commit). The source tree under test (`crates/`, `src/`, `tests/`,
`Cargo.toml`/`Cargo.lock`) is byte-identical to head `028e9726`'s — Tasks 1 and 2 only ever touched
`.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`. Task 3 itself spans two runs of the
MSRV and semver-checks gates: the MSRV gate (row 16) completed once, cleanly, before a host DNS
outage and reboot at 16:22-16:35 UTC on 2026-09-18; the semver-checks loop was interrupted by that
same outage after 7 of 11 packages, and — per the maintainer's decision recorded above under
"Plan 37-02, Task 3 — environment interruption during the first semver-checks re-run (2026-09-18)"
— was re-run exactly once, in full, after the outage. Rows 17-27 below are that single, complete,
post-outage re-run; the interrupted run's own per-package results are recorded only in the finding
above and in the preserved logs under `target/37-02/interrupted-20260918T1622Z/`, never as Local
sweep rows (no gate row is recorded from a run that never produced a verdict).

| 16 | `env RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets` (D-06 gate row 5, MSRV floor — head `af21ede9`, source tree == `028e9726`; CI's own flag set, no `--locked` passed, matching `ci.yml`'s `msrv` job) | `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 5m 07s`; `grep -c '^warning' target/37-02/g5-msrv.log` → `0` warnings; ran to completion at `end 2026-09-18T16:12:18Z`, **before** the 16:22 UTC outage, on a cold `target/` (post-`cargo clean`, per the pre-flight addendum above) — not re-run, per the maintainer's decision (only the semver loop was re-run) | ✅ PASS |

Gate row 4 — `cargo semver-checks check-release --package <pkg> --default-features
--baseline-version 0.9.0` against the `0.9.0` baseline, for the eleven crates named in
`ci.yml`'s `semver` job. `paladin-eval` is excluded: it has no published `0.9.0` baseline (it did
not exist at `0.9.0`), so the tool would error on a nonexistent baseline — the same underlying fact
D-17 exists to close, and the exclusion is CI's own, not this phase's choice. Every one of the
eleven runs below is from the single post-outage re-run authorized by the maintainer (loop started
`2026-09-18T16:49:27Z`), all against head `af21ede9` / source tree `028e9726`, all printing the
identical result shape §11 recorded (`major change` / `0 checks: 0 pass, 254 skip` / `Summary no
semver update required`):

| 17 | `cargo semver-checks check-release --package paladin-ai --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-ai v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [22.527s] paladin-ai` | ✅ PASS |
| 18 | `cargo semver-checks check-release --package paladin-ai-core --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-ai-core v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [12.747s] paladin-ai-core` | ✅ PASS |
| 19 | `cargo semver-checks check-release --package paladin-ports --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-ports v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [10.447s] paladin-ports` | ✅ PASS |
| 20 | `cargo semver-checks check-release --package paladin-battalion --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-battalion v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [10.399s] paladin-battalion` | ✅ PASS |
| 21 | `cargo semver-checks check-release --package paladin-herald --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-herald v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [7.051s] paladin-herald` | ✅ PASS |
| 22 | `cargo semver-checks check-release --package paladin-llm --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-llm v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [10.907s] paladin-llm` | ✅ PASS |
| 23 | `cargo semver-checks check-release --package paladin-memory --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-memory v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [7.409s] paladin-memory` | ✅ PASS |
| 24 | `cargo semver-checks check-release --package paladin-storage --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-storage v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [63.756s] paladin-storage` | ✅ PASS |
| 25 | `cargo semver-checks check-release --package paladin-notifications --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-notifications v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [157.253s] paladin-notifications` | ✅ PASS |
| 26 | `cargo semver-checks check-release --package paladin-content --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-content v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [171.033s] paladin-content` | ✅ PASS |
| 27 | `cargo semver-checks check-release --package paladin-web --default-features --baseline-version 0.9.0` (head `af21ede9`) | `Checking paladin-web v0.9.0 -> v0.10.0 (major change)` / `Checked [0.000s] 0 checks: 0 pass, 254 skip` / `Summary no semver update required` / `Finished [154.461s] paladin-web` | ✅ PASS |
| 28 | Tally of rows 17-27 (D-06 gate row 4 — head `af21ede9`) | **11/11 packages exit `0`**, all printing the identical `major change` / `0 checks: 0 pass, 254 skip` / `no semver update required` shape §11 recorded; `paladin-eval` excluded (no published `0.9.0` baseline). `git status --porcelain -- crates src Cargo.toml Cargo.lock` confirmed empty after the full re-run | ✅ PASS |

Full per-package logs (`start`/`end` timestamps, full `cargo semver-checks` output) live at
`target/37-02/semver-paladin-{name}.log` and `.exit`, this single post-outage re-run's own files;
the earlier, interrupted run's files are preserved separately at
`target/37-02/interrupted-20260918T1622Z/` and are not this gate's evidence (see the finding above).

### Plan 37-03 Task 1 head SHA (dispatch, D-06 gate row 6)

`git rev-parse HEAD` at dispatch of plan 37-03 → `ed0d3b06b897d4a905ce47989ccea10de38bc90b` (the tip
of `feature/phase-33`, `docs(37-02): complete local gate re-seal plan`). The source tree under test
(`crates/`, `src/`, `tests/`, `Cargo.toml`/`Cargo.lock`) is byte-identical to plan 37-02's own head
`af21ede9`'s — plan 37-02's Task 3 commit (`b95d2af0`) and Task 3's SUMMARY commit only ever touched
`.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md` and `37-02-SUMMARY.md`. Measured live
at dispatch: `df -BG --output=avail /workspace` → **120G**; `git status --porcelain` → empty (clean
tree). Re-measured immediately before launching gate row 6 (per this plan's action text — plan
37-02's numbers are a baseline, not a licence): still **120G** free, tree still clean, same head
SHA — both re-checks satisfy the plan's stated >= 40 GiB precondition with wide headroom.

**Quiet-machine wait (plan's pitfall 2, before launching g6):** the 1-minute load average was
checked before launch per the plan's quiet-machine rule (`release-check` stacks `clean-code`
immediately before the full workspace test suite, a previously-observed local timeout pattern in
this devcontainer). Readings, foreground-polled at ~60s intervals, no process killed: `4.38`,
`2.86`, `3.34`, `4.07`, `3.23`, `3.29`, `2.07`, `1.70` — settled under the ~2.0 threshold after ~7
minutes and launched at that reading (`1.70`). `nproc` → `8`.

Hosted detached per the plan's long-running-command protocol: `target/37-03/run.sh g6 make
publish-dry-run`, launched via `nohup ... &` + a separate `echo $! > target/37-03/g6.pid` statement
(no combined cleanup-plus-launch compound), polled to completion with repeated foreground
`timeout 560 tail --pid=... -f /dev/null` calls. Real exit code and full log captured at
`target/37-03/g6.exit` / `g6.log`.

| 29 | `make publish-dry-run` (D-06 gate row 6 — head `ed0d3b06`, hosted detached at `target/37-03/g6.{log,exit,pid}`) | Ran `2026-09-18T17:20:22Z`–`17:53:54Z` (**33m 32s** wall time, cold `target/` per plan 37-02's addendum — a genuine `release` profile build appears in the log at `13m 12s`, the longest single incremental compile step at `8m 02s`, consistent with a cold-cache run, not a no-op). Exit code `0`. `release-check` leg: all 40 `test result:` lines in the log report `ok` with `0 failed` (zero-failed-test statement satisfied); `cargo audit` reported the same **10** allowed pre-existing unmaintained/yanked-transitive warnings as `33-CI-EVIDENCE.md`/§11 (no new advisory introduced); `[0;32m✅ Release check passed![0m` printed. `cargo publish --workspace --dry-run` uploaded **12** crates in dependency order — `paladin-ai-core`, `paladin-ports`, `paladin-herald`, `paladin-llm`, `paladin-notifications`, `paladin-storage`, `paladin-web`, `paladin-battalion`, `paladin-content`, `paladin-memory`, `paladin-eval`, `paladin-ai` — each ending `warning: aborting upload due to dry run` (`grep -c 'aborting upload due to dry run' target/37-03/g6.log` → `12`, matching the required >= 12 count exactly). `paladin-doc-examples` (`publish = false`) was compiled and doc-tested (`Doc-tests paladin_doc_examples`, `running 0 tests`) as part of the workspace test suite but **never appears in the `Uploading` list** — correctly absent from the packaged/uploaded set. `git status --porcelain` empty after the run (full tree, not just the scoped paths); `git status --porcelain -- crates src tests Cargo.toml Cargo.lock` also empty. Free space after the run: **110G** (down ~10G from packaging + release-build artifacts, still far above threshold). | ✅ PASS (12/12, non-empty, dependency order) |

**Deviation from the plan's own `<automated>` verify block (disclosed, not a gate substitution of
meaning):** the plan's `<automated>` block chains `git status --porcelain` → `make
publish-dry-run` (foreground) → three `grep -c` assertions into one command. Run that way it would
both exceed the Bash tool's 600s ceiling (the gate took 33m 32s) and constitute hosting the gate
outside the mandated detached-with-real-exit-code protocol. Per this repo's own execution rules
(repo rule 2), the gate was hosted detached exactly once and the same three conditions — dry-run
abort count >= 12, zero `test result: FAILED` lines, `paladin-doc-examples` absent from the
`Uploading` list — were asserted directly against `target/37-03/g6.log`/`g6.exit` after the one run
completed, not re-run. No gate's pass/fail meaning was altered by this substitution.

### Plan 37-03 Task 2 head SHA (dispatch — adjacent checks)

`git rev-parse HEAD` at dispatch of Task 2 → `d2db617b0d1307a388fc44730810e24c602b6938` (Task 1's
own evidence-file commit). The source tree under test is byte-identical to gate row 6's own head
`ed0d3b06` — Task 1's commit touched only `37-CI-EVIDENCE.md`. Run **after** the dry run per the
plan's own ordering instruction (heavy gates do not stack needlessly); both hosted detached at
`target/37-03/{security,apisurface}.{log,exit,pid}` per the same protocol as gate row 6.

| 30 | `make security` (`cargo audit` + `cargo deny check`, head `d2db617b`) | Ran `2026-09-18T17:57:44Z`–`17:57:49Z` (5s — advisory DB and dependency tree already warm from gate row 6's own `cargo audit` leg minutes earlier). Exit `0`. `cargo audit`: `warning: 10 allowed warnings found` — the identical count and identical crate/RUSTSEC-ID set as gate row 6's `release-check` leg and as `33-CI-EVIDENCE.md`/§11's own recorded figure (no new, non-allowlisted advisory). `cargo deny check` verbatim verdict line: `advisories ok, bans ok, licenses ok, sources ok` — matching the §11 four-section model exactly. | ✅ PASS |
| 31 | `make api-surface` (`scripts/check-api-surface.sh`, head `d2db617b`) | Ran `2026-09-18T17:58:00Z`–`18:00:46Z` (2m 46s). Exit `0`. `✅ API surface extracted to /tmp/tmp.Xoax1ovIYA (3959 items)` — identical item count to `09-program-acceptance-audit.md` §11's own recorded reading. `✅ API surface unchanged` — zero drift against `.project/current-exports.txt`. `git status --porcelain -- .project/current-exports.txt` confirmed empty after the run (baseline not moved; `make api-surface-update` was never invoked). | ✅ PASS |

`git status --porcelain` confirmed empty (full tree) after both checks; free space after both:
**110G** (api-surface's temp-directory extraction does not persist).

### Local sweep — closing verdict tally (plan 37-03, Task 2, 2026-09-18)

**31 numbered Local sweep rows accounted for** (rows 1-31 above), matching the file's own row
count exactly:

- **29 unconditional local passes** (✅ PASS): rows 1-10, 12, 14-31 — every D-06 hard-assertion
  gate row (1 offline register guards + §9.2 set-equality, 2 `v0_9_config_boot`, 3 OpenAPI golden
  diff, 4 `cargo semver-checks` ×11 packages + tally, 5 MSRV floor, 6 `make publish-dry-run`
  packaging, 7 CHANGELOG completeness hard assertions), plus this plan's own two adjacent checks
  (`make security`, `make api-surface`), plus five of gate row 7's seven recorded topic-count
  readings (RAG, TokenUsage, Commissary, mdBook, examples).
- **2 named carried conditions** (⚠️ RECORDED, not a gate): rows 11 and 13 — the `rustdoc` and
  `intra-doc` documentation-phase topic-count readings inside gate row 7's recorded-not-gated
  scope, both `0`, both explicitly not a D-14 stop per the plan text that authored them (see
  "Findings carried forward" above) and both left unedited in `CHANGELOG.md`.
- **0 rows read as CI-attributed.** No numbered Local sweep row in this file claims the 82%
  workspace line-coverage floor — that gate is named honestly in the closing summary below as
  structurally unmeasurable here, never assigned a row number or a verdict cell in this table.

All seven of D-06's gate rows (1 through 7) now carry at least one Local sweep entry: rows 1-2 →
gate 1; row 14 → gate 2; row 15 → gate 3; rows 17-28 → gate 4; row 16 → gate 5; row 29 → gate 6;
rows 4-13 → gate 7. Rows 30-31 are the two adjacent house-sweep checks (`make security`, `make
api-surface`), not numbered D-06 gate rows themselves.

---

## CI-run table

**Opening note — the two read-only GitHub queries this plan ran (Task 1, step 5):**

`gh pr list --head feature/phase-33 --state all --json number,state` returned `[]` — **no PR
exists yet**, the expected pre-D-01 state.

`gh run list --branch feature/phase-33 --limit 5 --json workflowName,headSha,conclusion` did
**not** return an empty list — it returned 5 rows, all `"conclusion":"success"`, all at SHAs
older than this plan's own local re-seal head (`522ab1d4`): four rows at
`6fe5b70a1ff973e088a58184f57d5401c9f97895` (`ci.yml`, `codeql.yml`, `feature-flags.yml`,
`pre-commit`, all success) and one further row at `20195975c1c2665abb169b287fa178353d672bd2`
(`feature-flags.yml`, success). This is the same shape `29-CI-EVIDENCE.md` and
`33-CI-EVIDENCE.md` recorded for their own phases: proof that an earlier point on this branch was
fully green across the pushable workflows, **not** proof of anything about this phase's own
commits — none of the five rows' SHAs match `522ab1d4`. No `gh` command in this plan failed with
an auth or permission error; both queries returned normally.

No further rows are added to this table by this plan — the real pre-merge PR-head run(s), the
run on the post-§11-tick final SHA, the post-merge run on the tagged `main` merge commit, and the
real release run are each supplied by later plans in this phase, per the wave structure `37-PATTERNS.md`
and `37-RESEARCH.md` Q7 describe.

---

### Plan 37-06 — push + release PR opened (2026-09-18)

**Push (D-01, Task 1 step 1):** `git push origin feature/phase-33`, hosted detached
(`target/37-06/push.{log,exit,pid}`), ran `2026-09-18T18:50:42Z`–`18:54:12Z` (3m 30s). Exit `0`.
Full pre-push hook stage ran and every hook reported `Passed` (`cargo fmt`, `cargo clippy
--workspace --all-targets --all-features -- -D warnings`, `cargo build --workspace`, `cargo test
--workspace --lib`, `check-doc-examples`, `check-doc-config`, `check-api-surface`, `doc-check`,
`check-api-examples`) — `--no-verify` was never passed. Remote accepted the push:
`6fe5b70a..1bb94063  feature/phase-33 -> feature/phase-33`.

**Post-push verification:** `git fetch origin` then `git rev-parse HEAD` =
`git rev-parse origin/feature/phase-33` = `1bb9406343b7fb965e0724ac7a689d45e4e5f61c` (confirmed
equal). `git status --porcelain -- .project/current-exports.txt` — empty; the API-surface
baseline was not regenerated and did not move.

**Release PR (D-01, Task 1 step 2):** `gh pr list --head feature/phase-33 --state all` returned
`[]` immediately before creation (no duplicate). `gh pr create --base main --head
feature/phase-33 --title "release: v0.10.0 — Durable Agent Execution Runtime" --body-file
target/37-06/pr-body.md` succeeded:

- **PR #55:** https://github.com/DF3NDR/paladin-dev-env/pull/55
- **Base:** `main` / **Head:** `feature/phase-33`
- **Head SHA:** `1bb9406343b7fb965e0724ac7a689d45e4e5f61c`
- Body verified to contain the literal phrase `merge-commit`, a not-squash/not-rebase sentence,
  a pointer to this file (`37-CI-EVIDENCE.md`) and a pointer to `CHANGELOG.md`'s `[0.10.0]`
  section (`gh pr view --json body` grepped for both markers — both matched).

| Workflow | Run ID (URL) | Event | Conclusion | SHA | Notes |
|---|---|---|---|---|---|
| `ci.yml` | [35382874018](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382874018) | `push` | `success` | `1bb9406343b7fb965e0724ac7a689d45e4e5f61c` | Triggered by the D-01 push. Conclusion and job tally filled by plan 37-07 (this file's "Plan 37-07" section below): 37 jobs, 34 `success`, 3 `skipped` by design (`Benchmark Regression Signal (Non-Blocking)`, `Publish Dry Run` — tag-gated, `End-to-End Tests` — conditional job), 0 `failed`. |
| `ci.yml` | [35382953376](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382953376) | `pull_request` | `success` | `1bb9406343b7fb965e0724ac7a689d45e4e5f61c` | Triggered by PR #55's open against the same head SHA. Conclusion and job tally filled by plan 37-07: 37 jobs, 35 `success`, 2 `skipped` by design (`Publish Dry Run` — tag-gated, `End-to-End Tests` — conditional job), 0 `failed`. |

Both run IDs confirmed via `gh run list --branch feature/phase-33 --workflow ci.yml --limit 10
--json databaseId,event,headSha,status,conclusion` filtered to `headSha ==
1bb9406343b7fb965e0724ac7a689d45e4e5f61c` — exactly 2 rows matched, satisfying the plan's `R >=
2` acceptance criterion.

**Full Task 1 `<automated>` verify block, run after the PR existed:** all six chained assertions
(`HEAD == origin/feature/phase-33`; `.project/current-exports.txt` unmodified; exactly 1 open PR;
body contains `merge-commit`; body contains `37-CI-EVIDENCE`; >= 2 `ci.yml` runs at the pushed
head SHA) passed together — `ALL VERIFY CHECKS PASSED (N=1, R=2)`.

No merge, no tag, no workflow dispatch, no `gh run rerun`, and no force-push occurred in this
plan. No credential-shaped text appears in the PR body or in this record.

---

### Plan 37-07 — every workflow run on the PR head SHA, and the required-context tally (2026-09-18)

**Method substitution, disclosed per the dispatch's own instruction.** Plan 37-07's Task 1
precondition text reads `gh run list --branch feature/phase-33 --workflow ci.yml ...`. By the
time this plan ran, PR #55 was already merged (see "Maintainer acts and statements, 2026-09-18"
above) and GitHub had deleted the remote `feature/phase-33` branch — a branch-scoped `gh run
list` query against a deleted branch returns nothing useful. This plan substituted SHA-scoped
queries instead: `gh api "repos/DF3NDR/paladin-dev-env/actions/runs?head_sha=<sha>"` for the
workflow-run list, `gh api repos/DF3NDR/paladin-dev-env/commits/<sha>/check-runs` for the
per-job check-run list, `gh api repos/DF3NDR/paladin-dev-env/rules/branches/main` for the live
required-context set, and `gh pr checks 55 --required` (the PR object itself still resolves by
number after merge) for the required-check pass/skip tally. All four are read-only GETs against
the same PR head SHA `1bb9406343b7fb965e0724ac7a689d45e4e5f61c` the pending rows above already
named — no different commit is being evidenced, only a different query shape to reach it.

**Every workflow run on the PR head SHA** (`gh api "repos/DF3NDR/paladin-dev-env/actions/runs?head_sha=1bb9406343b7fb965e0724ac7a689d45e4e5f61c"`,
9 rows, all `status: completed`):

| Workflow | Run ID (URL) | Event | Conclusion | SHA | Notes |
|---|---|---|---|---|---|
| `ci.yml` | [35382874018](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382874018) | `push` | `success` | `1bb94063` | 37 jobs: 34 `success`, 3 `skipped` by design (`Benchmark Regression Signal (Non-Blocking)`, `Publish Dry Run` — tag-gated, does not run pre-tag, `End-to-End Tests` — conditional job), 0 `failed`. |
| `ci.yml` | [35382953376](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382953376) | `pull_request` | `success` | `1bb94063` | 37 jobs: 35 `success`, 2 `skipped` by design (`Publish Dry Run` — tag-gated, `End-to-End Tests` — conditional job), 0 `failed`. The `Coverage` job here is Task 2's evidence source below. |
| `codeql.yml` | [35382874047](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382874047) | `push` | `success` | `1bb94063` | The **workflow run** (the analysis completing without error) — advisory-only per `security.instructions.md`. Distinct from the separate "CodeQL" results **check**, recorded below, which is red. |
| `codeql.yml` | [35382953096](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382953096) | `pull_request` | `success` | `1bb94063` | Same distinction — advisory-only workflow-run conclusion, not the results check. |
| `pre-commit` | [35382874020](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382874020) | `push` | `success` | `1bb94063` | Required context `pre-commit run --all-files`. |
| `pre-commit` | [35382952999](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382952999) | `pull_request` | `success` | `1bb94063` | Required context `pre-commit run --all-files`. |
| `feature-flags.yml` | [35382874128](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382874128) | `push` | `success` | `1bb94063` | Every `Build & Test (<feature>)` required context lives in this workflow's runs, not `ci.yml`'s. |
| `feature-flags.yml` | [35382952857](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382952857) | `pull_request` | `success` | `1bb94063` | Same. |
| `Docs` (`docs.yml`) | [35382952930](https://github.com/DF3NDR/paladin-dev-env/actions/runs/35382952930) | `pull_request` | `success` | `1bb94063` | PR-gated (`pull_request` trigger only) — this is the **first** `docs.yml` run for any commit on `feature/phase-33`, matching `33-CI-EVIDENCE.md`'s own "not run" row for the pre-PR state. No `push`-event row exists for `docs.yml` by design (it has no `push` trigger). Job: `Build MDBook`, `success`. |

**Adjacent, non-required jobs on the same SHA** (recorded for completeness, not because either
gates the PR — `security.instructions.md` and this file both treat `Docker Build` /
`Kubernetes Smoke Test` as deliberately non-required):

| Job | Push-event check-run | Conclusion | PR-event check-run | Conclusion |
|---|---|---|---|---|
| `Docker Build` | `105729270066` | `success` | `105730543368` | `success` |
| `Kubernetes Smoke Test` | `105753476039` | `success` | `105754378951` | `success` |

**Required-context tally, computed from the live ruleset, not asserted.**
`gh api repos/DF3NDR/paladin-dev-env/rules/branches/main --jq '.[] | select(.type=="required_status_checks") | .parameters.required_status_checks[].context'`
returned **44 unique required context names** (full list captured; includes every
`Build & Test (<feature>)`, `Crate Isolation (<crate>)`, `Coverage`, `Code Quality`,
`Security Audit`, `OSV Scanner`, `License & Dependency Policy`, `pre-commit run --all-files`,
`Workflow Lint`, `Build MDBook`, `API Surface Tracking`, `Benchmark Compile Check`,
`Integration Tests`, `Docker Integration Tests`, `Unit Tests (stable|beta)`,
`Example Muster (Feature Matrix)`, `Feature Matrix Summary`, `CLI Snapshot Tests`,
`CLI Isolation (library without cli feature)`, `End-to-End Tests`). **`CodeQL` is not among the
44** — confirming it is not a required context on this ruleset, matching the `security.instructions.md`
advisory-only posture. `Docker Build` and `Kubernetes Smoke Test` are also not among the 44.

`gh pr checks 55 --required --json name,state,bucket,workflow` (the PR object still resolves by
number post-merge) returned **87 required check-run entries** (the 44 names, each appearing once
per triggering event — `push` and `pull_request` — for the workflows that run on both; `Docs`
only triggers on `pull_request`, accounting for the odd count): **85 `pass`, 2 `skipping`, 0**
anything else. Both `skipping` entries are `End-to-End Tests` (one per event) — named as
**skipped, not passed**, per this plan's own instruction; `End-to-End Tests` is a conditional job
this PR's diff did not trigger, not a required check that failed to run.

**Tally: 44/44 required contexts satisfied** (0 failures; the only non-`pass` bucket is
`End-to-End Tests`, correctly bucketed `skipping` by GitHub itself, not `pass` and not `fail`).

**D-15 (the single-rerun infrastructure-flake exception) was not invoked.** No required check,
and no run of any kind on this SHA, was red. There was nothing to classify and nothing to
re-run. The dispatch's own note that D-15's `gh run rerun --failed` is unavailable post-merge is
recorded here for completeness, not because it was needed: had a required check been red, this
plan would have stopped (D-14) rather than attempt a rerun that could not change anything the
maintainer can act on.

**The separate "CodeQL" results check — advisory, red, independently re-verified (not
re-triaged).** This is the code-scanning **results check** on the commit (distinct from the two
`codeql.yml` **workflow runs** recorded green above). Re-verified read-only by this plan:

- `gh api repos/DF3NDR/paladin-dev-env/check-runs/105726197798` → `conclusion: "failure"`,
  `status: "completed"`, title `"10 new alerts including 10 high severity security
  vulnerabilities"`, summary: *"New alerts in code changed by this pull request... Security
  Alerts: 10 high... Alerts not introduced by this pull request might have been detected because
  the code changes were too large."* (PR #55 carried 521 commits.)
- `gh api repos/DF3NDR/paladin-dev-env/check-runs/105726197798/annotations` → 10 annotated
  locations, cross-referenced by this plan against
  `gh api "repos/DF3NDR/paladin-dev-env/code-scanning/alerts?ref=refs/heads/main&state=open"`
  filtered to `rule.id == "rust/cleartext-logging"`: the 10 annotated locations match alert
  numbers **#31, #32, #33, #38, #40, #43, #44, #45, #46, #47** exactly, one location each. All
  ten carry `created_at: 2026-08-27T12:52:26Z` — already open on `main` before PR #55 existed.
- **Not a required context** — confirmed above (44 required names, none `CodeQL`); no
  `code_scanning` ruleset rule exists on `main`.
- **Maintainer decision pointer:** the maintainer's verbatim reply — `"We are going to proceed
  with 1."` (option 1 = proceed and record as advisory) — is already recorded with full
  provenance in this file's "Maintainer acts and statements, 2026-09-18 (post-CI, pre-tag)" entry,
  subsection (b), above. This plan does not re-quote it at length; it cites that entry as the
  record of the decision and adds only the independently re-verified alert-number cross-reference
  above, which that earlier entry did not itemize.
- **Stated plainly:** the ten alerts were **not** re-triaged as false positives by this plan or by
  any prior agent in this phase. They are carried forward as a named v0.11.0 triage finding, per
  the maintainer's own "proceed and record as advisory" instruction, not dismissed or downgraded.
- **This is not a D-14 stop.** `CodeQL`'s check-run conclusion is red, but it is not a required
  context, and the maintainer has already made the disposition decision (advisory) with
  provenance on record — the pre-declared condition for D-14 (a **required** check, or an
  unresolved red with no maintainer disposition) does not apply here.

---

### Plan 37-07 — SC2: the CI `Coverage` job's conclusion and printed figure (2026-09-18)

**The verdict rule, stated before the number, per the plan's own instruction: SC2 is satisfied
if and only if the `Coverage` job's own conclusion is `success`.** The printed figure below is
corroborating evidence, not the verdict — a non-`success` conclusion would be a D-14 stop
regardless of what the printed figure read, and a `success` conclusion is not overturned by
re-reading the number. Both instances of the job on this SHA are recorded because CI ran it
twice (once per `push`, once per `pull_request`); neither overturns the other.

**Job conclusions, verbatim (`gh api repos/DF3NDR/paladin-dev-env/commits/1bb9406343b7fb965e0724ac7a689d45e4e5f61c/check-runs`,
filtered to `name == "Coverage"`):**

| Run (event) | Job (check-run id) | Conclusion |
|---|---|---|
| `35382953376` (`pull_request`) | `105723181854` | `success` |
| `35382874018` (`push`) | `105722928700` | `success` |

**Both `success`. SC2 is satisfied.**

**Printed figures, recovered from the "Coverage summary" step's log and recorded to the exact
digits CI printed** — no rounding, no truncation, no re-derivation, no recomputation from any
other source (`curl` against each job's `/logs` endpoint with a bearer token obtained via
`gh auth token` and never printed, echoed, logged, or written to any file in this repository;
the raw logs themselves were read only in-memory by this plan and were not committed anywhere):

- **`pull_request` run (job `105723181854`):**
  ```
  Scope: --workspace --features integration-tests (the gated measurement)
  Lines:     111771/123587 = 90.44%
  Functions: 11785/14096 = 83.61%
  ```
- **`push` run (job `105722928700`):**
  ```
  Scope: --workspace --features integration-tests (the gated measurement)
  Lines:     111774/123587 = 90.44%
  Functions: 11786/14096 = 83.61%
  ```

The two runs' raw hit counts differ by a handful of lines/functions (111771 vs 111774 hit,
11785 vs 11786 hit, out of the same 123587/14096 denominators) — ordinary run-to-run noise from
async/timing-sensitive tests under instrumentation, not a regression between the two runs; both
round to the identical printed percentage, `90.44%` lines / `83.61%` functions. This matches the
figure already named in this file's own §11 sign-off brief (see "Maintainer acts and statements,
2026-09-18," subsection (d), above), independently re-derived here from the job logs directly
rather than merely repeated from that earlier mention.

**The tool invocation CI used**, so a reader can see which comparison produced the verdict
(`scripts/coverage.sh`, invoked by the `Measure coverage` step, delegated to from both `make
coverage` and the CI job per this file's own house form):

```
cargo llvm-cov --workspace --features integration-tests,llm-all \
    --lcov --output-path lcov.info --fail-under-lines "$FLOOR" -- --test-threads=1
```

**The configured floor is `82`** — read directly from `scripts/coverage.sh` (`FLOOR="${COVERAGE_FLOOR:-82}"`,
line 35), not assumed; `.github/workflows/ci.yml`'s `coverage` job does not set a `COVERAGE_FLOOR`
override in its `Measure coverage` step's `env:` block, so the default `82` is the floor CI
actually gated on for both runs above — matching ADR-0006 (`.planning/decisions/0006-coverage-gate.md`)
exactly.

**CI-attributed (D-00e), not a local measurement.** Docker is absent from this devcontainer
(`docker: command not found`, structurally unchanged since `33-CI-EVIDENCE.md` row 32 and every
earlier local sweep in this phase) — `scripts/coverage.sh`'s Redis/MinIO service-probe chain has
no target to resolve against outside a Docker network, so `make coverage` cannot complete here.
This is the structural reason the gate is evidenced from CI alone; local reproduction was not
attempted by this plan, consistent with plans 37-01 through 37-03. The
`2026-08-13-verify-local-coverage-reproduction.md` todo remains re-homed to the maintainer per
Phase 36.1 D-23 — recorded, not re-opened here. No local coverage figure, rounded, re-derived, or
otherwise, appears anywhere in this file as a substitute for the CI job's own reading.

---

## Registry verification (D-08)

**Pre-bootstrap baseline for `paladin-eval` (this plan, plan 37-01, Task 1 step 4):**

```
curl -s -o /tmp/pe_pre.json -w '%{http_code}' \
  -H 'User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)' \
  https://index.crates.io/pa/la/paladin-eval
```

**HTTP status: `404`** (body: an S3-style `NoSuchKey` XML error, the sparse index's un-bootstrapped
response shape). No `vers` field is present, because no version has ever been published. This is
the **expected pre-bootstrap state** per D-17 — recorded here as the D-17 baseline, not as a
failure of this plan or of the release. Task 2 hands this fact, and the reason it matters, to the
maintainer as a blocking-human checkpoint. Task 3 re-runs this identical query after the
checkpoint resolves and records the maintainer's reply verbatim alongside the post-checkpoint
status.

The full twelve-crate registry table (every publishable crate at `0.10.0`, per D-08, derived live
from `cargo metadata`) is **not** run by this plan — v0.10.0 has not been published anywhere yet.
That table is written by the post-tag wave plan once the real release run has completed.

**Post-checkpoint registry state for `paladin-eval` (this plan, Task 3):**

```
curl -s -o /tmp/pe_post.json -w '%{http_code}' \
  -H 'User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)' \
  https://index.crates.io/pa/la/paladin-eval
```

**HTTP status: `404`** (same `NoSuchKey` body shape as the pre-bootstrap baseline above —
re-verified 2026-09-18, after Task 2's checkpoint resolved). No `vers` field present.

**Maintainer's Task 2 reply, recorded verbatim** (obtained via the runtime's interactive question
mechanism, `AskUserQuestion`, presented against the three listed options "Bootstrapped 0.0.1" /
"Deferred" / "Not needed"; the maintainer answered in free text instead of selecting one):

> "You'll provide specific instructions (short runbook) for the Owner Gated  requirement when the
> requirement is needed and we will together make sure it is done properly."

**Orchestrator's classification of that reply (the orchestrator's reading, not the maintainer's
words): `deferred`.** Reasoning: the reply reports no publish and no placeholder version, so it is
not "bootstrapped `<version>`"; it makes no claim that crates.io now supports a pending publisher,
so it is not "not needed"; it postpones the act to the point of need and asks the agent to supply
a short runbook then, to be worked through together. This is also the fail-safe branch of the
three: it leaves plan 37-08's gate fully in force. The orchestrator stated this reading to the
maintainer in-session.

**Open obligation carried from this reply:** the agent owes the maintainer a short, specific D-17
runbook at the point of need. First natural opportunity: the PR CI wait in plans 37-06/37-07. Hard
deadline: before plan 37-08 Task 3's tag hand-off — per D-17's own instructions text, a `404` under
a "deferred" reply withholds the tag command and halts the phase there until the bootstrap is
actually done.

**Reading this 404 correctly:** the continuing 404 is the expected state under "deferred" — it is
**not** a failure of this plan (per Task 1's own note, and per this task's action text). The gate
that actually consumes this fact sits in plan 37-08, immediately before the tag hand-off, and
branches on the three-way reply captured verbatim above.

---

## Findings carried forward (D-00d)

Not populated by this plan. Task 3 appends the D-13 non-dispatch record and the first set of
carried documentation findings here; later plans append further findings as they are observed.
Nothing above this heading is edited by any later addition — additions are dated and appended
only.

### D-13 — dry-run dispatch not attempted (this plan, Task 3, 2026-09-18)

No `workflow_dispatch` of `release.yml` was attempted, and no rc tag or non-`v` shadow tag was
created.

**Traced reason:** the dispatch's `tag` input is used both as the ref to resolve (`verify-tag-source`,
`git rev-list -n 1 "$RELEASE_TAG"`) and as the literal version string matched against the
CHANGELOG heading (`create-release`) and against every publishable crate's manifest
(`check-release-consistency`). No single value satisfies all three constraints before a ref
literally named `v0.10.0` exists:
- `v0.10.0` / `0.10.0` (no such ref pre-tag) fails step 1 outright — `verify-tag-source` cannot
  resolve it as a revision.
- The exact 40-char merge-commit SHA resolves in step 1 but fails the changelog-heading match
  (`create-release`) — no `## [<40-hex-chars>]` heading exists.
- An existing older tag (e.g. `v0.9.0`) resolves and matches the changelog heading, but fails the
  manifest match (`check-release-consistency` clause 1) — the manifest is `0.10.0`, the tag strips
  to `0.9.0`.

`37-RESEARCH.md` Q1 carries the full row-by-row trace over these four candidates. D-13's own
fallback sentence — "if a dry run cannot be dispatched without a real tag, fall back to going
straight to the tag and record why; do not substitute an rc tag" — is the authority applied here.

**The theoretical non-`v`-prefixed shadow-tag escape hatch** (a lightweight tag literally named
`0.10.0`, no leading `v`, which would resolve step 1 and match steps 3-6 without matching the
`push: tags: v*.*.*` trigger glob) **was considered and rejected.** `37-RESEARCH.md` Q1 records it
as traced but untested by design, and explicitly not recommended: it adds an extra pushed tag
object outside the documented flow, for a low-value rehearsal, given the seven local gates already
prove packaging validity (Q5). No such tag was created.

### Carried documentation findings (this plan, Task 3, 2026-09-18)

Recorded only; nothing below is fixed by this plan (D-14 — this phase does not edit docs pages
under CONTEXT `<deferred>`; currency fixes are v0.11.0 scope):

- `docs/src/appendix/release-automation.md`'s per-crate Trusted Publishing table and Credential
  History ledger have no `paladin-eval` row — D-17's bootstrap (deferred, per the reply recorded
  above) is not yet reflected there, and won't be until the bootstrap actually happens.
- `docs/src/appendix/release-automation.md`'s "Canonical Publish Order" section still describes
  the pre-`paladin-eval` eleven-crate order; `scripts/publish-crates.sh`'s `CRATES` array is the
  live authority (twelve crates, `paladin-eval` at position 11 of 12).
- `docs/src/contributing/development-setup.md` still states eleven publishable crates; the tree
  (`cargo metadata`, live) says twelve — per D-00f, the shipped tree outranks any document.
- `docs/src/appendix/release-recovery.md` §1's example loop enumerates eleven crate names, not
  twelve.
- `CHANGELOG.md`'s `[0.10.0]` heading carries the date `2026-09-10`, earlier than the actual
  release date. Recorded, not edited: neither the D-06 gate set nor
  `scripts/check-release-consistency.sh` reads the date (clause 2 matches only the version
  heading), so this is a currency finding, not a gate failure.

### Plan 37-02, Task 3 — environment interruption during the first semver-checks re-run (2026-09-18)

**This is a dated, append-only finding recorded per the maintainer's decision below, before the
re-run it authorizes was executed.**

At 16:22 UTC on 2026-09-18 the host lost DNS resolution mid-run; the host then rebooted, coming
back up at **2026-09-18 16:35:02** (`uptime -s`). The executor running plan 37-02's Task 3 at that
moment was lost; a fresh continuation agent recorded this entry and performed the re-run described
below.

**The interrupted run:** `target/37-02/semver-loop.sh` (the plan's own unmodified wrapper around
the plan's exact `cargo semver-checks check-release --package <pkg> --default-features
--baseline-version 0.9.0` command, one log + one exit file per package) was started at
`start 2026-09-18T16:12:29Z` (`target/37-02/interrupted-20260918T1622Z/semver-loop.log`), against
head SHA `af21ede92079493e3f965fb66bee2738d51154bc` (Task 2's own commit; the source tree under
test is unchanged from `028e9726`, per the Task 2 head SHA note above). Per-package exit codes from
that run, in the order the loop iterates:

| Package | Exit code |
|---|---|
| paladin-ai | `0` |
| paladin-ai-core | `0` |
| paladin-ports | `0` |
| paladin-battalion | `0` |
| paladin-herald | `0` |
| paladin-llm | `0` |
| paladin-memory | `0` |
| paladin-storage | `101` |
| paladin-notifications | `101` |
| paladin-content | `101` |
| paladin-web | **no exit file — killed by the reboot before the command could finish** |

The first seven packages' logs each end `Summary no semver update required` — an ordinary,
complete verdict, not affected by the outage.

The last four packages never produced a semver verdict. The exact DNS-failure lines, quoted
verbatim from their preserved logs:

`target/37-02/interrupted-20260918T1622Z/semver-paladin-storage.log`:
```
error: `cargo metadata` exited with an error:     Updating crates.io index
warning: spurious network error (3 tries remaining): [6] Could not resolve hostname (Could not resolve host: index.crates.io)
warning: spurious network error (2 tries remaining): [6] Could not resolve hostname (Could not resolve host: index.crates.io)
warning: spurious network error (1 try remaining): [6] Could not resolve hostname (Could not resolve host: index.crates.io)
error: failed to get `paladin-storage` as a dependency of package `placeholder v0.0.0 (/workspace/target/semver-checks/registry-paladin_storage-0_9_0-x86_64_unknown_linux_gnu-ccbd4c2ebd266b33)`
```
ending: `[6] Could not resolve hostname (Could not resolve host: index.crates.io)` — exit `101` at
`end 2026-09-18T16:22:26Z`.

`target/37-02/interrupted-20260918T1622Z/semver-paladin-notifications.log`:
```
error: failed to retrieve index of crate versions from registry

Caused by:
    0: failed to read index metadata for crate 'paladin-notifications'
    1: error sending request for url (https://index.crates.io/pa/la/paladin-notifications)
    2: client error (Connect)
    3: dns error
    4: failed to lookup address information: Temporary failure in name resolution
```
exit `101` at `end 2026-09-18T16:22:27Z`.

`target/37-02/interrupted-20260918T1622Z/semver-paladin-content.log`:
```
error: failed to retrieve index of crate versions from registry

Caused by:
    0: failed to read index metadata for crate 'paladin-content'
    1: error sending request for url (https://index.crates.io/pa/la/paladin-content)
    2: client error (Connect)
    3: dns error
    4: failed to lookup address information: Temporary failure in name resolution
```
exit `101` at `end 2026-09-18T16:22:28Z`.

`target/37-02/interrupted-20260918T1622Z/semver-paladin-web.log` holds only
`start 2026-09-18T16:22:28Z` — the reboot killed the process before it printed anything further, and
no `.exit` file was ever written for it.

**No semver verdict was produced for `paladin-storage`, `paladin-notifications`, `paladin-content`,
or `paladin-web`.** Every one of the four failures traces to the loss of DNS resolution to
`index.crates.io`, not to any lint, assertion, or code-shape finding — no `cargo semver-checks`
lint ever evaluated a diff for these four packages.

**Classification and its provenance:** the orchestrator stopped at the workflow's safe-resume gate
(D-14 temperament) and put the question to the maintainer through the runtime's interactive
question mechanism (`AskUserQuestion`), offering three options: "Record, then full re-run
(Recommended)" / "Record, re-run only the 4" / "Treat as a D-14 stop". **The maintainer selected,
verbatim option label: "Record, then full re-run (Recommended)".** The reading the maintainer
accepted: these four invocations are classified **not-measured**, plainly not red — a red gate
requires an actual `cargo semver-checks` verdict that failed an assertion, and none of these four
ever reached that point. Re-running them is therefore not "re-running a locally-red gate hoping for
a different answer" (which D-14/D-15 forbid); it is completing a measurement an environment fault
prevented from ever producing a verdict. This authorization covers exactly **one** re-run of the
full 11-package semver loop for this specific outage. It does not relax D-14 for anything else in
this phase.

**Where the preserved logs live:** the complete interrupted run — all 11 packages' `.log`/`.exit`
pairs (or their absence, for `paladin-web`), plus `semver-loop.log` and `semver-loop.pid` — was
moved, unmodified, to `target/37-02/interrupted-20260918T1622Z/` before any re-run was started, per
the maintainer's decision. `target/37-02/semver-loop.sh` itself (the script that will be re-run) was
left in place, unmodified.

The re-run this finding authorizes, and its result, are recorded as gate row 4 in the Local sweep
below (Task 3 continuation), dated separately.

### Plan 37-02, Task 1 — zero documentation-phase topic counts (2026-09-18)

Local sweep rows 11 and 13 recorded `0` for the `rustdoc` and `intra-doc` topic-count readings
inside the `[0.10.0]` CHANGELOG section, against the plan's own four documentation-phase topics
(`mdBook`, `rustdoc`, `examples`, `intra-doc`). **Per the plan's action text this is explicitly not
a red gate** — D-06 gate row 7's binding machine checks are `make check-changelogs` and
`scripts/check-release-consistency.sh` clause 2 (both hard-asserted green in rows 3-4 above), not
these topic-count readings. Recorded for the maintainer's awareness only; `CHANGELOG.md` was not
edited. A spot grep of the `[0.10.0]` section's own `### Documentation` subsection (see rows 458,
476-499 for line references) shows substantial rustdoc/intra-doc-adjacent prose (e.g. "The
generated API documentation now builds warning-free," "a new code-quality gate keeps every future
[doctest] from shipping without one") that does not literally contain the strings `rustdoc` or
`intra-doc` — a wording gap, not a missing-content gap; left as-is per D-14 (this phase does not
edit CHANGELOG.md prose to make a topic grep pass).

### Maintainer acts and statements, 2026-09-18 (post-CI, pre-tag) — plan 37-06 continuation

**Scope note.** Plan 37-06 Task 3 is a `checkpoint:human-verify` (`gate="blocking-human"`) whose
own stated scope was narrower than what follows: confirm the PR reads correctly, then wait for
`ci.yml` to conclude. Between that checkpoint's dispatch and this continuation, the maintainer
acted well beyond the checkpoint's resume signal alone — merging the PR, resolving the D-17
bootstrap, and delivering the §11 pre-tag sign-off in the same session. All four statements are
recorded here, in one dated entry, rather than left to live only in chat, because the phase's own
`` `` evidence discipline (D-00d, D-05) treats chat as non-durable. Each classification below is
labelled as the **orchestrator's** reading; nothing is attributed to the maintainer as their own
words except what appears inside quotation marks.

**Independently re-verified facts (this continuation, read-only, 2026-09-18):**

- `gh run list --branch feature/phase-33 --workflow ci.yml --json databaseId,event,headSha,status,conclusion`
  filtered to head `1bb9406343b7fb965e0724ac7a689d45e4e5f61c`: both rows `status: "completed"`,
  `conclusion: "success"` — run `35382874018` (`push`) and run `35382953376` (`pull_request`).
- `gh pr view 55 --json state,mergeCommit,mergedAt,baseRefName,headRefName`: `state: "MERGED"`,
  `mergedAt: "2026-09-18T21:21:45Z"`, `mergeCommit.oid: "1d4a9724cc219b85856a23012543458d62559e47"`,
  base `main`, head `feature/phase-33`.
- `git cat-file -p 1d4a9724cc219b85856a23012543458d62559e47`: two `parent` lines
  (`8ed14aea05e9e5bca9695211da4c4ad6991e307b`, `1bb9406343b7fb965e0724ac7a689d45e4e5f61c`) — a true
  merge commit, not a squash or rebase, satisfying D-04 despite the order deviation. `git
  rev-parse 1d4a9724...^{tree}` equals `git rev-parse 1bb94063^{tree}` —
  `d5e056d87a5716a7ec82f00805ff44069b6aff2e` both — the merge commit's tree is byte-identical to
  the re-sealed PR head's tree, because `main` had not moved since the PR was opened. This is the
  mitigation for the order deviation: nothing landed on `main` that the local seven-gate re-seal
  and the CI run did not already cover.
- `gh api repos/DF3NDR/paladin-dev-env/commits/1bb9406343b7fb965e0724ac7a689d45e4e5f61c/check-runs`
  filtered to `name == "CodeQL"`: `conclusion: "failure"`, check-run id `105726197798`, summary
  "10 new alerts including 10 high severity" (all `rust/cleartext-logging`).
- `gh pr checks 55 --required --json name,state,bucket,workflow`: **44 unique required check
  names**, none named `CodeQL`; of the (duplicated across the `push` and `pull_request` trigger
  events) 87 required check-run entries this query returned, 85 `pass` and 2 `skipping` (both
  `End-to-End Tests`, a conditional job not gating this PR), **zero required-check failures**. This
  independently confirms the "44 required, none CodeQL" reading below is not merely repeated from
  the orchestrator's own earlier report.
- `curl -s -H 'User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)'
  https://index.crates.io/pa/la/paladin-eval`: HTTP `200` (was `404` at plan 37-01's pre-bootstrap
  baseline and at plan 37-06 Task 3's own post-checkpoint re-check above). Body:
  `{"name":"paladin-eval","vers":"0.0.1","deps":[],"cksum":"5652e4e010dbaf91c1d84896b119896b24ca550a168fd7dc4b0e50e58d996656","features":{},"yanked":false,"pubtime":"2026-09-18T21:17:04Z"}`.
- `https://crates.io/api/v1/crates/paladin-eval` (same User-Agent): `published_by.login: "Am0rfu5"`;
  `/owners` sub-resource: sole owner `Am0rfu5` — identical to `paladin-llm`'s own `/owners`
  response, independently queried the same way. The `0.0.1` version's `trustpub_data` field reads
  `null` — expected under a token-published placeholder, per the maintainer's own D-17 runbook
  text; the observable non-null proof of the Trusted Publisher link is deferred to `0.10.0`
  (plan 37-10), not this placeholder.

**(a) Task 3 resume signal.** Presented via the runtime's interactive question mechanism
(`AskUserQuestion`; options "Approved" / "Approved, haven't read it closely" / "Needs changes");
the maintainer answered in free text, recorded verbatim:

> "Approve and I merged already."

**Orchestrator's reading:** the resume signal is `approved`; the maintainer additionally reports
having merged PR #55 themselves, ahead of D-03's assumed order (local re-seal → push → PR CI green
→ evidence appended → §11 tick → tick pushed → CI re-run on true final SHA → merge). This is a
**process-order deviation from D-03**, made by the maintainer, their prerogative to make. It is
recorded plainly, neither softened nor dramatised: the merge happened; D-04 (merge-commit method)
was honoured per the two-parent commit and tree-identity check above; nothing was lost, because
`main` had not moved and the tree the maintainer merged is byte-identical to the re-sealed,
CI-green PR head.

**(b) CodeQL results check.** The orchestrator diagnosed the failing check read-only (data restated
above under "Independently re-verified facts"): ten `rust/cleartext-logging` alerts (#31, #32,
#33, #38, #40, #43, #44, #45, #46, #47), every one created `2026-08-27` and already open on `main`
before this PR — GitHub's own summary states alerts "not introduced by this pull request might have
been detected because the code changes were too large" (PR #55 carried 521 commits). Not a
required context (44 required names, none `CodeQL`; no `code_scanning` ruleset on `main`);
`CLAUDE.md`'s own record treats CodeQL as advisory-only (see `.github/instructions/security.instructions.md`,
"Known gap: no Rust SAST"). Phase 37 touched no file outside `.planning/` and `.project/`, so none
of the ten alerts can be attributed to this phase's own diff. Three options were put to the
maintainer via `AskUserQuestion` ("1. proceed and record as advisory" / "2. triage-dismiss first" /
"3. treat as a stop"). The maintainer's reply, recorded verbatim:

> "We are going to proceed with 1."

**Orchestrator's reading:** proceed, record as advisory — consistent with the project's own
existing CodeQL-advisory-only posture. **The ten alerts are carried forward as a named finding for
v0.11.0 triage; this record does not classify them as false positives, and no alert was dismissed,
triaged, or otherwise acted on by any agent in this session.**

**(c) D-17 `paladin-eval` bootstrap.** This statement **supersedes the earlier "deferred" record**
in the Registry verification (D-08) section above **without editing it** — that entry, and the
orchestrator's `deferred` classification of the maintainer's earlier reply, stay exactly as
written; this is a dated, appended update, not a correction. The maintainer's step-0 finding,
recorded verbatim:

> "No way to register a pending publisher.   There is no Trusted Publishing from Account Settings."

**Orchestrator's note (not the maintainer's words):** crates.io configures Trusted Publishing
per-crate, on a crate that already exists in the registry — which is exactly why a never-published
crate cannot have one configured ahead of its first publish. This raises D-17's own MEDIUM-confidence
premise (a blog post plus search synthesis, per `37-CONTEXT.md` D-17) to maintainer-confirmed for
this repository. The maintainer published the placeholder from a scratch directory outside this
repository (`/tmp/tmp.ReoF4L6ehW/paladin-eval`), supplying the crates.io token via a hidden
`read -rs` prompt never shown to any agent — no credential text appears anywhere in this record.
The maintainer's step statements, recorded verbatim:

> "4. Trusted Publisher completed."

> "5. Revoked"

> "bootstrapped 0.0.1"

The independent registry verification this continuation performed (restated above under
"Independently re-verified facts") confirms the sparse-index and crates.io-API-visible half of
this: HTTP `200` (was `404`), `vers=0.0.1`, `yanked=false`, `deps=0` (`0` entries in the `deps`
array), `cksum=5652e4e010dbaf91c1d84896b119896b24ca550a168fd7dc4b0e50e58d996656`; `published_by`
and sole `/owners` entry both `Am0rfu5`, matching `paladin-llm`'s own owner. **The Trusted
Publisher link itself is not publicly queryable** — crates.io does not expose it over any
unauthenticated endpoint this continuation could reach — so it is recorded here as **"linked
(reported by maintainer)"**, exactly the same evidentiary status the existing eleven Trusted
Publishing rows already carry in `docs/src/appendix/release-automation.md`, never upgraded to
"verified" by this record. The observable, independently-checkable proof of the link is a non-null
`trustpub_data` value on `paladin-eval` `0.10.0` at the real release (plan 37-10's own scope) —
this placeholder's `trustpub_data` reads `null`, which is the expected shape for a token-published
version and is not itself evidence for or against the Trusted Publisher link.

**(d) §11 sign-off.** The orchestrator presented a sign-off brief covering: the merge-commit
subject (`1d4a9724...`, tree `d5e056d8...` identical to the re-sealed PR head); the seven D-06
gate rows and the 31 Local sweep rows plus corpus audit §12, all green; the 44/44 required-context
read; the CI `Coverage` job's own success conclusion with `Lines: 111771/123587 = 90.44%` (against
the 82% ADR-0006 floor) and `Functions: 11785/14096 = 83.61%`; the D-17 bootstrap as reported by the
maintainer; and six carried findings named explicitly — the advisory-only CodeQL red, the two
zero-valued CHANGELOG topic readings (`rustdoc`/`intra-doc`, Local sweep rows 11 and 13), the
DNS-outage semver-checks interruption and its single authorized re-run, the stale "eleven crates"
documentation-currency finding, this entry's own process-order deviation (merge before §11 tick),
and the fact that `ci.yml` on the merge commit (run `35396397097`) was still `in_progress` at the
time this sign-off was sought. The mechanism was the maintainer's own choice, offered via
`AskUserQuestion`; the maintainer selected the option whose verbatim label read: "Sign in-session
now, tick rides chore PR (Recommended)". The maintainer's sign-off statement, recorded **verbatim,
including its own stray trailing quotation mark, reproduced exactly as replied** (no leading quote
mark was present in the reply; none is added here):

> I sign §11: the v0.10.0 tag may be cut on 1d4a9724, with the carried findings as recorded."

**This in-session statement is the pre-tag sign-off of record for this head.** The physical `- [x]`
tick on the §11 box in `.project/v0.10.0/09-program-acceptance-audit.md` remains the maintainer's
own hand edit, still to be made — on `chore/37-close`, per D-10 and per `.planning/decisions`
D-00a ("only the maintainer ticks the §11 sign-off box — never an agent, under any mode"). **No
agent, including this continuation, has touched or may touch any sign-off box** — the box in
`09-program-acceptance-audit.md` reads `- [ ] **The \`v0.10.0\` tag may be cut**`, unticked, at the
time this entry is written, and neither that file nor `29-ACCEPTANCE-AUDIT.md` was opened for
writing by this continuation.

**Summary of this entry's own scope, for a later reader:** this is a **record-only** continuation
dispatch. No push, no tag, no PR write, no workflow dispatch, and no gate re-run were performed
here — only the read-only re-verifications listed above, this evidence-file append, the
`.continue-here.md` supersession note (below), and the plan's own SUMMARY/STATE/ROADMAP updates.

---

## Summary and what remains

**What this plan (Task 1) proves:** the whole release-evidence path works end to end on one real,
non-rehearsed slice — a real pre-flight measurement block, a real Local sweep row 1 (captured
count, not exit-code-inferred), a real pre-bootstrap registry read for `paladin-eval`, and real
read-only `gh` reads for the PR and run state of `feature/phase-33`. Nothing here was simulated;
Task 2 and Task 3 build on these exact same rows rather than re-measuring from scratch.

**What remains:** Task 2 (this plan) hands the `paladin-eval` first-publish bootstrap to the
maintainer at a blocking-human checkpoint. Task 3 (this plan) records the post-checkpoint registry
state, the D-13 non-dispatch decision, and the first carried findings. The full D-06 seven-gate
re-seal sweep, the D-01 push/PR, the D-02 merge+tag checkpoint, the D-08 full registry table, the
real release run, and the MILESTONES.md entry are all later plans' work, not this plan's.

---

**Addendum — 2026-09-18, plan 37-03 (append-only, D-00d; nothing above this addendum is edited).**
This addendum supersedes nothing above — it records what plan 37-03 (not plan 37-01) proves, at
the point the Local sweep closes.

**What this record proves:** every one of D-06's seven gate rows now has a Local sweep entry
(rows 1-31 above — plan 37-02 supplied gate rows 1, 2, 3, 4, 5 and 7; this plan supplies gate row
6, the heaviest and last of the seven, plus the two adjacent house-sweep checks). All 31 numbered
rows are either an unconditional local pass (29 rows) or a named, non-blocking carried condition
(2 rows, both pre-existing CHANGELOG wording gaps, neither a D-14 stop) — 0 rows are labelled a
local pass for anything CI-attributed. Concretely, on the phase's local head SHA
(`ed0d3b06`/`d2db617b`, byte-identical source tree throughout Tasks 1-2 of this plan): the full
`release-check` chain (`clean-code` → workspace tests, 40/40 `test result:` lines `ok`, 0 failed →
doctests → `cargo audit`, 10 allowed pre-existing warnings, no new advisory → `build-release`, a
real 13m 12s release-profile build) passed; `cargo publish --workspace --dry-run` packaged and
verified **12** crates in dependency order with `paladin-doc-examples` correctly absent from the
uploaded set; `make security` (`cargo audit` + `cargo deny check`) passed with the identical
allowed-warning count and the `advisories ok, bans ok, licenses ok, sources ok` verdict; `make
api-surface` extracted 3959 items with zero drift against the committed baseline. The whole tree
stayed clean (`git status --porcelain` empty) throughout, and `.project/current-exports.txt` was
never regenerated.

**What this record does NOT claim (two paragraphs, per this plan's own instruction):**

*(a) The 82% workspace line-coverage floor (ADR-0006) is CI-attributed, not a local pass, and this
record makes no claim otherwise.* Docker is absent from this devcontainer — `docker: command not
found`, confirmed structurally in `09-program-acceptance-audit.md`'s own §11 reading and unchanged
here — so `make coverage` (`cargo llvm-cov --workspace --features integration-tests,llm-all --lcov
--output-path lcov.info --fail-under-lines 82 -- --test-threads=1`) cannot complete locally in this
environment; it was not attempted by this plan, exactly as neither plan 37-02 nor plan 37-01
attempted it. The CI `coverage` job on the pre-merge PR run is the **sole evidence source** for
this gate (D-00e) — no local figure, rounded, re-derived, or otherwise, stands in for it anywhere
in this file. Plan 37-07 is the plan that records that job's conclusion and its printed percentage
**to the exact digits CI prints**, and per D-14 a job conclusion other than success is a stop there
even if the printed number would read above the floor by eye.

*(b) No CI run on the final pre-merge SHA or on the `main` merge commit is claimed by this file
yet.* The `CI-run table` section below this addendum still carries only the read-only `gh` query
results plan 37-01 recorded against SHAs that predate this phase's own commits — proof that an
earlier point on this branch was fully green across the pushable workflows, not proof of anything
about the local re-seal work this plan and plan 37-02 just performed. The real pre-merge PR-head
run is plan 37-07's; the post-merge run on the tagged `main` merge commit is plan 37-09's. Neither
has happened as of this addendum.

---

*Phase: 37-v0-10-0-crate-release*
*Written: 2026-09-18*

### Orchestrator correction before the PR opened — `SHIP-05` traceability cell (2026-09-18)

- **Plan defect, corrected under the maintainer's authority, not silently.** Plan 37-04's action
  text, acceptance criterion and `<automated>` grep all mandated the literal traceability row
  `| SHIP-05 | Phase 37 | Complete |`, and its executor wrote exactly that (`7c413842`), flagging
  the oddity in its hand-back. The cell is a requirement-status column (earlier phases carried
  `Pending`, `Not started` and `Gaps Found` there; `phase.complete` flips `Pending` to `Complete` at
  close), so `Complete` asserted a release that has not happened — against prohibition P1 and D-09's
  "no passing by promise". The orchestrator stopped before plan 37-06 and asked the maintainer
  through the interactive question mechanism (`AskUserQuestion`; options "Set it to Pending now
  (Recommended)" / "Leave as the plan wrote it" / "Pause here"). Maintainer's selection, verbatim:
  "Set it to Pending now (Recommended)". `.planning/REQUIREMENTS.md` line 679 now reads
  `| SHIP-05 | Phase 37 | Pending |`; the `[ ] SHIP-05` definition row and every `SHIP-04` line are
  untouched. Consequence for re-validation: plan 37-04's own `<automated>` grep for the `Complete`
  literal no longer matches by design — read this entry, not a regression. Timing reason: after
  the release PR opens every commit costs a full CI cycle, and D-03 makes the §11 tick the last
  content commit on the branch.

---

### Plan 37-08 — Task 1 resolved by deviation; Task 2 read-only post-merge verification and the `paladin-eval` pre-tag gate re-check (2026-09-18)

**Task 1 — resolved by deviation, not presented as a checkpoint.** Per this dispatch's own
`<reality_delta>`, plan 37-08's Task 1 (`checkpoint:human-verify`, maintainer ticks §11) is not
run as written: the maintainer merged PR #55 before any tick commit existed (recorded above,
"Maintainer acts and statements, 2026-09-18," subsection (a)). The prior continuation's Task 3
sign-off brief already obtained the maintainer's pre-tag sign-off of record — subsection (d) of
that same entry, mechanism `AskUserQuestion`, the maintainer's chosen option verbatim-labelled
"Sign in-session now, tick rides chore PR (Recommended)", sign-off statement quoted there in
full. **That in-session statement is the pre-tag sign-off of record for merge commit
`1d4a9724cc219b85856a23012543458d62559e47`.** The physical `- [x]` tick remains the maintainer's
own hand edit, deferred to `chore/37-close` (D-10, D-00a). This dispatch authored no edit to
`.project/v0.10.0/09-program-acceptance-audit.md` and did not open it for writing at any point —
confirmed by `git status --porcelain` throughout. No push of any kind was made or attempted; the
remote `feature/phase-33` branch is confirmed deleted (`gh api
repos/DF3NDR/paladin-dev-env/branches/feature/phase-33` → `404 Branch not found`), so no push
target even exists.

**Task 2 — read-only verification, re-run against post-merge reality (not the plan's assumed
post-tick reality).** Several of the plan's own checks have no subject because no tick commit
exists; each is marked N/A-by-deviation below with its reason, never faked.

**a. §11 box state on `origin/main`.**
`git show origin/main:.project/v0.10.0/09-program-acceptance-audit.md | grep -n '...'` → line
1549 still reads `- [ ] **The \`v0.10.0\` tag may be cut**` — unticked; the in-session sign-off of
record (Task 1 above) stands in its place. File-wide, descriptive only, no assertion made on it:
`grep -c '^- \[ \]'` → `8`; `grep -c '^- \[x\]'` → `0` — unchanged from the established baseline
(8 open maintainer sign-off boxes across the file, 0 ticked); the other seven boxes remain the
maintainer's independent discretion, untouched by this record.

**b. PR #55 state.** `gh pr view 55 --json state,mergedAt,mergedBy,mergeCommit` →
`state: "MERGED"`, `mergedAt: "2026-09-18T21:21:45Z"`, `mergedBy.login: "Am0rfu5"`,
`mergeCommit.oid: "1d4a9724cc219b85856a23012543458d62559e47"`.

**c. Merge commit shape and containment.** `git cat-file -p 1d4a9724cc219b85856a23012543458d62559e47`
→ two `parent` lines (`8ed14aea05e9e5bca9695211da4c4ad6991e307b`,
`1bb9406343b7fb965e0724ac7a689d45e4e5f61c`) — a genuine two-parent merge commit (D-04).
`git merge-base --is-ancestor 1bb9406343b7fb965e0724ac7a689d45e4e5f61c origin/main` → exit `0`
(PR head is an ancestor of `origin/main`). `git rev-parse origin/main` →
`1d4a9724cc219b85856a23012543458d62559e47` — `origin/main`'s tip **equals** the merge commit
exactly; `main` has not moved past it (independently re-confirmed at this dispatch's own fetch,
not merely restated from plan 37-06's continuation).

**d. Tree identity.** `git rev-parse 1d4a9724cc219b85856a23012543458d62559e47^{tree}` =
`git rev-parse 1bb9406343b7fb965e0724ac7a689d45e4e5f61c^{tree}` =
`d5e056d87a5716a7ec82f00805ff44069b6aff2e` — byte-identical (independently re-confirmed at this
dispatch).

**e. The `paladin-eval` pre-tag gate (D-17), re-run immediately before the tag hand-off.**
```
curl -s -o /tmp/pe.json -w '%{http_code}' \
  -H 'User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)' \
  https://index.crates.io/pa/la/paladin-eval
```
→ HTTP `200`, body
`{"name":"paladin-eval","vers":"0.0.1","deps":[],"cksum":"5652e4e010dbaf91c1d84896b119896b24ca550a168fd7dc4b0e50e58d996656","features":{},"yanked":false,"pubtime":"2026-09-18T21:17:04Z"}`
— non-yanked, version `0.0.1`, below `0.10.0`. **Gate PASSES.** This is the third independent
reading of this same fact (plan 37-06's continuation read it first; this dispatch is the second
independent re-run at a later dispatch); the recorded maintainer reply this branches on is now
"bootstrapped 0.0.1" (superseding "deferred," per the entry above), and the 200/non-yanked/
sub-0.10.0 reading on its own already satisfies the gate's first named branch regardless of which
reply is cited. Trusted Publisher link status: **"linked (reported by maintainer)"** — not
independently verifiable over any unauthenticated crates.io endpoint (see the entry above);
restated, not re-verified, here. The four Trusted Publisher fields (`DF3NDR` / `paladin-dev-env`
/ `release.yml` / `crates-io`) are not publicly queryable and are not re-checked by this record —
the Task 3 hand-off asks the maintainer to re-confirm them in the crates.io UI immediately before
the tag push, since no agent check can.

**f. No `v0.10*` tag exists anywhere, and no `release.yml` run newer than the v0.9.0 one.**
`git tag -l 'v0.10*'` → empty. `git ls-remote --tags origin 'v0.10*'` → empty.
`gh run list --workflow release.yml --limit 5 --json databaseId,event,headSha,status,conclusion,createdAt`:
newest run is `33542459191` (`push`, `success`, `2026-09-01T18:14:25Z`, head
`0b5d41063aca8da315603aebb3ce55fdc529964b` — the v0.9.0 tag's own release run); nothing newer.

**g. `ci.yml` on the merge commit — HARD PRECONDITION for the tag, not yet satisfied.**
`gh run view 35396397097 --json databaseId,workflowName,headSha,event,status,conclusion,jobs` →
`headSha: 1d4a9724cc219b85856a23012543458d62559e47`, `event: "push"`, `status: "in_progress"`,
`conclusion:` (empty, not yet concluded). Job tally: **34 `success`, 1 `skipped`, 1
`in_progress`** (`Docker Build` — the same single still-running job this dispatch's own
`<reality_delta>` named at dispatch time). **This run has not concluded as of this record.** The
release pipeline's pre-publish consistency gate (`release.yml`'s `check-release-consistency` job)
queries for a recorded `success`-concluded `ci.yml` run on the tagged SHA itself; per D-12 this
dispatch does not wait or poll for it — it is stated as a hard precondition in the Task 3
hand-off below, with the exact one-line confirmation command:
`gh run view 35396397097 --json status,conclusion`.

**Item 4 (PR mergeability / pending required checks on the "post-tick head") —
N/A-by-deviation.** No post-tick head exists (Task 1 resolved by deviation; no tick commit was
ever created). The PR itself is already `MERGED`, so "mergeability" no longer applies to it. The
one still-open precondition gating the tag is item **g** above (`ci.yml`'s conclusion on the
merge commit), which the Task 3 hand-off states explicitly as a hard precondition.

**Subject-less plan checks, marked N/A-by-deviation (no tick commit exists to inspect):**
- "the tick commit is the last content commit on the branch" — N/A, no tick commit exists.
- "the tick commit's message carries no agent co-authorship trailer" — N/A, no tick commit
  exists to inspect; the maintainer's in-session sign-off statement (Task 1 above) is the
  provenance record in its place.
- "nothing unpushed behind the tick" — N/A, no tick commit exists; separately, this branch's
  ten local-only commits from plans 37-06/37-07 (`eaa07b67` through `cc1e56a7`) plus this
  dispatch's own two commits stay deliberately unpushed — the remote branch is deleted (item
  **Task 1** above) and D-10 routes them to `chore/37-close`, not to `feature/phase-33`.

**This dispatch's own scope, stated plainly.** No push, no tag, no PR write, no workflow
dispatch, no `gh run rerun`, and no `cargo publish` were performed. `.project/v0.10.0/09-program-acceptance-audit.md`
and `29-ACCEPTANCE-AUDIT.md` were not opened for writing. This entry and the
`.continue-here.md` append below are the only changes this dispatch makes, both committed
locally on `feature/phase-33`, neither pushed.

---
