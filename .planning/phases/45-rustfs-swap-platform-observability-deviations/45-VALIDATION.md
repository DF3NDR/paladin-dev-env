---
phase: 45
slug: rustfs-swap-platform-observability-deviations
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-09-29
---

# Phase 45 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (libtest + `#[tokio::test]`, `serial_test`), criterion 0.5 (bench), PyYAML / `docker compose config` / `git grep` static gates |
| **Config file** | root `Cargo.toml` (`integration-tests`, `s3-storage` features; `[[bench]] engine_benchmarks harness = false`); `tests/lib.rs` is the `lib` integration target including `tests/integration/mod.rs` |
| **Quick run command** | the task's targeted filter, e.g. `cargo test -p paladin-ai --lib application::services::run::worker` or `cargo test -p paladin-ai --lib infrastructure::telemetry` |
| **Full suite command** | `cargo test` then `cargo fmt --check`, `cargo clippy -- -D warnings`, `make clean-code`, `make api-surface`, `make security`, `make check-gates` |
| **Estimated runtime** | ~30-90 s per targeted filter after the first build; the contract suite against the native RustFS binary ~60 s |

---

## Sampling Rate

- **After every task commit:** the task's `<automated>` command (a targeted `cargo test` filter, a static gate, or a compose render), plus `cargo fmt --check` and `cargo clippy -p <crate> -- -D warnings`
- **After every plan wave:** `cargo test`, `make clean-code`, `make api-surface`, `make security`, `make check-gates`, `make check-doc-config check-doc-examples check-api-examples`
- **Before `/gsd-verify-work`:** full suite green locally; CI green on Integration Tests, Coverage (82 % floor), Docker Integration Tests, Kubernetes Smoke Test, actionlint and Docs (run ids recorded at UAT)
- **Max feedback latency:** 90 seconds for a targeted filter (the first build of the `lib` integration target with `s3-storage` is longer)

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 45-01-01 | 01 | 1 | STORE-02, STORE-01 | T-45-05 | harness never panics on a hostname; adapter creates its own bucket; throwaway credentials only | integration (native RustFS) | `USE_EXTERNAL_TEST_SERVICES=true TEST_REDIS_HOST=localhost TEST_REDIS_PORT=6380 TEST_MINIO_ENDPOINT=localhost:9010 TEST_MINIO_ACCESS_KEY=testuser TEST_MINIO_SECRET_KEY=testpass123 cargo test --test lib --features integration-tests,s3-storage -- --ignored --test-threads=1 test_file_storage_health_check test_file_upload_download_lifecycle` | ❌ W0 (harness fixes) | ⬜ pending |
| 45-01-02 | 01 | 1 | STORE-02 | T-45-01, T-45-02, T-45-03 | forged multipart token with `..` rejected before any request; presigned URLs never printed with their query; S3 error bodies bounded | unit + integration | `cargo test -p paladin-storage --features s3 --lib minio::tests` then the full `file_storage_integration_tests` run (>= 11 passed) | ❌ W0 (new cases) | ⬜ pending |
| 45-01-03 | 01 | 1 | STORE-01, STORE-02 | T-45-04 | local-mode image pinned to an exact tag with digest comment; advisory register re-pointed, not dropped | build + static | `cargo test --test lib --features integration-tests,s3-storage --no-run && ! grep -q 'name = "testcontainers-modules"' Cargo.lock && make security && make check-gates` | ✅ | ⬜ pending |
| 45-02-01 | 02 | 1 | PLAT-08 | T-45-09 | agent trace records carry ids/usage/cost only | unit | `cargo test -p paladin-ai --lib application::services::run::worker_tests::agent_kind_run_streams_done_live` | ❌ W0 (new test) | ⬜ pending |
| 45-02-02 | 02 | 1 | PLAT-08 | T-45-08, T-45-10, T-45-11 | one terminal event, `message: null` on failure; bus always unbound | unit | `cargo test -p paladin-ai --lib application::services::run::worker && cargo test -p paladin-ai --lib infrastructure::telemetry::herald_sink` | ❌ W0 (new tests) | ⬜ pending |
| 45-02-03 | 02 | 1 | PLAT-08 | T-45-07 | agent webhooks go through the unchanged SSRF-guarded delivery service; P2 on enqueue errors | unit | `cargo test -p paladin-ai --lib application::services::run::worker_tests::agent_kind_run_with_a_webhook_enqueues_a_delivery` | ✅ (inverted) | ⬜ pending |
| 45-03-01 | 03 | 1 | OBS-05 | — | N/A (bench harness) | bench (criterion test mode) | `cargo test --bench engine_benchmarks -- bench_superstep_cost > target/45-03-bench-test.log` then >= 6 `^Testing engine/bench_superstep_cost_sinks_...$` lines and >= 6 `^Success$` lines paired by `grep -A1` (criterion prints the two on separate lines) | ✅ | ⬜ pending |
| 45-03-02 | 03 | 1 | OBS-05 | T-45-13 | filtered target costs no serialisation; error lines carry no record content | unit | `cargo test -p paladin-ai --lib infrastructure::telemetry` | ❌ W0 (new tests) | ⬜ pending |
| 45-03-03 | 03 | 1 | OBS-05 | T-45-12, T-45-14, T-45-15 | bounded buffer retention; seq/tallies untouched; composite panic isolation kept | unit + existing replay suite | `cargo test -p paladin-ai --lib infrastructure::telemetry && cargo test -p paladin-ports --lib output::trace_sink_port && cargo test -p paladin-battalion --lib engine::hooks && cargo test -p paladin-ai --lib -- application::services::run::stream_tests application::services::run::events` | ❌ W0 (new tests) | ⬜ pending |
| 45-04-01 | 04 | 2 | STORE-01, STORE-02 | T-45-16, T-45-17, T-45-18, T-45-19 | pinned digest re-verified; suite cannot pass vacuously; throwaway CI literals; console off | static + listing + local step simulation | PyYAML service/image assertion + `cargo test --test lib --features integration-tests,s3-storage -- --list --ignored` (>= 11) + `make check-gates` | ✅ | ⬜ pending |
| 45-04-02 | 04 | 2 | STORE-01 | T-45-22 | tmpfs `/data` writable by uid 10001; no init container | static | PyYAML assertion on `docker/docker-compose.test.yml` + `make lint-shell && make check-gates` | ✅ | ⬜ pending |
| 45-04-03 | 04 | 2 | STORE-01 | T-45-20, T-45-21 | non-root pod, no privilege escalation, placeholder secrets | static | PyYAML assertion on `k8s/rustfs.yaml` (names, image, probes, uid 10001) + `make check-gates` | ✅ | ⬜ pending |
| 45-05-01 | 05 | 2 | STORE-01 | T-45-23, T-45-24 | no public bucket policy; no old MinIO volume mounted | static (compose render) | `docker compose --env-file target/45-05-e2e.env -f docker/docker-compose.yml config --format json` (PyYAML render with `${VAR:-default}` interpolation when the compose plugin is absent) + Python assertions | ✅ | ⬜ pending |
| 45-05-02 | 05 | 2 | STORE-01 | T-45-26 | console dev-only with non-default credentials | static (compose render) | `docker compose -f .devcontainer/docker-compose.yml config --format json` (PyYAML parse when the compose plugin is absent) + assertions; `make lint-shell` | ✅ | ⬜ pending |
| 45-05-03 | 05 | 2 | STORE-01 | T-45-25 | throwaway dev literals only | static + build | `bash -n scripts/coverage.sh && make lint-shell && cargo check --tests --features cli && make -n minio-console health storage-reset` | ✅ | ⬜ pending |
| 45-06-01 | 06 | 3 | STORE-03 | — | N/A (decision record) | file/grep | ADR headings = 7, `Next free ADR number: 0056`, `ADR-0055` cited in PROJECT.md and the storage page, `make check-doc-config` | ✅ | ⬜ pending |
| 45-06-02 | 06 | 3 | STORE-03 | T-45-27 | docs quote placeholders only | file/grep + doc gates | one Quick Start, retitled H1, `SUMMARY.md` link text, `make check-doc-config && make check-doc-examples` | ✅ | ⬜ pending |
| 45-06-03 | 06 | 3 | STORE-01, STORE-03 | T-45-28 | closures name the removing commits | static (phase-wide gate) | phase-wide no-MinIO grep, one-tag invariant, E2E contract render, todo moves, `make check-gates` | ✅ | ⬜ pending |
| 45-07-01 | 07 | 4 | OBS-05 | T-45-29 | measurement cannot be fabricated in auto mode (`blocking-human`) | manual (release bench, off-sandbox) | `cargo bench --bench engine_benchmarks -- bench_superstep_cost --warm-up-time 1 --measurement-time 3` at points A, B, C | ✅ | ⬜ pending |
| 45-07-02 | 07 | 4 | OBS-05 | T-45-30 | row 35 changed only through the ledger tool; amend row stays open until UAT acceptance | file/grep | no `PENDING (45-07)`, one `Verdict:` line, >= 18 sink-ID lines, ledger and docs cite `45-BENCH-EVIDENCE.md`, `make check-doc-config` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `tests/integration/mod.rs` — `check_service_availability` resolves hostnames (`ToSocketAddrs`, no `unwrap`) — prerequisite for every STORE-02 run (45-01-01)
- [ ] `tests/integration/file_storage_integration_tests.rs` — no `SystemLogAdapter`, `GenericImage` RustFS local mode, `test_multipart_upload_lifecycle`, `test_multipart_abort_leaves_no_object`, `test_etag_is_an_opaque_stable_token`, exercised `test_presigned_urls` (45-01-02, 45-01-03)
- [ ] `crates/paladin-storage/src/minio.rs` — `split_token`/`encode_multipart_token`/`decode_multipart_token` unit tests (45-01-02)
- [ ] `.github/workflows/ci.yml` — `s3-storage` in the contract-suite commands and the compiled-in / passed-count checks (45-04-01, 45-04-02)
- [ ] `src/application/services/run/worker_tests.rs` / `worker.rs` — `agent_kind_run_streams_done_live`, `agent_kind_run_emits_exactly_one_terminal_event`, `agent_kind_run_failure_enqueues_a_failed_delivery`, `failed_delivery_enqueue_error_never_changes_the_failed_status`, `graph_engine_failure_enqueues_failed_delivery`, `agent_model_label_names_the_model_or_none`; `herald_sink_summarises_an_agent_shaped_run` (45-02-01, 45-02-02)
- [ ] `src/infrastructure/telemetry/log_sink.rs` — guard and buffer tests; `crates/paladin-ports/src/output/trace_sink_port.rs` — `composite_sink_hands_an_equal_record_to_its_last_child` (45-03-02, 45-03-03)
- [ ] `45-BENCH-EVIDENCE.md` skeleton and the bench's discarding logger / `_target_off` rows (45-03-01)
- [ ] Native RustFS 1.0.0 release binary in the executor scratch area plus a TCP listener on 6380 — the local stand-in for the CI service containers (45-01-01; reused by 45-04-01 and 45-06-03)

*No framework install: `cargo test`, criterion and `serial_test` are already in the workspace; `testcontainers-modules` is removed, not added.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| RustFS service containers start healthy and the contract suite passes (>= 11) in CI | STORE-01, STORE-02 | the sandbox has no Docker daemon and cannot pull images (CI-attributed, as Phase 40) | Record the run ids of the Integration Tests, Coverage and Docker Integration Tests jobs on the phase tree; check each log for `test result: ok` with >= 11 passed |
| Kubernetes Smoke Test with `k8s/rustfs.yaml` | STORE-01 | needs kind + Docker | Record the kubernetes-smoke run id; `kubectl wait -l app=rustfs` succeeded |
| E2E job (main-only) against the dev compose | STORE-01 | runs only on `main` pushes | Record the run id after the merge to `main` |
| actionlint without the `services.command` suppression; Docs `mdbook build` + linkcheck | STORE-01, STORE-03 | tools not installed in the sandbox | Record the actionlint and Docs workflow run ids |
| Coverage >= 82 % (ADR-0006) | all | `cargo-llvm-cov` + services are CI-only | Read the Coverage job's summary on the phase tree |
| Tracing-overhead re-measure at points A, B, C | OBS-05 | a release criterion build does not fit the sandbox (~1.7 GB free) | Plan 45-07 Task 1 (`checkpoint:human-action`, `blocking-human`) |
| Maintainer acceptance of an amended row-35 figure | OBS-05 | D-19: acceptance is the maintainer's | At UAT; on acceptance run `gsd-tools windows waive <id> "<acceptance>"` |
| `/gsd-verify-work 40` flips Phase 40 UAT test 4 to `pass` | D-02 follow-up | needs the green CI run on the swapped tree | After CI is green; not a Phase 45 criterion |
| Local `make services-up` -> `make coverage` walk | todo 2026-08-13 (docs half folded) | needs a Docker-capable machine; user-owned | Maintainer walk by 2026-10-16 |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 45s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** {pending / approved YYYY-MM-DD}
