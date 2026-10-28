#!/usr/bin/env bash
# release-body-fit.sh
#
# The shared release-body size helper (CIREL-05, D-11, plan 52-02). GitHub
# rejects a release body longer than 125,000 characters with HTTP 422, and a
# long changelog is exactly what a big milestone release produces. This file
# is the one place that measures a release body and, when it is over the
# limit, cuts it at a line boundary with a logged, sentinel-marked footer.
# create-or-reuse-release.sh calls it before the create request; plan 52-04
# reuses it from finalize-release-body.sh, so there is one implementation of
# the size policy, never two that drift (the D-06 "one implementation"
# principle, sourced the way check-publish-order.sh sources publish-crates.sh).
#
# Environment (both optional; an empty value means "use the default"):
#   PALADIN_RELEASE_BODY_LIMIT    maximum body size in characters; the guard
#                                 triggers above it. Default 125000. Must be a
#                                 positive decimal integer.
#   PALADIN_RELEASE_BODY_RESERVE  headroom, in characters, left below the limit
#                                 when a body is cut, so the sections that
#                                 finalize-release-body.sh appends later still
#                                 fit. Default 5000. Must be a non-negative
#                                 integer smaller than the limit.
# Anything else is a named ERROR before any API call -- a malformed limit never
# degrades silently into "no limit".
#
# Length means Unicode code points of the strictly UTF-8-decoded text, counted
# by python3 -- never bytes, and never the locale-dependent character count
# of the `wc` utility.
# Invalid UTF-8 is a named failure; it is never measured as bytes, and never
# replaced by an empty or placeholder body.
#
# The fit rules (release_body_fit), in order:
#   1. A body of at most TARGET characters is copied byte-for-byte; nothing is
#      logged.
#   2. Otherwise any existing footer is stripped (from the sentinel line to the
#      end of the text), so re-fitting is idempotent and never stacks footers.
#   3. The text is cut at the last newline within TARGET - len(footer) - 16
#      characters, or hard-cut at that many characters when no usable newline
#      exists (the result is never empty).
#   4. If the kept text leaves a code fence open (a line of at most three
#      spaces then three or more backticks or tildes opens one; a fence of the
#      same character and at least that length closes it), a closing fence line
#      is appended.
#   5. The footer is appended and the result is asserted to be <= TARGET.
#   6. The original, target and final sizes are logged to stderr.
#
# The footer is fixed text with no sizes in it (so a re-fit is idempotent):
# a blank line, then the sentinel `<!-- paladin:release-truncated -->`, then a
# line saying the notes were truncated and naming CHANGELOG.md at the tag with
# a link to https://github.com/<REPO>/blob/<TAG>/CHANGELOG.md (the link is
# omitted when REPO is empty).
#
# Safety: this file never edits the caller's input file, writes OUTPUT only
# through a temp file in OUTPUT's directory that is moved into place (a failure
# never leaves a partial OUTPUT), and never logs, echoes or exports body text,
# only sizes (the credential-handling review in
# .github/instructions/security.instructions.md). The tag and repo are checked
# against a conservative character set before they reach the footer.
#
# Sourcing seam: set RELEASE_BODY_FIT_LIB_ONLY=1 before sourcing this file to
# load the functions without running release_body_fit_main.
#
# Usage:  ./scripts/release-body-fit.sh --in FILE --out FILE --tag TAG
#             [--repo OWNER/NAME]
#         Applies the create-time policy (release_body_fit_for_create).
#         --repo defaults to $GITHUB_REPOSITORY.
# Output: the fitted body in --out; sizes on stderr when a body was cut; when
#         GITHUB_ACTIONS is "true", a `::warning title=Release body truncated`
#         annotation on stdout.
# Exit:   0 on success; 1 for any named failure; 2 for a usage error.

set -euo pipefail

# release_body_limit -> prints the character limit.
release_body_limit() {
    local value="${PALADIN_RELEASE_BODY_LIMIT:-125000}"
    if ! [[ "${value}" =~ ^[0-9]{1,12}$ ]] || [ "$((10#${value}))" -lt 1 ]; then
        echo "ERROR: PALADIN_RELEASE_BODY_LIMIT must be a positive integer" >&2
        return 1
    fi
    echo "$((10#${value}))"
}

# release_body_reserve -> prints the headroom reserved below the limit.
release_body_reserve() {
    local limit value
    limit="$(release_body_limit)" || return 1
    value="${PALADIN_RELEASE_BODY_RESERVE:-5000}"
    if ! [[ "${value}" =~ ^[0-9]{1,12}$ ]] || [ "$((10#${value}))" -ge "${limit}" ]; then
        echo "ERROR: PALADIN_RELEASE_BODY_RESERVE must be a non-negative integer smaller than PALADIN_RELEASE_BODY_LIMIT (${limit})" >&2
        return 1
    fi
    echo "$((10#${value}))"
}

# _rbf_require_python -> a named failure when python3 is not installed.
_rbf_require_python() {
    if ! command -v python3 >/dev/null 2>&1; then
        echo "ERROR: python3 is required to measure the release body and was not found" >&2
        return 1
    fi
}

# release_body_length FILE -> prints the Unicode code-point count of FILE.
release_body_length() {
    _rbf_require_python || return 1
    if [ ! -f "${1:-}" ]; then
        echo "ERROR: release body file not found: ${1:-}" >&2
        return 1
    fi
    python3 -I - "$1" <<'PY'
import sys

try:
    text = open(sys.argv[1], "rb").read().decode("utf-8")
except UnicodeDecodeError:
    sys.stderr.write("ERROR: release body is not valid UTF-8\n")
    sys.exit(1)
print(len(text))
PY
}

# _rbf_python_fit IN OUT TARGET TAG REPO
#
# Does the measuring and cutting for release_body_fit in one python3 block.
# Prints "<original> <final>" (sizes only) on stdout. Any failure is a named
# ERROR on stderr (never body text) and a non-zero exit, with no OUT written.
_rbf_python_fit() {
    python3 -I - "$@" <<'PY'
import os
import re
import sys
import tempfile

SENTINEL = "<!-- paladin:release-truncated -->"
FENCE = re.compile(r"^ {0,3}(`{3,}|~{3,})(.*)$")

in_path, out_path, target_s, tag, repo = sys.argv[1:6]


def fail(message):
    sys.stderr.write("ERROR: " + message + "\n")
    sys.exit(1)


try:
    raw = open(in_path, "rb").read()
    text = raw.decode("utf-8")
except UnicodeDecodeError:
    fail("release body is not valid UTF-8")
except OSError as exc:
    fail("cannot read the release body: " + exc.strerror)

if not re.fullmatch(r"[0-9]{1,12}", target_s) or int(target_s) < 1:
    fail("fit target must be a positive integer")
target = int(target_s)
if not re.fullmatch(r"[A-Za-z0-9._+/-]{1,200}", tag):
    fail("release tag contains characters that are not allowed in the footer")
if repo and not re.fullmatch(r"[A-Za-z0-9._-]+/[A-Za-z0-9._-]+", repo):
    fail("repository must be OWNER/NAME")


def write_out(data):
    out_dir = os.path.dirname(os.path.abspath(out_path))
    fd, tmp = tempfile.mkstemp(prefix=".release-body-fit.", dir=out_dir)
    try:
        with os.fdopen(fd, "wb") as handle:
            handle.write(data)
        os.replace(tmp, out_path)
    except BaseException:
        try:
            os.unlink(tmp)
        except OSError:
            pass
        raise


original = len(text)
if original <= target:
    write_out(raw)
    print(original, original)
    sys.exit(0)

footer = "\n" + SENTINEL + "\n_Release notes truncated to fit GitHub's release-body size limit. Full changelog: CHANGELOG.md at " + tag
if repo:
    footer += " (https://github.com/" + repo + "/blob/" + tag + "/CHANGELOG.md)"
footer += "._\n"

# Strip an existing footer: from the sentinel line to the end of the text,
# plus the single blank line the footer put in front of it.
match = re.search(r"(?m)^" + re.escape(SENTINEL) + r"[ \t]*$", text)
if match:
    text = text[: match.start()]
    if text.endswith("\n\n"):
        text = text[:-1]


def open_fence(kept):
    """Return the opening fence run still unclosed at the end of kept, or ''."""
    run = ""
    for line in kept.split("\n"):
        found = FENCE.match(line)
        if not found:
            continue
        marker, rest = found.group(1), found.group(2)
        if not run:
            # A backtick fence may not carry a backtick in its info string.
            if marker[0] == "`" and "`" in rest:
                continue
            run = marker
        elif marker[0] == run[0] and len(marker) >= len(run) and rest.strip() == "":
            run = ""
    return run


def build(budget):
    if len(text) <= budget:
        kept = text
    else:
        cut = text[:budget].rfind("\n")
        kept = text[: cut + 1] if cut > 0 else text[:budget]
    if kept and not kept.endswith("\n"):
        kept += "\n"
    run = open_fence(kept)
    if run:
        kept += run + "\n"
    return kept + footer


budget = target - len(footer) - 16
if budget < 1:
    fail("fit target %d is too small to hold the truncation footer and any body text" % target)
result = build(budget)
while len(result) > target and budget > 1:
    budget -= len(result) - target
    if budget < 1:
        break
    result = build(budget)
if len(result) > target or budget < 1:
    fail("fit target %d is too small to hold the truncation footer and any body text" % target)

write_out(result.encode("utf-8"))
print(original, len(result))
PY
}

# release_body_fit IN OUT TARGET TAG REPO -> see the header's fit rules.
release_body_fit() {
    local in_file="$1" out_file="$2" target="$3" tag="$4" repo="${5:-}"
    local sizes original final

    _rbf_require_python || return 1
    if [ ! -f "${in_file}" ]; then
        echo "ERROR: release body file not found: ${in_file}" >&2
        return 1
    fi
    sizes="$(_rbf_python_fit "${in_file}" "${out_file}" "${target}" "${tag}" "${repo}")" || return 1
    original="${sizes%% *}"
    final="${sizes##* }"
    if [ "${original}" != "${final}" ] || [ "${original}" -gt "${target}" ]; then
        echo "release-body-fit: original ${original} chars, target ${target} chars, final ${final} chars" >&2
    fi
}

# release_body_fit_for_create IN OUT TAG REPO -> the create-time policy.
release_body_fit_for_create() {
    local in_file="$1" out_file="$2" tag="$3" repo="${4:-}"
    local limit reserve original final

    limit="$(release_body_limit)" || return 1
    reserve="$(release_body_reserve)" || return 1
    original="$(release_body_length "${in_file}")" || return 1

    if [ "${original}" -le "${limit}" ]; then
        cp -- "${in_file}" "${out_file}"
        return 0
    fi

    release_body_fit "${in_file}" "${out_file}" "$((limit - reserve))" "${tag}" "${repo}" || return 1
    final="$(release_body_length "${out_file}")" || return 1
    if [ "${GITHUB_ACTIONS:-}" = "true" ]; then
        echo "::warning title=Release body truncated::original ${original} chars, final ${final} chars, limit ${limit}"
    fi
}

# release_body_fit_main --in FILE --out FILE --tag TAG [--repo OWNER/NAME]
release_body_fit_main() {
    local in_file="" out_file="" tag="" repo="${GITHUB_REPOSITORY:-}"
    local usage="Usage: release-body-fit.sh --in FILE --out FILE --tag TAG [--repo OWNER/NAME]"

    while [ "$#" -gt 0 ]; do
        case "$1" in
            --in | --out | --tag | --repo)
                if [ "$#" -lt 2 ]; then
                    echo "ERROR: $1 requires a value." >&2
                    return 2
                fi
                case "$1" in
                    --in) in_file="$2" ;;
                    --out) out_file="$2" ;;
                    --tag) tag="$2" ;;
                    --repo) repo="$2" ;;
                esac
                shift 2
                ;;
            *)
                echo "ERROR: unknown flag: $1" >&2
                echo "${usage}" >&2
                return 2
                ;;
        esac
    done

    if [ -z "${in_file}" ] || [ -z "${out_file}" ] || [ -z "${tag}" ]; then
        echo "ERROR: --in, --out and --tag are required." >&2
        echo "${usage}" >&2
        return 2
    fi

    release_body_fit_for_create "${in_file}" "${out_file}" "${tag}" "${repo}"
}

if [ "${RELEASE_BODY_FIT_LIB_ONLY:-0}" != "1" ]; then
    release_body_fit_main "$@"
fi
