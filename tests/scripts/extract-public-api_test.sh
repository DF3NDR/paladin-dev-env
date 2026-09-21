#!/usr/bin/env bash
# extract-public-api_test.sh
#
# Committed regression harness for the toolchain pin in
# scripts/extract-public-api.sh (SHIP-06, Phase 37.1).
#
# Why this exists: on 2026-09-21 the required CI check "API Surface Tracking"
# went red on a documentation-only commit. The job installed a FLOATING
# `nightly`, and that day's nightly (rustc bba531001 2026-09-20) changed how
# rustdoc renders derived return types (`-> Self` instead of the fully
# qualified path): 606 baseline lines differed with no public item added,
# removed or changed. The fix pins the nightly through one variable,
# PUBLIC_API_TOOLCHAIN, which the script passes to cargo as `+<toolchain>`
# and CI sets to a dated nightly.
#
# cargo-public-api 0.52.0 has no --toolchain flag. It reads RUSTUP_TOOLCHAIN
# (set by `cargo +<toolchain>`), honours it when it is a nightly, and
# otherwise prints a warning and SILENTLY switches to plain `nightly`. A pin
# that names a non-nightly toolchain would therefore look applied and not be.
# The script refuses such a value, and case 3 below holds it to that.
#
# The script is exercised through a stub `cargo` placed first on PATH, so no
# assertion here compiles anything, runs rustdoc or touches the network.
#
# Usage:  ./tests/scripts/extract-public-api_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SCRIPT="${WORKSPACE_ROOT}/scripts/extract-public-api.sh"
CI_YML="${WORKSPACE_ROOT}/.github/workflows/ci.yml"

SCRATCH="$(mktemp -d)"
trap 'rm -rf "${SCRATCH}"' EXIT

PASSED=0
FAILED=0
pass() { echo "PASS: $1"; PASSED=$((PASSED + 1)); }
fail() { echo "FAIL: $1"; FAILED=$((FAILED + 1)); }

# Mutation baseline (house convention): this harness must leave no tracked
# file under scripts/ or .github/workflows/ modified.
BASELINE_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- scripts .github/workflows)"

# --- stub seam --------------------------------------------------------------
# A fake `cargo` that records every invocation's argv, one call per line, and
# answers the two calls the script makes. A fake `cargo-public-api` satisfies
# the script's `command -v` installed-check.
STUB_BIN="${SCRATCH}/bin"
mkdir -p "${STUB_BIN}"
cat > "${STUB_BIN}/cargo" <<'STUB'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "${CARGO_STUB_LOG}"
for a in "$@"; do
    if [ "$a" = "--version" ]; then echo "cargo-public-api 0.0.0-stub"; exit 0; fi
done
echo "pub fn stub_crate::alpha()"
echo "pub fn stub_crate::beta()"
STUB
chmod +x "${STUB_BIN}/cargo"
printf '#!/usr/bin/env bash\nexit 0\n' > "${STUB_BIN}/cargo-public-api"
chmod +x "${STUB_BIN}/cargo-public-api"

# run_script <log> <out> [VAR=value ...]  -> exit status of the script
run_script() {
    local log="$1" out="$2"; shift 2
    : > "${log}"
    ( cd "${WORKSPACE_ROOT}" && env -u PUBLIC_API_TOOLCHAIN "$@" \
        CARGO_STUB_LOG="${log}" PATH="${STUB_BIN}:${PATH}" \
        "${SCRIPT}" "${out}" >"${out}.stdout" 2>"${out}.stderr" )
}

# The extraction call is the logged line that carries --simplified.
extraction_call() { grep -- '--simplified' "$1" | head -n 1 || true; }

# --- case 1: default is the floating local nightly ---------------------------
LOG1="${SCRATCH}/c1.log"; OUT1="${SCRATCH}/c1.txt"
if run_script "${LOG1}" "${OUT1}"; then
    pass "default run exits 0"
else
    fail "default run exits 0 (got $?)"
fi
CALL1="$(extraction_call "${LOG1}")"
case "${CALL1}" in
    "+nightly public-api "*) pass "default run invokes cargo with +nightly (${CALL1})" ;;
    *) fail "default run invokes cargo with +nightly (got: '${CALL1}')" ;;
esac
if grep -q '^pub fn stub_crate::alpha()$' "${OUT1}"; then
    pass "default run writes the extracted items to the output file"
else
    fail "default run writes the extracted items to the output file"
fi

# --- case 2: a dated pin reaches cargo verbatim -------------------------------
LOG2="${SCRATCH}/c2.log"; OUT2="${SCRATCH}/c2.txt"
if run_script "${LOG2}" "${OUT2}" PUBLIC_API_TOOLCHAIN=nightly-2026-09-20; then
    pass "pinned run exits 0"
else
    fail "pinned run exits 0 (got $?)"
fi
CALL2="$(extraction_call "${LOG2}")"
case "${CALL2}" in
    "+nightly-2026-09-20 public-api "*) pass "pinned run invokes cargo with +nightly-2026-09-20 (${CALL2})" ;;
    *) fail "pinned run invokes cargo with +nightly-2026-09-20 (got: '${CALL2}')" ;;
esac

# --- case 3: a non-nightly value is refused, never silently swapped -----------
for BAD in stable 1.88 "nightly 2026" "nightly-latest" ""; do
    LOG3="${SCRATCH}/c3.log"; OUT3="${SCRATCH}/c3.txt"
    if run_script "${LOG3}" "${OUT3}" "PUBLIC_API_TOOLCHAIN=${BAD}"; then
        if [ -z "${BAD}" ]; then
            # An empty value means "unset": the default applies.
            case "$(extraction_call "${LOG3}")" in
                "+nightly public-api "*) pass "empty PUBLIC_API_TOOLCHAIN falls back to +nightly" ;;
                *) fail "empty PUBLIC_API_TOOLCHAIN falls back to +nightly" ;;
            esac
        else
            fail "PUBLIC_API_TOOLCHAIN='${BAD}' is refused (script exited 0)"
        fi
    else
        if [ -z "${BAD}" ]; then
            fail "empty PUBLIC_API_TOOLCHAIN falls back to +nightly (script failed)"
        elif [ -n "$(extraction_call "${LOG3}")" ]; then
            fail "PUBLIC_API_TOOLCHAIN='${BAD}' is refused BEFORE any extraction call"
        else
            pass "PUBLIC_API_TOOLCHAIN='${BAD}' is refused before any extraction call"
        fi
    fi
done

# --- case 4: CI pins a DATED nightly through the same single variable --------
# Scope every assertion to the api-surface job block.
JOB_BLOCK="$(awk '/^  api-surface:$/{f=1; print; next} f && /^  [A-Za-z0-9_-]+:$/{exit} f{print}' "${CI_YML}")"
if [ -z "${JOB_BLOCK}" ]; then
    fail "ci.yml has an api-surface job block"
else
    pass "ci.yml has an api-surface job block"
fi
PIN="$(printf '%s\n' "${JOB_BLOCK}" | sed -nE 's/^ +PUBLIC_API_TOOLCHAIN: *"?([^" ]+)"? *$/\1/p' | head -n 1)"
if printf '%s' "${PIN}" | grep -qE '^nightly-[0-9]{4}-[0-9]{2}-[0-9]{2}$'; then
    pass "ci.yml api-surface job pins a dated nightly (${PIN})"
else
    fail "ci.yml api-surface job pins a dated nightly (got: '${PIN}')"
fi
# shellcheck disable=SC2016  # the literal ${...} text is what ci.yml must contain
if printf '%s\n' "${JOB_BLOCK}" | grep -qF 'rustup toolchain install "${PUBLIC_API_TOOLCHAIN}"'; then
    pass "ci.yml installs the toolchain named by PUBLIC_API_TOOLCHAIN (one literal, no drift)"
else
    fail "ci.yml installs the toolchain named by PUBLIC_API_TOOLCHAIN (one literal, no drift)"
fi
if printf '%s\n' "${JOB_BLOCK}" | grep -qE 'rustup toolchain install nightly *$'; then
    fail "ci.yml api-surface job no longer installs a floating nightly"
else
    pass "ci.yml api-surface job no longer installs a floating nightly"
fi

# --- no mutation ---------------------------------------------------------------
AFTER_STATUS="$(cd "${WORKSPACE_ROOT}" && git status --porcelain -- scripts .github/workflows)"
if [ "${BASELINE_STATUS}" = "${AFTER_STATUS}" ]; then
    pass "(no mutation) git status --porcelain -- scripts .github/workflows is unchanged"
else
    fail "(no mutation) git status --porcelain -- scripts .github/workflows changed"
fi

echo ""
if [ "${FAILED}" -eq 0 ]; then
    echo "✅ ${PASSED} assertion(s) passed."
    exit 0
fi
echo "❌ ${FAILED}/$((PASSED + FAILED)) assertion(s) failed."
exit 1
