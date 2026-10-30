#!/usr/bin/env bash
# check-migration-allowlist_test.sh
#
# Committed regression harness for scripts/check-migration-allowlist.sh
# (PRIM-05). Mirrors the house pattern used by
# tests/scripts/check-codeql-dismissals_test.sh and
# tests/scripts/check-workflow-suppressions_test.sh: every fixture is built
# under a single `mktemp -d` scratch directory removed on exit via a trap,
# the guard is pointed at fixtures via MIGRATION_FILE/ALLOWLIST_FILE (the
# guard's only permitted departure from the CI step it mirrors), and a
# closing assertion compares a BEFORE/AFTER snapshot of the real register
# files to prove this harness wrote nothing (a plain "is the tree clean"
# check would fail whenever the register is mid-edit and uncommitted).
#
# Failing cases are asserted FIRST, matching the "prove it fails" house
# convention, before either passing case is treated as evidence the guard
# works at all. The final case runs the guard against the real,
# unmodified tree -- the live PRIM-05 evidence that both pair lists are empty
# (the §11.2 register and the allowlist hold zero rows and zero entries since
# Phase 52) and that no crate-wide cargo-semver-checks allow survives in a
# Cargo.toml. MIGRATION.md is three parts, newest first: §11 (v0.11.0 ->
# v0.11.1) is the open part and the only register the guard reads; §10 and §9
# are frozen records, so cases 12a-12f pin that the window reads §11.2 up to
# its §11.3 terminator and nothing else.
#
# Usage:  ./tests/scripts/check-migration-allowlist_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/check-migration-allowlist.sh"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-migration-allowlist-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0

# snapshot_real_tree -> prints a fingerprint of the two real register files:
# their `git status --porcelain` line(s) plus a checksum of each. Taken BEFORE
# any case runs and again at the end; the two must be identical, which proves
# this harness wrote nothing even while the register is mid-edit and
# uncommitted (a bare "git status is empty" check cannot tell).
snapshot_real_tree() {
    (
        cd "${WORKSPACE_ROOT}" || exit 1
        git status --porcelain -- MIGRATION.md .cargo/semver-checks-allowlist.toml
        cksum MIGRATION.md .cargo/semver-checks-allowlist.toml
    )
}
SNAPSHOT_BEFORE="$(snapshot_real_tree)"

# write_migration FILE BODY -> wraps BODY between a §11.2 header and a §11.3
# header, matching the section-scoping the guard's awk relies on: §11.2 is the
# open v0.11.1 register, the only one the guard reads, and the `## 11.3 `
# heading is the terminator of its window.
write_migration() {
    local file="$1" body="$2"
    {
        echo "## 11.2 Rust API changes (compile-affecting; the X-10 register)"
        echo ""
        echo "| Crate | Type | Change | Mitigation | Deliberate breaking? | Req |"
        echo "| --- | --- | --- | --- | --- | --- |"
        printf '%s\n' "${body}"
        echo ""
        echo "## 11.3 Toolchain & dependencies"
    } > "${file}"
}

# write_migration_three_parts FILE BODY11 BODY10 BODY9 [omit-11.3] -> the
# three-part layout MIGRATION.md has since Phase 52 (D-03): a §11.1 / §11.2 /
# §11.3 trio above a frozen §10.1 / §10.2 / §10.3 trio above a frozen §9.2 /
# §9.3 pair. BODY11 goes under §11.2, BODY10 under §10.2 and BODY9 under §9.2,
# each below the table header. A fourth argument of `omit-11.3` leaves out the
# `## 11.3 ` heading, so the §11.2 window has no terminator (case 12d).
write_migration_three_parts() {
    local file="$1" body11="$2" body10="$3" body9="$4" omit="${5:-}"
    {
        echo "## 11.1 Behavioral changes (user-visible without code changes)"
        echo ""
        echo "## 11.2 Rust API changes (compile-affecting; the X-10 register)"
        echo ""
        echo "| Crate | Type | Change | Mitigation | Deliberate breaking? | Req |"
        echo "| --- | --- | --- | --- | --- | --- |"
        printf '%s\n' "${body11}"
        echo ""
        if [ "${omit}" != "omit-11.3" ]; then
            echo "## 11.3 Toolchain & dependencies"
            echo ""
        fi
        echo "## 10.1 Behavioral changes (user-visible without code changes)"
        echo ""
        echo "## 10.2 Rust API changes (compile-affecting; the X-10 register)"
        echo ""
        echo "| Crate | Type | Change | Mitigation | Deliberate breaking? | Req |"
        echo "| --- | --- | --- | --- | --- | --- |"
        printf '%s\n' "${body10}"
        echo ""
        echo "## 10.3 Toolchain & dependencies"
        echo ""
        echo "## 9.2 Rust API changes (compile-affecting; the X-10 register)"
        echo ""
        echo "| Crate | Type | Change | Mitigation | Deliberate breaking? | Req |"
        echo "| --- | --- | --- | --- | --- | --- |"
        printf '%s\n' "${body9}"
        echo ""
        echo "## 9.3 Toolchain & dependencies"
    } > "${file}"
}

# migration_row CRATE TYPE MARKER -> a single well-formed register data row.
# MARKER is the raw contents of the "Deliberate breaking?" cell.
migration_row() {
    local crate="$1" type="$2" marker="$3"
    printf '| `%s` | `%s` (struct) | some change | see allowlist | %s | PRIM-05 |\n' "${crate}" "${type}" "${marker}"
}

# write_allowlist FILE ENTRIES -> writes a minimal allowlist.toml fixture.
write_allowlist() {
    local file="$1" entries="$2"
    printf '%s\n' "${entries}" > "${file}"
}

# allowlist_entry CRATE TYPE -> a single well-formed [[entry]] block.
allowlist_entry() {
    local crate="$1" type="$2"
    cat <<TOML
[[entry]]
crate = "${crate}"
lint = "struct_missing"
migration_row = "${crate} | ${type}"
requirement_id = "PRIM-05"
justification = "fixture justification"
TOML
}

# NO_MANIFESTS is an empty directory used as MANIFEST_ROOT by every fixture
# case that is not about the crate-wide-allow mirror check, so the real
# workspace's Cargo.toml lint tables are never compared against a fixture
# allowlist.
NO_MANIFESTS="${SCRATCH}/no-manifests"
mkdir -p "${NO_MANIFESTS}"

# run_guard MIGRATION ALLOWLIST [MANIFEST_ROOT] -> sets $LAST_OUTPUT and
# $LAST_STATUS
run_guard() {
    local migration="$1" allowlist="$2" manifests="${3:-${NO_MANIFESTS}}"
    LAST_OUTPUT="$(MIGRATION_FILE="${migration}" ALLOWLIST_FILE="${allowlist}" MANIFEST_ROOT="${manifests}" "${GUARD}" 2>&1)"
    LAST_STATUS=$?
}

# write_manifest ROOT CRATE LINT... -> writes ROOT/crates/CRATE/Cargo.toml
# with a cargo-semver-checks lints table allowing each LINT.
write_manifest() {
    local root="$1" crate="$2"
    shift 2
    mkdir -p "${root}/crates/${crate}"
    {
        echo "[package]"
        echo "name = \"${crate}\""
        echo "version = \"0.0.0\""
        echo ""
        echo "[package.metadata.cargo-semver-checks.lints]"
        local lint
        for lint in "$@"; do
            echo "${lint} = \"allow\""
        done
    } > "${root}/crates/${crate}/Cargo.toml"
}

assert_fire() {
    local migration="$1" allowlist="$2" desc="$3" manifests="${4:-${NO_MANIFESTS}}"
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${migration}" "${allowlist}" "${manifests}"
    if [ "${LAST_STATUS}" -eq 0 ]; then
        echo "FAIL: expected non-zero exit for: ${desc} (got 0)"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    echo "PASS (fire): ${desc}"
}

assert_silent() {
    local migration="$1" allowlist="$2" desc="$3" manifests="${4:-${NO_MANIFESTS}}"
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${migration}" "${allowlist}" "${manifests}"
    if [ "${LAST_STATUS}" -ne 0 ]; then
        echo "FAIL: expected zero exit (silent) for: ${desc} (got ${LAST_STATUS})"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    echo "PASS (silent): ${desc}"
}

# --- 1. Register row marked Y with no allowlist entry -> non-zero. --------
f_mig="${SCRATCH}/case1-migration.md"
f_allow="${SCRATCH}/case1-allowlist.toml"
write_migration "${f_mig}" "$(migration_row paladin-fixture Widget Y)"
write_allowlist "${f_allow}" ""
assert_fire "${f_mig}" "${f_allow}" "a Y-marked register row with no allowlist entry"

# --- 2. Allowlist entry with no Y row -> non-zero. -------------------------
f_mig="${SCRATCH}/case2-migration.md"
f_allow="${SCRATCH}/case2-allowlist.toml"
write_migration "${f_mig}" ""
write_allowlist "${f_allow}" "$(allowlist_entry paladin-fixture Widget)"
assert_fire "${f_mig}" "${f_allow}" "an allowlist entry with no matching Y row"

# --- 3. Missing MIGRATION.md -> named non-zero. ----------------------------
ASSERTIONS=$((ASSERTIONS + 1))
LAST_OUTPUT="$(MIGRATION_FILE="${SCRATCH}/does-not-exist.md" ALLOWLIST_FILE="${WORKSPACE_ROOT}/.cargo/semver-checks-allowlist.toml" MANIFEST_ROOT="${NO_MANIFESTS}" "${GUARD}" 2>&1)"
LAST_STATUS=$?
if [ "${LAST_STATUS}" -eq 0 ]; then
    echo "FAIL: expected non-zero exit for missing MIGRATION.md (got 0)"
    FAILED=$((FAILED + 1))
elif ! grep -qF "MIGRATION.md not found" <<<"${LAST_OUTPUT}"; then
    echo "FAIL: expected a named MIGRATION.md-not-found message, got:"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
else
    echo "PASS (fire): missing MIGRATION.md is a named non-zero failure"
fi

# --- 4. Missing allowlist -> named non-zero. -------------------------------
ASSERTIONS=$((ASSERTIONS + 1))
LAST_OUTPUT="$(MIGRATION_FILE="${WORKSPACE_ROOT}/MIGRATION.md" ALLOWLIST_FILE="${SCRATCH}/does-not-exist.toml" MANIFEST_ROOT="${NO_MANIFESTS}" "${GUARD}" 2>&1)"
LAST_STATUS=$?
if [ "${LAST_STATUS}" -eq 0 ]; then
    echo "FAIL: expected non-zero exit for missing allowlist (got 0)"
    FAILED=$((FAILED + 1))
elif ! grep -qF "allowlist not found" <<<"${LAST_OUTPUT}"; then
    echo "FAIL: expected a named allowlist-not-found message, got:"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
else
    echo "PASS (fire): missing allowlist is a named non-zero failure"
fi

# --- 5. A Mitigation cell containing a literal `|` inside backticks still
#         resolves (the shifted-column case the CI comment describes). ------
f_mig="${SCRATCH}/case5-migration.md"
f_allow="${SCRATCH}/case5-allowlist.toml"
row="$(printf '| `%s` | `%s` (struct) | some change | see \`a \\| b\` suppression | %s | PRIM-05 |\n' paladin-fixture Widget Y)"
write_migration "${f_mig}" "${row}"
write_allowlist "${f_allow}" "$(allowlist_entry paladin-fixture Widget)"
assert_silent "${f_mig}" "${f_allow}" "a Mitigation cell containing a literal pipe inside backticks still resolves"

# --- 6. Two Y rows for the same crate|type collapse to one set member,
#         matching a single allowlist entry (the paladin-ai | Settings
#         case). -------------------------------------------------------------
f_mig="${SCRATCH}/case6-migration.md"
f_allow="${SCRATCH}/case6-allowlist.toml"
rows="$(migration_row paladin-fixture Widget Y)
$(migration_row paladin-fixture Widget "Y — second occurrence")"
write_migration "${f_mig}" "${rows}"
write_allowlist "${f_allow}" "$(allowlist_entry paladin-fixture Widget)"
assert_silent "${f_mig}" "${f_allow}" "two Y rows for the same crate|type collapse to one set member"

# --- 7. Both empty -> exit 0. -----------------------------------------------
f_mig="${SCRATCH}/case7-migration.md"
f_allow="${SCRATCH}/case7-allowlist.toml"
write_migration "${f_mig}" ""
write_allowlist "${f_allow}" ""
assert_silent "${f_mig}" "${f_allow}" "both register and allowlist empty"

# --- 8. A crate-wide allowed lint with no allowlist entry for that crate
#         -> non-zero, naming the unmirrored crate | lint. The register and
#         allowlist agree (one Widget pair), so only the manifest check can
#         fire. The allowlist entry's lint is struct_missing; the manifest
#         also allows enum_missing, which nothing mirrors. -------------------
f_mig="${SCRATCH}/case8-migration.md"
f_allow="${SCRATCH}/case8-allowlist.toml"
f_root="${SCRATCH}/case8-root"
write_migration "${f_mig}" "$(migration_row paladin-fixture Widget Y)"
write_allowlist "${f_allow}" "$(allowlist_entry paladin-fixture Widget)"
write_manifest "${f_root}" paladin-fixture struct_missing enum_missing
assert_fire "${f_mig}" "${f_allow}" "a crate-wide allowed lint with no allowlist entry" "${f_root}"
if ! grep -qF "paladin-fixture | enum_missing" <<<"${LAST_OUTPUT}"; then
    echo "FAIL: expected the unmirrored 'paladin-fixture | enum_missing' to be named, got:"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- 9. A lint mirrored only under a DIFFERENT crate does not count. -------
f_mig="${SCRATCH}/case9-migration.md"
f_allow="${SCRATCH}/case9-allowlist.toml"
f_root="${SCRATCH}/case9-root"
write_migration "${f_mig}" "$(migration_row paladin-other Widget Y)"
write_allowlist "${f_allow}" "$(allowlist_entry paladin-other Widget)"
write_manifest "${f_root}" paladin-fixture struct_missing
assert_fire "${f_mig}" "${f_allow}" "a lint mirrored only under a different crate" "${f_root}"

# --- 10. Every crate-wide allowed lint mirrored for its own crate -> 0. ----
f_mig="${SCRATCH}/case10-migration.md"
f_allow="${SCRATCH}/case10-allowlist.toml"
f_root="${SCRATCH}/case10-root"
write_migration "${f_mig}" "$(migration_row paladin-fixture Widget Y)"
write_allowlist "${f_allow}" "$(allowlist_entry paladin-fixture Widget)"
write_manifest "${f_root}" paladin-fixture struct_missing
assert_silent "${f_mig}" "${f_allow}" "every crate-wide allowed lint is mirrored for its own crate" "${f_root}"

# --- 10a. A Y row under §11.2 with its allowlist entry -> 0 (the §11.2
#          register is read). ---------------------------------------------------
f_mig="${SCRATCH}/case10a-migration.md"
f_allow="${SCRATCH}/case10a-allowlist.toml"
write_migration_three_parts "${f_mig}" "$(migration_row paladin-fixture Widget Y)" "" ""
write_allowlist "${f_allow}" "$(allowlist_entry paladin-fixture Widget)"
assert_silent "${f_mig}" "${f_allow}" "a Y row under §11.2 with its allowlist entry (11.2 register is read)"

# --- 10b. A Y row under §11.2 with NO allowlist entry -> non-zero (an
#          unregistered deliberate break cannot hide under §11.2). -------------
f_mig="${SCRATCH}/case10b-migration.md"
f_allow="${SCRATCH}/case10b-allowlist.toml"
write_migration_three_parts "${f_mig}" "$(migration_row paladin-fixture Widget Y)" "" ""
write_allowlist "${f_allow}" ""
assert_fire "${f_mig}" "${f_allow}" "a Y row under §11.2 with no allowlist entry"

# --- 12a. Y rows ONLY under the frozen §10.2 and §9.2, empty allowlist -> 0
#          (the frozen registers record breaks already shipped against older
#          baselines, which 0.11.0 now contains; they are no longer read). -----
f_mig="${SCRATCH}/case12a-migration.md"
f_allow="${SCRATCH}/case12a-allowlist.toml"
write_migration_three_parts "${f_mig}" "" "$(migration_row paladin-fixture Widget Y)" "$(migration_row paladin-other Gadget Y)"
write_allowlist "${f_allow}" ""
assert_silent "${f_mig}" "${f_allow}" "Y rows only under the frozen §10.2 and §9.2 with an empty allowlist (frozen registers are not read)"

# --- 12b. A Y row under §11.2 plus Y rows under the frozen §10.2/§9.2, only
#          the §11.2 entry -> 0. ------------------------------------------------
f_mig="${SCRATCH}/case12b-migration.md"
f_allow="${SCRATCH}/case12b-allowlist.toml"
write_migration_three_parts "${f_mig}" "$(migration_row paladin-fixture Widget Y)" "$(migration_row paladin-fixture Old10 Y)" "$(migration_row paladin-other Old9 Y)"
write_allowlist "${f_allow}" "$(allowlist_entry paladin-fixture Widget)"
assert_silent "${f_mig}" "${f_allow}" "a §11.2 Y row with its entry passes beside Y rows under the frozen §10.2 and §9.2"

# --- 12c. A Y row placed under §11.1 (outside the register window), no
#          allowlist entry -> 0 (not read). -------------------------------------
f_mig="${SCRATCH}/case12c-migration.md"
f_allow="${SCRATCH}/case12c-allowlist.toml"
{
    echo "## 11.1 Behavioral changes (user-visible without code changes)"
    echo ""
    echo "| Crate | Type | Change | Mitigation | Deliberate breaking? | Req |"
    echo "| --- | --- | --- | --- | --- | --- |"
    migration_row paladin-fixture Widget Y
    echo ""
    echo "## 11.2 Rust API changes (compile-affecting; the X-10 register)"
    echo ""
    echo "## 11.3 Toolchain & dependencies"
} > "${f_mig}"
write_allowlist "${f_allow}" ""
assert_silent "${f_mig}" "${f_allow}" "a Y row under §11.1 is outside the register window and is not read"

# --- 12d. Pitfall 3: a §11.2 table with NO `## 11.3 ` heading, followed by a
#          frozen §10.2 Y row and an empty allowlist -> non-zero. Without its
#          terminator the window runs on into the frozen registers, so the §10.2
#          row is read and has no entry. ------------------------------------------
f_mig="${SCRATCH}/case12d-migration.md"
f_allow="${SCRATCH}/case12d-allowlist.toml"
write_migration_three_parts "${f_mig}" "" "$(migration_row paladin-fixture Widget Y)" "" omit-11.3
write_allowlist "${f_allow}" ""
assert_fire "${f_mig}" "${f_allow}" "a §11.2 window with no §11.3 terminator runs into the frozen §10.2 register"
if ! grep -qF "paladin-fixture | Widget" <<<"${LAST_OUTPUT}"; then
    echo "FAIL: expected the frozen §10.2 pair 'paladin-fixture | Widget' to be read and named, got:"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# --- 12e. Two §11.2 rows, allowlist entries listed in the REVERSE order -> 0
#          (set equality is order-independent). -----------------------------------
f_mig="${SCRATCH}/case12e-migration.md"
f_allow="${SCRATCH}/case12e-allowlist.toml"
rows="$(migration_row paladin-fixture Alpha Y)
$(migration_row paladin-fixture Omega Y)"
write_migration_three_parts "${f_mig}" "${rows}" "" ""
write_allowlist "${f_allow}" "$(allowlist_entry paladin-fixture Omega)
$(allowlist_entry paladin-fixture Alpha)"
assert_silent "${f_mig}" "${f_allow}" "allowlist entries in the reverse order of their §11.2 rows are still set-equal"

# --- 12f. The REAL MIGRATION.md carries `## 11.3 ` after `## 11.2 ` (the
#          window's terminator exists), both before the frozen `# 10.` part,
#          and holds no data row between them (the open register is empty). ------
ASSERTIONS=$((ASSERTIONS + 1))
real_mig="${WORKSPACE_ROOT}/MIGRATION.md"
l112="$(grep -n '^## 11\.2 ' "${real_mig}" | head -n1 | cut -d: -f1)"
l113="$(grep -n '^## 11\.3 ' "${real_mig}" | head -n1 | cut -d: -f1)"
l10="$(grep -n '^# 10\. ' "${real_mig}" | head -n1 | cut -d: -f1)"
if [ -z "${l112}" ] || [ -z "${l113}" ] || [ -z "${l10}" ]; then
    echo "FAIL: real MIGRATION.md is missing '## 11.2 ' (${l112:-none}), '## 11.3 ' (${l113:-none}) or '# 10. ' (${l10:-none})"
    FAILED=$((FAILED + 1))
elif ! { [ "${l112}" -lt "${l113}" ] && [ "${l113}" -lt "${l10}" ]; }; then
    echo "FAIL: real MIGRATION.md order is wrong: '## 11.2 ' at ${l112}, '## 11.3 ' at ${l113}, '# 10. ' at ${l10}"
    FAILED=$((FAILED + 1))
else
    window_rows="$(sed -n "$((l112 + 1)),$((l113 - 1))p" "${real_mig}" | { grep -c '^| `' || true; })"
    if [ "${window_rows}" -ne 0 ]; then
        echo "FAIL: real MIGRATION.md has ${window_rows} data row(s) between '## 11.2 ' and '## 11.3 ' (expected an empty register)"
        FAILED=$((FAILED + 1))
    else
        echo "PASS (shape): real MIGRATION.md has '## 11.3 ' (line ${l113}) after '## 11.2 ' (line ${l112}), before '# 10.' (line ${l10}), with no data row between"
    fi
fi

# --- 11. The REAL tree -> exit 0 with ZERO pairs on both sides (live evidence:
#         the §11.2 register and the allowlist are both empty, and no
#         crate-wide allow survives in any Cargo.toml). -----------------------
ASSERTIONS=$((ASSERTIONS + 1))
LAST_OUTPUT="$("${GUARD}" 2>&1)"
LAST_STATUS=$?
if [ "${LAST_STATUS}" -ne 0 ]; then
    echo "FAIL: expected zero exit for the real tree (got ${LAST_STATUS})"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
else
    pair_count="$(grep -c ' | ' <<<"${LAST_OUTPUT}" || true)"
    if [ "${pair_count}" -ne 0 ]; then
        echo "FAIL: expected zero pair lines on both sides for the real tree, guard reported ${pair_count}:"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
    else
        echo "PASS (silent): the real tree is set-equal with zero pairs on both sides"
    fi
fi

# --- The real tree must never be mutated by this test: the AFTER snapshot of
#     the two register files must equal the BEFORE snapshot taken at the top. --
ASSERTIONS=$((ASSERTIONS + 1))
SNAPSHOT_AFTER="$(snapshot_real_tree)"
if [ "${SNAPSHOT_BEFORE}" = "${SNAPSHOT_AFTER}" ]; then
    echo "PASS (no mutation): the BEFORE and AFTER snapshots of MIGRATION.md and the allowlist are identical"
else
    echo "FAIL: MIGRATION.md or the allowlist changed while this test ran:"
    echo "  before:"
    echo "${SNAPSHOT_BEFORE}" | sed 's/^/  | /'
    echo "  after:"
    echo "${SNAPSHOT_AFTER}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

echo ""
if [ "${FAILED}" -eq 0 ]; then
    echo "✅ ${ASSERTIONS} assertion(s) passed."
    exit 0
else
    echo "❌ ${FAILED}/${ASSERTIONS} assertion(s) failed."
    exit 1
fi
