# Phase 55: Correctness Fixes, Test Hygiene & Lint Bar - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-01
**Phase:** 55-correctness-fixes-test-hygiene-lint-bar
**Mode:** `--auto` — every question answered with the recommended option, single pass, no
AskUserQuestion; the codebase was scouted first (26 reachable stub sites measured, enum
exhaustiveness checked, the non-hermetic port path traced) so each recommendation rests on a
measured fact rather than the inventory's wording.
**Areas discussed:** Stub panics and the grep guard; Base-layer and content-processing review;
Unpriced-run status and the boot warning; The `Cancelled` eval value under the patch gate; Seed
row evidence; Hermetic run-API tests and the cancel signal; Lint, duplicate constant, commander
stubs and the rustdoc bar

---

## Stub panics and the grep guard (FIXES-01)

| Option | Description | Selected |
|--------|-------------|----------|
| Shell gate in `check-gates` | Offline bash+python script, shell regression test, Makefile target, CI step — the house release-gate shape | ✓ |
| Rust tree-scan test | A `tests/*.rs` walk like `entity_rename_guard.rs` | |

| Option | Description | Selected |
|--------|-------------|----------|
| Zero exemptions | Nothing in a published crate may panic from a stub; the dead file is deleted so no exemption is needed | ✓ |
| Exemption list | A committed per-file list with reasons | |
| Inline markers | `// stub-guard: allow` comments | |

| Option | Description | Selected |
|--------|-------------|----------|
| Delete `content_filtering_service.rs` | Never compiled, invalid Rust, six `todo!()` fields | ✓ |
| Keep it fenced | Leave the file commented out and exempt it | |

| Option | Description | Selected |
|--------|-------------|----------|
| Existing `RepositoryError::QueryError` | The enum is exhaustive; a new variant is a §11.2 break | ✓ |
| New `RepositoryError::Unimplemented` variant | Clearer, but a patch-level semver break | |

**User's choice:** auto — recommended options.
**Notes:** Measured 26 reachable sites in 3 files (20 storage bodies, 6 in the uncompiled file).

---

## Base-layer and content-processing review (FIXES-02)

| Option | Description | Selected |
|--------|-------------|----------|
| In-phase recorded review, keep/fence defaults, removal deferred | The review the todo asks for is done and written to `55-BASE-LAYER-REVIEW.md`; every `pub` item stays (removal is a break) | ✓ |
| Maintainer-only review, fence only | Fence the stubs, leave the review unscheduled | |
| Retire now (todo option B/C) | §11.2 rows + allowlist entries → milestone becomes `v0.12.0` | |

**User's choice:** auto — recommended option.
**Notes:** The owner position of 2026-10-13 ("keep everything until the review has been done")
and the milestone-open G3 decision ("reviewed and fenced, not finished") fix the defaults.

---

## Unpriced-run status and the boot warning (FIXES-03, FIXES-06)

| Option | Description | Selected |
|--------|-------------|----------|
| New additive `HaltReason::ModelUnpriced { model }` | `HaltReason` is `#[non_exhaustive]`; wire tag `model_unpriced` matches the admission 422 code | ✓ |
| Reuse `LedgerUnavailable` | No new tag, but a misleading reason on the wire | |
| Document the `Failed` mapping in the Treasurer guide | FIXES-03's fallback; not needed, the change is local | |

| Option | Description | Selected |
|--------|-------------|----------|
| Agent definitions + code-registry agents; graph nodes excluded and said so | `AssistantResolver::resolve` is id-keyed and cannot list graphs | ✓ |
| Walk every workflow graph | Needs a new listing method on the resolver port | |

| Option | Description | Selected |
|--------|-------------|----------|
| Boot time | Beside TENANT-08's assertion and the D-14 schedule scan | ✓ |
| Admission time | Admission already refuses agent-kind calls with 422 | |

**User's choice:** auto — recommended options.

---

## The `Cancelled` eval value under the patch gate (FIXES-04)

| Option | Description | Selected |
|--------|-------------|----------|
| Defer to the next minor with a recorded reason | No additive shape exists: `RunStatusValue` and `Assertion` are both exhaustive public enums of a published crate | ✓ |
| Register the break | §11.2 row + allowlist entry; milestone becomes `v0.12.0` for a test-harness convenience — forbidden by the scope guard (not a security fix) | |
| Shape it additively | A sibling assertion is itself a variant on exhaustive `Assertion`; marking either enum `#[non_exhaustive]` is also a break | |

**User's choice:** auto — recommended option.
**Notes:** The roadmap scope note listed exactly these three and asked for the decision here.

---

## Seed row evidence (FIXES-05)

| Option | Description | Selected |
|--------|-------------|----------|
| Decide from the recorded probes | `50.1-SEED-PROBE.md` row 1 usage + listing row 852, `51-EFFORT-PROBE.md` (41 reasoning tokens at `max`) → `reasoning: true` | ✓ |
| Re-probe live | No provider key in the sandbox; the records already satisfy ADR-0061 | |

**User's choice:** auto — recommended option.

---

## Hermetic run-API tests and the cancel signal (TESTS-01, TESTS-02)

| Option | Description | Selected |
|--------|-------------|----------|
| Additive wiring sibling taking an `Option<Arc<dyn PaladinPort>>` | Production `build_run_api` unchanged; tests inject a test-local scripted port | ✓ |
| Skip the tests without network | Not hermetic; the row stays open | |
| Reuse `paladin_eval::ScriptedLlm` | Would invert the layering (eval depends on the workspace crates) | |

| Option | Description | Selected |
|--------|-------------|----------|
| Additive `LocalRunTokens::is_registered` inspector + bounded poll | Smallest change; the registry is what `cancel` consults | ✓ |
| `watch`/`Notify` registration signal | Acceptable alternative, more surface | |

**User's choice:** auto — recommended options.

---

## Lint, duplicate constant, commander stubs and the rustdoc bar (TESTS-03..06, CIREL-08)

| Option | Description | Selected |
|--------|-------------|----------|
| Keep the running commander tests, replace the TODOs with a pointer | The four tests are not `#[ignore]`d and assert the strategy executes | ✓ |
| Delete the four tests | Loses the happy-path proof | |

| Option | Description | Selected |
|--------|-------------|----------|
| Remove all eight allows in this phase, crate-by-crate commits, fix at the source, no allow re-added | The requirement is the whole set; ADR-0033 amended in place | ✓ |
| Crate by crate across phases | Re-carries the debt a fifth time | |

| Option | Description | Selected |
|--------|-------------|----------|
| `doc-coverage-report.md` as a dated measurement record | Bar commands, results at a named SHA, the removed-allow table | ✓ |
| Per-item coverage table | Goes stale; needs a nightly tool | |

**User's choice:** auto — recommended options.
**Notes:** TESTS-05's duplicate was measured as real (`reasoning.rs:257` and `config/layers.rs:342`).

---

## Claude's Discretion

Names of the guard script, wiring sibling, inspector, registry iterator, boot-warning function and
the variant field; error/warning wording within house shapes; the guard's internals and fixtures;
`Subtask.expected_output`'s honest value and whether `ingest_content` refuses or logs; plan and
commit grouping; whether WINDOWS rows 67/73 close here or hand to Phase 57; whether the boot
warning shares `paladin-llm`'s warn-once set.

## Deferred Ideas

WINDOWS rows 85-87, the News API key-in-URL candidate, the webhook-secret placeholder refusal,
`RunStatusValue::Cancelled` (next minor), retiring the base-layer items (next minor), graph-node
pricing coverage, sharing the warn-once set, the `#[ignore]` inventory, the E9 feature stubs,
the all-features CI test job, the codebase-map refresh. Three todos reviewed and not folded
(coverage reproduction → Phase 56; web3sec bump → consumer; OpenAI Responses → next feature
milestone).
