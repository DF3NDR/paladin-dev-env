#!/usr/bin/env bash
# check-migration-allowlist.sh
#
# Single implementation of the semver-allowlist set-equality gate (PRIM-05;
# Phase 52 D-06 / CIREL-03). CI runs this file as the step "Check semver
# allowlist is set-equal to the MIGRATION.md register" of the required
# `License & Dependency Policy` job in .github/workflows/ci.yml (the inline
# awk copy that used to live in the `semver` job was deleted), and the same
# script backs `make check-migration-allowlist` and `make check-gates`, so a
# local run and CI produce exactly the same pair sets. It is offline: awk,
# grep, comm, sort and diff only.
#
# ROW-LEVEL (D-04): the comparison is the SET of `crate | type` PAIRS, not
# just crate names -- a §9.2 or §10.2 row and an allowlist entry naming DIFFERENT
# types under the SAME crate must fail. The set is a DEDUPLICATED set, never
# a row count against an entry count: `paladin-ai | Settings` is
# deliberately marked `Y` on separate §9.2 and §10.2 rows that all legitimately
# mirror the SAME allowlist entry -- they must collapse to one set member.
#
# Inputs are resolved relative to the workspace root (derived from this
# script's own location, never the caller's cwd), and may be overridden via
# MIGRATION_FILE / ALLOWLIST_FILE -- fixture seams for this guard's own
# regression harness, so it can point the script at fixtures without ever
# touching the real tree. A missing input file is a named non-zero failure,
# never a silently-empty comparison. Both pair lists are written to a
# `mktemp -d` scratch directory (never a fixed /tmp path, so concurrent runs
# cannot collide) and are printed before the diff.
#
# CRATE-WIDE ALLOW MIRROR (CI runs it as part of this same step): after the
# pair comparison, every lint id a crate allows crate-wide in its
# `[package.metadata.cargo-semver-checks.lints]` table (value "allow") must
# appear in at least one allowlist `[[entry]]` for that same crate. Without
# it the Cargo.toml allows -- which silence the tool for the whole crate,
# not for one item -- are a convention, not a gate. Manifests are read from
# `${MANIFEST_ROOT}/Cargo.toml` and `${MANIFEST_ROOT}/crates/*/Cargo.toml`;
# MANIFEST_ROOT defaults to the workspace root and is overridable for the
# same reason as the two files above (fixture seam).
#
# Usage:  ./scripts/check-migration-allowlist.sh
#         MIGRATION_FILE=/path/to/MIGRATION.md \
#         ALLOWLIST_FILE=/path/to/allowlist.toml \
#         MANIFEST_ROOT=/path/to/workspace \
#           ./scripts/check-migration-allowlist.sh
# Exit:   0 if the two crate|type pair sets are set-equal and every
#         crate-wide allowed lint is mirrored in the allowlist; non-zero
#         otherwise, or if either input file is missing.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MIGRATION_FILE="${MIGRATION_FILE:-${WORKSPACE_ROOT}/MIGRATION.md}"
ALLOWLIST_FILE="${ALLOWLIST_FILE:-${WORKSPACE_ROOT}/.cargo/semver-checks-allowlist.toml}"
MANIFEST_ROOT="${MANIFEST_ROOT:-${WORKSPACE_ROOT}}"

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

# Register side: scoped to the §9.2 and §10.2 sections only (MIGRATION.md is a
# two-part guide: §10.2 is the v0.11 register, §9.2 the v0.10 one, both read
# by this gate so a deliberate break cannot hide under a heading the window
# does not read), so a "Y" elsewhere in the
# document can't leak in. Data rows are selected by a leading pipe
# immediately followed by a backtick, which excludes the table header and
# its `---` separator row. Field 2 is the crate (trimmed of
# spaces/tabs/backticks); field 3's FIRST backtick-quoted identifier is the
# type, so a long parenthetical like "StopReason (defined in ...)" still
# reduces to `StopReason`. The deliberate-breaking marker is located by
# scanning fields 6 through NF (never a fixed column) for a trimmed value
# that is exactly "Y" or begins with "Y" followed by a space, em dash or
# hyphen: one register row's Mitigation cell contains a literal `|` inside a
# backticked span, which under naive `-F'|'` splitting shifts that row's
# marker one field to the right -- scanning the range instead of a fixed
# column is what keeps that row from being silently skipped. All trimming
# happens inside awk (no sed backreferences) so that shellcheck reports
# nothing here. Both greps below tolerate exit 1 (no matches: an empty
# register or an empty allowlist is a legal state -- both empty is the
# set-equal baseline this check shipped with) while still failing on exit 2
# (a real error).
awk '/^## (9|10)\.2 /{flag=1; next} /^## (9|10)\.3 /{flag=0} flag' "${MIGRATION_FILE}" \
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

echo "MIGRATION.md §9.2 and §10.2 deliberate-breaking crate|type pairs:"
cat "${MIGRATION_PAIRS}"
echo "Allowlist crate|type pairs:"
cat "${ALLOWLIST_PAIRS}"

STATUS=0
if ! diff -u "${MIGRATION_PAIRS}" "${ALLOWLIST_PAIRS}"; then
    echo "::error::Allowlist and MIGRATION.md §9.2 and §10.2 deliberate-breaking crate|type pairs are not set-equal (see diff above)."
    STATUS=1
fi

# Crate-wide allows side (the crate-wide-allow mirror, see the header). The
# allowlist side is every `crate | lint` pair named by an [[entry]]; the
# manifest side is every `crate | lint` pair a Cargo.toml allows in its
# cargo-semver-checks lints table. A manifest lint with no allowlist pair is
# unmirrored. Entries are flushed at the next [[entry]] header (or EOF) so
# the order of `crate` and `lint` inside an entry does not matter, and
# comment lines never match because the key must start the line.
ALLOWLIST_LINTS="${SCRATCH}/allowlist-lints.txt"
MANIFEST_LINTS="${SCRATCH}/manifest-lints.txt"
UNMIRRORED_LINTS="${SCRATCH}/unmirrored-lints.txt"

awk '
    function flush() { if (crate != "" && lint != "") print crate " | " lint; crate = ""; lint = "" }
    /^\[\[entry\]\]/ { flush(); next }
    /^[ \t]*crate[ \t]*=/ { split($0, a, "\""); crate = a[2]; next }
    /^[ \t]*lint[ \t]*=/ { split($0, a, "\""); lint = a[2]; next }
    END { flush() }
' "${ALLOWLIST_FILE}" | sort -u > "${ALLOWLIST_LINTS}"

: > "${MANIFEST_LINTS}"
for manifest in "${MANIFEST_ROOT}/Cargo.toml" "${MANIFEST_ROOT}"/crates/*/Cargo.toml; do
    [ -f "${manifest}" ] || continue
    awk -v file="${manifest}" '
        /^[ \t]*\[/ { sec = $0; gsub(/[ \t]/, "", sec); sub(/#.*$/, "", sec); next }
        sec == "[package]" && /^[ \t]*name[ \t]*=/ { split($0, a, "\""); pkg = a[2]; next }
        sec == "[package.metadata.cargo-semver-checks.lints]" && /^[ \t]*[A-Za-z0-9_]+[ \t]*=[ \t]*"allow"/ {
            key = $0; sub(/^[ \t]*/, "", key); sub(/[ \t]*=.*$/, "", key); lints[key] = 1
        }
        END {
            for (key in lints) {
                if (pkg == "") { print "ERROR: " file " allows lints but has no [package] name" > "/dev/stderr"; exit 2 }
                print pkg " | " key
            }
        }
    ' "${manifest}" >> "${MANIFEST_LINTS}"
done
sort -u -o "${MANIFEST_LINTS}" "${MANIFEST_LINTS}"

comm -23 "${MANIFEST_LINTS}" "${ALLOWLIST_LINTS}" > "${UNMIRRORED_LINTS}"
if [ -s "${UNMIRRORED_LINTS}" ]; then
    echo "Crate-wide cargo-semver-checks allows with no allowlist entry (crate | lint):"
    cat "${UNMIRRORED_LINTS}"
    echo "::error::Every lint allowed in a crate's [package.metadata.cargo-semver-checks.lints] table must appear in at least one ${ALLOWLIST_FILE##*/} entry for that crate (see list above)."
    STATUS=1
fi

if [ "${STATUS}" -ne 0 ]; then
    exit 1
fi
echo "Allowlist is set-equal to the MIGRATION.md §9.2 and §10.2 deliberate-breaking registers (crate|type pairs)."
echo "Every crate-wide cargo-semver-checks allow is mirrored by an allowlist entry (crate|lint pairs)."
