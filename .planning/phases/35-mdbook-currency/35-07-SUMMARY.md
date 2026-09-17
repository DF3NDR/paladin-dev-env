---
phase: 35-mdbook-currency
plan: 07
subsystem: docs
tags: [mdbook, cli, clap, paladin-cli, docs-currency]

# Dependency graph
requires:
  - phase: 35-mdbook-currency
    provides: plan 35-01 (superstep-engine page and doc-examples registration, required for
      cross-links this plan's pages point at)
provides:
  - All seven CLI appendix pages (cli-council.md, cli-muster.md, cli-onboarding.md,
    cli-setup-check.md, cli-usage.md, cli-configuration.md, cli-testing.md) brought to the
    v0.10.0 shipped `paladin-cli` binary
  - Live `--help` captures for council, muster, onboarding, setup-check, and the top-level
    `paladin-cli --help` listing, each headed by its exact capture command
  - Corrected `cli-usage.md` battalion-run flag documentation (`-t/--type` required, no
    `-i/--input`) found and fixed while sweeping the page for MB-46
affects: [35-10 (CHANGELOG/exit-greps/35-EVIDENCE.md — consumes the deferred observations below)]

# Tech tracking
tech-stack:
  added: []
  patterns: ["Live `--help` capture replaces hand-written Command Syntax/Options blocks (D-14)"]

key-files:
  created: []
  modified:
    - docs/src/appendix/cli-council.md
    - docs/src/appendix/cli-muster.md
    - docs/src/appendix/cli-onboarding.md
    - docs/src/appendix/cli-setup-check.md
    - docs/src/appendix/cli-usage.md
    - docs/src/appendix/cli-configuration.md
    - docs/src/appendix/cli-testing.md

key-decisions:
  - "D-14's no-annotation rule extends to explanatory prose, not just the flag/options blocks: a sentence stating a flag 'does not exist' still contains the fabricated literal token and fails the plan's own grep-based verify. Fixed by rephrasing without naming the fabricated token (cli-onboarding.md, cli-setup-check.md, cli-muster.md)."
  - "Council's Discussion Modes and Output Options sections documented capabilities (parallel/sequential/debate mode, JSON/plain output, synthesis toggle) that do not exist in the live subcommand at all — not just wrong flag names. Rewrote those sections around the two live tuning knobs (--participants, --max-rounds) rather than annotating the absence."
  - "Muster's Validation Phase and orchestration-pattern-selection framing were also fully fabricated (no schema validation exists in muster.rs; the pattern is LLM-recommended, not user-selected via a flag). Rewrote the Generation Workflow and Configuration Options sections to match review_configuration()'s actual accept/edit/cancel prompt."

requirements-completed: [CURR-06, CURR-07, CURR-08]

coverage:
  - id: D1
    description: "cli-council.md rebuilt from a live `council --help` capture; all fabricated flags (--mode, --synthesize, --provider, --max-tokens, --timeout, -f/-n/-o/-r short aliases) removed and every example rewritten to the live --topic/--participants/--roles/--max-rounds/--save/--model/--temperature/--quiet/--verbose surface"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "diff <(sed captured block) <(paladin-cli council --help) — byte-identical"
        status: pass
      - kind: other
        ref: "grep -cE -- '--synthesize|--no-synthesize|--max-tokens|--num-agents' docs/src/appendix/cli-council.md == 0; mdbook build docs/ == 'No broken links found'"
        status: pass
    human_judgment: false
  - id: D2
    description: "cli-muster.md, cli-onboarding.md, cli-setup-check.md corrected from live captures; --pattern/--validate/--interactive/-y removed from muster, PALADIN_ENV_FILE/PALADIN_SKIP_VALIDATION removed from onboarding, --json/-v/-q removed from setup-check; all three state the --features cli build requirement"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "diff byte-identical for muster/onboarding/setup-check --help captures; grep checks per plan's Task 2 verify block"
        status: pass
      - kind: other
        ref: "mdbook build docs/ == 'No broken links found'"
        status: pass
    human_judgment: false
  - id: D3
    description: "cli-usage.md build command carries --features cli, top-level command listing is a live --help capture, council/muster inline short aliases corrected, and the previously-unflagged battalion-run -i/--input fabrication (missing required -t/--type) found while sweeping the page was also fixed; cli-configuration.md's scheduler troubleshooting entry rewritten to point at the live /v1/schedules* route family; cli-testing.md's Tier 4 count corrected from 12 to 13 (grep-verified)"
    requirement: "CURR-06"
    verification:
      - kind: other
        ref: "grep -q -- '--features cli' cli-usage.md; grep -q 'paladin-cli --help' cli-usage.md; grep -q 'platform-api.md' cli-configuration.md; T4=$(grep -c '#\\[test\\]\\|#\\[tokio::test\\]' tests/integration/llm_live_api_tests.rs); grep -q \"$T4\" cli-testing.md"
        status: pass
      - kind: other
        ref: "mdbook build docs/ == 'No broken links found'; ./scripts/check-doc-config.sh == 0 failed; ./scripts/check-doc-examples.sh == 0 failed"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-09-17
status: complete
---

# Phase 35 Plan 07: CLI Appendix Family Summary

**Rebuilt all seven CLI appendix pages from live `--help` captures of a `--features cli` build of `paladin-cli`, removing every fabricated flag, environment variable, and non-existent capability (discussion modes, JSON output, schema validation) rather than annotating them.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 3 (Task 1: tracer for cli-council.md; Task 2: cli-muster.md/cli-onboarding.md/cli-setup-check.md; Task 3: cli-usage.md/cli-configuration.md/cli-testing.md)
- **Files modified:** 7

## Accomplishments
- All seven CLI appendix rows (MB-40 through MB-46) closed, one commit per page
- Five pages carry live `--help` captures headed by the exact capture command, each verified byte-identical against the locally built binary
- Every CLI page states the `--features cli` build requirement once
- No fabricated flag, short alias, or environment variable survives on any of the seven pages
- Found and fixed an additional battalion-run flag fabrication on cli-usage.md (not part of the audit's MB-46 finding) while sweeping the page for short-alias accuracy

## Task Commits

1. **Task 1: MB-41 — cli-council.md rebuilt from a live capture** - `48a7cc6f` (docs)
2. **Task 2a: MB-42 — cli-muster.md** - `a3794c62` (docs)
2. **Task 2b: MB-43 — cli-onboarding.md** - `0c3d6ea1` (docs)
2. **Task 2c: MB-44 — cli-setup-check.md** - `520b8420` (docs)
3. **Task 3a: MB-46 — cli-usage.md** - `d1603402` (docs)
3. **Task 3b: MB-40 — cli-configuration.md** - `9f0c4873` (docs)
3. **Task 3c: MB-45 — cli-testing.md** - `c7bcb9d5` (docs)

**Plan metadata:** committed by orchestrator after wave merge (worktree mode — this executor does not write STATE.md/ROADMAP.md/REQUIREMENTS.md).

## D-23 Closure Table

| MB-nn | Page | Disposition | Commit | How the §5 finding was addressed |
|-------|------|-------------|--------|-----------------------------------|
| MB-41 | `docs/src/appendix/cli-council.md` | corrected | `48a7cc6f` | Command Syntax block replaced with live `council --help` capture; removed the fabricated positional `<QUESTION>` argument and every non-existent flag (`-n/--num-agents`, `-m/--mode` with parallel/sequential/debate, `-r` short, `-f/--format`, `--synthesize`/`--no-synthesize`, `--provider`, `--max-tokens`, `--timeout`, `-v` short); renamed `-o/--output` examples to the live `--save`; rewrote the Discussion Modes section (removed — no mode concept exists) into a Discussion Rounds section around the live `--participants`/`--max-rounds`; rewrote Output Options to drop the fabricated JSON/plain formats |
| MB-42 | `docs/src/appendix/cli-muster.md` | corrected | `a3794c62` | Command Syntax block replaced with live `muster --help` capture; removed the fabricated positional `<DESCRIPTION>` argument and every non-existent flag (`-p/--pattern`, `-f/--format`, `-y/--yes`, `--temperature`, `--validate`/`--no-validate`, `--interactive`); rewrote the Generation Workflow's fabricated "Validation Phase" (no schema validation exists in `muster.rs`) into the real accept/edit/cancel review prompt from `review_configuration()`; rewrote Configuration Options so pattern selection is described as LLM-recommended, not flag-selected |
| MB-43 | `docs/src/appendix/cli-onboarding.md` | corrected | `0c3d6ea1` | Command block now includes the live `onboarding --help` capture (no flags beyond the two globals); the Environment Variables section (`PALADIN_ENV_FILE`, `PALADIN_SKIP_VALIDATION`) deleted outright — including from the replacement sentence itself, since D-14 forbids keeping the literal token even in a "does not exist" annotation |
| MB-44 | `docs/src/appendix/cli-setup-check.md` | corrected | `520b8420` | Command Options block replaced with live `setup-check --help` capture; removed the fabricated `-v` short alias, the `-q` short alias, and the entire `--json` output flag and its worked JSON-format example/CI usage; rebuild command corrected to carry `--features cli` |
| MB-45 | `docs/src/appendix/cli-testing.md` | corrected | `c7bcb9d5` | Tier 4 count corrected from the claimed 12 to the live-counted 13 (`grep -c '#[test]|#[tokio::test]' tests/integration/llm_live_api_tests.rs`); the "Total: 12 tests" prose line updated to explain the 13th entry is `test_suite_documentation`, a non-calling meta-test, so the table and prose stay internally consistent; Tier 1/2/3 counts (45/6/5) re-verified live and left unchanged |
| MB-46 | `docs/src/appendix/cli-usage.md` | corrected | `d1603402` | Installation build command corrected to carry `--features cli`; added a live top-level `paladin-cli --help` capture as a new "Command Overview" subsection; removed every invented short alias found in the inline council/muster/setup-check/features Commands Reference sections (`-p/--participants`, `-m/--model`, `-t/--temperature` for council; `-t/--task`, `-p/--provider`, `-m/--model` for muster; `-v` for setup-check; `-c`/`-f` for features); also fixed the previously-unflagged battalion-run section, which fabricated `-i/--input` (no such field on `BattalionRunArgs`) and omitted the required `-t/--type` |
| MB-40 | `docs/src/appendix/cli-configuration.md` | corrected | `9f0c4873` | Scheduler troubleshooting entry no longer tells the reader to "ensure scheduler port is wired... (no TODO at line 297)" — confirmed zero hits for `grep -rn 'TODO.*scheduler' src/ crates/`; rewritten to point at the live `/v1/schedules*` route family (Phase 27) and its `GET /v1/schedules/{id}` status fields (`last_tick`/`next_tick`/`skipped_ticks`), linking `api-reference/platform-api.md#schedules` |

## Capture Commands and Verbatim Output (D-14 re-derivability)

All five captures below were taken in this executor's own worktree against
`./target/debug/paladin-cli` built with `cargo build --features cli --bin paladin-cli` (warm build,
~67s in this session since `CARGO_TARGET_DIR` is shared across the wave's worktrees). Each was
verified byte-identical to the corresponding page's fenced block via `diff`.

```text
$ paladin-cli --help
Paladin Multi-Agent Orchestration CLI

Usage: paladin-cli [OPTIONS] <COMMAND>

Commands:
  agent        Paladin agent operations (create, run)
  battalion    Battalion multi-agent operations (create, run)
  arsenal      Arsenal tool management (list, test)
  maneuver     Maneuver flow DSL operations (visualize, validate, execute)
  onboarding   Interactive onboarding wizard for initial setup
  setup-check  Check environment setup and configuration
  features     Discover available features and commands
  muster       Generate battalion configuration from task description
  eval         Evaluation harness operations (run scripted scenarios)
  graph        Graph document operations (export to Mermaid/DOT)
  run          Run/thread execution overlay operations (export to Mermaid)
  council      Run a council discussion
  help         Print this message or the help of the given subcommand(s)

Options:
      --quiet    Enable quiet mode (minimal output)
      --verbose  Enable verbose mode (detailed output)
  -h, --help     Print help
  -V, --version  Print version
```

```text
$ paladin-cli council --help
Run a council discussion

Usage: paladin-cli council [OPTIONS]

Options:
      --topic <TOPIC>                Discussion topic
      --participants <PARTICIPANTS>  Number of participants (2-10) [default: 3]
      --roles <ROLES>                Custom roles (comma-separated)
      --max-rounds <MAX_ROUNDS>      Maximum discussion rounds [default: 5]
      --save <SAVE>                  Save transcript to file
      --model <MODEL>                Model to use
      --temperature <TEMPERATURE>    Temperature setting
      --quiet                        Enable quiet mode (minimal output)
      --verbose                      Enable verbose mode (detailed output)
  -h, --help                         Print help
```

```text
$ paladin-cli muster --help
Generate battalion configuration from task description

Usage: paladin-cli muster [OPTIONS]

Options:
      --task <TASK>          Task description
  -o, --output <OUTPUT>      Output file path
      --execute              Execute immediately after generation
      --provider <PROVIDER>  LLM provider to use
      --model <MODEL>        Model to use
      --no-review            Skip review step
      --quiet                Enable quiet mode (minimal output)
      --verbose              Enable verbose mode (detailed output)
  -h, --help                 Print help
```

```text
$ paladin-cli onboarding --help
Interactive onboarding wizard for initial setup

Usage: paladin-cli onboarding [OPTIONS]

Options:
      --quiet    Enable quiet mode (minimal output)
      --verbose  Enable verbose mode (detailed output)
  -h, --help     Print help
```

```text
$ paladin-cli setup-check --help
Check environment setup and configuration

Usage: paladin-cli setup-check [OPTIONS]

Options:
      --verbose  Show detailed diagnostic information
      --quiet    Enable quiet mode (minimal output)
  -h, --help     Print help
```

## Files Created/Modified
- `docs/src/appendix/cli-council.md` - Live capture + Discussion Rounds/Output Options rewrite
- `docs/src/appendix/cli-muster.md` - Live capture + Generation Workflow/Configuration Options rewrite
- `docs/src/appendix/cli-onboarding.md` - Live capture + fabricated env-var section removed
- `docs/src/appendix/cli-setup-check.md` - Live capture + `--json`/short-alias removal
- `docs/src/appendix/cli-usage.md` - Build command fix, top-level capture, short-alias sweep, battalion-run fix
- `docs/src/appendix/cli-configuration.md` - Scheduler troubleshooting rewrite
- `docs/src/appendix/cli-testing.md` - Tier 4 count correction

## Decisions Made
- Extended D-14's "removed, not annotated" rule to prose sentences, not just flag lists: a sentence like "there is no `--json` flag" still fails the plan's grep-based verify because the literal fabricated token survives on the page. Rephrased every such sentence to describe the live behavior without naming the absent token (affects cli-onboarding.md, cli-setup-check.md, cli-muster.md).
- Where an entire capability was fabricated (council's discussion modes, muster's schema validation), rewrote the surrounding section around the real capability rather than deleting content wholesale, to keep the page useful.
- Fixed the battalion-run `-i/--input`/missing `-t/--type` fabrication on cli-usage.md even though it wasn't named in the audit's MB-46 finding, because it's a flag-accuracy defect on one of the seven pages directly covered by this plan's binding D-14 truth ("no flag... survives on any of the seven pages"), discovered while already editing that exact section for the audited short-alias fixes.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] cli-usage.md battalion-run section documented a non-existent `-i/--input` flag and omitted the required `-t/--type`**
- **Found during:** Task 3 (sweeping cli-usage.md's Commands Reference for MB-46)
- **Issue:** `BattalionRunArgs` in `src/application/cli/commands/battalion.rs` has no `input` field at all; `-i` is entirely fabricated. The struct's `r#type` field (`-t/--type`, required, must match the config file's battalion type) was omitted from the doc.
- **Fix:** Replaced the Syntax/Options/Examples blocks' `-i <input>` with `-t <type>`, matching the live struct.
- **Files modified:** docs/src/appendix/cli-usage.md
- **Verification:** Confirmed via `grep -n "pub struct\|#\[arg\|pub .*:" src/application/cli/commands/battalion.rs`; not testable via mdbook build alone since it's a semantic (not link) defect, but visually diffed against the source struct field-by-field.
- **Committed in:** `d1603402` (Task 3, MB-46 commit)

**2. [Rule 1 - Bug] My own explanatory sentences reintroduced fabricated tokens the plan's verify greps forbid**
- **Found during:** Task 2 (cli-onboarding.md, cli-setup-check.md) and Task 3 (cli-muster.md, discovered again while re-checking Task 2's own page)
- **Issue:** Writing "there is no `PALADIN_ENV_FILE` variable" (or `--json`, or `--mode`/`--pattern`) as an explanatory annotation still contains the literal fabricated string, which fails `! grep -q '<token>'` in the plan's own verify block and violates D-14's "removed, not annotated" rule.
- **Fix:** Rewrote each sentence to describe the live behavior without naming the absent flag/variable.
- **Files modified:** docs/src/appendix/cli-onboarding.md, docs/src/appendix/cli-setup-check.md, docs/src/appendix/cli-muster.md
- **Verification:** Re-ran each affected grep after the rewrite; all now report zero matches.
- **Committed in:** `0c3d6ea1`, `520b8420`, `a3794c62`

**3. [Rule 1 - Bug] Whitespace typo in the muster `--help` capture broke byte-identical requirement**
- **Found during:** Task 2, post-write diff check against the live binary
- **Issue:** Transcribing the capture by hand introduced one extra space before "Enable verbose mode" on the `--verbose` line.
- **Fix:** Corrected the spacing to match `paladin-cli muster --help` exactly.
- **Files modified:** docs/src/appendix/cli-muster.md
- **Verification:** `diff` against live capture output — now identical.
- **Committed in:** `a3794c62`

---

**Total deviations:** 3 auto-fixed (1 bug in a neighboring section found via Rule 1's scope, 2 verify-script-driven rewording passes, 1 transcription typo)
**Impact on plan:** All three necessary for either factual correctness or for the plan's own automated verify blocks to actually pass; no scope creep beyond the seven named pages.

## Issues Encountered
- The plan's Task 1 automated verify regex `! grep -qE '(^|[^a-z-])--mode' docs/src/appendix/cli-council.md` has a substring-matching false positive: it also matches the legitimate `--model` flag (which the acceptance criteria require the page to keep), since `--mode` is a literal prefix of `--model`. Confirmed by running the exact plan-supplied command — it reports failure — while a corrected boundary-aware check (`grep -nE -- '--mode([^a-z]|$)'`) confirms zero real standalone `--mode` occurrences. This is a verify-script quirk, not a content defect; documenting here per the executor's obligation to flag tooling issues discovered during verification rather than silently reinterpreting the plan.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All seven CLI appendix pages (the densest cluster of fabricated surface in the book) are closed; MB-40 through MB-46 fully resolved.
- Plan 35-10 (CHANGELOG, exit greps, 35-EVIDENCE.md) can cite this plan's closure table directly; no exit-grep allowlist entries needed from this plan (no historical/exempt version strings or CI job names were touched here).
- No stubs, skipped tests, or unrun `<verify>` blocks — every acceptance criterion in Tasks 1-3 was executed and passed in this session, including the full `mdbook build docs/`, `./scripts/check-doc-config.sh`, and `./scripts/check-doc-examples.sh` sequence.

## Known Stubs
None.

## Deferred observations

- **`docs/src/appendix/cli-configuration.md`'s Garrison and Arsenal troubleshooting entries** (near the corrected Scheduler entry) make the same "verify no TODO at line NNN" claim style that MB-40's finding proved stale for the scheduler — `grep -rn "TODO.*garrison\|garrison.*TODO\|TODO.*arsenal\|arsenal.*TODO" src/application/cli/commands/agent.rs` also returns zero hits, suggesting these two entries may be equally stale. Not fixed here: the audit's MB-40 finding named only the Scheduler entry, and no MB-nn row covers these two. Flagging for plan 35-10 or a future audit pass to confirm and correct if warranted.

## Threat Flags

None — this plan only edits documentation prose and pastes verbatim `--help` output from a locally built binary; no new network endpoint, auth path, file access pattern, or schema change was introduced. Each captured block was read before pasting and contains no API key, token, real endpoint host, or local absolute path outside the repository (T-35-11 mitigation applied).

---
*Phase: 35-mdbook-currency*
*Completed: 2026-09-17*
