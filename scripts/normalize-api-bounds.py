#!/usr/bin/env python3
"""Canonicalise the ordering of adjacent auto-trait marker bounds in
`cargo public-api --simplified` output.

Why this exists: rustdoc synthesises auto-trait `impl` bounds (`Send`, `Sync`,
`Unpin`, `Freeze`, `UnsafeUnpin`) for generic types, and the order those bounds
are joined with `+` inside a `where` clause has no guarantee across toolchain
versions -- observed locally as `Sync + Send` and on CI as `Send + Sync` for
the same real API. An un-normalised baseline therefore records a property of
the machine that generated it (which nightly happened to run) rather than a
property of the API itself. This filter sorts only maximal runs of adjacent
marker bounds drawn from a fixed, closed set, alphabetically, so the baseline
is stable across toolchains, runs and machines.

Deliberately does NOT touch anything else on a line: item names, paths,
generics, argument lists, return types, non-marker trait bounds and lifetimes
are left byte-identical. The marker set below is intentionally closed -- a
broader match (e.g. any bound, or any trait path) risks silently reordering
semantically-ordered bounds that are not auto-traits.

Usage:
    cargo public-api --simplified | python3 scripts/normalize-api-bounds.py
    python3 scripts/normalize-api-bounds.py --self-test
"""

from __future__ import annotations

import re
import sys

# Closed, explicit set of auto-trait marker bounds this filter is allowed to
# reorder. Deliberately not "any trait" or "any core::marker::* path": a
# broader match risks reordering bounds that are NOT auto-traits and whose
# order might carry meaning (e.g. supertrait lists). If rustdoc ever
# synthesises a new auto-trait bound, add it here explicitly rather than
# widening the match.
MARKER_BOUNDS = (
    "core::marker::Freeze",
    "core::marker::Send",
    "core::marker::Sync",
    "core::marker::Unpin",
    "core::marker::UnsafeUnpin",
)

# Longest-first isn't required here (none of these strings are prefixes of
# each other), but sorting explicitly documents that the alternation order
# does not matter for correctness.
_MARKER_ALT = "|".join(re.escape(m) for m in MARKER_BOUNDS)

# Matches a maximal run of `+`-joined marker bounds: one marker, then zero or
# more `(whitespace * '+' * whitespace) marker` repetitions. A lifetime, a
# non-marker trait, or any other token is not part of the alternation, so it
# ends the run and is left where it is. `(?<![\w:])` / `(?![\w:])` guard
# against matching inside a longer path-like identifier.
_RUN_RE = re.compile(
    r"(?<![\w:])"
    r"(?:" + _MARKER_ALT + r")"
    r"(?:\s*\+\s*(?:" + _MARKER_ALT + r"))*"
    r"(?![\w:])"
)

# Splits a matched run into its items and the separators between them,
# keeping the exact separator text (spacing) so reassembly preserves the
# input's own spacing style rather than imposing a fixed one.
_SPLIT_RE = re.compile(r"(\s*\+\s*)")


def _normalize_run(match: "re.Match[str]") -> str:
    full = match.group(0)
    parts = _SPLIT_RE.split(full)
    items = parts[0::2]
    seps = parts[1::2]
    if len(items) <= 1:
        # A single marker bound (no '+' chain) is already canonical -- return
        # byte-identical rather than rebuilding it.
        return full
    sep = seps[0]
    return sep.join(sorted(items))


def normalize_line(line: str) -> str:
    """Canonicalise adjacent auto-trait marker bound runs in a single line.

    A line with no marker-bound run is returned byte-identical.
    """
    return _RUN_RE.sub(_normalize_run, line)


def normalize_stream(infile, outfile) -> None:
    for line in infile:
        outfile.write(normalize_line(line))


def _self_test() -> int:
    cases: list[tuple[str, str, str]] = []

    # Case 1: two lines identical except an adjacent marker pair is written in
    # the opposite order normalise to the same output line.
    line_a = "a + core::marker::Sync + core::marker::Send)\n"
    line_b = "a + core::marker::Send + core::marker::Sync)\n"
    cases.append(("adjacent pair converges", normalize_line(line_a), normalize_line(line_b)))

    # Case 2: a maximal run longer than two is emitted in total alphabetical
    # order, and a non-marker token ending the run is left untouched.
    line_c = (
        "impl<W> core::marker::Freeze for X where "
        "Y: core::marker::Unpin + core::marker::Freeze + core::marker::Send + core::marker::Sync, "
        "Z: core::marker::SomeOtherTrait\n"
    )
    expected_c = (
        "impl<W> core::marker::Freeze for X where "
        "Y: core::marker::Freeze + core::marker::Send + core::marker::Sync + core::marker::Unpin, "
        "Z: core::marker::SomeOtherTrait\n"
    )
    cases.append(("maximal run sorted, trailing token untouched", normalize_line(line_c), expected_c))

    # Case 3: a line with no marker-bound run passes through byte-identically.
    line_d = "pub fn foo(bar: &str) -> usize\n"
    cases.append(("no marker run passes through unchanged", normalize_line(line_d), line_d))

    # Case 4: a real line from the RunWorkerPool<W> block (27-VERIFICATION.md
    # gap 4), in both orders observed across toolchains, converges.
    real_local = (
        "impl<W> core::marker::Send for paladin::application::services::run::worker::RunWorkerPool<W> "
        "where alloc::sync::Arc<paladin_battalion::engine::WarEngine<W>>: core::marker::Send, "
        "alloc::sync::Arc<W>: core::marker::Send, "
        "core::option::Option<alloc::sync::Arc<(dyn core::ops::function::Fn"
        "(tokio_util::sync::cancellation_token::CancellationToken) -> "
        "paladin_battalion::engine::WarEngine<W> + core::marker::Sync + core::marker::Send)>>: "
        "core::marker::Send\n"
    )
    real_ci = (
        "impl<W> core::marker::Send for paladin::application::services::run::worker::RunWorkerPool<W> "
        "where alloc::sync::Arc<paladin_battalion::engine::WarEngine<W>>: core::marker::Send, "
        "alloc::sync::Arc<W>: core::marker::Send, "
        "core::option::Option<alloc::sync::Arc<(dyn core::ops::function::Fn"
        "(tokio_util::sync::cancellation_token::CancellationToken) -> "
        "paladin_battalion::engine::WarEngine<W> + core::marker::Send + core::marker::Sync)>>: "
        "core::marker::Send\n"
    )
    cases.append(
        ("real RunWorkerPool<W> line converges across toolchains", normalize_line(real_local), normalize_line(real_ci))
    )

    failed = False
    for name, actual, expected in cases:
        if actual != expected:
            failed = True
            print(f"FAIL: {name}", file=sys.stderr)
            print(f"  actual:   {actual!r}", file=sys.stderr)
            print(f"  expected: {expected!r}", file=sys.stderr)

    if failed:
        return 1

    print("ok")
    return 0


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        return _self_test()
    normalize_stream(sys.stdin, sys.stdout)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
