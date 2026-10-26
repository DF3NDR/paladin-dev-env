#!/usr/bin/env bash
# check-dependency-allowlist.sh
#
# Purity guard for the two innermost crates (Phase 46, CURR-25, D-21,
# ADR-0015). `paladin-core` and `paladin-ports` must stay free of provider
# SDKs, transport clients, storage drivers and web frameworks (ADR-0015
# invariant (i)); ADR-0015 records the measured `[dependencies]` sets that
# satisfy that invariant, and this script pins them so a dependency cannot
# enter either crate without a dated ADR-0015 note.
#
# What is compared: the KEYS of each manifest's `[dependencies]` table, read
# with python3's tomllib -- not package names, so a package-renamed key (the
# `paladin_core` key whose package is `paladin-ai-core`) is compared as
# written. Comparison is exact string equality: a near-spelling
# (`async_trait` for `async-trait`) is one unexpected plus one missing key.
# Any key under `[build-dependencies]`, `[target.<cfg>.dependencies]` or
# `[target.<cfg>.build-dependencies]` also fails -- ADR-0015 measures
# `[dependencies]` only, so a surplus crate must not slip in through a
# sibling table. `[dev-dependencies]` are out of scope and never read.
# An absent or empty `[dependencies]` table, an unreadable manifest, or a
# manifest path that does not exist is a named failure, never a vacuous pass.
#
# Every violation is collected, then printed sorted by crate then key, so two
# runs over the same input print identical output. The script is read-only:
# it opens two files and prints key names (no secret or environment value is
# read or echoed), so parallel runs cannot interfere.
#
# The sets below are mirrored in ADR-0015's 2026-10-24 amendment
# (.planning/decisions/0015-core-ports-dependency-allowlist.md); change both
# in one commit, with a dated note in the ADR, or this guard is silently
# blessing drift.
#
# Inputs are resolved relative to the workspace root (derived from this
# script's own location, never the caller's cwd) and may be overridden via
# CORE_MANIFEST / PORTS_MANIFEST -- added so this guard's regression harness
# (tests/scripts/check-dependency-allowlist_test.sh) can point it at fixtures
# without touching the real tree.
#
# Usage:  ./scripts/check-dependency-allowlist.sh
#         CORE_MANIFEST=/path/Cargo.toml PORTS_MANIFEST=/path/Cargo.toml \
#           ./scripts/check-dependency-allowlist.sh
# Exit:   0 if both [dependencies] key sets equal ADR-0015's sets exactly;
#         1 on any violation or input error.

set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORE_MANIFEST="${CORE_MANIFEST:-${WORKSPACE_ROOT}/crates/paladin-core/Cargo.toml}"
PORTS_MANIFEST="${PORTS_MANIFEST:-${WORKSPACE_ROOT}/crates/paladin-ports/Cargo.toml}"

if ! command -v python3 >/dev/null 2>&1; then
    echo "ERROR: python3 is required for Cargo.toml parsing." >&2
    exit 1
fi

python3 - "${CORE_MANIFEST}" "${PORTS_MANIFEST}" <<'PY'
import os
import sys
import tomllib

core_path, ports_path = sys.argv[1], sys.argv[2]

# ADR-0015's measured sets (2026-10-24 amendment). Keys, exactly as written
# in each manifest's [dependencies] table.
CORE_SET = frozenset([
    "serde", "serde_json", "uuid", "chrono", "thiserror", "async-trait",
    "tokio", "sha2", "blake3", "petgraph", "murmur3", "url", "regex",
    "futures",
])
PORTS_SET = frozenset([
    "paladin_core", "async-trait", "serde", "thiserror", "uuid", "chrono",
    "tokio", "serde_json", "futures", "log", "md5", "mime_guess",
])

CRATES = [
    ("paladin-core", core_path, CORE_SET),
    ("paladin-ports", ports_path, PORTS_SET),
]

# (crate, key, message) so sorting is by crate, then key, then text.
violations = []

for crate, path, expected in CRATES:
    if not os.path.isfile(path):
        violations.append((crate, "", f"{crate}: manifest not found at {path}"))
        continue
    try:
        with open(path, "rb") as fh:
            manifest = tomllib.load(fh)
    except (OSError, tomllib.TOMLDecodeError) as err:
        violations.append((crate, "", f"{crate}: cannot parse {path}: {err}"))
        continue

    deps = manifest.get("dependencies")
    if deps is None:
        violations.append((crate, "", f"{crate}: `[dependencies]` table is absent in {path}"))
    elif not deps:
        violations.append((crate, "", f"{crate}: `[dependencies]` table is empty in {path}"))
    else:
        actual = set(deps.keys())
        for key in actual - expected:
            violations.append((crate, key,
                f"{crate}: unexpected dependency `{key}` (not in ADR-0015's measured set)"))
        for key in expected - actual:
            violations.append((crate, key,
                f"{crate}: missing dependency `{key}` (in ADR-0015's set)"))

    # Sibling tables that would let a surplus crate in beside [dependencies].
    for key in manifest.get("build-dependencies", {}):
        violations.append((crate, key,
            f"{crate}: `build-dependencies` key `{key}` is not allowed"))
    for cfg, table in manifest.get("target", {}).items():
        if not isinstance(table, dict):
            continue
        for sub in ("dependencies", "build-dependencies"):
            for key in table.get(sub, {}):
                violations.append((crate, key,
                    f"{crate}: `target.{cfg}.{sub}` key `{key}` is not allowed"))

if violations:
    for _crate, _key, message in sorted(violations):
        print(message)
    print(f"::error::paladin-core / paladin-ports [dependencies] differ from ADR-0015's measured sets "
          f"({len(violations)} violation(s) above). Change the sets only with a dated note in "
          f".planning/decisions/0015-core-ports-dependency-allowlist.md in the same commit.")
    sys.exit(1)

print(f"paladin-core ({len(CORE_SET)}) and paladin-ports ({len(PORTS_SET)}) "
      f"[dependencies] match ADR-0015's measured sets.")
PY
