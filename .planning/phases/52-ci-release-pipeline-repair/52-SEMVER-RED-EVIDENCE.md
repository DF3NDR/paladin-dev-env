---
phase: 52
slug: ci-release-pipeline-repair
status: local-proven
created: 2026-10-29
---

# Phase 52 -- Semver gate: proven-red evidence

> CIREL-01 and ROADMAP Phase 52 success criterion 1 require the re-armed semver gate to be shown able to fail. A gate
> that read `0 checks` for a whole release cycle and still exited 0 must be proven red by a real public-API break, not
> assumed. This file records one scripted mutation, its transcript, and the restoration of the tree. No mutated source
> is committed and the mutation script is kept outside the repository; this file is the only artifact. The CI half of
> the proof (a red `Semver Checks (vs v0.11.0)` job on a real PR run) is recorded below as pending the maintainer's
> decision at plan 52-07's Task 2 checkpoint.

Tree: 5f8e3300b1552f9736e3ca8167816ba687ea197b (git tree object `b026cbc14b1b01a68e58c89b8c69072004a9f020`), branch `claude/laughing-dirac-e0h2ax`

Environment: `cargo-semver-checks 0.50.0`, `rustc 1.97.1 (8bab26f4f 2026-07-14)`, baseline = published `0.11.0` from crates.io, `--release-type patch`, `--default-features` (all fixed inside `scripts/check-semver.sh`). Every gate invocation ran with `SEMVER_PRUNE_LOCAL_CACHE=1`, one package at a time, never a workspace `cargo build`, `cargo test` or `cargo doc`.

A run counts as proof only when all of these hold (checked by counts and presence, never by diagnostic order):

1. the mutation applied (otherwise the script aborts with `MUTATION-NOT-APPLIED`);
2. the `paladin-storage` run exits non-zero;
3. its log contains `function_missing` and `cron_field_count`;
4. its summary line contains ` 1 fail`;
5. the `paladin-eval` and `paladin-ai` runs (the crates that depend on `paladin-storage`) exit 0 with N >= 1 checks;
6. after the script exits, `git diff --quiet -- crates/paladin-storage/src/cron.rs` succeeds and `git status --porcelain` is empty.

## Experiment 1 -- local

The break. `crates/paladin-storage/src/cron.rs:44`, `pub fn cron_field_count(expr: &str) -> usize` becomes
`pub(crate) fn cron_field_count`. This is the primary break chosen at planning time (PD-07-1); the fallback
(`paladin-ports` `current_attempt_spend`) was **not needed** because the primary was detected (conditions 3 and 4 hold).

Why this item is a valid target and not a vacuous one:

- `pub mod cron;` in `crates/paladin-storage/src/lib.rs` (line 102) is always compiled: no feature gate and no
  `#[doc(hidden)]`, so cargo-semver-checks sees the item in its rustdoc JSON under `--default-features`.
- The item is `pub fn` in the published 0.11.0 (`paladin-storage-0.11.0/src/cron.rs:44`), so removing its public
  visibility is a real `function_missing` break against the baseline.
- The in-crate users still compile: `cron.rs:138` (`parse_run_cron`) and `scheduler.rs:90`
  (`crate::cron::cron_field_count`, in the same crate) and the unit tests in `cron.rs`. The plan's planning-time note
  said the only caller was inside `cron.rs`; the workspace grep at execution time also found `scheduler.rs:90`. It is
  in the same crate, so `pub(crate)` keeps it compiling and the finding does not change the proof. No other crate
  references the symbol, and no `paladin-herald` item and no non-default-feature item was used.

Script (`red_proof_semver.sh`, kept in the executor scratchpad outside the repository and never committed):

```bash
#!/usr/bin/env bash
# Phase 52 plan 07, Experiment 1: make paladin_storage::cron::cron_field_count pub(crate) and prove the
# re-armed semver gate goes red on paladin-storage alone, while its dependents stay clean.
# Usage: red_proof_semver.sh <repository-root>
set -uo pipefail
REPO="${1:?usage: red_proof_semver.sh <repo-root>}"
cd "$REPO" || exit 2
RED_DIR="${RED_DIR:-${TMPDIR:-/tmp}/52-red-proof}"
mkdir -p "$RED_DIR"
FILE=crates/paladin-storage/src/cron.rs

# 1. Baseline guard: refuse to start on a dirty tree.
if [ -n "$(git status --porcelain)" ]; then
  echo "DIRTY-BASELINE"
  exit 2
fi

# 2. Restore on success, failure and interrupt, installed before the first edit.
trap 'git checkout -- crates/paladin-storage/src/cron.rs' EXIT

# 3. The mutation: one substitution, public -> crate-private.
sed -i 's/^pub fn cron_field_count/pub(crate) fn cron_field_count/' "$FILE"

# 4. Mutation-applied guard: a pattern that stops matching must abort, never go vacuously green.
if [ "$(grep -c '^pub(crate) fn cron_field_count' "$FILE")" != "1" ]; then
  echo "MUTATION-NOT-APPLIED"
  exit 3
fi
echo "mutation-applied=1"

# 5. The gate, one package at a time, disk read around every run, abort under 200 MB free.
free_mb() { df -Pm / | awk 'NR==2 {print $4}'; }
declare -A RC
for pkg in paladin-storage paladin-eval paladin-ai; do
  before="$(free_mb)"
  echo "disk-before[$pkg]=${before}MB"
  if [ "$before" -lt 200 ]; then
    echo "DISK-LOW-ABORT before $pkg"
    exit 4
  fi
  SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package "$pkg" >"$RED_DIR/$pkg.log" 2>&1
  RC[$pkg]=$?
  echo "exit[$pkg]=${RC[$pkg]}"
  echo "disk-after[$pkg]=$(free_mb)MB"
  if grep -q 'No space left on device' "$RED_DIR/$pkg.log"; then
    echo "NO-SPACE-ABORT in $pkg"
    exit 4
  fi
done

# 6. Verdict by counts and presence only.
why=""
S="$RED_DIR/paladin-storage.log"
[ "${RC[paladin-storage]}" -ne 0 ] || why="$why storage-exit-was-zero"
grep -q 'function_missing' "$S" || why="$why no-function_missing"
grep -q 'cron_field_count' "$S" || why="$why no-cron_field_count"
grep -q '^semver: paladin-storage: .* 1 fail' "$S" || why="$why summary-line-lacks-1-fail"
for pkg in paladin-eval paladin-ai; do
  [ "${RC[$pkg]}" -eq 0 ] || why="$why $pkg-exit-nonzero"
  n="$(grep -Eo '^semver: '"$pkg"': [0-9]+ checks:' "$RED_DIR/$pkg.log" | grep -Eo '[0-9]+ checks' | grep -Eo '^[0-9]+' | tail -n 1)"
  [ -n "$n" ] && [ "$n" -ge 1 ] || why="$why $pkg-no-checks"
done
if [ -z "$why" ]; then
  echo "RED-PROVEN"
  exit 0
fi
echo "NOT-PROVEN:$why"
exit 1
```

Run: `shellcheck --severity=warning` clean first, then `RED_DIR=<scratchpad> bash red_proof_semver.sh "$PWD"` from the
repository root on the tree above, starting from a clean tree (`git status --porcelain` empty).

Script stdout and exit code:

```text
mutation-applied=1
disk-before[paladin-storage]=1615MB
exit[paladin-storage]=1
disk-after[paladin-storage]=1615MB
disk-before[paladin-eval]=1615MB
exit[paladin-eval]=0
disk-after[paladin-eval]=1615MB
disk-before[paladin-ai]=1615MB
exit[paladin-ai]=0
disk-after[paladin-ai]=1615MB
RED-PROVEN
script-exit=0
```

Disk (`df`, free space on `/`): 1.6 GB before the run (`df -h /` showed `1.6G` available, 96% used), 1615 MB before and after every one of the
three packages, and 1.6 GB afterwards. `No space left on device` count in the three logs: `0`, `0`, `0`.

The proof conditions, by counts and presence:

| # | Condition | Observed |
|---|---|---|
| 1 | mutation applied | `mutation-applied=1`; `grep -c '^pub(crate) fn cron_field_count'` was `1` |
| 2 | storage run exits non-zero | `exit[paladin-storage]=1` (the gate's own exit; the tool inside exited 100) |
| 3 | log has `function_missing` and `cron_field_count` | both present: the failure heading and the `Failed in:` line below |
| 4 | summary line has ` 1 fail` | `223 checks: 222 pass, 1 fail, 0 warn, 31 skip` |
| 5 | dependents exit 0 with N >= 1 | `paladin-eval` exit 0, 223 checks; `paladin-ai` exit 0, 223 checks |
| 6 | tree restored | see "Restoration" below |

The failure block from the `paladin-storage` log (ANSI colour stripped; build chatter omitted):

```text
     Checked [   0.018s] 223 checks: 222 pass, 1 fail, 0 warn, 31 skip

--- failure function_missing: pub fn removed or renamed ---

Description:
A publicly-visible function cannot be imported by its prior path. A `pub use` may have been removed, or the function itself may have been renamed or removed entirely.
        ref: https://doc.rust-lang.org/cargo/reference/semver.html#item-remove
       impl: https://github.com/obi1kenobi/cargo-semver-checks/tree/v0.50.0/src/lints/function_missing.ron

Failed in:
  function paladin_storage::cron::cron_field_count, previously in file /root/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/paladin-storage-0.11.0/src/cron.rs:44

     Summary semver requires new major version: 1 major and 0 minor checks failed
    Finished [  39.172s] paladin-storage
```

The gate's summary lines for the broken crate (the line CI's `Semver check counts` notice and `::error` annotation carry):

```text
semver: paladin-storage: 223 checks: 222 pass, 1 fail, 0 warn, 31 skip -- lint failure (exit 100)
semver: 1 package(s) checked against 0.11.0 at --release-type patch; 1 failure(s)
```

The dependents' lines, each with the tool's own `Checking ... (assume patch change)` line (so the `--release-type patch`
pin is visible, not inferred):

```text
    Checking paladin-eval v0.11.0 -> v0.11.0 (assume patch change)
semver: paladin-eval: 223 checks: 223 pass, 31 skip
semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)
    Checking paladin-ai v0.11.0 -> v0.11.0 (assume patch change)
semver: paladin-ai: 223 checks: 223 pass, 31 skip
semver: 1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)
```

What this shows: the red is attributable to the break and to the broken crate alone. The same crate reads `223 checks:
223 pass, 31 skip` at the green baseline, the break moves it to `222 pass, 1 fail`, and the two crates that depend on
it keep their clean counts because the rustdoc of each dependent describes its own public surface only.

Restoration, taken after the script returned:

```text
git diff --quiet -- crates/paladin-storage/src/cron.rs   -> exit 0
git diff --quiet -- crates/                              -> exit 0
git status --porcelain (printed between brackets)        -> []
grep -c '^pub fn cron_field_count' crates/paladin-storage/src/cron.rs -> 1
git ls-files for red_proof_semver.sh                     -> (empty, not tracked)
```

The script installs its `trap 'git checkout -- crates/paladin-storage/src/cron.rs' EXIT` before its first edit and
refuses to start on a dirty tree (`DIRTY-BASELINE`, exit 2); the script lives outside the repository.

Verdict: RED-PROVEN (local). `function_missing` fired on `paladin_storage::cron::cron_field_count` at `--release-type
patch`, `paladin-storage` alone failed, `paladin-eval` and `paladin-ai` stayed clean, and the tree is restored.

## Green baseline

The twelve-package green table is `## D-02 allow-table measurement` in
[`52-CI-EVIDENCE.md`](52-CI-EVIDENCE.md) (plan 52-05, `cargo-semver-checks 0.50.0`, `rustc 1.97.1`, no crate-wide allow
in force, HEAD `95659216ab4a514c3c615194638130f9d9bd9944` when measured). Its twelve lines, quoted:

```text
paladin-ai-core        223 checks: 223 pass, 31 skip
paladin-ports          223 checks: 223 pass, 31 skip
paladin-herald         223 checks: 223 pass, 31 skip
paladin-battalion      223 checks: 223 pass, 31 skip
paladin-llm            223 checks: 223 pass, 31 skip
paladin-memory         223 checks: 223 pass, 31 skip
paladin-web            223 checks: 223 pass, 31 skip
paladin-notifications  223 checks: 223 pass, 31 skip
paladin-content        223 checks: 223 pass, 31 skip
paladin-storage        223 checks: 223 pass, 31 skip
paladin-eval           223 checks: 223 pass, 31 skip
paladin-ai             223 checks: 223 pass, 31 skip
```

`paladin-storage` read `223 checks: 223 pass, 31 skip` there and `223 checks: 222 pass, 1 fail, 0 warn, 31 skip` in
Experiment 1, with the same toolchain, baseline and flags: the only difference is the one visibility edit.

## CI half

Not yet recorded. Plan 52-07 Task 2 is a blocking decision checkpoint (who pushes `scratch/52-semver-red-proof` and the
phase branch and opens the draft PR). Until the maintainer answers, nothing has been pushed and no pull request exists.
`## Experiment 2 -- CI` and `## Close-out` are written by Task 3 after the answer; if the CI half is deferred, it is
handover item H-03.
