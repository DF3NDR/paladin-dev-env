---
phase: 56-dogfood-findings
plan: 03
subsystem: dogfood-runbooks
tags: [runbook, coverage, keyed-walk, web3sec, stage-b, dogf-02, dogf-04, dogf-05]
status: complete

requires:
  - phase: 56-dogfood-findings
    provides: "plans 56-01 and 56-02: scripts/check-findings-log.sh and the findings log (DF-01..DF-06), so the runbooks tell the maintainer to append entries that parse under make check-findings-log"
provides:
  - "56-RUNBOOK-COVERAGE.md: DOGF-05 / D-17 local coverage walk, paste slots C1..C6, LF/LH snippet proven on a synthetic lcov.info"
  - "56-RUNBOOK-KEYED-WALK.md: DOGF-04 / D-13 / D-14 keyed first-run walk, paste slots K1..K9, key via read -rs only"
  - "56-RUNBOOK-WEB3SEC.md: DOGF-02 / D-15 / D-16 return relay, paste slots W1..W2 (empty), triage checklist, no-return decision section"
affects: [56-06 fills the keyless-half section, 56-08 triages K1..K9, 56-09 triages C1..C6, 56-10 triages W1..W2]

key-files:
  created:
    - .planning/phases/56-dogfood-findings/56-RUNBOOK-COVERAGE.md
    - .planning/phases/56-dogfood-findings/56-RUNBOOK-KEYED-WALK.md
    - .planning/phases/56-dogfood-findings/56-RUNBOOK-WEB3SEC.md
  modified: []

key-decisions:
  - "The coverage snippet sits at column 0 between the lcov-snippet markers, outside the numbered list, so it can be extracted mechanically and copied without breaking the heredoc terminator"
  - "The keyed walk redaction is a shell function (redact) that also masks API_KEY= assignments, not only sk- tokens"
  - "The web3sec return-contract items are quoted byte-for-byte from 47-HANDOFF-web3sec.md (checked programmatically), and W1 stays empty"

requirements-completed: []
requirements-advanced: [DOGF-02, DOGF-04, DOGF-05]  # runbooks only; the human-gated runs and triage (56-06, 56-08, 56-09, 56-10) satisfy them

duration: ~25min
completed: 2026-11-02
---

# Phase 56 Plan 03: Stage B runbooks Summary

**Three maintainer runbooks (local coverage walk, keyed first-run walk, web3sec return relay) with numbered, empty paste slots and agent triage checklists, so any human-gated run can start without waiting on Stage A.**

## Performance

- **Tasks:** 3 of 3 (1 tracer, 2 auto)
- **Files:** 3 created, none modified
- No cargo build or test was run (disk constraint); the only execution was shell and python3.

## Accomplishments

- **Coverage runbook (tracer).** Walks the testing guide's Test Coverage section in its own order, names `scripts/coverage.sh` as the single source of the measured command, states the D-17 pass rule (equal whole percents, both >= 82, same commit), and lists six traps to record before any workaround. The percentage snippet was proven on a synthetic lcov (below).
- **Keyed-walk runbook.** Scratch directory from `mktemp -d`, checked to be outside any git repository, `CARGO_TARGET_DIR` inside it, key via `read -rs OPENAI_API_KEY`, a `redact` function before every paste, install line `cargo install paladin-ai --version 0.11.0 --features cli --locked --bin paladin-cli --root "$WALK/cli"`, cleanup that deletes the onboarding `.env` then the scratch directory and asserts `test ! -e "$WALK"`. Contains the empty `## Verified by the keyless half` section for plan 56-06.
- **web3sec runbook.** Quotes the four section-6 items verbatim (confirmed with a programmatic substring check against the hand-off), SHA `70657919beadbd341a72be2d0107080017a803f0`, baselines 2,683 / 0 / 2 and 2,592 / 0 / 2, a one-DF-per-divergence triage checklist with `**By:** web3sec`, D-04 severities, and the D-16 no-return options (hold the tag, or re-home with owner and date, recorded in the REQUIREMENTS.md row and the todo front matter).

## Tracer self-test (snippet proof)

Run in a scratch directory under the session scratchpad. The snippet was extracted from between `<!-- lcov-snippet:start -->` and `<!-- lcov-snippet:end -->`, fence lines dropped:

```bash
D=<scratch>; printf 'SF:a.rs\nLF:100\nLH:90\nend_of_record\nSF:b.rs\nLF:100\nLH:75\nend_of_record\n' > "$D/lcov.info"
awk '/lcov-snippet:start/{f=1;next}/lcov-snippet:end/{f=0}f' 56-RUNBOOK-COVERAGE.md | grep -v '^```' > "$D/snippet.sh"
(cd "$D" && bash snippet.sh)
```

Output, matched with `grep -qxF`:

```
Lines: 165/200 = 82.50%  (truncated: 82)
```

The tracer feedback gate (auto mode active) re-ran the full task verify after the commit and it passed, so expansion proceeded. `make -n coverage`, `make -n coverage-html`, `make -n services-up` and `make -n services-down` each exit 0.

## Task Commits

1. **Task 1 (tracer): coverage runbook** - `38d3c3a71`
2. **Task 2: keyed-walk runbook** - `3b08bd785`
3. **Task 3: web3sec runbook** - `627a5101c`

## Section headings and paste slots

| Runbook | Sections | Paste slots |
|---------|----------|-------------|
| COVERAGE | Purpose and decision, Pass rule, Before you start, Steps, Traps to record, Redaction, Cleanup, Paste slots, Triage checklist | C1 environment, C2 services, C3 first coverage run, C4 workaround run, C5 snippet output line, C6 stumbles |
| KEYED-WALK | Purpose and decision, Before you start, Steps, Traps to record, Redaction, Cleanup, Verified by the keyless half, Paste slots, Triage checklist | K1 environment, K2 Quickstart run, K3 trace watch-list, K4 CLI install, K5 CLI version, K6 onboarding, K7 setup-check, K8 Anthropic row, K9 stumbles and cleanup line |
| WEB3SEC | Purpose and decision, Return contract, Before you start, Steps, Redaction, Paste slots, Triage checklist, If no return has arrived | W1 consumer return (verbatim), W2 received date and channel |

Every paste slot's fenced block is empty (checked by regex: 6, 9 and 2 empty slot fences).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Snippet moved out of the numbered list**
- **Found during:** Task 1 verify
- **Issue:** First draft indented the snippet inside list step 5. The plan's own extraction (`grep -v '^```'`) would not drop indented fence lines, and an indented `PY` heredoc terminator would never close, so neither the verify nor a maintainer copy-paste would work.
- **Fix:** Put the markers and fenced block at column 0 after the step text.
- **Files modified:** 56-RUNBOOK-COVERAGE.md (before its commit)
- **Commit:** 38d3c3a71

**2. [Rule 1 - Accuracy] Trap 5 worded from the compose file, not from the plan's summary**
- **Found during:** Task 1 (reading `docker/docker-compose.yml`)
- **Issue:** The plan says `make services-up` starts every compose service including `qdrant` and the Redis UIs. In the file, `redis-commander`, `redis-insight` and `qdrant` carry `dev`/`debug` compose profiles, so a plain `up -d` does not start them; `paladin-app` has no profile and a `build:` section, so it is built and started.
- **Fix:** Trap 5 states what the file says and tells the maintainer to confirm against `docker ps` in `C2` and record time and anything built.

**3. [Rule 2 - Missing detail] Two walk facts surfaced as notes in the keyed-walk runbook**
- **Issue:** At the `v0.11.0` tag the Quickstart dependency block says `0.10.0` (read via `git show v0.11.0:...`), and its `main.rs` installs no tracing subscriber, so `RUST_LOG=info` may print nothing and `none` in K3 would prove nothing.
- **Fix:** K2 tells the maintainer to use `0.11.0`, say so, and log the mismatch in K9; K3 asks them to say whether any log lines printed and to name any subscriber they add.

None of these change a plan decision. Task 2 also adds a `redact` function (a second expression for `API_KEY=` lines), which is an addition to the plan's sed one-liner; the plan's exact one-liner is kept in `## Redaction`.

## Auth gates

None.

## Known Stubs

None. The empty paste slots and the empty `## Verified by the keyless half` section are intentional and owned by plans 56-06, 56-08, 56-09 and 56-10 or by the maintainer; they are not wiring gaps.

## Flagged assumptions carried forward (not resolved here)

- **E4 (DOGF-02):** when and whether the web3sec return arrives is outside the tree. The runbook only prepares the relay; the no-return choice stays with the maintainer (P11).
- **E13 (DOGF-05):** whether the local walk reproduces CI is unknown until a Docker machine runs it. RESEARCH A3 (does the password-protected compose Redis pass the unauthenticated probe) is listed as trap 2 rather than assumed.

## Threat Flags

None. No code, endpoint or schema changed; the three files are documentation. Threat register items T-56-11 (key leakage) and T-56-12 (fabricated return) are mitigated as planned: `read -rs`, scratch directory outside any repo, redaction before paste, cleanup, and an empty `W1`.

## Self-Check: PASSED

- FOUND: 56-RUNBOOK-COVERAGE.md, 56-RUNBOOK-KEYED-WALK.md, 56-RUNBOOK-WEB3SEC.md under `.planning/phases/56-dogfood-findings/`
- FOUND commits: 38d3c3a71, 3b08bd785, 627a5101c
