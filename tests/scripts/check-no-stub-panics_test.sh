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

# A `#[cfg(test)]` that sits on a non-item (a struct field, an enum variant, a
# match arm, a function parameter, a struct-literal field) exempts nothing: it
# must never claim the enclosing block or the next braced item (WR-03).

fresh cfg-test-struct-field
plant crates/x/src/lib.rs 'pub struct S { #[cfg(test)] probe: u8, real: u8, }'
plant crates/x/src/lib.rs 'impl S { pub fn f(&self) -> u8 { todo!() } }'
assert_fire "${FX}" "#[cfg(test)] on a struct field does not exempt the impl after it" \
    "crates/x/src/lib.rs:${PLANT_LINE}: todo! outside #[cfg(test)]"

fresh cfg-test-enum-variant
plant crates/x/src/lib.rs 'pub enum E { #[cfg(test)] Mock, Real, }'
plant crates/x/src/lib.rs 'pub fn f() { todo!() }'
assert_fire "${FX}" "#[cfg(test)] on an enum variant does not exempt the fn after it" \
    "crates/x/src/lib.rs:${PLANT_LINE}: todo! outside #[cfg(test)]"

fresh cfg-test-match-arm
plant crates/x/src/lib.rs 'pub fn f(e: u8) -> u8 {'
plant crates/x/src/lib.rs '    match e {'
plant crates/x/src/lib.rs '        #[cfg(test)]'
plant crates/x/src/lib.rs '        0 => 0,'
plant crates/x/src/lib.rs '        _ => todo!(),'
STUB_LINE="${PLANT_LINE}"
plant crates/x/src/lib.rs '    }'
plant crates/x/src/lib.rs '}'
assert_fire "${FX}" "#[cfg(test)] on a match arm does not exempt the arm next to it" \
    "crates/x/src/lib.rs:${STUB_LINE}: todo! outside #[cfg(test)]"

fresh cfg-test-fn-param
plant crates/x/src/lib.rs 'pub fn f(#[cfg(test)] _x: u8, y: u8) -> u8 { todo!() }'
assert_fire "${FX}" "#[cfg(test)] on a function parameter does not exempt the fn body" \
    "crates/x/src/lib.rs:${PLANT_LINE}: todo! outside #[cfg(test)]"

fresh cfg-test-struct-literal-field
plant crates/x/src/lib.rs 'pub struct S { pub probe: u8, pub real: u8 }'
plant crates/x/src/lib.rs 'pub fn f() -> S {'
plant crates/x/src/lib.rs '    S {'
plant crates/x/src/lib.rs '        #[cfg(test)]'
plant crates/x/src/lib.rs '        probe: 0,'
plant crates/x/src/lib.rs '        real: unimplemented!(),'
STUB_LINE="${PLANT_LINE}"
plant crates/x/src/lib.rs '    }'
plant crates/x/src/lib.rs '}'
assert_fire "${FX}" "#[cfg(test)] on a struct-literal field does not exempt the field next to it" \
    "crates/x/src/lib.rs:${STUB_LINE}: unimplemented! outside #[cfg(test)]"

fresh cfg-test-item-then-prod
plant crates/x/src/lib.rs '#[cfg(test)]'
plant crates/x/src/lib.rs 'fn t() { todo!() }'
plant crates/x/src/lib.rs 'pub fn f() { todo!() }'
assert_fire "${FX}" "an exempt #[cfg(test)] fn ends at its own closing brace" \
    "crates/x/src/lib.rs:${PLANT_LINE}: todo! outside #[cfg(test)]"

# A match arm whose pattern is a macro looks like a macro-invocation head. A
# `<` in its body or guard must not swallow the neighbouring arm's stub.
fresh cfg-test-macro-pattern-arm
plant crates/x/src/lib.rs 'pub fn f(x: u8) -> bool {'
plant crates/x/src/lib.rs '    match x {'
plant crates/x/src/lib.rs '        #[cfg(test)]'
plant crates/x/src/lib.rs '        m!() => x < 3,'
plant crates/x/src/lib.rs '        _ => todo!(),'
STUB_LINE="${PLANT_LINE}"
plant crates/x/src/lib.rs '    }'
plant crates/x/src/lib.rs '}'
assert_fire "${FX}" "#[cfg(test)] on a macro-pattern match arm with a < in its body exempts nothing" \
    "crates/x/src/lib.rs:${STUB_LINE}: todo! outside #[cfg(test)]"

fresh cfg-test-macro-pattern-guard
plant crates/x/src/lib.rs 'pub fn f(x: u8) -> u8 {'
plant crates/x/src/lib.rs '    match x {'
plant crates/x/src/lib.rs '        #[cfg(test)]'
plant crates/x/src/lib.rs '        m!() if x < 3 => 0,'
plant crates/x/src/lib.rs '        _ => todo!(),'
STUB_LINE="${PLANT_LINE}"
plant crates/x/src/lib.rs '    }'
plant crates/x/src/lib.rs '}'
assert_fire "${FX}" "#[cfg(test)] on a macro-pattern match arm with a < in its guard exempts nothing" \
    "crates/x/src/lib.rs:${STUB_LINE}: todo! outside #[cfg(test)]"

# An inline `const { ... }` block statement has no trailing `;`: it exempts its
# own block only, never the rest of the enclosing fn.
fresh cfg-test-const-block
plant crates/x/src/lib.rs 'pub fn f() {'
plant crates/x/src/lib.rs '    #[cfg(test)] const { () }'
plant crates/x/src/lib.rs '    todo!()'
STUB_LINE="${PLANT_LINE}"
plant crates/x/src/lib.rs '}'
assert_fire "${FX}" "#[cfg(test)] on an inline const block exempts only that block" \
    "crates/x/src/lib.rs:${STUB_LINE}: todo! outside #[cfg(test)]"

# A `#[cfg(test)]` inside a generic parameter list sits after `<` or `,`, not
# at an item position, so it exempts nothing: not the rest of the list, and
# never the production items up to the next `;` later in the file (WR-03,
# round 3). Each case ends with a later top-level `use ...;` so a scan that
# ran on to a `;` would swallow the stubs before it.
fresh cfg-test-const-generic
printf '%s\n' \
    'pub fn g<#[cfg(test)] const N: usize>() {}' \
    'pub fn a() { todo!() }' \
    'pub fn b() { unimplemented!() }' \
    'use std::fmt as _f;' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "#[cfg(test)] on a const generic parameter exempts no later item" \
    "crates/x/src/lib.rs:2: todo! outside #[cfg(test)]" \
    "crates/x/src/lib.rs:3: unimplemented! outside #[cfg(test)]"

fresh cfg-test-type-generic
plant crates/x/src/lib.rs 'pub fn h<#[cfg(test)] T>() {}'
plant crates/x/src/lib.rs 'pub fn a() { todo!() }'
STUB_LINE="${PLANT_LINE}"
plant crates/x/src/lib.rs 'use std::fmt as _f;'
assert_fire "${FX}" "#[cfg(test)] on a type generic parameter exempts no later item" \
    "crates/x/src/lib.rs:${STUB_LINE}: todo! outside #[cfg(test)]"

fresh cfg-test-lifetime-generic
plant crates/x/src/lib.rs "pub fn l<#[cfg(test)] 'a>() {}"
plant crates/x/src/lib.rs 'pub fn a() { todo!() }'
STUB_LINE="${PLANT_LINE}"
plant crates/x/src/lib.rs 'use std::fmt as _f;'
assert_fire "${FX}" "#[cfg(test)] on a lifetime parameter exempts no later item" \
    "crates/x/src/lib.rs:${STUB_LINE}: todo! outside #[cfg(test)]"

fresh cfg-test-impl-const-generic
plant crates/x/src/lib.rs 'pub struct S;'
plant crates/x/src/lib.rs 'impl<#[cfg(test)] const N: usize> S {}'
plant crates/x/src/lib.rs 'pub fn a() { todo!() }'
STUB_LINE="${PLANT_LINE}"
plant crates/x/src/lib.rs 'use std::fmt as _f;'
assert_fire "${FX}" "#[cfg(test)] on a const generic in an impl header exempts no later item" \
    "crates/x/src/lib.rs:${STUB_LINE}: todo! outside #[cfg(test)]"

# Deliberate, strict side: an expression statement is not an item, so it
# exempts nothing even though rustc compiles it out of a non-test build. Wrap
# it in a block (`#[cfg(test)] { ... }`, a PASS case below) instead.
fresh cfg-test-expression-statement
plant crates/x/src/lib.rs 'pub fn f() { #[cfg(test)] self_check(|| todo!()); }'
assert_fire "${FX}" "#[cfg(test)] on an expression statement is reported (strict by design)" \
    "crates/x/src/lib.rs:${PLANT_LINE}: todo! outside #[cfg(test)]"

# The raw identifier `r#where` is not the `where` keyword: it must not open a
# where clause that stops the top-level `,` ending the scan (WR-03, round 4).
fresh cfg-test-raw-where
printf '%s\n' \
    'pub struct S { #[cfg(test)] pub union: u8, pub other: u8 }' \
    'pub fn make(r#where: u8) -> S {' \
    '    S {' \
    '        #[cfg(test)]' \
    '        union: r#where,' \
    '        other: todo!(),' \
    '    }' \
    '}' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "the raw identifier r#where opens no where clause" \
    "crates/x/src/lib.rs:6: todo! outside #[cfg(test)]"

# `union` is only a contextual keyword: as a struct-literal field name or a
# path segment it heads no item, so it exempts nothing (WR-03, round 4).
fresh cfg-test-union-field
printf '%s\n' \
    'pub struct S { #[cfg(test)] pub union: bool, pub other: u8 }' \
    'pub fn make(a: u8, b: u8) -> S {' \
    '    S {' \
    '        #[cfg(test)]' \
    '        union: a < b,' \
    '        other: todo!(),' \
    '    }' \
    '}' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "#[cfg(test)] on a struct-literal field named union exempts nothing" \
    "crates/x/src/lib.rs:6: todo! outside #[cfg(test)]"

fresh cfg-test-union-path-arm
printf '%s\n' \
    "pub mod union { pub const A: char = 'a'; }" \
    'pub fn classify(c: char) -> u8 {' \
    '    match c {' \
    '        #[cfg(test)]' \
    "        union::A | 'b' | 'c' => 0," \
    "        'd' => todo!()," \
    '        _ => 1 | 2,' \
    '    }' \
    '}' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "#[cfg(test)] on a match arm whose pattern is the path union::A exempts nothing" \
    "crates/x/src/lib.rs:6: todo! outside #[cfg(test)]"

# `#![cfg(test)]` makes a file test-only only in its leading run of inner
# attributes. Inside macro tokens, or anywhere after the first item, it is not
# the file's own attribute (WR-03, round 4).
fresh inner-cfg-test-in-macro-tokens
printf '%s\n' \
    'pub fn f() -> u8 { todo!() }' \
    'pub const NOTE: &str = stringify!(#![cfg(test)]);' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "#![cfg(test)] inside macro tokens after an item does not make the file test-only" \
    "crates/x/src/lib.rs:1: todo! outside #[cfg(test)]"

# Inside a macro invocation's delimiter group or a macro_rules! body the tokens
# are macro input, not items: a #[cfg(test)] there exempts nothing and records
# no out-of-line test module, because the macro decides what (if anything) it
# expands to (WR-03, round 4).
fresh cfg-test-mod-in-macro-tokens
printf '%s\n' \
    'pub mod prod;' \
    'pub const NOTE: &str = stringify!{ #[cfg(test)] mod prod; };' >"${FX}/crates/x/src/lib.rs"
printf 'pub fn p() -> u8 { todo!() }\n' >"${FX}/crates/x/src/prod.rs"
assert_fire "${FX}" "#[cfg(test)] mod x; inside macro tokens does not make x.rs test-only" \
    "crates/x/src/prod.rs:1: todo! outside #[cfg(test)]"

fresh cfg-test-mod-in-macro-rules-body
printf '%s\n' \
    'pub mod prod;' \
    'macro_rules! never { () => { #[cfg(test)] mod prod; }; }' >"${FX}/crates/x/src/lib.rs"
printf 'pub fn p() -> u8 { todo!() }\n' >"${FX}/crates/x/src/prod.rs"
assert_fire "${FX}" "#[cfg(test)] mod x; inside a macro_rules! body does not make x.rs test-only" \
    "crates/x/src/prod.rs:1: todo! outside #[cfg(test)]"

fresh cfg-test-in-macro-input
printf '%s\n' \
    'macro_rules! strip { (#[cfg(test)] $i:item) => { $i }; }' \
    'strip! {' \
    '    #[cfg(test)]' \
    '    pub fn f() -> u8 { todo!() }' \
    '}' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "#[cfg(test)] inside a macro invocation's input exempts nothing" \
    "crates/x/src/lib.rs:4: todo! outside #[cfg(test)]"

# A `#[cfg(test)]` out-of-line module with a `#[path = "..."]` attribute loads
# that literal path, never `name.rs`: the mock-swap pattern must not exempt the
# production file of the same name (WR-03, round 4).
fresh cfg-test-path-mod
printf '%s\n' \
    '#[cfg(not(test))]' \
    'pub mod imp;' \
    '#[cfg(test)]' \
    '#[path = "imp_mock.rs"]' \
    'pub mod imp;' >"${FX}/crates/x/src/lib.rs"
printf 'pub fn run() -> u8 { todo!() }\n' >"${FX}/crates/x/src/imp.rs"
printf 'pub fn run() -> u8 { 0 }\n' >"${FX}/crates/x/src/imp_mock.rs"
assert_fire "${FX}" "#[cfg(test)] #[path] mod imp; does not make the production imp.rs test-only" \
    "crates/x/src/imp.rs:1: todo! outside #[cfg(test)]"

fresh cfg-test-path-first-mod
printf '%s\n' \
    '#[cfg(not(test))]' \
    'pub mod imp;' \
    '#[path = "imp_mock.rs"]' \
    '#[cfg(test)]' \
    'pub mod imp;' >"${FX}/crates/x/src/lib.rs"
printf 'pub fn run() -> u8 { todo!() }\n' >"${FX}/crates/x/src/imp.rs"
printf 'pub fn run() -> u8 { 0 }\n' >"${FX}/crates/x/src/imp_mock.rs"
assert_fire "${FX}" "#[path] before #[cfg(test)] on mod imp; does not make imp.rs test-only either" \
    "crates/x/src/imp.rs:1: todo! outside #[cfg(test)]"

# An out-of-line module declared inside an inline `mod outer { ... }` resolves
# under outer/, as rustc reads it: it never exempts the same-named production
# file beside the declaring file (WR-03, round 4).
fresh cfg-test-mod-in-inline-mod
printf '%s\n' \
    'pub mod outer {' \
    '    #[cfg(test)]' \
    '    mod helper;' \
    '}' \
    'pub mod helper;' >"${FX}/crates/x/src/lib.rs"
printf 'pub fn h() -> u8 { todo!() }\n' >"${FX}/crates/x/src/helper.rs"
mkdir -p "${FX}/crates/x/src/outer"
printf 'fn t() {}\n' >"${FX}/crates/x/src/outer/helper.rs"
assert_fire "${FX}" "#[cfg(test)] mod helper; inside inline mod outer does not make src/helper.rs test-only" \
    "crates/x/src/helper.rs:1: todo! outside #[cfg(test)]"

# A file is test-only only when every module declaration that loads it is
# test-only. One production declaration anywhere (beside a test one, as the
# target of a test alias, or beside a test-only file's own child declaration)
# compiles it into the published crate (WR-03, round 5).
fresh test-and-prod-decl-same-file
printf '%s\n' \
    '#[cfg(test)]' \
    'mod backend;' \
    '#[cfg(not(test))]' \
    'mod backend;' \
    'pub use backend::f;' >"${FX}/crates/x/src/lib.rs"
printf 'pub fn f() -> u8 { todo!() }\n' >"${FX}/crates/x/src/backend.rs"
assert_fire "${FX}" "a file declared by both a test and a production mod is not test-only" \
    "crates/x/src/backend.rs:1: todo! outside #[cfg(test)]"

fresh test-path-alias-of-prod-file
printf '%s\n' \
    'mod clock;' \
    '#[cfg(test)]' \
    '#[path = "clock.rs"]' \
    'mod clock_under_test;' \
    'pub use clock::now;' >"${FX}/crates/x/src/lib.rs"
printf 'pub fn now() -> u64 { todo!() }\n' >"${FX}/crates/x/src/clock.rs"
assert_fire "${FX}" "a test-only #[path] alias of a production file does not make it test-only" \
    "crates/x/src/clock.rs:1: todo! outside #[cfg(test)]"

fresh test-only-child-shares-prod-file
printf '#[cfg(test)]\nmod support;\npub mod util;\n' >"${FX}/crates/x/src/lib.rs"
printf '#[path = "util.rs"]\nmod u;\n' >"${FX}/crates/x/src/support.rs"
printf 'pub fn f() -> u8 { todo!() }\n' >"${FX}/crates/x/src/util.rs"
assert_fire "${FX}" "a test-only file's child that production also declares is not test-only" \
    "crates/x/src/util.rs:1: todo! outside #[cfg(test)]"

# `include!("gen.rs")` compiles gen.rs into the crate where it stands, so a
# production include! is a production loader like a production mod (WR-03,
# round 5).
fresh include-of-test-mod-file
printf '#[cfg(test)]\nmod gen;\ninclude!("gen.rs");\n' >"${FX}/crates/x/src/lib.rs"
printf 'pub fn g() -> u8 { todo!() }\n' >"${FX}/crates/x/src/gen.rs"
assert_fire "${FX}" "a file that a test mod declares and production include!s is not test-only" \
    "crates/x/src/gen.rs:1: todo! outside #[cfg(test)]"

fresh include-outside-src
printf 'include!("../gen/g.rs");\n' >"${FX}/crates/x/src/lib.rs"
mkdir -p "${FX}/crates/x/gen"
printf 'pub fn g() -> u8 { todo!() }\n' >"${FX}/crates/x/gen/g.rs"
assert_fire "${FX}" "a production include! target outside src/ is scanned" \
    "crates/x/gen/g.rs:1: todo! outside #[cfg(test)]"

# A crate root is compiled whatever a test module declares, so a test alias of
# main.rs never makes the binary's root test-only.
fresh test-alias-of-crate-root
printf '#[cfg(test)]\n#[path = "main.rs"]\nmod bin_under_test;\n' >"${FX}/crates/x/src/lib.rs"
printf 'fn main() { todo!() }\n' >"${FX}/crates/x/src/main.rs"
assert_fire "${FX}" "a test-only #[path] alias of src/main.rs does not make the crate root test-only" \
    "crates/x/src/main.rs:1: todo! outside #[cfg(test)]"

# A production `#[path]` module compiles its target into the published crate
# even when the file sits outside src/ (eval.rs pulls in
# tests/helpers/e2e_fixtures.rs this way), so the target and its own children
# are scanned (WR-03, round 4).
fresh path-mod-outside-src
printf '#[path = "../tests/helpers/fx.rs"]\npub mod fx;\n' >"${FX}/crates/x/src/lib.rs"
mkdir -p "${FX}/crates/x/tests/helpers"
printf 'pub mod sub;\npub fn f() -> u8 { todo!() }\n' >"${FX}/crates/x/tests/helpers/fx.rs"
printf 'pub fn s() { unimplemented!() }\n' >"${FX}/crates/x/tests/helpers/sub.rs"
assert_fire "${FX}" "a production #[path] target outside src/, and its child, are scanned" \
    "crates/x/tests/helpers/fx.rs:2: todo! outside #[cfg(test)]" \
    "crates/x/tests/helpers/sub.rs:1: unimplemented! outside #[cfg(test)]"

# A raw C string `cr"..."` has no escapes: its `\"` must not run the literal on
# to a later quote and hide the code in between (WR-03, round 4).
fresh raw-c-string
printf '%s\n' \
    'pub fn f() -> &'"'"'static core::ffi::CStr { cr"C:\" }' \
    'pub fn g() -> u8 { todo!() }' \
    'pub fn h() -> &'"'"'static str { "x" }' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "a raw C string ending in a backslash does not swallow the next line" \
    "crates/x/src/lib.rs:2: todo! outside #[cfg(test)]"

# The macro can be reached under another name: a `use` that renames `todo` or
# `unimplemented`, or the bare identifier handed to a macro that builds the
# call (`call!(todo)` expanding `$m!()`). Both are reported (WR-03, round 4).
fresh renamed-stub-import
printf '%s\n' \
    'use core::todo as later;' \
    'pub use std::{fmt as _f, unimplemented as nope};' \
    'pub fn f() -> u8 { later!() }' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "a use that renames todo or unimplemented is reported" \
    "crates/x/src/lib.rs:1: todo! (renamed by use ... as) outside #[cfg(test)]" \
    "crates/x/src/lib.rs:2: unimplemented! (renamed by use ... as) outside #[cfg(test)]"

fresh stub-ident-macro-token
printf '%s\n' \
    'macro_rules! call { ($m:ident) => { $m!() }; }' \
    'pub fn f() -> u8 { call!(todo) }' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "todo handed to a macro as a bare identifier is reported" \
    "crates/x/src/lib.rs:2: todo! (passed to a macro as an identifier) outside #[cfg(test)]"

# The bare identifier followed by another token is still the macro's name to a
# matcher that takes it apart (`($m:ident $x:ident) => { $m!() }`): an
# identifier or a literal after it does not clear it (WR-03, round 5).
fresh stub-ident-before-token
printf '%s\n' \
    'macro_rules! call { ($m:ident $x:ident) => { $m!() }; }' \
    'macro_rules! lit { ($m:ident $x:literal) => { $m!() }; }' \
    'pub fn f() -> u8 { call!(todo now) }' \
    'pub fn g() -> u8 { lit!(unimplemented 1) }' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "todo or unimplemented handed to a macro before an identifier or a literal is reported" \
    "crates/x/src/lib.rs:3: todo! (passed to a macro as an identifier) outside #[cfg(test)]" \
    "crates/x/src/lib.rs:4: unimplemented! (passed to a macro as an identifier) outside #[cfg(test)]"

# `todo!` with no delimiter after the `!` is no direct call, but a matcher such
# as `($m:ident !) => { $m!() }` builds one from it (WR-03, round 5).
fresh stub-ident-bang-no-delimiter
printf '%s\n' \
    'macro_rules! call { ($m:ident !) => { $m!() }; }' \
    'pub fn f() -> u8 { call!(todo!) }' \
    'pub fn g() -> u8 { call!(unimplemented !) }' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "todo! or unimplemented ! handed to a macro with no delimiter is reported" \
    "crates/x/src/lib.rs:2: todo! (passed to a macro as an identifier) outside #[cfg(test)]" \
    "crates/x/src/lib.rs:3: unimplemented! (passed to a macro as an identifier) outside #[cfg(test)]"

# A path whose last segment is the macro (`core::todo`, not followed by `::`)
# is the macro to a matcher that pastes it before a `!` (WR-03, round 5).
fresh stub-ident-path-last-segment
printf '%s\n' \
    'macro_rules! call { ($($p:tt)*) => { $($p)*!() }; }' \
    'pub fn f() -> u8 { call!(core::todo) }' \
    'pub fn g() -> u8 { call!(::core::unimplemented) }' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "a path ending in todo or unimplemented handed to a macro is reported" \
    "crates/x/src/lib.rs:2: todo! (passed to a macro as an identifier) outside #[cfg(test)]" \
    "crates/x/src/lib.rs:3: unimplemented! (passed to a macro as an identifier) outside #[cfg(test)]"

# Strict by design, as the header says: a `use` that renames a module named
# `todo` cannot be told from a rename of the macro, so it is reported too.
fresh renamed-todo-module
printf '%s\n' \
    'pub mod todo { pub fn f() {} }' \
    'use crate::todo as t;' \
    'pub fn g() { t::f() }' >"${FX}/crates/x/src/lib.rs"
assert_fire "${FX}" "a use that renames a module named todo is reported (strict by design)" \
    "crates/x/src/lib.rs:2: todo! (renamed by use ... as) outside #[cfg(test)]"

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

fresh c-strings
plant crates/x/src/lib.rs 'pub const D: &core::ffi::CStr = c"todo!()";'
plant crates/x/src/lib.rs 'pub const E: &core::ffi::CStr = cr#"unimplemented!()"#;'
assert_silent "${FX}" "C strings and raw C strings holding the macros pass" "OK"

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

# rustc skips a leading UTF-8 byte-order mark, so the attribute right after it
# is still at the start of the file, an item position (WR-03, round 4).
fresh bom-test-mod
printf '\xef\xbb\xbf#[cfg(test)]\nmod tests { fn t() { todo!() } }\n' >"${FX}/crates/x/src/lib.rs"
assert_silent "${FX}" "a #[cfg(test)] mod right after a leading byte-order mark is exempt" "OK"

fresh all-test-mod
plant crates/x/src/lib.rs '#[cfg(all(test, feature = "mock"))]'
plant crates/x/src/lib.rs 'mod x { fn t() { unimplemented!() } }'
assert_silent "${FX}" "an unimplemented!() inside #[cfg(all(test, ...))] passes" "OK"

fresh item-heads
plant crates/x/src/lib.rs '#[cfg(test)]'
plant crates/x/src/lib.rs '#[inline]'
plant crates/x/src/lib.rs 'pub(crate) async fn h() { todo!() }'
plant crates/x/src/lib.rs '#[cfg(test)] pub(in crate) const unsafe fn k() { todo!() }'
plant crates/x/src/lib.rs '#[cfg(test)] unsafe impl Send for W<u8, u16> { fn z() { todo!() } }'
plant crates/x/src/lib.rs '#[cfg(test)] thread_local! { static T: u8 = todo!(); }'
plant crates/x/src/lib.rs '#[cfg(test)] std::thread_local! { static U: u8 = todo!(); }'
plant crates/x/src/lib.rs '#[cfg(test)] macro_rules! m { () => { todo!() }; }'
plant crates/x/src/lib.rs '#[cfg(test)] const Q: u8 = todo!();'
plant crates/x/src/lib.rs '#[cfg(test)] union U { a: [u8; todo!()] }'
plant crates/x/src/lib.rs 'pub fn body() { #[cfg(test)] let _v: u8 = if true { 1 } else { todo!() }; }'
plant crates/x/src/lib.rs '#[cfg(test)]'
plant crates/x/src/lib.rs 'fn bounded<A, B>(a: A, b: B) -> u8'
plant crates/x/src/lib.rs 'where'
plant crates/x/src/lib.rs '    A: Clone,'
plant crates/x/src/lib.rs '    B: Copy + '"'"'static,'
plant crates/x/src/lib.rs '{ todo!() }'
assert_silent "${FX}" "#[cfg(test)] on a real item head (visibility, qualifiers, macro, let, union) is exempt" "OK"

# A closure parameter list `|a, b|` in a statement initialiser has its commas
# at depth 0; they must not end the exempt statement early.
fresh closure-let
plant crates/x/src/lib.rs 'pub fn h() { #[cfg(test)] let g = |a: u8, b: u8| -> u8 { todo!() }; }'
assert_silent "${FX}" "a #[cfg(test)] let whose closure takes two parameters stays exempt" "OK"

fresh closure-const
plant crates/x/src/lib.rs '#[cfg(test)] const G: fn(u8, u8) -> u8 = |a, b| todo!();'
assert_silent "${FX}" "a #[cfg(test)] const whose closure takes two parameters stays exempt" "OK"

fresh closure-move-let
plant crates/x/src/lib.rs 'pub fn h() { #[cfg(test)] let g = move |a: (u8, u8), b: Vec<u8>| unimplemented!(); }'
assert_silent "${FX}" "a #[cfg(test)] let whose move closure takes two parameters stays exempt" "OK"

# A block statement (plain, unsafe, async or const) exempts exactly its block.
fresh block-statement
plant crates/x/src/lib.rs 'pub fn b() {'
plant crates/x/src/lib.rs '    #[cfg(test)] { todo!() }'
plant crates/x/src/lib.rs '    #[cfg(test)] unsafe { todo!() }'
plant crates/x/src/lib.rs '    #[cfg(test)] const { todo!() }'
plant crates/x/src/lib.rs '}'
assert_silent "${FX}" "a #[cfg(test)] block statement exempts its own block" "OK"

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

fresh out-of-line-path-test-mod
printf '%s\n' \
    '#[cfg(not(test))]' \
    'pub mod imp;' \
    '#[cfg(test)]' \
    '#[path = "mocks/imp_mock.rs"]' \
    'pub mod imp;' >"${FX}/crates/x/src/lib.rs"
printf 'pub fn run() -> u8 { 0 }\n' >"${FX}/crates/x/src/imp.rs"
mkdir -p "${FX}/crates/x/src/mocks"
printf 'mod helper;\npub fn run() -> u8 { todo!() }\n' >"${FX}/crates/x/src/mocks/imp_mock.rs"
printf 'pub fn h() { unimplemented!() }\n' >"${FX}/crates/x/src/mocks/helper.rs"
assert_silent "${FX}" "a #[cfg(test)] #[path] module and its own children (beside it, as rustc reads a #[path] file) are test-only" "OK"

fresh out-of-line-test-mod-in-inline-mod
printf 'pub mod outer {\n    #[cfg(test)]\n    mod helper;\n}\npub mod a;\n' >"${FX}/crates/x/src/lib.rs"
mkdir -p "${FX}/crates/x/src/outer" "${FX}/crates/x/src/a/inner"
printf 'pub fn h() { todo!() }\n' >"${FX}/crates/x/src/outer/helper.rs"
printf 'pub mod inner {\n    #[cfg(test)]\n    mod t;\n}\n' >"${FX}/crates/x/src/a.rs"
printf 'fn t() { unimplemented!() }\n' >"${FX}/crates/x/src/a/inner/t.rs"
assert_silent "${FX}" "an out-of-line test module inside an inline module resolves under its directory (lib.rs and a non-mod-rs file)" "OK"

# A plain `mod x;` declared inside an exempt `#[cfg(test)] mod tests { ... }`
# is gone from a non-test build with its parent, so x.rs is test-only.
fresh mod-decl-in-exempt-inline-mod
printf '#[cfg(test)]\nmod tests {\n    mod x;\n}\n' >"${FX}/crates/x/src/lib.rs"
mkdir -p "${FX}/crates/x/src/tests"
printf 'fn t() { todo!() }\n' >"${FX}/crates/x/src/tests/x.rs"
assert_silent "${FX}" "a mod declared inside an exempt inline test module loads a test-only file" "OK" "(1 test-only)"

fresh inner-cfg-test
printf '#![cfg(test)]\nfn t() { todo!() }\n' >"${FX}/crates/x/src/lib.rs"
assert_silent "${FX}" "a file whose first item is #![cfg(test)] is wholly test-only" "OK"

fresh inner-cfg-test-after-inner-attrs
printf '//! Test-only crate.\n#![forbid(unsafe_code)]\n#![cfg(test)]\nfn t() { todo!() }\n' >"${FX}/crates/x/src/lib.rs"
assert_silent "${FX}" "#![cfg(test)] after a doc comment and another inner attribute is still leading" "OK"

# An include! inside an exempt span loads its file for a test build only.
fresh include-in-test-mod
printf '%s\n' \
    '#[cfg(test)]' \
    'mod tests {' \
    '    include!("tests_body.rs");' \
    '}' \
    '#[cfg(test)]' \
    'include!("../gen/g.rs");' >"${FX}/crates/x/src/lib.rs"
printf 'fn t() { todo!() }\n' >"${FX}/crates/x/src/tests_body.rs"
mkdir -p "${FX}/crates/x/gen"
printf 'fn u() { todo!() }\n' >"${FX}/crates/x/gen/g.rs"
assert_silent "${FX}" "a file include!d only inside #[cfg(test)] is test-only and not pulled in from outside src/" \
    "OK" "scanned 2 file(s) (1 test-only)"

fresh stub-names-not-macros
plant crates/x/src/lib.rs '#[cfg(test)] use core::todo as later;'
plant crates/x/src/lib.rs 'pub mod todo { pub fn f() {} }'
plant crates/x/src/lib.rs 'use crate::todo::f as g;'
plant crates/x/src/lib.rs 'pub struct T { pub todo: u8 }'
plant crates/x/src/lib.rs 'pub fn h(t: &T) -> u8 { todo::f(); g(); m!(t.todo, crate::todo::f) }'
assert_silent "${FX}" "a cfg(test) rename, a module or field named todo and a path through it pass" "OK"

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

fresh test-path-mod-outside-src
printf '%s\n' \
    '#[cfg(test)]' \
    '#[path = "../tests/helpers/fx.rs"]' \
    'mod fx;' \
    '#[cfg(test)]' \
    'mod tests {' \
    '    #[path = "../tests/helpers/gx.rs"]' \
    '    mod gx;' \
    '}' >"${FX}/crates/x/src/lib.rs"
mkdir -p "${FX}/crates/x/tests/helpers"
printf 'pub fn f() -> u8 { todo!() }\n' >"${FX}/crates/x/tests/helpers/fx.rs"
printf 'pub fn g() -> u8 { todo!() }\n' >"${FX}/crates/x/tests/helpers/gx.rs"
assert_silent "${FX}" "a test-only #[path] target outside src/ is not pulled into the scan" "OK" "scanned 1 file(s)"

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
