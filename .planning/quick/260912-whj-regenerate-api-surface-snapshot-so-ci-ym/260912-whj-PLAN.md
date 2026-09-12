---
phase: quick-260912-whj
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - .project/current-exports.txt
  - CHANGELOG.md
  - crates/paladin-llm/CHANGELOG.md
autonomous: true
requirements:
  - "QUICK-260912-whj: the ci.yml `API Surface Tracking` job passes on feature/v0.10.0-web3sec-dogfooding"
user_setup: []

must_haves:
  truths:
    - "`./scripts/check-api-surface.sh .project/current-exports.txt` exits 0 in the worktree, which is the exact command the ci.yml `api-surface` job runs."
    - "The only content change to the tracked baseline is additive: eight new facade-level Commissary-family re-export lines, the regenerated header timestamp, and the item count moving 3936 -> 3944."
    - "No line is REMOVED from the baseline other than the two lines the check itself filters (the generated-timestamp header and the previous total)."
    - "The baseline still records cargo-public-api v0.52.0, matching what CI installs, so CI's regeneration reproduces this file."
    - "No Rust source file, no script under scripts/, and no workflow under .github/workflows/ is modified."
  artifacts:
    - ".project/current-exports.txt — regenerated public API baseline (3944 items)"
    - "CHANGELOG.md — one Added bullet under the existing `## [Unreleased]` section"
    - "crates/paladin-llm/CHANGELOG.md — one Added bullet under the existing `## [Unreleased]` section"
  key_links:
    - "scripts/check-api-surface.sh filters ONLY `^# Public API Surface - Generated` and `^Total public items:`. Every other line difference fails the job, which is why the diff-shape gate in Task 1 must hold before anything is committed."
    - "CI installs cargo-public-api fresh and got v0.52.0; the devcontainer has 0.52.0. If the local tool were a different version the `# Generated using cargo-public-api vX` line would change, that line is NOT filtered, and CI would still fail."
    - "extract-public-api.sh overwrites its output file in place. A failed or wrong-shaped regeneration leaves a clobbered baseline — `git checkout -- .project/current-exports.txt` is the recovery path."
---

<objective>
Regenerate the tracked public-API baseline `.project/current-exports.txt` so the `API Surface
Tracking` CI job goes green on `feature/v0.10.0-web3sec-dogfooding`, and record the new public
exports in the two Keep-a-Changelog `## [Unreleased]` sections that already exist.

Purpose: CI run 34610398326 on commit 35fd8390 failed ONLY this job. The two Commissary commits
(348f5910, 35fd8390) added eight facade-level re-exports. They are additive, intentional and
non-breaking — Semver Checks and every other job passed. The baseline snapshot simply has not been
refreshed since Phase 28 close.

Output: an updated baseline plus two changelog bullets, in one conventional commit.
</objective>

<execution_context>
@/workspace/.claude/gsd-core/workflows/execute-plan.md
@/workspace/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@/workspace/CLAUDE.md
@/workspace/scripts/check-api-surface.sh
@/workspace/scripts/extract-public-api.sh

This is a generated-artifact refresh. Do NOT edit Rust source, `scripts/*.sh`, or
`.github/workflows/ci.yml` — the tooling is correct, only the snapshot is stale.

The legacy file `api_surface_current.txt` at the repo root is NOT read by the CI check.
Leave it alone.
</context>

<tasks>

<task type="auto">
  <name>Task 1: Regenerate .project/current-exports.txt and prove the diff is purely additive</name>
  <files>.project/current-exports.txt</files>
  <precondition>`cargo public-api --version` reports 0.52.0 and `rustup toolchain list` includes a nightly toolchain — the extraction builds rustdoc JSON for the whole workspace on nightly, and a different cargo-public-api version would rewrite the unfiltered `# Generated using cargo-public-api vX` header line and keep CI red. Halt and report if either is unmet.</precondition>
  <read_first>scripts/extract-public-api.sh, scripts/check-api-surface.sh</read_first>
  <action>
From the worktree root, export CARGO_TARGET_DIR=/workspace/target FIRST so the nightly
rustdoc-JSON build reuses the warm target cache of the main checkout instead of compiling the
whole workspace cold into the worktree. Then run the repo's own generator against the tracked
baseline path:

    export CARGO_TARGET_DIR=/workspace/target
    ./scripts/extract-public-api.sh .project/current-exports.txt

This is the same generator scripts/check-api-surface.sh invokes internally, piped through
scripts/normalize-api-bounds.py, so the output is byte-comparable with what CI computes.

Then INSPECT the resulting diff before trusting it. Run `git diff --stat` and `git diff -U0` on
the baseline and read the output. The expected shape is: the regenerated header timestamp line,
the item total moving from 3936 to 3944, and exactly eight added facade-level re-export lines for
the Commissary prompt-budgeting types — the precise set is enumerated in this task's acceptance
criteria below, and the gate script asserts it mechanically.

Anything outside that shape — any removed line, any added line that is not one of the expected
set, or a changed cargo-public-api version header — is a real, unreviewed API surface change.
STOP and report it in the summary. Do not commit it and do not widen the expected set to make the
gate pass. Recovery is `git checkout -- .project/current-exports.txt`.

Finally run the CI command itself and require a clean exit. Note it re-runs the extraction (a
second rustdoc-JSON pass), which is fast against the warm cache but not instant — do not
interpret a long-running invocation as a hang.
  </action>
  <verify>
    <automated>bash -c 'set -uo pipefail; cd "$(git rev-parse --show-toplevel)"; D=$(git diff -U0 -- .project/current-exports.txt); ADD=$(printf "%s\n" "$D" | grep "^+" | grep -v "^+++" || true); DEL=$(printf "%s\n" "$D" | grep "^-" | grep -v "^---" || true); N=$(printf "%s\n" "$ADD" | grep -c "^+pub use paladin::" || true); UNEXPECTED_ADD=$(printf "%s\n" "$ADD" | grep -vE "^\+# Public API Surface - Generated|^\+Total public items: 3944|^\+pub use paladin::(Commissary|CommissaryError|CommissaryPlan|Consignment|ConsignmentItem|DispensedItem|ShedItem|Stockpile)$" | grep -c . || true); UNEXPECTED_DEL=$(printf "%s\n" "$DEL" | grep -vE "^-# Public API Surface - Generated|^-Total public items: 3936" | grep -c . || true); echo "added_exports=$N unexpected_add=$UNEXPECTED_ADD unexpected_del=$UNEXPECTED_DEL"; [ "$N" -eq 8 ] || { echo "FAIL: expected 8 added exports"; exit 1; }; [ "$UNEXPECTED_ADD" -eq 0 ] || { echo "FAIL: unreviewed added lines"; exit 1; }; [ "$UNEXPECTED_DEL" -eq 0 ] || { echo "FAIL: lines removed from baseline"; exit 1; }; grep -q "^# Generated using cargo-public-api v0.52.0$" .project/current-exports.txt || { echo "FAIL: tool version header changed"; exit 1; }; export CARGO_TARGET_DIR=/workspace/target; ./scripts/check-api-surface.sh .project/current-exports.txt'</automated>
  </verify>
  <done>
`./scripts/check-api-surface.sh .project/current-exports.txt` exits 0.

`git diff -U0 -- .project/current-exports.txt` contains exactly these eight added lines and no
other additions beyond the regenerated timestamp header and `Total public items: 3944`:
`pub use paladin::Commissary`, `pub use paladin::CommissaryError`,
`pub use paladin::CommissaryPlan`, `pub use paladin::Consignment`,
`pub use paladin::ConsignmentItem`, `pub use paladin::DispensedItem`,
`pub use paladin::ShedItem`, `pub use paladin::Stockpile`.

The only removed lines are the previous timestamp header and `Total public items: 3936`. The
header line recording cargo-public-api v0.52.0 is unchanged. No file other than
`.project/current-exports.txt` is dirty.
  </done>
</task>

<task type="auto">
  <name>Task 2: Record the new exports under the existing Unreleased changelog sections</name>
  <files>CHANGELOG.md, crates/paladin-llm/CHANGELOG.md</files>
  <read_first>CHANGELOG.md lines 1-12, crates/paladin-llm/CHANGELOG.md lines 1-12</read_first>
  <action>
Both files already carry a Keep-a-Changelog `## [Unreleased]` heading (line 8 in each) that is
currently empty, sitting directly above `## [0.10.0] - 2026-09-10`. Add an `### Added`
subsection under each `## [Unreleased]` heading with a single bullet.

In `crates/paladin-llm/CHANGELOG.md` the bullet describes the new prompt-budgeting Commissary
service itself, in the crate where it lives (`crates/paladin-llm/src/services/commissary.rs`),
using the project's Medieval Military ubiquitous language.

In the root `CHANGELOG.md` the bullet describes the same capability from the consumer's angle:
the Commissary prompt-budgeting types are now re-exported from the `paladin` facade, naming the
types by their public identifiers.

Keep to the surrounding style: hyphen bullets, sentence case, lines wrapped at roughly 100
columns, no trailing whitespace. Do NOT create a new `## [Unreleased]` heading anywhere, do not
touch the `## [0.10.0]` sections, and do not renumber or reorder any release. This is purely
additive prose under two headings that already exist.
  </action>
  <verify>
    <automated>bash -c 'set -euo pipefail; cd "$(git rev-parse --show-toplevel)"; for f in CHANGELOG.md crates/paladin-llm/CHANGELOG.md; do [ "$(grep -c "^## \[Unreleased\]$" "$f")" -eq 1 ] || { echo "FAIL: Unreleased heading count wrong in $f"; exit 1; }; awk "/^## \[Unreleased\]/{u=1;next} /^## \[0\.10\.0\]/{u=0} u" "$f" | grep -q "^### Added$" || { echo "FAIL: no Added section under Unreleased in $f"; exit 1; }; awk "/^## \[Unreleased\]/{u=1;next} /^## \[0\.10\.0\]/{u=0} u" "$f" | grep -qi "commissary" || { echo "FAIL: Unreleased section in $f does not mention the new service"; exit 1; }; done; git diff --name-only | sort | tr "\n" " "; echo; [ "$(git diff --name-only | grep -cvE "^(CHANGELOG\.md|crates/paladin-llm/CHANGELOG\.md|\.project/current-exports\.txt)$")" -eq 0 ] || { echo "FAIL: unexpected files modified"; exit 1; }'</automated>
  </verify>
  <done>
Each of the two changelogs has exactly one `## [Unreleased]` heading, now followed by an
`### Added` subsection containing one bullet naming the Commissary prompt-budgeting capability.
The `## [0.10.0]` sections are byte-identical to before. The working tree is dirty in exactly
three files: the baseline and the two changelogs.
  </done>
</task>

<task type="auto">
  <name>Task 3: Commit the regenerated baseline and changelog entries</name>
  <files>.project/current-exports.txt, CHANGELOG.md, crates/paladin-llm/CHANGELOG.md</files>
  <action>
Stage exactly the three files this plan touched — do not `git add .`, and specifically do not
stage `api_surface_current.txt` at the repo root if some earlier step disturbed it.

Commit with a conventional-commit message whose subject is
`chore(api): regenerate public API baseline for Commissary exports`, with body lines that state
the item count moved 3936 -> 3944, that the eight added items are additive facade re-exports from
commits 348f5910 and 35fd8390, that this unblocks the ci.yml API Surface Tracking job, and that
reference quick task 260912-whj.

The repo's pre-commit hook runs full-workspace clippy and is slow; `workflow.worktree_skip_hooks`
is true, so commit with `--no-verify`. The orchestrator runs the hooks against the merged tree
afterwards. Do not run `make clean-code` or a workspace clippy pass here — no Rust source changed,
so there is nothing for those gates to say about this commit.
  </action>
  <verify>
    <automated>bash -c 'set -euo pipefail; cd "$(git rev-parse --show-toplevel)"; git status --porcelain | grep -E "current-exports\.txt|CHANGELOG\.md" && { echo "FAIL: changes still uncommitted"; exit 1; }; git log -1 --pretty=%s | grep -q "^chore(api): regenerate public API baseline for Commissary exports$" || { echo "FAIL: subject line mismatch"; exit 1; }; git log -1 --pretty=%B | grep -q "260912-whj" || { echo "FAIL: quick task id missing from body"; exit 1; }; [ "$(git show --name-only --pretty=format: HEAD | grep -c .)" -eq 3 ] || { echo "FAIL: commit does not contain exactly 3 files"; git show --name-only --pretty=format: HEAD; exit 1; }; echo OK'</automated>
  </verify>
  <done>
HEAD is a single conventional commit touching exactly three files, with the prescribed subject
line and a body citing quick task 260912-whj and the 3936 -> 3944 count change. The working tree
is clean with respect to those three paths.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| generated artifact -> merge gate | `.project/current-exports.txt` is the tracked snapshot the `api-surface` CI job diffs against. Regenerating it is the one operation that can make an unreviewed public API change invisible to that gate. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-quick-whj-01 | Tampering | `.project/current-exports.txt` regeneration | medium | mitigate | Task 1's gate asserts the diff is exactly 8 known additions plus the two filtered lines, and fails on ANY removal — a removal is a breaking API change and must not be absorbed silently into a refreshed snapshot. |
| T-quick-whj-02 | Spoofing | cargo-public-api toolchain version | low | mitigate | Task 1 asserts the baseline still records v0.52.0, the version CI installs; a mismatched local tool would produce a snapshot CI cannot reproduce. |
| T-quick-whj-03 | Tampering | npm/pip/cargo installs | low | accept | No package is installed by this plan. `cargo-public-api` 0.52.0 is already present in the devcontainer and is asserted, not installed, by Task 1's precondition. |
</threat_model>

<verification>
1. `./scripts/check-api-surface.sh .project/current-exports.txt` exits 0 (the literal CI step).
2. `git show --stat HEAD` shows exactly three files, all documentation/generated artifacts.
3. `git diff main...HEAD -- src crates scripts .github` is empty for this commit — no source,
   script or workflow change rode along.
</verification>

<success_criteria>
- The `API Surface Tracking` job's command passes locally on a warm cache.
- The baseline grew by exactly eight public items (3936 -> 3944) and lost none.
- Both `## [Unreleased]` sections name the Commissary prompt-budgeting addition.
- One conventional commit, three files, referencing quick task 260912-whj.
</success_criteria>

<output>
Create `.planning/quick/260912-whj-regenerate-api-surface-snapshot-so-ci-ym/260912-whj-SUMMARY.md`
when done. Record the observed item count, the exact diff shape seen, and — if the diff contained
anything outside the expected eight additions — the full unexpected lines, verbatim, instead of a
completion claim.
</output>
