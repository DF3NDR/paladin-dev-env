---
phase: 46-docs-currency-hygiene
plan: 10
subsystem: docs-migration-guide
tags: [migration, semver-gate, awk-window, ci, harness, curr-23, d-01, d-02]

requires:
  - phase: 46-05
    provides: check-gates membership and the shell-guard harness conventions this plan extends
  - phase: 46-08
    provides: corrected M-B-06, BattalionConfig, ConclaveConfig and DR-8 rows, moved verbatim (the load-refusal error text cites section 10.1 row M-B-06)
  - phase: 46-09
    provides: M-B-11 D-24 sentence, moved verbatim
provides:
  - "MIGRATION.md as a two-part guide: '# Upgrading Paladin', section 10 (v0.10.1 -> v0.11.0) above the kept v0.10 record (section 9)"
  - "Eight section-10 headings 10.1 .. 10.8, each holding every v0.11 line of the matching 9.x section"
  - "Register window /^## (9|10)\\.2 / .. /^## (9|10)\\.3 / in scripts/check-migration-allowlist.sh, the ci.yml semver step (byte-identical awk line) and the harness"
  - "Four harness cases for a 10.2 register"
affects: [46-11, 46-12, 46-13, 46-14, every later phase that appends to section 10]

tech-stack:
  added: []
  patterns:
    - "Gate window widened in the same commit as the first row moved, so a Y row can never sit under a heading the gate does not read"
    - "Move proved line by line against a saved git diff -U0 delta rather than by reading the moved blocks"

key-files:
  created: []
  modified:
    - MIGRATION.md
    - scripts/check-migration-allowlist.sh
    - .github/workflows/ci.yml
    - tests/scripts/check-migration-allowlist_test.sh

key-decisions:
  - "Four in-place register rows are split: v0.10.1 text restored in 9.2, same-key v0.11 delta row in 10.2 (D-02, RESEARCH F3)"
  - "10.7 carries the v0.11.0 statement: no item is #[deprecated]; Phases 48, 50 and 51 are clean breaks under ADR-0060, ADR-0061, ADR-0062 (D-00e)"

requirements-completed: [CURR-23]

duration: 13 min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 10: Two-part MIGRATION.md and widened register gate Summary

`MIGRATION.md` is now a two-part guide (section 10, v0.10.1 -> v0.11.0, above the byte-identical v0.10 record in section 9) with the semver register gate widened to read both `9.2` and `10.2` in the script, CI and harness in the same commit as the first row moved.

## Accomplishments

- **Tracer (commit `c72d7442`):** the two-part skeleton, all 128 v0.11 register rows moved to 10.2 (plus the four in-place deltas, 132 rows), and the awk window widened to `/^## (9|10)\.2 /{flag=1; next} /^## (9|10)\.3 /{flag=0} flag` in `scripts/check-migration-allowlist.sh`, in the `ci.yml` `semver` step (step renamed "...MIGRATION.md §9.2 and §10.2 registers"), and in the harness. The awk line is byte-identical in script and CI (`grep -cF` prints 1 in each; the old 9.2-only window prints 0 in each). The harness gains `write_migration_two_parts` and four cases: a `Y` row under 10.2 with its entry is silent; without an entry it fires; `Y` rows in both 10.2 and 9.2 collapse to one pair; a `Y` row under 10.1 is not read.
- **Expansion (commit `e2ec52e9`):** every other v0.11 line moved verbatim into its 10.x section; part 9 restored to the v0.10.1 text with one pointer line under each 9.x heading.
- The tracer verify was re-run against the committed tree (auto-mode gate) and passed before expansion started: log line "Tracer verified end-to-end -- expanding".

### Proof records

- Register key-set comparison (script `check.py` in the scratchpad, never committed): 150 distinct `crate | type` keys before and after, sets equal; 166 rows before, 170 after. Counts rise by exactly +1 for the four in-place keys (`paladin-ai-core | PaladinResult`, `paladin-ai | Settings` (2 -> 3: that type already carried two rows), `paladin-web | ExecuteResponse`, `paladin-ai-core | TraceEvent::NodeFinished/RunFinished`); no other key changes.
- Identity sets: `M-B-NN` ids 22 = 22 (no id missing or new), `DR-NN` ids 26 = 26, row-start counts equal (22 and 26).
- Lines moved (non-blank added lines of `git diff -U0 v0.10.1..HEAD -- MIGRATION.md`, 487 added + 6 changed): 9.1 18 (M-B-05..M-B-22), 9.2 128, 9.3 3, 9.4 13, 9.5 44, 9.6 59, 9.8 216 (8 TENANT-01 note + 208 Library consumers subsection) = 481 lines incl. blanks, 456 non-blank. Check (b): all 456 non-blank added lines found verbatim in part 10, 0 missing. The six in-place lines (206, 207, 225, 228, 946, 948) are the only ones handled specially.
- Part 9 diff: `diff <(git show v0.10.1:MIGRATION.md | sed -n '2,$p' | cat -s) <(part 9 minus pointer lines | cat -s)` is empty. Eight pointer lines exist; every pointer anchor resolves to a `## 10.x` heading slug.
- `grep -c 'TBD' MIGRATION.md` prints 0 after each commit; `.cargo/semver-checks-allowlist.toml` untouched (no `migration_row` value changed).
- `make check-migration-allowlist`, `make test-shell-guards`, `make check-gates`, `make check-doc-config`, `make lint-shell` all exit 0; `cargo test --offline --test legacy_removal_guard --test entity_rename_guard` passes (8 + 3 tests); ci.yml parses as YAML.

## Deviations from Plan

None - plan executed exactly as written, with the following judgement calls recorded for the reviewer.

**Authored text (all inside the four split rows and 10.7, none re-registers a row):**
- The four 10.2 delta rows keep the first two cells verbatim (same key) and carry the inserted v0.11 text verbatim in the cell it came from (leading ". " or "; " dropped, "The" capitalised for `Settings`). Cells the in-place edit did not touch (the Change cell for `PaladinResult`, the Mitigation cell for `ExecuteResponse`, and the Deliberate-breaking cells) are short factual sentences pointing at the v0.10 row in 9.2; the Deliberate cell is `Y` for the three constructible-struct rows (the combined rows were `Y`) and the verbatim `N/A (new type; ...)` text for `TraceEvent`. Req cells carry the requirement ids named in the inserted text (`PRICE-03, PARITY-02`; `PRICE-01`; `LEDGR-04, ALLOW-05`; `PRICE-03`).
- The `PaladinResult` Mitigation insertion contains an unescaped `|` inside a backticked span (`paladin-ai-core | PaladinResult`), carried verbatim from HEAD; the gate's field-6-to-NF scan tolerates it. Escaping it would be a separate cosmetic fix.
- 10.7 text is authored: the two v0.11 sentences carried by 9.7 were rewritten to name the types they referred to as "them", and the Phases 48/50/51 clean-break statement was added (D-00e). Its in-text cross-references already point at 10.2; plan 46-11 handles the remaining section-10 cross-references.
- 10.2 intro reuses the 9.2 intro paragraphs verbatim and adds one sentence naming 10.2 the v0.11.0 register, 9.2 the v0.10 one, both read by the gate.

## Issues Encountered

- The harness's closing "no mutation" assertion (`git status --porcelain -- MIGRATION.md ...`) fails on any uncommitted `MIGRATION.md` change, so `make test-shell-guards` is red between editing and committing (1 of 16 assertions in this harness, nothing else). Each commit's verify was therefore run pre-commit for everything else and the harness re-run green on the committed tree. Pre-existing design, not changed here.
- `make check-migration-allowlist`'s Makefile help text still says "§9.2 register". `Makefile` is outside this plan's `files_modified`; left as is (cosmetic).

## Known Stubs

None.

## Threat Flags

None. The only trust-boundary change is T-46-19, mitigated: the register window now reads both headings in script, CI and harness in the same commit as the first row moved, and the harness proves 10.2 is read and 10.1 is not.

## Next Phase Readiness

Ready for 46-11 (section-10 cross-reference rewrite, G-1..G-7, the 10.8 operator checklist around the TENANT-01 note, the section-10 opening summary).

## Self-Check: PASSED

- Commits found: `c72d7442`, `e2ec52e9`.
- Files exist: `MIGRATION.md`, `scripts/check-migration-allowlist.sh`, `.github/workflows/ci.yml`, `tests/scripts/check-migration-allowlist_test.sh`.
- Acceptance criteria re-run: awk line identical in script and CI, old window absent; line 1 `# Upgrading Paladin`; eight `## 10.x` headings in order above `## 9.1`; harness 16/16 on the committed tree and lists the four 10.2 cases; allowlist untouched; Library consumers and Operators subsections above `## 9.1`; eight pointer lines; part 9 diff empty; TBD count 0.
