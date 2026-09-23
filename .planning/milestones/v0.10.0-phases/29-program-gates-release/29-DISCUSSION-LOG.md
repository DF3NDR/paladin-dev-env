# Phase 29: Program Gates & Release - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-09
**Phase:** 29-program-gates-release
**Mode:** `--auto` — every question below was answered by selecting the recommended option;
no `AskUserQuestion` was issued. `[auto]` lines record the selection verbatim.
**Areas discussed:** MIGRATION.md closure & Upgrading page (SHIP-01); Backward-compat proofs
(SHIP-02); Program acceptance audit (SHIP-03); Version bump, changelogs & dry-run publish
(SHIP-04); Close-out hygiene: CI evidence, WINDOWS.md, doc sweep

`[--auto] Selected all gray areas: MIGRATION.md closure & Upgrading page (SHIP-01); Backward-compat proofs (SHIP-02); Program acceptance audit (SHIP-03); Version bump, changelogs & dry-run publish (SHIP-04); Close-out hygiene: CI evidence, WINDOWS.md, doc sweep.`
`[auto] cross_reference_todos — 1 match, "Verify local make coverage reproduces CI's 82.39% figure" (score 0.2 < 0.4) → reviewed, not folded.`

---

## MIGRATION.md closure & Upgrading page (SHIP-01)

`[auto] Q: "How is the 'no TBD' claim proven?" → Selected: "A CI step in the semver job asserting zero TBD in MIGRATION.md" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| CI gate in the `semver` job | Durable; ends the per-phase "TBD owned by a later phase" recalibration pattern | ✓ |
| One-time grep recorded in the audit | Cheaper, but the claim decays the day after release | |

`[auto] Q: "Shape of the §9.8 operator checklist?" → Selected: "One ordered copy-pasteable list, each step naming the concrete command/file" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Single ordered list | Matches overview §9.8's own wording; one read top-to-bottom | ✓ |
| Per-subsystem sub-checklists | More structure, but operators upgrade once, not per subsystem | |

`[auto] Q: "Where does the mdBook 'Upgrading' page live and how does it link MIGRATION.md?" → Selected: "New docs/src/api-reference/upgrading.md, hand-written, repository-URL link; migration-guide.md gains a v0.10.0 pointer" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Standalone hand-written page + repo link | No linkcheck exposure; mirrors §9.1 table and §9.8 checklist | ✓ |
| `{{#include}}` of MIGRATION.md | Renders, but MIGRATION.md's relative `crates/…` links fail `warning-policy = "error"` | |
| Append to migration-guide.md | That page is a v0.1→v0.5 history; SHIP-01 names an "Upgrading" page | |

`[auto] Q: "Does the allowlist/§9.2 CI check stay crate-level?" → Selected: "Tighten to row-level (crate + type via migration_row)" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Row-level set equality | Machine-checks SHIP-01's "matching exactly"; nine pairs expected to match today | ✓ |
| Keep crate-level + one-time audit | Nine rows and nine entries already pass by crate name alone, so mismatched types would slip | |

**Notes:** D-05 adds the doc-08 step-6 public-API diff using `git show v0.9.0:.project/current-exports.txt` rather than adopting `cargo public-api`.

---

## Backward-compat proofs (SHIP-02)

`[auto] Q: "Source of the v0.9 sample config?" → Selected: "Frozen snapshot of git show v0.9.0:config.example.yml committed under tests/fixtures/config/ with provenance header" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Committed snapshot with provenance | CI has no tags (`fetch-depth: 1`); the fixture is the real v0.9 sample | ✓ |
| `git show` at test time | Fails in CI checkouts | |
| Hand-trimmed minimal v0.9 config | Not the file a v0.9 operator actually has | |

`[auto] Q: "What does 'legacy behavior' assert?" → Selected: "Config resolution AND behavioral (route set)" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Config + behavioral | Every new struct == Default / None, every enable flag off, AND the server mounts exactly the v0.9 route set | ✓ |
| Config resolution only | Proves parsing, not behavior; SHIP-02 says "asserts legacy behavior" | |

`[auto] Q: "OpenAPI golden diff mechanism?" → Selected: "Path-restricted deep equality with $ref closure, info.version excluded" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Path-restricted deep equality with `$ref` closure | Exactly the "restricted to pre-existing paths" wording; only `info.version` normalised | ✓ |
| Whole-document diff with new paths stripped | New shared schemas referenced only by new paths would show as spurious additions | |

`[auto] Q: "Where do the tests live?" → Selected: "Config boot test as a root [[test]] target; openapi golden in crates/paladin-web/tests/ with its fixture beside it" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Split by crate | Each test sits next to the generator/loader it exercises; both run under `cargo test --workspace` | ✓ |
| Both at root | The openapi test would reach into a crate's internals from outside | |

---

## Program acceptance audit (SHIP-03)

`[auto] Q: "Where does the audit artefact live?" → Selected: ".project/v0.10.0/09-program-acceptance-audit.md, pointer file in the phase dir" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Corpus doc 09 + phase-dir pointer | The corpus is the permanent source of truth; doc 08 calls this "the post-implementation audit" | ✓ |
| Phase dir only | Archived per milestone; the corpus would lack its own closing record | |

`[auto] Q: "FR-to-test evidence method?" → Selected: "Per-FR table with named test anchors, seeded from VERIFICATION/VALIDATION files and doc-08 rows, script-assisted, human-curated" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Per-FR table with anchors | doc-08 step 5 is per-FR; the anchors already exist in prior artefacts | ✓ |
| Per-phase summary only | Does not answer "every FR has a passing test" | |

`[auto] Q: "Orphan-behavior scope?" → Selected: "Integration/E2E test targets added since v0.9.0 plus paladin-eval scenarios" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Integration targets since v0.9.0 | Behaviors are witnessed by integration tests; bounded and diffable against the tag | ✓ |
| Every test in the tree | Thousands of unit tests; not what "orphan behavior" means | |

`[auto] Q: "Ubiquitous-language conformance?" → Selected: "Table + grep; file deviations, no renames this phase" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| File, don't rename | A public-type rename is an X-10 break in a release-gate phase; `Frontier`/`Vanguard` recorded as accepted alias | ✓ |
| Rename deviations now | Breaks X-03/X-10 at the worst moment | |

`[auto] Q: "Phase 28 bench-overhead FAIL (PRD 07 acceptance 6)?" → Selected: "Accept for v0.10.0 as a documented deviation, re-scope the bar as follow-up — flagged ⚠" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Accept + re-scope (⚠ developer may overturn) | Sinks are opt-in; no default workflow pays the overhead; WINDOWS row + known-limitation + changelog line | ✓ |
| Optimise before shipping | Becomes an X-03 production change; the phase would halt on it | |
| Accept silently | Not acceptable — the verifier asked for an adjudication record | |

`[auto] Q: "M-B-04 vs doc-08 step 7 ('nothing else')?" → Selected: "Cite the recorded Phase 22 stop-and-flag decision; maintainer countersigns" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Cite the recorded decision | M-B-04 was added by ENG-08 with a recorded decision; the audit confirms and asks for countersignature | ✓ |
| Remove M-B-04 | Would hide a real, documented behavior of the new engine | |

**Notes:** D-12 bounds the fix set (docs, tests, citations, register corrections, gates, changelog text); any production change is stop-and-flag. D-17 leaves judgment-tier sign-offs as unchecked boxes for the human.

---

## Version bump, changelogs & dry-run publish (SHIP-04)

`[auto] Q: "When/where is 0.10.0 bumped?" → Selected: "On the feature branch in this phase via cargo release version … --workspace (no tag); tag on the merge commit" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Bump in PR, tag on `main` | v0.9.0 precedent (PR #50); SHIP-04's CI-green claims verifiable pre-merge | ✓ |
| Leave bump to `make release` on `main` | Would discover semver/MSRV results on `main`, after merge | |

`[auto] Q: "Changelog finalization?" → Selected: "Script stamps dated sections; root CHANGELOG curated by hand; paladin-eval's is its initial release" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Script + curated root | Matches `make release`'s own tooling; the root body is the GitHub release body | ✓ |
| All by hand | Twelve files, error-prone; the consistency gate is strict | |

`[auto] Q: "Dry-run publish evidence?" → Selected: "cargo publish --workspace --dry-run; fix make publish-dry-run; publish-crates.sh stays the real carrier" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| `--workspace --dry-run` + Makefile fix | Resolves intra-workspace deps locally; removes `|| true`, wrong package name, missing `paladin-herald`, stale docs path | ✓ |
| Per-crate loop as today | Cannot be green before anything is published; failures are swallowed | |

`[auto] Q: "What counts as 'the release commit' for CI evidence?" → Selected: "Both the PR head with the bump and the tagged merge commit on main" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Both | Phase closes on pre-merge evidence; milestone close records the post-merge run | ✓ |
| PR head only | The tag lands on a different SHA | |

---

## Close-out hygiene: CI evidence, WINDOWS.md, doc sweep

`[auto] Q: "Evidence record shape?" → Selected: "29-CI-EVIDENCE.md mirroring 27/28" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Mirror 27/28 | Established form; local sweep table + CI run table | ✓ |
| Inline in SUMMARY | Harder to find at milestone close | |

`[auto] Q: "WINDOWS.md 25 open rows?" → Selected: "Evidence-based triage in one plan; waive/fix with reasons; new row for the bench FAIL" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Evidence-based triage | `/gsd-ship` blocks on `open_count > 0`; rows never deleted, only moved with a cited reason | ✓ |
| Leave register untouched | Forces a manual override at ship time | |

`[auto] Q: "Doc sweep scope?" → Selected: "Bounded list" (recommended default)`

| Option | Description | Selected |
|--------|-------------|----------|
| Bounded list | Upgrading page, migration-guide pointer, release-checklist order, Makefile path, §4 `Halt` errata | ✓ |
| Full docs audit | Unbounded in a release-gate phase | |

---

## Claude's Discretion

- Wording/layout of the Upgrading page and the curated root changelog.
- Whether the FR-grep helper is kept as a script or left as a fenced command in the audit.
- Audit document file split.
- Plan ordering (suggested waves: SHIP-02 tests + gate tightening → audit + WINDOWS triage →
  MIGRATION closure + Upgrading page → bump + changelogs + evidence).
- Fixture provenance location (per-file header vs sibling README).

## Deferred Ideas

- TraceDispatcher/LogTraceSink overhead optimisation (D-16 follow-up).
- `Frontier` → `Vanguard` rename.
- `cargo public-api` adoption.
- Consolidating `migration-guide.md` and `MIGRATION.md` into one versioned history.
- `release.yml` `dry_run` dispatch switching to `cargo publish --workspace --dry-run`.
- `qdrant --all-features` rustdoc break; webhook DNS-rebinding pinning; LLM-call child spans;
  `FallbackHop.node_id`; live SSE attach; heartbeat wiring; `DeltaMerged` real dispatch/writers;
  `run_run_export` fired edges (WINDOWS #34); FUT-01…05; a v0.11 `MIGRATION.md` scaffold.
- Reviewed todo, not folded: "Verify local `make coverage` reproduces CI's 82.39 % figure".
