# Requirements: Paladin — Milestone v0.11.1 "Dogfood Cleanup"

**Defined:** 2026-10-27
**Core Value:** A Rust developer can compose and run multi-agent workflows against any supported LLM
provider through stable port abstractions — without their own domain code depending on a provider,
transport, or storage implementation.

**Input of record:** `.project/v0.11.1/` — `01-carried-gaps-inventory.md` (rows A1…G4),
`02-dogfood-findings-log.md` (entries DF-nn), `03-suggestions-and-scoping.md` (themes, decisions, exit
criteria). Each requirement below names its inventory row so the roadmap, the fix commit and the
inventory stay cross-referenced. Every row cited was re-verified against the tree on 2026-10-27 at the
milestone open unless marked otherwise; one inventory item (F1, nine phases NOT-VALIDATED) was found
already closed — all fifteen Phase 38–51 `VALIDATION.md` files read `status: validated` — and is not a
requirement.

**Decisions at open (2026-10-27):** OpenAI Responses API out; `${VAR}` auth keys refused at boot (no
expansion path); research skipped; smaller decisions per the outline (see PROJECT.md *Current
Milestone*). Patch bump; becomes `v0.12.0` only if a security fix forces a persisted or public-surface
break.

## v1 Requirements

Requirements for this milestone. Each maps to exactly one roadmap phase.

### CI and Release Repair (CIREL)

- [x] **CIREL-01**: The CI semver job checks every publishable crate (now including `paladin-eval`)
      against the published `0.11.0` baseline, is named for that baseline, and reports a non-zero
      check count (today: `Semver Checks (vs v0.9.0)`, `0 checks: 0 pass, 254 skip`) — inventory D1

- [ ] **CIREL-02**: The crate-wide semver allows in `crates/paladin-core/Cargo.toml` and
      `crates/paladin-battalion/Cargo.toml` (`struct_missing`, `enum_missing`, `module_missing`,
      `enum_variant_missing`, `function_missing`) and their mirrored
      `.cargo/semver-checks-allowlist.toml` entries are removed, and the semver job stays green — D2

- [x] **CIREL-03**: `check-migration-allowlist` runs in `ci.yml` on every PR, not only under
      `make check-gates` — D3

- [ ] **CIREL-04**: `End-to-End Tests` and `Publish Dry Run` run before merge (on PRs or on a
      `release` label), not only on `main` pushes — D3

- [x] **CIREL-05**: `scripts/create-or-reuse-release.sh` builds the release body without passing it
      through argv (`jq --rawfile` or stdin), with a regression test whose body exceeds 131,072
      bytes, so no manual changelog condensation is ever needed again — D4

- [x] **CIREL-06**: The 24 advisory CodeQL alerts on the release PR heads are triaged into
      `CODEQL-DISMISSALS.md` (each dismissed with a reason or fixed); the advisory-only posture is
      unchanged — D7

- [x] **CIREL-07**: GitHub issue `DF3NDR/paladin-dev-env#41` is closed with a reference to the
      `v0.11.0` GitHub Release created by `release.yml`, or restated with what is still missing — D6

- [ ] **CIREL-08**: The eight crate-level `#![allow(rustdoc::…)]` lines (`paladin-ports` ×2,
      `paladin-llm`, `paladin-storage`, `paladin-notifications`, root `src/lib.rs` ×3) are removed
      with `cargo doc` warning-free, and `doc-coverage-report.md` is regenerated — D10 (the
      all-features CI test job stays out by decision)

- [ ] **CIREL-09**: The publish-order gate reading (`ci.yml` `Check publish order` plus the
      dependency-ordered publish job, no separate `release.yml` step) is recorded as accepted in the
      release documentation — D5

### Tenant Isolation (TENANT, continuing from v0.11.0's TENANT-01…02)

*Roadmap-time finding (2026-10-27): Phase 40's `40-REVIEW-FIX.md` records WR-01…WR-04 as fixed
(`ensure_thread_visible` in `submission.rs`, service-layer tenant check on cancel, fixed run-store
error text), which the inventory built from `40-REVIEW.md` alone did not see. TENANT-04, -05 and -06
are therefore prove-against-the-tree-and-add-the-missing-regression-test requirements; only what
fails is rebuilt.*

- [ ] **TENANT-03**: Every `/v1/threads/*` handler (list, get, state, history, resume, fork, delete)
      resolves the thread through one visibility gate (as `run_controller::load_visible_run` does), so
      a principal of another tenant gets 404 and cannot enumerate or read another tenant's threads or
      Waypoint state — A1, WINDOWS row 58

- [ ] **TENANT-04**: `POST /v1/runs {thread_id}` and `POST /v1/threads/{id}/fork` refuse a thread
      owned by another tenant (hidden == missing), so a user-role principal cannot run against or
      mutate it — A2, 40-REVIEW WR-01

- [ ] **TENANT-05**: `RunSubmissionService::cancel` enforces the tenant at the service layer (not
      only in the HTTP controller), so library and future-route callers cannot bypass it — A3, WR-02

- [ ] **TENANT-06**: A corrupt or half-attributed run row, and any `Serialization` error, surfaces
      to an API client as a redacted error that preserves the hidden-equals-missing guarantee; no
      backend text reaches the response body — A4, WR-03/WR-04

- [ ] **TENANT-07**: `GET /agents/{id}/jobs/{job_id}` checks the principal and tenant before
      returning a job — A5, IN-04

- [ ] **TENANT-08**: When a Treasurer is configured, server boot asserts that the worker pool was
      built through `with_engine_factory` (settlement, spend guard and cancel probe attached) and
      refuses to start otherwise, with a test that proves the refusal — A8, audit G2

- [ ] **TENANT-09**: Reserved literals are rejected as tenant and API-key names at config load — A6,
      IN-01

- [ ] **TENANT-10**: Server boot logs one `warn!` listing schedules with `NULL created_by`
      (pre-Phase-41, firing unattributed and ungated); no backfill route — A7, WINDOWS row 63

### First-Run Usability (FIRST)

- [ ] **FIRST-01**: An `http.auth.api_keys[].key` whose value is an unexpanded `${…}` placeholder is
      refused at config load with an error naming the key; `config.example.yml` and
      `k8s/server/configmap.yaml` (including its "expanded by the server at load time" header) no
      longer imply expansion — B1, C3, WINDOWS row 62, DF-01

- [ ] **FIRST-02**: Every install snippet and version line a user copies from reads the current
      release: `README.md:108` (`"0.5"`), `README.md:183` ("Current version: 0.8.0"),
      `docs/src/appendix/sentinel.md:66`, `crate-map.md:190`, `feature-flags.md:132,167,333`
      (`"0.10.0"`); historical rows inside the migration guide are exempt — C1, C2, DF-02

- [ ] **FIRST-03**: A `check-version-strings` guard in `make check-gates` and CI fails when any
      user-facing version string (README, `docs/src`, `k8s/`, `openapi.json`) lags the workspace
      version, so a Phase-46-style currency sweep cannot miss it again — C1 gate

- [ ] **FIRST-04**: `k8s/server/worker-deployment.yaml` (`v0.10.0`) and `k8s/server/deployment.yaml`
      (`v0.6.0`) `version:` labels read the current release and are covered by FIRST-03's guard — C4

- [ ] **FIRST-05**: The `[0.11.1]` entries of `crates/paladin-memory/CHANGELOG.md` and
      `crates/paladin-content/CHANGELOG.md` correct the `[0.11.0]` "No functional change" wording
      with the one-way persisted-key change (`paladin.node` → `paladin.data`, MIGRATION 10.1
      M-B-12) — C5, 47-REVIEW WR-01

- [ ] **FIRST-06**: The `src/application/services/treasurer/tests.rs` doc comment and ADR-0056's
      Downstream Consumers line no longer say Phase 42 closes the over-admission race with a
      reservation; both cite ADR-0057 (check-only) — C6

- [ ] **FIRST-07**: Stale counts and versions are corrected: `release.toml:41,47` and
      `scripts/finalize-crate-changelogs.sh:8` ("ten" crates), `ci.yml` count comment,
      `release.toml:49` (`cargo-release 1.1.2` → the version actually used), and the dangling
      `[Unreleased]: …/compare/v0.2.0...HEAD` link at `CHANGELOG.md:3396` — C7, C8, 47-REVIEW WR-02

- [ ] **FIRST-08**: `.project/current-exports.txt` is regenerated with `make api-surface` on the
      pinned nightly and committed, carrying the `UnsupportedParameter` / `check_reasoning` rows — C10

- [ ] **FIRST-09**: The node-cache guide states that the server attaches no node cache (the PACE-04
      stampede lock is a library builder only) and documents the cache-key version and the
      `key_prefix` rotation for a provider-level default effort change — B10, B12

### Correctness Fixes (FIXES)

- [ ] **FIXES-01**: No `todo!()` / `unimplemented!()` is reachable outside `#[cfg(test)]` in any
      published crate, held by a grep guard in `make check-gates`. Measured at roadmap time: the
      twenty reachable sites are the `impl ContentListRepository` bodies for `SqliteStore` and
      `MySqlContentRepository` in `paladin-storage` (`sqlite_content_repository.rs`,
      `mysql_content_repository.rs`; both return `Result<_, RepositoryError>`, so a typed `Err` is
      body-only); the `content_filtering_service.rs` file the inventory names is commented out of
      `services/mod.rs` and not compiled — delete it or keep it fenced, it is not the reachable panic
      — B2, DF-03 (target corrected 2026-10-27 at roadmap time)

- [ ] **FIXES-02**: The unfinished content / general-data base-layer items (`Field`, `Collection`,
      `element`, `FieldVersionService`, `CollectionVersioningService`) and the content-processing
      stubs (`planning_service.rs:417` placeholder output, `file_content_fetcher.rs` ingestion
      TODOs) are reviewed with a recorded keep / fence / remove decision each; anything kept and
      unfinished refuses cleanly and says so in its rustdoc — G3, E9, todo 2026-10-13

- [ ] **FIXES-03**: A `DispatchBudget::Unpriced` outcome on an agent-kind run records `Halted`, not
      `Failed` (or, if the mapping is not a local change, the behavior is documented in the Treasurer
      guide with the reason) — B3, audit G3

- [ ] **FIXES-04**: `paladin-eval`'s `RunStatusValue` gains `Cancelled` so a scenario can assert
      `run_status: cancelled`; the schema snapshot is refreshed — B5, 42-06

- [ ] **FIXES-05**: The `claude-opus-4-8` seeded registry row is verified against the 50.1 probe
      record and its `reasoning` flag corrected if wrong — B6, audit G7

- [ ] **FIXES-06**: Server boot (or admission) warns once per model that is reachable by a metered
      principal but has no `treasurer.pricing` row; workflow assistants are not refused — B4,
      WINDOWS row 67

### Tests and Lints (TESTS)

- [ ] **TESTS-01**: `run_api_wiring::tests::build_run_api_persists_*` (2 tests) run hermetically
      against a scripted port with no network, pass in a sandbox, and WINDOWS row 73 is
      dispositioned — E1

- [ ] **TESTS-02**: `cancel_tests::local_cancel_signals_token` waits on a dispatch-registration
      signal instead of a fixed 50 ms sleep — E2

- [ ] **TESTS-03**: Single-feature `cargo clippy --tests` on `paladin-llm` is warning-free (unused
      `chat`, `cadence::PinnedSpread` / `pin_waiter_spread`, `fallback::test_support::ProfiledStubPort`)
      — E3

- [ ] **TESTS-04**: The Phase 51 lint leftovers are cleared: the `NODE_CACHE_KEY_VERSION` rustdoc
      link in `engine/cache_key.rs`, redundant explicit links in `reasoning.rs`, dead
      `thinking_effort_wire_value` and the unused `provider_factory.rs` import under default
      features — E4

- [ ] **TESTS-05**: The self-hosted provider pair (`ollama`, `openai-compatible`) is defined once;
      the suspected duplicate in the config layer is found and folded in, or 51 IN-01 is closed as
      not-reproducing — E5

- [ ] **TESTS-06**: The `commander.rs:3834+` "TODO: Enhance MockPaladinPort" test stubs are deleted
      or point at `tests/integration/commander_error_paths_test.rs` — E7

### Dogfood Findings (DOGF)

- [ ] **DOGF-01**: Every `DF-nn` entry in `02-dogfood-findings-log.md` carries a disposition
      (`in v0.11.1`, `out` with a reason, or `duplicate of`) before the tag; none is `open`

- [ ] **DOGF-02**: The web3sec consumer's return against `v0.11.0` (`47-HANDOFF-web3sec.md` §6:
      commit SHA, D-2 choice, gate totals, divergences) is triaged into the findings log, one entry
      per divergence — G1, todo 2026-10-14

- [ ] **DOGF-03**: Each section of `config.example.yml` round-trips through the loader with every
      optional key present, as a test, and any key the loader rejects or silently ignores is logged as
      a finding — §6 "config loading in general"

- [ ] **DOGF-04**: The first-run path is walked on the published `0.11.0` crates — README Quick
      Example → `cargo add` → `basic_paladin`, then `paladin-cli onboarding` and `setup-check` against
      one real provider key — and every stumble is logged as a finding — §"Surfaces worth exercising"

- [ ] **DOGF-05**: The maintainer walks `docs/src/contributing/testing-guide.md`'s local
      `make coverage` procedure on a Docker-capable machine and the todo
      `2026-08-13-verify-local-coverage-reproduction` is closed with the measured figure — G4

- [ ] **DOGF-06**: Every finding promoted to `in v0.11.1` with severity S1 or S2 is fixed before the
      tag, in a phase inserted once the findings are known

### Register Hygiene (REG)

- [ ] **REG-01**: The Phase 45 and 51 SUMMARY `requirements-completed` frontmatter carries
      STORE-02 and REASON-02/03/05/06 — F2

- [ ] **REG-02**: WINDOWS rows 56 (already fixed in tree), 68, 69 and 79 are dispositioned with their
      superseding evidence, and `WINDOWS.md` reads `open_count: 0` at the tag — F3, E6, exit criterion

- [ ] **REG-03**: The three `diagnosed` sessions in `.planning/debug/` (kimi fallback profile,
      openai max_completion_tokens, vendor token usage fidelity) are archived as resolved — F4

- [ ] **REG-04**: The Phase 40 judgment-tier prohibitions that carry LLM-judge verdicts get a bounded
      human read recorded in the phase's register; the Phase 42 subset is accepted as-is — F5

- [ ] **REG-05**: Phase 40 review warnings WR-01…04 and IN-04 are dispositioned (closed by
      TENANT-04…07 or waived with a reason), leaving zero undispositioned review warnings across
      Phases 38–51 — exit criterion

- [ ] **REG-06**: The three `zip 3.0.0` RustSec exceptions in `SECURITY-EXCEPTIONS.md` are
      re-checked against the current `utoipa-swagger-ui` release and either lifted or re-dated with
      the check recorded — D8

### Release (SHIP, continuing from v0.10.0's SHIP-01…07)

- [ ] **SHIP-08**: `v0.11.1` is tagged on `main` and ships through `release.yml` end to end with no
      manual step; all 12 publishable crates are on crates.io at `0.11.1`

- [ ] **SHIP-09**: `MIGRATION.md` states that a `0.11.0` consumer takes `0.11.1` without adaptation,
      or records the one deliberate break (in which case the version is `v0.12.0`, not a quietly
      breaking patch)

- [ ] **SHIP-10**: The milestone audit reads `passed`, not `tech_debt`: every carried item either
      fixed, waived with a reason, or re-homed with an owner and a date

## v2 Requirements

Deferred. Tracked, not in this roadmap.

### Provider parity and measurement

- **PROV-05**: OpenAI Responses API wire so the pro tier (`gpt-5.5-pro` and siblings) is usable —
  G2, todo 2026-10-20; the first item of the next feature milestone

- **PROV-06**: Anthropic `Budget(n)` refusal message names the config key an operator must set — B7
  (promote into v0.11.1 only if a dogfood run trips on it)

- **PROV-07**: Ollama and OpenAI-compatible probed and seeded; quota-body mapping verified for Kimi,
  Qwen, Grok, Ollama, OpenAI-compatible; retry counters unified — C9 (needs live keys and a probe plan)

- **PROV-08**: Grok vendor `cost_in_usd_ticks` / `num_sources_used` parsed and compared with the
  Treasurer price under the ADR-0061 visibility rule — B9

### Runtime

- **TENANT-11**: Backfill or `PATCH` re-assignment route for pre-Phase-41 unattributed schedules — A7
  (only if a dogfood deployment actually has such rows)

- **PACE-06**: A guard that warns when a `FallbackLlmAdapter` chain is built without Cadence — B11
- **PACE-07**: A node cache wired into the server composition root — B10 (feature work)

### CI and docs

- **CIREL-10**: `cargo test --workspace --all-features` job in CI — D10 (CI-cost call; the per-feature
  matrix still finds single-feature leftovers)

- **DOCS-09**: An `#[ignore]` inventory in the testing guide (110 sites: CLI live tests, scheduler,
  file storage, Qdrant, MySQL, live MCP) so a dogfooder knows what `cargo test` does not cover — E8

## Out of Scope

| Item | Reason |
|---|---|
| Finishing stub features (content ingestion, trigger JSONPath/cooldown, Vault policies, `paladin user` CLI) | Milestone definition: stubs are fenced, not finished; they stay on the FUT register |
| OBS-05 tracing overhead toward the ≤ 3 % bar (WINDOWS 35/61) | Waived by the maintainer 2026-09-30; not a performance milestone |
| ADR-0057 Treasurer limits: check-only boundary (row 65), streamed-call overshoot (row 66), agent-loop no-Waypoint (row 64) | Documented contracts; only documentation that contradicts them is fixed |
| Aggregate HTTP spend route (audit G1) | Feature work; per-tenant/key/model spend stays CLI + library |
| New ports, adapters or HTTP routes | Scope guard; a tenant column on threads is a migration, not a feature |
| `LlmRequest` built directly gets omit + `warn!` rather than a refusal (B8) | CHANGELOG 0.11.0 known limitation; revisit only on a dogfood report |
| `TokenUsage.total_tokens = prompt + completion` (D-02) | Owner-kept; vendor disagreement surfaces as `paladin.usage_unreconciled` |
| Archival `.project/Milestone_*` docs still saying `Node<T>` (C11) | Archival; at most a one-line errata note |
| FUT-01…FUT-13 (SDK polish, IDE, HA replication, billing, RBAC, Vault policies, LLM-judge evals, long-context pricing, multi-currency, DNS-rebinding pinning) | v2 register from v0.11.0 |
| Nyquist validation for archived Phases 05-21 | Pre-v0.10.0 archive; not a v0.11.1 exit criterion |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| CIREL-01 | Phase 52 | Complete |
| CIREL-02 | Phase 52 | Pending |
| CIREL-03 | Phase 52 | Complete |
| CIREL-04 | Phase 52 | Pending |
| CIREL-05 | Phase 52 | Complete |
| CIREL-06 | Phase 52 | Complete |
| CIREL-07 | Phase 52 | Complete |
| CIREL-08 | Phase 55 | Pending |
| CIREL-09 | Phase 52 | Pending |
| TENANT-03 | Phase 53 | Pending |
| TENANT-04 | Phase 53 | Pending |
| TENANT-05 | Phase 53 | Pending |
| TENANT-06 | Phase 53 | Pending |
| TENANT-07 | Phase 53 | Pending |
| TENANT-08 | Phase 53 | Pending |
| TENANT-09 | Phase 53 | Pending |
| TENANT-10 | Phase 53 | Pending |
| FIRST-01 | Phase 54 | Pending |
| FIRST-02 | Phase 54 | Pending |
| FIRST-03 | Phase 54 | Pending |
| FIRST-04 | Phase 54 | Pending |
| FIRST-05 | Phase 54 | Pending |
| FIRST-06 | Phase 54 | Pending |
| FIRST-07 | Phase 54 | Pending |
| FIRST-08 | Phase 54 | Pending |
| FIRST-09 | Phase 54 | Pending |
| FIXES-01 | Phase 55 | Pending |
| FIXES-02 | Phase 55 | Pending |
| FIXES-03 | Phase 55 | Pending |
| FIXES-04 | Phase 55 | Pending |
| FIXES-05 | Phase 55 | Pending |
| FIXES-06 | Phase 55 | Pending |
| TESTS-01 | Phase 55 | Pending |
| TESTS-02 | Phase 55 | Pending |
| TESTS-03 | Phase 55 | Pending |
| TESTS-04 | Phase 55 | Pending |
| TESTS-05 | Phase 55 | Pending |
| TESTS-06 | Phase 55 | Pending |
| DOGF-01 | Phase 56 | Pending |
| DOGF-02 | Phase 56 | Pending |
| DOGF-03 | Phase 56 | Pending |
| DOGF-04 | Phase 56 | Pending |
| DOGF-05 | Phase 56 | Pending |
| DOGF-06 | Phase 56 | Pending |
| REG-01 | Phase 57 | Pending |
| REG-02 | Phase 57 | Pending |
| REG-03 | Phase 57 | Pending |
| REG-04 | Phase 57 | Pending |
| REG-05 | Phase 57 | Pending |
| REG-06 | Phase 57 | Pending |
| SHIP-08 | Phase 58 | Pending |
| SHIP-09 | Phase 58 | Pending |
| SHIP-10 | Phase 58 | Pending |

**Coverage:**

- v1 requirements: 53 total
- Mapped to phases: 53
- Unmapped: 0

| Phase | Requirements | Count |
|-------|--------------|-------|
| 52. CI & Release Pipeline Repair | CIREL-01, CIREL-02, CIREL-03, CIREL-04, CIREL-05, CIREL-06, CIREL-07, CIREL-09 | 8 |
| 53. Tenant Isolation Closure | TENANT-03, TENANT-04, TENANT-05, TENANT-06, TENANT-07, TENANT-08, TENANT-09, TENANT-10 | 8 |
| 54. First-Run Usability | FIRST-01, FIRST-02, FIRST-03, FIRST-04, FIRST-05, FIRST-06, FIRST-07, FIRST-08, FIRST-09 | 9 |
| 55. Correctness Fixes, Test Hygiene & Lint Bar | CIREL-08, FIXES-01, FIXES-02, FIXES-03, FIXES-04, FIXES-05, FIXES-06, TESTS-01, TESTS-02, TESTS-03, TESTS-04, TESTS-05, TESTS-06 | 13 |
| 56. Dogfood Findings | DOGF-01, DOGF-02, DOGF-03, DOGF-04, DOGF-05, DOGF-06 | 6 |
| 57. Register Hygiene Close-Out | REG-01, REG-02, REG-03, REG-04, REG-05, REG-06 | 6 |
| 58. Crate Release | SHIP-08, SHIP-09, SHIP-10 | 3 |

Mapping note: CIREL-08 (the eight crate-level `#![allow(rustdoc::…)]` removals) sits in Phase 55 with
TESTS-03/TESTS-04, not in Phase 52 with the rest of CIREL — it shares their lint and rustdoc bar and
files, and its warning count is unbounded, so it must not delay arming the CI gates Phase 52 delivers.
DOGF-06's fixes may land in decimal phases (56.1+) inserted under Phase 56; they inherit this mapping.

---
*Requirements defined: 2026-10-27*
*Last updated: 2026-10-27 after roadmap creation (Phases 52-58; 53/53 requirements mapped)*
