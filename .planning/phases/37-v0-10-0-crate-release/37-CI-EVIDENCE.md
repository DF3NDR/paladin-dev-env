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
- **PR head at open (D-01):** `pending` — filled in by the plan that pushes the branch and opens
  the release PR.
- **Post-§11-tick final SHA (D-03):** `pending` — filled in after the maintainer ticks §11 in
  `.project/v0.10.0/09-program-acceptance-audit.md` and that tick commit is pushed.
- **`main` merge commit (D-02, D-04):** `pending` — filled in after the maintainer merges the PR
  with the merge-commit method.
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
