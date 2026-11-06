#!/usr/bin/env bash
# check-release-consistency.sh
#
# The pre-publish consistency gate (PUBOPS-01). Verifies that a release
# tag's version string exactly matches the manifest version of every
# publishable Cargo workspace package, before `publish-crates` is allowed to
# run. PUBOPS-01's binding claim is "no crate is published until the tag,
# every manifest version and every changelog agree" -- this script is the
# single source of truth both the release.yml gate job and the local `make
# check-release-consistency` target invoke; no gate logic is duplicated in
# workflow YAML.
#
# This script is offline when --metadata-json is supplied, and otherwise
# makes exactly one local `cargo metadata` call -- it never talks to the
# network. It accumulates every mismatch into one report rather than
# stopping at the first, so a release with three broken crates gets one
# report naming all three, never a fix-one-rerun-find-the-next loop. It only
# reads: given the same tag and the same metadata, running it twice produces
# byte-identical output and the same exit code.
#
# Clauses implemented so far (D-08 clauses 1-3 of 4; SHA agreement is
# future scope):
#
#   1. Manifest agreement. Every publishable package in the Cargo workspace
#      -- discovered via `cargo metadata --no-deps --format-version 1`,
#      never a hardcoded list -- has a `version` field that is exactly equal
#      (string equality, no semver normalisation or coercion) to the tag's
#      version with at most one leading `v` stripped. `0.8.1` and
#      `0.8.1-rc.2` are different strings and therefore a mismatch. A
#      package is publishable when its manifest's `publish` field is absent
#      (`cargo metadata` reports this as JSON `null`); a package carrying
#      `publish = false` (`cargo metadata` reports this as an empty list)
#      is excluded by design, never counted as a mismatch.
#
#   2. Changelog-section agreement. Every publishable package's own
#      changelog -- `CHANGELOG.md` in the directory containing that
#      package's `manifest_path`, never a hardcoded `crates/*/CHANGELOG.md`
#      glob (the root package's manifest sits at the workspace root, so its
#      changelog is the root `CHANGELOG.md`; every crate resolves to its own
#      file) -- carries a heading matching `## [<exact tag version>]`,
#      optionally followed by a dated suffix (`## [1.2.3] - 2026-01-01`
#      satisfies `--tag v1.2.3`; `## [1.2.3-rc.1]` and `## [1.2.30]` do not,
#      because the match is on the bracketed content exactly, never a
#      prefix). A changelog file that is entirely absent and one that exists
#      but carries no section for this version are two distinct failure
#      messages, never conflated into one.
#
#   3. CI-conclusion agreement (PUBOPS-02, D-10). The tagged commit (--sha)
#      must have a recorded, successful `ci.yml` run. `--ci-runs-json`, when
#      supplied, is read instead of calling the API -- the same fixture seam
#      pattern as --metadata-json, and the parsing code path is identical to
#      what a live `gh api` response takes. Otherwise this script calls
#      `gh api repos/${GITHUB_REPOSITORY}/actions/workflows/ci.yml/runs`
#      with `head_sha=<sha>` and `status=completed`, paginated. Assumption
#      A2 (that endpoint accepts the workflow filename in the path) is
#      *removed*, not assumed: a clean 404 triggers a fallback that resolves
#      `ci.yml`'s numeric workflow id from
#      `repos/${GITHUB_REPOSITORY}/actions/workflows` (matching `path`
#      ending in `/ci.yml`) and retries against that id. Any other lookup
#      failure -- transport, authorisation, rate limiting -- is
#      CI_LOOKUP_FAILED, a non-zero exit distinct from "no successful run
#      exists"; it is never silently treated as a pass. The deciding run is
#      selected by sorting the returned array by `created_at` ascending with
#      `id` as the tiebreak, then taking the last element -- never indexing
#      the array unsorted, so two runs sharing a timestamp still resolve
#      deterministically to the higher id, and an older success followed by
#      a newer failure on the same SHA fails the clause.
#
#      Granularity: the whole-run conclusion, not a named job subset. This
#      is re-derived from `ci.yml` each time this comment is read, not
#      carried on trust: on a push to `main` (the only event that produces
#      the run this clause reads), the single job carrying a job-level
#      `continue-on-error` is `benchmark-regression-signal`, whose `if:`
#      restricts it to `pull_request` and `workflow_dispatch` -- it does not
#      run at all on the push this clause inspects. `osv-scanner`'s
#      tolerance is step-level (`continue-on-error: true` on individual
#      steps), so the job itself still gates. Every other job in `ci.yml` is
#      hard-gating. There is therefore no job that is simultaneously
#      known-flaky, release-irrelevant, and able to turn the run red -- the
#      condition that would make whole-run granularity unusably strict. The
#      `release.yml` Build Binaries flakiness recorded in Phase 19's
#      evidence lives in a different workflow and does not bear on this
#      run's conclusion. If a non-blocking job is ever added to `ci.yml`
#      that can fail the run, this clause must be revisited rather than
#      loosened ad hoc.
#
#      MISSING_SHA: when `GITHUB_ACTIONS=true` and no `--sha` was supplied,
#      the whole gate fails closed with this token before any clause runs --
#      the CI-conclusion clause can never be silently absent on the CI path.
#      Outside GitHub Actions, an absent `--sha` still runs clauses 1-2 and
#      states explicitly in the report that clause 3 was not checked, so a
#      local pass is never misread as a full-gate pass.
#
#      No `-L`/`--location` (or any other redirect-following option) is
#      ever passed to `curl` or `gh` here, per the credential-header control
#      in `security.instructions.md`.
#
# Discovering zero publishable packages is a named ZERO_PACKAGES failure
# with a non-zero exit -- a broken enumeration must never present as a pass
# over an empty set, matching the convention `check-changelogs.sh` and
# `check-workflow-triggers.sh` already establish for this repo's gate
# scripts.
#
# Sourcing seam: set CHECK_RELEASE_CONSISTENCY_LIB_ONLY=1 before sourcing
# this file to load the check_release_consistency_main function without
# executing it -- this file's own regression harness (and later plans'
# harnesses) use this to exercise the function directly.
#
# Usage:  ./scripts/check-release-consistency.sh --tag <vX.Y.Z|X.Y.Z>
#             [--metadata-json <path>] [--workspace-root <path>]
#             [--sha <sha>] [--ci-runs-json <path>]
#         --tag is required. --metadata-json, when supplied, is read
#         instead of invoking `cargo metadata` -- the fixture seam this
#         script's regression test uses so it never invokes cargo.
#         --workspace-root overrides the repo root `cargo metadata` is run
#         from (default: derived from BASH_SOURCE); it has no effect when
#         --metadata-json is supplied. --sha names the commit clause 3
#         checks; the caller supplies it (release.yml's `--sha` wiring is
#         plan 20-05's `release.yml` pass). --ci-runs-json, when supplied,
#         is read instead of calling `gh api` -- the fixture seam this
#         script's regression test uses so it never touches the network.
#         An unrecognised flag is a usage error.
# Exit:   0 if every publishable package's manifest version exactly equals
#         the tag version, every publishable package's own changelog
#         carries a section for it, AND (when checked) the tagged SHA's most
#         recent completed ci.yml run concluded success (status OK);
#         non-zero for MISMATCH, CHANGELOG_MISMATCH, CI_MISMATCH, any
#         combination of those three joined with `_AND_`, CI_LOOKUP_FAILED,
#         ZERO_PACKAGES, MISSING_TAG, MISSING_SHA, or a usage error
#         (unknown flag / missing python3).

set -euo pipefail

# _crc_fetch_ci_runs SHA OUT_FILE -> populates OUT_FILE with the raw `gh
# api` response body for ci.yml's completed runs at SHA (paginated, written
# as-is -- the python parser is defensive about concatenated pages, so this
# never needs to merge pages itself). Tries the workflow-filename path
# first; on a clean 404 (Assumption A2 removed, not assumed), resolves
# ci.yml's numeric workflow id from repos/{owner}/{repo}/actions/workflows
# and retries against that id. Any other non-2xx / transport / auth
# failure returns non-zero -- CI_LOOKUP_FAILED territory, never silently
# treated as "no successful run". No -L/--location (or any other
# redirect-following option) is ever passed here, per the credential-header
# control in security.instructions.md.
_crc_fetch_ci_runs() {
    local sha="$1" out_file="$2"
    local err_file
    err_file="$(mktemp "${TMPDIR:-/tmp}/check-release-consistency-ci-err.XXXXXX")"

    # --method GET is load-bearing: `-f` fields default `gh api` to POST, and
    # the runs endpoint answers POST with HTTP 404 -- indistinguishable from
    # the filename-path 404 the fallback below exists for. Found live on the
    # v0.8.1-rc.4 rehearsal (Phase 20, 20-07 finding 6).
    if gh api --method GET "repos/${GITHUB_REPOSITORY:-}/actions/workflows/ci.yml/runs" \
        -f head_sha="${sha}" -f status=completed --paginate \
        > "${out_file}" 2> "${err_file}"; then
        rm -f "${err_file}"
        return 0
    fi

    if grep -q 'HTTP 404' "${err_file}"; then
        # ci.yml as a filename 404'd on this repo -- resolve its numeric
        # workflow id and retry against that instead of assuming the
        # filename path is universally accepted (Assumption A2, removed).
        local workflows_file
        workflows_file="$(mktemp "${TMPDIR:-/tmp}/check-release-consistency-workflows.XXXXXX.json")"
        if ! gh api "repos/${GITHUB_REPOSITORY:-}/actions/workflows" --paginate \
            > "${workflows_file}" 2> "${err_file}"; then
            cat "${err_file}" >&2
            rm -f "${err_file}" "${workflows_file}"
            return 1
        fi

        local workflow_id
        workflow_id="$(python3 -c '
import json
import sys

path = sys.argv[1]
with open(path, "r", encoding="utf-8") as fh:
    text = fh.read()

decoder = json.JSONDecoder()
idx = 0
wf_id = ""
while idx < len(text):
    while idx < len(text) and text[idx].isspace():
        idx += 1
    if idx >= len(text):
        break
    obj, end = decoder.raw_decode(text, idx)
    idx = end
    workflows = obj.get("workflows", []) if isinstance(obj, dict) else []
    for wf in workflows:
        p = wf.get("path", "")
        if isinstance(p, str) and p.endswith("/ci.yml"):
            wf_id = str(wf.get("id", ""))
            break
    if wf_id:
        break
print(wf_id)
' "${workflows_file}")"
        rm -f "${workflows_file}"

        if [ -z "${workflow_id}" ]; then
            echo "ERROR: could not resolve ci.yml's numeric workflow id from repos/${GITHUB_REPOSITORY:-}/actions/workflows." >&2
            rm -f "${err_file}"
            return 1
        fi

        if gh api --method GET "repos/${GITHUB_REPOSITORY:-}/actions/workflows/${workflow_id}/runs" \
            -f head_sha="${sha}" -f status=completed --paginate \
            > "${out_file}" 2> "${err_file}"; then
            rm -f "${err_file}"
            return 0
        fi
    fi

    cat "${err_file}" >&2
    rm -f "${err_file}"
    return 1
}

check_release_consistency_main() {
    local WORKSPACE_ROOT
    WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

    local TAG="" METADATA_JSON="" WORKSPACE_ROOT_ARG="" SHA="" CI_RUNS_JSON=""

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --tag)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --tag requires a value." >&2
                    return 1
                fi
                TAG="$2"
                shift 2
                ;;
            --metadata-json)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --metadata-json requires a value." >&2
                    return 1
                fi
                METADATA_JSON="$2"
                shift 2
                ;;
            --workspace-root)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --workspace-root requires a value." >&2
                    return 1
                fi
                WORKSPACE_ROOT_ARG="$2"
                shift 2
                ;;
            --sha)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --sha requires a value." >&2
                    return 1
                fi
                SHA="$2"
                shift 2
                ;;
            --ci-runs-json)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --ci-runs-json requires a value." >&2
                    return 1
                fi
                CI_RUNS_JSON="$2"
                shift 2
                ;;
            *)
                echo "ERROR: unknown flag: $1" >&2
                echo "Usage: check-release-consistency.sh --tag <vX.Y.Z|X.Y.Z> [--metadata-json <path>] [--workspace-root <path>] [--sha <sha>] [--ci-runs-json <path>]" >&2
                return 1
                ;;
        esac
    done

    if [ -n "${WORKSPACE_ROOT_ARG}" ]; then
        WORKSPACE_ROOT="${WORKSPACE_ROOT_ARG}"
    fi

    if ! command -v python3 >/dev/null 2>&1; then
        echo "ERROR: python3 is required for Cargo metadata parsing." >&2
        return 1
    fi

    local REPORT
    if [ -z "${TAG}" ]; then
        REPORT=$'MISSING_TAG\nFAIL: --tag is required. Usage: check-release-consistency.sh --tag vX.Y.Z (or X.Y.Z, or --metadata-json/--workspace-root for local runs).'
    elif [ "${GITHUB_ACTIONS:-}" = "true" ] && [ -z "${SHA}" ]; then
        # The CI-conclusion clause can never be silently absent on the CI
        # path -- a SHA-less run inside GitHub Actions fails closed before
        # any other clause runs, rather than passing a partial gate.
        REPORT=$'MISSING_SHA\nFAIL: running inside GitHub Actions (GITHUB_ACTIONS=true) requires --sha so the CI-conclusion clause (D-10) can run; refusing to run a partial gate. Pass --sha <commit-sha> (release.yml supplies this).'
    else
        # Strip at most one leading "v" -- "${TAG#v}" removes the shortest
        # matching prefix once, so "v1.2.3" -> "1.2.3" and "1.2.3" -> "1.2.3"
        # unchanged. No semver parsing anywhere in this script.
        local TAG_VERSION="${TAG#v}"

        local METADATA_PATH="" CLEANUP_METADATA=""
        local CI_RUNS_PATH="" CLEANUP_CI_RUNS="" CI_MODE="not_checked"
        trap '[ -n "${CLEANUP_METADATA}" ] && rm -f "${CLEANUP_METADATA}"; [ -n "${CLEANUP_CI_RUNS}" ] && rm -f "${CLEANUP_CI_RUNS}"' RETURN

        if [ -n "${METADATA_JSON}" ]; then
            if [ ! -f "${METADATA_JSON}" ]; then
                echo "ERROR: --metadata-json file not found: ${METADATA_JSON}" >&2
                return 1
            fi
            METADATA_PATH="${METADATA_JSON}"
        else
            METADATA_PATH="$(mktemp "${TMPDIR:-/tmp}/check-release-consistency-metadata.XXXXXX.json")"
            CLEANUP_METADATA="${METADATA_PATH}"
            if ! (cd "${WORKSPACE_ROOT}" && cargo metadata --no-deps --format-version 1) > "${METADATA_PATH}"; then
                echo "ERROR: 'cargo metadata --no-deps --format-version 1' failed in ${WORKSPACE_ROOT}." >&2
                return 1
            fi
        fi

        # Clause 3 input resolution. --ci-runs-json (fixture seam) always
        # wins over a live --sha lookup, so a test can pass both (SHA for
        # the message text, --ci-runs-json for the data) without touching
        # the network. With neither flag, clause 3 is not checked at all --
        # CI_MODE stays "not_checked" and the report says so explicitly.
        if [ -n "${CI_RUNS_JSON}" ]; then
            if [ ! -f "${CI_RUNS_JSON}" ]; then
                echo "ERROR: --ci-runs-json file not found: ${CI_RUNS_JSON}" >&2
                return 1
            fi
            CI_RUNS_PATH="${CI_RUNS_JSON}"
            CI_MODE="runs_path"
        elif [ -n "${SHA}" ]; then
            CI_RUNS_PATH="$(mktemp "${TMPDIR:-/tmp}/check-release-consistency-ci-runs.XXXXXX.json")"
            CLEANUP_CI_RUNS="${CI_RUNS_PATH}"
            if _crc_fetch_ci_runs "${SHA}" "${CI_RUNS_PATH}"; then
                CI_MODE="runs_path"
            else
                CI_MODE="lookup_failed"
            fi
        fi

        if [ "${CI_MODE}" = "lookup_failed" ]; then
            REPORT="CI_LOOKUP_FAILED"$'\n'"FAIL: could not resolve the ci.yml conclusion for SHA '${SHA}' -- a transport, authorisation or rate-limit failure querying the GitHub API (see command output above for detail). This is never conflated with \"no successful run exists\"; fix API access and re-run."
        else
        REPORT=$(python3 - "${METADATA_PATH}" "${TAG_VERSION}" "${CI_MODE}" "${CI_RUNS_PATH}" "${SHA}" <<'PY'
import json
import os
import re
import sys

metadata_path = sys.argv[1]
tag_version = sys.argv[2]
ci_mode = sys.argv[3] if len(sys.argv) > 3 else "not_checked"
ci_runs_path = sys.argv[4] if len(sys.argv) > 4 else ""
sha = sys.argv[5] if len(sys.argv) > 5 else ""

try:
    with open(metadata_path, "r", encoding="utf-8") as fh:
        data = json.load(fh)
except (OSError, json.JSONDecodeError) as exc:
    print("MISMATCH")
    print(f"FAIL: could not read/parse metadata JSON at {metadata_path}: {exc}")
    sys.exit(0)

packages = data.get("packages", []) if isinstance(data, dict) else []
publishable = [
    p for p in packages
    if isinstance(p, dict) and p.get("publish") is None
]

if not publishable:
    print("ZERO_PACKAGES")
    print("FAIL: zero publishable packages discovered from cargo metadata -- a broken "
          "enumeration is a named failure, never a pass over an empty set. A package is "
          "publishable when its manifest omits `publish` (cargo metadata reports this as "
          "`null`); one carrying `publish = false` (reported as an empty list) is excluded "
          "by design, not a bug.")
    sys.exit(0)

# Clause 1: manifest version agreement. String equality only, no semver
# coercion -- "0.8.1" and "0.8.1-rc.2" are different strings.
version_failures = []
for p in publishable:
    name = p.get("name", "<unknown>")
    version = p.get("version", "")
    if version != tag_version:
        version_failures.append(
            (name, f"{name}: manifest version {version!r} != tag version {tag_version!r}")
        )

# Clause 2: changelog-section agreement. A package's changelog is
# CHANGELOG.md in the directory containing its own manifest_path -- this is
# what makes the root package (manifest at the workspace root) resolve to
# the root CHANGELOG.md and each crate resolve to its own file, with no
# special-casing and no hardcoded crates/*/CHANGELOG.md glob. The heading
# pattern anchors immediately after the bracketed version so a longer
# version sharing a prefix, or a prerelease variant, never satisfies a
# stable tag.
heading_re = re.compile(r"^##\s*\[" + re.escape(tag_version) + r"\](\s|$)")
changelog_failures = []
for p in publishable:
    name = p.get("name", "<unknown>")
    manifest_path = p.get("manifest_path") or ""
    if not manifest_path:
        changelog_failures.append(
            (name, f"{name}: metadata carries no manifest_path -- cannot locate its changelog")
        )
        continue

    changelog_path = os.path.join(os.path.dirname(manifest_path), "CHANGELOG.md")

    if not os.path.isfile(changelog_path):
        changelog_failures.append((
            name,
            f"{name}: changelog not found at {changelog_path} "
            f"(expected a section for version {tag_version!r})",
        ))
        continue

    found = False
    try:
        with open(changelog_path, "r", encoding="utf-8") as fh:
            for line in fh:
                if heading_re.match(line):
                    found = True
                    break
    except OSError as exc:
        changelog_failures.append(
            (name, f"{name}: could not read changelog at {changelog_path}: {exc}")
        )
        continue

    if not found:
        changelog_failures.append((
            name,
            f"{name}: changelog at {changelog_path} has no section for version {tag_version!r}",
        ))

# Clause 3: CI-conclusion agreement for the tagged SHA (D-10, PUBOPS-02).
# ci_mode == "runs_path" means the caller resolved a runs data file (either
# a --ci-runs-json fixture or a live gh api response, written by
# _crc_fetch_ci_runs) -- this parser does not care which, it is the same
# code path either way. ci_mode == "not_checked" means neither --sha nor
# --ci-runs-json was supplied outside CI; that is not a failure, but the
# report must say so explicitly rather than silently omitting the clause.
ci_failure_msg = None
ci_note = None

if ci_mode == "runs_path" and ci_runs_path:
    runs = []
    parse_error = None
    try:
        with open(ci_runs_path, "r", encoding="utf-8") as fh:
            text = fh.read()
    except OSError as exc:
        parse_error = f"could not read CI runs data at {ci_runs_path}: {exc}"
        text = ""

    if parse_error is None:
        # Defensive against `gh api --paginate` writing more than one JSON
        # document back-to-back (one per page) rather than a single merged
        # object -- every "workflow_runs" array found across all documents
        # in the file is merged before sorting.
        decoder = json.JSONDecoder()
        idx = 0
        while idx < len(text):
            while idx < len(text) and text[idx].isspace():
                idx += 1
            if idx >= len(text):
                break
            try:
                obj, end = decoder.raw_decode(text, idx)
            except json.JSONDecodeError as exc:
                parse_error = f"could not parse CI runs data at {ci_runs_path}: {exc}"
                break
            idx = end
            if isinstance(obj, dict):
                wr = obj.get("workflow_runs")
                if isinstance(wr, list):
                    runs.extend(wr)
            elif isinstance(obj, list):
                runs.extend(obj)

    if parse_error is not None:
        ci_failure_msg = parse_error
    elif not runs:
        ci_failure_msg = f"no completed ci.yml run found for SHA {sha!r}."
    else:
        # Sort by created_at ascending with id as the tiebreak, then take
        # the last element -- never index the array unsorted, so two runs
        # sharing a timestamp still resolve deterministically to the
        # higher id, regardless of array order.
        def _ci_sort_key(run):
            return (run.get("created_at") or "", run.get("id") or 0)

        deciding = sorted(runs, key=_ci_sort_key)[-1]
        conclusion = deciding.get("conclusion")
        if conclusion != "success":
            ci_failure_msg = (
                f"most recent completed ci.yml run for SHA {sha!r} concluded "
                f"{conclusion!r} (not 'success') -- re-run CI on main at that SHA, "
                f"or fix and re-tag."
            )
elif ci_mode == "not_checked":
    ci_note = (
        "NOTE: the CI-conclusion clause (D-10) was not checked -- no --sha was "
        "supplied and this is not a GitHub Actions run. A local pass above is "
        "therefore not a full-gate pass."
    )

if version_failures or changelog_failures or ci_failure_msg is not None:
    # Merge clauses 1 and 2 into the same failures list, sorted by package
    # name (clause 1's message before clause 2's for the same package);
    # clause 3's single message (there is exactly one deciding run) is
    # appended after, so a run reporting more than one clause stays
    # deterministic.
    combined = [(name, 0, msg) for name, msg in version_failures] + \
               [(name, 1, msg) for name, msg in changelog_failures]
    combined.sort(key=lambda t: (t[0], t[1]))

    parts = []
    if version_failures:
        parts.append("MISMATCH")
    if changelog_failures:
        parts.append("CHANGELOG")
    if ci_failure_msg is not None:
        parts.append("CI")

    status_by_parts = {
        ("MISMATCH",): "MISMATCH",
        ("CHANGELOG",): "CHANGELOG_MISMATCH",
        ("CI",): "CI_MISMATCH",
        ("MISMATCH", "CHANGELOG"): "MISMATCH_AND_CHANGELOG",
        ("MISMATCH", "CI"): "MISMATCH_AND_CI",
        ("CHANGELOG", "CI"): "CHANGELOG_AND_CI",
        ("MISMATCH", "CHANGELOG", "CI"): "MISMATCH_AND_CHANGELOG_AND_CI",
    }
    status = status_by_parts[tuple(parts)]

    print(status)
    print(
        f"FAIL: {len(version_failures)} manifest-version mismatch(es), "
        f"{len(changelog_failures)} changelog-section issue(s) and "
        f"{'1' if ci_failure_msg is not None else '0'} CI-conclusion issue among "
        f"{len(publishable)} publishable package(s):"
    )
    for _, _, msg in combined:
        print(f"  - {msg}")
    if ci_failure_msg is not None:
        print(f"  - {ci_failure_msg}")
    if ci_note:
        print(ci_note)
    sys.exit(0)

print("OK")
print(f"{len(publishable)} publishable package(s) checked, all match tag version "
      f"{tag_version!r} with a changelog section for it.")
if ci_note:
    print(ci_note)
sys.exit(0)
PY
)
        fi
    fi

    local STATUS_LINE DETAIL
    STATUS_LINE=$(head -n1 <<<"${REPORT}")
    DETAIL=$(tail -n +2 <<<"${REPORT}")

    if [ "${STATUS_LINE}" = "OK" ]; then
        echo "✅ OK: ${DETAIL}"
        return 0
    else
        echo "❌ Pre-publish consistency check failed (${STATUS_LINE})"
        echo ""
        echo "${DETAIL}"
        echo ""
        echo "If this failure is unexpected:"
        echo "  1. MISMATCH: a publishable crate's Cargo.toml [package] version does not"
        echo "     equal the tag -- update the crate version (workspace lockstep) or fix"
        echo "     the tag."
        echo "  2. ZERO_PACKAGES: cargo metadata enumerated no publishable packages --"
        echo "     check for a broken workspace or an accidental publish = false on every"
        echo "     crate."
        echo "  3. MISSING_TAG: pass --tag vX.Y.Z (or X.Y.Z)."
        echo "  4. CHANGELOG_MISMATCH (changelog not found): a publishable package has no"
        echo "     changelog file next to its own manifest -- add one following the"
        echo "     Keep-a-Changelog shape already used by its siblings."
        echo "  5. CHANGELOG_MISMATCH (no section for this version): a changelog file"
        echo "     exists but carries no '## [X.Y.Z]' heading for this tag version -- this"
        echo "     section is normally written by the release tooling, so a human seeing"
        echo "     this locally should run the release flow rather than hand-editing"
        echo "     every changelog."
        echo "  6. MISMATCH_AND_CHANGELOG: both of the above are true for at least one"
        echo "     package in this run -- see the entries above for which."
        echo "  7. CI_MISMATCH: the tagged SHA's most recent completed ci.yml run did not"
        echo "     conclude 'success' (or none was found) -- re-run CI on main at that SHA,"
        echo "     or fix and re-tag."
        echo "  8. CI_LOOKUP_FAILED: the GitHub API lookup itself failed (transport,"
        echo "     authorisation, rate limiting) -- this is never the same as 'no"
        echo "     successful run exists'; fix API access and re-run."
        echo "  9. MISSING_SHA: running inside GitHub Actions requires --sha so the"
        echo "     CI-conclusion clause can run; refusing to run a partial gate."
        echo "  10. MISMATCH_AND_CI / CHANGELOG_AND_CI / MISMATCH_AND_CHANGELOG_AND_CI: more"
        echo "      than one clause above is true in this run -- see the entries for which."
        echo "  11. If this guard is wrong about a crate or the tag, fix the guard rather"
        echo "      than working around it."
        return 1
    fi
}

if [ "${CHECK_RELEASE_CONSISTENCY_LIB_ONLY:-0}" != "1" ]; then
    check_release_consistency_main "$@"
fi
