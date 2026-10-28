#!/usr/bin/env bash
# finalize-release-body.sh
#
# Binds real upstream job outputs -- the container image's immutable digest,
# the tag reference the run actually pushed, and the measured image size --
# into the GitHub release body `create-or-reuse-release.sh` already created
# from the curated CHANGELOG.md section (ARTIFACT-03, ARTIFACT-04). Assembly
# order is inverted from the old shape: `create-release` publishes the
# curated notes alone, and this script -- run from a terminal
# `finalize-release-body` job -- appends the artifact sections afterwards,
# from what the run actually produced, never from a hand-reconstructed
# guess (D-04).
#
# ARTIFACT-05 (plan 21-04) extends the same contract with a checksum
# aggregation step: `--aggregate-checksums` downloads every already-uploaded
# `*.tar.gz` release asset back from the release itself and writes one
# `SHA256SUMS` file covering exactly what is visible at that point -- never
# from a build-time list, so it attests to what a consumer will actually
# receive. The asset inventory the body advertises (the "Downloads and
# verification" section) is always read back from the release's own
# `gh release view --json assets` -- an asset a failed leg never uploaded is
# never listed (ARTIFACT-03's inventory half). The SBOM section, when a
# `--sbom-asset` name is supplied, states its real scope: a CycloneDX
# document for the root `paladin-ai` package only, not the whole workspace
# (D-12).
#
# Idempotency (Phase 20 D-03: every job must be safe to run twice): the
# current release body is read back and truncated at a fixed literal marker
# (`<!-- paladin:release-artifacts -->`) using bash's longest-match
# parameter-expansion trim (`${body%%"$MARKER"*}`) -- never a regex, so a
# curated section containing regex-special characters can never perturb the
# cut point. The body is then fully REBUILT from current inputs, never
# appended to -- `gh release edit --notes-file` replaces the whole body, and
# an append-based implementation would duplicate the artifact block on every
# re-run. If the marker is absent (the first run over a tag), the entire
# current body is treated as the curated section and is preserved in full.
# Running this script N times over the same upstream outputs reproduces a
# byte-identical body every time -- including the checksum-aggregation step,
# which re-clobbers the same sums asset rather than failing on a duplicate
# name (Phase 20 D-03).
#
# Digest normalisation: docker/build-push-action's `digest` output's exact
# wire format (whether it already carries the `sha256:` prefix) is
# unconfirmed (RESEARCH.md Assumption A1) -- a value already prefixed
# `sha256:` is used verbatim; a bare hex value is prefixed exactly once, so
# neither answer breaks the pull line. The digest-pinned image reference is
# derived by stripping the tag suffix from the supplied --image-ref, never
# by rebuilding it from the repository name (the existing `Verify image
# size` step's own comment documents why: ghcr requires lowercase,
# github.repository does not, and metadata-action's own JSON output is the
# only value that has actually been proven correct).
#
# The image-size figure is always advisory prose -- stated against the
# 500 MB target -- and never affects this script's exit code (D-10): a run
# whose image exceeds the target still ends green, honestly reporting the
# measurement in the release body instead of a `::warning::` annotation
# nobody reads.
#
# A container-image section is emitted only when BOTH a digest and an image
# reference are present -- a half-populated section (e.g. a digest with no
# reference) is never advertised. A downloads-and-verification section is
# emitted only when the asset list is non-empty; within it, the one-command
# verification block is emitted only when a checksum-aggregation run in
# *this* invocation actually attached a sums file -- naming a file the run
# did not attach is worse than no instructions at all. A SBOM section is
# emitted only when a SBOM asset name was supplied. When no section can be
# built at all, the body states plainly that no artifacts were recorded for
# this run, rather than an artifact heading with nothing under it.
#
# The `gh` executable is resolved through GH_BIN (default: `gh`) -- the same
# testability seam scripts/create-or-reuse-release.sh's regression harness
# uses to stub the GitHub API, so this script's own harness never touches
# the network either.
#
# Size guard (CIREL-05, D-11): GitHub rejects a release body longer than
# 125,000 characters, and the curated section `create-or-reuse-release.sh`
# publishes can already sit near that cap before the artifact sections below
# are appended. After composing, `_frb_fit_composed_body` measures the body
# against PALADIN_RELEASE_BODY_LIMIT (default 125000, positive integer; the
# same variable and the same one implementation, scripts/release-body-fit.sh,
# the create step uses) and, only when it is over, re-fits JUST the curated
# section to limit minus the artifact part, then recomposes. The artifact
# sections are never shortened, dropped or reordered, and a body within the
# limit is left byte-for-byte as composed (nothing logged). The cut is logged
# by size (`release-body-fit: original N chars, ...`) and leaves a sentinel
# footer in the body; the helper strips an existing footer before re-fitting,
# so any number of re-runs publishes the same bytes with the footer once. If
# the artifact sections alone leave no room for the footer plus one curated
# character, the script fails by name (`the release artifact sections alone
# need N chars; limit L leaves no room for the curated notes`) rather than
# publish an over-limit body; an invalid PALADIN_RELEASE_BODY_LIMIT and a
# missing release-body-fit.sh are named failures too. `${GITHUB_REPOSITORY}`
# supplies the footer's changelog link (release.yml passes no --repo).
#
# Sourcing seam: set FINALIZE_RELEASE_BODY_LIB_ONLY=1 before sourcing this
# file to load finalize_release_body_main (and its helpers, including the
# pure compose_release_body) without executing it -- this file's own
# regression harness uses this to exercise composition directly.
#
# Usage:  ./scripts/finalize-release-body.sh --tag <vX.Y.Z>
#             [--image-digest <value>] [--image-ref <reference>]
#             [--image-size-mb <integer>] [--aggregate-checksums]
#             [--assets-dir <path>] [--assets-file <path>]
#             [--sums-name <name>] [--sbom-asset <name>] [--output <path>]
#         --tag is required. --image-digest, --image-ref, --image-size-mb
#         and --sbom-asset are optional; when omitted (or empty) the
#         corresponding section is omitted from the body -- a caller (the
#         `finalize-release-body` workflow job) passes empty strings rather
#         than omitting the flag when an upstream leg did not succeed.
#         --aggregate-checksums is an opt-in switch (no value) that downloads
#         every `*.tar.gz` release asset and writes/uploads a single sums
#         file covering them. --assets-dir defaults to
#         "${RUNNER_TEMP:-/tmp}/release-assets". --sums-name defaults to
#         "SHA256SUMS". --assets-file supplies an explicit asset-name list
#         (one name per line, used verbatim, no re-sort) so a caller -- this
#         script's own harness -- can populate the downloads section without
#         calling the GitHub API; when omitted, the asset list is read from
#         `gh release view --json assets` and sorted under LC_ALL=C.
#         --output defaults to "${RUNNER_TEMP:-/tmp}/final-body.md". An
#         unrecognised flag is a usage error.
# Output: writes the fully-composed release body to --output, then
#         publishes it with `$GH_BIN release edit "$TAG" --notes-file
#         <output>`. When --aggregate-checksums is set and at least one
#         archive was downloaded, also writes "<assets-dir>/<sums-name>" and
#         uploads it via `$GH_BIN release upload "$TAG" <path> --clobber`.
# Exit:   0 on success; non-zero for a usage error or any non-zero `gh`
#         exit (a non-2xx response from `gh release edit` is already a hard
#         failure -- no status-code introspection is needed here, unlike
#         create-or-reuse-release.sh's create/reuse decision table).
#         A size-guard failure (invalid PALADIN_RELEASE_BODY_LIMIT, missing
#         release-body-fit.sh, artifact sections that leave no room, a fitted
#         body still over the limit) is also a non-zero exit, always before
#         `gh release edit`.

set -euo pipefail

# The fixed literal marker separating the curated changelog section (never
# touched) from the artifact sections this script owns (always rebuilt).
# Consumer-facing, so it does not carry this planning tool's own `gsd:`
# namespace the way RESEARCH.md/PATTERNS.md sketch it -- deliberately
# deviated here (see 21-03-PLAN.md's flagged assumptions).
MARKER='<!-- paladin:release-artifacts -->'

# _frb_normalize_digest VALUE -> prints VALUE with exactly one `sha256:`
# prefix: verbatim if VALUE already carries it, prefixed otherwise. Prints
# nothing when VALUE is empty.
_frb_normalize_digest() {
    local value="$1"
    if [ -z "${value}" ]; then
        return 0
    fi
    case "${value}" in
        sha256:*) printf '%s' "${value}" ;;
        *) printf 'sha256:%s' "${value}" ;;
    esac
}

# _frb_sha256_cmd FILE
#
# Prints a sha256sum-format line ("<hex-digest>  <filename>") for FILE.
# Selects `sha256sum` when it exists on PATH and falls back to
# `shasum -a 256` otherwise -- mirrors package-release-binaries.sh's
# sha256_cmd helper. This job runs on ubuntu-latest, but the fallback keeps
# the function honest if it is ever reused on a runner without GNU
# coreutils.
_frb_sha256_cmd() {
    local file="$1"
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum "${file}"
    else
        shasum -a 256 "${file}"
    fi
}

# aggregate_checksums GH TAG ASSETS_DIR SUMS_NAME
#
# Downloads every already-uploaded `*.tar.gz` release asset for TAG back
# into ASSETS_DIR via `GH release download`, so the sums file it writes
# covers exactly what is visible on the release at this point -- never a
# build-time list. Zero archives downloaded is a normal, non-failing
# outcome: no sums file is written and no upload is attempted (an empty
# checksum file must never be attached). One or more archives: computes a
# digest per archive with _frb_sha256_cmd (bare filenames, never paths),
# sorts the lines under LC_ALL=C by filename so the file is byte-stable
# across runs and equal digests never collapse to one entry, writes the
# result to "ASSETS_DIR/SUMS_NAME", and uploads it with
# `GH release upload TAG <path> --clobber` so a re-run replaces the
# existing asset rather than failing on a duplicate name (Phase 20 D-03).
aggregate_checksums() {
    local gh="$1" tag="$2" assets_dir="$3" sums_name="$4"

    mkdir -p "${assets_dir}"

    if ! "${gh}" release download "${tag}" --pattern '*.tar.gz' --dir "${assets_dir}" --clobber; then
        : # No assets matched *.tar.gz (e.g. every build-binaries leg failed, or
          # this job ran before any archive was uploaded) -- `gh release download`
          # exits non-zero in that case, but that is not a failure of this
          # function's own contract. Fall through to the archives=() check below,
          # which already treats zero archives as a normal, non-failing outcome.
    fi

    local -a archives=()
    local archive_path
    while IFS= read -r archive_path; do
        if [ -n "${archive_path}" ]; then
            archives+=("$(basename "${archive_path}")")
        fi
    done < <(find "${assets_dir}" -maxdepth 1 -type f -name '*.tar.gz' 2>/dev/null)

    if [ "${#archives[@]}" -eq 0 ]; then
        return 0
    fi

    local sums_path="${assets_dir}/${sums_name}"
    local name
    {
        for name in "${archives[@]}"; do
            (cd "${assets_dir}" && _frb_sha256_cmd "${name}")
        done
    } | LC_ALL=C sort -k2,2 > "${sums_path}"

    "${gh}" release upload "${tag}" "${sums_path}" --clobber
}

# _frb_build_container_section IMAGE_REF NORMALIZED_DIGEST -> prints a
# `### Container image` section with a fenced, copy-paste-runnable
# `docker pull` line pinned to the digest, and the pushed tag reference that
# line was derived from. IMAGE_REF's tag suffix is stripped with bash's
# shortest-match trim (`${image_ref%:*}`) -- correct even when the registry
# host itself carries a port (`host:port/repo:tag` keeps `host:port`,
# strips only the trailing `:tag`).
_frb_build_container_section() {
    local image_ref="$1" digest="$2"
    local image_without_tag="${image_ref%:*}"
    printf '### Container image\n\n```\ndocker pull %s@%s\n```\n\nPushed tag: `%s`\n' \
        "${image_without_tag}" "${digest}" "${image_ref}"
}

# _frb_build_downloads_section ASSET_LIST SUMS_NAME -> prints a
# `### Downloads and verification` section listing each newline-separated
# name in ASSET_LIST as a list item (duplicates collapsed to a single
# entry -- a name appearing twice in ASSET_LIST is never listed twice), plus
# -- only when SUMS_NAME is non-empty, meaning this invocation actually
# attached a sums file -- a fenced one-command verification block naming
# both the Linux (`sha256sum -c`) and macOS (`shasum -a 256 -c`) forms.
# Blank lines in ASSET_LIST are skipped so a trailing newline never emits an
# empty list item.
_frb_build_downloads_section() {
    local asset_list="$1" sums_name="$2"
    printf '### Downloads and verification\n\n'
    local name
    local -A seen=()
    while IFS= read -r name; do
        if [ -n "${name}" ] && [ -z "${seen[${name}]+x}" ]; then
            seen["${name}"]=1
            printf -- '- `%s`\n' "${name}"
        fi
    done <<< "${asset_list}"
    if [ -n "${sums_name}" ]; then
        printf '\nVerify the downloaded archives against `%s`:\n\n```\nsha256sum -c %s\n```\n\nOn macOS: `shasum -a 256 -c %s`\n' \
            "${sums_name}" "${sums_name}" "${sums_name}"
    fi
}

# _frb_build_sbom_section SBOM_ASSET -> prints a `### SBOM` section naming
# the attached CycloneDX document and stating in one sentence that it
# covers the root `paladin-ai` package only, not the eleven-crate workspace
# (D-12) -- `cargo cyclonedx --all` writes one document per crate; only the
# root package's document is attached to the release.
_frb_build_sbom_section() {
    local sbom_asset="$1"
    printf '### SBOM\n\n`%s` is a CycloneDX SBOM for the root `paladin-ai` package only -- it does not cover the other crates in this workspace.\n' \
        "${sbom_asset}"
}

# _frb_build_size_section SIZE_MB -> prints a `### Image size` section
# stating the measured whole-megabyte figure and its advisory verdict
# against the 500 MB target. At exactly 500 the verdict is "within target";
# above it, "over target". Never affects the caller's exit code -- purely
# informational prose (D-10).
_frb_build_size_section() {
    local size_mb="$1"
    local verdict="within target"
    if [ "${size_mb}" -gt 500 ]; then
        verdict="over target"
    fi
    printf '### Image size\n\nMeasured image size: %s MB -- %s (500 MB target, advisory only).\n' \
        "${size_mb}" "${verdict}"
}

# compose_release_body CURATED DIGEST IMAGE_REF SIZE_MB ASSET_LIST
#                       SUMS_NAME SBOM_ASSET OUTPUT
#
# Pure composition: writes the full new release body to OUTPUT from CURATED
# (the already-truncated curated section) plus the artifact inputs. Touches
# no network. DIGEST is normalised here (see _frb_normalize_digest) so
# callers may pass either wire format. ASSET_LIST is a newline-separated
# string of asset names (empty string when there are none). SUMS_NAME is
# only non-empty when a checksum-aggregation run in this same invocation
# actually attached a sums file -- it gates the verification block, not the
# downloads list itself. SBOM_ASSET is the attached SBOM's asset name, or
# empty when none was supplied.
#
# Sections are emitted in a fixed declared order -- container image,
# downloads and verification, SBOM, image size -- and each is independently
# gated on its own inputs: a container-image section only when both DIGEST
# and IMAGE_REF are non-empty; a downloads-and-verification section only
# when ASSET_LIST is non-empty; a SBOM section only when SBOM_ASSET is
# non-empty; an image-size section only when SIZE_MB is non-empty. When none
# of the four are emitted, a single line states that no artifacts were
# recorded -- never an empty heading. The per-crate changelogs are never
# inlined or linked here -- the root section is the release notes (D-03).
#
# CURATED is normalised to end with exactly one trailing newline before the
# marker is appended (adding one only if missing, never adding a second) --
# this is what keeps a second run over this function's own previous output
# byte-identical to the first: re-extracting the curated section from a
# composed body always yields text that already ends in a single newline,
# so no further growth occurs on subsequent runs.
compose_release_body() {
    local curated="$1" digest="$2" image_ref="$3" size_mb="$4"
    local asset_list="$5" sums_name="$6" sbom_asset="$7" output="$8"

    local norm_digest=""
    norm_digest="$(_frb_normalize_digest "${digest}")"

    local has_container=0 has_size=0 has_downloads=0 has_sbom=0
    local container_section="" size_section="" downloads_section="" sbom_section=""

    if [ -n "${norm_digest}" ] && [ -n "${image_ref}" ]; then
        has_container=1
        container_section="$(_frb_build_container_section "${image_ref}" "${norm_digest}")"
    fi

    if [ -n "${asset_list}" ]; then
        has_downloads=1
        downloads_section="$(_frb_build_downloads_section "${asset_list}" "${sums_name}")"
    fi

    if [ -n "${sbom_asset}" ]; then
        has_sbom=1
        sbom_section="$(_frb_build_sbom_section "${sbom_asset}")"
    fi

    if [ -n "${size_mb}" ]; then
        has_size=1
        size_section="$(_frb_build_size_section "${size_mb}")"
    fi

    local curated_out="${curated}"
    if [ -n "${curated}" ]; then
        case "${curated}" in
            *$'\n') : ;;
            *) curated_out="${curated}"$'\n' ;;
        esac
    fi

    {
        printf '%s' "${curated_out}"
        printf '%s\n' "${MARKER}"
        printf '\n---\n\n## Release Artifacts\n\n'
        if [ "${has_container}" -eq 0 ] && [ "${has_downloads}" -eq 0 ] && [ "${has_sbom}" -eq 0 ] && [ "${has_size}" -eq 0 ]; then
            printf 'No artifacts were recorded for this run.\n'
        else
            if [ "${has_container}" -eq 1 ]; then
                printf '%s\n' "${container_section}"
            fi
            if [ "${has_downloads}" -eq 1 ]; then
                printf '%s\n' "${downloads_section}"
            fi
            if [ "${has_sbom}" -eq 1 ]; then
                printf '%s\n' "${sbom_section}"
            fi
            if [ "${has_size}" -eq 1 ]; then
                printf '%s\n' "${size_section}"
            fi
        fi
    } > "${output}"
}

# _frb_load_fit_lib -> loads the shared release-body size helper
# (scripts/release-body-fit.sh, plan 52-02) from this script's own location.
# Idempotent. A missing helper is a named failure, raised before any `gh`
# call, never a silently unguarded publish.
_frb_load_fit_lib() {
    local root fit_lib
    if declare -F release_body_fit >/dev/null 2>&1; then
        return 0
    fi
    root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
    fit_lib="${root}/scripts/release-body-fit.sh"
    if [ ! -f "${fit_lib}" ]; then
        echo "ERROR: scripts/release-body-fit.sh not found (looked at ${fit_lib})" >&2
        return 1
    fi
    # shellcheck source=scripts/release-body-fit.sh
    RELEASE_BODY_FIT_LIB_ONLY=1 source "${fit_lib}"
}

# _frb_fit_composed_body_in WORK OUTPUT CURATED TAG REPO DIGEST IMAGE_REF
#     SIZE_MB ASSET_LIST SUMS_NAME SBOM_ASSET -> the body of
# _frb_fit_composed_body, run with WORK as a scratch directory the caller
# removes afterwards.
_frb_fit_composed_body_in() {
    local work="$1" output="$2" curated="$3" tag="$4" repo="$5"
    local digest="$6" image_ref="$7" size_mb="$8" asset_list="$9"
    local sums_name="${10}" sbom_asset="${11}"
    local limit total curated_out curated_chars artifact_chars budget
    local curated_file fitted_file err_file fitted

    limit="$(release_body_limit)" || return 1
    total="$(release_body_length "${output}")" || return 1
    if [ "${total}" -le "${limit}" ]; then
        return 0
    fi

    # The curated part exactly as compose_release_body wrote it: a single
    # trailing newline is added when one is missing.
    curated_out="${curated}"
    if [ -n "${curated}" ]; then
        case "${curated}" in
            *$'\n') : ;;
            *) curated_out="${curated}"$'\n' ;;
        esac
    fi
    curated_file="${work}/curated.md"
    fitted_file="${work}/curated-fitted.md"
    err_file="${work}/fit.err"
    printf '%s' "${curated_out}" > "${curated_file}"
    curated_chars="$(release_body_length "${curated_file}")" || return 1

    artifact_chars=$((total - curated_chars))
    budget=$((limit - artifact_chars))
    if [ "${budget}" -lt 1 ]; then
        echo "ERROR: the release artifact sections alone need ${artifact_chars} chars; limit ${limit} leaves no room for the curated notes" >&2
        return 1
    fi

    # Fit in this shell, not a command substitution, so the helper's size log
    # reaches the runner. Its stderr is held back only to recognise the one
    # failure that means "the budget cannot hold the footer"; it is always
    # relayed.
    if ! release_body_fit "${curated_file}" "${fitted_file}" "${budget}" "${tag}" "${repo}" 2> "${err_file}"; then
        cat "${err_file}" >&2
        if grep -q 'is too small to hold the truncation footer' "${err_file}"; then
            echo "ERROR: the release artifact sections alone need ${artifact_chars} chars; limit ${limit} leaves no room for the curated notes" >&2
        fi
        return 1
    fi
    cat "${err_file}" >&2

    # Read the fitted text without letting command substitution strip its
    # trailing newline, then rebuild the whole body from the same artifact
    # inputs so the artifact part is byte-identical to the unfitted one.
    fitted="$(cat "${fitted_file}"; printf x)"
    fitted="${fitted%x}"
    compose_release_body "${fitted}" "${digest}" "${image_ref}" "${size_mb}" \
        "${asset_list}" "${sums_name}" "${sbom_asset}" "${output}"

    total="$(release_body_length "${output}")" || return 1
    if [ "${total}" -gt "${limit}" ]; then
        echo "ERROR: the fitted release body is ${total} chars, still over the limit ${limit}; refusing to publish it" >&2
        return 1
    fi
}

# _frb_fit_composed_body OUTPUT CURATED TAG REPO DIGEST IMAGE_REF SIZE_MB
#     ASSET_LIST SUMS_NAME SBOM_ASSET
#
# The size guard (CIREL-05, D-11). OUTPUT is the body compose_release_body
# just wrote from CURATED plus the artifact inputs (the last six arguments,
# exactly what compose_release_body received). When OUTPUT is within
# PALADIN_RELEASE_BODY_LIMIT it is left untouched -- no re-fit, no log line.
# Otherwise only the curated part is cut, by the shared release_body_fit, to
# limit minus the artifact part's length, and OUTPUT is recomposed; the
# artifact sections are never shortened, dropped or reordered. A budget too
# small for the footer plus one curated character is a named failure, as is
# an OUTPUT that is still over the limit afterwards. The helper strips a
# footer already present in CURATED before re-fitting, so a re-run over this
# function's own published output never stacks footers.
_frb_fit_composed_body() {
    local work rc=0
    _frb_load_fit_lib || return 1
    if ! work="$(mktemp -d "${TMPDIR:-/tmp}/finalize-release-fit.XXXXXX")"; then
        echo "ERROR: could not create a scratch directory for the release body fit" >&2
        return 1
    fi
    _frb_fit_composed_body_in "${work}" "$@" || rc=$?
    rm -rf -- "${work}"
    return "${rc}"
}

finalize_release_body_main() {
    local TAG="" IMAGE_DIGEST="" IMAGE_REF="" IMAGE_SIZE_MB=""
    local AGGREGATE_CHECKSUMS=0
    local ASSETS_DIR="${RUNNER_TEMP:-/tmp}/release-assets"
    local ASSETS_FILE=""
    local SUMS_NAME="SHA256SUMS"
    local SBOM_ASSET=""
    local OUTPUT="${RUNNER_TEMP:-/tmp}/final-body.md"
    local GH="${GH_BIN:-gh}"

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
            --image-digest)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --image-digest requires a value." >&2
                    return 1
                fi
                IMAGE_DIGEST="$2"
                shift 2
                ;;
            --image-ref)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --image-ref requires a value." >&2
                    return 1
                fi
                IMAGE_REF="$2"
                shift 2
                ;;
            --image-size-mb)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --image-size-mb requires a value." >&2
                    return 1
                fi
                IMAGE_SIZE_MB="$2"
                shift 2
                ;;
            --aggregate-checksums)
                AGGREGATE_CHECKSUMS=1
                shift 1
                ;;
            --assets-dir)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --assets-dir requires a value." >&2
                    return 1
                fi
                ASSETS_DIR="$2"
                shift 2
                ;;
            --assets-file)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --assets-file requires a value." >&2
                    return 1
                fi
                ASSETS_FILE="$2"
                shift 2
                ;;
            --sums-name)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --sums-name requires a value." >&2
                    return 1
                fi
                SUMS_NAME="$2"
                shift 2
                ;;
            --sbom-asset)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --sbom-asset requires a value." >&2
                    return 1
                fi
                SBOM_ASSET="$2"
                shift 2
                ;;
            --output)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: --output requires a value." >&2
                    return 1
                fi
                OUTPUT="$2"
                shift 2
                ;;
            *)
                echo "ERROR: unknown flag: $1" >&2
                echo "Usage: finalize-release-body.sh --tag <vX.Y.Z> [--image-digest <value>] [--image-ref <reference>] [--image-size-mb <integer>] [--aggregate-checksums] [--assets-dir <path>] [--assets-file <path>] [--sums-name <name>] [--sbom-asset <name>] [--output <path>]" >&2
                return 1
                ;;
        esac
    done

    if [ -z "${TAG}" ]; then
        echo "ERROR: --tag is required." >&2
        return 1
    fi

    _frb_load_fit_lib || return 1

    local current_body=""
    current_body="$("${GH}" release view "${TAG}" --json body -q .body)"

    local curated_section=""
    curated_section="${current_body%%"${MARKER}"*}"

    # Aggregation runs before the asset list is read (below), so a sums file
    # it attaches appears in the published inventory rather than being
    # advertised a run behind.
    local sums_attached_name=""
    if [ "${AGGREGATE_CHECKSUMS}" -eq 1 ]; then
        aggregate_checksums "${GH}" "${TAG}" "${ASSETS_DIR}" "${SUMS_NAME}"
        if [ -f "${ASSETS_DIR}/${SUMS_NAME}" ]; then
            sums_attached_name="${SUMS_NAME}"
        fi
    fi

    local asset_list=""
    if [ -n "${ASSETS_FILE}" ]; then
        asset_list="$(cat "${ASSETS_FILE}")"
    else
        asset_list="$("${GH}" release view "${TAG}" --json assets -q '.assets[].name' | LC_ALL=C sort)"
    fi

    compose_release_body "${curated_section}" "${IMAGE_DIGEST}" "${IMAGE_REF}" "${IMAGE_SIZE_MB}" \
        "${asset_list}" "${sums_attached_name}" "${SBOM_ASSET}" "${OUTPUT}"

    # Size guard: never hand `gh release edit` a body over the limit (D-11).
    _frb_fit_composed_body "${OUTPUT}" "${curated_section}" "${TAG}" "${GITHUB_REPOSITORY:-}" \
        "${IMAGE_DIGEST}" "${IMAGE_REF}" "${IMAGE_SIZE_MB}" \
        "${asset_list}" "${sums_attached_name}" "${SBOM_ASSET}"

    "${GH}" release edit "${TAG}" --notes-file "${OUTPUT}"
}

if [ "${FINALIZE_RELEASE_BODY_LIB_ONLY:-0}" != "1" ]; then
    finalize_release_body_main "$@"
fi
