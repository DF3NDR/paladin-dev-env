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
# closing assertion double-checks the real tree was never mutated.
#
# Failing cases are asserted FIRST, matching the "prove it fails" house
# convention, before either passing case is treated as evidence the guard
# works at all. The final case runs the guard against the real,
# unmodified tree -- the live PRIM-05 evidence that 15 pairs match 15
# pairs.
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

# write_migration FILE BODY -> wraps BODY between a §9.2 header and a §9.3
# header, matching the section-scoping the guard's awk relies on.
write_migration() {
    local file="$1" body="$2"
    {
        echo "## 9.2 Rust API changes (compile-affecting; the X-10 register)"
        echo ""
        echo "| Crate | Type | Change | Mitigation | Deliberate breaking? | Req |"
        echo "| --- | --- | --- | --- | --- | --- |"
        printf '%s\n' "${body}"
        echo ""
        echo "## 9.3 Toolchain & dependencies"
    } > "${file}"
}

# migration_row CRATE TYPE MARKER -> a single well-formed §9.2 data row.
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

# run_guard MIGRATION ALLOWLIST -> sets $LAST_OUTPUT and $LAST_STATUS
run_guard() {
    local migration="$1" allowlist="$2"
    LAST_OUTPUT="$(MIGRATION_FILE="${migration}" ALLOWLIST_FILE="${allowlist}" "${GUARD}" 2>&1)"
    LAST_STATUS=$?
}

assert_fire() {
    local migration="$1" allowlist="$2" desc="$3"
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${migration}" "${allowlist}"
    if [ "${LAST_STATUS}" -eq 0 ]; then
        echo "FAIL: expected non-zero exit for: ${desc} (got 0)"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    echo "PASS (fire): ${desc}"
}

assert_silent() {
    local migration="$1" allowlist="$2" desc="$3"
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${migration}" "${allowlist}"
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
LAST_OUTPUT="$(MIGRATION_FILE="${SCRATCH}/does-not-exist.md" ALLOWLIST_FILE="${WORKSPACE_ROOT}/.cargo/semver-checks-allowlist.toml" "${GUARD}" 2>&1)"
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
LAST_OUTPUT="$(MIGRATION_FILE="${WORKSPACE_ROOT}/MIGRATION.md" ALLOWLIST_FILE="${SCRATCH}/does-not-exist.toml" "${GUARD}" 2>&1)"
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

# --- 8. The REAL tree -> exit 0 (live PRIM-05 evidence: 15 pairs = 15 pairs). -
ASSERTIONS=$((ASSERTIONS + 1))
LAST_OUTPUT="$("${GUARD}" 2>&1)"
LAST_STATUS=$?
if [ "${LAST_STATUS}" -ne 0 ]; then
    echo "FAIL: expected zero exit for the real tree (got ${LAST_STATUS})"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
else
    pair_count="$(grep -c ' | ' <<<"${LAST_OUTPUT}" || true)"
    echo "PASS (silent): the real tree is set-equal (guard reported ${pair_count} pair line(s) across both lists)"
fi

# --- The real tree must never be mutated by this test. ----------------------
ASSERTIONS=$((ASSERTIONS + 1))
git_status="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- MIGRATION.md .cargo/semver-checks-allowlist.toml)"
if [ -z "${git_status}" ]; then
    echo "PASS (no mutation): git status --porcelain -- MIGRATION.md .cargo/semver-checks-allowlist.toml is empty"
else
    echo "FAIL: MIGRATION.md or the allowlist was mutated by this test run:"
    echo "${git_status}" | sed 's/^/  | /'
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
