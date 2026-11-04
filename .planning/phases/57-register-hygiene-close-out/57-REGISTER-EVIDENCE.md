# Phase 57 Register Evidence (plan 57-11)

Recorded 2026-11-04 on branch `claude/laughing-dirac-e0h2ax`. Phase head when the rows below were first
re-run: `7ab20afbacf316a578adb9e45880f3fd363495cb` (57-16's metadata commit). `windows status` before any change in 57-11:
`open_count: 26`, `waived_count: 40`, `fixed_count: 34`, `total_count: 100`.

Rule (D-40): "before" cells are copied from the plan SUMMARYs, which measured them on each plan's own start SHA.
"Head" cells are re-run in 57-11, in this session, on the head above, and are the only cells a WINDOWS row is
closed on. A row whose closing evidence is a Docker-gated CI job is flipped only after that job's log is read
(PD-11-4). Raw logs sit in the session scratchpad and are not committed; the lines that matter are quoted here.

## WINDOWS dispositions

| Row | Kind / phase | Disposition | Evidence (re-run on head, or CI run and job id) | Reason cell |
|-----|--------------|-------------|-------------------------------------------------|-------------|
| 56 | lint-warning / 38 | fixed | `cargo doc -p paladin-ai-core --no-deps` (7ab20afba): `Documenting paladin-ai-core v0.11.0 ... Finished`, `warning:` line count 0. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`: exit 0, `Generated .../target/doc/paladin/index.html and 14 other files`. First measured in `55-CORRECTNESS-EVIDENCE.md` Carry-forward. | Closed by Phase 57 (57-11, D-05): cargo doc -p paladin-ai-core --no-deps prints 0 warning lines and cargo doc --workspace --all-features --no-deps with RUSTDOCFLAGS set to -D warnings exits 0 on 7ab20afba (first measured in 55-CORRECTNESS-EVIDENCE.md); see 57-REGISTER-EVIDENCE.md |

## REG-01 SUMMARY frontmatter

pending 57-11 Task 2/3

## REG-03 debug archive

pending 57-11 Task 2/3

## REG-04 judgment read

pending 57-11 Task 2/3

## REG-05 review dispositions

pending 57-11 Task 2/3

## REG-06 security exceptions

pending 57-11 Task 2/3

## Phase gates

pending 57-11 Task 2/3

## CI on the Phase 57 head

pending 57-11 Task 2/3

## Orphaned deferrals

pending 57-11 Task 2/3

## Handover

pending 57-11 Task 2/3
