#!/usr/bin/env bash
# check-public-api-examples.sh
#
# Gate: every D-05 "public API entry point" -- a `pub` struct whose name ends in
# `Builder`, a `pub` trait whose name ends in `Port`, or a `pub` struct whose name ends
# in `Service`, declared under crates/*/src/** or src/**, excluding declarations inside
# a #[cfg(test)] region and items in an unpublished crate (`publish = false`) -- must
# carry an `# Examples` heading (plural, D-06) in its own preceding `///` doc block or
# the file's leading `//!` module doc. See
# .planning/phases/36.1-deferred-items-closure/36.1-ENTRY-POINTS.md
# for the 101-item snapshot at Phase 36.1 close (D-11) -- a point-in-time record, not
# an input: this script re-derives the enumeration live on every run.
#
# No stable-Rust lint performs this check. `rustdoc::missing_doc_code_examples` exists
# only behind `#![feature(rustdoc_missing_doc_code_examples)]` on nightly, and every
# workflow in this repository pins `dtolnay/rust-toolchain@stable`. This script is the
# honest fallback for that gap -- not a workaround for a built-in that exists somewhere
# else. None does, on stable Rust, for this specific check.
#
# Usage:  ./scripts/check-public-api-examples.sh          gate mode (default)
#         ./scripts/check-public-api-examples.sh --list   report mode
# Exit:   Gate mode (default): 0 if every enumerated entry point carries a plural
#         `# Examples` heading; 1 if any is MISSING or carries the singular `# Example`
#         spelling, or if the derivation itself is degenerate (zero entry points
#         found, which would otherwise let a broken derivation report false success).
#         Report mode (--list): always exits 0. It prints the same derived table and
#         is NOT a gate -- do not wire it into CI expecting it to fail a build.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${WORKSPACE_ROOT}"

MODE="gate"
if [[ "${1:-}" == "--list" ]]; then
    MODE="list"
fi

# --- D-05 exclusion: items in a crate that never ships (`publish = false`) ---------
is_unpublished_crate() {
    local file="$1"
    local dir
    dir="$(dirname "${file}")"
    # Walk up to and INCLUDING the workspace root ("."). Terminating before "."
    # left the root Cargo.toml — which governs the whole src/** tree — never
    # inspected, so a `publish = false` there would have been silently inert.
    while :; do
        if [[ -f "${dir}/Cargo.toml" ]]; then
            if grep -qE '^[[:space:]]*publish[[:space:]]*=[[:space:]]*false' "${dir}/Cargo.toml"; then
                return 0
            fi
            return 1
        fi
        [[ "${dir}" == "." || "${dir}" == "/" ]] && break
        dir="$(dirname "${dir}")"
    done
    return 1
}

# --- D-05 exclusion: declarations inside a #[cfg(test)] region ---------------------
# Returns the line number of the first `#[cfg(test)]` attribute immediately followed
# by a `mod` declaration (this tree's consistent `#[cfg(test)] mod tests {` shape).
# A bare `#[cfg(test)] use ...;` does not start a test region, so it does not exclude
# real declarations that happen to follow a test-only import earlier in the file.
test_region_start() {
    awk '
        /^[[:space:]]*#\[cfg\(test\)\][[:space:]]*$/ { pending = NR; next }
        pending && /^[[:space:]]*mod[[:space:]]/ { print pending; exit }
        { pending = 0 }
    ' "$1"
}

# --- Doc-heading detection ----------------------------------------------------------
# Prints the contiguous ///-doc block (attributes skipped) immediately preceding line
# $2 in file $1 -- empty string if the item has no immediately preceding doc comment.
own_doc_block() {
    local file="$1" decl_line="$2"
    awk -v decl="${decl_line}" '
        NR == decl { exit }
        /^[[:space:]]*\/\/\// { doc = doc "\n" $0; next }
        /^[[:space:]]*#\[/    { next }
        { doc = "" }
        END { print doc }
    ' "${file}"
}

# Prints the file's leading //! module-doc block -- empty string if none.
module_doc_block() {
    awk '
        /^[[:space:]]*\/\/!/ { doc = doc "\n" $0; next }
        /^[[:space:]]*$/     { next }
        { exit }
        END { print doc }
    ' "$1"
}

# Classifies doc text: prints "plural", "singular", or "" (no heading found). A
# trailing word boundary distinguishes the two: "# Examples" never matches the
# singular pattern because "Example" immediately followed by "s" has no \b there.
heading_spelling() {
    local text="$1"
    # Anchored to the start of a doc-comment line. The text arriving here retains
    # its `///` / `//!` prefix, so the anchor must allow it — but WITHOUT the anchor
    # any prose merely mentioning the phrase mid-sentence counted as a real heading,
    # letting an undocumented item pass the gate (CR-01, 16-REVIEW.md). Verified to
    # produce classifications identical to the unanchored form on the current tree.
    if grep -qE '^[[:space:]]*(///|//!)[[:space:]]*#{1,2} Examples\b' <<< "${text}"; then
        printf 'plural'
    elif grep -qE '^[[:space:]]*(///|//!)[[:space:]]*#{1,2} Example\b' <<< "${text}"; then
        printf 'singular'
    else
        printf ''
    fi
}

# --- Derive the entry-point set and classify each one -------------------------------
declare -a rows=()
total=0
missing=0
singular_count=0
plural_count=0

derive_kind() {
    local label="$1" pattern="$2" name_pattern="$3"
    local file line rest name own mod_doc spelling status
    while IFS=: read -r file line rest; do
        [[ -z "${file}" ]] && continue
        if is_unpublished_crate "${file}"; then
            continue
        fi
        local region
        region="$(test_region_start "${file}")"
        if [[ -n "${region}" && "${line}" -ge "${region}" ]]; then
            continue
        fi
        name="$(grep -oE "${name_pattern}" <<< "${rest}" | head -1)"
        own="$(own_doc_block "${file}" "${line}")"
        spelling="$(heading_spelling "${own}")"
        if [[ -z "${spelling}" ]]; then
            mod_doc="$(module_doc_block "${file}")"
            spelling="$(heading_spelling "${mod_doc}")"
        else
            mod_doc=""
        fi
        if [[ "${spelling}" == "plural" ]]; then
            status="OK"
            plural_count=$((plural_count + 1))
        elif [[ "${spelling}" == "singular" ]]; then
            status="SINGULAR"
            singular_count=$((singular_count + 1))
        else
            if [[ -z "${own}" && -z "${mod_doc}" ]]; then
                status="MISSING (empty doc block -- degenerate input)"
            else
                status="MISSING"
            fi
            missing=$((missing + 1))
        fi
        total=$((total + 1))
        rows+=("${label}"$'\t'"${name}"$'\t'"${file}:${line}"$'\t'"${status}")
    done < <(grep -rnE "${pattern}" crates/*/src src --include='*.rs')
}

derive_kind "Builder" '^[[:space:]]*pub struct [A-Za-z0-9_]*Builder\b' '[A-Za-z0-9_]*Builder\b'
derive_kind "Port"    '^[[:space:]]*pub trait [A-Za-z0-9_]*Port\b'    '[A-Za-z0-9_]*Port\b'
derive_kind "Service" '^[[:space:]]*pub struct [A-Za-z0-9_]*Service\b' '[A-Za-z0-9_]*Service\b'

# --- Degenerate-input guard: a check that analysed nothing is worse than no check ---
if [[ "${total}" -eq 0 ]]; then
    echo "ERROR: derivation found zero D-05 entry points -- crates/*/src and src/ produced" >&2
    echo "no pub *Builder / *Port / *Service declarations. This almost certainly means the" >&2
    echo "derivation is broken (wrong working directory, moved source tree), not that the" >&2
    echo "public API genuinely shrank to nothing. Refusing to report a vacuous pass." >&2
    exit 1
fi

if [[ "${MODE}" == "list" ]]; then
    echo "REPORT MODE -- this is not a gate; --list always exits 0 regardless of findings."
    printf '%s\t%s\t%s\t%s\n' "KIND" "NAME" "FILE:LINE" "STATUS"
    for row in "${rows[@]}"; do
        printf '%s\n' "${row}"
    done
    echo "TOTAL: ${total} entry points -- ${plural_count} OK, ${missing} MISSING, ${singular_count} SINGULAR"
    exit 0
fi

# --- Gate mode ------------------------------------------------------------------------
violations=0
for row in "${rows[@]}"; do
    status="${row##*$'\t'}"
    if [[ "${status}" != "OK" ]]; then
        printf '%s\n' "${row}"
        violations=$((violations + 1))
    fi
done

if [[ "${violations}" -gt 0 ]]; then
    echo "ERROR: ${violations} of ${total} D-05 public API entry points lack a plural" >&2
    echo "'# Examples' heading (MISSING) or use the singular '# Example' spelling" >&2
    echo "(SINGULAR). Add or fix the heading directly above the item (or in the file's" >&2
    echo "leading //! module doc), following src/application/services/paladin/paladin_builder.rs" >&2
    echo "as the worked pattern. Run with --list for the full derived table." >&2
    exit 1
fi

echo "All ${total} D-05 public API entry points carry a plural '# Examples' heading."
exit 0
