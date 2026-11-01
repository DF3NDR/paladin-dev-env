# Phase 55: Correctness Fixes, Test Hygiene & Lint Bar - Context

**Gathered:** 2026-11-01
**Status:** Ready for planning
**Mode:** `--auto` (recommended option selected for every question, single pass; no SPEC.md for
this phase, so the roadmap's five success criteria and FIXES-01..06, TESTS-01..06 and CIREL-08
are the requirement of record)

<domain>
## Phase Boundary

Nothing in a published crate panics from an unfinished stub or misreports a status, the unfinished
content / general-data base-layer code is fenced with a recorded decision, and the test suite and
lint bar are hermetic, non-flaky and warning-free, so Phases 56-58 and the `v0.11.1` tag run on a
clean baseline:

- No `todo!()` / `unimplemented!()` is reachable outside `#[cfg(test)]` in any published crate,
  held by a grep guard in `make check-gates` that fails on a planted one; the twenty reachable
  storage stubs return a typed `Err`; the uncompiled `content_filtering_service.rs` is gone.
- `Field`, `Collection`/`CollectionType`/`Item`, `element`, `FieldVersionService`,
  `CollectionVersioningService`, `planning_service.rs`'s placeholder output and
  `file_content_fetcher.rs`'s ingestion stubs each carry a recorded keep / fence / remove decision;
  what is kept and unfinished refuses cleanly and says so in rustdoc.
- A `DispatchBudget::Unpriced` outcome on an agent-kind run records `Halted` with a named reason;
  the `claude-opus-4-8` seed row's `reasoning` flag matches the probe records; server boot warns
  once per unpriced model reachable by a metered principal without refusing workflow assistants.
- FIXES-04 (`RunStatusValue::Cancelled`) is **re-homed to the next minor with the reason recorded**
  (D-05): no additive shape exists under the patch-level semver gate.
- The two `run_api_wiring::tests::build_run_api_persists_*` tests run against a scripted port with
  no network; `cancel_tests::local_cancel_signals_token` waits on dispatch registration, not a
  sleep; WINDOWS row 73 is dispositioned.
- Single-feature `cargo clippy --tests` on `paladin-llm` is clean, the Phase 51 lint leftovers are
  cleared, the self-hosted provider pair is defined once, the `commander.rs` TODO stubs point at
  their relocation, and `cargo doc` is warning-free under both CI bars with all eight crate-level
  `#![allow(rustdoc::…)]` lines removed and `doc-coverage-report.md` regenerated.

Requirements FIXES-01..06, TESTS-01..06, CIREL-08 (inventory rows B2-B6, E1-E5, E7, E9 in part,
G3, D10 in part; WINDOWS rows 67 and 73). Out of scope: finishing any stub or the content /
general-data system, any `MIGRATION.md` §11.2 break (none here is a security fix), new ports,
adapters or HTTP routes, the all-features CI test job (D10, out by decision), WINDOWS rows 85-87
and the other correctness candidates listed under Deferred Ideas (not in this phase's requirement
set), and the lockstep bump (Phase 58).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by the milestone open, Phases 52-54 and the roadmap — not re-asked)

- **D-00a:** Gates as Phase 54 D-00a: 82 % workspace line coverage (ADR-0006), `make clean-code`
  (which runs `make doc-check`, both rustdoc bars), `make check-gates`, `make test-shell-guards`,
  `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for an intentional
  public-surface change), `make security`, `make check-migration-allowlist`,
  `make check-version-strings`, `make check-doc-config` for any fenced YAML touched in `docs/src`,
  `shellcheck`/`actionlint` for every script and workflow edit, and the manual credential-handling
  review in `.github/instructions/security.instructions.md`. No `unwrap`/`expect`/`panic!` in
  library code. TDD: every behaviour below is watched failing first.
- **D-00b:** Semver as Phase 53 D-02 / Phase 54 D-00b: every public change is judged at
  `--release-type patch` against `0.11.0` for all twelve published packages
  (`scripts/check-semver.sh`, `make check-semver`). The milestone stays `v0.11.1`
  (`53-VERSION-DECISION.md`); the allowlist stays at zero entries and §11.2 stays header-only. A
  change the gate flags is **reshaped to additive or dropped**, never registered: nothing in this
  phase is a security fix, so the "forced break → `v0.12.0`" branch does not apply. Behavioural
  changes (a panic that becomes an `Err`, a `Failed` row that becomes `Halted`) go in
  `MIGRATION.md` §11.1 and `CHANGELOG.md` `[Unreleased]`, never a §11.2 row. No `#[deprecated]`
  attribute is added anywhere (it is a patch-level lint).
- **D-00c:** Frozen records: `MIGRATION.md` §9/§10, the root `CHANGELOG.md` `[0.11.0]` section and
  every crate changelog's published `[0.11.0]` section are byte-frozen; corrections go in
  `[Unreleased]` entries. Logging stays on the `log` facade. ADRs: no new ADR is expected;
  ADR-0033 is amended in place with a dated note (the ADR-0015/0056/0059 pattern). If a planner
  finds one is needed, it takes the next free number from `.planning/decisions/PROMOTION.md`
  (currently **0064**) and advances the line in the same commit.
- **D-00d:** Milestone decisions of 2026-10-27 that bind this phase, as `PROJECT.md` "Current
  Milestone" restates them: **B4** unpriced reachable models get a boot/admission warning and
  workflow assistants are not refused; **D10** the eight crate-level `#![allow(rustdoc::…)]` are
  removed but the all-features CI test job stays out; **G3** the base-layer content/data items
  are reviewed and fenced, not finished; scope guard: **stubs that panic are fenced with an
  `Err`, never finished**.
- **D-00e:** **Prove-then-lock** (Phase 53 D-00d / Phase 54 D-00f): every enumerated site is
  re-measured against the tree before it is changed; what is already closed is recorded with the
  proving command, not re-done. Line numbers in the inventory and requirements have drifted (the
  storage stubs are at `sqlite_content_repository.rs:650-694` and
  `mysql_content_repository.rs:627-671` today, the cancel sleep at `cancel_tests.rs:~359`, the
  commander TODO at `commander.rs:3834`); locate items by content, never by line number. Facts from
  this session's scout (2026-11-01) that the plans must re-confirm:
  - Reachable `todo!()`/`unimplemented!()` outside `#[cfg(test)]` modules, non-comment lines, in
    `crates/*/src` and `src/`: **26 sites in 3 files** — 10 in `sqlite_content_repository.rs`,
    10 in `mysql_content_repository.rs` (both `impl ContentListRepository`, every method returning
    `Result<_, RepositoryError>`), and 6 in `content_filtering_service.rs`, which is commented out
    of `crates/paladin-content/src/services/mod.rs` (lines 6 and 9) and is not valid Rust.
  - `HaltReason` (`paladin-core`, `allowance.rs`) and `RunFinishStatus` (`trace.rs`) are
    `#[non_exhaustive]`; `RepositoryError` (`paladin-ports`, `repository_port.rs`),
    `RunStatusValue` (`paladin-eval`, `scenario.rs:571`) and `Assertion` (`scenario.rs`) are
    **exhaustive**.
  - The `Unpriced` → `Failed` mapping is one arm in `src/application/services/run/worker.rs`
    (~1691), `derive_dispatch_budget` (~1873) documents it; `openapi.json` mentions
    `ledger_unavailable` once, so the halt-reason set may be in the published OpenAPI document.
  - `SELF_HOSTED_PROVIDERS` (`crates/paladin-llm/src/reasoning.rs:257`) and
    `REASONING_WIRE_PROVIDERS` (`crates/paladin-llm/src/config/layers.rs:342`) both spell
    `["ollama", "openai-compatible"]` — 51 IN-01 **reproduces**.
  - `build_run_api_with_cadence` builds the run engine's `PaladinPort` from `Settings` through
    `paladin_port_from_settings_with_cadence` (`run_api_wiring.rs` ~884-892); the two failing
    tests set a fake `OPENAI_API_KEY` and register a `FailingExecutor` that the agent-kind path
    never consults, so the model call leaves the process.
  - `LocalRunTokens` (`src/application/services/run/cancel.rs:179`) exposes `register`, `remove`
    and `cancel_if_local` only — no read-only inspector.
  - The eight allows are at `crates/paladin-ports/src/lib.rs:51-52`, `crates/paladin-llm/src/lib.rs:49`,
    `crates/paladin-storage/src/lib.rs:20`, `crates/paladin-notifications/src/lib.rs:14` and
    `src/lib.rs:117-119`; ADR-0033's 2026-09-17 amendment measured **309** hidden diagnostics
    across the five crates (ports 119, facade 108, llm 69, storage 10, notifications 3).
  - The four `commander.rs` tests at ~3805-3960 are **not** `#[ignore]`d: each runs, asserts
    `result.is_ok()` on a strategy execution, and carries a `TODO:` block; the relocation
    `tests/integration/commander_error_paths_test.rs` holds the four real error-path tests.
  - The `claude_opus_4_8` seed builder sets `.with_reasoning(false)` on the comment "probe 1
    carried no reasoning text; listing capabilities truncated", while its own effort block cites
    `51-EFFORT-PROBE.md` reporting 41 reasoning tokens at `max` paired with `thinking: adaptive`.
- **D-00f:** Ordering from the roadmap: TESTS-01's hermetic wiring seam lands before FIXES-03,
  whose regression test rides on it. CIREL-08 shares its files and bar with TESTS-03/04 and is
  planned with them.

### Stub panics and the grep guard (FIXES-01; B2, DF-03)

- **D-01:** **The guard is an offline shell gate in `make check-gates`, zero exemptions.**
  `scripts/check-no-stub-panics.sh` (name is the planner's) follows the `check-crate-names.sh` /
  `check-version-strings.sh` shape: bash with embedded `python3`, no argument, about a second,
  reports every finding as `file:line` before exiting, and treats "scanned zero files" as a named
  failure. It scans every `.rs` under the `src/` of each publishable package (manifests whose
  `[package] publish` is not `false`, read with `tomllib` — `crates/doc-examples`, `tests/`,
  `benches/`, `examples/` are out). Rule: a `todo!(` or `unimplemented!(` token on a code line
  (line comments, block comments, doc comments and string literals blanked first, the
  `tests/entity_rename_guard.rs` technique) that is not inside a `#[cfg(test)]`-attributed item or
  module block (brace-tracked after the attribute). There is no exemption list and no inline
  marker: nothing in a published crate may panic from a stub. `tests/scripts/check-no-stub-panics_test.sh`
  with fixtures proves both sides (a planted `todo!()` in production code fails; one inside
  `#[cfg(test)] mod tests`, in a `///` example and in a string literal passes); a Makefile
  target joins the `check-gates` prerequisite list and a CI step joins the gate-scripts job
  (`ci.yml` ~139-184). The red proof is the harness fixture plus one run against the real tree
  before the stubs are fixed (the 26 sites listed), captured in the evidence file.
  `[auto] Guard shape — Q: "Shell gate in check-gates, or a Rust tree-scan test like entity_rename_guard.rs?" → "Shell gate in check-gates" (recommended; release gates are shell scripts here, the Rust guard is for vocabulary freezes)`
  `[auto] Exemptions — Q: "Exemption list, inline markers, or zero exemptions?" → "Zero exemptions" (recommended)`
- **D-02:** **Twenty storage bodies return an existing `RepositoryError` variant; the dead file is
  deleted.** `RepositoryError` is exhaustive, so no variant is added; every `ContentListRepository`
  method on `SqliteStore` (feature `sqlite`) and `MySqlContentRepository` (feature `mysql`) returns
  `Err(RepositoryError::QueryError(..))` with one fixed, greppable message shape naming the method
  and backend, e.g. `ContentListRepository::get_by_id is unfinished for the sqlite backend: content
  lists are not persisted`. Both impl blocks' rustdoc say the content-list half is unfinished
  (G3) and point at `55-BASE-LAYER-REVIEW.md`. A table test per backend (feature-gated) proves every
  method returns that `Err` and nothing panics, watched failing first (today they panic).
  `crates/paladin-content/src/services/content_filtering_service.rs` is deleted together with the
  two commented `mod` lines: it never compiled, is not valid Rust, and keeping it would be the
  guard's only exemption. `CHANGELOG.md` `[Unreleased]` entries: `paladin-storage` (behavioural:
  a panic becomes a typed error) and `paladin-content` (file removed; never compiled, no consumer
  effect); `MIGRATION.md` §11.1 one line for the storage behaviour.
  `[auto] Dead file — Q: "Delete content_filtering_service.rs or keep it fenced?" → "Delete" (recommended)`
  `[auto] Error variant — Q: "New RepositoryError variant or an existing one?" → "Existing QueryError" (recommended; the enum is exhaustive, a variant is a §11.2 break)`

### Base-layer and content-processing review (FIXES-02; G3, E9, todo 2026-10-13)

- **D-03:** **The review is done in this phase and recorded; removal is off the table this
  milestone.** Every one of the seven items is a `pub` item of a published crate, so "remove" is
  a §11.2 break that the scope guard forbids; the review still records what the design meant
  each item for (read `.project/Milestone_1-MVP`, the `base/mod.rs` "Retained pending review"
  rustdoc and the todo's option list), its current state, the decision and the evidence, in
  `55-BASE-LAYER-REVIEW.md`, one row per item. The dispositions, each confirmed by the executor's
  own read before it is written:
  - `Field`, `Collection`/`CollectionType`/`Item`, the `element` module: **keep.** Rustdoc on each
    item (not only on `base/mod.rs`) states it is unfinished scaffolding of the content /
    general-data orchestration system, retained pending the ADR-0060 (g) review, with the review
    file named. No behaviour to fence. The residual entity-sense identifiers (`Field.nid`,
    `FieldVersion.node_id` and siblings, `get_node_field_versions`) stay: renaming is a break.
  - `FieldVersionService`, `CollectionVersioningService`: **keep and fence.** Any method whose
    body is a placeholder or returns a fabricated value refuses with an existing `VersioningError`
    variant (no new variant unless the planner proves the enum is `#[non_exhaustive]`) and its
    rustdoc says so; working, tested methods are untouched.
  - `planning_service.rs` `"Expected output from subtask execution"`: **fence.** The LLM plan
    schema carries no expected output, so the plan stops fabricating one: `Subtask.expected_output`
    is set to an honest value (empty, or the subtask's own description — the planner's call,
    stated in `PlanningService` rustdoc) and a test pins that no fabricated sentence appears in a
    produced plan. Not finished (no new LLM prompt field).
  - `file_content_fetcher.rs`: **fence.** The three metadata helpers keep returning `None` (already
    honest) with rustdoc saying they are not implemented; `ContentIngestionPort::ingest_content`,
    which validates and then returns `Ok(())` having ingested nothing, refuses instead with a
    named `Err` saying ingestion is unfinished — unless the planner finds a shipped path that
    depends on the no-op `Ok(())`, in which case the no-op stays, logs one `warn!`, and the review
    file records why.
  - The "finish" (todo option A) and "retire" (options B/C, with §12.2 rows, allowlist entries and
    a guard rule) paths are recorded in the review file as next-minor candidates. The todo
    `2026-10-13-review-content-data-orchestration-base-items.md` is closed by this phase
    (`resolves_phase: 55`) with the review file as its disposition.
  `[auto] Review owner — Q: "In-phase recorded review with keep/fence defaults, or hand the review to the maintainer and only fence?" → "In-phase recorded review" (recommended; G3 asks for the review, the no-break rule fixes the defaults)`

### Unpriced-run status and the boot warning (FIXES-03, FIXES-06; B3, B4, WINDOWS row 67)

- **D-04:** **`Unpriced` records `Halted` with a new, additive `HaltReason::ModelUnpriced`.**
  `HaltReason` is `#[non_exhaustive]`, so adding `ModelUnpriced { model: String }` is not a
  semver break; wire form `{"reason":"model_unpriced","model":"…"}`, `as_str()` `"model_unpriced"`
  (the same code the admission 422 uses), herald line `⛔ halted: model <name> has no
  treasurer.pricing row` (names the model, never a tenant or key — D-00g of the Treasurer
  phases). The worker's `Unpriced` arm emits `RunFinishStatus::Halted` with that reason and
  persists through `persist_agent_halt`, exactly like the other two halts; the `Failed` row and
  its "has no treasurer.pricing row" failure text go; `derive_dispatch_budget`'s rustdoc follows.
  The Treasurer-guide fallback in FIXES-03 is not needed: the mapping is local. Docs: the halt
  reason tables in `docs/src/architecture/treasurer.md` and `docs/src/api-reference/platform-api.md`
  gain the third reason; if the OpenAPI document enumerates halt reasons, `make openapi`
  regenerates `openapi.json` and `openapi_matches_committed_baseline` is refreshed (additive).
  Regression test: an agent-kind run driven through `RunWorkerPool::run_once` with a Treasurer
  whose price table lacks the model, on TESTS-01's hermetic seam (D-08), asserting the run row
  reads `halted` with `halt_reason.reason == "model_unpriced"` and the SSE terminal event agrees.
  `MIGRATION.md` §11.1 and `CHANGELOG.md` `[Unreleased]` record the status change.
  `[auto] Reason shape — Q: "New HaltReason variant, reuse LedgerUnavailable, or document the Failed mapping?" → "New additive variant model_unpriced" (recommended)`
  — **Reversibility:** reversible — a variant on a non-exhaustive enum; the wire tag is new.
- **D-05 (FIXES-04): `RunStatusValue::Cancelled` is re-homed to the next minor, with the reason
  recorded; no code change in `paladin-eval` this phase.** Measured: `RunStatusValue` and
  `Assertion` are both exhaustive public enums of `paladin-eval`, a published package under the
  Phase 52 patch gate; adding a variant to either fires `enum_variant_added` (major), marking either
  `#[non_exhaustive]` fires `enum_marked_non_exhaustive`, and a sibling assertion is a variant on
  `Assertion` — so no additive shape exists, and the scope guard forbids a non-security §11.2 row
  (which would also make the milestone `v0.12.0` for a test-harness convenience). Phase 55
  delivers the record: FIXES-04 moves to the `REQUIREMENTS.md` deferred list beside PROV-05 with a
  dated note and its traceability row reads re-homed (owner: the next minor's first phase), the
  ROADMAP SC2 clause is reconciled by a dated note (the Phase 54 FIRST-08 pattern), rustdoc on
  `RunStatusValue` and `Assertion::RunStatus` and `docs/src/user-guides/eval-harness.md` say a
  cancelled finish matches no value until the next minor, the phase's `deferred-items.md` carries
  it, and inventory B5 / 42-06 are closed by pointer. No WINDOWS row is opened (the ledger counts
  defects this milestone must close; SHIP-10 accepts "re-homed with an owner and a date"). No
  semver run is spent on it.
  `[auto] Patch-gate call — Q: "Defer to v0.12.0 with a recorded reason, register the break (milestone → v0.12.0), or shape it additively?" → "Defer with recorded reason" (recommended; the other two are closed off by D-00b and the measured enums)`
  — **Reversibility:** reversible — nothing is built; the next minor adds the variant as the
  one-line change the inventory wanted.
- **D-06 (FIXES-05): the `reasoning` flag follows the two recorded probes, no live re-probe.**
  The seed row is corrected against `50.1-SEED-PROBE.md` (`### Probes: claude-opus-4-8`, row 1's
  usage carries `output_tokens_details.thinking_tokens`; listing row 852 `effort.supported:
  true`) and `51-EFFORT-PROBE.md` (`### claude-opus-4-8`: 0 reasoning tokens with thinking
  disabled, 41 at `max` paired with `thinking: adaptive`): the model reasons, so
  `.with_reasoning(true)`, and the builder comment cites those evidence lines instead of "not
  evidenced". The registry test near `registry/mod.rs:1734` pins the row; `verified_on` stays
  the probe date (ADR-0061's "seeded only when measured end to end" is met by the record; no key
  is available here and none is needed). If the executor's read of the records finds no reasoning
  tokens anywhere, the flag stays `false` and the evidence file says so — prove-then-lock.
  Effect to record in `paladin-llm`'s `CHANGELOG.md` `[Unreleased]`: the Commissary / direct-config
  300 s reasoning-timeout default now applies to this model.
  `[auto] Evidence — Q: "Re-probe live or decide from the recorded probes?" → "From the recorded probes" (recommended)`
- **D-07 (FIXES-06): one boot-time `warn!` per unpriced model that a metered principal can reach,
  in `build_run_api_with_cadence`, beside the TENANT-08 assertion.** A pure function
  (`unpriced_reachable_models(..)`, name the planner's) takes: whether a Treasurer is configured,
  whether any ceiling exists (`allowance.tenants` or `allowance.api_keys` non-empty — that is "a
  metered principal exists"), the `PriceTable`, and the reachable model names; it returns the
  sorted, deduplicated unpriced set. Reachable = every `AgentDefinition.model` in the agents
  config plus every code-registry Paladin's `data.model` (an additive iterator on
  `paladin_web::AgentRegistry` if none exists). Workflow assistants' graph-node models are **not**
  enumerable through `AssistantResolver::resolve` (id-keyed, no listing) and are excluded; the
  warning text and `docs/src/architecture/treasurer.md` say so. One `log::warn!` per model under
  `PRICING_LOG_TARGET`: the model is reachable by a metered principal but has no
  `treasurer.pricing` row; agent-kind calls on it are refused at admission (`422 model_unpriced`)
  or halted at dispatch (`model_unpriced`, D-04); workflow assistants are not refused. Boot never
  refuses (B4). The D-08 settle-time warn-once set in `paladin-llm` is untouched (a boot line
  plus a first-settle line is acceptable; sharing the set through an additive `pub fn` is the
  planner's call only if it costs nothing). Table test over configured/unconfigured, no ceilings,
  priced/unpriced, duplicate names. WINDOWS row 67 is dispositioned on this evidence: waived with
  the reason that the operator contract (price every model a metered principal can reach) stands,
  the boot warning now names agent models, and graph-node pricing coverage is the accepted
  contract of the B4 decision — or handed to Phase 57 with the pointer.
  `[auto] Reachable set — Q: "Agent definitions + registry, or also walk every workflow graph?" → "Agent definitions + registry; graph nodes excluded and said so" (recommended; the resolver cannot list)`
  `[auto] When — Q: "Boot or admission time?" → "Boot" (recommended; admission already refuses with 422, and FIXES-06 sits beside TENANT-08's boot assertion)`

### Hermetic run-API tests and the cancel signal (TESTS-01, TESTS-02; E1, E2, WINDOWS row 73)

- **D-08:** **An additive wiring sibling injects the run engine's `PaladinPort`; production is
  unchanged.** `build_run_api_with_cadence` gains a sibling (or a small `#[non_exhaustive]`
  overrides struct with builder methods, born non-exhaustive) that accepts an
  `Option<Arc<dyn PaladinPort>>` and uses it in place of `paladin_port_from_settings_with_cadence`;
  `build_run_api` and `build_run_api_with_cadence` keep their signatures and bodies byte-for-byte.
  The two `build_run_api_persists_*` tests inject a test-local scripted `PaladinPort` double (the
  `cancel_tests`/`worker_tests` `engine_factory` + `UnusedPaladinPort` pattern, one canned
  answer, no network), drop the `OPENAI_API_KEY` env dance and the `serial` group it needed, and
  pass with the network off (proof: a run with no proxy / `HTTPS_PROXY` unset recorded in the
  evidence file). `paladin_eval::ScriptedLlm` is not used: `paladin-eval` depends on the
  workspace crates, so a root dev-dependency would invert the layering. WINDOWS row 73 → `fixed`
  with the two test names and the no-network run as evidence (or handed to Phase 57 with the
  pointer).
  `[auto] Seam — Q: "Additive wiring sibling taking a port, or make the tests skip without network?" → "Additive wiring sibling" (recommended; a skip is not hermetic)`
- **D-09:** **The cancel test waits on registration, not time.** `LocalRunTokens` gains an
  additive read-only inspector (`is_registered(&RunId) -> bool`; a `watch`/`Notify` signal is the
  alternative if the planner prefers, the inspector is smaller); the test polls it on a short
  interval inside its existing 30 s outer timeout and cancels only once the run is registered.
  The fixed 50 ms sleep and its comment go. Flake proof: the test run at least 20 times under the
  full parallel `cargo test --workspace --lib --bins` load locally, zero failures, recorded.

### Lint, duplicate constant, commander stubs and the rustdoc bar (TESTS-03..06, CIREL-08; E3-E5, E7, D10)

- **D-10:** **Lint leftovers are fixed at the source, never silenced.**
  - TESTS-03: the unused `chat` binding is removed; `cadence::PinnedSpread` / `pin_waiter_spread`
    and `fallback::test_support::ProfiledStubPort` are scoped to the feature combinations or the
    `#[cfg(test)]` paths that use them — no blanket `#[allow(dead_code)]`. Proof: `cargo clippy -p
    paladin-llm --tests --no-default-features --features <f>` for each provider feature, the matrix
    recorded in the evidence file.
  - TESTS-04: the four named sites, fixed as named (the `NODE_CACHE_KEY_VERSION` link in
    `engine/cache_key.rs`, the redundant explicit links in `reasoning.rs`, dead
    `thinking_effort_wire_value` under default features, the unused `provider_factory.rs` import).
  - TESTS-05: one `pub(crate) const SELF_HOSTED_PROVIDERS` in `reasoning.rs` (the reasoning domain
    owns the pair); `config/layers.rs` imports it and `REASONING_WIRE_PROVIDERS` goes; a test
    asserts both call sites see the same pair. 51 IN-01 is closed as reproduced and fixed, with the
    two measured sites.
  - TESTS-06: the four commander tests keep their running happy-path assertions; the four `TODO:`
    blocks are replaced by one doc-comment line each pointing at
    `tests/integration/commander_error_paths_test.rs` and the test that holds the relocated
    error-path coverage. A test is deleted instead only if the planner finds it asserts nothing
    beyond construction. The stale "5 `#[ignore]` tests" line in `.planning/codebase/CONCERNS.md`
    is not edited (map refresh is `/gsd-map-codebase`).
  `[auto] Commander stubs — Q: "Delete the four tests or point them at the relocation?" → "Keep the running tests, replace the TODOs with the pointer" (recommended)`
- **D-11:** **All eight allows go in this phase, measured first, fixed at the source, no allow
  re-added at any level.** First measurement with the eight lines removed: both bars (`cargo doc
  --workspace --no-deps` with zero `warning:` lines, and `RUSTDOCFLAGS="-D warnings" cargo doc
  --workspace --all-features --no-deps`), per-crate diagnostic counts recorded against ADR-0033's
  309 figure of 2026-09-17. Then every diagnostic is fixed where it is written: broken intra-doc
  links become fully qualified paths or plain code spans (the Phase 36 `RD-nn` and WINDOWS row 56
  `cost.rs` pattern), redundant explicit links become shorthand `[`Type`]`, invalid HTML tags are
  wrapped in backticks. Item-level `#[allow(rustdoc::…)]` is forbidden too (same silence); the
  `#![warn(rustdoc::…)]` line above the ports allows stays. Commits are crate-by-crate (ports,
  llm, storage, notifications, facade) so each intermediate head is green under both bars.
  ADR-0033 is amended in place with a dated note closing its "kept and justified, not cleared, for
  v0.10.0" amendment (D-00c), no new ADR. `make doc-check` (in `make clean-code`) and the two CI
  steps are the proof.
  `[auto] Scope — Q: "Remove all eight now, or crate by crate across phases?" → "All eight in this phase, crate-by-crate commits" (recommended; the requirement is the whole set)`
  — **Reversibility:** reversible, but re-adding any allow needs a reason in ADR-0033 — the
  amendment records that none is needed.
- **D-12:** **`docs/src/appendix/doc-coverage-report.md` is regenerated as a dated measurement
  record, not a per-item coverage table.** The "Archived — historical" banner becomes
  "Regenerated 2026-11 (Phase 55, CIREL-08)"; the page records the two bar commands and their
  zero-warning results at the phase head SHA, the per-crate `#![warn(missing_docs)]` roots, and
  the table of the eight removed allows with the diagnostics each had hidden. No new tool (the
  nightly `--show-coverage` is not introduced; the pinned nightly stays api-surface only). The
  `SUMMARY.md` link is unchanged; `make check-doc-config` runs if any fenced YAML is touched.
  `[auto] Report shape — Q: "A coverage table per crate, or a dated measurement record?" → "Dated measurement record" (recommended)`

### Evidence, registers and the semver line (all requirements)

- **D-13:** One `55-CORRECTNESS-EVIDENCE.md` holds the prove-then-lock results (the 26-site
  measurement and the guard's red run, the rustdoc counts before and after, the clippy feature
  matrix, the 20-run flake proof, the no-network test run), and `55-BASE-LAYER-REVIEW.md` holds
  D-03. `scripts/check-semver.sh` runs on the phase head and its twelve lines are recorded, as
  `53-VERSION-DECISION.md` did: the additive items here (the `HaltReason` variant, the wiring
  sibling, the `LocalRunTokens` inspector, the `AgentRegistry` iterator, the `pub(crate)` const)
  are expected to read `223 checks: 223 pass` — any red line is reshaped, never registered.
  `make api-surface-update` and the CHANGELOG entries cover the intentional additions
  (`HaltReason::ModelUnpriced`, the wiring sibling, the inspector, the iterator). `deferred-items.md`
  carries FIXES-04 and the Deferred Ideas below.

### Research reconciliation (2026-11-01, after `55-RESEARCH.md`; amends the decisions above in place)

`55-RESEARCH.md` measured every D-00e fact against `a78338ddb` and raised six open questions. These
resolutions bind the plans; where they narrow a decision above, this note wins and the decision's
original text stays as the record of what was first assumed.

- **D-06 (FIXES-05) is reversed on the evidence: the `reasoning` flag stays `false`; FIXES-05 closes
  as verified-not-wrong.** `ModelProfile.reasoning` (`crates/paladin-llm/src/registry/model_profile.rs`)
  is documented as "whether the default call to this model returned reasoning text", not "can emit
  reasoning tokens". The 50.1 probe row 1 records `reasoning text: absent`; the 51 probe's default
  and adaptive rows report 0 reasoning tokens, and the 41 tokens appear only on a non-default
  `max` + `thinking: adaptive` request. Sibling rows with the same evidence are `false`. So the
  seed is correct under the field's contract; D-06's "the model reasons → true" read the wrong
  contract. The deliverable: correct the builder comment (drop the "listing truncated" reasoning;
  cite the 50.1 row-1 `absent` and the 51 default/adaptive 0-token lines), keep
  `.with_reasoning(false)`, pin the row in the registry test
  (`anthropic_opus_4_8_seed_matches_the_transcript` or the planner's name), and record
  "FIXES-05 verified against both probe records, flag not wrong" in `55-CORRECTNESS-EVIDENCE.md`
  so audit G7 / inventory B6 close on that line. No CHANGELOG entry (no behaviour changes).
  Widening the field to "can reason" (which would move the 300 s timeout default for every such
  model and need a sibling sweep) is deferred, not taken.
- **D-08: "byte-for-byte" means the public contract, not the literal text.** The body of
  `build_run_api_with_cadence` moves into a `pub(crate)` function that takes the optional
  `Arc<dyn PaladinPort>` override; the two public functions keep their signatures and delegate
  with `None`. No public item is added, so `make api-surface` reads unchanged and no CHANGELOG
  line is owed for the seam itself.
- **D-00f ordering is not binding.** `worker_tests.rs` already holds a hermetic test for the exact
  arm (`agent_kind_run_whose_model_became_unpriced_fails_without_calling_the_llm`); FIXES-03's
  red-then-green is that test asserting `Halted` + `model_unpriced` (renamed accordingly). TESTS-01
  and FIXES-03 may land in either order or in parallel; an end-to-end check through the D-08 seam
  is optional, not required.
- **D-07: the boot warning names every class it cannot see.** Besides graph-node models, runtime-
  provisioned agents and stored agent-kind assistants are not in `AgentRegistry` at boot. If
  `AssistantRepositoryPort::list` exists (research assumption A6, planner confirms first), the
  pure function also takes the stored agent-kind assistants' models through one bounded page walk
  on the D-14 pattern; otherwise they are a named gap. The warning text and
  `docs/src/architecture/treasurer.md` list the uncovered classes either way.
- **D-03 fences in `CollectionVersioningService`: least-destructive, recorded.**
  `compare_collection_versions` is working and tested; only `detect_collection_changes` fabricates
  item ids with `Uuid::new_v4()`. The fence is a documented nil-UUID sentinel ("item not tracked")
  in `ItemAdded`/`ItemRemoved` with rustdoc saying so, not a refusal of the whole method; the
  review file records the choice. `FieldVersionHistory.node_id` on a field with no versions returns
  `Err(VersioningError::EntityNotFound(field_id))` after the test near
  `field_version_service.rs:596` is read.
- **D-01: the guard resolves out-of-line test modules.** A naive per-file scan reports 31 sites;
  the true count is 26 because `#[cfg(test)] mod name;` declarations pull in whole files (e.g.
  `engine/test_support.rs`). The scanner resolves such declarations to `name.rs` / `name/mod.rs`
  and excludes those files; the harness fixture includes this case. The red run reads 26 lines in
  3 files.
- **D-11 sizing: 474 diagnostics under the default bar, 455 under the all-features bar, two
  classes, zero `invalid_html_tags`.** Per crate (default bar): ports 187, facade 140, llm 117,
  storage 27, notifications 3; 444 `unresolved link`, 30 `redundant explicit link target`. The two
  bars flag different sets, so every fix is checked under both. 380 diagnostics carry no source
  span and are located by grepping the printed link text. The facade's `invalid_html_tags` allow
  is removed with nothing to fix. ADR-0033's 309 figure is superseded by this measurement in the
  amendment note.
- **D-10 matrix: every feature of `crates/paladin-llm/Cargo.toml`, `mock`-only included.** The
  `reasoning_defaults` test module (ten warnings under `mock` alone) is gated on "a provider
  feature is on"; the measured 13-combination loop is the proof.
- **D-13 budget:** a full `scripts/check-semver.sh` over twelve packages is about 10-15 minutes
  here (46 s measured for `paladin-eval`); crates.io was reachable through the proxy. `actionlint`
  and `cargo-llvm-cov` are absent: the CI "Workflow Lint" and "Coverage" jobs are the proof of
  record for those two, recorded as unrun locally.

### Claude's Discretion

- Exact names: the guard script, the wiring sibling or overrides struct, the inspector, the
  registry iterator, the boot-warning function, the `HaltReason` variant's field name (D-01,
  D-04, D-07, D-08, D-09).
- Error and warning wording within the house shapes (names the entry or the model, never a key
  value; backend text only to `log::error!`).
- The guard's internals: the python/bash split, the comment-blanking and brace-tracking
  implementation, fixture layout (D-01).
- `Subtask.expected_output`'s honest value (D-03) and whether `ingest_content` refuses or stays a
  logged no-op, per the caller read the review file records.
- Plan and commit grouping; which crate's rustdoc fixes land in which plan (D-11).
- Whether WINDOWS rows 67 and 73 are closed in this phase's final plan or handed to Phase 57 with
  evidence pointers (either is acceptable; the row text must cite the test names and the
  warning test).
- Whether the boot warning shares `paladin-llm`'s warn-once set (D-07).

### Folded Todos

- **Review the unfinished content / general data orchestration system before removing any
  base-layer item** (`.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md`,
  score 0.6, `resolves_phase: 55`): the whole of D-03. Its option list (A finish, B retire,
  C partial) and the owner position of 2026-10-13 ("keep everything until the review has been
  done; do not let a consumer count decide it") are the review file's inputs; the todo closes on
  the review file.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements of record
- `.planning/ROADMAP.md` "Phase 55: Correctness Fixes, Test Hygiene & Lint Bar" — goal, five
  success criteria, scope notes (FIXES-01 reachability, the FIXES-04 semver decision point, the
  TESTS-01 → FIXES-03 ordering, CIREL-08 placement).
- `.planning/REQUIREMENTS.md` "Correctness Fixes (FIXES)", "Tests and Lints (TESTS)", CIREL-08
  and the mapping note; the "v2 Requirements" list where FIXES-04 is re-homed (D-05).
- `.planning/PROJECT.md` "Current Milestone: v0.11.1 Dogfood Cleanup" — decisions at open (B4,
  D10, G3) and the scope guards (stubs fenced with an `Err`, never finished; no non-security
  break).

### Phase input of record
- `.project/v0.11.1/01-carried-gaps-inventory.md` §B rows B2-B6, §E rows E1-E5, E7, E9, §D row
  D10, §G row G3 — sources, severities, dispositions.
- `.project/v0.11.1/03-suggestions-and-scoping.md` §7 — the `v0.11.1`-vs-`v0.12.0` naming rule.
- `.planning/WINDOWS.md` rows 67 (unpriced graph node; the B4 contract) and 73 (the two
  network-bound tests) — closing conditions this phase addresses; rows 85-87 are read-only
  context (Deferred Ideas).
- `.planning/todos/pending/2026-10-13-review-content-data-orchestration-base-items.md` — the
  review's problem statement, options and owner position (D-03).

### Governing decisions and frozen records
- `.planning/phases/54-first-run-usability/54-CONTEXT.md` D-00a..D-00f, D-11 (gate shape),
  D-18 (API-surface snapshot handling) — carried forward as D-00a..e and the D-01 shape.
- `.planning/phases/53-tenant-isolation-closure/53-CONTEXT.md` D-02 (reshape to additive first),
  D-12 (`assert_treasurer_enforced`, the pure boot check D-07 sits beside), D-14 (the boot scan
  pattern).
- `.planning/phases/53-tenant-isolation-closure/53-VERSION-DECISION.md` — the patch decision and
  the twelve-line semver evidence shape D-13 repeats.
- `.planning/phases/52-ci-release-pipeline-repair/52-CONTEXT.md` D-01, D-03, D-06 — the
  patch-level gate, the §11.2 register, the gate-script CI wiring.
- `MIGRATION.md` §11.1 (behavioural entries for D-02, D-04), §11.2 (stays header-only); §9/§10
  frozen.
- `.cargo/semver-checks-allowlist.toml` — stays at zero entries.
- `.planning/decisions/0033-cargo-doc-warning-bar.md` — the two-bar rule and the 2026-09-17
  amendment table of the eight allows (D-11, D-12); amended in place by this phase.
- `.planning/decisions/0057-mid-run-halt-contract.md` — the halt contract the new reason joins
  (D-04).
- `.planning/decisions/0060-base-entity-rename.md` (g) — the five retained base items and the
  bounded identifier exception (D-03).
- `.planning/decisions/0061-per-model-parameter-support-policy.md` — seeded only when measured
  end to end (D-06).
- `.planning/decisions/PROMOTION.md` — next free ADR number 0064; no new ADR expected.
- `.github/instructions/security.instructions.md` — the manual credential review that governs
  every new log line and error message (D-02, D-04, D-07).

### v0.11.0 phase evidence
- `.planning/milestones/v0.11.0-phases/50.1-consumer-surface-readiness/50.1-SEED-PROBE.md`
  `### Probes: claude-opus-4-8` and listing row 852 (D-06).
- `.planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/51-EFFORT-PROBE.md`
  `### claude-opus-4-8` under `## Anthropic` (D-06).
- `.planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/deferred-items.md` — IN-01
  (the provider pair) and the lint leftovers E4 (D-10).
- `.planning/milestones/v0.11.0-phases/50-provider-parity-config-driven-model-swap/deferred-items.md` 50-15/50-16 — the single-feature
  clippy warnings E3 (D-10).
- `.planning/milestones/v0.11.0-phases/42-mid-run-halt-sse-terminal-status/deferred-items.md` —
  42-06 (`RunStatusValue::Cancelled`, closed by pointer under D-05) and the wave-6 note on the
  cancel test (D-09).
- `.planning/milestones/v0.10.0-phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-12-closing-measurement.txt`
  — the per-allow diagnostic measurement D-11 re-measures.
- `.planning/milestones/v0.11.0-MILESTONE-AUDIT.md` — G3 (Failed-vs-Halted) and G7 (the seed
  row), the tech-debt line for D10.

### Code and files this phase edits
- `crates/paladin-storage/src/sqlite_content_repository.rs` (~650-694) and
  `mysql_content_repository.rs` (~627-671) — the twenty `ContentListRepository` bodies (D-02).
- `crates/paladin-content/src/services/content_filtering_service.rs` (deleted) and
  `crates/paladin-content/src/services/mod.rs` lines 6 and 9 (D-02).
- `crates/paladin-ports/src/output/repository_port.rs` — `RepositoryError`, read-only (exhaustive).
- `scripts/check-no-stub-panics.sh` (new), `tests/scripts/check-no-stub-panics_test.sh` and
  fixtures (new), `Makefile` (`check-gates`), `.github/workflows/ci.yml` gate-scripts job (D-01).
  Siblings to copy: `scripts/check-crate-names.sh`, `scripts/check-version-strings.sh`,
  `tests/entity_rename_guard.rs` (comment/string blanking).
- `crates/paladin-core/src/base/entity/field.rs`, `base/entity/collection.rs`,
  `base/component/element.rs`, `base/service/field_version_service.rs`,
  `base/service/collection_versioning_service.rs`, `base/mod.rs` (D-03).
- `src/application/services/paladin/planning_service.rs` (~417) and
  `crates/paladin-core/src/platform/container/planning.rs` (`Subtask.expected_output`) (D-03).
- `crates/paladin-content/src/adapters/input/file_content_fetcher.rs` (~105-182) (D-03).
- `crates/paladin-core/src/platform/container/allowance.rs` — `HaltReason`, `as_str`,
  `herald_line` (D-04); `trace.rs` `RunFinishStatus`, read-only.
- `src/application/services/run/worker.rs` — the `Unpriced` arm (~1691), `derive_dispatch_budget`
  (~1873), `persist_agent_halt` (D-04).
- `crates/paladin-web/openapi.json` — regenerated by `make openapi` only if the halt-reason set is
  in it (D-04).
- `crates/paladin-eval/src/scenario.rs` (`RunStatusValue` ~569, `Assertion` `RunStatus` arm) and
  `src/assertion.rs` `run_status_matches` — rustdoc notes only (D-05).
- `crates/paladin-llm/src/registry/seeded.rs` `fn claude_opus_4_8` and `registry/mod.rs` tests
  near 1713/1734 (D-06).
- `src/infrastructure/web/run_api_wiring.rs` — `build_run_api_with_cadence` (~787-1040: the price
  table at ~974, `paladin_port_from_settings_with_cadence` at ~890, `assert_treasurer_enforced` at
  ~1035, the D-14 schedule scan) and its test module (~2950-3100) (D-07, D-08);
  `crates/paladin-web/src/agent_registry.rs` `AgentRegistry` (D-07); `src/config/agents.rs`
  `AgentDefinition.model`; `src/config/treasurer.rs` `AllowanceConfig`, `price_table()`;
  `crates/paladin-llm/src/pricing.rs` `PRICING_LOG_TARGET`, `UnpricedModelWarnings` (read-only
  unless shared).
- `src/application/services/run/cancel.rs` `LocalRunTokens` and
  `src/application/services/run/cancel_tests.rs` `local_cancel_signals_token` (~312-375) (D-09).
- `crates/paladin-llm/src/reasoning.rs` (`SELF_HOSTED_PROVIDERS` ~257, redundant links ~11/34),
  `crates/paladin-llm/src/config/layers.rs` (`REASONING_WIRE_PROVIDERS` ~342),
  `crates/paladin-llm/src/conformance.rs` (`thinking_effort_wire_value` ~280),
  `crates/paladin-llm/src/provider_factory.rs` (~1935), `crates/paladin-llm/src/cadence.rs`,
  `crates/paladin-llm/src/fallback.rs` `test_support`, `crates/paladin-battalion/src/engine/cache_key.rs`
  (D-10).
- `crates/paladin-battalion/src/commander.rs` tests ~3805-3960 and
  `tests/integration/commander_error_paths_test.rs` (read-only target) (D-10).
- `crates/paladin-ports/src/lib.rs:51-52`, `crates/paladin-llm/src/lib.rs:49`,
  `crates/paladin-storage/src/lib.rs:20`, `crates/paladin-notifications/src/lib.rs:14`,
  `src/lib.rs:117-119`, and every doc comment the two bars then flag (D-11);
  `docs/src/appendix/doc-coverage-report.md` (D-12).
- `docs/src/architecture/treasurer.md`, `docs/src/api-reference/platform-api.md` (halt reasons;
  the boot warning), `docs/src/user-guides/eval-harness.md` (D-05 note).
- `CHANGELOG.md` and the crate changelogs' `[Unreleased]` anchors (`paladin-storage`,
  `paladin-content`, `paladin-ai-core`, `paladin-llm`, `paladin-web`, the facade);
  `MIGRATION.md` §11.1; `.project/current-exports.txt` via `make api-surface-update`.
- `.planning/WINDOWS.md` rows 67 and 73; `.planning/REQUIREMENTS.md` FIXES-04 (D-05).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `scripts/check-crate-names.sh` and `scripts/check-version-strings.sh`: offline `tomllib`
  manifest parsing, set-and-report style, named failures, per-finding `file:line` lines — the
  template for D-01. `tests/scripts/*_test.sh` with `tests/scripts/fixtures/` is the harness
  `make test-shell-guards` auto-discovers and CI runs.
- `tests/entity_rename_guard.rs`: the std-only repository walk that blanks comments, string and
  character literals before matching, and skips build output — the technique D-01's scanner
  reuses in python.
- `assert_treasurer_enforced` (`run_api_wiring.rs`, Phase 53 D-12): a pure boot check with a table
  test and one integration proof — the shape for D-07's warning function; the D-14 `NULL
  created_by` scan is the model for a boot-time advisory that never blocks boot.
- `UnpricedModelWarnings` + `PRICING_LOG_TARGET` (`paladin-llm/src/pricing.rs`): the settle-time
  warn-once set and the log target D-07 emits under.
- `HaltReason::AllowanceExhausted` / `LedgerUnavailable`: serde tag, `as_str`, `herald_line` and
  their tests — D-04 adds a third arm to each.
- `FailingExecutor` (`run_api_wiring.rs` tests), `UnusedPaladinPort` and the `engine_factory`
  helper (`cancel_tests.rs`), `worker_tests.rs` doubles: the test-local port doubles D-08's
  scripted port follows.
- `tests/integration/commander_error_paths_test.rs` and `tests/helpers/FaultyPaladinPort`: the
  relocation D-10 points at.
- Phase 36's `RD-nn` doc-link fixes and WINDOWS row 56's `cost.rs` fix: the fully-qualified-path
  pattern D-11 applies 309 times.
- `openapi::tests::openapi_matches_committed_baseline` and `make openapi`: the drift guard D-04
  may need to refresh.

### Established Patterns
- Gates are offline, take no argument, finish in about a second, report every finding before
  exiting, treat "scanned nothing" as a failure, and are checks, not fixers; each is a Makefile
  target in `check-gates` plus its own CI step and shell regression test.
- Additive-first under the patch gate: new inherent methods, new functions, new variants on
  `#[non_exhaustive]` enums, private state; an exhaustive public enum is never widened in a patch
  (Phase 53 D-02/D-11, ADR-0063's "kept until the next minor").
- Operator messages name the entry or the model and never a secret; backend text goes only to
  `log::error!`.
- Measured, not assumed (prove-then-lock); evidence files record the command and its output.
- Frozen published records are corrected forward in `[Unreleased]`; an ADR is amended in place
  with a dated note.
- Library code never panics: `Result` out, `PoisonError::into_inner` on locks (`pricing.rs`).

### Integration Points
- `build_run_api_with_cadence` is the single place the Treasurer, price table, registry,
  resolver, worker pool and boot checks meet (D-07, D-08); `src/bin/paladin-server.rs` calls
  `build_run_api` and must see no signature change.
- `RunWorkerPool::run_once` → `derive_dispatch_budget` → the three no-call outcomes → the
  one-terminal-event path (`emit_agent_run_finished`, `persist_agent_halt` / `persist_failure`)
  and the SSE terminal event (D-04).
- `make clean-code` → `make doc-check` runs both rustdoc bars locally; CI's "Check documentation"
  steps (`ci.yml` ~73-82) are the gate D-11 must pass crate by crate.
- Phase 57's register sweep reads this phase's evidence to disposition rows 67/73 if handed over;
  Phase 58's pre-tag audit re-runs `check-semver.sh` and reads FIXES-04's re-home note.

</code_context>

<specifics>
## Specific Ideas

- The storage refusal reads like its neighbours' errors and is greppable by method name, e.g.
  `ContentListRepository::save is unfinished for the mysql backend: content lists are not
  persisted`; the wording is the planner's, the shape (method, backend, no key material) is not.
- The guard's red run against the real tree, before the fixes, is itself evidence: 26 lines,
  three files — the same numbers the evidence file carries after the fixes read zero.
- The boot warning is one line per model an operator can grep once, under `paladin::pricing`,
  naming the action (add a `treasurer.pricing` row) and the consequence (agent-kind refusal or
  halt), and stating that workflow assistants are not refused and graph-node models are not
  enumerated.
- The FIXES-04 re-home note says plainly why: two exhaustive public enums, a major-level lint at
  patch, a scope guard that forbids a non-security break — and that the next minor's change is
  one variant plus a schema snapshot refresh.
- `doc-coverage-report.md` after D-12 is the page a Phase 58 reviewer reads to see the bar green
  at a named SHA, not a table that goes stale.

</specifics>

<deferred>
## Deferred Ideas

- **WINDOWS rows 85-87** (53-REVIEW WR-01..03: `UnknownSchemaVersion` aborting the thread walk, the
  `limit + 1` decode locking an owner out, ownership transfer on an Admin fork or unattributed
  run) carry "proposed owner Phase 55" but are not in FIXES-01..06; the roadmap scope is fixed, so
  they are not folded. They are code fixes, so a `56.x` decimal phase (or `/gsd-phase edit 55` by
  the maintainer) fits better than Phase 57's register-only sweep; the `v0.11.1` exit criterion
  needs them fixed or waived either way.
- **News API key travels in the URL** (`D-52-03-01`, `news_api_fetcher.rs`: header-carried key,
  URL stripped from the mapped error, `Policy::none()`) — a correctness candidate for the same
  decimal phase.
- **Refuse `${…}` in the treasurer webhook `secret`** (Phase 54 WR-04) — same trap class as
  FIRST-01, new behaviour; same home.
- **`RunStatusValue::Cancelled` and the scenario schema refresh** — the next minor (D-05).
- **Retire or partially retire the base-layer items** (todo options B/C, §12.2 rows, allowlist
  entries, a guard rule) and the entity-sense identifier renames — the next minor, on the review
  file's findings (D-03). **Finishing** the content / general-data system (option A) is a feature
  milestone.
- **Pricing-coverage check at graph registration** (WINDOWS row 67's original closing condition)
  and enumerating workflow assistants' node models for the boot warning — needs a listing method
  on `AssistantResolver`; a later minor.
- **Sharing the warn-once set between boot and settle time** through an additive `pub fn` in
  `paladin-llm` — only if the planner finds it free (D-07).
- **An `#[ignore]` inventory in the testing guide** (E8) — Phase 56 dogfooding of the testing
  guide, or a docs pass.
- **The feature-stub TODOs** in `trigger.rs` (JSONPath payload matching, cooldown),
  `service_runner.rs` (notification adapter wrappers) and `council_service.rs` (registry
  parameter) — E9, out: feature work, none a reachable panic.
- **The all-features CI test job** (D10's second half) — out by the milestone-open decision.
- **Refreshing `.planning/codebase/CONCERNS.md` and `TESTING.md`** (dated 2026-07-30, stale on
  the commander tests) — `/gsd-map-codebase`.

### Reviewed Todos (not folded)
`[auto] todo.match-phase returned the same four matches at score 0.6 as Phases 52-54. The
auto-fold rule (score ≥ 0.4) was applied to the one match that is this phase's own requirement
(G3 → FIXES-02) and overridden on the Phase 53/54 precedent for the other three, whose matches
are keyword-only and whose folding would widen the phase against the milestone scope guard.`
- `2026-08-13-verify-local-coverage-reproduction.md`: assigned to Phase 56
  (`resolves_phase: 56`, DOGF-05).
- `2026-10-14-web3sec-single-bump-to-v0-11-0.md`: the downstream consumer's own step; its return
  is Phase 56's DOGF-02 input.
- `2026-10-20-openai-responses-api-for-pro-tier.md`: ruled out at milestone open (PROV-05, next
  feature milestone).

</deferred>

---

*Phase: 55-correctness-fixes-test-hygiene-lint-bar*
*Context gathered: 2026-11-01*
