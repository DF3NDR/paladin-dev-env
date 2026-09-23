---
phase: 29-program-gates-release
reviewed: 2026-09-10T00:00:00Z
depth: standard
files_reviewed: 41
files_reviewed_list:
  - .github/workflows/ci.yml
  - .project/v0.10.0/00-program-overview.md
  - .project/v0.10.0/09-program-acceptance-audit.md
  - CHANGELOG.md
  - Cargo.toml
  - MIGRATION.md
  - Makefile
  - crates/doc-examples/Cargo.toml
  - crates/paladin-battalion/CHANGELOG.md
  - crates/paladin-battalion/Cargo.toml
  - crates/paladin-content/CHANGELOG.md
  - crates/paladin-content/Cargo.toml
  - crates/paladin-core/CHANGELOG.md
  - crates/paladin-core/Cargo.toml
  - crates/paladin-eval/CHANGELOG.md
  - crates/paladin-eval/Cargo.toml
  - crates/paladin-herald/CHANGELOG.md
  - crates/paladin-herald/Cargo.toml
  - crates/paladin-llm/CHANGELOG.md
  - crates/paladin-llm/Cargo.toml
  - crates/paladin-memory/CHANGELOG.md
  - crates/paladin-memory/Cargo.toml
  - crates/paladin-notifications/CHANGELOG.md
  - crates/paladin-notifications/Cargo.toml
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/Cargo.toml
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-storage/Cargo.toml
  - crates/paladin-web/CHANGELOG.md
  - crates/paladin-web/Cargo.toml
  - crates/paladin-web/openapi.json
  - crates/paladin-web/tests/fixtures/README.md
  - crates/paladin-web/tests/fixtures/openapi-v0.9.0.json
  - crates/paladin-web/tests/openapi_golden_v0_9.rs
  - docs/src/SUMMARY.md
  - docs/src/api-reference/migration-guide.md
  - docs/src/api-reference/upgrading.md
  - docs/src/appendix/release-checklist.md
  - docs/src/operations/observability.md
  - tests/fixtures/config/README.md
  - tests/fixtures/config/v0.9.0-config.example.yml
  - tests/fixtures/config/v0.9.0-config.test.yml
  - tests/integration/v0_9_config_boot_test.rs
findings:
  critical: 0
  warning: 3
  info: 1
  total: 4
status: issues_found
---

# Phase 29: Code Review Report

**Reviewed:** 2026-09-10
**Depth:** standard
**Files Reviewed:** 41 (per `diff_base` 1a7641d9..HEAD scope)
**Status:** issues_found

## Summary

This phase (Program Gates & Release, v0.10.0 close) is docs/tests/CI/release-plumbing only, per
its own X-03 no-behavior-change rule — confirmed: no production `.rs` file changed
(`git diff --stat 1a7641d9..HEAD -- '*.rs'` touches only the two new integration test files,
`tests/integration/v0_9_config_boot_test.rs` and `crates/paladin-web/tests/openapi_golden_v0_9.rs`).

Both new Rust test files were independently verified, not just read: the six restricted v0.9 paths
in `crates/paladin-web/openapi.json` are byte-identical to the frozen `tests/fixtures/
openapi-v0.9.0.json` baseline (confirmed by an independent Python re-implementation of
`restrict_paths`/`ref_closure` against both committed JSON documents — 8/8 schema closure members
match, `components.securitySchemes` matches, `info` sans `version` matches), so
`openapi_golden_v0_9.rs`'s claims are sound and its 6 tests will pass. `v0_9_config_boot_test.rs`'s
26-entry `ALL_PLATFORM_APP_ENV_VARS` list was cross-checked against `MIGRATION.md` §9.5's own
per-struct env-var enumeration and the two counts agree exactly; its 9 `#[test]`/`#[tokio::test]`
functions match the `e2e-platform-api` CI job's `-lt 9` selected-count gate. The `semver` CI job's
new row-level `crate | type` set-equality awk script was traced by hand against the one §9.2 row
that genuinely contains a literal `|` inside a backtick span (the `paladin-ai | Settings` row,
OBS-01/OBS-02) — the field-range scan (6..NF) correctly recovers the marker despite the field
shift, exactly as the step's own comment claims. Version consistency across all twelve publishable
manifests (root + eleven `crates/*/Cargo.toml`) is clean — every `version = "0.10.0"`, all
intra-workspace pins moved together, and the rewritten `Makefile` `publish-dry-run` target
(`cargo publish --workspace --dry-run`) matches the corrected release-checklist doc and the actual
`scripts/publish-crates.sh` dependency order. `MIGRATION.md` carries no unfilled placeholder
marker.

Two real prose/documentation gaps were found (§9.2 below) that a reader relying on these docs as
release-gate evidence would hit: an incomplete crate list in one release-checklist section, and
per-crate `CHANGELOG.md` files whose freshly-stamped `[0.10.0]` sections are empty on ten of eleven
in-tree publishable crates despite MIGRATION.md §9.2 attributing substantial breaking/additive API
changes to several of them (`paladin-core`, `paladin-ports`, `paladin-battalion`, `paladin-llm`,
`paladin-web`). Neither is a behavior or CI-gating defect — `scripts/check-changelogs.sh` only
checks file *presence*, and the release-checklist gap is prose-only — but both directly undercut
this phase's own stated purpose (closing out the release documentation for v0.10.0) and are
first-order findings for a "program gates & release" phase specifically.

## Warnings

### WR-01: Ten of eleven per-crate CHANGELOG.md files have an empty `[0.10.0]` section despite substantial in-crate changes

**File:** `crates/paladin-core/CHANGELOG.md:10`, `crates/paladin-battalion/CHANGELOG.md:10`, and
identically for `paladin-content`, `paladin-herald`, `paladin-llm`, `paladin-memory`,
`paladin-notifications`, `paladin-ports`, `paladin-storage`, `paladin-web` (only
`crates/paladin-eval/CHANGELOG.md` has real content under its `[0.10.0]` heading — expected, since
it is a first-release crate).

**Issue:** `make finalize-crate-changelogs VERSION=0.10.0` (per `.project/v0.10.0/
09-program-acceptance-audit.md` line ~1335) stamped a dated `## [0.10.0] - 2026-09-10` heading into
every publishable crate's changelog, but every one of these ten crates has **nothing** between that
heading and the next (`## [0.9.0] - 2026-09-01`) — confirmed directly: `sed -n '/## \[0.10.0\]/,/##
\[0.9.0\]/p' crates/paladin-core/CHANGELOG.md` yields exactly the two headings and a blank line.
This is despite `MIGRATION.md` §9.2 attributing multiple breaking/additive Rust API changes
directly to several of these crates in this same release — `paladin-core` (`StopReason`,
`BattalionError`, `PaladinError`, `GarrisonEntry`, `PaladinResult`, `Waypoint`), `paladin-ports`
(`LlmError`, `LlmRequest`, five new port traits), `paladin-battalion` (`Commander`/
`CommanderBuilder`, `CampaignExecutionService`), `paladin-llm` (`FallbackLlmAdapter`, the
conformance suite), `paladin-web` (`ThreadApiState`, `ResumeAcceptedResponse`,
`require_authentication`). A downstream consumer who pulls `paladin-core = "0.10.0"` from
crates.io and reads its shipped `CHANGELOG.md` for what changed sees a heading with no content, and
has to go find the root-level `MIGRATION.md`/`CHANGELOG.md` instead — which is not what ships
inside the crate's own package. `scripts/check-changelogs.sh` does not catch this: it only checks
that each publishable crate directory has *a* `CHANGELOG.md` file at all, never that a dated
section under it is non-empty, so this gap is invisible to CI.

**Fix:** Populate each crate's own `[0.10.0]` section with (at minimum) a pointer to the relevant
`MIGRATION.md` §9.2 rows/CHANGELOG.md root entries that apply to that crate, or a short per-crate
summary, mirroring the pattern `paladin-eval/CHANGELOG.md` already uses. Example for
`paladin-core`:
```markdown
## [0.10.0] - 2026-09-10

### Changed
- `StopReason`, `BattalionError`, `PaladinError` gained new `#[non_exhaustive]` variants (typed
  error taxonomy, FT-01). See root `MIGRATION.md` §9.2 for the full register.
- `GarrisonEntry` gained `is_summary: bool` (`#[serde(default)]`, RT-03).
- `PaladinResult` gained `served_by: Option<String>` (FT-05 model fallback).
```

### WR-02: `docs/src/appendix/release-checklist.md` §6 "Publish" list omits `paladin-eval`

**File:** `docs/src/appendix/release-checklist.md:79-90`

**Issue:** §5 ("Dry-Run Publish Validation") correctly lists all twelve publishable crates
including `paladin-eval` (tier 5, between the leaf tier and `paladin-ai`) and even explains why it
is included there despite being excluded from the `semver` CI job. §6 ("Publish"), immediately
below it, lists only eleven crates — `paladin-ai-core`, `paladin-ports`, `paladin-herald`, the
seven leaf-tier crates, and `paladin-ai` — with `paladin-eval` missing entirely from the ordered
list. This contradicts both `scripts/publish-crates.sh`'s actual `CRATES=(...)` array (which places
`paladin-eval` immediately before `paladin-ai`, the array this script and `release.yml` actually
publish from) and this same document's own §5 six lines above it. An operator following §6 by hand,
or auditing the real publish order against this doc, would not know where `paladin-eval` belongs.

**Fix:** Add `paladin-eval` to §6's ordered list in the same position `scripts/publish-crates.sh`
uses:
```markdown
1. paladin-ai-core
2. paladin-ports
3. paladin-herald
4. paladin-battalion, paladin-llm, paladin-memory, paladin-web, paladin-notifications,
   paladin-content, paladin-storage (leaf tier)
5. paladin-eval
6. paladin-ai
```

### WR-03: `v0_9_config_boot`'s new "fewer than 9" CI guard step doesn't defend against its own zero-match edge case, unlike sibling guards in the same file

**File:** `.github/workflows/ci.yml` (step "Fail if the v0_9_config_boot run selected fewer than
the full test set", added in this phase's diff, immediately following the `e2e-platform-api` job's
pre-existing "Fail if the run selected zero tests" step which has the identical gap)

**Issue:** The step is:
```bash
ACTUAL=$(grep -oP 'test result: ok\. \K[0-9]+(?= passed)' /tmp/v0-9-config-boot-test.log | tail -1)
echo "Passed: ${ACTUAL:-0}"
if [ -z "$ACTUAL" ] || [ "$ACTUAL" -lt 9 ]; then
```
GitHub Actions' default `bash` invocation for a `run:` step is `bash --noprofile --norc -eo
pipefail {0}` (pipefail *and* `-e` on by default, with no explicit `set` needed to enable them). If
`grep -oP` finds zero matches (exit 1) while `tail -1` still exits 0 on empty input, `pipefail`
makes the pipeline's exit status 1 (the rightmost non-zero), and `-e` then aborts the script at the
`ACTUAL=$(...)` assignment line — before `echo "Passed:"` or the `if` check ever run, and before the
intended `::error::…` diagnostic prints. The job still fails overall (a bare pipe failure is still
non-zero), so this is not a false-pass risk, but it silently defeats the specific, carefully-worded
diagnostic message this step (and its sibling two steps above it in the same job) was written to
produce — the exact failure category this file's other guards (e.g., the "Verify MIGRATION.md
carries no unfilled placeholder marker" step a few hundred lines above, which uses `grep -c ... ||
true`) are explicit about defending against. In practice this zero-match branch is rarely reached
(a genuine `cargo test` failure would already have failed the preceding `cargo test` step and
stopped the job before this one runs), which is why it is a Warning rather than a Blocker, but it
is an inconsistency worth fixing for the same reason the file's own comments elsewhere call this
class of gap out explicitly.

**Fix:** Mirror the pattern already used elsewhere in this file for a possibly-empty grep, e.g.:
```bash
ACTUAL=$(grep -oP 'test result: ok\. \K[0-9]+(?= passed)' /tmp/v0-9-config-boot-test.log | tail -1 || true)
```
or add `set +e` / an explicit `|| true` on the pipeline before the `if` check.

## Info

### IN-01: `crates/doc-examples/Cargo.toml` does not inherit `rust-version.workspace = true`

**File:** `crates/doc-examples/Cargo.toml:1-6`

**Issue:** Every other workspace member (including the non-publishable-but-versioned
`paladin-doc-examples`'s siblings) inherits the MSRV floor via `rust-version.workspace = true`
(confirmed via `grep -rn rust-version crates/*/Cargo.toml`), but `crates/doc-examples/Cargo.toml`
has no `rust-version` field at all. Since this crate is `publish = false` and exists purely to
compile-check mdBook examples, this is unlikely to cause a real MSRV drift in practice (it's built
in the same workspace resolve as everything else, and the `msrv` CI job runs `cargo check
--workspace --all-features --all-targets`, which covers it), but it is an inconsistency against the
pattern every publishable crate in this phase's diff was checked for.

**Fix:** Add `rust-version.workspace = true` to `crates/doc-examples/Cargo.toml`'s `[package]`
table for consistency, even though it is not required by any current gate.

---

_Reviewed: 2026-09-10_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
