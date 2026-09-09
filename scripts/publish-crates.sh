#!/usr/bin/env bash
# publish-crates.sh
#
# The publish-crates job's whole loop, extracted into a locally-inspectable,
# unit-testable script (PUBOPS-03 criterion 2, PUBOPS-04). Replaces two
# guesses with two reads: whether a crate is already published is decided by
# querying crates.io for that exact name and version -- never by matching
# text in `cargo publish`'s own output -- and whether it is safe to publish
# the next dependent is decided by polling the crates.io sparse index (the
# thing cargo's own resolver actually reads) until the version is visible or
# a bounded timeout is reached, never by a fixed sleep.
#
# Carrier decision (D-06, already settled by Phase 20's research -- not
# reopened here): native `cargo publish --workspace` is NOT adopted as the
# carrier. It is documented non-atomic, has no confirmed already-published
# -skip-on-rerun behaviour, and gives no per-crate outcome hook -- it fails
# two of D-06's three adoption conditions. The explicit per-crate loop
# (Milestone 10 Epic 3's `publish_one()`, carried into this script) stays
# the carrier; what changes is its detection (D-04: registry state, not
# matched error prose) and its wait (D-05: a bounded poll, not `sleep 20`).
# The condition that would reopen this verdict: a future cargo release
# explicitly documenting resume semantics for a partially-published
# workspace (see 20-RESEARCH.md's "State of the Art" table).
#
# Every crate in a run ends in exactly one of four states:
#   published-now           -- not on the registry beforehand; this run
#                               published it and the sparse index showed it
#                               visible before the poll timeout.
#   already-at-this-version -- the versioned crates.io endpoint already
#                               reported this exact version (HTTP 200) before
#                               any publish was attempted, including a
#                               version that is yanked (a yanked version can
#                               never be re-uploaded, so it still counts as
#                               published).
#   skipped                 -- the run did not attempt this crate at all:
#                               every crate in dry-run mode, and every crate
#                               after the first `failed` one in dependency
#                               order (a dependent whose dependency did not
#                               land cannot succeed, so attempting it would
#                               only produce a second, misleading failure).
#   failed                  -- the pre-check returned an unhandled HTTP
#                               status, `cargo publish` itself exited
#                               non-zero, or the post-publish index-visibility
#                               poll reached its timeout without the version
#                               becoming visible.
#
# Decision table (per crate, real run):
#   pre-check 200            -> already-at-this-version, no publish attempted
#   pre-check 404            -> attempt `cargo publish`
#     publish fails           -> failed
#     publish succeeds, index visible by timeout -> published-now
#     publish succeeds, index NOT visible by timeout -> failed
#   pre-check 429             -> bounded retry with a growing pause, then
#                                 re-evaluate; exhausted retries -> failed
#   pre-check any other status -> failed, no publish attempted
#   dry-run (any pre-check)   -> `cargo publish --dry-run`, outcome skipped
#
# Exit rules, in this order:
#   1. Any crate `failed`                       -> exit non-zero.
#   2. Dry-run                                   -> exit zero regardless of
#                                                    the outcome counts (rule
#                                                    3 applies to real runs
#                                                    only).
#   3. Real run, zero crates `published-now`     -> exit non-zero. This is
#                                                    the point of PUBOPS-04: a
#                                                    fully-complete re-run
#                                                    must be an honest,
#                                                    diagnosable red, not a
#                                                    green indistinguishable
#                                                    from a release that
#                                                    worked. The message
#                                                    names the version, says
#                                                    the tag appears fully
#                                                    published, and points at
#                                                    docs/src/appendix/release-recovery.md.
#   4. Otherwise                                 -> exit zero.
#
# The loop keeps no state of its own between runs -- every decision is
# re-derived from the registry (crates.io's API and sparse index), so an
# interrupted run leaves nothing a later run must reconcile. Discovering
# zero crates to publish (an empty `--crates-file`, or a broken default
# list) is itself a named failure, never a report of success over an empty
# set.
#
# Every crates.io call carries the required `User-Agent` header (crates.io
# answers 403 without one) and never passes `-L`/`--location` (the
# credential-header redirect control in security.instructions.md -- these
# calls carry no credential, but the convention is followed uniformly). No
# fixed-duration pause (`sleep <literal>`) exists anywhere in this script --
# every wait is `sleep "$variable"`, driven by the configured poll interval
# or a computed 429-retry backoff, never a bare guessed number.
#
# Sourcing seam: set PUBLISH_CRATES_LIB_ONLY=1 before sourcing this file to
# load its functions (including publish_crates_main) without executing
# anything -- this file's own regression harness uses this to exercise
# individual functions (the registry pre-check, the index poll) directly
# against stubbed CURL_BIN/CARGO_BIN binaries, with no network call.
#
# Seams: CURL_BIN (default: curl), CARGO_BIN (default: cargo) -- point these
# at stub scripts to exercise this file without touching the network or
# actually publishing anything.
#
# Usage:  ./scripts/publish-crates.sh --version <X.Y.Z> [--dry-run]
#             [--poll-timeout <seconds>] [--poll-interval <seconds>]
#             [--crates-file <path>]
#         --version is required. --dry-run runs `cargo publish --dry-run`
#         for every crate and records each as `skipped` (exit 0 regardless
#         of outcome counts). --poll-timeout (default 180) and
#         --poll-interval (default 5) bound the post-publish index-visibility
#         poll; 180s sits comfortably above the ~25-35s publish-to-visible
#         times Phase 19 observed. --poll-interval must be a positive
#         integer; --poll-timeout must be >= --poll-interval; either
#         violation is a usage error before any network call. --crates-file
#         overrides the built-in eleven-crate dependency order with a
#         newline-delimited file -- purely a harness seam; a file resolving
#         to zero entries is the named zero-crates failure, same as a
#         corrupted default list.
# Exit:   0 if every crate in the run reached a valid terminal state, no
#         crate failed, and (real runs only) at least one crate reached
#         published-now; non-zero for a usage error, a zero-crate discovery,
#         any per-crate failure, or the real-run zero-published-now case.

set -euo pipefail

UA='User-Agent: paladin-release-check (github.com/DF3NDR/paladin-dev-env)'

CURL_BIN="${CURL_BIN:-curl}"
CARGO_BIN="${CARGO_BIN:-cargo}"

# Canonical dependency-first publish order (package names) -- carried
# unchanged from release.yml's `publish_one()` loop, Phase 19's reconciled
# output (D-01/D-02), consumed here, not re-derived.
#
# paladin-herald sits AFTER paladin-ports, not directly after
# paladin-ai-core, despite depending only on paladin-ai-core as a normal
# [dependencies] edge. crates/paladin-herald/Cargo.toml also carries a
# version-pinned [dev-dependencies] edge on paladin-ports; cargo records a
# version-carrying dev-dependency in the published manifest and crates.io
# validates it against the index, so paladin-ports must already be on the
# registry before paladin-herald is published. Placing it after
# paladin-ai-core (its normal dependency), after paladin-ports (its
# dev-dependency) and before paladin-ai (its only dependent) satisfies all
# three constraints at once.
CRATES=(
    paladin-ai-core
    paladin-ports
    paladin-herald
    paladin-battalion
    paladin-llm
    paladin-memory
    paladin-web
    paladin-notifications
    paladin-content
    paladin-storage
    paladin-eval
    paladin-ai
)

# OUTCOME[crate] -> one of published-now / already-at-this-version / skipped
# / failed. Global (not `local`) so the loop and the table renderer share
# it; publish_crates_main resets it at the start of every run.
declare -A OUTCOME

# PC_LAST_POLL_ITERATIONS -> set by _pc_wait_for_index_visibility to the
# number of poll iterations the most recent call performed, for callers
# (and the regression harness) that need it. Not read elsewhere in this
# file -- it is part of the function's external return-value seam, read by
# the regression harness after sourcing this script.
# shellcheck disable=SC2034
PC_LAST_POLL_ITERATIONS=0

# _pc_http_status URL -> echoes the numeric HTTP status code, discarding the
# response body. Never passes -L/--location. Always carries the required
# User-Agent.
_pc_http_status() {
    local url="$1"
    "${CURL_BIN}" -s -o /dev/null -w '%{http_code}' -H "${UA}" "${url}" 2>/dev/null || echo "000"
}

# _pc_crate_published NAME VERSION -> return 0 if the versioned crates.io
# endpoint reports the version already exists (HTTP 200 -- a yanked version
# still returns 200, and still counts as published, since a version can
# never be re-uploaded); return 1 on a clean 404 (not yet published); return
# 2 on any other status after a small bounded 429 retry with a growing
# pause, or on any status this script does not recognise -- a hard,
# per-crate failure, never an assumption that the crate is unpublished.
_pc_crate_published() {
    local name="$1" version="$2"
    local url="https://crates.io/api/v1/crates/${name}/${version}"
    local attempt=1 max_attempts=3 status retry_delay

    while :; do
        status="$(_pc_http_status "${url}")"
        case "${status}" in
            200)
                return 0
                ;;
            404)
                return 1
                ;;
            429)
                if [ "${attempt}" -ge "${max_attempts}" ]; then
                    echo "::error::${name}@${version}: crates.io rate-limited (429) after ${attempt} pre-check attempt(s)." >&2
                    return 2
                fi
                retry_delay=$((attempt * 3))
                echo "::warning::${name}@${version}: crates.io returned 429 on the pre-check, retrying in ${retry_delay}s (attempt ${attempt}/${max_attempts})." >&2
                sleep "${retry_delay}"
                attempt=$((attempt + 1))
                ;;
            *)
                echo "::error::${name}@${version}: unexpected HTTP status '${status}' from the crates.io pre-check." >&2
                return 2
                ;;
        esac
    done
}

# _pc_index_path NAME -> echoes the sparse-index path for NAME, using the
# standard one-, two-, three- and longer-name conventions.
_pc_index_path() {
    local name="$1"
    local len=${#name}
    case "${len}" in
        1) echo "1/${name}" ;;
        2) echo "2/${name}" ;;
        3) echo "3/${name:0:1}/${name}" ;;
        *) echo "${name:0:2}/${name:2:2}/${name}" ;;
    esac
}

# _pc_version_in_index NAME VERSION -> return 0 if the sparse index carries
# a non-yanked line for VERSION; return 1 if the fetch failed (index not yet
# created, or a transient error -- the poll's own timeout bounds this, not
# this function) or no matching line was found; return 2 if a line was
# returned but is not parseable JSON -- a malformed body fails loudly rather
# than being silently read as "not yet visible" forever.
_pc_version_in_index() {
    local name="$1" version="$2"
    local path url body
    path="$(_pc_index_path "${name}")"
    url="https://index.crates.io/${path}"

    if ! body="$("${CURL_BIN}" -sf -H "${UA}" "${url}" 2>/dev/null)"; then
        return 1
    fi
    [ -z "${body}" ] && return 1

    local line vers yanked
    while IFS= read -r line; do
        [ -z "${line}" ] && continue
        if ! vers="$(printf '%s' "${line}" | jq -e -r '.vers // empty' 2>/dev/null)"; then
            echo "::error::${name}: malformed sparse-index entry -- failing loudly rather than reading it as absent: ${line}" >&2
            return 2
        fi
        yanked="$(printf '%s' "${line}" | jq -r '.yanked // false' 2>/dev/null)"
        if [ "${vers}" = "${version}" ] && [ "${yanked}" = "false" ]; then
            return 0
        fi
    done <<<"${body}"

    return 1
}

# _pc_wait_for_index_visibility NAME VERSION TIMEOUT INTERVAL -> polls the
# sparse index at INTERVAL-second steps until VERSION is visible or TIMEOUT
# is reached. Sets PC_LAST_POLL_ITERATIONS to the number of poll iterations
# performed. Returns 0 (visible), 1 (timed out) or 2 (malformed index body
# -- propagated from _pc_version_in_index, stops polling immediately rather
# than retrying a response that will never parse).
_pc_wait_for_index_visibility() {
    local name="$1" version="$2" timeout="$3" interval="$4"
    local waited=0 iterations=0 rc

    while :; do
        iterations=$((iterations + 1))
        rc=0
        _pc_version_in_index "${name}" "${version}" || rc=$?
        # shellcheck disable=SC2034 # external return-value seam, read by the regression harness
        PC_LAST_POLL_ITERATIONS="${iterations}"

        if [ "${rc}" -eq 0 ]; then
            echo "::notice::${name}@${version} visible in the sparse index after ${iterations} poll iteration(s) (~${waited}s)." >&2
            return 0
        elif [ "${rc}" -eq 2 ]; then
            return 2
        fi

        waited=$((waited + interval))
        if [ "${waited}" -ge "${timeout}" ]; then
            echo "::error::${name}@${version} not visible in the sparse index after ${timeout}s (poll timeout, ${iterations} poll iteration(s))." >&2
            return 1
        fi
        sleep "${interval}"
    done
}

# _pc_publish_one NAME VERSION DRY_RUN POLL_TIMEOUT POLL_INTERVAL -> runs one
# crate through the decision table above, recording its outcome into
# OUTCOME[NAME]. Returns 0 unless the crate's outcome is `failed`.
_pc_publish_one() {
    local name="$1" version="$2" dry_run="$3" poll_timeout="$4" poll_interval="$5"

    if [ "${dry_run}" = "true" ]; then
        echo "::group::Publishing ${name} (dry-run)"
        "${CARGO_BIN}" publish --dry-run -p "${name}" || true
        OUTCOME["${name}"]="skipped"
        echo "::endgroup::"
        return 0
    fi

    echo "::group::Publishing ${name}"

    local pc_rc=0
    _pc_crate_published "${name}" "${version}" || pc_rc=$?

    if [ "${pc_rc}" -eq 0 ]; then
        echo "::notice::${name}@${version} already on crates.io -- skipping publish."
        OUTCOME["${name}"]="already-at-this-version"
        echo "::endgroup::"
        return 0
    elif [ "${pc_rc}" -eq 2 ]; then
        OUTCOME["${name}"]="failed"
        echo "::endgroup::"
        return 1
    fi

    # pc_rc == 1: not yet published -- attempt the real publish.
    if ! "${CARGO_BIN}" publish -p "${name}"; then
        echo "::error::${name}: cargo publish failed." >&2
        OUTCOME["${name}"]="failed"
        echo "::endgroup::"
        return 1
    fi

    if _pc_wait_for_index_visibility "${name}" "${version}" "${poll_timeout}" "${poll_interval}"; then
        OUTCOME["${name}"]="published-now"
        echo "::endgroup::"
        return 0
    else
        OUTCOME["${name}"]="failed"
        echo "::endgroup::"
        return 1
    fi
}

# _pc_render_table VERSION CRATE... -> renders the Markdown outcome table,
# iterating CRATE... in the order given (dependency order) so two runs over
# the same registry state produce byte-identical tables.
_pc_render_table() {
    local version="$1"
    shift
    local c
    echo "## Publish outcome -- ${version}"
    echo ""
    echo "| Crate | Outcome |"
    echo "|---|---|"
    for c in "$@"; do
        echo "| ${c} | ${OUTCOME[${c}]} |"
    done
}

publish_crates_main() {
    local VERSION="" DRY_RUN="false" POLL_TIMEOUT="180" POLL_INTERVAL="5" CRATES_FILE=""

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --version)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --version requires a value." >&2
                    return 1
                fi
                VERSION="$2"
                shift 2
                ;;
            --dry-run)
                DRY_RUN="true"
                shift
                ;;
            --poll-timeout)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --poll-timeout requires a value." >&2
                    return 1
                fi
                POLL_TIMEOUT="$2"
                shift 2
                ;;
            --poll-interval)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --poll-interval requires a value." >&2
                    return 1
                fi
                POLL_INTERVAL="$2"
                shift 2
                ;;
            --crates-file)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --crates-file requires a value." >&2
                    return 1
                fi
                CRATES_FILE="$2"
                shift 2
                ;;
            *)
                echo "ERROR: unknown flag: $1" >&2
                echo "Usage: publish-crates.sh --version <X.Y.Z> [--dry-run] [--poll-timeout <seconds>] [--poll-interval <seconds>] [--crates-file <path>]" >&2
                return 1
                ;;
        esac
    done

    if [ -z "${VERSION}" ]; then
        echo "ERROR: --version is required. Usage: publish-crates.sh --version <X.Y.Z> [--dry-run] [--poll-timeout <seconds>] [--poll-interval <seconds>] [--crates-file <path>]" >&2
        return 1
    fi

    # Strip at most one leading "v" -- "${VERSION#v}" removes the shortest
    # matching prefix once, so a release tag ("v1.2.3", release.yml's own
    # tag-derived version convention -- see check-release-consistency.sh's
    # identical TAG_VERSION="${TAG#v}" stripping) and a bare version
    # ("1.2.3") both resolve to the same crates.io-comparable string. No
    # semver parsing anywhere in this script.
    VERSION="${VERSION#v}"

    # Validate before any network call: a non-positive interval, or a
    # timeout below one interval, is a usage error.
    if ! [[ "${POLL_INTERVAL}" =~ ^[0-9]+$ ]] || [ "${POLL_INTERVAL}" -le 0 ]; then
        echo "ERROR: --poll-interval must be a positive integer (got '${POLL_INTERVAL}')." >&2
        return 1
    fi
    if ! [[ "${POLL_TIMEOUT}" =~ ^[0-9]+$ ]] || [ "${POLL_TIMEOUT}" -lt "${POLL_INTERVAL}" ]; then
        echo "ERROR: --poll-timeout (${POLL_TIMEOUT}) must be >= --poll-interval (${POLL_INTERVAL})." >&2
        return 1
    fi

    local -a CRATE_LIST=()
    if [ -n "${CRATES_FILE}" ]; then
        if [ ! -f "${CRATES_FILE}" ]; then
            echo "ERROR: --crates-file not found: ${CRATES_FILE}" >&2
            return 1
        fi
        local line
        while IFS= read -r line; do
            [ -z "${line}" ] && continue
            CRATE_LIST+=("${line}")
        done <"${CRATES_FILE}"
    else
        CRATE_LIST=("${CRATES[@]}")
    fi

    if [ "${#CRATE_LIST[@]}" -eq 0 ]; then
        echo "ERROR: zero crates to publish -- a broken crate list is a named failure, never a report of success over an empty set." >&2
        return 1
    fi

    OUTCOME=()
    local aborted="false" c

    for c in "${CRATE_LIST[@]}"; do
        if [ "${aborted}" = "true" ]; then
            # An earlier crate in this dependency-first order failed: every
            # crate after it cannot succeed (its dependency did not land),
            # so it is recorded skipped and no publish is attempted.
            OUTCOME["${c}"]="skipped"
            continue
        fi
        if ! _pc_publish_one "${c}" "${VERSION}" "${DRY_RUN}" "${POLL_TIMEOUT}" "${POLL_INTERVAL}"; then
            aborted="true"
        fi
    done

    # Every crate must have exactly one recorded state before the table is
    # emitted -- a crate with no state is a defect in the loop and must fail
    # loudly rather than print a blank cell.
    for c in "${CRATE_LIST[@]}"; do
        case "${OUTCOME[${c}]:-}" in
            published-now | already-at-this-version | skipped | failed) ;;
            *)
                echo "::error::internal error: '${c}' has no recorded outcome state." >&2
                return 1
                ;;
        esac
    done

    local table
    table="$(_pc_render_table "${VERSION}" "${CRATE_LIST[@]}")"
    echo "${table}"
    if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
        echo "${table}" >>"${GITHUB_STEP_SUMMARY}"
    fi

    local any_failed="false" published_now_count=0
    for c in "${CRATE_LIST[@]}"; do
        case "${OUTCOME[${c}]}" in
            failed) any_failed="true" ;;
            published-now) published_now_count=$((published_now_count + 1)) ;;
        esac
    done

    # Exit rules, in order: any failure wins first; a dry-run is always zero
    # (rule 3 below applies to real runs only); a real run that moved zero
    # crates to published-now is the honest, diagnosable red PUBOPS-04
    # exists for.
    if [ "${any_failed}" = "true" ]; then
        return 1
    fi

    if [ "${DRY_RUN}" = "true" ]; then
        return 0
    fi

    if [ "${published_now_count}" -eq 0 ]; then
        echo "::error::all ${#CRATE_LIST[@]} crate(s) already at ${VERSION} -- this tag appears fully published. If this was a recovery re-run, there was nothing left to recover. See docs/src/appendix/release-recovery.md." >&2
        return 1
    fi

    return 0
}

if [ "${PUBLISH_CRATES_LIB_ONLY:-0}" != "1" ]; then
    publish_crates_main "$@"
fi
