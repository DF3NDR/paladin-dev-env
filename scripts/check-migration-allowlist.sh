#!/usr/bin/env bash
# check-migration-allowlist.sh
#
# Local, offline mirror of the CI step "Verify allowlist is set-equal to the
# MIGRATION.md §9.2 register" in .github/workflows/ci.yml's `semver` job
# (PRIM-05). That step exists only inline in ci.yml -- there is no local
# script and no make target, so it cannot be sampled after a single task
# commit without hand-copying the step into a shell. This script is that
# copy: the awk/grep logic below is copied VERBATIM from ci.yml (including
# the tolerant `|| test $? -eq 1` grep idiom and the field-6-to-NF marker
# scan) so it produces exactly the same pair sets CI produces. Do not
# "improve" the parsing here without also updating ci.yml, or the two will
# silently diverge.
#
# ROW-LEVEL (D-04): the comparison is the SET of `crate | type` PAIRS, not
# just crate names -- a §9.2 row and an allowlist entry naming DIFFERENT
# types under the SAME crate must fail. The set is a DEDUPLICATED set, never
# a row count against an entry count: `paladin-ai | Settings` is
# deliberately marked `Y` on two separate §9.2 rows that both legitimately
# mirror the SAME allowlist entry -- they must collapse to one set member.
#
# Inputs are resolved relative to the workspace root (derived from this
# script's own location, never the caller's cwd), and may be overridden via
# MIGRATION_FILE / ALLOWLIST_FILE -- the only permitted departure from the
# CI step, added so this guard's own regression test can point it at
# fixtures without ever touching the real tree. A missing input file is a
# named non-zero failure, never a silently-empty comparison. Both pair lists
# are written to a `mktemp -d` scratch directory (never a fixed /tmp path,
# so concurrent runs cannot collide) and are printed before the diff,
# matching the CI step's own reporting.
#
# Usage:  ./scripts/check-migration-allowlist.sh
#         MIGRATION_FILE=/path/to/MIGRATION.md \
#         ALLOWLIST_FILE=/path/to/allowlist.toml \
#           ./scripts/check-migration-allowlist.sh
# Exit:   0 if the two crate|type pair sets are set-equal; non-zero
#         otherwise, or if either input file is missing.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATION_FILE="${MIGRATION_FILE:-${WORKSPACE_ROOT}/MIGRATION.md}"
ALLOWLIST_FILE="${ALLOWLIST_FILE:-${WORKSPACE_ROOT}/.cargo/semver-checks-allowlist.toml}"

if [ ! -f "${MIGRATION_FILE}" ]; then
    echo "ERROR: MIGRATION.md not found at ${MIGRATION_FILE}" >&2
    exit 1
fi
if [ ! -f "${ALLOWLIST_FILE}" ]; then
    echo "ERROR: allowlist not found at ${ALLOWLIST_FILE}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-migration-allowlist.XXXXXX")"
trap 'rm -rf "${SCRATCH}"' EXIT

MIGRATION_PAIRS="${SCRATCH}/migration-deliberate-pairs.txt"
ALLOWLIST_PAIRS="${SCRATCH}/allowlist-pairs.txt"

# Register side: scoped to the §9.2 section only, so a "Y" elsewhere in the
# document can't leak in. Data rows are selected by a leading pipe
# immediately followed by a backtick, which excludes the table header and
# its `---` separator row. Field 2 is the crate (trimmed of
# spaces/tabs/backticks); field 3's FIRST backtick-quoted identifier is the
# type, so a long parenthetical like "StopReason (defined in ...)" still
# reduces to `StopReason`. The deliberate-breaking marker is located by
# scanning fields 6 through NF (never a fixed column) for a trimmed value
# that is exactly "Y" or begins with "Y" followed by a space, em dash or
# hyphen: one §9.2 row's Mitigation cell contains a literal `|` inside a
# backticked span, which under naive `-F'|'` splitting shifts that row's
# marker one field to the right -- scanning the range instead of a fixed
# column is what keeps that row from being silently skipped. All trimming
# happens inside awk (no sed backreferences) so that shellcheck reports
# nothing here. Both greps below tolerate exit 1 (no matches: an empty
# register or an empty allowlist is a legal state -- both empty is the
# set-equal baseline this check shipped with) while still failing on exit 2
# (a real error).
awk '/^## 9\.2 /{flag=1; next} /^## 9\.3 /{flag=0} flag' "${MIGRATION_FILE}" \
    | { grep -E '^\| `' || test $? -eq 1; } \
    | awk -F'|' '{
        crate=$2; gsub(/^[ \t`]+|[ \t`]+$/, "", crate);
        type=$3; ident="";
        if (match(type, /`[^`]+`/)) ident = substr(type, RSTART+1, RLENGTH-2);
        deliberate="";
        for (k = 6; k <= NF; k++) {
          v = $k; gsub(/^[ \t]+|[ \t]+$/, "", v);
          if (v == "Y" || v ~ /^Y[ \t—-]/) { deliberate = "Y"; break }
        }
        if (deliberate == "Y") print crate " | " ident
      }' \
    | sort -u > "${MIGRATION_PAIRS}"

# Allowlist side: each [[entry]]'s `migration_row = "crate | type"` value is
# split on the pipe (with surrounding whitespace), and the type portion has
# backticks stripped and is reduced to its first whitespace-delimited token
# -- e.g. `require_authentication (fn)` reduces to `require_authentication`,
# matching the register side's reduction of `` `require_authentication`
# (fn) `` to the same identifier. Same shellcheck-cleanliness reasoning as
# the register side: all trimming happens inside awk, never a sed
# backreference.
{ grep -E '^\s*migration_row\s*=' "${ALLOWLIST_FILE}" || test $? -eq 1; } \
    | awk -F'"' '{print $2}' \
    | awk -F' *\\| *' '{
        crate=$1; type=$2; gsub(/`/, "", type); sub(/ .*$/, "", type);
        print crate " | " type
      }' \
    | sort -u > "${ALLOWLIST_PAIRS}"

echo "MIGRATION.md §9.2 deliberate-breaking crate|type pairs:"
cat "${MIGRATION_PAIRS}"
echo "Allowlist crate|type pairs:"
cat "${ALLOWLIST_PAIRS}"

if ! diff -u "${MIGRATION_PAIRS}" "${ALLOWLIST_PAIRS}"; then
    echo "::error::Allowlist and MIGRATION.md §9.2 deliberate-breaking crate|type pairs are not set-equal (see diff above)."
    exit 1
fi
echo "Allowlist is set-equal to the MIGRATION.md §9.2 deliberate-breaking register (crate|type pairs)."
