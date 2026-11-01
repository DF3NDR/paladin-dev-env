# Phase 55 deferred items

Out-of-scope discoveries and re-homed requirements recorded by plan executors (scope boundary: not
caused by a plan's own changes, or not deliverable at patch level, so not fixed in-phase). Plan 55-13
reads this file when it writes the carry-forward.

## From plan 55-07 (FIXES-04 re-home, 2026-11-01)

### FIXES-04: `RunStatusValue` has no `Cancelled` value

- Where: `crates/paladin-eval/src/scenario.rs` (`RunStatusValue`, `Assertion::RunStatus`),
  `crates/paladin-eval/src/assertion.rs` (`run_status_matches`), and the `RunStatusValue` definition in
  `docs/schemas/eval-scenario.schema.json`. `paladin_core`'s `RunFinishStatus::Cancelled` has no
  counterpart, so a run that finished `cancelled` matches no `run_status` value and a scenario cannot
  assert `run_status: cancelled`.
- Why: `RunStatusValue` and `Assertion` are exhaustive public enums of the published `paladin-eval`.
  Adding a variant fires `enum_variant_added`, and marking either `#[non_exhaustive]` fires
  `enum_marked_non_exhaustive`; both are major at `--release-type patch` against `0.11.0`. The v0.11.1
  scope guard forbids a non-security `MIGRATION.md` section 11.2 break, which would also make the
  milestone `v0.12.0` (D-05, D-00b).
- Not fixed here: this plan records the gap in the rustdoc, the schema descriptions, the eval guide and
  the requirement registers instead. No variant, no `#[non_exhaustive]`, no WINDOWS row, no semver run.
- Owner: the next minor's first phase. The change is one variant plus
  `UPDATE_EVAL_SCHEMA=1 cargo test -p paladin-eval --test schema_golden`. Closes B5 and 42-06 by pointer
  (the archived 42 deferred-items file is untouched).

## Deferred by CONTEXT (not this phase's requirements)

Every item of `55-CONTEXT.md` `<deferred>`, with its proposed home.

- WINDOWS rows 85-87 (53-REVIEW WR-01..03: `UnknownSchemaVersion` aborting the thread walk, the
  `limit + 1` decode locking an owner out, ownership transfer on an Admin fork or unattributed run):
  code fixes outside FIXES-01..06; a `56.x` decimal phase or `/gsd-phase edit 55` by the maintainer. The
  v0.11.1 exit criterion needs them fixed or waived either way.
- The News API key travelling in the URL (`D-52-03-01`, `news_api_fetcher.rs`): the same `56.x` decimal
  phase.
- Refusing `${...}` in the treasurer webhook `secret` (Phase 54 WR-04): new behaviour; the same `56.x`
  decimal phase.
- `RunStatusValue::Cancelled` and the scenario schema refresh: the next minor (D-05; the FIXES-04 entry
  above).
- Retiring or partially retiring the base-layer items (todo options B/C, section 12.2 rows, allowlist
  entries, a guard rule) and the entity-sense identifier renames: the next minor, on the 55-12 review
  file's findings (D-03). Finishing the content / general-data system (option A) is a feature milestone.
- The pricing-coverage check at graph registration (WINDOWS row 67's original closing condition) and
  enumerating workflow assistants' node models for the boot warning (needs a listing method on
  `AssistantResolver`): a later minor.
- Sharing the warn-once set between boot and settle time through an additive `pub fn` in `paladin-llm`:
  only if a plan finds it free (D-07); otherwise a later minor.
- An `#[ignore]` inventory in the testing guide (E8, DOCS-09): Phase 56 dogfooding of the testing guide,
  or a docs pass.
- The feature-stub TODOs in `trigger.rs` (JSONPath payload matching, cooldown), `service_runner.rs`
  (notification adapter wrappers) and `council_service.rs` (registry parameter) (E9): out, feature work,
  none a reachable panic.
- The all-features CI test job (CIREL-10, D10's second half): out by the milestone-open decision.
- Refreshing `.planning/codebase/CONCERNS.md` and `TESTING.md` (dated 2026-07-30, stale on the commander
  tests): `/gsd-map-codebase`.
