#!/usr/bin/env bash
# check-no-stub-panics.sh
#
# Offline, check-only gate that holds every published crate free of stub
# panics (Phase 55, FIXES-01, D-01). A `todo!()` or `unimplemented!()` that is
# reachable outside test code takes down the caller's thread the first time
# the unfinished path runs (B2, DF-03: the twenty ContentListRepository stubs).
# This gate turns a new stub into a red `make check-gates` and a red CI job
# instead of a one-off sweep that rots.
#
# The rule: a `todo!` or `unimplemented!` macro token on a code line of any
# `.rs` file under `src/` of a published package is a finding, printed as
# `<path>:<line>: ...`, and the script exits 1. A published package is one whose
# manifest `[package] publish` is not `false`: the root Cargo.toml plus every
# crates/*/Cargo.toml, read with tomllib (a `publish.workspace = true` package
# inherits `[workspace.package] publish`). `crates/doc-examples` (publish =
# false) and every tests/, benches/ and examples/ tree are out of scope, except
# a file that a scanned production module loads from outside src/ through
# `#[path = "..."]` (or as a child module of such a file): rustc compiles it
# into the published crate, so it is scanned too (src/application/cli/
# commands/eval.rs loads tests/helpers/e2e_fixtures.rs this way). A
# `#[cfg(test)]` one, or one declared inside an exempt span, is not followed. A
# crates/<dir> with no readable Cargo.toml is skipped silently.
#
# Exactly two cfg forms exempt code, and only the item or module block they
# attach to: `#[cfg(test)]` and `#[cfg(all(test, ...))]`. `#[cfg(not(test))]`
# is NOT exempt: it is production code. A file whose leading run of inner
# attributes (only whitespace, comments and other `#![...]` before it) holds
# `#![cfg(test)]` is wholly test-only; the same tokens anywhere else, such as
# inside a macro invocation's tokens, are not the file's attribute.
#
# Macro input first: inside a macro invocation's delimiter group (`m!( )`,
# `m![ ]`, `m!{ }`) or a `macro_rules!` body the tokens are macro input, not
# items, so a `#[cfg(test)]` there exempts nothing and records no out-of-line
# test module (the macro decides what it expands to).
#
# Position next: the attribute exempts something only at an item position, that
# is, when the previous significant character before it (looking back over
# whitespace, comments and any preceding `#[...]`/`#![...]` attributes) is the
# start of the file (after a leading byte-order mark, if any), `;`, `{` or `}`.
# After `<`, `(`, `,`, `|`, `=` or anything else it sits inside a generic
# parameter list (`fn g<#[cfg(test)] const N: usize>`, a type or lifetime
# parameter, an impl header), a parameter list, a field, variant or arm list,
# closure parameters or an initialiser, and exempts nothing. Then the head: at
# an item position the attribute exempts something only when (after any further
# attributes) a real item head follows: optional visibility, optional
# `async`/`unsafe`/`const`/`default`/`extern "abi"`, then `fn`, `mod`, `impl`,
# `struct`, `enum`, `union` (only as `union Name` followed by `<`, `{` or
# `where`: it is a contextual keyword, so a field `union: x` or a path
# `union::A` is no head), `trait`, `type`, `const`, `static`, `use`, `extern`,
# `let`, `macro_rules!` or a macro invocation such as `thread_local!`, or a
# block statement (`{`, `unsafe {`, `async {`, inline `const {`). A block
# statement exempts exactly its block; a macro invocation exactly its delimiter
# group (and a `;` right after it); a `let`/`const`/`static`/`use`/`type` up to
# its top-level `;` (a closure parameter list `|a, b|` inside it is read whole),
# and one with no `;` before the enclosing block or the file ends exempts
# nothing. A struct field, enum variant or match arm that passes the position
# rule (the first of its list after `{`, or an arm after a `}`-bodied arm) is no
# item head, so it exempts nothing; an arm whose pattern is a macro exempts only
# that macro's group. An expression exempts nothing. The item scan stops at a
# top-level `=>`, a top-level `,` or an unmatched closer, so it never runs past
# the enclosing block.
#
# Strict by design: `#[cfg(test)]` on an expression statement such as
# `self_check(|| todo!());` is not an item, so a stub inside it is reported
# even though rustc compiles it out of a non-test build. Wrap it in a block,
# `#[cfg(test)] { self_check(|| todo!()); }`, which is exempt.
#
# Out-of-line test modules: `#[cfg(test)] mod name;` resolves as rustc reads
# it, to `name.rs` or `name/mod.rs` in the declaring file's module directory
# (its own directory for lib.rs, main.rs and mod.rs, otherwise the path
# without `.rs`) plus the inline `mod outer { ... }` nesting around the
# declaration: `mod outer { #[cfg(test)] mod helper; }` in lib.rs is
# outer/helper.rs, never a production helper.rs beside lib.rs. With a
# `#[path = "..."]` attribute in its attribute run (before or after the cfg) it
# resolves to that literal path only, relative to the declaring file's
# directory (or the nested directory inside an inline module), never to
# `name.rs`: `#[cfg(test)] #[path = "imp_mock.rs"] mod imp;` makes imp_mock.rs
# test-only and leaves a production imp.rs scanned. A file loaded through
# `#[path]` resolves its own children beside it, as rustc reads it like a
# mod.rs. A `#[path]` on an inline module itself is not followed. Those files
# are test-only, transitively: a test-only file's own `mod child;` children
# are test-only too. So crates/paladin-battalion/src/engine/test_support.rs
# reports nothing.
#
# Matching runs on the macro token after line comments, doc comments (so a
# `///` doc example never fires), nested block comments, string, byte-string,
# C-string, raw-string (including raw byte and raw C strings) and char literals
# are blanked to spaces with newlines kept (the technique of
# tests/entity_rename_guard.rs). `todo ! (`, `todo![` and `unimplemented!{` all
# match; a lifetime such as `'a` is not read as a char literal.
#
# Never vacuous: besides FOUND and OK there are three named failures, in
# precedence order. NO_PACKAGES (no publishable package under the root),
# ZERO_FILES (the publishable packages hold no `.rs` file under `src/`) and
# UNREADABLE (a `.rs` file is not valid UTF-8 or cannot be read: it is named,
# never skipped). The status is the first applicable of NO_PACKAGES,
# ZERO_FILES, UNREADABLE, FOUND, then OK; every named failure and every finding
# is listed before exit, sorted by path and then by line, so two runs on one
# tree print byte-identical output.
#
# Renamed and indirect calls: a `use` declaration that renames the macro
# (`use core::todo as later;`, `use std::{unimplemented as nope}`) is a finding
# at the rename, and so is the bare identifier `todo` or `unimplemented` inside
# a macro invocation's tokens or a `macro_rules!` body (`call!(todo)`, whose
# macro may expand `$m!()`), each named in the finding line. A path segment, a
# field or a `$todo` metavariable is not the macro. Strict by design: a local
# variable named `todo` handed to a macro (`println!("{}", todo)`) is reported
# too; rename it. A call built some other way (a proc macro, `concat_idents!`)
# is beyond a lexical gate.
#
# There is NO exemption list, no inline allow marker, no environment switch and
# no path filter (D-01). To clear a finding, replace the stub with a typed `Err`
# or delete the dead code.
#
# Offline: no network, no cargo; python3 with tomllib only. Check-only: this
# script never writes to, rewrites or reformats a file it scans, takes no
# argument, and has no write mode, so an interrupted run, or the CI step and
# `make check-gates` running at once, leave the tree unchanged.
#
# Test-only seam: STUB_PANICS_ROOT relocates every input (the manifests and the
# scanned files) to a fixture tree. CI and the Makefile target set no
# environment, so the default is the repository root.
#
# Usage:  ./scripts/check-no-stub-panics.sh
# Exit:   0 when no stub macro is reachable outside test code, 1 otherwise.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ROOT="${STUB_PANICS_ROOT:-${WORKSPACE_ROOT}}"

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required for Cargo.toml parsing." >&2
    exit 1
fi

echo "🔍 Checking published crates for stub panics outside #[cfg(test)] ..."

REPORT=$(python3 - "${ROOT}" <<'PY'
import glob
import os
import re
import sys
import tomllib

root = os.path.abspath(sys.argv[1])

# `todo!` / `unimplemented!` followed by an opening delimiter. `\b` keeps
# `my_todo!` and `unimplemented_x!` out.
STUB = re.compile(r'\b(todo|unimplemented)\s*!\s*[\(\[{]')
# The same macros under another name: a `use` declaration that renames one
# (`use core::todo as later;`), and the bare identifier handed to a macro,
# which can build the call from it (`call!(todo)` expanding `$m!()`). A path
# segment (`crate::todo::f`, `todo::f`), a field (`t.todo`, `todo: u8`) or a
# `$todo` metavariable is not the macro.
USE_DECL = re.compile(r'(?<![\w$])use\b[^;]*;')
STUB_RENAME = re.compile(r'(?<![\w$])(todo|unimplemented)\s+as\b')
# An identifier or a literal after the name does not clear it: a matcher such as
# `($m:ident $x:ident) => { $m!() }` takes `call!(todo now)` apart. Nor does a
# `!` with no delimiter after it (`($m:ident !) => { $m!() }` and
# `call!(todo!)`); a `todo!(` that STUB also matches is deduplicated by offset.
# `\b` already keeps `todo_x` out.
STUB_IDENT = re.compile(r'(?<![\w$.:])(todo|unimplemented)\b(?!\s*[:.])')
RENAMED = " (renamed by use ... as)"
MACRO_TOKEN = " (passed to a macro as an identifier)"
# The two exempt attribute forms: cfg(test) and cfg(all(test, ...)).
CFG_TEST_ATTR = re.compile(
    r'#\s*\[\s*cfg\s*\(\s*(?:test|all\s*\(\s*test\s*,[^\]]*\))\s*\)\s*\]'
)
INNER_CFG_TEST = re.compile(r'#\s*!\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]')
INNER_ATTR_OPEN = re.compile(r'#\s*!\s*\[')
ATTR_START = re.compile(r'\s*#\s*\[')
# A module declaration header: optional visibility, `mod name ;`.
# An out-of-line module declaration `mod name;`, the visibility that may come
# before it, and a `#[path = "..."]` attribute (read from the raw source, since
# literals are blanked).
MOD_DECL = re.compile(r'(?<![\w$])mod\s+(?:r#)?([A-Za-z_]\w*)\s*;')
INLINE_MOD = re.compile(r'(?<![\w$])mod\s+(?:r#)?([A-Za-z_]\w*)\s*\{')
VIS_BEFORE = re.compile(r'(?<![\w$])pub\s*(?:\([^()]*\)\s*)?$')
PATH_ATTR = re.compile(r'#\s*\[\s*path\s*=\s*"([^"\\]*)"\s*\]')
# A real item head, read after any run of attributes: optional visibility
# (`pub`, `pub(crate)`, `pub(super)`, `pub(in path)`), optional qualifiers
# (`async`, `unsafe`, `const`, `default`, `extern "abi"`; the ABI string is
# already blanked), then an item keyword, `macro_rules!`, or a macro invocation
# `path::ident!`. Anything else (a field, a variant, a match arm, a parameter,
# a struct-literal field, an expression) is not an item, so exempts nothing.
# `union` is only a contextual keyword: it heads an item only when a name and
# then `<`, `{` or `where` follow (`union U {`), never as a field name
# (`union: x`) or a path segment (`union::A`).
ITEM_HEAD = re.compile(
    r'\s*(?:pub(?:\s*\([^)]*\)\s*|\s+))?'
    r'(?:(?:async|unsafe|const|default|extern)\s+)*'
    r'(?P<kind>(?:fn|mod|impl|struct|enum|trait|type|const|static|use|extern|let)\b'
    r'|union(?=\s+(?:r#)?[A-Za-z_]\w*\s*(?:<|\{|where\b))'
    r'|macro_rules\s*!|[A-Za-z_]\w*(?:\s*::\s*[A-Za-z_]\w*)*\s*!)'
)
# Items that end at their own top-level `;`, with any braces nested inside.
STATEMENT_KINDS = {"let", "const", "static", "use", "type"}
# A block expression statement: `{`, `unsafe {`, `async {`, `async move {` or
# an inline `const {`. It is read before ITEM_HEAD so `const {` is not taken for
# a `const` item.
BLOCK_HEAD = re.compile(r'\s*(?:(?:unsafe|const|async(?:\s+move)?)\s*)?\{')
# A closure may open its `|params|` list after one of these keywords.
CLOSURE_KEYWORDS = {"move", "async", "return", "break"}
WHERE_KW = re.compile(r'where\b')
# A macro invocation `name!(`, `name![`, `name!{` (also the last segment of a
# path such as `std::thread_local!`), and a `macro_rules! name {` definition.
# The keywords below can precede a prefix `!` (`if !(x)`, `return !(y)`), so
# they never name a macro.
MACRO_CALL = re.compile(r'(?<![\w$])([A-Za-z_]\w*)\s*!\s*[\(\[{]')
MACRO_RULES = re.compile(r'(?<![\w$])macro_rules\s*!\s*(?:r#)?[A-Za-z_]\w*\s*[\(\[{]')
NOT_MACRO = {
    "as", "break", "else", "if", "in", "let", "match", "move", "mut",
    "return", "while", "yield",
}
RAW_OPEN = re.compile(r'(?:br|cr|r)(#*)"')
CHAR_LIT = re.compile(r"'(?:\\.[^']*|[^\\'])'")


def rel(path):
    return os.path.relpath(path, root).replace(os.sep, "/")


def load_manifest(path):
    try:
        with open(path, "rb") as fh:
            return tomllib.load(fh)
    except (OSError, tomllib.TOMLDecodeError):
        return None


def blank(src):
    """Replace comments and string/char literals with spaces, keeping newlines.

    Handles nested block comments, raw strings (`r#"..."#`, `br"..."`, raw C
    strings `cr"..."`), byte and C strings, and tells a char literal from a
    lifetime. A leading UTF-8
    byte-order mark, which rustc skips, is blanked too, so the start of the
    file stays an item position.
    """
    out, n, i = list(src), len(src), 0

    def wipe(a, b):
        for k in range(a, min(b, n)):
            if out[k] != "\n":
                out[k] = " "

    if src.startswith("﻿"):
        wipe(0, 1)
        i = 1

    while i < n:
        c = src[i]
        if src.startswith("//", i):
            j = src.find("\n", i)
            j = n if j < 0 else j
            wipe(i, j)
            i = j
        elif src.startswith("/*", i):
            depth, j = 1, i + 2
            while j < n and depth:
                if src.startswith("/*", j):
                    depth += 1
                    j += 2
                elif src.startswith("*/", j):
                    depth -= 1
                    j += 2
                else:
                    j += 1
            wipe(i, j)
            i = j
        elif (
            c in "rbc"
            and (i == 0 or not (src[i - 1].isalnum() or src[i - 1] == "_"))
            and RAW_OPEN.match(src, i)
        ):
            m = RAW_OPEN.match(src, i)
            end = src.find('"' + m.group(1), m.end())
            j = n if end < 0 else end + 1 + len(m.group(1))
            wipe(i, j)
            i = j
        elif c == '"' or (c == "b" and src.startswith('b"', i)):
            j = i + (2 if c == "b" else 1)
            while j < n and src[j] != '"':
                j += 2 if src[j] == "\\" else 1
            wipe(i, j + 1)
            i = j + 1
        elif c == "'":
            m = CHAR_LIT.match(src, i)
            if m:
                wipe(i, m.end())
                i = m.end()
            else:
                i += 1  # a lifetime
        else:
            i += 1
    return "".join(out)


def skip_attrs(b, j):
    """Index just past any run of `#[...]` attributes starting at j."""
    while True:
        m = ATTR_START.match(b, j)
        if not m:
            return j
        depth, k = 0, m.end() - 1
        while k < len(b):
            depth += (b[k] == "[") - (b[k] == "]")
            if depth == 0:
                break
            k += 1
        j = k + 1


def match_group(b, k):
    """Index of the closer matching the opener `(`, `[` or `{` at k, or None."""
    depth = 0
    for i in range(k, len(b)):
        if b[i] in "([{":
            depth += 1
        elif b[i] in ")]}":
            depth -= 1
            if depth == 0:
                return i
    return None


def closure_end(b, k):
    """Index of the `|` closing a closure parameter list opened at k, or None.

    The `|` at k opens a closure only in expression position: after an
    operator or opening punctuation, or after `move`/`async`/`return`/`break`.
    After an operand (an identifier, `)`, `]`, `}` or `?`) it is a bitwise or
    logical `|`, and None is returned. The list ends at the next `|` at bracket
    depth 0; a `;` or an unmatched closer first means it was not a list (None).
    """
    p = k - 1
    while p >= 0 and b[p].isspace():
        p -= 1
    if p >= 0 and (b[p].isalnum() or b[p] == "_"):
        q = p
        while q >= 0 and (b[q].isalnum() or b[q] == "_"):
            q -= 1
        if b[q + 1 : p + 1] not in CLOSURE_KEYWORDS:
            return None
    elif p >= 0 and b[p] in ")]}?":
        return None
    depth = 0
    for i in range(k + 1, len(b)):
        c = b[i]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            if depth == 0:
                return None
            depth -= 1
        elif c == ";" and depth == 0:
            return None
        elif c == "|" and depth == 0:
            return i
    return None


def attr_open(b, p):
    """Index of the `#` opening the attribute whose closing `]` is at p, or None.

    Matches `#[...]` and `#![...]` (whitespace allowed between the tokens) by
    counting brackets backwards; comments and literals are already blanked.
    None means the `]` closes something other than an attribute.
    """
    depth = 0
    while p >= 0:
        depth += (b[p] == "]") - (b[p] == "[")
        if depth == 0:
            break
        p -= 1
    if p < 0:
        return None
    p -= 1
    while p >= 0 and b[p].isspace():
        p -= 1
    if p >= 0 and b[p] == "!":
        p -= 1
        while p >= 0 and b[p].isspace():
            p -= 1
    return p if p >= 0 and b[p] == "#" else None


def attrs_before(b, i):
    """(attrs, p) for the run of attributes that ends just before index i.

    attrs: (start, end) of each `#[...]` or `#![...]` in the run, nearest
    first. p: the index of the previous significant character before the run
    (-1 at the start of the file), or None when a `]` there closes something
    other than an attribute. Comments are already blanked.
    """
    attrs, p = [], i - 1
    while True:
        while p >= 0 and b[p].isspace():
            p -= 1
        if p < 0 or b[p] != "]":
            return attrs, p
        start = attr_open(b, p)
        if start is None:
            return attrs, None
        attrs.append((start, p + 1))
        p = start - 1


def at_item_position(b, i):
    """True when the attribute (or item) starting at i sits where an item can start.

    Looks back from i over whitespace (comments are already blanked) and any
    run of preceding attributes (`#[...]`, `#![...]`). An item position is the
    start of the file or just after `;`, `{` or `}`. After anything else, and
    in particular after `<`, `(`, `,`, `|` or `=` (inside a generic parameter
    list, a parameter list, a field or arm list, closure parameters or an
    initialiser), the attribute cannot attach to an item, so it exempts nothing.
    A `]` that closes something other than an attribute is not a position.
    """
    _attrs, p = attrs_before(b, i)
    return p is not None and (p < 0 or b[p] in ";{}")


def item_end(b, j):
    """End index of the item an exempt attribute ending at j attaches to, or None.

    The caller has already checked that the attribute is at an item position
    (`at_item_position`); one inside a generic parameter list, a parameter
    list, a field list, closure parameters or an initialiser never reaches
    here. None means the attribute (after any further attributes) is not
    followed by a real item head (ITEM_HEAD) or a block statement
    (BLOCK_HEAD): it sits on a struct field, an enum variant or a match arm
    that passed the position rule (the first of its list after `{`, or an arm
    after a `}`), or on an expression, and exempts nothing.

    - A block statement (`{`, `unsafe {`, `async {`, `const {`) ends at the `}`
      matching its `{`.
    - A macro invocation (`thread_local! { ... }`, `m!(...);`) ends at the
      closer of its own delimiter group, or at the `;` right after it. Anything
      else after the `!` (a match-arm pattern `m!() =>`, an operand `m!() < x`)
      only ever exempts the group itself.
    - A statement-shaped item (`let`, `const`, `static`, `use`, `type`) ends at
      its top-level `;`, with braces nested inside it. One that reaches an
      unmatched closer or the end of the file before any `;` is not a
      statement and exempts nothing.
    - Any other item ends at its first top-level `;`, or at the `}` matching
      its first top-level `{`.

    The header scan skips a closure parameter list `|a, b|` whole, stops at a
    top-level `=>` (a match arm: exempts nothing), at a top-level `,` (outside
    a `where` clause, whose predicates are comma-separated) and at an
    unmatched `)`, `]` or `}`, so it can never run past the block that
    encloses the attribute.
    """
    k = skip_attrs(b, j)
    block = BLOCK_HEAD.match(b, k)
    if block:
        return match_group(b, block.end() - 1)
    head = ITEM_HEAD.match(b, k)
    if not head:
        return None
    kind = head.group("kind")
    k, n = head.end(), len(b)
    if kind.endswith("!") and not kind.startswith("macro_rules"):
        while k < n and b[k].isspace():
            k += 1
        if k >= n or b[k] not in "([{":
            return None
        end = match_group(b, k)
        if end is None:
            return None
        k = end + 1
        while k < n and b[k].isspace():
            k += 1
        return k if k < n and b[k] == ";" else end
    statement = kind in STATEMENT_KINDS
    paren = angle = brace = 0
    in_where = False  # a `where` clause separates its predicates with top-level `,`
    while k < n:
        c = b[k]
        if (
            c == "w"
            and paren == 0
            and brace == 0
            and WHERE_KW.match(b, k)
            # `r#where` is a raw identifier, not the keyword
            and not (b[k - 1].isalnum() or b[k - 1] in "_#")
        ):
            in_where = True
        elif c == "|":
            close = closure_end(b, k)
            if close is not None:
                k = close
            elif b.startswith("||", k):
                k += 1  # a logical `||`: never read its second `|` as an opener
        elif c in "([":
            paren += 1
        elif c in ")]":
            if paren == 0:
                # an unmatched closer: the enclosing list ends here
                return None if statement else k - 1
            paren -= 1
        elif c == "<":
            angle += 1
        elif c == ">" and b[k - 1] not in "-=":
            angle = max(0, angle - 1)
        elif c == "=" and b.startswith("=>", k) and paren == 0 and brace == 0:
            return None  # a match arm, not an item
        elif c == "{":
            if paren == 0 and brace == 0 and not statement:
                break  # the item's own body
            brace += 1
        elif c == "}":
            if brace == 0:
                # an unmatched `}`: the enclosing block ends here
                return None if statement else k - 1
            brace -= 1
        elif c == ";" and paren == 0 and brace == 0:
            return k
        elif c == "," and paren == 0 and angle == 0 and brace == 0 and not in_where:
            return k - 1  # a top-level `,`: the enclosing list continues
        k += 1
    if k >= n:
        return None if statement else k
    depth = 0
    while k < n:
        depth += (b[k] == "{") - (b[k] == "}")
        if depth == 0:
            break
        k += 1
    return k


def leading_inner_cfg_test(b):
    """True when `#![cfg(test)]` is in the file's leading run of inner attributes.

    Only whitespace, comments (already blanked) and other `#![...]` attributes
    may come before it. The same tokens anywhere else (inside a macro
    invocation's tokens, after the first item) are not the file's own
    attribute, so they never make the file test-only.
    """
    k, n = 0, len(b)
    while True:
        while k < n and b[k].isspace():
            k += 1
        if INNER_CFG_TEST.match(b, k):
            return True
        m = INNER_ATTR_OPEN.match(b, k)
        if not m:
            return False
        end = match_group(b, m.end() - 1)
        if end is None:
            return False
        k = end + 1


def macro_groups(b):
    """(open, close) of every macro invocation group and macro_rules! body.

    The tokens inside are macro input, not items: rustc expands them only as
    the macro decides, so a `#[cfg(test)]` there is never at an item position.
    """
    groups = []
    for m in MACRO_CALL.finditer(b):
        if m.group(1) in NOT_MACRO or m.group(1) == "macro_rules":
            continue
        close = match_group(b, m.end() - 1)
        if close is not None:
            groups.append((m.end() - 1, close))
    for m in MACRO_RULES.finditer(b):
        close = match_group(b, m.end() - 1)
        if close is not None:
            groups.append((m.end() - 1, close))
    return groups


def in_groups(groups, i):
    return any(a < i < z for a, z in groups)


def mod_decls(raw, b, groups):
    """Every out-of-line `mod name;` at an item position outside macro input.

    Each is a dict: `name`; `test`, true when its attribute run holds
    `#[cfg(test)]` or `#[cfg(all(test, ...))]`; `path`, the literal of a
    `#[path = "..."]` in that run (read from the raw source), else None;
    `nesting`, the names of the inline `mod name { ... }` blocks around it,
    outermost first; `at`, the index of its `mod` keyword. analyse() adds
    `exempt`: the decl is `test` or sits inside an exempt span.
    """
    inline = []
    for m in INLINE_MOD.finditer(b):
        if in_groups(groups, m.start()):
            continue
        close = match_group(b, m.end() - 1)
        if close is not None:
            inline.append((m.end() - 1, close, m.group(1)))
    decls = []
    for m in MOD_DECL.finditer(b):
        if in_groups(groups, m.start()):
            continue  # macro input, not a declaration
        head = m.start()
        vis = VIS_BEFORE.search(b, max(0, head - 120), head)
        if vis:
            head = vis.start()
        attrs, p = attrs_before(b, head)
        if p is None or not (p < 0 or b[p] in ";{}"):
            continue
        path = None
        for a, z in attrs:
            pm = PATH_ATTR.match(raw, a)
            if pm and pm.end() <= z:
                path = pm.group(1)
        decls.append({
            "name": m.group(1),
            "test": any(CFG_TEST_ATTR.match(b, a) for a, _z in attrs),
            "path": path,
            "nesting": [name for a, z, name in inline if a < m.start() < z],
            "at": m.start(),
        })
    return decls


def analyse(raw, blanked):
    """Return (hits, decls); hits is None when the whole file is test-only.

    hits: (line, macro, how) of each stub macro outside an exempt item; how is
    "" for a direct `todo!(`, else RENAMED or MACRO_TOKEN.
    decls: the file's out-of-line module declarations (see mod_decls).
    """
    groups = macro_groups(blanked)
    decls = mod_decls(raw, blanked, groups)
    if leading_inner_cfg_test(blanked):
        for d in decls:
            d["exempt"] = True
        return None, decls  # the file's own #![cfg(test)]: the whole file is test-only
    spans = []
    for m in CFG_TEST_ATTR.finditer(blanked):
        if in_groups(groups, m.start()):
            continue  # macro input, not an item
        if not at_item_position(blanked, m.start()):
            continue  # in a generic, parameter, field or arm list: not on an item
        end = item_end(blanked, m.end())
        if end is not None:
            spans.append((m.start(), end))
    for d in decls:
        d["exempt"] = d["test"] or any(a <= d["at"] <= z for a, z in spans)
    found = [(m.start(), m.group(1), "") for m in STUB.finditer(blanked)]
    for u in USE_DECL.finditer(blanked):
        found += [
            (m.start(), m.group(1), RENAMED)
            for m in STUB_RENAME.finditer(blanked, u.start(), u.end())
        ]
    found += [
        (m.start(), m.group(1), MACRO_TOKEN)
        for m in STUB_IDENT.finditer(blanked)
        if in_groups(groups, m.start())
    ]
    # One finding per token: a `todo` that two rules match (a rename inside
    # macro tokens) keeps the first label, direct, then renamed, then token.
    first = {}
    for at, macro, how in found:
        first.setdefault(at, (macro, how))
    hits = [
        (blanked.count("\n", 0, at) + 1, macro, how)
        for at, (macro, how) in sorted(first.items())
        if not any(a <= at <= z for a, z in spans)
    ]
    return hits, decls


# Files rustc loads through a `#[path]` attribute. rustc treats every such file
# like a mod.rs: its own `mod child;` resolves beside it.
path_loaded = set()


def module_dir(path):
    base = os.path.basename(path)
    if base in ("lib.rs", "main.rs", "mod.rs") or path in path_loaded:
        return os.path.dirname(path)
    return path[: -len(".rs")]


def decl_files(path, decl):
    """The files rustc may load for the out-of-line module decl in path.

    The directory is module_dir(path) plus the inline module nesting around
    the decl (`mod outer { mod helper; }` in lib.rs is outer/helper.rs). A
    `#[path = "..."]` names exactly one file, relative to that directory when
    the decl is nested in an inline module and to the declaring file's own
    directory otherwise; the module name is then never used. Otherwise it is
    `name.rs` or `name/mod.rs` in that directory.
    """
    d = os.path.join(module_dir(path), *decl["nesting"])
    if decl["path"] is not None:
        base = d if decl["nesting"] else os.path.dirname(path)
        return [os.path.normpath(os.path.join(base, decl["path"]))]
    return [os.path.join(d, decl["name"] + ".rs"), os.path.join(d, decl["name"], "mod.rs")]


# --- The published packages. ---
root_manifest = load_manifest(os.path.join(root, "Cargo.toml"))
ws_publish = None
if root_manifest is not None:
    ws_publish = root_manifest.get("workspace", {}).get("package", {}).get("publish")

manifests = [os.path.join(root, "Cargo.toml")]
manifests += sorted(glob.glob(os.path.join(root, "crates", "*", "Cargo.toml")))
packages = []
for manifest_path in manifests:
    manifest = (
        root_manifest if manifest_path == os.path.join(root, "Cargo.toml")
        else load_manifest(manifest_path)
    )
    if manifest is None:
        continue
    package = manifest.get("package")
    if not isinstance(package, dict):
        continue
    publish = package.get("publish", True)
    if isinstance(publish, dict) and publish.get("workspace") is True:
        publish = True if ws_publish is None else ws_publish
    if publish is False:
        continue
    packages.append(os.path.dirname(manifest_path))

# --- The scan set: sorted, deterministic. ---
files = sorted({
    os.path.join(dirpath, filename)
    for pkg in packages
    for dirpath, _dirnames, filenames in os.walk(os.path.join(pkg, "src"))
    for filename in filenames
    if filename.endswith(".rs")
})

parsed = {}
unreadable = []


def load(path):
    try:
        with open(path, "r", encoding="utf-8") as fh:
            raw = fh.read()
    except (UnicodeDecodeError, OSError):
        unreadable.append(path)
        return
    parsed[path] = analyse(raw, blank(raw))


def grow_path_loaded():
    """Add every `#[path]` target to path_loaded, to a fixed point.

    A nested `#[path]` decl resolves through module_dir(), which itself
    depends on path_loaded; the set only ever grows, so this terminates.
    """
    grown = True
    while grown:
        grown = False
        for path, (_hits, decls) in parsed.items():
            for decl in decls:
                if decl["path"] is not None:
                    for target in decl_files(path, decl):
                        if target not in path_loaded:
                            path_loaded.add(target)
                            grown = True


for path in files:
    load(path)
grow_path_loaded()

# A production module (not `#[cfg(test)]`, not inside an exempt span, not in a
# test-only file) that loads a file outside the scan set, through `#[path]` or
# as a child of such a file, compiles that file into the published crate: scan
# it too (eval.rs pulls in tests/helpers/e2e_fixtures.rs this way).
scan = set(files)
pending = list(files)
while pending:
    path = pending.pop()
    if path not in parsed or parsed[path][0] is None:
        continue
    for decl in parsed[path][1]:
        if decl["exempt"]:
            continue
        for target in decl_files(path, decl):
            if target not in scan and os.path.isfile(target):
                scan.add(target)
                if decl["path"] is not None:
                    path_loaded.add(target)
                load(target)
                pending.append(target)
grow_path_loaded()
files = sorted(scan)
test_only = {path for path, (hits, _decls) in parsed.items() if hits is None}


def resolve_children(path, decls):
    """Mark every scanned file that a decl in `decls` (declared in `path`) loads as test-only."""
    added = []
    for decl in decls:
        for cand in decl_files(path, decl):
            if cand in parsed and cand not in test_only:
                test_only.add(cand)
                added.append(cand)
    return added


# `#[cfg(test)] mod x;` in any file makes x test-only; then, transitively, every
# `mod y;` declared inside a test-only file.
queue = []
for path, (_hits, decls) in parsed.items():
    queue += resolve_children(path, [d for d in decls if d["test"]])
queue += [p for p in test_only]
seen = set()
while queue:
    path = queue.pop()
    if path in seen:
        continue
    seen.add(path)
    queue += resolve_children(path, parsed[path][1])

findings = sorted(
    (rel(path), line, macro, how)
    for path in files
    if path in parsed and path not in test_only
    for line, macro, how in parsed[path][0]
)

# --- Named failures, in status-line priority order. ---
named = []  # (token, detail line)
if not packages:
    named.append(("NO_PACKAGES",
                  f"NO_PACKAGES: no publishable package (a [package] table whose "
                  f"publish is not false) under {root}; a broken tree is a named "
                  "failure, never a pass."))
elif not files:
    named.append(("ZERO_FILES",
                  f"ZERO_FILES: scanned 0 .rs file(s) under src/ of "
                  f"{len(packages)} publishable package(s); a broken scan set is a "
                  "named failure, never a pass."))
for path in unreadable:
    named.append(("UNREADABLE",
                  f"UNREADABLE: {rel(path)} is not valid UTF-8 or cannot be read; "
                  "it is never skipped."))
named.sort(key=lambda item: ("NO_PACKAGES", "ZERO_FILES", "UNREADABLE").index(item[0]))

summary = (f"scanned {len(files)} file(s) ({len(test_only)} test-only), "
           f"{len(findings)} finding(s)")

if named or findings:
    print(named[0][0] if named else "FOUND")
    for _, detail in named:
        print(detail)
    for path, line, macro, how in findings:
        print(f"{path}:{line}: {macro}!{how} outside #[cfg(test)] in a published crate")
    if files:
        print(summary)
    sys.exit(0)

print("OK")
print(summary)
sys.exit(0)
PY
)

STATUS_LINE=$(head -n1 <<<"${REPORT}")
DETAIL=$(tail -n +2 <<<"${REPORT}")

if [ "${STATUS_LINE}" = "OK" ]; then
    echo "✅ OK: ${DETAIL}"
    exit 0
else
    echo "❌ Stub-panic check failed (${STATUS_LINE})"
    echo ""
    echo "${DETAIL}"
    echo ""
    echo "If this failure is unexpected:"
    echo "  1. Replace each reported stub with a typed Err (or delete the dead code);"
    echo "     a stub macro outside #[cfg(test)] in a published crate panics a caller."
    echo "  2. This gate has no exemption list, no inline marker and no environment"
    echo "     switch (D-01); do not add one to silence a finding."
    exit 1
fi
