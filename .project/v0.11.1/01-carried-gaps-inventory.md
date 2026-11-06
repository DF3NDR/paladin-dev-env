# v0.11.1 — Carried gaps inventory (snapshot 2026-10-27)

Every open item v0.11.0 left behind, grouped by what kind of work closes it. Each row names its **source of
record** (the file that owns the item; this inventory never becomes the record), a **severity read**
(S1 security / data-isolation · S2 wrong result or crash a user can hit · S3 misleading docs or config ·
S4 lint, test or register hygiene), and a **triage recommendation** for v0.11.1:

- **in** — belongs in a cleanup/dogfood milestone as-is
- **decide** — needs an owner decision before it is in or out (the question is stated)
- **out** — accepted contract, performance work, or feature work; stays on its register

Where I verified the item against the tree on 2026-10-27 the row says so; everything else is as recorded
by its source.

---

## A. Security and tenant isolation (all found in Phase 40, none dispositioned)

| # | Item | Source | Sev | Rec |
|---|---|---|---|---|
| A1 | `/v1/threads/*` (list, get, state, history, resume, fork, delete) are not tenant-scoped: threads carry no tenant, so an authenticated principal can enumerate another tenant's threads and read Waypoint state, although that tenant's `/runs` answer 404. Closing condition already written: record a tenant on the thread (or derive from its runs) and route every `/threads/{id}*` handler through one visibility gate like `run_controller::load_visible_run`. | WINDOWS row 58 (D-14, T-40-23); `crates/paladin-web/src/thread_controller.rs` | S1 | **in** — the row itself names "a v0.11 hygiene phase" as the owner |
| A2 | `POST /v1/runs {thread_id}` and `POST /v1/threads/{id}/fork` accept another tenant's thread: a user-role principal can run against and mutate it. | 40-REVIEW WR-01; `src/application/services/run/submission.rs:262-285, 437-466` | S1 | **in** (same fix family as A1) |
| A3 | `RunSubmissionService::cancel` feeds only the role, not the tenant, to `authorize`; tenant isolation for cancel is enforced only in the HTTP controller, so any library or future-route caller bypasses it. | 40-REVIEW WR-02; `submission.rs:333-367` | S1 | **in** |
| A4 | A corrupt or half-attributed run row becomes a 500 that leaks backend text and breaks the "hidden == missing" guarantee; new Phase 40 `Serialization` messages reach API clients through `ApiError::internal(e.to_string())`. | 40-REVIEW WR-03, WR-04; `run_controller.rs:849-865, 978-991`; `run/sqlite.rs`, `postgres.rs` `row_to_run` | S1/S2 | **in** |
| A5 | `GET /agents/{id}/jobs/{job_id}` ignores the principal and tenant. | 40-REVIEW IN-04; `agent_controller.rs:740-748` | S1 | **in** |
| A6 | Reserved literals are accepted as real tenant and key names. | 40-REVIEW IN-01; `src/config/agents.rs:170-235` | S3 | **decide** — cheap validation; is the reserved set stable enough to reject? |
| A7 | Schedules created before Phase 41 have `NULL created_by` and keep firing unattributed and ungated by any allowance. Closing condition recorded: operator re-creates them; backfill / `PATCH` re-assignment were deferred ideas. | WINDOWS row 63 (D-08); `migrations/sqlite/010_add_run_schedule_created_by.sql` | S2 | **decide** — a boot-time `warn!` listing unattributed schedules is cheap and in scope; a backfill route is feature work |
| A8 | Treasurer enforcement (ledger settlement, spend guard, cancel probe) attaches only on the `engine_factory` branch of `RunWorkerPool`; shipped wiring always uses the factory but **nothing asserts it when a treasurer is configured** — a misconfiguration drops enforcement silently. | Audit G2 (ALLOW-03/LEDGR-02); `src/application/services/run/worker.rs` (`with_engine_factory`) | S1 | **in** — one boot-time assertion plus a test |

## B. Wrong-result and crash paths a user can hit

| # | Item | Source | Sev | Rec |
|---|---|---|---|---|
| B1 | **Config loader performs no `${VAR}` expansion** for `http.auth.api_keys[].key` (pinned by `treasurer.rs::yaml_env_placeholder_is_not_expanded`), yet `config.example.yml` and `k8s/server/configmap.yaml` (whose header says "expanded by the server at load time") show `key: "${PALADIN_API_KEY_CI}"`. A copied example loads the literal placeholder as the API key. (The LLM `api_key: "${OPENAI_API_KEY}"` indirection is a separate, working mechanism; the trap is specific to HTTP auth keys.) Verified 2026-10-27: both examples still carry the placeholders. | WINDOWS row 62 (Phase 41) | S2/S3 | **in** — decide direction: implement expansion for auth keys, or reject an unexpanded `${…}` at boot with a named error and fix the examples. Rejecting is the smaller change. |
| B2 | `ContentFilter::filter_content_list` (library code, not a test) is built from six `todo!()` fields and panics on any call. | Verified 2026-10-27: `crates/paladin-content/src/services/content_filtering_service.rs:21-26` | S2 | **in** — remove or make it return an error; do not finish the feature |
| B3 | `DispatchBudget::Unpriced` on an agent-kind run records `Failed`, not `Halted` (post-admission price-row-loss edge). | Audit G3 | S2 | **in** if the fix is a status-mapping one-liner; otherwise document |
| B4 | A graph Paladin node on a model with no `treasurer.pricing` row settles nothing, so no allowance bounds it; workflow assistants are **not** refused at admission for an unpriced model while agent routes are (422 `model_unpriced`). Inconsistent surface. | WINDOWS row 67 (G15, D-00c); `engine/settlement.rs` | S2 | **decide** — a boot-time or admission-time warning naming unpriced reachable models is in scope; refusing workflow assistants changes an accepted contract |
| B5 | `paladin-eval`'s `RunStatusValue` has no `Cancelled` value, so an eval scenario cannot assert `run_status: cancelled` although the engine reports it. Verified 2026-10-27: still four variants. | 42 deferred-items (42-06); `crates/paladin-eval/src/scenario.rs:571` | S2 | **in** — additive variant plus schema snapshot refresh Re-homed 2026-11-01 (Phase 55, D-05): exhaustive enums at the patch gate; owner the next minor; 42-06 closed by this pointer; see REQUIREMENTS.md v2 Eval harness. |
| B6 | `claude-opus-4-8` seeded `reasoning=false` while carrying effort levels; affects only the Commissary/direct-config 300 s reasoning-timeout default. Not re-verified by the audit. | Audit G7 (PARITY-04/REASON-04); `crates/paladin-llm/src/registry/seeded.rs:98` | S2 | **in** — verify against the 50.1 probe record, fix the seed row |
| B7 | REASON-06: Anthropic `Budget(n)` refuses on **every** seeded Anthropic model until the operator declares `llm.anthropic.models.<id>.effort.budget`; Gemini 2.5 ids stay `Unmeasured` and refuse until overridden. Owner-accepted 2026-10-23. | Audit tech_debt Phase 51 | S3 | **decide** — accepted policy; but if dogfooding trips on it, the usability fix is a better refusal message that names the config key, not a seeded span |
| B8 | A caller that builds an `LlmRequest` directly gets omit + `warn!` + metadata rather than a refusal for an unsupported parameter. | CHANGELOG 0.11.0 Known limitations (3) | S3 | **out** unless a dogfood report shows it hid a real misconfiguration |
| B9 | Grok's vendor-computed `cost_in_usd_ticks` / `num_sources_used` are not parsed or compared with the Treasurer price (owner-accepted 2026-10-17). | WINDOWS row 77; `crates/paladin-llm/src/grok/adapter.rs` | S3 | **decide** — parse-and-compare under the ADR-0061 visibility rule is small; the owner deferred it once already |
| B10 | PACE-04 stampede lock is a tested library builder only; **no shipped composition root attaches it** (the server attaches no node cache). | Audit tech_debt Phase 43 | S3 | **decide** — is "server has no node cache" by design? If yes, document it on the node-cache guide; if no, it is a wiring gap |
| B11 | Cadence outage blind spot: during a Redis outage a worker cannot see other workers' 429s; hand-built `FallbackLlmAdapter` chains that skip `with_cadence` are unpaced. | ADR-0058 A1-A3 | S3 | **out** (documented), unless a guard that warns when a fallback chain is built without Cadence is cheap |
| B12 | Provider-level default reasoning effort is not in the node-cache key; operators rotate `key_prefix` (documented). Companion: cache key `unwrap_or_default()` on a serde string. | 51 deferred-items WR-04, IN-02; `engine/cache_key.rs` | S3 | **decide** together — both need the same cache-version decision |

## C. Docs, config examples and version strings (first-run usability)

| # | Item | Source | Sev | Rec |
|---|---|---|---|---|
| C1 | `README.md:183` says **"Current version: 0.8.0"**; `README.md:108` install snippet pins `paladin-ai = "0.5"`. Verified 2026-10-27. | tree | S3 | **in** — and add a gate (a `check-version-strings` script in `make check-gates`) so Phase 46-style currency sweeps cannot miss it again |
| C2 | `docs/src/api-reference/crate-map.md:190` and `feature-flags.md:132,167,333` pin `paladin-ai = "0.10.0"` (a yanked-partial release); `docs/src/appendix/sentinel.md:66` pins `"0.5"`. | tree | S3 | **in** (same sweep as C1) |
| C3 | `k8s/server/configmap.yaml` header claims `${ENV_VAR}` is "expanded by the server at load time" — see B1. | WINDOWS row 62 | S3 | **in** (with B1) |
| C4 | `k8s/server/worker-deployment.yaml:20,35` labels `version: v0.10.0`. | 47-CI-EVIDENCE D-00d | S3 | **in** — consider templating the label from the crate version |
| C5 | `paladin-memory` and `paladin-content` `CHANGELOG.md` `[0.11.0]` say "No functional change" while root CHANGELOG / MIGRATION 10.1 M-B-12 document the one-way persisted-key change (`paladin.node` → `paladin.data`). The published crates are immutable; the correction goes in the **0.11.1** entry of both crate changelogs. | 47-REVIEW WR-01 | S3 | **in** |
| C6 | `src/application/services/treasurer/tests.rs` doc comment and ADR-0056's Downstream Consumers line still say Phase 42 closes the over-admission race with a reservation; ADR-0057 supersedes (check-only). | 42 deferred-items (42-02) | S3 | **in** (doc-only) |
| C7 | Stale count comments: `release.toml:41`, `scripts/finalize-crate-changelogs.sh:8`, `.github/workflows/ci.yml:277` say "ten" crates (there are 11 crate changelogs, 12 publishable crates); `release.toml:49` records `cargo-release 1.1.2` where 1.1.7 was used. | 47-CI-EVIDENCE D-00d | S4 | **in** (trivial) |
| C8 | Dangling `[Unreleased]: …/compare/v0.2.0...HEAD` link definition at `CHANGELOG.md:3396`. | 47-REVIEW WR-02 | S4 | **in** (trivial) |
| C9 | Switching-providers page: Ollama and OpenAI-compatible were never probed (no seeded models, adapter-wide values, `paladin.unprofiled_model`); quota mapping unverified for Kimi, Qwen, Grok, Ollama, OpenAI-compatible; retry counters not unified across adapters. | CHANGELOG 0.11.0 Known limitations (1), (2), (5) | S3 | **out** as measurement work; **in** only where dogfooding on one of those providers produces a captured body |
| C10 | `.project/current-exports.txt` last committed 2026-10-21 (plan 51-08), carries no `UnsupportedParameter` / `check_reasoning` rows; CI's API Surface Tracking is green, so this is the committed copy lagging. | Audit G4 | S4 | **in** — `make api-surface` on the pinned nightly, commit *(Reconciled 2026-11-01, Phase 54, D-18: snapshot current, `make api-surface` unchanged on nightly-2026-09-20; the two items live in paladin-ports, which the facade-only snapshot does not re-export, so no regeneration lists them; guarded by `make check-semver`; precedent WINDOWS row 59; see 54-FIRST-EVIDENCE.md)* |
| C11 | Archival `.project/Milestone_*` docs still say `Node<T>`. | Audit G6 | S4 | **out** — archival; a one-line errata note at most |

## D. Release process and CI (Phase 47 carried findings, "next milestone's first phase")

| # | Item | Source | Sev | Rec |
|---|---|---|---|---|
| D1 | CI `Semver Checks (vs v0.9.0)` is a **no-op** (`0 checks: 0 pass, 254 skip`): re-pin `--baseline-version` to `0.11.0`, add `paladin-eval` (now has a published baseline), rename the job. Expect the one known finding against 0.10.1 (`enum_struct_variant_field_added` on `RunOutcome::Halted.cause`) to disappear at the 0.11.0 baseline. | 47-CI-EVIDENCE D-09; `ci.yml:346,399`; 50 deferred-items | S4 | **in** — first |
| D2 | Remove the crate-wide semver allows now that 0.11.0 is the published baseline, exactly as their dated notes say: `struct_missing`, `enum_missing`, `module_missing` (+`enum_variant_missing`) in `crates/paladin-core/Cargo.toml:136-145`; `function_missing`, `module_missing` in `crates/paladin-battalion/Cargo.toml:113-115`; mirrored `.cargo/semver-checks-allowlist.toml` entries. Verified 2026-10-27: the allows and the "remove once 0.11.0 is the baseline" notes are still in place. | 44-REVIEW WR-05 (todo completed 2026-10-24 with this as the trigger) | S4 | **in** — with D1 |
| D3 | `check-migration-allowlist` is not wired into `ci.yml` (only `make check-gates` runs it); `End-to-End Tests` and `Publish Dry Run` run only on `main` pushes, so their first run on a tree is after the irreversible merge. | 47-CI-EVIDENCE D-14 | S4 | **in** — wire the allowlist check; run the two `main`-only jobs on PRs (or on a `release` label) |
| D4 | Release-body builder (`scripts/create-or-reuse-release.sh:105`, `jq -n --arg`) fails above the 131,072-byte argv cap; v0.11.0 shipped only after a manual condensation of the changelog body. Fix: `jq --rawfile` (or stdin) so the body never crosses argv. | 47-CI-EVIDENCE B-1 | S2 (release-blocking) | **in** |
| D5 | `release.yml` has no publish-order step of its own; the gate is `ci.yml`'s `Check publish order` plus the dependency-ordered publish job. The literal ROADMAP wording wanted it in `release.yml`. | 47 A-SC3 reading | S4 | **decide** — add the script as a `release.yml` step (cheap) or record the reading as accepted |
| D6 | GitHub issue `DF3NDR/paladin-dev-env#41` "Automatic Releases on GitHub" (2026-08-28): "process for crates.io … needs to also pin releases on GitHub". `release.yml` now creates the GitHub Release (`create-or-reuse-release.sh`, asset and SBOM upload). | GitHub | S4 | **in** — confirm against the v0.11.0 release page and close with a reference, or restate what is still missing |
| D7 | Advisory CodeQL red on the PR heads (24 high alerts) — untriaged. | Audit tech_debt Phase 47 | S4 | **in** — triage into `CODEQL-DISMISSALS.md`; advisory-only posture unchanged |
| D8 | Three RustSec advisories on `zip 3.0.0` via `utoipa-swagger-ui 9.0.2` (build-dep) carried as exceptions, review date **2026-12-31**, revisit when `utoipa-swagger-ui` depends on `zip >= 9`. | `SECURITY-EXCEPTIONS.md` | S4 | **out** for now — re-check the upstream at milestone open; the review date falls inside v0.11.1's likely window |
| D9 | RUSTSEC-2026-0386 (tokio): **resolved** — `Cargo.lock` carries tokio 1.53.3 (verified 2026-10-27). | 46/47 deferred-items | — | closed; no action |
| D10 | Carried unchanged since Phases 36.1/37/37.1: `cargo test --workspace --all-features` in CI; remove the eight crate-level `#![allow(rustdoc::…)]` (verified: still present in `paladin-llm`, `paladin-notifications`, `paladin-ports` ×2, `paladin-storage`, root `src/lib.rs` ×3); regenerate `doc-coverage-report.md`. | Audit tech_debt Phase 47 | S4 | **decide** — the rustdoc allows are a real hygiene item that keeps re-carrying; the all-features test job is a CI-cost decision |

## E. Tests, lints and flakes

| # | Item | Source | Sev | Rec |
|---|---|---|---|---|
| E1 | `run_api_wiring::tests::build_run_api_persists_*` (2 tests) submit an agent-kind run whose model call goes to a real provider with a fake key and fail in any sandbox without network ("the run never reached a terminal status", 10 s worker deadline). Pre-existing since before Phase 42. | WINDOWS row 73; 42 and 44 deferred-items | S4 | **in** — make them hermetic (a scripted port, no real provider), then disposition row 73 |
| E2 | `cancel_tests::local_cancel_signals_token` waits a fixed 50 ms sleep before `cancel`; under a loaded suite it cancels before the run is registered (`was_local must be true` seen once). Fix: a dispatch-signal wait, not a sleep. Verified 2026-10-27: comment at `cancel_tests.rs:359` still describes the sleep. | 42 deferred-items (wave 6 gate) | S4 | **in** |
| E3 | Single-feature `--tests` clippy warnings in `paladin-llm` (unused `chat` variable, `cadence::PinnedSpread` / `pin_waiter_spread`, `fallback::test_support::ProfiledStubPort`); the workspace all-features gate is clean. | 50 deferred-items (50-15/16) | S4 | **in** (small) |
| E4 | 51 lint leftovers: rustdoc unresolved link `NODE_CACHE_KEY_VERSION` in `engine/cache_key.rs`; redundant explicit links in `reasoning.rs:11,34`; `conformance.rs:280` `thinking_effort_wire_value` dead under default features; `provider_factory.rs:1935` unused import under default features. | 51 deferred-items | S4 | **in** (small) |
| E5 | 51 IN-01 duplicate self-hosted provider list in `reasoning.rs` and `config/layers.rs`. Verified 2026-10-27: only one `SELF_HOSTED_PROVIDERS` constant found (`reasoning.rs:257`); `layers.rs` may encode the pair differently — confirm before closing. | 51 deferred-items | S4 | **in** (verify-then-close) |
| E6 | WINDOWS row 56 (`cost.rs` rustdoc broken links) — **already fixed in the tree** (links are fully qualified; 38-09 fixed them) but the row is still `open`. | WINDOWS row 56 | S4 | disposition only (planning hygiene, not code) |
| E7 | `commander.rs:3834-3957` test bodies carry "TODO: Enhance MockPaladinPort to support error injection" stubs whose real coverage was relocated to `tests/integration/commander_error_paths_test.rs`. | tree | S4 | **in** — delete the stubs or point them at the relocation |
| E8 | 110 `#[ignore]` sites: 6 CLI tests "Requires valid API key" (`setup_check.rs`, `onboarding.rs`), 10 `paladin-storage/scheduler.rs` tests (real cron engine, multi-thread), 11 `file_storage_integration_tests`, 7 Qdrant, 1 MySQL, 1 live MCP. All documented; none a bug. | tree | S4 | **out** — but an `ignore` inventory in the testing guide would help a dogfooder know what `cargo test` does not cover |
| E9 | Code `TODO`s that are **feature stubs** (not v0.11.1): `trigger.rs:216,261` (JSONPath payload matching, cooldown), `file_content_fetcher.rs:105-182` (audio/video/image ingestion), `service_runner.rs:601` (notification adapter wrappers), `planning_service.rs:417` ("Expected output" placeholder), `council_service.rs:734` (registry parameter). | tree | — | **out** — unless any of them is a reachable panic or a silent wrong result; `planning_service.rs:417` returns placeholder text into a real plan and is worth one look |

## F. Planning-register hygiene (GSD commands, not code; listed so the milestone can budget them)

| # | Item | Source | Rec |
|---|---|---|---|
| F1 | Nine phases NOT-VALIDATED (`VALIDATION.md` still `draft`): 40, 41, 42, 43, 45, 47, 49, 50.1, 51. (Phase 38 was PARTIAL at the audit; the maintainer set it `nyquist_compliant: true` on 2026-10-27, commit `1bf6984`.) | Audit Nyquist table | `/gsd-validate-phase N` each; coverage TODO, not a failure |
| F2 | SUMMARY `requirements-completed` frontmatter missing STORE-02 (45) and REASON-02/03/05/06 (51). | Audit | one hygiene commit |
| F3 | WINDOWS rows 68/69 (make security / local coverage not run at Phase 48 seal) superseded in practice by Phase 47 evidence (`make security` exit 0; Coverage 92.74 %) but not dispositioned; row 79 (50.1 probe run by Claude in the devcontainer) likewise; row 56 per E6. | Audit | disposition at milestone open |
| F4 | `.planning/debug/` holds three `diagnosed` sessions (kimi fallback profile, openai max_completion_tokens, vendor token usage fidelity) whose fixes landed (WINDOWS rows 74–76 `fixed`). | tree | archive or mark resolved |
| F5 | `~40` judgment-tier prohibitions in Phases 40 and 42 carry non-authoritative LLM-judge verdicts (ADR-550 D4); human review recommended. | Audit | **decide** — a bounded human read, or accept as-is |

## G. Pending todos and external inputs

| # | Item | Source | Rec |
|---|---|---|---|
| G1 | **web3sec single bump to v0.11.0** — awaits the consumer's return (`47-HANDOFF-web3sec.md` §6). Its divergence list is the primary external bug source for v0.11.1. | `todos/pending/2026-10-14-web3sec-single-bump-to-v0-11-0.md` | keep pending; triage its return into `02-dogfood-findings-log.md` |
| G2 | **OpenAI Responses API for the pro tier** (`gpt-5.5-pro` and siblings 404 on Chat Completions). A new wire, selected per model, behind the existing `LlmPort`. | `todos/pending/2026-10-20-openai-responses-api-for-pro-tier.md` | **decide** — it is feature work by shape, but "the newest model of a supported provider is unusable" reads as a usability gap. If in, it is the one feature item and should be fenced as such. |
| G3 | **Content / general data orchestration base-items review** (`Field`, `Collection`, `element`, `FieldVersionService`, `CollectionVersioningService`: zero consumers outside `base/`; owner says "unfinished, not dormant"). | `todos/pending/2026-10-13-review-content-data-orchestration-base-items.md` | **decide** — the *review* (keep / remove / fence) fits a cleanup milestone; finishing the system does not. B2 (`todo!()` panic) is in the same subsystem. |
| G4 | **Local `make coverage` reproduction** — maintainer-owned walkthrough on a Docker-capable machine, carried since v0.8.0. | `todos/pending/2026-08-13-verify-local-coverage-reproduction.md` | maintainer; fits a dogfood milestone naturally (it *is* dogfooding the testing guide) |

## H. Accepted contracts — explicitly **out** of v0.11.1

Recorded so they are not re-litigated by accident. Each is a documented design limit with an ADR or a
waived ledger row; v0.11.1 only fixes documentation that contradicts them.

- WINDOWS 64 — worker-dispatched `Runnable::Agent` runs write no Waypoint; a Treasurer-halted agent-kind run
  resumes by re-submission, not fork (D-08; ALLOW-03 wording override accepted 2026-10-07).
- WINDOWS 65 — mid-run boundary check is check-only; same-instant admissions can all start (ADR-0057 D-01).
- WINDOWS 66 — a true streamed `execute/stream` call is one provider call with no after-model boundary (G2,
  ADR-0057 group e).
- WINDOWS 61 (waived) — OBS-05 tracing overhead +19.36 % / +16.19 % against a ≤ 3 % bar (maintainer accepted
  2026-09-30).
- Audit G1 — no aggregate HTTP spend route (per-tenant/key/model spend is CLI + library only). Feature.
- FUT-01…FUT-13 on the v2 register (SDK polish, IDE, HA replication, billing, RBAC, Vault policies, LLM-judge
  evals, long-context tier pricing, multi-currency, DNS-rebinding pinning).
- `TokenUsage.total_tokens = prompt + completion` (D-02, owner-kept); vendor disagreement surfaces as
  `paladin.usage_unreconciled`.
