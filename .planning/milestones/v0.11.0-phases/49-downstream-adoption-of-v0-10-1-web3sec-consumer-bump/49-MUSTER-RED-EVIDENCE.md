---
phase: 49
slug: downstream-adoption-of-v0-10-1-web3sec-consumer-bump
status: verified
created: 2026-10-14
---

# Phase 49 — Consumer-surface muster: proven-red evidence

> ADOPT-04 and ROADMAP Phase 49 success criterion 4 require the muster (`tests/consumer_surface_muster.rs`)
> to be "proven red": it must fail when (for example) a `bool` is re-added to `Commissary::new`. A guard that
> cannot fail proves nothing. This file records two valid, scripted mutations, the transcript of each, and the
> restoration of the tree. No mutated source is committed; this file is the only artifact.

Tree: b8b98493648e2c6ce7c5a415570a8773ae45ae1b (git tree object `a44086a8112dc8e0ec54bbd8229f172193333d59`)

Every cargo invocation ran with `CARGO_INCREMENTAL=0` and `--offline`.

A run counts as proof only when all of these hold (checked by counts and presence, never by diagnostic order):

1. the mutation applied (otherwise the script aborts with `MUTATION-NOT-APPLIED`);
2. `cargo check --offline --test consumer_surface_muster` exits non-zero;
3. exactly one `^error: could not compile` line appears and it names `` `paladin-ai` (test "consumer_surface_muster") ``,
   which shows that no library crate failed first;
4. at least one `--> tests/consumer_surface_muster.rs:` location is reported.

## Experiment 1 — Commissary exactness argument re-added

The ROADMAP's own example. `_is_exact_counter: bool` is added after `counter` in `Commissary::new`, and the only two
non-test callers are patched so the library still compiles: `Commissary::from_port` in
`crates/paladin-llm/src/services/commissary.rs` and the `Commissary::new("rag", ..)` call in
`crates/paladin-memory/src/services/rag_retrieval_service.rs`. The remaining `Commissary::new` callers are
`#[cfg(test)]` code and `examples/token_economy_commissary.rs`, neither of which `cargo check --test
consumer_surface_muster` compiles.

Script (`/tmp/49-red-proof/red_proof_commissary.sh`, outside the repository and never committed):

```bash
#!/usr/bin/env bash
# Phase 49 plan 03, Experiment 1: re-add a bool to Commissary::new, patch both non-test library
# callers so the library still compiles, and prove ONLY the consumer-surface muster fails.
# Usage: red_proof_commissary.sh <repository-root>
set -uo pipefail
REPO="${1:?usage: red_proof_commissary.sh <repo-root>}"
cd "$REPO" || exit 2
RED_DIR="${RED_DIR:-${TMPDIR:-/tmp}/49-red-proof}"
LOG="$RED_DIR/commissary.log"
export CARGO_INCREMENTAL=0
FILES=(
  crates/paladin-llm/src/services/commissary.rs
  crates/paladin-memory/src/services/rag_retrieval_service.rs
)

# 1. Baseline guard: refuse to start on a dirty library tree.
if [ -n "$(git status --porcelain -- crates src)" ]; then
  echo "DIRTY-BASELINE"
  exit 2
fi

# 2. Restore on success, failure and interrupt, installed before the first edit.
trap 'git checkout -- crates/paladin-llm/src/services/commissary.rs crates/paladin-memory/src/services/rag_retrieval_service.rs' EXIT

# 3. The three mutations.
perl -0pi -e 's/(counter: Arc<dyn TokenCounterPort>,\n)(        config: CommissaryPlan,\n    \) -> Result<Self, CommissaryError> \{\n        let provider = provider\.into\(\);)/$1        _is_exact_counter: bool,\n$2/' "${FILES[0]}"
perl -0pi -e 's/(llm\.get_capabilities\(\),\n            counter,\n)(            config,)/$1            false,\n$2/' "${FILES[0]}"
perl -0pi -e 's/Arc::clone\(&self\.token_counter\), plan\)\?;/Arc::clone(&self.token_counter), false, plan)?;/' "${FILES[1]}"

# 4. Mutation-applied guard: a pattern that stops matching must abort, never go vacuously green.
if [ "$(grep -c '_is_exact_counter: bool' "${FILES[0]}")" != "1" ] \
  || ! grep -q 'Arc::clone(&self.token_counter), false, plan)?;' "${FILES[1]}" \
  || ! grep -q '^            false,$' "${FILES[0]}"; then
  echo "MUTATION-NOT-APPLIED"
  exit 3
fi

# 5. The check.
cargo check --offline --test consumer_surface_muster >"$LOG" 2>&1
rc=$?
echo "cargo-check-rc=$rc"

# 6. Verdict by counts and presence only, never by diagnostic order.
why=""
[ "$rc" -ne 0 ] || why="$why rc-was-zero"
[ "$(grep -c '^error: could not compile' "$LOG")" -eq 1 ] || why="$why could-not-compile-count-not-1"
grep '^error: could not compile' "$LOG" | grep -qF '`paladin-ai` (test "consumer_surface_muster")' \
  || why="$why could-not-compile-line-does-not-name-muster"
grep -q -- '--> tests/consumer_surface_muster.rs:' "$LOG" || why="$why no-muster-location"
if [ -z "$why" ]; then
  echo "RED-PROVEN"
  exit 0
fi
echo "NOT-PROVEN:$why"
exit 1
```

Run: `bash /tmp/49-red-proof/red_proof_commissary.sh "$PWD"` on the tree above, `df -h .` showing 5.6 GB free beforehand.

Script stdout and exit code:

```text
cargo-check-rc=101
RED-PROVEN
```

- Script exit code: `0`
- `rc` of the `cargo check --offline --test consumer_surface_muster`: `101`
- The `^error: could not compile` count in the log: `1`

The verbatim `could not compile` line:

```text
error: could not compile `paladin-ai` (test "consumer_surface_muster") due to 2 previous errors
```

Every `error[E....]` header with the `-->` location that follows it (`grep -A2 '^error\[' commissary.log`):

```text
error[E0308]: mismatched types
   --> tests/consumer_surface_muster.rs:111:48
error[E0061]: this function takes 5 arguments but 4 arguments were supplied
   --> tests/consumer_surface_muster.rs:125:13
```

The first lands on the fn-pointer pin `let _new: fn(String, ProviderCapabilities, Arc<dyn TokenCounterPort>,
CommissaryPlan) -> Result<Commissary, CommissaryError> = Commissary::new;` and the second on the real
`Commissary::new("literal-str", ..)` call pin, both in the muster. The third `-->` line in the log,
`crates/paladin-llm/src/services/commissary.rs:344:12`, is only the "associated function defined here" note under
`E0061`, not an error location. The log also shows `paladin-eval` and the other library crates reaching
`Checking`, with no library error ahead of the muster.

Post-restore `git status --porcelain -- crates src` (printed between brackets so that empty output is visible):

```text
[]
```

`git diff --quiet -- crates src` exit code: `0`. The muster afterwards: `test result: ok. 1 passed; 0 failed`.

Verdict: RED-PROVEN

## Experiment 3 — facade re-export dropped

`ConsignmentItem` is removed from the `pub use paladin_llm::services::commissary::{..}` group in `src/lib.rs`. The
substitution `\bConsignment, ConsignmentItem, ` only matches inside that group, and `ConsignmentItem` occurred
exactly once in `src/lib.rs` beforehand, so the mutation-applied guard requires it to occur zero times afterwards.
No library crate under `crates/` is touched, so every library crate compiles unchanged.

Script (`/tmp/49-red-proof/red_proof_facade.sh`, outside the repository and never committed):

```bash
#!/usr/bin/env bash
# Phase 49 plan 03, Experiment 3: drop ConsignmentItem from the facade's commissary re-export and
# prove ONLY the consumer-surface muster fails.
# Usage: red_proof_facade.sh <repository-root>
set -uo pipefail
REPO="${1:?usage: red_proof_facade.sh <repo-root>}"
cd "$REPO" || exit 2
RED_DIR="${RED_DIR:-${TMPDIR:-/tmp}/49-red-proof}"
LOG="$RED_DIR/facade.log"
export CARGO_INCREMENTAL=0

# 1. Baseline guard: refuse to start on a dirty library tree.
if [ -n "$(git status --porcelain -- crates src)" ]; then
  echo "DIRTY-BASELINE"
  exit 2
fi

# 2. Restore on success, failure and interrupt, installed before the first edit.
trap 'git checkout -- src/lib.rs' EXIT

# 3. The mutation (it only matches inside the commissary re-export group).
perl -pi -e 's/\bConsignment, ConsignmentItem, /Consignment, /' src/lib.rs

# 4. Mutation-applied guard: ConsignmentItem occurred once before and must now occur zero times.
if [ "$(grep -c 'ConsignmentItem' src/lib.rs)" != "0" ]; then
  echo "MUTATION-NOT-APPLIED"
  exit 3
fi

# 5. The check.
cargo check --offline --test consumer_surface_muster >"$LOG" 2>&1
rc=$?
echo "cargo-check-rc=$rc"

# 6. Verdict by counts and presence only, never by diagnostic order.
why=""
[ "$rc" -ne 0 ] || why="$why rc-was-zero"
[ "$(grep -c '^error: could not compile' "$LOG")" -eq 1 ] || why="$why could-not-compile-count-not-1"
grep '^error: could not compile' "$LOG" | grep -qF '`paladin-ai` (test "consumer_surface_muster")' \
  || why="$why could-not-compile-line-does-not-name-muster"
grep -q -- '--> tests/consumer_surface_muster.rs:' "$LOG" || why="$why no-muster-location"
grep -q 'error\[E0432\]' "$LOG" || why="$why no-E0432"
grep -q 'ConsignmentItem' "$LOG" || why="$why log-does-not-name-ConsignmentItem"
if [ -z "$why" ]; then
  echo "RED-PROVEN"
  exit 0
fi
echo "NOT-PROVEN:$why"
exit 1
```

Run: `bash /tmp/49-red-proof/red_proof_facade.sh "$PWD"` on the same tree (`b8b98493`, tree object
`a44086a8112dc8e0ec54bbd8229f172193333d59`), `df -h .` showing 5.6 GB free beforehand.

Script stdout and exit code:

```text
cargo-check-rc=101
RED-PROVEN
```

- Script exit code: `0`
- `rc` of the `cargo check --offline --test consumer_surface_muster`: `101`
- The `^error: could not compile` count in the log: `1`

The verbatim `could not compile` line:

```text
error: could not compile `paladin-ai` (test "consumer_surface_muster") due to 1 previous error
```

Every `error[E....]` header with the `-->` location that follows it (`grep -A2 '^error\[' facade.log`):

```text
error[E0432]: unresolved import `paladin::ConsignmentItem`
  --> tests/consumer_surface_muster.rs:63:5
```

The log shows `paladin-llm`, `paladin-memory`, `paladin-eval` and the `paladin-ai` facade library all reaching
`Checking` with no error, and the only error is the muster's own `use paladin::{.. ConsignmentItem ..}` at
`tests/consumer_surface_muster.rs:63:5`. This is a different error class (`E0432`, an unresolved import) from
Experiment 1 (`E0308` and `E0061`, signature mismatches), so the two proofs cover independent pins.

Post-restore `git status --porcelain -- crates src`:

```text
[]
```

`git diff --quiet -- crates src` exit code: `0`. The muster afterwards: `test result: ok. 1 passed; 0 failed`.

Verdict: RED-PROVEN

## Not used as proof

Widening the token-usage reasoning builder's parameter type (`TokenUsage::with_reasoning` from `u32` to `u64`) is
49-RESEARCH.md Experiment 2 and is **not** a valid proof. The library's own provider adapters pass `u32` variables
to that builder, so `paladin-llm` fails to compile with `E0308` before the muster target is ever built. The log would
then contain a library-crate `could not compile` line, not the muster's, and would show nothing about the muster. It
is kept here only as the cautionary example behind the "exactly one `could not compile` line, and it names the muster
target" rule that both scripts enforce. It was not run for this plan.

## Trait-implementer pins

`MusterLlm`, `MusterPaladinPort` and `MusterArsenal` implement only the required methods of `LlmPort`, `PaladinPort`
and `ArsenalPort` and are coerced to `Arc<dyn ..>`, so a new required method (`E0046`), a changed signature (`E0053`)
or a loss of object safety fails to compile by construction. These pins are not red-proven by experiment here:
adding a required method to a port trait breaks every in-tree implementer first, so the library would fail before
the muster, which is exactly the vacuous shape the acceptance rule rejects.

## Restoration

Final state after both experiments, taken after the last script returned and after the muster was re-run:

- `git status --porcelain -- crates src`:

  ```text
  []
  ```

- `git diff --stat -- crates src`:

  ```text
  []
  ```

- `git diff --quiet -- crates src` exit code: `0`
- Muster: `CARGO_INCREMENTAL=0 cargo test --offline --test consumer_surface_muster` ended with
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
- Each script installs its `trap 'git checkout -- <mutated files>' EXIT` before its first edit and refuses to start on
  a dirty `crates/` or `src/` baseline (`DIRTY-BASELINE`, exit 2); both scripts live outside the repository.
- Commits of this plan name only this evidence file (and the plan's own planning documents), never a path under
  `crates/` or `src/`.

Verdict: RED-PROVEN (Experiment 1 and Experiment 3; the muster fails alone, at muster locations, and the tree is
restored).
