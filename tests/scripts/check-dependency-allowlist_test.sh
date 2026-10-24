#!/usr/bin/env bash
# check-dependency-allowlist_test.sh
#
# Committed regression harness for scripts/check-dependency-allowlist.sh
# (Phase 46, CURR-25, D-21). Mirrors the house pattern used by
# tests/scripts/check-migration-allowlist_test.sh: every fixture is built
# under a single `mktemp -d` scratch directory removed on exit via a trap,
# the guard is pointed at fixtures via CORE_MANIFEST / PORTS_MANIFEST, and a
# closing assertion double-checks the real manifests were never mutated.
#
# Fixtures are copies of the two real manifests edited with sed to plant or
# remove one key. Failing cases are asserted FIRST ("prove it fails"), then
# the passing cases, and the real tree last -- the live evidence that
# paladin-core (14) and paladin-ports (12) match ADR-0015's recorded sets.
#
# Usage:  ./tests/scripts/check-dependency-allowlist_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/check-dependency-allowlist.sh"
REAL_CORE="${WORKSPACE_ROOT}/crates/paladin-core/Cargo.toml"
REAL_PORTS="${WORKSPACE_ROOT}/crates/paladin-ports/Cargo.toml"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-dependency-allowlist-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0

# Checksums of the real manifests, to prove the run never mutates them.
BEFORE_SUMS="$(cksum "${REAL_CORE}" "${REAL_PORTS}")"

# fresh_manifests NAME -> copies both real manifests to SCRATCH/NAME/ and
# sets $FX_CORE and $FX_PORTS to the copies.
fresh_manifests() {
    local name="$1"
    mkdir -p "${SCRATCH}/${name}"
    FX_CORE="${SCRATCH}/${name}/core.toml"
    FX_PORTS="${SCRATCH}/${name}/ports.toml"
    cp "${REAL_CORE}" "${FX_CORE}"
    cp "${REAL_PORTS}" "${FX_PORTS}"
}

# plant_dependency FILE LINE -> inserts LINE directly under `[dependencies]`.
plant_dependency() {
    local file="$1" line="$2"
    sed -i "s|^\[dependencies\]\$|[dependencies]\n${line}|" "${file}"
}

# run_guard CORE PORTS -> sets $LAST_OUTPUT and $LAST_STATUS
run_guard() {
    local core="$1" ports="$2"
    LAST_OUTPUT="$(CORE_MANIFEST="${core}" PORTS_MANIFEST="${ports}" "${GUARD}" 2>&1)"
    LAST_STATUS=$?
}

# assert_fire CORE PORTS DESC NEEDLE... -> non-zero exit AND every NEEDLE
# appears in the output.
assert_fire() {
    local core="$1" ports="$2" desc="$3"
    shift 3
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${core}" "${ports}"
    if [ "${LAST_STATUS}" -eq 0 ]; then
        echo "FAIL: expected non-zero exit for: ${desc} (got 0)"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    local needle
    for needle in "$@"; do
        if ! grep -qF -- "${needle}" <<<"${LAST_OUTPUT}"; then
            echo "FAIL: output for '${desc}' does not name: ${needle}"
            echo "${LAST_OUTPUT}" | sed 's/^/  | /'
            FAILED=$((FAILED + 1))
            return
        fi
    done
    echo "PASS (fire): ${desc}"
}

# assert_silent CORE PORTS DESC -> zero exit.
assert_silent() {
    local core="$1" ports="$2" desc="$3"
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${core}" "${ports}"
    if [ "${LAST_STATUS}" -ne 0 ]; then
        echo "FAIL: expected zero exit (silent) for: ${desc} (got ${LAST_STATUS})"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    echo "PASS (silent): ${desc}"
}

# --- Failing cases first ----------------------------------------------------

fresh_manifests surplus-core
plant_dependency "${FX_CORE}" 'reqwest = "0.12"'
assert_fire "${FX_CORE}" "${FX_PORTS}" "planted reqwest under paladin-core [dependencies]" \
    "paladin-core" "reqwest"

fresh_manifests surplus-ports
plant_dependency "${FX_PORTS}" 'axum = "0.7"'
assert_fire "${FX_CORE}" "${FX_PORTS}" "planted axum under paladin-ports [dependencies]" \
    "paladin-ports" "axum"

fresh_manifests missing-core
sed -i '/^regex = /d' "${FX_CORE}"
assert_fire "${FX_CORE}" "${FX_PORTS}" "paladin-core with regex removed names regex as missing" \
    "paladin-core" "missing dependency \`regex\`"

fresh_manifests renamed-ports
sed -i 's/^async-trait = /async_trait = /' "${FX_PORTS}"
assert_fire "${FX_CORE}" "${FX_PORTS}" "paladin-ports async-trait renamed to async_trait" \
    "unexpected dependency \`async_trait\`" "missing dependency \`async-trait\`"

fresh_manifests build-deps-ports
printf '\n[build-dependencies]\ncc = "1"\n' >> "${FX_PORTS}"
assert_fire "${FX_CORE}" "${FX_PORTS}" "[build-dependencies] key in paladin-ports" \
    "paladin-ports" "build-dependencies" "cc"

fresh_manifests target-deps-core
printf "\n[target.'cfg(unix)'.dependencies]\nlibc = \"0.2\"\n" >> "${FX_CORE}"
assert_fire "${FX_CORE}" "${FX_PORTS}" "[target.'cfg(unix)'.dependencies] key in paladin-core" \
    "paladin-core" "cfg(unix)" "libc"

fresh_manifests target-build-deps-core
printf "\n[target.'cfg(windows)'.build-dependencies]\nwinres = \"0.1\"\n" >> "${FX_CORE}"
assert_fire "${FX_CORE}" "${FX_PORTS}" "[target.*.build-dependencies] key in paladin-core" \
    "paladin-core" "cfg(windows)" "winres"

fresh_manifests empty-deps
printf '[package]\nname = "paladin-ai-core"\nversion = "0.0.0"\n\n[dependencies]\n' > "${FX_CORE}"
assert_fire "${FX_CORE}" "${FX_PORTS}" "empty [dependencies] table" \
    "paladin-core" "empty"

fresh_manifests absent-deps
printf '[package]\nname = "paladin-ai-core"\nversion = "0.0.0"\n' > "${FX_CORE}"
assert_fire "${FX_CORE}" "${FX_PORTS}" "absent [dependencies] table" \
    "paladin-core" "absent"

fresh_manifests nonexistent-path
assert_fire "${SCRATCH}/nonexistent-path/does-not-exist.toml" "${FX_PORTS}" \
    "CORE_MANIFEST path that does not exist" \
    "paladin-core" "does-not-exist.toml"

# Ordering: two planted keys are printed sorted, aaa before zzz.
fresh_manifests ordering
plant_dependency "${FX_CORE}" 'zzz = "1"'
plant_dependency "${FX_CORE}" 'aaa = "1"'
ASSERTIONS=$((ASSERTIONS + 1))
run_guard "${FX_CORE}" "${FX_PORTS}"
aaa_line="$(grep -nF 'unexpected dependency `aaa`' <<<"${LAST_OUTPUT}" | head -n1 | cut -d: -f1)"
zzz_line="$(grep -nF 'unexpected dependency `zzz`' <<<"${LAST_OUTPUT}" | head -n1 | cut -d: -f1)"
if [ "${LAST_STATUS}" -ne 0 ] && [ -n "${aaa_line}" ] && [ -n "${zzz_line}" ] && [ "${aaa_line}" -lt "${zzz_line}" ]; then
    echo "PASS (fire): two planted keys are reported sorted (aaa before zzz)"
else
    echo "FAIL: expected exit 1 with aaa printed before zzz (status ${LAST_STATUS}, aaa line '${aaa_line}', zzz line '${zzz_line}')"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
fi

# Determinism: a second run over the same input prints identical output.
ASSERTIONS=$((ASSERTIONS + 1))
first_output="${LAST_OUTPUT}"
run_guard "${FX_CORE}" "${FX_PORTS}"
if [ "${first_output}" = "${LAST_OUTPUT}" ]; then
    echo "PASS (fire): two runs over the same input print identical output"
else
    echo "FAIL: two runs over the same input printed different output"
    FAILED=$((FAILED + 1))
fi

# --- Passing cases ----------------------------------------------------------

# [dev-dependencies] are out of scope: ADR-0015 measures [dependencies] only.
fresh_manifests dev-only
sed -i 's|^\[dev-dependencies\]$|[dev-dependencies]\nsome-test-only-crate = "1"|' "${FX_CORE}"
grep -q '^some-test-only-crate = ' "${FX_CORE}" || { echo "ERROR: dev-dependencies fixture was not planted" >&2; exit 1; }
assert_silent "${FX_CORE}" "${FX_PORTS}" "a crate added only under [dev-dependencies] passes"

# --- The real tree ----------------------------------------------------------

ASSERTIONS=$((ASSERTIONS + 1))
LAST_OUTPUT="$("${GUARD}" 2>&1)"
LAST_STATUS=$?
if [ "${LAST_STATUS}" -ne 0 ]; then
    echo "FAIL: expected zero exit for the real tree (got ${LAST_STATUS})"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
elif [ "$(grep -c 'paladin-core (14) and paladin-ports (12)' <<<"${LAST_OUTPUT}")" -ne 1 ]; then
    echo "FAIL: the real-tree OK line does not name 14 and 12 exactly once"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
else
    echo "PASS (silent): the real tree matches ADR-0015 (14 and 12)"
fi

# --- The real manifests must never be mutated by this test. -----------------
ASSERTIONS=$((ASSERTIONS + 1))
AFTER_SUMS="$(cksum "${REAL_CORE}" "${REAL_PORTS}")"
if [ "${BEFORE_SUMS}" = "${AFTER_SUMS}" ]; then
    echo "PASS (no mutation): the real manifests are byte-identical before and after"
else
    echo "FAIL: a real manifest was mutated by this test run"
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
