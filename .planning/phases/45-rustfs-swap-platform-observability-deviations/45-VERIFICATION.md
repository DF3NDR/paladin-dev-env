---
phase: 45-rustfs-swap-platform-observability-deviations
verified: 2026-09-30T17:30:00Z
status: passed
score: 5/8 must-haves verified
behavior_unverified: 0
overrides_applied: 0
gaps: []
human_verification:

  - test: "Push the phase tree and read the ci.yml run for the Integration Tests, Coverage, Docker Integration Tests and Kubernetes Smoke Test jobs"
    expected: "Each RustFS service container / compose service / k8s pod becomes healthy on /health/ready; the contract suite step prints 'test result: ok. N passed' with N >= 11 (compiled-in count check also >= 11); the smoke test applies k8s/rustfs.yaml and `kubectl wait -l app=rustfs` succeeds. Record the run id in the UAT."
    why_human: "No Docker daemon or image pulls in the authoring sandbox. Local mode (GenericImage + /health/ready poll), container start under Actions, /data ownership for uid 10001, and `curl` inside the image are only exercised in CI. Backstop truths from 45-01 and 45-04."

  - test: "Read the actionlint job in the same CI run"
    expected: "actionlint passes with the `paths:` suppression removed from .github/actionlint.yaml (no `command:` key remains in any workflow)."
    why_human: "actionlint is not installed in the sandbox. Backstop truth from 45-04."

  - test: "Read the Docs workflow (mdbook build + mdbook-linkcheck) for the retitled docs/src/appendix/minio-file-repository-setup.md"
    expected: "Build and linkcheck pass; SUMMARY.md entry 'S3-Compatible File Storage Setup' resolves."
    why_human: "mdbook is not installed in the sandbox. Backstop truth from 45-06."

  - test: "Maintainer acceptance of the OBS-05 re-measured tracing overhead (WINDOWS.md row 61, 45-BENCH-EVIDENCE.md)"
    expected: "Decide whether to accept +19.36 % (log_sink) / +16.19 % (composite) vs the <= 3 % PRD 07 bar as the new figure, then run `gsd-tools windows waive 61 \"<acceptance text>\"`. If rejected, row 61 stays open and further optimisation or the I/O-bound re-scope becomes a follow-up. Weigh the noise caveat: the earlier point C run 1 measured +6.98 % / +7.29 %, and the target-off rows still cost +16-19 %, so the remaining cost is in the dispatcher/sink path, not serialisation."
    why_human: "The roadmap criterion requires the bar be met OR a new ACCEPTED figure be recorded. The bar is not met and acceptance is the maintainer's decision (Phase 28 D-37 precedent, CONTEXT D-19). Backstop truth from 45-07."

  - test: "Follow-ups (not Phase 45 criteria): `make services-up` then `make coverage` on a Docker-capable machine; `/gsd-verify-work 40` once CI is green"
    expected: "Coverage figure reproduces CI's; Phase 40 UAT test 4 flips from blocked to pass."
    why_human: "Maintainer-owned walk (todo 2026-08-13, still pending) and the D-02 follow-up; both depend on a container runtime."
---

# Phase 45: RustFS Swap & Platform/Observability Deviations Verification Report

**Phase Goal:** The dev/test/CI object store runs on a maintained, pinned image instead of a terminal MinIO pin, and two standing accepted deviations (legacy-agent SSE/webhook silence, tracing overhead) are closed or re-measured.
**Verified:** 2026-09-30T17:30:00Z
**Status:** human_needed
**Re-verification:** No, initial verification

No truth is FAILED. Everything verifiable without a container runtime or a CI run was verified against the code. The remaining items are CI-attributed backstop truths and the maintainer's acceptance of the OBS-05 figure, and all of them route to human verification.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1 (static): every live configuration pins `rustfs/rustfs:1.0.0` (digest in an adjacent comment), bucket bootstrap no longer uses `mc`, and no MinIO image remains | VERIFIED | `git grep` phase-wide gate (quay.io/minio, minio/minio, minio/mc, MC_RELEASE, mc alias, MINIO_ROOT_, /minio/health, minio-init, minio-test, anonymous set public, testcontainers.modules; comment lines filtered; `tests/scripts/fixtures` excluded) prints nothing. One-tag invariant: `git grep -h -o 'rustfs/rustfs:[0-9A-Za-z._-]+'` yields only `rustfs/rustfs:1.0.0`. No `:latest`. Seen in both `ci.yml` service blocks (lines 704, 1442), `docker/docker-compose.test.yml:26`, `docker/docker-compose.yml:29`, `.devcontainer/docker-compose.yml:97`, `k8s/rustfs.yaml`, and the suite's `RUSTFS_IMAGE`/`RUSTFS_TAG` consts. `k8s/minio.yaml` is gone (renamed to `k8s/rustfs.yaml`). The compose files have no init service. `testcontainers-modules` is removed from `Cargo.toml` and `Cargo.lock`. No workflow has a `command:` key and `actionlint.yaml` keeps only `self-hosted-runner`. |
| 2 | SC1 (runtime): the Coverage, Integration Tests, Docker Integration Tests and Kubernetes Smoke Test jobs run green against RustFS | ? UNCERTAIN (backstop, human) | Wiring verified: service blocks with `/health/ready` health checks, `TEST_MINIO_ENDPOINT=localhost:9010`, `docker compose ... up -d --wait redis-test rustfs-test`, `kubectl apply -f k8s/rustfs.yaml` + `kubectl wait -l app=rustfs`. No CI run is available from the sandbox. See Human Verification 1. |
| 3 | SC2 (code): the `FileStoragePort` contract suite covers presigned URLs, multipart uploads and ETags through the EXISTING adapter; no second adapter, feature or crate | VERIFIED | `cargo test --test lib --features integration-tests,s3-storage -- --list --ignored` lists 11 `file_storage_integration_tests` cases (8 existing plus `test_multipart_upload_lifecycle`, `test_multipart_abort_leaves_no_object`, `test_etag_is_an_opaque_stable_token`), all registered in `run_all_file_storage_tests`. `minio.rs` has real `upload_part`/`complete_multipart_upload`/`abort_multipart_upload` (no "not fully implemented" string) behind `encode_multipart_token`/`split_token`/`decode_multipart_token`, the last re-running `validate_path`. Path-style `create_with_path_style` bootstrap is in `create_bucket`. `cargo test -p paladin-storage --features s3 minio`: 14 passed, including the four token tests. `test_presigned_urls` does a real PUT and GET and prints only the query-stripped URL. The ETag test asserts opaque, quote-stripped, stable, changes on overwrite, and never compares to an MD5. The only assertions removed from existing cases are two URL-host substring checks, superseded by the real PUT/GET. `Cargo.toml` diff removes only `testcontainers-modules`. `make api-surface` (nightly-2026-09-20): "API surface unchanged" (4044 items). |
| 4 | SC2 (behavior): the suite passes against RustFS | ? UNCERTAIN (backstop, human) | The 45-01 executor reports `11 passed; 0 failed` against a native RustFS 1.0.0 on an empty data directory. I could not reproduce this: the native binary was deleted for disk and there is no Docker. The CI run (Integration Tests + Docker Integration Tests, each asserting compiled-in >= 11 and passed >= 11) is the decision gate per D-08. REQUIREMENTS.md correctly leaves STORE-02 `Pending`. See Human Verification 1. |
| 5 | SC3: storage docs record whether the production Kubernetes manifest also moves to RustFS | VERIFIED | `docs/src/appendix/minio-file-repository-setup.md` has a "Production Kubernetes manifest (ADR-0055)" subsection (line 628): `k8s/rustfs.yaml` is the one manifest for the smoke test and reference deployment; production points the same adapter at AWS S3, a managed endpoint, MinIO or another SigV4 store. The page keeps its path and `SUMMARY.md` entry, cites ADR-0055, and keeps the MinIO/AWS/Spaces notes. ADR-0055 exists with all seven headings and is indexed in `PROMOTION.md` (row 0055, next free 0056) and PROJECT.md. `make check-doc-config`: 150 YAML blocks, 0 failed. Both MinIO todos are in `todos/completed/` with a "Resolution (Phase 45)" section; the coverage todo stays pending. |
| 6 | SC4: a legacy `Runnable::Agent` run emits SSE live events and webhook deliveries the same way a graph run does | VERIFIED | `worker.rs` `run_agent` (lines 1278-1453) builds the per-run sink with `build_run_sink` + `compose_run_sink` (herald), a per-run `TraceDispatcher`, `bus.bind` before dispatch, emits `RunStarted`/`NodeStarted`/`NodeFinished`/`RunFinished`, wraps `execute_scoped` in `with_run_trace_scope`, waits `TRACE_DRAIN_GRACE_PERIOD`, then `unbind`. The success path enqueues via `enqueue_webhook_delivery` -> `webhook_delivery_for_outcome` strictly after `update_status` -> `record_outcome` -> `ack`. The failure path goes through `persist_failure` (no second bus publish) and `record_engine_failure` now also enqueues a `Failed` delivery. The `RunScope` with run id and ledger scope is unchanged. `cargo test --lib`, the 11 named tests: `agent_kind_run_with_a_webhook_enqueues_a_delivery` (inverted pinning test), `agent_kind_run_streams_done_live` (asserts NodeStarted, NodeFinished, Done, all Live), `agent_kind_run_emits_exactly_one_terminal_event`, `agent_kind_run_failure_enqueues_a_failed_delivery`, `graph_engine_failure_enqueues_failed_delivery`, `engine_failure_still_reaches_the_error_wire_name`, the two P2 repository-error tests, the two `RunScope` tests, `herald_sink_summarises_an_agent_shaped_run`: 11 passed. `enqueues_no_delivery` survives only in historical/ledger text. |
| 7 | SC5 (code and measurement): `LogTraceSink` skips serialisation when the target is disabled and reuses its buffer; `bench_superstep_cost_sink_variants` is re-measured | VERIFIED | `log_sink.rs`: `trace_target_enabled()` uses `log::log_enabled!(target: "paladin::trace", Info)` and `on_event` returns before any `serde_json` call when false. `write_trace_line` uses `serde_json::to_writer` into a thread-local `Vec<u8>` with `TRACE_BUF_RETAIN_MAX` (64 KiB) release. `LogTraceSink` stays `#[derive(Debug, Default, Clone, Copy)]` and the API surface is unchanged. `CompositeSink` moves the record into its last child (`split_last`). `TraceDispatcher::emit` keeps the `seq` stamp and usage/cost tallies with no guard; this matches CONTEXT D-17 ("only where ... a sink-independent no-op") and the rustdoc records why. Tests pass: `log_sink_skips_serialisation_when_the_trace_target_is_disabled`, `on_event_writes_no_line_when_the_trace_target_is_disabled`, the buffer-reuse and oversize tests, and `CompositeSink` (11 passed). No new dependencies. The bench adds `DiscardLogger`/`set_trace_target` and `_target_off` rows and keeps the three original IDs. `45-BENCH-EVIDENCE.md` holds the machine block, pasted raw criterion output for A/B/C, and no `PENDING` marker. I re-computed the point C ratio from the pasted output: 143.23/120.00 = +19.36 %, 139.43/120.00 = +16.19 %. |
| 8 | SC5 (outcome): the bar is met (<= 3 %) OR a new ACCEPTED figure is recorded | ? UNCERTAIN (backstop, human) | The bar is NOT met. The D-19 amend branch applies: the figure is recorded in `45-BENCH-EVIDENCE.md`, `observability.md`, PROJECT.md and CHANGELOG `[Unreleased]`, and WINDOWS.md row 61 (open) amends row 35 (still `waived`). Nothing is silently re-waived, which is honest. But "accepted" is the maintainer's call at UAT, so the criterion is not yet satisfiable by the codebase alone. See Human Verification 4. |

**Score:** 5/8 truths verified (0 present-but-behavior-unverified; 3 UNCERTAIN, all CI- or maintainer-attributed backstop truths)

### Deferred Items

None. Later phases (41-44, 46) do not own any of the UNCERTAIN items.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `crates/paladin-storage/src/minio.rs` | multipart trio, token helpers, path-style bootstrap | VERIFIED | Substantive and wired (`MinioAdapter` implements `AdvancedFileStoragePort`; suite exercises it). Any `unwrap`/`expect` sits under `#[cfg(test)]` (line 1157 onward). |
| `tests/integration/file_storage_integration_tests.rs` | 11 cases, GenericImage local mode | VERIFIED | 11 cases listed and registered; `GenericImage::new(RUSTFS_IMAGE, RUSTFS_TAG)` with `/health/ready` poll. |
| `tests/integration/mod.rs` | hostname-safe availability check | VERIFIED | `to_socket_addrs` present (45-01). |
| `.github/workflows/ci.yml` | RustFS services plus contract-suite steps with vacuous-pass guard | VERIFIED (static) | Compiled-in count >= 11 and passed >= 11 guards in both the Integration and Docker jobs; E2E carries `s3-storage`. Runtime is UNCERTAIN (truth 2). |
| `docker/docker-compose.test.yml`, `docker/docker-compose.yml`, `docker/docker-compose.dev.yml`, `.devcontainer/docker-compose.yml` | rustfs services, renamed volumes, no init | VERIFIED | `rustfs_data` / `rustfs-data` volumes; no `minio_data`/`minio-data` anywhere. |
| `k8s/rustfs.yaml`, `k8s/deployment.yaml`, `k8s/configmap.yaml`, `k8s/secret.yaml.example` | renamed manifest, `wait-for-rustfs`, `paladin-rustfs:9000` | VERIFIED | Non-root uid 10001, console off, `/health` and `/health/ready` probes, `emptyDir` 10Gi. |
| `scripts/coverage.sh`, `Makefile`, `.env.example` | probe and targets on `/health/ready` | VERIFIED | `probe_object_store`, `rustfs:9000` fallback, `minio-console` target kept and pointed at the RustFS console. |
| `.planning/decisions/0055-...md`, `PROMOTION.md`, storage docs | ADR and docs | VERIFIED | See truth 5. |
| `src/application/services/run/worker.rs`, `worker_tests.rs`, `herald_sink.rs` | agent-run parity | VERIFIED | See truth 6. |
| `src/infrastructure/telemetry/log_sink.rs`, `crates/paladin-ports/src/output/trace_sink_port.rs`, `benches/engine_benchmarks.rs` | guard, buffer reuse, bench harness | VERIFIED | See truth 7. |
| `45-BENCH-EVIDENCE.md`, `observability.md`, WINDOWS.md rows 60 and 61 | evidence and ledger | VERIFIED (records), acceptance UNCERTAIN | See truth 8. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `minio.rs` | rust-s3 multipart and bucket APIs | `create_with_path_style`, `put_multipart_chunk`, `complete_multipart_upload`, `abort_upload` | WIRED | All four present. |
| `minio.rs` | `FileStorageUtils::validate_path` | `decode_multipart_token` | WIRED | Line 99; traversal and empty-id tests pass. |
| `ci.yml` | `file_storage_integration_tests.rs` | `--features integration-tests,s3-storage ... file_storage_integration_tests -- --ignored` | WIRED | Integration, Docker and E2E jobs. |
| `k8s/deployment.yaml` | `k8s/rustfs.yaml` | `nc -z paladin-rustfs 9000` | WIRED | |
| `k8s/configmap.yaml` | `k8s/rustfs.yaml` | `minio.endpoint: paladin-rustfs:9000` | WIRED | |
| `docker/docker-compose.yml` | `minio.rs` | `APP_MINIO_ENDPOINT=rustfs:9000` with store credentials shared through `${RUSTFS_*}` | WIRED | Adapter creates `paladin-files` itself. |
| `worker.rs` `run_agent` | `build_run_sink` / `RunEventBusSink` / `map_trace_event` | per-run dispatcher | WIRED | Proven by `agent_kind_run_streams_done_live`. |
| `worker.rs` | `webhook_delivery_for_outcome` | `enqueue_webhook_delivery` from the success arm and `persist_failure` | WIRED | |
| `log_sink.rs` | `log::log_enabled!` | `write_trace_line_if_enabled` | WIRED | |
| `WINDOWS.md` | `45-BENCH-EVIDENCE.md` | row 61 | WIRED | |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Produces Real Data | Status |
|----------|------|--------|--------------------|--------|
| Agent-run SSE events | `RunStreamEvent`s | `RunFinished.usage` from `dispatcher.total_usage()`, fed by `NodeFinished` carrying `PaladinResult` usage | Test asserts non-zero `usage.total_tokens` on `node_finished` and `done` | FLOWING |
| Agent-run webhook delivery | `Pending` delivery row | `webhook_delivery_for_outcome(run, ...)` | Test asserts payload with run id, status and event | FLOWING |
| Trace log line | JSON line | `serde_json::to_writer(&record)` | Tests compare each line byte-for-byte with the record's own JSON | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Contract suite compiled in | `cargo test --test lib --features integration-tests,s3-storage -- --list --ignored` (filtered to `file_storage_integration_tests`) | 11 cases | PASS |
| Multipart token and config unit tests | `cargo test -p paladin-storage --features s3 minio` | 14 passed | PASS |
| PLAT-08 tests | `cargo test --lib -- agent_kind_run graph_engine_failure_enqueues ...` | 11 passed | PASS |
| OBS-05 sink tests | `cargo test --lib -- log_sink on_event_writes_no_line trace_buf composite` | 13 passed | PASS |
| `CompositeSink` tests | `cargo test -p paladin-ports --lib trace_sink` | 11 passed | PASS |
| Docs config gate | `make check-doc-config` | 150 blocks, 0 failed | PASS |
| Public surface | `make api-surface PUBLIC_API_TOOLCHAIN=nightly-2026-09-20` | unchanged (4044 items) | PASS |
| Contract suite against a live RustFS | not runnable (no container runtime or binary) | n/a | SKIP -> Human Verification 1 |

The full workspace `make test` was not re-run by me; the orchestrator reports 3995 tests with 0 failures as the regression gate.

### Probe Execution

No probes declared by the phase plans; none discovered for this phase. SKIPPED.

### Requirements Coverage

| Requirement | Source Plan(s) | Description | Status | Evidence |
|-------------|----------------|-------------|--------|----------|
| STORE-01 | 45-01, 45-04, 45-05, 45-06 | RustFS pinned in compose and the four CI jobs; `mc` bootstrap replaced; no MinIO image in live config | SATISFIED statically; runtime NEEDS HUMAN | Truth 1 verified; truth 2 is CI-attributed. REQUIREMENTS.md already marks it Complete before a CI run id exists (see Warnings). |
| STORE-02 | 45-01, 45-04 | Contract suite (presigned URLs, multipart, ETags) passes against RustFS using the existing adapter | NEEDS HUMAN | Truth 3 verified; truth 4 is CI-attributed. REQUIREMENTS.md correctly says Pending. |
| STORE-03 | 45-06 | Storage docs updated, production-manifest decision recorded | SATISFIED | Truth 5. |
| PLAT-08 | 45-02 | Legacy agent runs emit SSE and webhook deliveries like graph runs | SATISFIED | Truth 6. |
| OBS-05 | 45-03, 45-07 | Skip serialisation when disabled, reuse buffers, re-measure, meet bar or record an accepted figure | PARTIAL, NEEDS HUMAN | Truth 7 verified; the bar is not met and the new figure is unaccepted (truth 8). REQUIREMENTS.md marks it Complete ahead of acceptance (see Warnings). |

All five phase requirement IDs appear in at least one plan's `requirements:` field and in REQUIREMENTS.md. No orphaned requirements: REQUIREMENTS.md maps exactly STORE-01, STORE-02, STORE-03, PLAT-08 and OBS-05 to Phase 45.

### Prohibitions (must_haves.prohibitions)

All 21 prohibitions across the seven plans are descriptor-less (`status: unresolved`, `verification: null`). They are therefore dispositioned flagged-unverified, `unverified-prohibition: human review recommended`. The verdicts below are a NON-AUTHORITATIVE LLM judgement from the evidence I gathered, not a green pass.

| Plan | Prohibition (short) | Non-authoritative verdict | Evidence |
|------|---------------------|---------------------------|----------|
| 45-01 | No second adapter/feature/crate unless the suite fails | not violated | No new adapter file; the `Cargo.toml` diff removes one dependency only. |
| 45-01 | No weakened/skipped/deleted existing suite case | not violated | Only two URL-host substring asserts were removed, replaced by real PUT/GET. No new `#[ignore]` on existing cases beyond the suite's existing convention. |
| 45-01, 45-04, 45-05 | No default or real credential; throwaway literals only | not violated | `git grep rustfsadmin` empty; only `testuser`/`testpass123`, `e2euser`, `paladin-dev` literals. |
| 45-02 | Webhook enqueue failure must not change status/ack (P2) | not violated | Two dedicated tests pass; `enqueue_webhook_delivery` logs and never propagates. |
| 45-02 | No input/output/error text in wire events or payloads | not violated | Failure path emits `RunFinished{Failed}` (`message: null`); text stays on the run row. |
| 45-02 | Ledger settlement unchanged (RunScope verbatim) | not violated | `RunScope` construction at `worker.rs:1353-1355` is unchanged; the two scope tests pass. |
| 45-02, 45-07 | No hand-edit of WINDOWS.md | not violated (cannot prove absence) | Ledger JSON consistent with the tool (`windows status`: 61 rows, `last_updated` matches the two 45 commits). |
| 45-03 | Keep `seq` stamp/tallies/bounded queue; no unbounded buffering | not violated | `emit` unchanged in behavior; buffer capped at 64 KiB. |
| 45-03 | No fixture/ID/command change | not violated | Three original IDs kept; same command recorded. |
| 45-03 | No `sonic-rs`/`simd-json`/new dependency | not violated | `Cargo.toml` diff. |
| 45-04 | No vacuous-pass CI; no `:latest`/preview tag; no credential or presigned query in logs | not violated | Compiled-in and passed counts asserted with `pipefail`; single tag; `println!` strips the query string. |
| 45-05 | No anonymous policy, no init container, no reused MinIO volume names | not violated | Gate greps empty. |
| 45-06 | No rewrite of released CHANGELOG or history; MinIO/AWS/Spaces notes kept; todos closed with commit refs | not violated | CHANGELOG diff removes no lines; the notes are present; both todos carry a Resolution. |
| 45-07 | No predicted/partial figure as measured; no silent re-waive; no pre-acceptance | not violated | Every figure computed from pasted raw output; row 61 open; row 35 untouched. |

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (phase-modified files) | n/a | TBD/FIXME/XXX debt markers | none | None added in the diff. |
| `src`, `crates` (non-test) | n/a | `unwrap`/`expect`/`panic!` | none | All additions are in `#[cfg(test)]` code. |
| `.github/copilot-instructions.md` | 134, 419, 422, 484, 533 | Stale "MinIO File Storage (Existing)" / "Services only (Redis, MinIO)" prose | Info | Not a live configuration and not a config quote; outside the plans' file lists. Imported into CLAUDE.md, so a follow-up wording refresh is worthwhile. |

### Warnings (non-blocking)

1. **Traceability drift.** REQUIREMENTS.md marks STORE-01 and OBS-05 `Complete` although their CI-runtime edge (STORE-01) and maintainer acceptance (OBS-05) are still backstop truths. Only STORE-02 is honestly `Pending`. Consider holding those two at `Pending` until the UAT records a CI run id and the row 61 acceptance.
2. **Row 31 status.** WINDOWS.md row 31 stays `waived`; closure is recorded by fixed row 60 because the tool only allows open->fixed|waived transitions (40-06 precedent). The behavior is closed; the ledger presentation is indirect.
3. **OBS-05 noise.** The evidence shows the `_target_off` rows still cost +16-19 % after the serialisation guard. The D-17 fixes did not bring the figure under the bar, and point C run 1 differs markedly (+6.98 %/+7.29 %). Both are disclosed in `45-BENCH-EVIDENCE.md`. The maintainer should weigh this before accepting.

### Human Verification Required

See the `human_verification` frontmatter: (1) CI run on the phase tree for the four jobs and the suite count, (2) actionlint, (3) mdbook build/linkcheck, (4) maintainer acceptance of +19.36 %/+16.19 % (row 61), (5) maintainer-owned follow-ups (local `make coverage` walk, `/gsd-verify-work 40`).

### Gaps Summary

No FAILED truths and no blockers. The codebase delivers: RustFS pinned everywhere with no MinIO image left; a completed 11-case contract suite with a real multipart implementation on the reused adapter; the ADR and storage docs recording the production-manifest decision; live SSE and webhook parity for `Runnable::Agent` runs, pinned by passing tests; and the enablement guard, buffer reuse and a fully evidenced re-measure. Status is `human_needed` because the phase's own backstop truths cannot be confirmed here: CI green on RustFS (no container runtime), actionlint, mdbook, and the maintainer's acceptance of the new tracing-overhead figure, since the <= 3 % bar was not met.

---

_Verified: 2026-09-30T17:30:00Z_
_Verifier: Claude (gsd-verifier)_
