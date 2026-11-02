---
phase: 55-correctness-fixes-test-hygiene-lint-bar
fixed_at: 2026-11-02T00:00:00Z
review_path: .planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-REVIEW.md
iteration: 5
fix_scope: critical_warning
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 55: Code Review Fix Report

**Fixed at:** 2026-11-02
**Source review:** .planning/phases/55-correctness-fixes-test-hygiene-lint-bar/55-REVIEW.md
**Iteration:** 5 (fourth re-fix round, WR-03 only, after the verifiers' and an adversary's reports at `a2afe7045`)

**Summary:**
- Findings in scope: 3 (WR-01, WR-02, WR-03; Info findings not touched)
- Fixed: 3
- Skipped: 0

Fix order was WR-03, WR-02, WR-01, with one commit per warning on branch
`claude/laughing-dirac-e0h2ax`. The work ran in the main working tree, as the task asked. Nothing was pushed.

**Re-fix round (iteration 2):** the verifiers rejected WR-02 and WR-03. Both rejections rest on problems in the WR-03 gate commit and in the WR-01 scan code, plus two minor doc gaps for WR-02. Each problem was addressed in a new commit; no accepted fix was reverted, and no history was rewritten:
- `70ff58443` fix(55): ... (WR-03, re-fix): the stub-panic gate
- `2bd816bc5` fix(55): ... (WR-02, re-fix): docs only
- `55a2bd8e6` fix(55): ... (WR-01, re-fix): the assistant-cursor guard. Both verifiers raised it, but it is WR-01's scan code, so the commit is labelled WR-01.

Each finding below has a "Re-fix (iteration 2)" subsection.

**Re-fix round (iteration 3):** two verifiers rejected WR-03 at HEAD `55a2bd8e6`. A `#[cfg(test)]` on a `const` generic parameter still exempted every production item up to the next `;` later in the file. This was a regression from `34196e5c4`: the original gate reported those stubs. It is fixed in one new commit, again with no history rewrite:
- `aad3d343e` fix(55): exempt cfg(test) only at item positions in the stub-panic gate (WR-03, round 3)

WR-01 and WR-02 are unchanged in this round. WR-03 has a "Re-fix (iteration 3)" subsection, and the earlier WR-03 claims that this defect made false are corrected in place.

**Re-fix round (iteration 4):** at HEAD `aad3d343e` the verifiers closed the const-generic defect, but reported one minor strict-side false positive (a leading byte-order mark), ten adversary false negatives and a scope gap in the WR-03 gate, and three WR-01 gaps (stop-line wording, a stale doc, an untested guard arm). Each was fixed in its own new commit, each with a test or fixture watched failing first. No history was rewritten and nothing was pushed:
- `637a184eb` byte-order mark (WR-03)
- `1f1421c6e` `r#where` (WR-03, FN-2)
- `3fa4449dc` contextual `union` (WR-03, FN-1 and FN-3)
- `9042c4130` `#![cfg(test)]` only in the leading inner attributes (WR-03, FN-4)
- `83aca0f1f` macro input is never an item position (WR-03, FN-5 and FN-10)
- `7e09c722c` `#[cfg(test)] #[path]` modules resolve to their literal path (WR-03, FN-6)
- `c500d1616` inline module nesting in module resolution (WR-03, FN-7)
- `e6cb6bf89` production `#[path]` targets outside `src/` are scanned (WR-03, scope gap)
- `26472e112` raw C strings (WR-03, FN-8)
- `ffef65e05` renamed and macro-built stub calls (WR-03, FN-9)
- `22c861bdd` the stop line names both causes (WR-01)
- `a2afe7045` a test for the empty-page arm of the assistant-cursor guard (WR-01)

WR-03 and WR-01 each have a "Re-fix (iteration 4)" subsection. WR-02 is unchanged.

**Re-fix round (iteration 5):** at HEAD `a2afe7045` the verifiers found the round-4 `ffef65e05` header false in three ways (macro-built calls still missed), one header inconsistency, one wrong shellcheck count in this report, and five adversary false negatives (FN-A..E) in test-only file marking. Each was fixed in a new commit, each with a fixture that failed first (one doc-only commit carries a pin fixture instead; see below). The work ran in an isolated git worktree on a temp branch, which was fast-forwarded onto `claude/laughing-dirac-e0h2ax`. Nothing was rewritten and nothing was pushed:
- `7bcbb6771` a stub name followed by an identifier or a literal in macro tokens (WR-03)
- `352583381` a stub name followed by a bare `!` in macro tokens (WR-03)
- `49dcdcb05` a path ending in a stub name in macro tokens (WR-03)
- `5a2cdbe5d` the header now names the strict reading of a `todo`-module rename (WR-03, doc only)
- `572bc9845` a file is test-only only when every loader is test-only (WR-03, FN-A, FN-B, FN-C)
- `a9e65afd5` `include!` counts as a module loader (WR-03, FN-D)
- `64639b071` `#[path]` on inline modules is honoured (WR-03, FN-E)

WR-03 has a "Re-fix (iteration 5)" subsection. WR-01 and WR-02 are unchanged.

**Process notes from the verifiers:**
- This report is still untracked. That is deliberate: the fixer writes it, and the orchestrating workflow commits it. It has not been committed in this round either.
- The span-comparison helper named in iteration 3, and the round-4 helpers below, are scratch tools. They are not committed, and they live in the session scratchpad: `/tmp/claude-0/-home-user-paladin-dev-env/c62b0292-6a28-5efd-8101-f37a9af13e46/scratchpad/compare_spans.py` (iteration 3), plus `.../scratchpad/r4/compare_r4.py`, `.../scratchpad/r4/testonly_set.py` and `.../scratchpad/r4/verify.sh` (iteration 4). The adversary's rustc oracle is `.../scratchpad/gate-adversary-harness/run.py`. The committed regression evidence is the harness, `tests/scripts/check-no-stub-panics_test.sh`.

## Fixed Issues

### WR-03: In the stub-panic gate, `#[cfg(test)]` on a non-item exempts the next braced item

**Status:** fixed: requires human verification (logic change in a CI gate)
**Commit:** 34196e5c4
**Files modified:** `scripts/check-no-stub-panics.sh`, `tests/scripts/check-no-stub-panics_test.sh`
**Applied fix:**
- New `ITEM_HEAD` regex. After `skip_attrs`, an exemption applies only when a real item head follows:
  - optional visibility: `pub`, `pub(crate)`, `pub(super)` or `pub(in path)`
  - optional qualifiers: `async`, `unsafe`, `const`, `default` or `extern "abi"`
  - then one of `fn`, `mod`, `impl`, `struct`, `enum`, `union`, `trait`, `type`, `const`, `static`, `use`, `extern`, `let`, `macro_rules!`, or a macro invocation `path::ident!`. This keeps the `thread_local!` case working.
- `item_end()` now returns `None` for any other position (field, variant, match arm, parameter, struct-literal field, expression). The span assembly skips `None`, so nothing is exempted there.
- The header scan tracks `()`/`[]`/`<>`/`{}` depth. It stops at:
  - a top-level `,`, except inside a `where` clause, whose predicates are comma-separated
  - an unmatched `)`, `]` or `}`

  This means a scan can never run past the enclosing block. *(Overclaim, corrected in iteration 3: this did not hold for a `#[cfg(test)]` inside a generic parameter list. `const N: usize` there matched `ITEM_HEAD` as a `const` item, and a `>` is not a closer, so the scan ran on to the next top-level `;` in the file. The same is true of the claim above that `item_end()` returns `None` on a parameter: it did not on a `const` generic parameter. See "Re-fix (iteration 3)".)*
- `let`, `const`, `static`, `use` and `type` end at their top-level `;`, with any braces inside nested. Every other item ends at its first top-level `;` or at the `}` that matches its first top-level `{`. This is the same rule as before.
- The out-of-line `#[cfg(test)] mod x;` resolution (`MOD_ITEM`) is unchanged. The header comment now describes the narrowed exemption.
- Harness, FIRE fixtures (all five failed against the old gate before the fix):
  - struct field followed by an impl with `todo!()`
  - enum variant followed by a fn with `todo!()`
  - match arm next to an arm with `todo!()`
  - function parameter in a fn whose body has `todo!()`
  - struct-literal field next to a field set to `unimplemented!()`
  - one extra FIRE case: an exempt fn's span ends at its own brace
- Harness, PASS fixture `item-heads`: visibility, qualifiers, stacked attributes, `unsafe impl` with generic commas, `thread_local!`, `std::thread_local!`, `macro_rules!`, `const`, a `let` with braces in its initialiser, and a multi-line `where` clause with a trailing comma.

**Verification:**
- `./tests/scripts/check-no-stub-panics_test.sh`: 41 assertions passed (34 before). Before the fix, 6 of 41 failed: the 5 new FIRE cases, plus the `let` case in the PASS fixture.
- `./scripts/check-no-stub-panics.sh` on the real tree: `OK: scanned 623 file(s) (19 test-only), 0 finding(s)`. The counts are the same as the baseline.
- Real-tree span comparison (old `item_end` against new, over all 572 exempt attributes in the repo): 0 classified as non-item, 0 span differences. A first version of the fix cut `otel_sink.rs:162` short at its `where`-clause comma. This was caught here and fixed before the commit.
- `make test-shell-guards`: every suite passed, exit 0.
- `shellcheck --severity=warning` on both scripts (the `make lint-shell` level): clean. Plain `shellcheck` reports 8 info/style notes (SC2317, SC2001) in the harness. The same 8 notes are present at HEAD before this change.

**Correction to iteration 1:** the paragraphs above describe the narrowing only as closing false negatives. It also added new strict-side false positives, which the verifiers found:
- a `#[cfg(test)]` `let`/`const` whose closure takes two parameters was reported
- a block statement `#[cfg(test)] { todo!() }` was reported
- an expression statement `#[cfg(test)] f(|| todo!());` was reported

The pre-fix gate exempted all three. Iteration 2 removes the first two. The third is kept on purpose and is now documented.

**Second correction (found by the iteration-3 verifiers):** iteration 1 also added a strict-side false negative. A `#[cfg(test)]` on a `const` generic parameter, followed later in the file by a top-level `;`, exempted every production item in between. The pre-fix gate reported those stubs. Iteration 2 did not close this. Iteration 3 does.

#### Re-fix (iteration 2)

**Status:** fixed: requires human verification (logic change in a CI gate)
**Commit:** 70ff58443
**Files modified:** `scripts/check-no-stub-panics.sh`, `tests/scripts/check-no-stub-panics_test.sh`

**Problems addressed:**
1. **Closure commas cut a statement short (regression).** The header scan now reads a closure parameter list `|a, b|` whole, through the new `closure_end()`. A `|` opens a closure only in expression position: after an operator or opening punctuation, or after `move`, `async`, `return` or `break`. After an operand it is a bitwise or logical `|`. The list ends at the next `|` at bracket depth 0; a `;` or an unmatched closer before that means it was not a list. The top-level `,` stop is kept for statement kinds, so a `const` generic parameter followed by `, T>` still ends at its comma.
2. **Macro-pattern match arm (residual false negative).** A macro-invocation head (`m!`, `thread_local!`, `std::thread_local!`) now exempts exactly its own delimiter group, plus a `;` right after it. It no longer runs the angle-counting header scan. So `m!() => x < 3, _ => todo!()` and `m!() if x < 3 => 0, _ => todo!()` exempt only `()`, and the neighbouring stub is reported. As a second line of defence, a top-level `=>` in the header scan of any other head now returns None (a match arm, exempt nothing).
3. **Block statements (regression).** The new `BLOCK_HEAD` exempts exactly the block of `{ ... }`, `unsafe { ... }`, `async { ... }`, `async move { ... }` and inline `const { ... }`. It is checked before `ITEM_HEAD`, so `const {` is no longer read as a `const` item.
4. **Inline `const { ... }` ran to the end of the fn (false negative the fix introduced).** Closed by item 3. Also, a statement-shaped item (`let`, `const`, `static`, `use`, `type`) that reaches an unmatched closer, or the end of the file, before any `;` now exempts nothing. The example is a `const` generic parameter, `fn g<#[cfg(test)] const N: usize>() { todo!() }`. Before, the span ran to the enclosing `}`, or to EOF at the top level. *(Overclaim, corrected in iteration 3: this closed the `const` generic case only when no top-level `;` followed anywhere later in the file. With a later `;` (a `use`, a `const`, a unit struct), the scan still ran on to it and exempted every production item in between. The single-line fixture had no later `;`, so it passed by accident.)*
5. **Expression statements (behaviour change, now documented).** `#[cfg(test)] self_check(|| todo!());` is still reported, because the gate cannot tell a statement position from a struct-literal or match-arm position, which both follow `{` or `,`. This is the strict side. The script header now says so and names the remedy, `#[cfg(test)] { ... }`, which is exempt. A FIRE fixture pins the behaviour.
6. **Docs.** The header comment and the `item_end` docstring now describe every case: block statement, macro group, statement `;`, the no-`;` case exempting nothing, closures, `=>`, and the strict expression-statement case. *(Overclaim, corrected in iteration 3: the header also said a generic parameter exempts nothing, which was false for a `const` generic parameter with a later `;`.)*

**New harness cases:**
- FIRE: macro-pattern arm with `<` in the body; macro-pattern arm with `<` in the guard; inline `const { () }` followed by `todo!()`; `const` generic parameter; expression statement (strict by design)
- PASS: `let` with closure `|a: u8, b: u8|`; `const G: fn(u8, u8) -> u8 = |a, b| todo!();`; `let` with `move |a: (u8, u8), b: Vec<u8>|`; block, `unsafe` and `const` block statements

Before the change, 8 of the 9 new cases failed. The expression-statement FIRE case already fired.

**Verification:**
- `./tests/scripts/check-no-stub-panics_test.sh`: 50 assertions passed (41 before).
- `./scripts/check-no-stub-panics.sh` on the real tree: `OK: scanned 623 file(s) (19 test-only), 0 finding(s)`. The counts are unchanged.
- Span comparison over all 572 `#[cfg(test)]`/`#[cfg(all(test, ..))]` attributes in the repo's tracked `.rs` files: 0 non-item classifications. There are 0 span differences against both the iteration-1 gate (`34196e5c4`) and the original pre-fix gate (`34196e5c4^`).
- Extra probes, all as expected:
  - reported: array element `[#[cfg(test)] m!() < x, todo!()]`, `let v = x | y; todo!();`, `let v = || 1; todo!();`, `let v = a || b; todo!();`, call argument `g(#[cfg(test)] |a, b| 1, todo!())`, and a `const` generic parameter inside an impl *(overclaim, corrected in iteration 3: this holds only when no top-level `;` follows later in the file. With one, `impl<#[cfg(test)] const N: usize> S {}` exempted every stub up to that `;`, at both `34196e5c4` and `70ff58443`.)*
  - exempt: `let g = async |a: u8, b: u8| todo!();`, `async move { todo!() };` and `let Some(x) = y else { todo!() };`
- `make test-shell-guards`: every suite passed, exit 0.
- `shellcheck --severity=warning` on both scripts: clean. Plain `shellcheck`: the same 8 pre-existing info/style notes in the harness, and none in the gate.

#### Re-fix (iteration 3)

**Status:** fixed: requires human verification (logic change in a CI gate)
**Commit:** aad3d343e
**Files modified:** `scripts/check-no-stub-panics.sh`, `tests/scripts/check-no-stub-panics_test.sh`

**Problem:** take a `#[cfg(test)]` on a `const` generic parameter, as in `pub fn g<#[cfg(test)] const N: usize>() {}` or `impl<#[cfg(test)] const N: usize> S {}`. `ITEM_HEAD` matched it as kind `const`, which is a statement kind. `item_end` then scanned past the generic list's closing `>` to the next top-level `;` anywhere later in the file. Every production item in between was exempted. The original gate (`34196e5c4^`) reported these stubs. `34196e5c4` and `70ff58443` did not, so this was a regression. The iteration-2 fixture was a single line with no later `;`, so it passed by accident.

**Applied fix: a position rule, not another special case.**
- New `at_item_position(b, i)`, with a helper `attr_open(b, p)`. It looks back from the attribute's `#` over:
  - whitespace (comments are already blanked)
  - any run of preceding `#[...]` or `#![...]` attributes, matched by counting brackets backwards
- The attribute is at an item position only when the previous significant character is the start of the file, `;`, `{` or `}`.
- After `<`, `(`, `,`, `|`, `=` or any other character, the attribute is inside one of these, so it exempts nothing and resolves no out-of-line `mod`:
  - a generic parameter list
  - a parameter list
  - a field, variant or arm list
  - a closure parameter list
  - an initialiser
- A `]` that does not close an attribute also counts as not an item position. This is the strict side.
- `analyse()` calls the check before `item_end()`.
- Every earlier rule is kept unchanged:
  - `ITEM_HEAD` and `BLOCK_HEAD`
  - statement kinds ending at their own `;`
  - block statements exempting their own block
  - macro invocations exempting their own delimiter group
  - closure-parameter handling
  - `=>` returning None
  - the where-clause comma rule
  - out-of-line `#[cfg(test)] mod x;` resolution
  - no `;` found exempts nothing
- No earlier special case was removed, because these still guard positions the rule lets through: the top-level `,` stop, `=>` and unmatched closers cover the first field, variant or arm after `{`, and a macro-pattern arm after a `}`-bodied arm.
- The script header now gives the position rule first, then the head rule. It no longer claims that a generic parameter exempts nothing because of the head scan. The `item_end` docstring says the caller has already applied the position rule, and drops the `const`-generic example from the no-`;` bullet.

**Harness:**
- Replaced the accidental-pass const-generic fixture with the four-line reproducer, written as the whole `lib.rs`. It must report `lib.rs:2` (`todo!`) and `lib.rs:3` (`unimplemented!`):
  - `pub fn g<#[cfg(test)] const N: usize>() {}`
  - `pub fn a() { todo!() }`
  - `pub fn b() { unimplemented!() }`
  - `use std::fmt as _f;`
- New FIRE fixtures. Each is followed by a production `fn` with `todo!()` and then a later `use std::fmt as _f;`:
  - type generic: `pub fn h<#[cfg(test)] T>() {}`
  - lifetime parameter: `pub fn l<#[cfg(test)] 'a>() {}`
  - const generic in an impl header: `pub struct S;` then `impl<#[cfg(test)] const N: usize> S {}`
- Watched them fail first. At `55a2bd8e6`, 2 of 53 assertions failed: the const-generic reproducer and the impl-header const generic.
- The type-generic and lifetime fixtures already passed at `55a2bd8e6`, because `ITEM_HEAD` never matches `T` or `'a`. They are regression pins for the position rule, not demonstrations of the defect.

**Verification:**
- `./tests/scripts/check-no-stub-panics_test.sh`: 53 assertions passed (50 before, plus 3 new ones; the replaced fixture now makes two needle checks in one assertion).
- `./scripts/check-no-stub-panics.sh` on the real tree: `OK: scanned 623 file(s) (19 test-only), 0 finding(s)`.
- Span comparison over every `#[cfg(test)]`/`#[cfg(all(test, ..))]` attribute in the repo's 877 tracked `.rs` files, 572 attributes in all:
  - Compared the exempted span and the `mod` resolution of each attribute under the original gate (`34196e5c4^`), the iteration-2 gate (`55a2bd8e6`) and the new gate.
  - Result: 0 differences against either, and 0 attributes classified as not at an item position.
  - The previous significant character before the 572 attributes was `}` 523 times, `;` 35 times, `{` once and start-of-file 13 times.
  - The helper was sanity-checked on the reproducers, where it does show the differences: the original ends the span inside the generic list, `55a2bd8e6` runs to the later `;`, and the new gate exempts nothing.
  - The helper is `/tmp/claude-0/-home-user-paladin-dev-env/c62b0292-6a28-5efd-8101-f37a9af13e46/scratchpad/compare_spans.py`. It is not committed.
- Reproducers checked at each gate version:
  - `fn` const generic: reported at `34196e5c4^` and at the new gate. Missed (`OK`) at `34196e5c4` and `70ff58443`.
  - `impl` const generic: the same pattern.
- `make test-shell-guards`: all 18 suites passed, exit 0.
- `shellcheck --severity=warning` on both scripts: clean. Plain `shellcheck`: the same 8 pre-existing info/style notes in the harness (SC2001, SC2317), and none in the gate.

#### Re-fix (iteration 4)

**Status:** fixed: requires human verification (logic changes in a CI gate)
**Commits:** 637a184eb, 1f1421c6e, 3fa4449dc, 9042c4130, 83aca0f1f, 7e09c722c, c500d1616, e6cb6bf89, 26472e112, ffef65e05
**Files modified:** `scripts/check-no-stub-panics.sh`, `tests/scripts/check-no-stub-panics_test.sh`

**The iteration-3 defect is closed.** The four-line const-generic reproducer reports `lib.rs:2` (`todo!`) and `lib.rs:3` (`unimplemented!`) at `aad3d343e` and at every later commit. The harness pins it with fixture `cfg-test-const-generic`.

**Problems addressed, one commit each:**
1. **Byte-order mark (`637a184eb`, strict-side false positive).** `at_item_position` treated only `p < 0` as the start of the file. A BOM is not whitespace, so a `#[cfg(test)] mod tests` right after a BOM was reported. `blank()` now blanks a leading BOM, which rustc skips. This also makes the header's "start of the file" claim true for that file shape. PASS fixture: `bom-test-mod`.
2. **`r#where` (`1f1421c6e`, FN-2).** A `where` after `#` is a raw identifier, not the keyword, so it no longer opens a where clause. FIRE fixture: `cfg-test-raw-where`, the adversary's x02 layout.
3. **Contextual `union` (`3fa4449dc`, FN-1, FN-3).** `union` heads an item only as `union Name` followed by `<`, `{` or `where`. A struct-literal field `union: a < b` and a path pattern `union::A | 'b' => ...` are no longer item heads, so the angle count and the fake closure skip that followed are never reached from them. No other contextual keyword is an item kind. FIRE fixtures: `cfg-test-union-field` and `cfg-test-union-path-arm`. The `item-heads` PASS fixture pins that `#[cfg(test)] union U { ... }` stays exempt.
4. **`#![cfg(test)]` (`9042c4130`, FN-4).** It now makes a file test-only only in the file's leading run of inner attributes: only whitespace, comments and other `#![...]` may come before it. `stringify!(#![cfg(test)])` after an item no longer exempts the file. FIRE fixture: `inner-cfg-test-in-macro-tokens`. PASS fixture: `inner-cfg-test-after-inner-attrs`.
5. **Macro input (`83aca0f1f`, FN-5, FN-10).** A `#[cfg(test)]` inside a macro invocation's delimiter group (`m!( )`, `m![ ]`, `m!{ }`, path macros too) or inside a `macro_rules!` body exempts nothing and records no test module. Keywords that can come before a prefix `!` (`if`, `return`, `while`, ...) are not macros. FIRE fixtures: `cfg-test-mod-in-macro-tokens`, `cfg-test-mod-in-macro-rules-body` and `cfg-test-in-macro-input`.
6. **`#[cfg(test)] #[path]` (`7e09c722c`, FN-6).** New `mod_decls()` records every out-of-line `mod name;` at an item position outside macro input, with its attribute run: the cfg(test) flag and any `#[path]` literal. The literal is read from the raw source, because literals are blanked. A `#[path]` decl resolves to that one file, never to `name.rs`, so the mock-swap pattern no longer exempts the production `imp.rs`. A `#[path]`-loaded file resolves its own children beside it, as rustc treats it like a `mod.rs`. FIRE fixtures: `cfg-test-path-mod` and `cfg-test-path-first-mod` (`#[path]` before the cfg). PASS fixture: `out-of-line-path-test-mod`.
7. **Inline nesting (`c500d1616`, FN-7).** Resolution goes through the inline `mod outer { ... }` blocks around a declaration. `mod outer { #[cfg(test)] mod helper; }` in lib.rs is `outer/helper.rs`, and in a non-mod-rs `a.rs` it is `a/outer/helper.rs`. The header's old "over-approximation" sentence is removed. FIRE fixture: `cfg-test-mod-in-inline-mod`. PASS fixture: `out-of-line-test-mod-in-inline-mod`.
8. **Scope gap (`e6cb6bf89`).** After the `src/` scan, a production module declaration pulls its target into the scan set, transitively, when the target lies outside that set. A production declaration here means one that is not cfg(test), not inside an exempt span and not in a test-only file. The real tree now scans 624 files, the extra one being `tests/helpers/e2e_fixtures.rs` from `eval.rs`. FIRE fixture: `path-mod-outside-src` (the target and its child are both reported). PASS fixture: `test-path-mod-outside-src` (a cfg(test) target, and one inside a cfg(test) mod, are not pulled in).
9. **Raw C strings (`26472e112`, FN-8).** `cr"..."` and `cr#"..."#` are raw-string prefixes. `c"..."` already lexed as a normal string. FIRE fixture: `raw-c-string`. PASS fixture: `c-strings`.
10. **Renamed and macro-built calls (`ffef65e05`, FN-9).** Two new findings, each named in the finding line:
    - a `use` that renames `todo` or `unimplemented` (`(renamed by use ... as)`)
    - the bare identifier inside a macro invocation's tokens or a `macro_rules!` body (`(passed to a macro as an identifier)`)

    A path segment, a field or a `$todo` metavariable is not the macro. This rule is strict by design, and the header says so: a local named `todo` handed to a macro, or `use crate::todo as x` for a module named `todo`, is reported too. A call built some other way (a proc macro, `concat_idents!`) is beyond a lexical gate. FIRE fixtures: `renamed-stub-import` and `stub-ident-macro-token`. PASS fixture: `stub-names-not-macros`. *(Overclaim, corrected in iteration 5: the header said every bare identifier inside macro tokens is a finding, but the lookahead skipped one followed by an identifier, a literal or a bare `!`, and the lookbehind skipped a path's last segment. rustc builds and panics on all four. Also, the header never named the `todo`-module rename strictness, and its "a path segment is not the macro" contradicted it. See "Re-fix (iteration 5)".)*

**Corrections to earlier iterations:**
- Iteration 3 said that only the start of the file, `;`, `{` or `}` is an item position. That stays true, but the start of the file was missed when a file began with a BOM (item 1).
- Iteration 1 called the by-file resolution an over-approximation that "can only exempt a file that exists at that exact path". It did exempt real production files, in the `#[path]` and inline-nesting layouts (items 6 and 7).
- The earlier rounds said the gate scans `src/` of published packages. That missed `#[path]` targets outside `src/` that rustc compiles into a published crate (item 8).
- One more correction applies to this round's own commits. Commits `637a184eb` and `7e09c722c` each left one header comment line over 100 characters. Both were reflowed, in `c500d1616` and `e6cb6bf89`, and both commit messages say so. The gate now has no line over 100 characters.

**Harness:**
- 74 assertions pass (53 before this round, plus 21 new fixtures).
- All 14 new FIRE fixtures were run against the gate before their fix, and each failed there.
- Three new PASS fixtures failed before their fix: `bom-test-mod`, `out-of-line-path-test-mod` and `out-of-line-test-mod-in-inline-mod`.
- The other four new PASS fixtures already passed before their fix and are regression pins: `inner-cfg-test-after-inner-attrs`, `test-path-mod-outside-src`, `c-strings` and `stub-names-not-macros`.

**Verification (after each commit, and again at the end):**
- `./tests/scripts/check-no-stub-panics_test.sh`: 74 assertions passed.
- `./scripts/check-no-stub-panics.sh` on the real tree: `OK: scanned 624 file(s) (19 test-only), 0 finding(s)`. The scan set is the iteration-3 set plus `tests/helpers/e2e_fixtures.rs`, and the 19 test-only files are the same 19.
- Whole-tree comparison against the `aad3d343e` gate, over all 877 tracked `.rs` files (not only the scan set): 0 per-file hit differences and 0 differences in the test-module or all-module name lists. Thirteen files carry new `#[path]` annotations (`eval.rs` plus 12 under `tests/`), all on non-test declarations. None of the 572 `#[cfg(test)]` attributes sits inside a macro group, and no out-of-line module is declared inside an inline module.
- rustc oracle (the adversary's `run.py`): it compiles each of 56 fixture crates with `rustc -Zunpretty=expanded`, with and without `--test`, and compares the gate with the stubs rustc keeps in the non-test build. At `aad3d343e`, 12 cases were false negatives. Now every case that compiles agrees with rustc, including x01 through x10, m03 and m04. `f12_where_clause` does not compile, so it has no oracle verdict.
- The adversary's scope-gap tree (`gate-adversary-path-outside-src`) now reports `tests/helpers/fx.rs:1`.
- `make test-shell-guards`: all 18 suites passed, exit 0.
- `make lint-shell` and `shellcheck --severity=warning` on both scripts: clean. Plain `shellcheck` on the gate: clean.
- Plain `shellcheck` on the harness reports 10 info/style notes: the 8 already present at `aad3d343e` (6 SC2001, 2 SC2317), plus 2 new SC2016 info notes. The SC2016 notes flag the intentional `$m`/`$i` Rust macro metavariables inside single-quoted fixture lines. *(Corrected in iteration 5: this line first said "10 ... already present at `aad3d343e` (7 SC2001, 3 SC2317), plus 3 new SC2016". Measured with shellcheck 0.9.0 JSON output, `aad3d343e` has 8 (6 SC2001, 2 SC2317) and `a2afe7045` has 10 (6 SC2001, 2 SC2317, 2 SC2016). Iteration 3's own count of 8 was right.)*

**Known residual limits (stated, not fixed):**
- `closure_end` still reads a `|` after a blanked char or string literal as a closure opener. After the `union` fix I found no position where that `|` reaches the header scan from a non-item: macro-pattern arms only exempt their own group. That is a search result, not a proof.
- A `#[path]` on an inline module itself is not followed. *(Closed in iteration 5, `64639b071`.)*
- An out-of-line module declared inside an exempt inline `#[cfg(test)] mod tests { mod x; }` in `src/` is still scanned as production. This is the strict side, unchanged from earlier rounds. *(Closed in iteration 5, `572bc9845`.)*
- *(Added in iteration 5: this round also missed FN-A..E, five layouts where a test-only mark landed on a file production compiles. The test-only marking was global and by reachability from any test decl. See "Re-fix (iteration 5)".)*

#### Re-fix (iteration 5)

**Status:** fixed: requires human verification (logic changes in a CI gate)
**Commits:** 7bcbb6771, 352583381, 49dcdcb05, 5a2cdbe5d, 572bc9845, a9e65afd5, 64639b071
**Files modified:** `scripts/check-no-stub-panics.sh`, `tests/scripts/check-no-stub-panics_test.sh`

**The task brief's open defect was already closed.** The brief restated the iteration-3 const-generic defect, with the four-line reproducer. At `a2afe7045`, before any change in this round, that reproducer gives `FOUND` with `crates/x/src/lib.rs:2: todo!` and `crates/x/src/lib.rs:3: unimplemented!`, and exit 1. Fixture `cfg-test-const-generic` has been the four-line reproducer since `aad3d343e`. It is not the single-line accidental pass the brief describes. No code change was needed for it. It still reports at `64639b071`.

**Problems addressed:**
1. **Identifier or literal after the name (`7bcbb6771`, BLOCKER).** `STUB_IDENT`'s lookahead `(?!\s*[!:.\w])` skipped `todo` before any identifier or number. So `call!(todo now)` with `($m:ident $x:ident) => { $m!() }`, and `call!(todo 1)` with `$x:literal`, printed OK, even though rustc builds both and they panic. `\w` is dropped from the lookahead; `\b` still keeps `todo_x` out. Hits are now deduplicated by offset, keeping the first label in this order: direct, renamed, macro token. So `m!{ use todo as x; }` stays one finding. FIRE fixture: `stub-ident-before-token`.
2. **Bare `!` (`352583381`, BLOCKER).** `call!(todo!)` with `($m:ident !) => { $m!() }` was not reported. STUB needs a delimiter after the `!`, and STUB_IDENT excluded `!`. STUB_IDENT no longer excludes it. A direct `todo!(` that both rules match is one finding, labelled direct. FIRE fixture: `stub-ident-bang-no-delimiter` (`todo!` and `unimplemented !`).
3. **Path last segment (`49dcdcb05`, WARNING; fixed, not just marked).** `call!(core::todo)` with `($($p:tt)*) => { $($p)*!() }` was not reported, because the lookbehind excluded `:`. The lookbehind is now `(?<![\w$.])`. `crate::todo::f` stays clear because the lookahead skips a name followed by `::`. The header now lists the four shapes inside macro tokens that are not read as the macro: `$todo`, a field or method after `.`, a segment followed by `::`, and a name followed by a single `:`. It calls them residual limits and gives a counterexample checked with rustc: `($p:ident :: $f:ident) => { $p!() }` on `call!(todo::f)` builds a panicking call and is not reported. FIRE fixture: `stub-ident-path-last-segment` (`core::todo`, `::core::unimplemented`). The `stub-names-not-macros` PASS fixture still passes.
4. **Header inconsistency (`5a2cdbe5d`, WARNING, doc only).** The header now says that `use crate::todo as t;` for a module named `todo` is reported, because the gate cannot tell it from a rename of the macro. This sits next to the local-variable case, with the same remedy. Fixture `renamed-todo-module` pins the behaviour. It passed before the commit too: it pins behaviour that already existed, and does not show a defect.
5. **FN-A, FN-B, FN-C (`572bc9845`, adversary).** Test-only marking used to be global. Every file reachable from any `#[cfg(test)]` decl was dropped from the findings, even when a production decl also loaded it. `test_only` is now a least fixed point. A file is test-only when one of these holds:
   - its own leading `#![cfg(test)]` says so
   - it is not a crate root, at least one declaration loads it, and every declaration that loads it is exempt (`#[cfg(test)]`, or inside an exempt span) or sits in a test-only file

   Crate roots are `src/lib.rs`, `src/main.rs`, `src/bin/*.rs`, `src/bin/*/main.rs` and the manifest `[lib]`/`[[bin]]` paths. A declaration never makes a crate root test-only. A cycle stays production, which is the strict side. As a side effect, a plain `mod x;` inside an exempt `#[cfg(test)] mod tests { ... }` now counts as a test-only loader. This closes the iteration-4 residual limit, and rustc confirms the file is absent from the non-test build. FIRE fixtures: `test-and-prod-decl-same-file` (FN-A), `test-path-alias-of-prod-file` (FN-B), `test-only-child-shares-prod-file` (FN-C) and `test-alias-of-crate-root`. PASS fixture: `mod-decl-in-exempt-inline-mod`.
6. **FN-D (`a9e65afd5`, adversary).** The gate did not read `include!` at all. `mod_decls` now records each `include!("...")` with a literal argument, `std::include!` included, as a decl with an `include` flag. The target is the literal, relative to the directory of the file holding it. An include! outside an exempt span is a production loader. It keeps its target out of `test_only`, and it pulls a target outside `src/` into the scan, like a production `#[path]` module. One inside an exempt span is a test-only loader. FIRE fixtures: `include-of-test-mod-file` (FN-D) and `include-outside-src`. PASS fixture: `include-in-test-mod`. *(Commit-message inaccuracy: the `a9e65afd5` message says it "reflows one header line the previous commit left over 100 characters". That is wrong. `572bc9845` has no line over 100 characters. The long line came from `a9e65afd5`'s own edit and was reflowed before that commit. Neither commit contains a line over 100 characters. The message was left as it is, because amending would rewrite history.)*
7. **FN-E (`64639b071`, adversary).** `decl_files` built the directory from an inline module's name and ignored its `#[path]`. `mod_decls` now records the `#[path]` literal of each enclosing inline module. The new `decl_dir()` follows rustc:
   - The walk starts at the declaring file's directory, with the non-mod-rs file's own name pending.
   - Each inline module adds that pending name once, then its own name.
   - An inline `#[path]` replaces both with its literal.

   Checked with rustc 1.97.1 in both build modes. In lib.rs, and in a non-mod-rs `a.rs`, `#[path = "fakes"] mod outer { mod helper; }` is `src/fakes/helper.rs`. In `a.rs`, `mod inner { #[path = "p"] mod b { mod c; } }` is `src/a/inner/p/c.rs`. `#[path = "x/y"] mod o { mod i { mod t; } }` is `src/x/y/i/t.rs`. FIRE fixture: `inline-path-mod-prod-decl`, a production decl inside an inline `#[path]` module, which `572bc9845` alone still missed. PASS fixture: `test-mod-in-inline-path-mod` (lib.rs, non-mod-rs and nested), which was a strict-side false positive before.

**Not changed, with evidence:**
- **624 against 623 (WARNING, justified).** The verifier's reading is right. The real tree scans 624 files because of the round-4 scope fix `e6cb6bf89`: `tests/helpers/e2e_fixtures.rs` is loaded by a production `#[path]` in `src/application/cli/commands/eval.rs:34`. The 623 figure in the task brief predates that fix. The acceptance check is now `OK: scanned 624 file(s) (19 test-only), 0 finding(s)`. This round leaves the scan set and the 19-file test-only set byte-identical to `a2afe7045`'s.
- **Brief history note.** The brief says this report records two fix rounds. Before this round it recorded four, and the latest WR-03 commit was `ffef65e05`. This report now records five.
- **Strict-side false positives x07, x08** (`#![cfg(test)]` inner on an inline module; a `{` inside a const-generic argument of a `#[cfg(test)] impl`). Recorded, not changed. Neither occurs in the real tree. The oracle still shows them over-reporting, together with f05 and m05, at the same counts as at `a2afe7045`.

**Harness:** 88 assertions pass (74 before this round, plus 14 new fixtures).
- All 13 new behaviour fixtures failed against the gate before their fix: 10 FIRE and 3 PASS.
- `renamed-todo-module` is a pin, as noted in item 4.

**Verification (after each commit, and again at the end):**
- `./tests/scripts/check-no-stub-panics_test.sh`: 88 assertions passed.
- `./scripts/check-no-stub-panics.sh` on the real tree: `OK: scanned 624 file(s) (19 test-only), 0 finding(s)`. Both the test-only set and the scan set were compared after each module-marking commit, and both are identical to `a2afe7045`'s.
- Whole-tree comparison against the `a2afe7045` gate, over all 877 tracked `.rs` files: 0 per-file hit differences and 0 module-list differences. The real tree has no bare `todo`/`unimplemented` identifier outside comments, and no `include!`.
- rustc oracle, three suites: the round-4 adversary suite, the round-5 suite (`.../scratchpad/gate-adversary-r5/tools/run.py`), and 17 new probe cases (`.../scratchpad/r5/oracle/mine/cases.py`).
  - Copies pointed at the worktree gate are in `.../scratchpad/r5/oracle/`.
  - Every case that compiles agrees with rustc on whether a non-test stub exists, with one exception: the stated residual `i12_residual_path_split`.
  - FN-A..E (s01..s05) were false negatives at `a2afe7045` and agree now.
- `make test-shell-guards`: all 18 suites passed, exit 0.
- `make lint-shell` and `shellcheck --severity=warning` on both scripts: clean. Plain `shellcheck` on the gate: clean.
- Plain `shellcheck` on the harness (0.9.0, JSON) reports 14 notes, all info or style: 6 SC2001, 2 SC2317 and 6 SC2016.
  - 10 of them were already present at `a2afe7045`.
  - The 4 new ones are SC2016, from the intentional `$m`/`$x`/`$p` Rust metavariables in the new single-quoted fixture lines.

**Known residual limits (stated, not fixed):**
- Inside macro tokens, a stub name is not read as the macro in five shapes: `$todo`, after `.`, before `.`, before `::`, and before a single `:`. A matcher that takes such tokens apart can still build the call. The header names this with a rustc-checked example. (The round-5 verifier found the fifth shape, `call!(todo.f)` with `($m:ident . $f:ident) => { $m!() }`, which rustc builds into a panic and the gate does not report; the header listed only four until the follow-up doc fix.)
- `include!` is followed only with a single string-literal argument. `include!(concat!(...))` is not resolved.
- A file's `mod child;` declarations resolve from that file's own location even when the file is reached only through `include!`.
- `closure_end`'s handling of a `|` after a blanked literal is unchanged from iteration 4 (a search result, not a proof).

### WR-02: M-B-31 never says a webhook subscribed to `failed` but not `halted` now receives no delivery

**Status:** fixed (documentation only, no code change)
**Commit:** 30f390bc9
**Files modified:** `MIGRATION.md` (section 11 only), `CHANGELOG.md` (`[Unreleased]` only)
**Cross-check:**
- `src/application/services/run/worker.rs`: `persist_agent_halt` calls `enqueue_webhook_delivery(run, RunStatus::Halted, ...)`. `run_status_to_event_kind` maps this to `RunEventKind::Halted`.
- `webhook_delivery_for_outcome` returns `None` unless `webhook.events.contains(&kind)`.
- `WebhookSpec.events` is a plain `Vec<RunEventKind>`, and the DTO defaults it to empty, so there is no implicit "all" subscription.
- Each schedule tick's run carries the schedule's stored webhook. `PATCH /schedules/{id}` replaces the whole webhook object.

The sentence added to the docs is therefore exactly true.

**Applied fix:**
- M-B-31 required-action cell (section 11.1) now says:
  - a run or schedule whose `webhook.events` subscribes to `failed` but not `halted` gets no delivery for this case, because the delivery is never enqueued
  - add `halted` to `webhook.events`; existing stored schedules must be updated, and a run already queued keeps the events it was submitted with
  - match `halt_reason.reason == "model_unpriced"`
- New section 11.8 operator step 7. It tells operators to:
  - list the schedules with `GET /v1/schedules`
  - add `halted` with `PATCH /v1/schedules/{schedule_id}`, sending the whole `webhook` object (URL, secret and events), because a read returns the secret only as `"***"`
  - match the reason
  - make the same change in run-submitting clients
- The root CHANGELOG `[Unreleased]` bullet for the model_unpriced halt carries the same sentence and points to section 11.8 step 7.
- Sections 9 and 10 and every `[0.11.0]` changelog section are untouched.

**Verification:**
- `make check-migration-allowlist`: pass, exit 0.
- `make check-changelogs`: pass, exit 0 (11 publishable crates).

**Deviation:** this commit's subject uses the prefix `docs(55):` rather than the required `fix(55):`. Changing it would mean rewriting history, which the task forbids, so the commit was left as it is. The message body names the finding and ends with the required trailers.

#### Re-fix (iteration 2)

**Status:** fixed (documentation only, no code change)
**Commit:** 2bd816bc5
**Files modified:** `MIGRATION.md` (section 11.8 step 7 only), `docs/src/api-reference/platform-api.md`

The verifier found WR-02 itself fully resolved. The rejection rested on three things:
- the closure-comma regression in the WR-03 commit, fixed in `70ff58443` (see WR-03)
- the assistant-cursor loop in the WR-01 scan, fixed in `55a2bd8e6` (see WR-01)
- the two minor doc gaps below

**Applied fix:**
- MIGRATION.md 11.8 step 7 now says:
  - `GET /v1/schedules` is paginated: 20 per page by default, at most 100 with `?limit=`. Pass each page's `next_cursor` back as `?cursor=` until it is `null`.
  - `PATCH /v1/schedules/{schedule_id}` needs an API key with the admin role. `patch_schedule` calls `require_admin`, and any other key gets `403` `admin role required`. The `GET` needs authentication only.
- platform-api.md: the `POST /v1/runs` example's `webhook.events` is now `["completed", "failed", "halted", "cancelled", "awaiting_input"]`, so a copied example receives the `model_unpriced` halt delivery.
  - Both added kinds are accepted by `run_controller::parse_event_kind`. `run_status_to_event_kind` maps `Halted` and `Cancelled`, and the worker's transition path enqueues both.
- Sections 9 and 10 and every `[0.11.0]` changelog section are untouched.
- The `docs(55):` prefix on `30f390bc9` is left as it is, because rewriting history is forbidden. The re-fix commit uses `fix(55):`.

**Verification:**
- `make check-migration-allowlist`: pass, exit 0.
- `make check-changelogs`: pass, exit 0 (11 publishable crates).
- `make check-doc-config`: pass (161 YAML blocks, 0 failed).
- `make check-version-strings`: pass.

### WR-01: The boot pricing warning checks only each stored assistant's latest version

**Status:** fixed: requires human verification (logic change in the boot scan)
**Commit:** ee5c261b8
**Files modified:** `src/infrastructure/web/run_api_wiring.rs`, `docs/src/architecture/treasurer.md`, `docs/src/getting-started/configuration.md`, `MIGRATION.md` (section 11.1 M-B-32 and section 11.8 step 6 only)
**Applied fix (the complete fix, not the documentation-only option):**
- New private `collect_agent_version_models(repo, id, page_size, &mut models) -> bool`. It walks each listed assistant's whole history with `list_versions(&id, page_size, cursor)`, ascending, following `next_cursor` until `None`.
  - It collects the model of every agent-kind version whose body has a string `model`.
  - Non-agent versions and versions without a string model are skipped.
- `scan_stored_agent_models_paged` calls it for every item of `repo.list(page_size, cursor, false)`, in place of `get_version(id, latest)`. The semantics are kept:
  - Any page or version-page read error stops the whole scan with `stopped_after = Some(last fully read assistant id)`. This records the position only, never error text.
  - The function still never returns an error, so boot is never blocked.
- Termination guard: a version cursor that is not greater than the previous one, or a `Some` cursor on an empty page, stops the scan the same way. *(Iteration-1 overclaim, corrected in iteration 2: this guard covered only the version cursor. The outer assistant-list cursor had no guard until `55a2bd8e6`; see below.)*
- Duplicate model names are left in the scan list. `unpriced_reachable_models` removes them, as before.
- There are no new `pub` items, no `unwrap`/`expect`/`panic!` in non-test code, and no `#[allow]`.
- Warning line (`unpriced_model_warning_line`) now says that every stored version, including pinned older ones, of the agent-kind assistants stored at boot is checked. It also says that agent-kind assistants or versions created after boot are not checked.
  - The test needles were updated to match.
  - The line still has no tenant id or key material; the forbidden-needle checks are unchanged.
- The same coverage statement is now in treasurer.md (prose and the sample line), the configuration.md bullet, MIGRATION.md M-B-32 and section 11.8 step 6. The treasurer.md sample line was checked to be byte-identical to the rendered format string.

**Tests:**
- `stored_agent_model_scan_reads_agent_kind_models_only` now expects `["stored-old", "stored-a"]`. It failed against the old code before the fix.
- New `stored_agent_model_scan_pages_through_every_version`: a 5-version history plus a second assistant, read with page size 2. The history includes a version with no model and one with a non-string model.
- New `stored_agent_model_scan_stops_at_an_unreadable_version_page`: the double fails the third `list_versions` call. The test checks that the stop is after `a-1` and that the line carries no error text.
- New `stored_agent_model_scan_stops_at_a_non_advancing_version_cursor`: the double always returns cursor `Some(1)`. The test checks that the scan terminates within a 10 s timeout and that `stopped_after` is `Some(None)`.
- The test double `FailsSecondListAssistants` was generalised to `ScriptedAssistants`, which can fail a chosen `list` or `list_versions` call or return a stuck cursor. The existing unreadable-page test uses it with the same behaviour as before.
- Before the fix, all four new or updated scan tests failed. After it, they pass.

**Verification:**
- `cargo fmt --check`: pass.
- `cargo clippy -p paladin-ai --features web-server --lib --tests --offline -- -D warnings`: pass, no warnings.
- `cargo test -p paladin-ai --features web-server --lib --offline run_api_wiring`: 47 passed, 0 failed.
- `make check-doc-config`: pass (161 YAML blocks, 0 failed).
- `make check-version-strings`: pass.
- `make check-migration-allowlist`: pass.

#### Re-fix (iteration 2)

**Status:** fixed: requires human verification (logic change in the boot scan)
**Commit:** 55a2bd8e6
**Files modified:** `src/infrastructure/web/run_api_wiring.rs`

**Problem:** the WR-02 and WR-03 verifiers both found that the outer `repo.list` loop in `scan_stored_agent_models_paged` followed `page.next_cursor` with no advancement guard. This was pre-existing code. A store whose assistant cursor never advances therefore still looped boot, and the iteration-1 report's claim overstated the guard.

**Applied fix:**
- The walk keeps a `HashSet<AssistantId>` of every assistant cursor it has followed.
- Either of these ends the walk like an unreadable page: a cursor already in the set, or a `Some` cursor on an empty page. The scan then records `stopped_after = Some(last fully read assistant id)`, the position only, with no error text, and never returns an error.
- Set membership is used rather than the suggested `next <= prev`. The sqlite and in-memory stores order by byte order, but Postgres orders `assistant_id` by the database collation, and a locale such as `en_US.UTF-8` can differ from byte order. An id comparison could therefore stop a healthy Postgres walk and log a false stop line.
- On any finite store, the set guard also catches every cycle, not just a cursor that repeats immediately.
- The version-cursor guard (`u32`, unambiguous order) is unchanged.
- The function docs now say that the boot scan never loops.
- The stop line still reads "stopped at an unreadable page or version". A store that hands back a non-advancing cursor is a pagination fault of the same kind, so the operator-facing wording and the docs that quote it are unchanged. *(Corrected in iteration 4: on a cursor-guard stop every read succeeded, so the wording was not literally true. No doc outside the code quotes the line. See "Re-fix (iteration 4)".)*
- No new `pub` item, no `unwrap`/`expect`/`panic!` in non-test code, and no `#[allow]`.

**Test:**
- The `ScriptedAssistants` double gains `stuck_list_cursor: Option<AssistantId>`, which names the same next cursor on every `list` page.
- New `stored_agent_model_scan_stops_at_a_non_advancing_assistant_cursor`:
  - setup: page size 1, assistants a-1..a-3, with the stuck cursor set to a-1
  - expects `["model-1", "model-2"]` and `stopped_after == Some(Some(a-2))` within a 10 s timeout
- Before the fix, the test failed by timing out at 10 s. After it, the test passes.

**Verification:**
- `cargo fmt --check`: pass.
- `cargo clippy -p paladin-ai --features web-server --lib --tests --offline -- -D warnings`: pass, no warnings.
- `cargo test -p paladin-ai --features web-server --lib --offline run_api_wiring`: 48 passed, 0 failed.

#### Re-fix (iteration 4)

**Status:** fixed (operator wording, docs and test coverage; no change to scan behaviour)
**Commits:** 22c861bdd, a2afe7045
**Files modified:** `src/infrastructure/web/run_api_wiring.rs`

**Problems addressed:**
1. **Stop line (`22c861bdd`).** The line said "stopped at an unreadable page or version". That is true only for the two read-error stops: an `Err` from `list` and an `Err` from `list_versions`. It is not true for the two cursor-guard stops, where every read succeeded: a repeated or empty-page assistant cursor, and a non-advancing version cursor.
   - The line now reads `stored agent-kind assistant scan for unpriced models stopped at an unreadable or non-advancing page or version ({position}); assistants from there on were not checked -- inspect the assistants store directly`. It is still position-only, with no error text.
   - The `stored_scan_stop_line` doc says why it names both causes: the scan does not record which one stopped it.
   - No doc outside this file quotes the line.
2. **`stopped_after` doc (`22c861bdd`).** It now lists every stop: an unreadable page or version page, a repeated assistant cursor, a `Some` cursor on an empty page, and a version cursor that is not greater than the previous one. It now matches the function docs.
3. **Empty-page arm (`a2afe7045`, coverage gap).** The `ScriptedAssistants` double gains `empty_list_pages`: every `list` page comes back empty with a fresh, never-repeated cursor, `ghost-<call>`. The set of followed cursors alone never stops such a store.
   - New test `stored_agent_model_scan_stops_at_an_empty_page_with_a_cursor` expects a stop after one `list` call, no models, `stopped_after == Some(None)` and an `at start` line, within a 10 s timeout.

**Tests watched failing first:**
- The two cursor-guard tests now assert that the line says `non-advancing`. Both failed before the wording change. The unreadable-page test now also asserts `unreadable`.
- The new empty-page test was run with `page.items.is_empty() ||` removed from the assistant-cursor guard. It failed by timing out at 10 s (`the scan terminates: Elapsed(())`). With the guard restored, it passes.

**Verification:**
- `cargo fmt --check`: pass.
- `cargo clippy -p paladin-ai --features web-server --lib --tests --offline -- -D warnings`: pass, no warnings.
- `cargo test -p paladin-ai --features web-server --lib --offline run_api_wiring`: 49 passed, 0 failed (48 before, plus 1 new).
- There are no new `pub` items, no `unwrap`/`expect`/`panic!` in non-test code, and no `#[allow]`.

---

_Fixed: 2026-11-02_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 5_
