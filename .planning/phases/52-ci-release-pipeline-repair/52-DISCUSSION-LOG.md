# Phase 52: CI & Release Pipeline Repair - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-28
**Phase:** 52-ci-release-pipeline-repair
**Areas discussed:** Semver gate re-pin & allow scope, Pre-merge gate triggers, Release-body builder & its test, Carry-over closeouts

---

## Pending todos (cross-reference)

| Option | Description | Selected |
|--------|-------------|----------|
| Verify local coverage reproduces CI | 2026-08-13 todo; CI-adjacent, folded as a verification note | ✓ |
| Review content orchestration before removal | 2026-10-13 todo; keyword-only match on paladin-core | |
| web3sec single bump to v0.11.0 | 2026-10-14 todo; consumer-owned | |
| OpenAI Responses API for pro tier | 2026-10-20 todo; ruled out at milestone open | |

**User's choice:** fold the coverage-reproduction todo only.

---

## Semver gate re-pin & allow scope

### Release type the re-pinned job evaluates at

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit `--release-type patch` | Strictest; major- and minor-level lints fire; verdict cannot drift when Phase 58 bumps the version | ✓ |
| Explicit `--release-type minor` | Only major-level lints; matches the Phase 44 D-27 local diagnostic | |
| Manifest-derived, no flag | Tool infers from version delta; behaviour at 0.11.0 == 0.11.0 unmeasured; changes if the milestone becomes v0.12.0 | |

**User's choice:** Explicit `--release-type patch`.

### Which crate-wide allows are removed

| Option | Description | Selected |
|--------|-------------|----------|
| Measure, then remove every redundant one | Drop all crate-wide allows in both crates, run the real gate at 0.11.0, re-add only a lint that still fires and has a registered Y row | ✓ |
| Only the six named lines | Exactly CIREL-02's list; the other five core allows stay | |

**User's choice:** Measure, then remove every redundant one.

### MIGRATION register window roll

| Option | Description | Selected |
|--------|-------------|----------|
| Open §11 now; gate reads only §11.2 | New section 11 with an empty 11.2 register; ci.yml step, script and make target retargeted; §9/§10 frozen; allowlist empty | ✓ |
| Keep reading §10.2 too; prune its Y markers | Mutates a shipped record | |
| You decide | Planner settles mechanics within the frozen-sections rule | |

**User's choice:** Open §11 now; gate reads only §11.2.

### Red proof

| Option | Description | Selected |
|--------|-------------|----------|
| Scratch PR + evidence file | Draft PR with one deliberate break; red job URL and check-count line into `52-SEMVER-RED-EVIDENCE.md`; closed unmerged | ✓ |
| Local measurement only | Same command locally; does not prove the CI wiring | |
| Both | Local first to pick the break, then the scratch PR | |

**User's choice:** Scratch PR + evidence file (the CONTEXT notes a local run first to pick the break, which the user's choice does not exclude).

**Notes:** "Next area" after four questions; no follow-ups.

---

## Pre-merge gate triggers

### When End-to-End Tests and Publish Dry Run run pre-merge

| Option | Description | Selected |
|--------|-------------|----------|
| Dry Run every PR; E2E under `release` label | Publish Dry Run cheap enough for every PR; E2E on labelled PRs plus main push; `labeled` trigger type | ✓ |
| Both on every PR | Simplest; every PR pays the Docker stack | |
| Both under `release` label only | Smallest cost; packaging breaks still first caught post-merge | |

**User's choice:** Dry Run every PR; E2E under `release` label.

### `check-migration-allowlist` wiring

| Option | Description | Selected |
|--------|-------------|----------|
| Replace inline step with the script call | One implementation, one test | ✓ |
| Add the script as a new step, keep inline too | Literal reading; two copies can diverge | |
| You decide | Planner picks placement and whether the inline copy goes | |

**User's choice:** Replace inline step with the script call.

### `release` label management

| Option | Description | Selected |
|--------|-------------|----------|
| Document in the release runbook; maintainer applies | Runbook and release-automation.md state it; trigger-policy register records `labeled` | ✓ |
| Auto-label PRs whose branch is `release/**` | More automation, more governed surface | |
| Not applicable | No label-gated job | |

**User's choice:** Document in the release runbook; maintainer applies.

### Required-check policy

| Option | Description | Selected |
|--------|-------------|----------|
| Required where they run | Publish Dry Run required on every PR; E2E required when it runs; semver stays required; branch-protection edit handed to the maintainer | ✓ |
| Informational this phase | Report only; promotion deferred to Phase 58 | |
| You decide | Planner sets the required set | |

**User's choice:** Required where they run.

**Notes:** "Next area" after four questions.

---

## Release-body builder & its test

### Replacement for `jq --arg body` (argv cap)

| Option | Description | Selected |
|--------|-------------|----------|
| `jq --rawfile body FILE` | jq reads the file; nothing crosses argv; minimal diff | ✓ |
| Pipe the body on stdin | Equivalent safety, more lines changed | |
| You decide | Never through argv, never shell-interpolated | |

**User's choice:** `jq --rawfile body FILE`.

### Regression fixture for a >131,072-byte body

| Option | Description | Selected |
|--------|-------------|----------|
| Generate at test time | ~140 KB body built in the mktemp scratch dir; assert the stubbed gh saw `.body | length` over the cap | ✓ |
| Commit a fixture file | Deterministic but a large blob in git | |

**User's choice:** Generate at test time.

### GitHub 125,000-character body cap

| Option | Description | Selected |
|--------|-------------|----------|
| Truncate in-script with a CHANGELOG pointer | Configurable limit, cut at a line boundary, footer to the full changelog, logged, tested | ✓ |
| Fail loudly over the cap | Reintroduces the manual step | |
| Out of scope here | Argv path only; cap deferred to Phase 58 | |

**User's choice:** Truncate in-script with a CHANGELOG pointer.

### Verification against the real workflow

| Option | Description | Selected |
|--------|-------------|----------|
| Shell test + shellcheck + actionlint only | Existing CI checks; no live release call this phase | ✓ |
| Also a `workflow_dispatch` dry run | A `--dry-run` seam and manual dispatch path | |

**User's choice:** Shell test + shellcheck + actionlint only.

**Notes:** "Next area" after four questions; the `finalize-release-body.sh` follow-up was offered and not taken, but CONTEXT D-11 asks the planner to check it for the same overflow path.

---

## Carry-over closeouts

### Disposition of the 24 advisory CodeQL alerts

| Option | Description | Selected |
|--------|-------------|----------|
| Triage each: fix if cheap and real, else register | Pull via the documented `gh api` command; one-line real fixes fixed; false positives get full 11-field rows | ✓ |
| Register all 24, fix none | Fastest; risks dismissing a real finding | |
| Fix all 24, register none | Likely impossible for heuristic false positives | |

**User's choice:** Triage each: fix if cheap and real, else register.

### Closing GitHub issue #41

| Option | Description | Selected |
|--------|-------------|----------|
| Close via API in-phase with evidence comment | One comment citing the v0.11.0 Release URL and the producing release.yml jobs; close as completed | ✓ |
| Draft the comment; maintainer closes | Phase 47 D-03 posture | |
| Restate instead of close | Keep open with what is still missing | |

**User's choice:** Close via API in-phase with evidence comment.

### Publish-order gate reading (D5 / CIREL-09)

| Option | Description | Selected |
|--------|-------------|----------|
| Record as accepted in release-automation.md | Dated paragraph under Canonical Publish Order; no workflow change | ✓ |
| Also add the script as a release.yml step | Belt-and-braces; changes the release surface before Phase 58 | |

**User's choice:** Record as accepted in release-automation.md.

### `review_date` policy for new dismissal rows

| Option | Description | Selected |
|--------|-------------|----------|
| One shared date, next milestone horizon | All Phase 52 rows ~2027-06-30; one sweep re-triages them | ✓ |
| Per-row dates by confidence | More precise, more churn | |
| You decide | Consistent with the existing 2026-12-31 rows | |

**User's choice:** One shared date, next milestone horizon.

---

## Wrap-up

**Question:** "Which gray areas remain unclear?" — options "I'm ready for context" / "Explore more gray areas".
**User's choice:** I'm ready for context.

## Claude's Discretion

- Placement of the `check-migration-allowlist.sh` CI step (semver job vs gate-scripts job).
- Truncation footer wording/anchor, line-boundary rule, and the limit's environment-variable name.
- Which throwaway break the red proof uses.
- Evidence file naming beyond `52-SEMVER-RED-EVIDENCE.md`.

## Deferred Ideas

- `workflow_dispatch` dry-run seam for `release.yml` (Phase 58 release-prep candidate).
- Auto-labelling `release/**` PRs (revisit if the manual label is forgotten).
- Re-check of the three `zip` RustSec exceptions (inventory D8, review date 2026-12-31).
