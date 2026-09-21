#!/usr/bin/env bash
# create-or-reuse-release.sh
#
# Makes the `create-release` job safe to run twice on the same tag (PUBOPS-03).
# Looks up a GitHub release by tag; if one already exists (HTTP 200), reuses
# it; if none exists (HTTP 404), creates it. Any other HTTP status from the
# lookup -- including a transport or authorisation failure -- is a hard,
# named failure, never a silent fall-through to "create". This is the point
# of the script: an exit-code-only check cannot tell a 404 apart from a
# network failure, and treating them the same is exactly the failure mode
# that produces a 422 on a healthy-but-flaky re-run.
#
# Decision table:
#   lookup 200                       -> reuse the looked-up release
#   lookup 404                       -> create a new release
#   lookup other (401/403/500/...)   -> hard failure naming the status; no
#                                        create attempted
#   create 201                       -> use the newly created release
#   create 422 (concurrent creation) -> re-fetch by tag once; 200 -> reuse,
#                                        anything else -> hard failure
#   create other                     -> hard failure naming the status
#
# The release body is data, never shell text: the request payload is built
# structurally with `jq -n --arg` from the body file's literal contents (a
# line that is exactly `EOF`, or one containing backticks and `$(...)`,
# reaches the API unchanged) and sent to `gh api` on stdin via `--input -`.
# Commit-subject text must never be interpolated into a command line or into
# $GITHUB_OUTPUT.
#
# The `gh` executable is resolved through GH_BIN (default: `gh`) -- this is
# the seam this script's regression harness uses to stub the GitHub API with
# a scripted fixture, so the test suite never touches the network.
#
# Sourcing seam: set CREATE_OR_REUSE_RELEASE_LIB_ONLY=1 before sourcing this
# file to load the create_or_reuse_release_main function (and its helpers)
# without executing it -- this file's own regression harness uses this to
# exercise the function directly.
#
# Usage:  ./scripts/create-or-reuse-release.sh --tag <vX.Y.Z>
#             [--repo <owner/name>] [--body-file <path>] [--name <title>]
#             [--prerelease <true|false>]
#         --tag is required. --repo defaults to $GITHUB_REPOSITORY.
#         --body-file, when omitted, sends an empty release body.
#         --name defaults to "Release <tag>". --prerelease defaults to
#         "true" when the tag contains a hyphen and "false" otherwise,
#         matching the `contains(version, '-')` expression release.yml
#         already uses. An unrecognised flag is a usage error.
# Output: on success, prints `upload_url=<value>` and `version=<tag>` to
#         stdout, and appends the same two lines to $GITHUB_OUTPUT when that
#         variable is set and non-empty.
# Exit:   0 on success; non-zero for any HTTP-status failure, a malformed
#         response (missing/null upload_url), or a usage error.

set -euo pipefail

# _cor_gh_call METHOD ENDPOINT [PAYLOAD]
#
# Invokes `$GH_BIN api -i -X METHOD ENDPOINT [--input -]`, feeding PAYLOAD on
# stdin when supplied. `-i`/`--include` is what makes the HTTP status
# readable at all (Pitfall 1: gh's bare exit code alone cannot distinguish a
# 404 from a transport failure). The command's own exit code is deliberately
# ignored here -- the parsed status line is the single source of truth for
# what happened, per this script's decision table.
#
# Sets (in the caller's scope, via bash's dynamic scoping): HTTP_STATUS (a
# 3-digit code, or "000" when no HTTP status line could be parsed at all)
# and HTTP_BODY (everything after the first blank line).
_cor_gh_call() {
    local method="$1" endpoint="$2" payload="${3:-}"
    local raw

    if [ -n "${payload}" ]; then
        raw=$(printf '%s' "${payload}" | "${GH}" api -i -X "${method}" "${endpoint}" --input - 2>&1) || true
    else
        raw=$("${GH}" api -i -X "${method}" "${endpoint}" 2>&1) || true
    fi

    local status_line
    status_line="${raw%%$'\n'*}"
    status_line="${status_line//$'\r'/}"
    HTTP_STATUS=$(printf '%s' "${status_line}" | sed -nE 's#^HTTP/[0-9.]+ ([0-9]{3}).*#\1#p')
    if [ -z "${HTTP_STATUS}" ]; then
        HTTP_STATUS="000"
    fi
    HTTP_BODY=$(printf '%s\n' "${raw}" | tr -d '\r' | awk 'f{print} /^$/{f=1}')
}

# _cor_build_payload TAG NAME BODY_FILE PRERELEASE
#
# Prints a JSON release-creation payload to stdout, built structurally with
# `jq -n --arg` so BODY_FILE's contents (commit-subject-derived text) reach
# the API as data, never as interpolated shell/command text.
_cor_build_payload() {
    local tag="$1" name="$2" body_file="$3" prerelease="$4"
    local body_text=""

    if [ -n "${body_file}" ]; then
        if [ ! -f "${body_file}" ]; then
            echo "ERROR: --body-file not found: ${body_file}" >&2
            return 1
        fi
        body_text="$(cat "${body_file}")"
    fi

    jq -n \
        --arg tag_name "${tag}" \
        --arg name "${name}" \
        --arg body "${body_text}" \
        --argjson draft false \
        --argjson prerelease "${prerelease}" \
        '{tag_name: $tag_name, name: $name, body: $body, draft: $draft, prerelease: $prerelease}'
}

create_or_reuse_release_main() {
    local TAG="" REPO="${GITHUB_REPOSITORY:-}" BODY_FILE="" NAME="" PRERELEASE=""
    GH="${GH_BIN:-gh}"

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
            --repo)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --repo requires a value." >&2
                    return 1
                fi
                REPO="$2"
                shift 2
                ;;
            --body-file)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --body-file requires a value." >&2
                    return 1
                fi
                BODY_FILE="$2"
                shift 2
                ;;
            --name)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --name requires a value." >&2
                    return 1
                fi
                NAME="$2"
                shift 2
                ;;
            --prerelease)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --prerelease requires a value." >&2
                    return 1
                fi
                PRERELEASE="$2"
                shift 2
                ;;
            *)
                echo "ERROR: unknown flag: $1" >&2
                echo "Usage: create-or-reuse-release.sh --tag <vX.Y.Z> [--repo <owner/name>] [--body-file <path>] [--name <title>] [--prerelease <true|false>]" >&2
                return 1
                ;;
        esac
    done

    if [ -z "${TAG}" ]; then
        echo "ERROR: --tag is required." >&2
        return 1
    fi
    if [ -z "${REPO}" ]; then
        echo "ERROR: --repo is required (or set GITHUB_REPOSITORY)." >&2
        return 1
    fi
    if [ -z "${NAME}" ]; then
        NAME="Release ${TAG}"
    fi
    if [ -z "${PRERELEASE}" ]; then
        case "${TAG}" in
            *-*) PRERELEASE="true" ;;
            *) PRERELEASE="false" ;;
        esac
    fi

    local HTTP_STATUS="" HTTP_BODY="" RELEASE_JSON=""

    # --- Lookup by tag. -----------------------------------------------------
    _cor_gh_call GET "repos/${REPO}/releases/tags/${TAG}"

    if [ "${HTTP_STATUS}" = "200" ]; then
        echo "Release for ${TAG} already exists -- reusing (idempotent re-run)." >&2
        RELEASE_JSON="${HTTP_BODY}"
    elif [ "${HTTP_STATUS}" = "404" ]; then
        # --- Create. ---------------------------------------------------------
        local PAYLOAD
        if ! PAYLOAD=$(_cor_build_payload "${TAG}" "${NAME}" "${BODY_FILE}" "${PRERELEASE}"); then
            return 1
        fi

        _cor_gh_call POST "repos/${REPO}/releases" "${PAYLOAD}"

        if [ "${HTTP_STATUS}" = "201" ]; then
            RELEASE_JSON="${HTTP_BODY}"
        elif [ "${HTTP_STATUS}" = "422" ]; then
            # A concurrent run created the release between our lookup and our
            # create attempt. Re-fetch by tag once and reuse if it now
            # exists; fail loudly if it still does not (a 422 for any other
            # reason must not be silently swallowed).
            echo "Create returned 422 (likely a concurrent release) -- re-fetching by tag." >&2
            _cor_gh_call GET "repos/${REPO}/releases/tags/${TAG}"
            if [ "${HTTP_STATUS}" = "200" ]; then
                RELEASE_JSON="${HTTP_BODY}"
            else
                echo "ERROR: create returned 422 but the re-fetch by tag returned HTTP ${HTTP_STATUS}, not 200 -- cannot resolve." >&2
                return 1
            fi
        else
            echo "ERROR: release create failed with HTTP status ${HTTP_STATUS}." >&2
            return 1
        fi
    else
        echo "ERROR: release lookup by tag failed with HTTP status ${HTTP_STATUS} (expected 200 or 404)." >&2
        return 1
    fi

    local UPLOAD_URL
    if ! UPLOAD_URL=$(printf '%s' "${RELEASE_JSON}" | jq -e -r '.upload_url'); then
        echo "ERROR: response JSON has a missing or null upload_url field." >&2
        return 1
    fi

    echo "upload_url=${UPLOAD_URL}"
    echo "version=${TAG}"
    if [ -n "${GITHUB_OUTPUT:-}" ]; then
        {
            echo "upload_url=${UPLOAD_URL}"
            echo "version=${TAG}"
        } >> "${GITHUB_OUTPUT}"
    fi

    return 0
}

if [ "${CREATE_OR_REUSE_RELEASE_LIB_ONLY:-0}" != "1" ]; then
    create_or_reuse_release_main "$@"
fi
