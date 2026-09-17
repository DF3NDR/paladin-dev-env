#!/usr/bin/env bash
# 34-check.sh — completeness / ID-uniqueness / SC5 read-only gate for Phase 34.
#
# Usage: 34-check.sh --seed | --final
#
# --seed mode asserts (a)-(d) below (the invariants that hold from plan 34-01
# onward). --final mode additionally asserts (e)-(g), which only hold once the
# phase's later plans have swept every page/warning/example and assembled the
# Phase 35/36 work lists.
#
# DEVIATION (Rule 1 — bug, recorded in 34-01-SUMMARY.md): the plan text
# specifies the SC5 proof as
#   git diff --stat $(git merge-base HEAD main)..HEAD -- . ':!.planning'
# Run literally, this is never empty on this branch: `main` is merged only
# through Phase 26 (`git merge-base HEAD main` = 8ed14aea), so the diff
# against it includes 196 files of already-shipped, already-verified Phase
# 27-33 work that has nothing to do with Phase 34. That would make this gate
# permanently red regardless of Phase 34's own read-only compliance — not the
# literal SC5 the ROADMAP and D-22 ask for ("the phase's commits touch only
# .planning/"). The mechanically correct base for that question is the SHA at
# which Phase 34 itself started, not `main`. PHASE34_BASE_SHA below is fixed
# at that point (the HEAD 34-01 found before making any change) and is
# overridable via the PHASE34_BASE_SHA environment variable for later plans
# or a rebased branch.
PHASE34_BASE_SHA="${PHASE34_BASE_SHA:-ee1fb160f8e743e638b32beb6c4e32be4ede9325}"

set -u

MODE="${1:-}"
if [ "$MODE" != "--seed" ] && [ "$MODE" != "--final" ]; then
  echo "usage: 34-check.sh --seed|--final" >&2
  exit 2
fi

PHASE_DIR=".planning/phases/34-documentation-currency-audit"
AUDIT="${PHASE_DIR}/34-AUDIT.md"
DEFERRED="${PHASE_DIR}/deferred-items.md"
PLACEHOLDER='pending — not yet swept (no signal class run)'

FAIL=0

pass() { echo "PASS: $1"; }
fail() { echo "FAIL: $1"; FAIL=1; }

if [ ! -f "$AUDIT" ]; then
  fail "(setup) $AUDIT does not exist"
  echo "--- 34-check.sh $MODE: 1 assertion(s) failed ---"
  exit 1
fi

# (a) every docs/src/*.md path appears as a row in §2
MISSING_PAGES=$(comm -23 \
  <(find docs/src -name '*.md' | sort) \
  <(grep -oE 'docs/src/[A-Za-z0-9_./-]+\.md' "$AUDIT" | sort -u))
if [ -z "$MISSING_PAGES" ]; then
  pass "(a) every docs/src/*.md path appears in a §2 row"
else
  fail "(a) pages missing from §2: $(echo "$MISSING_PAGES" | tr '\n' ' ')"
fi

# (b) every MB-/RD-/EX- ID is unique at mint time (§1-§4, before the work lists exist).
#
# DEVIATION (Rule 1 -- bug, recorded in 34-09-SUMMARY.md): this assertion originally scanned the
# whole file for a duplicate MB-/RD-/EX- token. That was a correct duplicate-mint detector through
# plan 34-08, when §5/§6/§7 were still the "Empty." stubs plan 34-01 seeded -- every ID appeared
# exactly once, in its own §2/§3/§4 originating row, so any second occurrence really was a bug.
# Plan 34-09's own Task 1 <verify> line (this plan's PLAN.md) and 34-check.sh's own --final
# assertion (g) both *require* every ID to appear a second time, in its §5/§6 work-list row (or the
# EX-nn "confirmed current" list, or deferred-items.md) -- D-03's whole citability guarantee
# ("Phase 35/36 close items by ID") depends on that second occurrence existing. Once §5/§6 are
# populated, the original whole-file scan is structurally guaranteed to "fail" on all 265 IDs,
# which is not a duplicate-mint bug, it is the phase working as designed. The mechanically correct
# scope for "is this ID unique at the point it is minted" is §1-§4 (everything before the first
# "## §5" heading) -- a real duplicate mint (the same ID accidentally assigned to two different
# findings) still fails this narrower check exactly as before; a legitimate §5/§6/§7 cross-reference
# no longer does.
ORIGIN_TEXT=$(awk '/^## §5/{exit} {print}' "$AUDIT")
DUP_IDS=$(echo "$ORIGIN_TEXT" | grep -oE '(MB|RD|EX)-[0-9]+' | sort | uniq -d)
if [ -z "$DUP_IDS" ]; then
  pass "(b) every MB-/RD-/EX- ID is unique at mint time (§1-§4)"
else
  fail "(b) duplicate IDs at mint time (§1-§4): $(echo "$DUP_IDS" | tr '\n' ' ')"
fi

# (c) no settled verdict row (current/stale/missing) has an empty or seeded
#     findings cell — the mechanical form of D-00b.
SETTLED_BAD=$(awk -F'|' -v ph="$PLACEHOLDER" '
  /^\|/ && ($3 ~ /current/ || $3 ~ /stale/ || $3 ~ /missing/) {
    gsub(/^[ \t]+|[ \t]+$/, "", $3)
    verdict = $3
    findings = $4
    gsub(/^[ \t]+|[ \t]+$/, "", findings)
    if (verdict == "current" || verdict == "stale" || verdict == "missing") {
      if (findings == "" || index(findings, ph) > 0) {
        print
      }
    }
  }
' "$AUDIT")
if [ -z "$SETTLED_BAD" ]; then
  pass "(c) no settled verdict row has an empty/seeded findings cell"
else
  fail "(c) settled rows with empty/seeded findings cells found (see rows above)"
fi

# (d) SC5 read-only proof
DIRTY=$(git status --porcelain -- . ':!.planning')
if [ -z "$DIRTY" ]; then
  pass "(d1) git status --porcelain -- . ':!.planning' is empty"
else
  fail "(d1) working tree has non-.planning changes: $DIRTY"
fi

BRANCH_DIFF=$(git diff --stat "${PHASE34_BASE_SHA}"..HEAD -- . ':!.planning' 2>/dev/null || true)
if [ -z "$BRANCH_DIFF" ]; then
  pass "(d2) git diff --stat ${PHASE34_BASE_SHA}..HEAD -- . ':!.planning' is empty"
else
  fail "(d2) non-.planning diff since Phase 34 start ($PHASE34_BASE_SHA): $BRANCH_DIFF"
fi

if [ "$MODE" = "--final" ]; then
  # (e) no §2 row still carries the seeded placeholder
  REMAINING_PLACEHOLDER=$(grep -c -- "$PLACEHOLDER" "$AUDIT" || true)
  if [ "$REMAINING_PLACEHOLDER" = "0" ]; then
    pass "(e) no §2 row still carries the seeded placeholder"
  else
    fail "(e) $REMAINING_PLACEHOLDER occurrence(s) of the seeded placeholder remain"
  fi

  # (f) every examples/*.rs and crates/doc-examples/src/*.rs (except lib.rs) appears in §4
  MISSING_EX=$(comm -23 \
    <( { find examples -name '*.rs'; find crates/doc-examples/src -name '*.rs' ! -name 'lib.rs'; } | sort) \
    <(grep -oE '[A-Za-z0-9_./-]+\.rs' "$AUDIT" | sort -u))
  if [ -z "$MISSING_EX" ]; then
    pass "(f) every examples/*.rs and doc-examples/src/*.rs (non-lib.rs) appears in §4"
  else
    fail "(f) files missing from §4: $(echo "$MISSING_EX" | tr '\n' ' ')"
  fi

  # (g) every MB-/RD-/EX- ID cited in §2/§3/§4 also appears in §5/§6/deferred-items.md
  CITED_IDS=$(grep -oE '(MB|RD|EX)-[0-9]+' "$AUDIT" | sort -u)
  ROUTED_TEXT=$(awk '/^## §5/,/^## §7/' "$AUDIT" 2>/dev/null || true)
  ROUTED_IDS=$(printf '%s\n%s' "$ROUTED_TEXT" "$(cat "$DEFERRED" 2>/dev/null || true)" | grep -oE '(MB|RD|EX)-[0-9]+' | sort -u)
  UNROUTED=$(comm -23 <(echo "$CITED_IDS") <(echo "$ROUTED_IDS"))
  if [ -z "$UNROUTED" ]; then
    pass "(g) every cited MB-/RD-/EX- ID is routed to §5, §6 or deferred-items.md"
  else
    fail "(g) IDs cited but not routed: $(echo "$UNROUTED" | tr '\n' ' ')"
  fi
fi

if [ "$FAIL" -eq 0 ]; then
  echo "--- 34-check.sh $MODE: all assertions PASSED ---"
  exit 0
else
  echo "--- 34-check.sh $MODE: one or more assertions FAILED ---"
  exit 1
fi
