#!/usr/bin/env bash
# check-no-stub-panics_test.sh
#
# Committed regression harness for scripts/check-no-stub-panics.sh
# (Phase 55, FIXES-01, D-01). Mirrors the house pattern used by
# tests/scripts/check-version-strings_test.sh: every fixture tree is built under
# a single `mktemp -d` scratch directory removed on exit via a trap, and the
# guard is pointed at a fixture through the test-only STUB_PANICS_ROOT seam (the
# guard itself takes no argument).
#
# Every case starts from `make_tree`, a clean fixture workspace with one
# publishable crate on which the guard passes, and changes exactly one thing.
# Failing cases are asserted FIRST ("prove it fails"), then the named failures,
# determinism and the fixture no-mutation check, then the passing cases, then
# the real tree, and last a checksum assertion that this run never mutated the
# real scanned files.
#
# Usage:  ./tests/scripts/check-no-stub-panics_test.sh
#         Or via `make test-shell-guards`.
# Exit:   0 if every assertion passes; non-zero otherwise, with a report of
#         which assertion(s) failed.

set -uo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GUARD="${WORKSPACE_ROOT}/scripts/check-no-stub-panics.sh"

if [ ! -f "${GUARD}" ]; then
    echo "ERROR: guard script not found at ${GUARD}" >&2
    exit 1
fi

SCRATCH="$(mktemp -d "${TMPDIR:-/tmp}/check-no-stub-panics-test.XXXXXX")"
cleanup() {
    rm -rf "${SCRATCH}"
}
trap cleanup EXIT

FAILED=0
ASSERTIONS=0

# Checksums of every real scanned file, to prove the run never mutates them.
real_sums() {
    (
        cd "${WORKSPACE_ROOT}" || exit 1
        find src crates/*/src -type f -name '*.rs' 2>/dev/null | LC_ALL=C sort | xargs cksum
    )
}
BEFORE_SUMS="$(real_sums)"

# make_tree DIR -> a clean fixture workspace: a root manifest with a [workspace]
# table and no [package], plus one publishable crate x holding a clean lib.rs.
make_tree() {
    local d="$1"
    mkdir -p "${d}/crates/x/src"
    printf '[workspace]\nmembers = ["crates/*"]\n' >"${d}/Cargo.toml"
    printf '[package]\nname = "x"\nversion = "0.1.0"\n' >"${d}/crates/x/Cargo.toml"
    printf '//! Clean fixture crate.\npub fn clean() {}\n' >"${d}/crates/x/src/lib.rs"
}

# fresh NAME -> builds SCRATCH/NAME and sets $FX to it.
fresh() {
    FX="${SCRATCH}/$1"
    make_tree "${FX}"
}

# plant FILE TEXT -> appends TEXT as a new last line of FILE (relative to $FX)
# and sets $PLANT_LINE to its 1-based line number.
plant() {
    mkdir -p "$(dirname "${FX}/$1")"
    printf '%s\n' "$2" >>"${FX}/$1"
    PLANT_LINE="$(wc -l <"${FX}/$1" | tr -d ' ')"
}

# line_of FILE TEXT -> 1-based number of the first line of FILE (relative to
# $FX) containing the literal TEXT.
line_of() {
    grep -nF -- "$2" "${FX}/$1" | head -n1 | cut -d: -f1
}

# tree_sums DIR -> checksums of every file under DIR, for the no-mutation case.
tree_sums() {
    (cd "$1" && find . -type f | LC_ALL=C sort | xargs cksum)
}

# run_guard ROOT -> sets $LAST_OUTPUT and $LAST_STATUS
run_guard() {
    LAST_OUTPUT="$(STUB_PANICS_ROOT="$1" "${GUARD}" 2>&1)"
    LAST_STATUS=$?
}

# assert_fire ROOT DESC NEEDLE... -> non-zero exit AND every NEEDLE appears in
# the output.
assert_fire() {
    local root="$1" desc="$2"
    shift 2
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${root}"
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

# assert_silent ROOT DESC [NEEDLE...] -> zero exit AND every NEEDLE appears.
assert_silent() {
    local root="$1" desc="$2"
    shift 2
    ASSERTIONS=$((ASSERTIONS + 1))
    run_guard "${root}"
    if [ "${LAST_STATUS}" -ne 0 ]; then
        echo "FAIL: expected zero exit (silent) for: ${desc} (got ${LAST_STATUS})"
        echo "${LAST_OUTPUT}" | sed 's/^/  | /'
        FAILED=$((FAILED + 1))
        return
    fi
    local needle
    for needle in "$@"; do
        if ! grep -qF -- "${needle}" <<<"${LAST_OUTPUT}"; then
            echo "FAIL: output for '${desc}' does not contain: ${needle}"
            echo "${LAST_OUTPUT}" | sed 's/^/  | /'
            FAILED=$((FAILED + 1))
            return
        fi
    done
    echo "PASS (silent): ${desc}"
}

# --- Failing cases first: a stub in published production code ---------------

fresh prod-todo
plant crates/x/src/lib.rs 'pub fn f() { todo!() }'
assert_fire "${FX}" "a production todo!() in crates/x/src/lib.rs names file:line" \
    "crates/x/src/lib.rs:${PLANT_LINE}: todo! outside #[cfg(test)] in a published crate" \
    "FOUND"

fresh prod-unimplemented
plant crates/x/src/lib.rs 'pub fn f() { unimplemented!("later") }'
assert_fire "${FX}" "a production unimplemented!() names file:line" \
    "crates/x/src/lib.rs:${PLANT_LINE}: unimplemented! outside #[cfg(test)] in a published crate"

fresh cfg-not-test
plant crates/x/src/lib.rs '#[cfg(not(test))]'
plant crates/x/src/lib.rs 'fn f() { todo!() }'
assert_fire "${FX}" "#[cfg(not(test))] is not exempt" \
    "crates/x/src/lib.rs:${PLANT_LINE}: todo! outside #[cfg(test)]"

fresh reached-module
plant crates/x/src/lib.rs 'pub mod sub;'
plant crates/x/src/sub.rs 'pub fn g() { todo!() }'
assert_fire "${FX}" "a todo!() in sub.rs reached by a plain pub mod sub; is reported" \
    "crates/x/src/sub.rs:${PLANT_LINE}: todo! outside #[cfg(test)]"

fresh spaced-bang
plant crates/x/src/lib.rs 'pub fn f() { todo ! () }'
assert_fire "${FX}" "the spelling todo ! () is matched" \
    "crates/x/src/lib.rs:${PLANT_LINE}: todo! outside #[cfg(test)]"

fresh bracket-form
plant crates/x/src/lib.rs 'pub fn f() { todo![] }'
assert_fire "${FX}" "the spelling todo![] is matched" \
    "crates/x/src/lib.rs:${PLANT_LINE}: todo! outside #[cfg(test)]"

fresh brace-form
plant crates/x/src/lib.rs 'pub fn f() { unimplemented!{} }'
assert_fire "${FX}" "the spelling unimplemented!{} is matched" \
    "crates/x/src/lib.rs:${PLANT_LINE}: unimplemented! outside #[cfg(test)]"

fresh root-package
printf '[package]\nname = "rootpkg"\nversion = "0.1.0"\n' >"${FX}/Cargo.toml"
mkdir -p "${FX}/src"
printf 'pub fn r() { todo!() }\n' >"${FX}/src/lib.rs"
assert_fire "${FX}" "the root package's src/ is scanned too" \
    "src/lib.rs:1: todo! outside #[cfg(test)]"

# --- Named failures ---------------------------------------------------------

fresh no-packages
printf '[workspace]\nmembers = []\n' >"${FX}/Cargo.toml"
rm -rf "${FX}/crates"
assert_fire "${FX}" "a tree with no publishable package is the named failure NO_PACKAGES" \
    "NO_PACKAGES"

fresh zero-files
rm -f "${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "a publishable package with an empty src/ is the named failure ZERO_FILES" \
    "ZERO_FILES"

fresh unreadable
plant crates/x/src/lib.rs 'pub mod bin;'
printf '\xff\xfe' >"${FX}/crates/x/src/bin.rs"
assert_fire "${FX}" "a .rs file holding invalid UTF-8 is UNREADABLE and names its path" \
    "UNREADABLE" "crates/x/src/bin.rs"

fresh only-unpublished
printf '[package]\nname = "x"\nversion = "0.1.0"\npublish = false\n' >"${FX}/crates/x/Cargo.toml"
assert_fire "${FX}" "a tree whose only package is publish = false has no publishable package" \
    "NO_PACKAGES"

# --- Determinism and no-mutation --------------------------------------------

fresh determinism
plant crates/x/src/lib.rs 'pub mod b;'
plant crates/x/src/lib.rs 'pub mod a;'
plant crates/x/src/a.rs 'pub fn a2() { todo!() }'
plant crates/x/src/b.rs 'pub fn b1() { todo!() }'
plant crates/x/src/b.rs 'pub fn b2() { unimplemented!() }'
run_guard "${FX}"
FIRST_OUTPUT="${LAST_OUTPUT}"
run_guard "${FX}"
ASSERTIONS=$((ASSERTIONS + 1))
if [ "${FIRST_OUTPUT}" = "${LAST_OUTPUT}" ] && [ -n "${FIRST_OUTPUT}" ]; then
    echo "PASS (determinism): two runs on one tree print byte-identical output"
else
    echo "FAIL: two runs on the same tree printed different output"
    FAILED=$((FAILED + 1))
fi
ASSERTIONS=$((ASSERTIONS + 1))
ORDER="$(grep -oE '^crates/x/src/[ab]\.rs:[0-9]+' <<<"${FIRST_OUTPUT}" | tr '\n' ' ')"
if [ "${ORDER}" = "crates/x/src/a.rs:1 crates/x/src/b.rs:1 crates/x/src/b.rs:2 " ]; then
    echo "PASS (order): findings are sorted by path, then by line"
else
    echo "FAIL: findings are not sorted by path then line: ${ORDER}"
    FAILED=$((FAILED + 1))
fi

fresh no-mutate-fixture
plant crates/x/src/lib.rs 'pub fn f() { todo!() }'
BEFORE_FX="$(tree_sums "${FX}")"
run_guard "${FX}"
AFTER_FX="$(tree_sums "${FX}")"
ASSERTIONS=$((ASSERTIONS + 1))
if [ "${LAST_STATUS}" -ne 0 ] && [ "${BEFORE_FX}" = "${AFTER_FX}" ]; then
    echo "PASS (no mutation): the fixture tree is byte-identical across a failing run"
else
    echo "FAIL: the fixture tree changed across a failing run (or the run did not fail)"
    FAILED=$((FAILED + 1))
fi

# --- Passing cases ----------------------------------------------------------

fresh clean
assert_silent "${FX}" "a clean fixture tree passes" "OK" "0 finding(s)"

fresh doc-example
plant crates/x/src/lib.rs '/// ```'
plant crates/x/src/lib.rs '/// todo!()'
plant crates/x/src/lib.rs '/// ```'
plant crates/x/src/lib.rs '//! todo!()'
plant crates/x/src/lib.rs '// unimplemented!()'
plant crates/x/src/lib.rs 'pub fn documented() {}'
assert_silent "${FX}" "doc comments and line comments naming the macro pass" "OK"

fresh string-literal
plant crates/x/src/lib.rs 'pub const A: &str = "todo!()";'
assert_silent "${FX}" "a string literal holding todo!() passes" "OK"

fresh raw-string
plant crates/x/src/lib.rs 'pub const B: &str = r#"todo!()"#;'
assert_silent "${FX}" "a raw string holding todo!() passes" "OK"

fresh byte-string
plant crates/x/src/lib.rs 'pub const C: &[u8] = b"todo!()";'
assert_silent "${FX}" "a byte string holding todo!() passes" "OK"

fresh nested-block
plant crates/x/src/lib.rs '/* outer /* todo!() */ still a comment unimplemented!() */'
plant crates/x/src/lib.rs 'pub fn after_block() {}'
assert_silent "${FX}" "a nested block comment holding the macros passes" "OK"

fresh lifetime
plant crates/x/src/lib.rs "pub fn f<'a>(x: &'a str) {}"
plant crates/x/src/lib.rs '#[cfg(test)]'
plant crates/x/src/lib.rs 'mod tests { fn t() { todo!() } }'
assert_silent "${FX}" "a lifetime is not read as a char literal, so the test module after it is still exempt" "OK"

fresh char-literal
plant crates/x/src/lib.rs "pub const Q: char = '\"';"
plant crates/x/src/lib.rs 'pub fn after_char() {}'
assert_silent "${FX}" "a double-quote char literal does not open a string" "OK"

fresh inline-test-mod
plant crates/x/src/lib.rs '#[cfg(test)]'
plant crates/x/src/lib.rs 'mod tests { fn t() { todo!() } }'
assert_silent "${FX}" "a todo!() inside #[cfg(test)] mod tests { } passes" "OK"

fresh all-test-mod
plant crates/x/src/lib.rs '#[cfg(all(test, feature = "mock"))]'
plant crates/x/src/lib.rs 'mod x { fn t() { unimplemented!() } }'
assert_silent "${FX}" "an unimplemented!() inside #[cfg(all(test, ...))] passes" "OK"

fresh out-of-line-test-mod
plant crates/x/src/lib.rs '#[cfg(test)]'
plant crates/x/src/lib.rs 'mod ext;'
printf 'mod deeper;\npub fn e() { todo!() }\n' >"${FX}/crates/x/src/ext.rs"
mkdir -p "${FX}/crates/x/src/ext"
printf 'pub fn d() { unimplemented!() }\n' >"${FX}/crates/x/src/ext/deeper.rs"
assert_silent "${FX}" "an out-of-line test module and its own mod children are test-only" "OK"

fresh out-of-line-mod-rs
plant crates/x/src/lib.rs '#[cfg(test)]'
plant crates/x/src/lib.rs 'mod support;'
mkdir -p "${FX}/crates/x/src/support"
printf 'pub fn s() { todo!() }\n' >"${FX}/crates/x/src/support/mod.rs"
assert_silent "${FX}" "an out-of-line test module resolved to name/mod.rs is test-only" "OK"

fresh inner-cfg-test
printf '#![cfg(test)]\nfn t() { todo!() }\n' >"${FX}/crates/x/src/lib.rs"
assert_silent "${FX}" "a file whose first item is #![cfg(test)] is wholly test-only" "OK"

fresh unpublished-crate
mkdir -p "${FX}/crates/y/src"
printf '[package]\nname = "y"\nversion = "0.1.0"\npublish = false\n' >"${FX}/crates/y/Cargo.toml"
printf 'pub fn y() { todo!() }\n' >"${FX}/crates/y/src/lib.rs"
assert_silent "${FX}" "a stub in a publish = false crate is not reported" "OK"

fresh outside-src
mkdir -p "${FX}/crates/x/tests" "${FX}/crates/x/benches" "${FX}/crates/x/examples"
printf 'fn t() { todo!() }\n' >"${FX}/crates/x/tests/t.rs"
printf 'fn b() { todo!() }\n' >"${FX}/crates/x/benches/b.rs"
printf 'fn e() { todo!() }\n' >"${FX}/crates/x/examples/e.rs"
assert_silent "${FX}" "tests/, benches/ and examples/ trees are out of scope" "OK"

fresh crate-without-manifest
mkdir -p "${FX}/crates/stray/src"
printf 'pub fn s() { todo!() }\n' >"${FX}/crates/stray/src/lib.rs"
assert_silent "${FX}" "a crates/<dir> with no Cargo.toml is skipped silently" "OK"

# --- The real tree ----------------------------------------------------------
# After plan 55-01 Task 2 no published crate holds a stub macro outside test
# code, so the real workspace is green and the guard reads zero findings.

ASSERTIONS=$((ASSERTIONS + 1))
LAST_OUTPUT="$("${GUARD}" 2>&1)"
LAST_STATUS=$?
if [ "${LAST_STATUS}" -ne 0 ]; then
    echo "FAIL: expected zero exit for the real tree (got ${LAST_STATUS})"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
elif ! grep -qF 'OK: scanned' <<<"${LAST_OUTPUT}" || ! grep -qF ', 0 finding(s)' <<<"${LAST_OUTPUT}"; then
    echo "FAIL: the real-tree output carries no 'OK: scanned ..., 0 finding(s)' line"
    echo "${LAST_OUTPUT}" | sed 's/^/  | /'
    FAILED=$((FAILED + 1))
else
    echo "PASS (silent): the real tree passes with zero findings"
fi

ASSERTIONS=$((ASSERTIONS + 1))
if grep -q 'test_support.rs' <<<"${LAST_OUTPUT}"; then
    echo "FAIL: the real-tree output names engine/test_support.rs (out-of-line test module misread)"
    FAILED=$((FAILED + 1))
else
    echo "PASS (real tree): the out-of-line test module engine/test_support.rs is not reported"
fi

# --- The real scanned files must never be mutated by this test. -------------
ASSERTIONS=$((ASSERTIONS + 1))
AFTER_SUMS="$(real_sums)"
if [ -n "${BEFORE_SUMS}" ] && [ "${BEFORE_SUMS}" = "${AFTER_SUMS}" ]; then
    echo "PASS (no mutation): the real scanned files are byte-identical before and after"
else
    echo "FAIL: a real scanned file was mutated by this test run (or none were found)"
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
