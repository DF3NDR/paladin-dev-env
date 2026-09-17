#!/usr/bin/env bash
# 34-signals.sh — D-07 evidence engine.
#
# Runs the Phase 16 eight signal classes (16-DOCS-01-VERDICTS.md Method) plus
# the Phase 34 ninth class (D-07: the shipped-surface checklist hit, D-08) on
# a single mdBook page. Prints one labelled block per signal class naming the
# literal command run and its result (hits, or the word "none"). This is a
# measurement tool, not a gate — it always exits 0 regardless of what fires.
#
# Usage: 34-signals.sh <path-under-docs/src>
#
# Class 9 reads .planning/phases/34-documentation-currency-audit/34-shipped-tokens.txt
# if present (written by plan 34-02); until then it prints an explicit SKIPPED
# line rather than failing, since that file does not exist yet at plan 34-01.

set -u

F="${1:-}"
if [ -z "$F" ] || [ ! -f "$F" ]; then
  echo "usage: 34-signals.sh <path-under-docs/src>" >&2
  exit 2
fi

PHASE_DIR=".planning/phases/34-documentation-currency-audit"
TOKENS_FILE="${PHASE_DIR}/34-shipped-tokens.txt"

echo "=== 34-signals.sh: $F ==="

# Class 1: version strings vs root Cargo.toml [workspace.package] version
echo "--- class 1: version strings ---"
echo "cmd: grep -nE 'v?[0-9]+\\.[0-9]+\\.[0-9]+' \"$F\""
RESULT_1=$(grep -nE 'v?[0-9]+\.[0-9]+\.[0-9]+' "$F" || true)
if [ -n "$RESULT_1" ]; then
  echo "$RESULT_1"
  echo "compared against: Cargo.toml [workspace.package] rust-version (and any crate version)"
else
  echo "none"
fi

# Class 2: dependency pins vs the owning crate manifest
echo "--- class 2: dependency pins ---"
echo "cmd: grep -nE '^[a-z-]+ *= *\\{? *version' \"$F\""
RESULT_2=$(grep -nE '^[a-z-]+ *= *\{? *version' "$F" || true)
if [ -n "$RESULT_2" ]; then
  echo "$RESULT_2"
  echo "compared against: owning crate's Cargo.toml"
else
  echo "none"
fi

# Class 3: crate names vs ls crates/
echo "--- class 3: crate names ---"
echo "cmd: grep -noE 'paladin-[a-z-]+' \"$F\" | sort -u"
RESULT_3=$(grep -noE 'paladin-[a-z-]+' "$F" | sort -u || true)
if [ -n "$RESULT_3" ]; then
  echo "$RESULT_3"
  echo "compared against: ls crates/"
else
  echo "none"
fi

# Class 4: module/source paths, each fed to test -f
echo "--- class 4: module/source paths ---"
echo "cmd: grep -noE '(crates|src)/[A-Za-z0-9_/.-]+\\.rs' \"$F\""
RESULT_4=$(grep -noE '(crates|src)/[A-Za-z0-9_/.-]+\.rs' "$F" || true)
if [ -n "$RESULT_4" ]; then
  echo "$RESULT_4"
  echo "compared against: test -f on each path"
else
  echo "none"
fi

# Class 5: make targets vs Makefile
echo "--- class 5: make targets ---"
echo "cmd: grep -noE 'make [a-z-]+' \"$F\""
RESULT_5=$(grep -noE 'make [a-z-]+' "$F" || true)
if [ -n "$RESULT_5" ]; then
  echo "$RESULT_5"
  echo "compared against: grep -oE '^[a-z-]+:' Makefile"
else
  echo "none"
fi

# Class 6: workflow and job names vs ls .github/workflows/
echo "--- class 6: workflow / job names ---"
echo "cmd: grep -noE '[a-z-]+\\.yml' \"$F\""
RESULT_6=$(grep -noE '[a-z-]+\.yml' "$F" || true)
if [ -n "$RESULT_6" ]; then
  echo "$RESULT_6"
  echo "compared against: ls .github/workflows/"
else
  echo "none"
fi

# Class 7: error types vs grep -rn in crates/ and src/
echo "--- class 7: error types ---"
echo "cmd: grep -noE '[A-Z][A-Za-z]*Error(::[A-Za-z]+)?' \"$F\""
RESULT_7=$(grep -noE '[A-Z][A-Za-z]*Error(::[A-Za-z]+)?' "$F" || true)
if [ -n "$RESULT_7" ]; then
  echo "$RESULT_7"
  echo "compared against: grep -rn '<type>' crates/ src/"
else
  echo "none"
fi

# Class 8: feature flags narrowed to tokens under a [features] heading
echo "--- class 8: feature flags ---"
echo "cmd: grep -noE '\"[a-z-]+\"' \"$F\""
RESULT_8=$(grep -noE '"[a-z-]+"' "$F" || true)
if [ -n "$RESULT_8" ]; then
  echo "$RESULT_8"
  echo "compared against: [features] headings in any workspace manifest"
else
  echo "none"
fi

# Class 9: shipped-surface hit against the token file plan 34-02 writes
echo "--- class 9: shipped-surface checklist hit ---"
if [ -f "$TOKENS_FILE" ]; then
  echo "cmd: grep -nFf \"$TOKENS_FILE\" \"$F\""
  RESULT_9=$(grep -nFf "$TOKENS_FILE" "$F" || true)
  if [ -n "$RESULT_9" ]; then
    echo "$RESULT_9"
  else
    echo "none"
  fi
else
  echo "class 9: SKIPPED — 34-shipped-tokens.txt not yet written by plan 34-02"
fi

exit 0
