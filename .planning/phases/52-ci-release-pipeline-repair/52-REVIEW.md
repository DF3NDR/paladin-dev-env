---
phase: 52-ci-release-pipeline-repair
reviewed: 2026-10-29T00:00:00Z
depth: standard
files_reviewed: 24
files_reviewed_list:
  - .cargo/semver-checks-allowlist.toml
  - .github/rulesets/protect-main-branch.json
  - .github/workflows/ci.yml
  - crates/paladin-battalion/Cargo.toml
  - crates/paladin-core/Cargo.toml
  - crates/paladin-llm/Cargo.toml
  - crates/paladin-ports/Cargo.toml
  - crates/paladin-web/Cargo.toml
  - docs/src/appendix/branch-protection.md
  - docs/src/appendix/release-automation.md
  - docs/src/appendix/release-checklist.md
  - docs/src/contributing/branching-model.md
  - docs/src/deployment/cicd.md
  - scripts/check-migration-allowlist.sh
  - scripts/check-semver.sh
  - scripts/create-or-reuse-release.sh
  - scripts/finalize-release-body.sh
  - scripts/release-body-fit.sh
  - tests/scripts/check-migration-allowlist_test.sh
  - tests/scripts/check-semver_test.sh
  - tests/scripts/create-or-reuse-release_test.sh
  - tests/scripts/extract-changelog-section_test.sh
  - tests/scripts/finalize-release-body_test.sh
  - tests/scripts/release-body-fit_test.sh
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
status: issues_found
---

# Phase 52: Code Review Report

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 24
**Status:** issues_found

## Summary

Reviewed the Phase 52 bash scripts, their harnesses, the ci.yml trigger and `if:` changes, the ruleset record, the Cargo.toml metadata-table deletions and the mdBook docs. All six shell harnesses pass (20, 70, 49, 16, 129 and 85 assertions). `check-workflow-triggers.sh` passes. Every Cargo.toml parses, and none still carries a `cargo-semver-checks` table. The ruleset JSON has 46 unique contexts, and they match the list in `branch-protection.md`.

Checked and found sound:
- **No release-body or credential leakage.** The body reaches `jq` via `--rawfile` and `gh api` via stdin. `printf` is a builtin and function arguments are not argv. No path logs the response body or text, only sizes. Python runs with `-I`.
- **`check-semver.sh`.** The `N checks:` parse handles ANSI colour between the count and `checks:`; I ran it against a stub with escapes in the line. Zero-check and no-count failures are named. `PIPESTATUS` is captured correctly. Sourcing `publish-crates.sh` through the lib-only seam does not clobber `WORKSPACE_ROOT`.
- **`ci.yml` expressions.** The E2E `if:` does what the docs claim. `contains()` over the labels array is an element match, and the folded `>-` scalar yields a valid expression. `integration-tests` has no job-level `if:`, so `needs:` does not suppress E2E on a labelled PR. The comment about the concurrency groups is now accurate, because `head_ref` and `ref` do produce different groups.

No blockers. The defects below are three robustness or gate-strength warnings and five minor items.

## Warnings

### WR-01: `set -e` is suppressed inside the size-fit functions, so write failures are swallowed and a partial or empty body can be published

**File:** `scripts/release-body-fit.sh:274-276`, `scripts/create-or-reuse-release.sh:241`, `scripts/finalize-release-body.sh:429,457-458,488`
**Issue:** `release_body_fit_for_create` is invoked as `if ! release_body_fit_for_create ...`, and `_frb_fit_composed_body_in` is invoked as `... || rc=$?`. In both contexts bash disables errexit for the whole function body, so every unchecked command is non-fatal. Two cases:
- `cp -- "${in_file}" "${out_file}"` is followed by an explicit `return 0`. If the copy fails (ENOSPC or an unwritable path), the function still reports success. The caller then builds the create payload from an empty or partial `FITTED` file, because `mktemp` already created it. A real release would be created with a truncated or empty body.
- Inside `_frb_fit_composed_body_in`, `printf '%s' ... > "${curated_file}"` and the second `compose_release_body ... > "${output}"` are likewise unchecked. A short write is only caught if it happens to change the measured length, and the final check validates size, not content.

I reproduced the first case. With `set -euo pipefail` sourced, `if ! release_body_fit_for_create in.md /nonexistent/dir/out v1 o/r` printed the `cp` error and then "REPORTED SUCCESS (rc=0)". The CI runner disk can fill during the release job, so this is not purely theoretical.
**Fix:** Check the writes explicitly:
```bash
cp -- "${in_file}" "${out_file}" || {
    echo "ERROR: could not copy the release body to ${out_file}" >&2
    return 1
}
```
Do the same for the `printf ... > "${curated_file}"` redirect. In `_frb_fit_composed_body_in`, run `compose_release_body ... || return 1`, and have `compose_release_body` end its `{ ...; } > "${output}"` block with a status check.

### WR-02: `check-migration-allowlist.sh` silently passes when the `## 11.2 ` heading is missing or renamed

**File:** `scripts/check-migration-allowlist.sh:97`
**Issue:** The register window opens only on `/^## 11\.2 /`. If that heading is renamed, reformatted or deleted, the awk range yields no rows, so the register pair set is empty. The allowlist is now also empty ("EMPTY AS OF PHASE 52"), so two empty sets compare equal and the gate prints success. Any `Y` row in the renamed section is then invisible, and no allowlist entry is demanded for it.

The header says the `## 11.3 ` terminator "MUST exist" and relies on harness case 12d. That terminator failure is loud only because the frozen §10.2 and §9.2 registers still hold `Y` rows. The leading `## 11.2 ` heading has no equivalent guard.

I reproduced it. A MIGRATION.md with `## 11.2b Rust API` and a `Y` row, plus an empty allowlist, exited 0 with "Allowlist is set-equal...". Before this phase the allowlist was non-empty, which masked the problem. Now the gate is tautologically green on that failure.
**Fix:** Require exactly one window-opening heading before comparing, and add a harness case for it:
```bash
if [ "$(grep -c '^## 11\.2 ' "${MIGRATION_FILE}")" -ne 1 ]; then
    echo "ERROR: MIGRATION.md must contain exactly one '## 11.2 ' heading (the open register)." >&2
    exit 1
fi
```
Optionally require a `## 11.3 ` heading in the same way, so the terminator does not depend on frozen rows.

### WR-03: The crate-wide-allow mirror cannot see `warn`, `[workspace.metadata...]`, or non-double-quoted forms

**File:** `scripts/check-migration-allowlist.sh:161-173`
**Issue:** The gate exists so no lint is silenced for a whole crate without a registered `Y` row. The manifest scan only matches section `[package.metadata.cargo-semver-checks.lints]` with a value written exactly as `"allow"` in double quotes. The following all silence or downgrade lints without being mirrored:
- `= "warn"`, which cargo-semver-checks treats as non-failing.
- `= 'allow'`, a TOML literal string.
- A `[workspace.metadata.cargo-semver-checks.lints]` table in the root Cargo.toml. It would apply to every crate and is never read.
- The inline or dotted forms (`lints = { x = "allow" }`, or `lints.x = "allow"` under `[package.metadata.cargo-semver-checks]`).

`check-semver.sh`'s zero-check tripwire does not catch any of these, because checks still run and count. The Phase 52 claim that "no crate-wide allow survives" therefore depends on a scan with blind spots.
**Fix:** Make the legal state "no lint overrides at all". Treat any key in any `cargo-semver-checks` metadata table, package or workspace, as an override that needs a mirrored entry:
```awk
/^[ \t]*\[/ { sec = $0; gsub(/[ \t]/, "", sec); sub(/#.*$/, "", sec); next }
sec ~ /^\[(package|workspace)\.metadata\.cargo-semver-checks\.lints\]$/ && /^[ \t]*[A-Za-z0-9_]+[ \t]*=/ { ... }
```
Drop the `= "allow"` value restriction, and add harness fixtures for `"warn"` and for the workspace table. For the workspace table, the crate side needs a defined mapping (for example, fail outright if the workspace table is non-empty).

## Info

### IN-01: `_frb_build_size_section` reports "within target" for a non-integer size and prints an `[` error

**File:** `scripts/finalize-release-body.sh:281-288`
**Issue:** `--image-size-mb` is documented as an integer but is never validated. With `abc`, `[ "${size_mb}" -gt 500 ]` prints "integer expression expected" to stderr and is false. The body then states "Measured image size: abc MB -- within target", a misleading advisory verdict.
**Fix:** Validate with `[[ "${size_mb}" =~ ^[0-9]+$ ]]` and omit the section, or state "size unavailable", when the value is not numeric.

### IN-02: Every non-zero tool exit is reported as "lint failure"

**File:** `scripts/check-semver.sh:146-147`
**Issue:** The header says exit 100 means a lint fired. The code labels any non-zero status, including 1 (baseline not found or network error), 101 or 127 (cargo missing), as "lint failure (exit N)". The ::error annotation and summary line then misdirect triage for the infrastructure failures this gate is meant to name precisely.
**Fix:** Use `if [ "${status}" -eq 100 ]; then failure="lint failure (exit 100)"; elif [ "${status}" -ne 0 ]; then failure="tool error (exit ${status})"`.

### IN-03: `\x1b` in `sed` is a GNU extension, but `make check-semver` is advertised for local use

**File:** `scripts/check-semver.sh:98`
**Issue:** BSD sed on macOS does not interpret `\x1b`. If a maintainer exports `CARGO_TERM_COLOR=always`, the escapes are not stripped. The `[0-9]+ checks:` match then fails when colour codes sit between the count and `checks:`, producing a false "reported no check count" failure.
**Fix:** Use `sed $'s/\x1b\\[[0-9;]*m//g'`, or `LC_ALL=C sed "s/$(printf '\033')\[[0-9;]*m//g"`.

### IN-04: Temp files are not cleaned up on abnormal exit

**File:** `scripts/check-semver.sh:123`, `scripts/create-or-reuse-release.sh:237`
**Issue:** The semver log (the full tool output per package) and the fitted-body temp file are removed only on the normal path. An interrupt, or a `set -e` abort between `mktemp` and `rm`, leaks them. This is minor on ephemeral runners but real on local `make check-semver` runs on a small disk, which the Makefile comment already notes.
**Fix:** Add `trap 'rm -f "${log}"' RETURN` (or a script-level EXIT trap) in the helper.

### IN-05: Sentinel and marker matching is first-occurrence anywhere, which is latent data loss if the changelog quotes them

**File:** `scripts/release-body-fit.sh:191`, `scripts/finalize-release-body.sh:600`
**Issue:** `finalize` truncates the curated section at the first literal `<!-- paladin:release-artifacts -->` anywhere in the body, including inside a code span. `release-body-fit` strips from the first line that is exactly the truncation sentinel, even inside a code fence. A CHANGELOG section that documents either token would silently lose the rest of the release notes on the next finalize or fit. I grepped CHANGELOG.md and neither token appears today, so this is latent. The Phase 52 docs and the next release's notes are the likeliest place for it to appear.
**Fix:** Anchor the finalize cut to a line that is exactly the marker (`grep -n -m1 -x`), and ignore sentinel lines that fall inside an open fence in the footer-strip step. Alternatively, add a create-time check that rejects a body containing either token.

---

_Reviewed: 2026-10-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
