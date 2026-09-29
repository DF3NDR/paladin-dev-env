# Phase 45: RustFS Swap & Platform/Observability Deviations - Research

**Researched:** 2026-09-29
**Domain:** S3-compatible object-store swap (RustFS) for dev/test/CI/reference k8s; legacy-agent SSE + webhook parity in the run worker; trace-sink overhead (`LogTraceSink`, `TraceDispatcher`) and its benchmark
**Confidence:** HIGH on RustFS server behaviour (run natively, see below); MEDIUM on container-runtime details (CI-attributed); MEDIUM on the tracing outcome (predicted, must be measured)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

(Copied from `45-CONTEXT.md` `## Implementation Decisions`. Research below answers HOW, never WHETHER.)

**Carried forward (not re-asked)**
- **D-00a:** Shipped tree outranks any document (Phase 34 D-00g and successors). 82 % workspace line-coverage floor (ADR-0006). `make clean-code`, `make api-surface` (+ `make api-surface-update` and a CHANGELOG entry for an intentional public-surface change), `make security`, `make check-api-examples` and the manual credential-handling review in `.github/instructions/security.instructions.md` gate every commit. No log line, error, response body or CI log ever carries a real credential; the RustFS root credentials used in CI and compose are throwaway test literals exactly as the MinIO ones were.
- **D-00b:** X-03 governs public API: any break needs a `MIGRATION.md` §9.2 row and, when marked `Y`, a `.cargo/semver-checks-allowlist.toml` entry naming one crate and one lint. This phase is designed to be **additive on the Rust surface** (D-08, D-17), so no §9.2 row is expected; if the planner finds one is needed it is written in the same commit as the code.
- **D-00c:** ADRs take the next free number from `.planning/decisions/PROMOTION.md` (currently **0055**) and advance that line in the same commit.
- **D-00d:** Vocabulary: Medieval-military words for roles, plain words for units and identifiers. "RustFS", "bucket", "object store", "trace sink" are plain identifiers; no new officer is minted.
- **D-00e:** Phase 27 D-24 (live SSE = `RunEventBusSink` `TraceSink` adapter feeding a per-run broadcast bus; `map_trace_event` is the ONE trace-to-wire mapping) and D-40 (webhook delivery is a persisted queue drained by `WebhookDeliveryService`, enqueued strictly after the run's own status write and ack, never rolled back) are the shape this phase wires the legacy agent path into. Phase 28 D-03 (one `TraceDispatcher` per run; below-engine producers reach it through the `RUN_TRACE_EMITTER` task-local) is how the agent loop's own trace events reach that sink.
- **D-00f:** Phase 28 D-16 / STATE.md D-37 (the <= 3 % bar measured at +22.18 % log sink / +18.46 % composite, ACCEPTED for v0.10.0) is the baseline this phase re-measures against, same fixture, same command, same three variants (`none`, `log_sink`, `composite`). Any optimisation must leave the gapless-`seq` guarantee and `RunStreamMode::Replay` intact; the observability/replay suite is a hard gate, not the throughput number alone (Pitfall 13).
- **D-00g:** Phases 39-05/39-07 and 40-04 put a `RunScope` carrying the Platform run id and the ledger scope on `run_agent`'s `execute_scoped` call. That call and its scope are preserved verbatim; nothing changes what the ledger settles for an agent run.
- **D-00h:** `security.instructions.md`'s redact-then-truncate rule and the SSRF guard on webhook URLs (checked at submit and at send) are untouched; an agent run's webhook delivery goes through the identical `WebhookDeliveryService` path a graph run's does.

**Sequencing and the interim MinIO re-pin**
- **D-01:** Phase 45 executes **now, ahead of Phases 41-44**. The roadmap's "Depends on: Phase 44" is a diff-conflict-avoidance preference, not a functional dependency. The first plan amends the Phase 45 `Depends on` line in `.planning/ROADMAP.md` to record the resequencing (one line, docs only).
- **D-02:** The interim quick task (`.planning/todos/pending/2026-09-29-interim-minio-image-repin-quay-locked.md`) is **superseded, not executed**: no MinIO image is re-pinned anywhere. Both MinIO todos are closed by this phase's SUMMARY with a pointer to the commits that removed the last reference. After the swap lands green, `/gsd-verify-work 40` is re-run so Phase 40 UAT test 4 flips from `blocked` to `pass` (a follow-up noted in the SUMMARY, not a Phase 45 success criterion).

**RustFS image pin, service shape and bucket bootstrap**
- **D-03:** Image is `rustfs/rustfs` at the **newest exact GA tag the researcher verifies is pullable at plan time**, written as `image: rustfs/rustfs:<tag>` with the resolved manifest digest in an adjacent comment. Never `:latest`, never a floating or `-alpha` tag. The same tag in every live configuration (both `ci.yml` service blocks, `docker/docker-compose.test.yml`, `docker/docker-compose.yml`, `docker/docker-compose.dev.yml` if it declares its own image, `.devcontainer/docker-compose.yml`, the Kubernetes manifest, and the `testcontainers` `GenericImage` in the contract suite's local mode) - one pin, one place per file, one CHANGELOG line naming it. A later re-pin is a deliberate commit, never automatic. *Reversibility: reversible.*
- **D-04:** Service-side environment variables follow the RustFS image's own documented names (researcher confirms exact names, ports, health path). Application-side names are **unchanged**: the `minio:` config section (`config.example.yml`, `k8s/configmap.yaml`), `APP_MINIO_ACCESS_KEY`/`APP_MINIO_SECRET_KEY`, `MinioConfig`, and the harness's `TEST_MINIO_ENDPOINT`/`TEST_MINIO_ACCESS_KEY`/`TEST_MINIO_SECRET_KEY` all keep their names. The Kubernetes secret keys created by the smoke test and consumed by `k8s/deployment.yaml` (`MINIO_ROOT_USER`/`MINIO_ROOT_PASSWORD` for the store, `MINIO_ACCESS_KEY`/`MINIO_SECRET_KEY` for the app) are renamed on the store side only if the RustFS container needs different names; the app-side pair stays.
- **D-05:** **Bucket bootstrap is the adapter's own `ensure_bucket_exists` -> `Bucket::create` path, which already ships in `minio.rs` and runs on every `MinioAdapter::new`.** The compose `minio-init` / `minio-test-init` `mc` containers, the CI "Install MinIO Client" and "Setup MinIO buckets" steps, and the checksum-verified `mc` binary download are **deleted, not replaced** with an `rc` container. Fallback, taken only if the suite proves RustFS rejects the adapter's `PUT /bucket` with a default `BucketConfiguration`: a pinned `rustfs/rc` init container using the same tag discipline as D-03, recorded as a deviation in the SUMMARY.
- **D-06:** Health checks move from `/minio/health/live|ready` to RustFS's health endpoint in every place they appear (both `ci.yml` service `--health-cmd`s, all compose `healthcheck`s, the k8s liveness/readiness probes, the E2E job's `curl -f` check, and the Makefile/devcontainer preflights). `k8s/deployment.yaml`'s `wait-for-minio` init container keeps its `nc -z <svc> 9000` shape against the renamed service.
- **D-07:** Local mode of the contract suite (`FileStorageTestContext::new_local`) swaps `testcontainers_modules::minio::MinIO` for a `testcontainers::GenericImage` of the pinned RustFS tag with the D-04 env vars and a wait-for condition (log line or health), and the `minio` feature is removed from the root `Cargo.toml`'s `testcontainers-modules` dependency. The `testing-guide.md` `GenericImage::new(...)` example is updated to the same pin (docs-currency-gated quote).

**Contract-suite gate, multipart gap and adapter reuse**
- **D-08:** **Reuse first.** No new adapter file, no new Cargo feature, no new crate unless the contract suite fails against RustFS. The decision gate is the suite green in CI's **Docker Integration Tests** job (and the Coverage / Integration Tests service-container jobs) on the phase's tree; the sandbox cannot run containers, so the evidence is CI-attributed as Phase 40's was. Only a suite failure that traces to a RustFS S3-surface gap opens the second-adapter path (new file beside `minio.rs`, behind its own `paladin-storage` feature, sharing the one `rust-s3` dependency), with the failure quoted in the SUMMARY.
- **D-09:** **The suite is completed before it is used as the gate.** Implement the three stubbed methods with `rust-s3 0.35.1`'s existing `put_multipart_chunk` / `complete_multipart_upload` / `abort_upload` (no version bump, no new dependency), and add to the suite: (a) a multipart lifecycle case (create -> >= 2 parts -> complete -> download equals the concatenation; and create -> abort -> object absent), (b) an ETag assertion that treats ETag as an **opaque, quote-stripped, stable token** (same value from `get_file_info` and `list_files` for the same object; a re-upload of different bytes changes it), never as an MD5 of the content, and (c) the presigned-URL case exercises the URLs (a real `PUT` through the upload URL and a real `GET` through the download URL), not just their generation. The adapter's existing `md5_hash = ETag` population stays as-is (a label, not a verified digest) and its rustdoc says so. *Reversibility: reversible.*
- **D-10:** Adapter identity is unchanged: `MinioAdapter`, `MinioConfig`, the `s3` feature in `paladin-storage` and the `s3-storage` facade feature keep their names; prose is corrected to "S3-compatible store (RustFS in dev/test/CI; MinIO, AWS S3, DigitalOcean Spaces or any SigV4 S3 endpoint in production)". A rename is a public break and is deferred.

**Production manifest and the decision record (STORE-03)**
- **D-11:** `k8s/minio.yaml` is one manifest serving both the Kubernetes Smoke Test and the reference deployment, so the reference manifest follows: renamed `k8s/rustfs.yaml` (Deployment + Service `paladin-rustfs`, labels `app: rustfs`, `component: storage`), with the pinned image, D-04 env, D-06 probes and the same `emptyDir`/resource shape. Every reader is updated in the same commit: `ci.yml`'s `kubectl apply`/`kubectl wait`/log-dump lines, `k8s/deployment.yaml`'s init container and service name, `k8s/configmap.yaml`'s `minio.endpoint`, `k8s/README.md`. No split into a smoke-only manifest. *Reversibility: costly* - operators who `kubectl apply -f k8s/minio.yaml` by path lose that path; CHANGELOG and `k8s/README.md` name the rename explicitly.
- **D-12:** Decision recorded as **ADR-0055 "Dev/test and reference object store is RustFS"** in `.planning/decisions/0055-*.md` (Context: 2026-09-12 Docker Hub deletion, terminal quay pin, 2026-09-24 quay lockout; Decision: RustFS exact-tag pin everywhere, adapter reuse proven by the contract suite, reference manifest follows, bootstrap by the adapter; Consequences: one vendor image to re-pin deliberately, re-pin trigger is a RustFS security release or a suite regression), cited from `docs/src/appendix/minio-file-repository-setup.md` (file keeps its path; title and body change to the S3-compatible framing, RustFS quick-start replacing the MinIO one, MinIO/AWS/Spaces notes kept). PROMOTION.md line advances to 0056.
- **D-13:** Every doc that quotes the MinIO pin is updated in the same plan as the config it quotes: `docs/src/appendix/integration-tests.md`, `docs/src/contributing/testing-guide.md`, `docs/src/contributing/branching-model.md`, `docs/src/deployment/cicd.md`, `docs/src/deployment/docker.md`, `.devcontainer/CI-CD.md`, `.devcontainer/README.md`/`QUICKSTART.md`/`FILES.md`/`SETUP_COMPLETE.md` where they name the image, `k8s/README.md`, and the Makefile help text / `minio-console` target (kept as a target name, pointed at RustFS's console). `make docs` (the mdBook build) runs before the commit. *(Research note: there is no `make docs` target; see Pitfall 14.)*

**Legacy `Runnable::Agent` SSE and webhook parity (PLAT-08, row 31)**
- **D-14:** **Same machinery, not a hand-published `done`.** `run_agent` gains the graph path's per-run assembly: `build_run_sink(&self.trace_config, bus_sink, self.run_trace_port.clone())` (+ the `HeraldTraceSink` when a herald is wired, composed exactly as `run_once` does), one `TraceDispatcher::with_capacity` bound to this run, `event_bus.bind` before dispatch, the `execute_scoped` call wrapped in `with_run_trace_scope(&emitter, ...)`, and `event_bus.unbind` after the same `TRACE_DRAIN_GRACE_PERIOD` the graph path waits. Because no engine runs, `run_agent` itself emits `TraceEvent::RunStarted` before the call and `TraceEvent::RunFinished` after it (`status: Completed` or `Failed`, `total_supersteps: 0`, `usage`/`cost` from the dispatcher's `total_usage()`/`total_cost()`, `duration_ms` measured, `trace_dropped_total` overwritten by the dispatcher as today) so the wire `done`/`error` events come from the ONE `map_trace_event` mapping. A `NodeFinished`-style per-call record is the planner's choice. *Reversibility: reversible.*
- **D-15:** **Webhook enqueue uses the shared helper.** On the success path, strictly after `update_status` -> `record_outcome` -> `ack` succeed, `run_agent` calls `webhook_delivery_for_outcome(&run, RunEventKind::Completed, RunStatus::Completed, None, now)` and enqueues it, logging (never propagating) an enqueue error (P2). On the failure path `record_engine_failure` gains the same enqueue for `RunEventKind::Failed` after its own status write and ack, so the graph path's `Err` branch also gets the webhook it was missing; a test confirms a graph run failing through `record_engine_failure` now enqueues exactly one `Failed` delivery. `RunFinished{Failed}` from D-14 and `record_engine_failure`'s existing direct publish must not produce two `error` wire events - the planner picks one, pinned by a test asserting exactly one terminal event per agent run.
- **D-16:** `agent_kind_run_with_a_webhook_enqueues_no_delivery` is inverted to `agent_kind_run_with_a_webhook_enqueues_a_delivery` (one `Pending` delivery whose payload carries the run id, `Completed`, and the assistant fields), plus a sibling asserting the SSE subscriber of an agent run receives `RunStarted`-derived and `done` events in `Live` mode. `WINDOWS.md` row 31 is closed through the ledger tool with the inverted test named as closing evidence; the field docs on `event_bus`/`webhook_deliveries`, `run_agent`'s rustdoc, `webhook/mod.rs`'s `WebhookPayload` carve-out and `docs/src/api-reference/platform-api.md`'s "A run against a code-registered agent never fires a webhook" limitation are rewritten or deleted in the same commit. Cancel/halt `done` status for agent runs is **not** touched (PLAT-09, Phase 42).

**Tracing overhead (OBS-05, row 35)**
- **D-17:** Two no-new-dependency fixes, in this order, each measured: (1) an **enablement guard** - `LogTraceSink::on_event` checks `log::log_enabled!(target: "paladin::trace", log::Level::Info)` before serialising and returns `Ok(())` without touching `serde_json` when the target is filtered; the same guard shape inside `TraceDispatcher`'s consumer only where the planner can show it is a sink-independent no-op (the dispatcher must never learn a sink's log configuration); (2) **buffer reuse** - `write_trace_line` serialises with `serde_json::to_writer` into a reused `Vec<u8>` and logs `str::from_utf8(&buf)` rather than allocating a fresh `String` per record, and `TraceDispatcher::emit`'s per-event `clone()`s (`thread_id`, `run_id`, `usage`) are reduced where an `Arc`/borrow is equivalent, without changing `TraceRecord`'s public shape. `sonic-rs`/`simd-json` are **not** adopted. No unbounded buffering; the drop-oldest bounded queue stays the only backpressure. *Reversibility: reversible.*
- **D-18:** **Same fixture, same command, comparable numbers.** `cargo bench --bench engine_benchmarks -- bench_superstep_cost --warm-up-time 1 --measurement-time 3` on `bench_superstep_cost_sink_variants` (`build_width_graph(8)`, variants `none`/`log_sink`/`composite`), recorded in `45-BENCH-EVIDENCE.md` with the machine/toolchain/free-memory block and raw criterion output as `28-BENCH-EVIDENCE.md` does, run **before and after** each of D-17's two fixes on the same machine in the same session. Bar: PRD 07 criterion 6's <= 3 % for BOTH `log_sink` and `composite` against `none`. The measurement is taken with the `paladin::trace` target **enabled** (an `Info`-level logger installed, output discarded); a second row with the target disabled is recorded for information.
- **D-19:** **Outcome recording.** If both variants meet <= 3 %: row 35 is closed (`fixed`) with the evidence file named; the "Known limitations" bullet in `docs/src/operations/observability.md` and the `[0.10.0]`-era CHANGELOG limitation get a `[Unreleased]` "Fixed" entry citing the new figures. If either still misses: row 35 is **amended, not re-waived silently** - its closing condition is rewritten to the new measured figure, the same figure replaces +22 %/+18 % in `observability.md`, `PROJECT.md`'s known-deviation lines and the CHANGELOG, and the maintainer's acceptance is requested at this phase's UAT (Phase 28 D-37 precedent). No I/O-bound bench variant is added either way.

### Claude's Discretion

- Exact `GenericImage` wait strategy for RustFS in local mode (stdout message vs. HTTP health poll); whether the compose files keep a console port mapping at all.
- Whether `LogTraceSink` keeps `#[derive(Copy)]` with a thread-local buffer, or drops `Copy` for a `Mutex<Vec<u8>>` field - dropping `Copy` on a public facade type is an api-surface change that needs `make api-surface-update` and a CHANGELOG line, so prefer the thread-local unless it is measurably worse.
- The exact `RunStarted` payload for an agent run (`graph_fingerprint` is meaningless without a graph - an empty/`"agent"` sentinel is acceptable if `map_trace_event` and the OTel/herald sinks tolerate it; add a test) and whether a per-call `NodeStarted`/`NodeFinished` pair is emitted.
- How the E2E job's `docker/.env` is written (variable names follow D-04) and whether `docker-compose.dev.yml` needs its own image line or inherits.
- Test topology for the multipart/ETag cases (new `#[ignore]`d `#[tokio::test]`s in `file_storage_integration_tests.rs` registered in `run_all_file_storage_tests`, matching the existing shape) and the part size used (>= 5 MiB per non-final part is the S3 minimum - verify RustFS's own minimum).
- Plan split and wave order - the researcher's RustFS verification is the natural first plan; the agent-wiring and tracing plans are independent of it and of each other.

### Deferred Ideas (OUT OF SCOPE)

- Rename `MinioAdapter`/`MinioConfig`/the `minio:` config section/`APP_MINIO_*`/`TEST_MINIO_*` to an S3-neutral name (public API and operator-config break; future clean-break phase).
- Re-scope the tracing bar to an I/O-bound superstep (Phase 29 D-16's follow-up; only if D-19 records a miss).
- `rust-s3` 0.35.1 -> 0.37.x housekeeping bump (independent change).
- Resolve-then-connect address pinning for the webhook SSRF guard (FUT-13) - untouched.
- A `paladin-storage` `rustfs` feature / native RustFS SDK adapter - only if the D-08 gate fails.
- Multi-arch verification of the RustFS image in the release-image workflow (no arm64 CI leg).
- Out of scope: SSE `done` status for a caller-cancelled/Treasurer-halted run (PLAT-09, Phase 42); Treasurer/allowance/pacing (41-43); legacy Battalion removals (44); Treasurer mdBook page and `MIGRATION.md` v0.10 -> v0.11 guide (46); the user-owned local `make coverage` walk on a Docker-capable machine.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STORE-01 | Dev/test compose stack and the Coverage, Integration Tests, Docker Integration Tests and Kubernetes Smoke Test CI jobs run against RustFS pinned to an exact tag; bucket bootstrap replaces `mc`; no MinIO image remains in any live configuration | RustFS facts table (exact tag `1.0.0`, digest, env, ports, health paths, non-root uid, entrypoint) verified from the 1.0.0 source tree + Docker Hub API + native run; full live-reference inventory (file:line) in "MinIO Reference Inventory"; bucket bootstrap works only after the `create_with_path_style` fix (Pitfall 1) |
| STORE-02 | `FileStoragePort` contract suite (presigned URLs, multipart uploads, ETags) passes against RustFS using the existing S3 adapter; new adapter only if the suite fails | Empirically run against the real RustFS 1.0.0 binary: with the fix set in "Adapter and suite fix set" all 8 existing cases plus a multipart lifecycle case pass; the suite is currently NOT run by any CI job and cannot run as written (Pitfalls 2-6), so the fix set is prerequisite work |
| STORE-03 | Storage docs updated; decision on whether the production k8s manifest also moves to RustFS recorded | ADR-0055 mechanics (PROMOTION.md line 76, template ADR-0054), docs rewrite scope for `minio-file-repository-setup.md` (909 lines, duplicated Quick Start sections), k8s manifest shape with verified probes/env/uid |
| PLAT-08 | Legacy `Runnable::Agent` runs emit SSE live events and webhook deliveries like a graph run | `run_agent`/`record_engine_failure` read in full; sink consumers checked for `total_supersteps: 0` tolerance; single-terminal-event design; `RunStarted` maps to NO wire event (D-16 sibling test must assert `done`, not a `RunStarted`-derived event); ledger tool cannot move `waived` to `fixed` (Pitfall 9) |
| OBS-05 | `LogTraceSink`/`TraceDispatcher::emit` skip serialisation when logging disabled and reuse buffers; bench re-measured; <= 3 % or new accepted figure | `write_trace_line` root cause, `log_enabled!` semantics on `log 0.4.30`, thread-local buffer keeps the public surface unchanged, the bench installs NO logger today (Phase 28 numbers = target-disabled-but-serialising), avoidable clones identified, and a predicted outcome for D-19 |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

Directives extracted from `./CLAUDE.md` and its imported `.github/*.md` files; the planner must verify each plan against them:

- **TDD (Red-Green-Refactor)**; **82 % workspace line coverage** (ADR-0006, `cargo llvm-cov --fail-under-lines` in CI's `coverage` job); every public API needs a doc test.
- **Dependencies flow inward only** (core -> nothing; ports -> core; adapters -> core + ports). The run-worker changes live in the facade `src/application/services/run/`; the trace-sink change in `src/infrastructure/telemetry/`; `CompositeSink` in `paladin-ports` (no infrastructure import).
- **Ubiquitous language**: Medieval-military terms in code/docs/comments (Paladin, Battalion, Garrison, Arsenal, Citadel, Herald, Quest); RustFS/bucket/trace sink are plain identifiers (D-00d).
- **Before committing a parent task**: `cargo test` -> `cargo fmt --check` -> `cargo clippy` -> `make api-surface` (an intentional surface change: `make api-surface-update` + CHANGELOG entry); conventional-commit message; stop after each major task.
- **Security**: `make security` (cargo-audit + cargo-deny) and `cargo clippy -- -D warnings` on new/modified code, plus the manual credential-handling review in `security.instructions.md`. CodeQL is advisory-only; **do not reintroduce Snyk** and do not record a phase as blocked on a Snyk scan.
- **No `unwrap()`/`expect()`/`panic!` in library code** (the existing `check_service_availability` `unwrap()` in the test harness is exactly the defect Pitfall 3 documents); prefer borrowing over cloning; iterators lazy.
- Port traits are `Send + Sync`; errors are layer-specific `thiserror` enums converted at boundaries.
- `.github/instructions/security.instructions.md`: redact-then-truncate response bodies before embedding in errors/logs; no log statement interpolates an API key; HTTP clients carrying a credential header do not follow redirects; webhook SSRF guard at write and send time (untouched here but the agent path now reaches it).

## Summary

RustFS `1.0.0` (GA, 2026-09-16, Apache-2.0, multi-arch amd64+arm64, manifest-list digest `sha256:8cc9801755448b71a786705ce76692c77e14936cccd87cf2fc31842e58f4d1ff`) is the right pin, and its S3 surface is **not** the risk the roadmap flagged: I ran the actual `rustfs-linux-x86_64-musl-v1.0.0` release binary (the same artifact the Docker image packages) natively in the sandbox and drove the repo's real `MinioAdapter` and test suite against it. Presigned PUT/GET (real HTTP through the URLs), multipart create/upload-part/complete/abort with `rust-s3 0.35.1`, ETags (quoted, stable across HEAD and list, changes on re-upload, composite `md5-N` after multipart), and the 5 MiB non-final-part minimum all behave like AWS S3. **No second adapter is needed.** The risk is instead in the repo: the contract suite that is supposed to be the gate has never actually run in CI, and cannot run as written.

Six pre-existing defects stand between the tree and a meaningful gate (all reproduced empirically): (1) `MinioAdapter::create_bucket` calls `Bucket::create`, which uses virtual-hosted addressing (`<bucket>.<host>`), so D-05's "bootstrap already ships" is only true after a one-line switch to `Bucket::create_with_path_style` - RustFS answers `501 NotImplemented` to virtual-hosted requests; (2) `copy_file` passes `"<bucket>/<key>"` to `copy_object_internal`, which prepends the bucket itself, giving `NoSuchKey`; (3) the three multipart methods are stubs; (4) `TestEnvironment::check_service_availability` `unwrap()`s a `SocketAddr` parse and panics on any hostname (`localhost:6380`, which is what CI sets); (5) the suite's `SystemLogAdapter::new` re-inits `env_logger` after `TestEnvironment::new` already did, panicking every test; (6) **no CI command enables the `s3-storage` feature**, so `file_storage_integration_tests` is `cfg`'d out of the Integration Tests, Coverage, Docker Integration and E2E jobs and every "run" matches zero tests. With a fix set of roughly a hundred lines (bucket path-style, copy, three multipart methods via a stateless upload-id token, harness hostname resolution, dropping the log adapter from the suite) all eight existing cases plus a multipart lifecycle case passed against RustFS 1.0.0 with the adapter bootstrapping the bucket itself on a fresh data directory.

For PLAT-08 the wiring is mechanical but has two traps: `RunStarted` maps to **no** wire event in `map_trace_event` (so D-16's sibling test must assert `done`, plus `node_*` events only if the planner emits a node pair), and the graph path's `record_engine_failure` cannot be reused unchanged without double-publishing `error`. For OBS-05, `TraceDispatcher::emit` does no serialisation and its `thread_id`/`run_id` clones are forced by `TraceRecord`'s owned fields, so the only real serialisation fix is in `LogTraceSink`; the bench installs no logger today, and with the target enabled a <= 3 % bar (about 3 microseconds for ~22 records) is arithmetically implausible, so plan for D-19's "amend" branch. Two ledger facts matter to the planner: `gsd-tools windows fixed` refuses rows that are already `waived` (rows 31 and 35 both are), and there is no `make docs` target.

**Primary recommendation:** Pin `rustfs/rustfs:1.0.0` (digest in the comment), make the contract suite real first (fix set + `s3-storage` in the CI commands) and prove it locally against the native release binary before touching compose/CI/k8s; then swap infrastructure, then docs/ADR; run the agent-parity and tracing plans in parallel, each behind its own tests.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Object storage service (dev/test/CI/reference) | External service container (compose / Actions `services:` / k8s Deployment) | - | Infrastructure, pinned image; no app code owns it |
| Bucket bootstrap | Infrastructure adapter (`paladin-storage::minio::MinioAdapter::new`) | - | Adapter already owns connectivity; D-05 deletes the external `mc` step |
| Multipart upload state | Infrastructure adapter (stateless token in the returned upload id) | - | Port contract returns an opaque `String`; adapter is the only layer that knows the S3 key/upload-id pairing |
| `FileStoragePort` contract suite | Test harness (`tests/integration/`) | CI job wiring | Verifies the adapter against a real S3 endpoint |
| Legacy-agent SSE + webhook parity | Application service (`RunWorkerPool::run_agent`) | Infrastructure sinks (`build_run_sink`, `RunEventBusSink`) | Worker composes per-run sinks and enqueues deliveries; wire mapping stays in `map_trace_event` |
| Trace serialisation guard/buffer | Infrastructure telemetry (`LogTraceSink`) | Port adapter (`CompositeSink` clone) | Serialisation is a sink concern; the dispatcher (battalion) must stay sink-agnostic |
| Ledger/ADR/CHANGELOG bookkeeping | Planning docs | - | Governance artefacts, no runtime tier |

## Standard Stack

### Core

| Component | Version | Purpose | Why Standard |
|-----------|---------|---------|--------------|
| `rustfs/rustfs` image | `1.0.0` (manifest list `sha256:8cc9801755448b71a786705ce76692c77e14936cccd87cf2fc31842e58f4d1ff`; amd64 `sha256:ba0a1b53e36f321c0d46f3867104abef169f7bc59c467c664ddac87e7ddc9a8b`; arm64 `sha256:42edb61d588775f9431ff436216d14392d2234d4eb2ed68321569fbf7245b36b`) | Dev/test/CI/reference S3 endpoint | Newest GA tag; Apache-2.0; multi-arch; Docker Hub org `rustfs`, 12.5 M pulls [VERIFIED: hub.docker.com v2 API 2026-09-29] |
| `rust-s3` | 0.35.1 (unchanged) | S3 client inside `MinioAdapter` | Already a dependency; `put_multipart_chunk`, `complete_multipart_upload`, `abort_upload`, `create_with_path_style`, `copy_object_internal` all exist in 0.35.1 [VERIFIED: `~/.cargo/registry/src/*/rust-s3-0.35.1/src/bucket.rs`] |
| `testcontainers` | 0.24.0 (unchanged) | Local-mode `GenericImage` | `GenericImage::new(name, tag)`, `.with_exposed_port(9000.tcp())`, `ImageExt::with_env_var` [VERIFIED: registry source]; same shape already used by `redis_queue_integration_test.rs` |
| `log` | 0.4.30 (Cargo.lock) | `log_enabled!` guard | `log_enabled!(target: "...", Level::Info)` exists; default max level is `Off` when no logger installed [VERIFIED: `log-0.4.30/src/macros.rs`, `lib.rs`] |
| `serde_json` | workspace (unchanged) | `to_writer` into a reused buffer | Already the serializer |

### Supporting

| Component | Version | Purpose | When to Use |
|-----------|---------|---------|-------------|
| `rustfs/rc` image | `v0.1.36` (2026-09-16, amd64+arm64) [VERIFIED: Docker Hub API] | S3 client for an init container | **Fallback only** (D-05); not needed - bootstrap verified via the adapter |
| Native release binary `rustfs-linux-x86_64-musl-v1.0.0.zip` (~186 MB) from `github.com/rustfs/rustfs/releases/download/1.0.0/` | 1.0.0 | Local red/green loop for the suite without Docker | Executor sandbox without a Docker daemon: run `./rustfs --address 127.0.0.1:9010 --console-address 127.0.0.1:9011 <dir>` with `RUSTFS_ACCESS_KEY`/`RUSTFS_SECRET_KEY` set |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `rustfs/rustfs:1.0.0` (musl default) | `1.0.0-glibc` (digest `sha256:bffcab0c9d647aab0055d1c69d340b202d0909966b385932d4ead1aeb7602858`) | Larger image (147 MB vs ~105 MB); no behavioural need; keep the default musl tag |
| `1.0.1-preview.11` (pushed 2026-09-29) | - | Pre-release; D-03 forbids it. Re-check the tag list at execution time and take the newest **non-preview, non-rc** tag |
| `WaitFor::http` in the testcontainers wait | Poll `/health/ready` from the test with the existing `reqwest` | `WaitFor::http` needs `testcontainers`' `http_wait` feature (adds `reqwest` to that crate's feature set); a 10-line poll helper needs no `Cargo.toml` change |
| Upload-id -> key `HashMap` in the adapter | Stateless length-prefixed token (recommended) | The map leaks entries and breaks across restarts/replicas; the token needs no state |

**Installation:** none - no new Rust crate. **Removal:** delete `testcontainers-modules` from root `Cargo.toml` `[dev-dependencies]` entirely (the `minio` feature is its only use: `git grep testcontainers_modules` finds only `file_storage_integration_tests.rs:7`), then refresh `Cargo.lock`.

**Version verification (executor re-run at plan/execute time):**
```bash
curl -sS "https://hub.docker.com/v2/repositories/rustfs/rustfs/tags?page_size=25&ordering=last_updated" \
  | python3 -c "import sys,json;[print(t['name'],t['last_updated'][:10],t['digest']) for t in json.load(sys.stdin)['results']]"
# with a Docker daemon: docker buildx imagetools inspect rustfs/rustfs:1.0.0
```

### RustFS facts (what the planner writes into config)

| Fact | Value | Status |
|------|-------|--------|
| Newest GA tag | `1.0.0` (2026-09-16); `1.0.1-preview.11` is pre-release | Confirmed [VERIFIED: Docker Hub API; CITED: github.com/rustfs/rustfs/releases shows 1.0.0 as Stable GA] |
| Architectures | linux/amd64 + linux/arm64 on `1.0.0` | Confirmed [VERIFIED: Docker Hub API] |
| Licence | Apache-2.0 (README; image `license` label) | Confirmed [VERIFIED: 1.0.0 README + Dockerfile label] |
| Root credential env | `RUSTFS_ACCESS_KEY`, `RUSTFS_SECRET_KEY` (also `*_FILE` variants; legacy `RUSTFS_ROOT_USER`; `MINIO_ROOT_USER` is mapped to it by a compat shim - do not rely on it) | Confirmed [VERIFIED: 1.0.0 `entrypoint.sh`, `docker-compose.yml`, `startup_preflight.rs`; native run with these names] |
| Credential rules | Access key >= 3 chars, secret >= 8; empty value = hard failure; the literal `rustfsadmin` only WARNs. No credentials baked in | Confirmed [VERIFIED: entrypoint.sh, `credentials.rs`] |
| Ports | S3 API `:9000`, console `:9001` (image `EXPOSE 9000 9001`) | Confirmed [VERIFIED: Dockerfile + config constants] |
| Health (API port) | `GET /health` (liveness, always 200), `GET /health/live` (alias), `GET /health/ready` (readiness; 503 until storage+IAM ready; JSON body), plus MinIO aliases `/minio/health/live` and `/minio/health/ready` | Confirmed [VERIFIED: `rustfs/src/server/health.rs`, `prefix.rs`; native run: `/health` 200, `/health/ready` 200 with `"ready":true`] |
| Console health | `GET :9001/rustfs/console/health` -> 200; console UI `http://host:9001/rustfs/console/index.html`; `GET /` -> 403 | Confirmed [VERIFIED: native run] |
| Console switch | `RUSTFS_CONSOLE_ENABLE=false` removes the console listener; API `/health` unaffected. Default enabled; image sets `RUSTFS_CONSOLE_CORS_ALLOWED_ORIGINS="*"` | Confirmed [VERIFIED: native run; Dockerfile] |
| Command / args | **None needed.** `ENTRYPOINT ["/entrypoint.sh"]`, `CMD ["rustfs"]`; image ENV `RUSTFS_VOLUMES="/data"`; entrypoint appends `/data` and `mkdir -p`s it. So no `command:` in any service block | Confirmed by source [VERIFIED: Dockerfile + entrypoint.sh]; container start itself is CI-attributed |
| Volume / user | `VOLUME ["/data"]`; runs as non-root `rustfs` uid/gid `10001:10001`; `/data` and `/logs` pre-owned by that user; bind mounts must be writable by 10001 | Confirmed [VERIFIED: Dockerfile + README] |
| Logging | Image sets `RUSTFS_OBS_LOG_DIRECTORY=/logs`, `RUSTFS_OBS_LOGGER_LEVEL=warn`: logs go to files, so **stdout carries almost nothing** - do not use a stdout `WaitFor` | Confirmed [VERIFIED: Dockerfile; native run] |
| Image tools | Alpine 3.24 base with `curl`, `coreutils`, `ca-certificates` (so `--health-cmd "curl -f ..."` works inside the container). No `HEALTHCHECK` baked in | Confirmed [VERIFIED: Dockerfile] |
| Multipart min part size | 5 MiB for every non-final part, enforced at `CompleteMultipartUpload` (`EntityTooSmall`, HTTP 400) | Confirmed [VERIFIED: source `GLOBAL_MIN_PART_SIZE = 5 MiB`; native run reproduced 400 EntityTooSmall] |
| ETag behaviour | Quoted on the wire; simple object = MD5 hex; multipart = composite `<md5>-<N>`; `CompleteMultipartUpload` accepts quoted or unquoted part ETags; same value on HEAD and list; changes on overwrite | Confirmed [VERIFIED: native run + source `trim_etag`] |
| Bucket create | Path-style `PUT /bucket` with `<LocationConstraint>us-east-1</LocationConstraint>` (what `BucketConfiguration::default()` + `Region::Custom` sends) -> 200; repeating it on an existing bucket -> **200** (not 409) | Confirmed [VERIFIED: native run] |
| Virtual-hosted addressing | Unsupported without `RUSTFS_SERVER_DOMAINS`: `501 NotImplemented` | Confirmed [VERIFIED: native run with a `*.localhost` hosts entry] |
| Memory | ~175 MB RSS after the suite (fits the existing k8s 512Mi/1Gi shape) | Measured [VERIFIED: native run] |
| tmpfs data dir | Works (`/dev/shm` data dir passed health + an adapter smoke run) | Measured natively as root; Docker's tmpfs mode/uid behaviour is CI-attributed [ASSUMED] |

## Package Legitimacy Audit

No new Rust/npm/PyPI package is installed by this phase, so the `gsd-tools package-legitimacy` seam does not apply; `testcontainers-modules` is **removed**. The only new third-party artefact is a container image, assessed manually:

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `rustfs/rustfs` (image) | Docker Hub | registered 2024-10-22 (~11 mo) | 12.57 M pulls, 49 stars | github.com/rustfs/rustfs (Apache-2.0; Dockerfile downloads the release zip and verifies its sha256 digest at build) | OK (manual) | Approved; pin exact tag + digest comment |
| `rustfs/rc` (image) | Docker Hub | 35 tags, `v0.1.36` 2026-09-16 | - | github.com/rustfs/rustfs (client crate; `rc` executable) | 0.x, fallback only | Not installed; if D-05's fallback is ever taken, add a `checkpoint:human-verify` and pin tag+digest |
| `testcontainers-modules` | crates.io | - | - | - | n/a | REMOVED from dev-dependencies |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none (`rustfs/rc` is a documented fallback that is not installed by default)

## Architecture Patterns

### System Architecture Diagram

```
                         ┌────────────────────────── STORE track ──────────────────────────┐
 developer / CI job ──▶  │ compose / Actions services: / k8s Deployment                      │
                         │   rustfs/rustfs:1.0.0  (no command; RUSTFS_ACCESS_KEY/SECRET_KEY) │
                         │   :9000 S3 API   /health, /health/ready      :9001 console        │
                         └───────────────▲──────────────────────────────────────────────────┘
                                         │ SigV4 path-style HTTP
 config `minio:` / APP_MINIO_* ──▶ MinioAdapter::new ─▶ ensure_bucket_exists
   TEST_MINIO_* (harness)              │                 └─ list ─404→ Bucket::create_with_path_style
                                       ├─ upload/get/head/list/delete/copy_object_internal(key)
                                       ├─ presign_put / presign_get
                                       └─ multipart: initiate ─▶ token "<len>:<key><s3-upload-id>"
                                                     upload_part / complete / abort decode token

                         ┌────────────────────────── PLAT-08 track ────────────────────────┐
 queue.dequeue ─▶ run_once ─▶ resolver ─┬─ Runnable::Workflow ─▶ engine (emits RunStarted/RunFinished)
                                        └─ Runnable::Agent ─▶ run_agent
                                             build_run_sink(+Herald) ─▶ TraceDispatcher (per run)
                                             bus.bind ─▶ emit RunStarted
                                             with_run_trace_scope(execute_scoped) ── agent loop emits Middleware/Progress/Fallback
                                             emit NodeStarted/NodeFinished(usage,cost) + RunFinished{Completed|Failed}
                                             update_status ▶ record_outcome ▶ ack ▶ webhook enqueue (shared helper)
                                             sleep(TRACE_DRAIN_GRACE_PERIOD) ▶ bus.unbind
                       dispatcher ─▶ CompositeSink ─▶ RunEventBusSink ─map_trace_event─▶ SSE done/error
                                                  └▶ LogTraceSink / Persisting / Otel / Herald

                         ┌────────────────────────── OBS-05 track ─────────────────────────┐
 emit() ─ stamp seq, clone thread_id/run_id (forced by TraceRecord), queue ─▶ consumer task
   ─▶ CompositeSink (record.clone() per child; move on last child) ─▶ LogTraceSink::on_event
        log_enabled!(paladin::trace, Info)? no ─▶ return Ok (no serde)
                                            yes ─▶ thread-local Vec<u8> ◀ serde_json::to_writer ─▶ log::info!
```

### Recommended Project Structure (files touched)

```
crates/paladin-storage/src/minio.rs        # bucket path-style, copy fix, 3 multipart methods, split_token helper, rustdoc S3-neutral
crates/paladin-ports/src/output/trace_sink_port.rs   # CompositeSink: move record into the last child
src/infrastructure/telemetry/log_sink.rs   # guard + thread_local buffer (no struct change; Copy kept)
src/application/services/run/worker.rs     # run_agent wiring, persist_failure helper, docs
src/application/services/run/worker_tests.rs  # inverted test + SSE sibling + terminal-once + graph-Err webhook
src/application/services/run/webhook/mod.rs   # delete WR-02 carve-out (lines ~51-58)
tests/integration/{mod.rs,file_storage_integration_tests.rs}  # harness fixes, GenericImage local mode, multipart/ETag/presign cases
benches/engine_benchmarks.rs               # discarding logger install, enabled + target-off rows
k8s/rustfs.yaml (renamed from minio.yaml), k8s/{deployment,configmap}.yaml, k8s/README.md
docker/docker-compose{,.test,.dev}.yml, .devcontainer/docker-compose.yml, .github/workflows/ci.yml, .github/actionlint.yaml
Makefile, scripts/{coverage,run_integration_tests}.sh, .env.example
.planning/decisions/0055-*.md, PROMOTION.md, ROADMAP.md, WINDOWS.md (via tool), 45-BENCH-EVIDENCE.md, CHANGELOG.md, docs/...
```

### Pattern 1: Stateless multipart upload token (adapter)

**What:** `create_multipart_upload` returns `"{key.len()}:{key}{s3_upload_id}"`; `upload_part`/`complete`/`abort` split it. No adapter state, survives restarts and multiple replicas, no new dependency. Run the decoded key through `path_to_object_name`/`validate_path` again (a caller can forge a token) and reject a malformed token with `FileStorageError::InvalidPath`.
**When to use:** the port returns an opaque `String` upload id but `rust-s3`'s chunk/complete/abort calls need the key too.
**Example:** (verified working against RustFS 1.0.0; saved diff of the whole fix set lives in the researcher scratchpad and is reproduced in the essentials here)
```rust
// create
let response = self.bucket.initiate_multipart_upload(&object_name, "application/octet-stream").await
    .map_err(|e| FileStorageError::IoError(format!("Failed to initiate multipart upload: {}", e)))?;
Ok(format!("{}:{}{}", object_name.len(), object_name, response.upload_id))

// upload_part: put_multipart_chunk(chunk: Vec<u8>, path: &str, part_number: u32, upload_id: &str, content_type: &str) -> Result<Part, S3Error>
let (key, id) = split_token(upload_id)?;
let part = self.bucket.put_multipart_chunk(content.to_vec(), key, part_number, id, "application/octet-stream").await
    .map_err(|e| FileStorageError::IoError(format!("Failed to upload part: {}", e)))?;
Ok(part.etag)            // raw header value, quotes included; the port treats it as opaque

// complete: complete_multipart_upload(path: &str, upload_id: &str, parts: Vec<Part>) -> Result<ResponseData, S3Error>
let parts = parts.into_iter().map(|(part_number, etag)| s3::serde_types::Part { part_number, etag }).collect();
let resp = self.bucket.complete_multipart_upload(key, id, parts).await.map_err(/* IoError */)?;
// S3 may return 200 with an <Error> body on complete: check resp.as_str() for "<Error>" before trusting it
self.get_file_info(Path::new(key)).await

// abort: abort_upload(key: &str, upload_id: &str) -> Result<(), S3Error>
fn split_token(token: &str) -> FileStorageResult<(&str, &str)> {
    let bad = || FileStorageError::InvalidPath("malformed multipart upload id".to_string());
    let (len, rest) = token.split_once(':').ok_or_else(bad)?;
    let n: usize = len.parse().map_err(|_| bad())?;
    Ok((rest.get(..n).ok_or_else(bad)?, rest.get(n..).ok_or_else(bad)?))
}
```
Note: `put_multipart_chunk` **aborts the whole upload itself** when a part returns non-2xx, then returns the error; a later `abort_multipart_upload` on that id returns `NoSuchUpload` (404). Treat that as already-aborted in the port method if the suite needs idempotent abort. RustFS: abort of a live upload -> 200, abort again -> `404 NoSuchUpload` [VERIFIED: native run].

### Pattern 2: Bucket bootstrap and copy (two one-line fixes)

```rust
// create_bucket(): virtual-hosted -> path style
Bucket::create_with_path_style(&self.config.bucket, self.bucket.region(), credentials, config)
// copy_file(): pass the bare key; rust-s3's copy_object_internal prepends "<bucket>/" itself
self.bucket.copy_object_internal(&source_object, &dest_object)
```
`create_with_path_style` returns 200 on a fresh bucket and 200 again when it already exists [VERIFIED]; the existing `HttpFailWithBody(409, _)` arm can stay as a harmless safety net.

### Pattern 3: `run_agent` per-run composition (PLAT-08)

```rust
// after the paladin_port nack check and input_text; sink assembly mirrors run_once (lines ~1000-1040)
let bus_sink = self.event_bus.as_ref().map(|bus| Arc::new(RunEventBusSink::new(Arc::clone(bus))) as Arc<dyn TraceSink>);
let base = build_run_sink(&self.trace_config, bus_sink, self.run_trace_port.clone());
let herald = self.herald.as_ref().map(|h| Arc::new(HeraldTraceSink::new(Arc::clone(h), paladin.node.model.clone())) as Arc<dyn TraceSink>);
let sink = /* same (None,None)/(Some,None)/(None,Some)/(Some,Some -> CompositeSink) match as run_once */;
let dispatcher = sink.map(|s| Arc::new(TraceDispatcher::with_capacity(
    run.thread_id.clone(), Some(run.run_id.clone()), Some(s), self.trace_config.channel_capacity)));
let emitter: Option<Arc<dyn TraceEmitter>> = dispatcher.clone().map(|d| d as Arc<dyn TraceEmitter>);
if let Some(bus) = &self.event_bus { bus.bind(run.thread_id.clone(), run.run_id.clone()).await; }
if let Some(d) = &dispatcher { d.emit(TraceEvent::RunStarted { run_id: Some(run.run_id.clone()), graph_fingerprint: "agent".to_string() }); }
let started = std::time::Instant::now();
let result = with_run_trace_scope(&emitter, paladin_port.execute_scoped(paladin.as_ref(), &input_text, &HeartbeatHandle::new(), &run_scope)).await;
// emit NodeStarted/NodeFinished{usage: result.usage, cost: result.cost, duration_ms: result.execution_time_ms}
// (or NodeFinished{outcome: Failed}) so dispatcher.total_usage()/total_cost() are real, THEN
// RunFinished{status, total_supersteps: 0, usage: d.total_usage(), cost: d.total_cost(), duration_ms, trace_dropped_total: 0}
// then: update_status -> record_outcome -> ack -> webhook enqueue -> sleep(TRACE_DRAIN_GRACE_PERIOD) -> bus.unbind
```
Keep the `RunScope` construction and the `execute_scoped` arguments verbatim (D-00g). Keep the `let Some(paladin_port) = ... else { nack }` early return BEFORE `bind`, so a nacked run never leaves a bound channel.

**Single-terminal-event design (D-15 resolution, recommended):** split the tail of `record_engine_failure` into a private `persist_failure(&self, leased, run, error_text) -> Result<bool, WorkerError>` (status `Running -> Failed`, `record_outcome`, `ack`/`nack`, and - only on the ack path - the `Failed` webhook enqueue via `webhook_delivery_for_outcome`). `record_engine_failure` stays exactly what it is today for its two non-agent callers (bus `error` publish + `unbind`, then `persist_failure`), so `engine_failure_still_reaches_the_error_wire_name` (worker.rs ~1888) keeps passing and the graph `Err` branch and the corrupt-`fork_from` case gain the webhook. `run_agent`'s failure arm does NOT call `record_engine_failure`: it emits `RunFinished{Failed}` through the dispatcher (the ONE `error` wire event, `message: null` exactly like a graph run's engine-emitted failure), calls `persist_failure`, then waits the grace period and unbinds. The failure text stays available via `GET /runs/{id}` (`error` field). This yields exactly one terminal wire event per agent run without touching the graph path's accepted duplicate (documented on `record_engine_failure`).

### Pattern 4: `LogTraceSink` guard + reusable buffer (public surface unchanged)

```rust
thread_local! { static TRACE_BUF: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) }; }
const TRACE_BUF_RETAIN_MAX: usize = 64 * 1024;

fn write_trace_line<T: serde::Serialize>(value: &T) {
    TRACE_BUF.with(|cell| match cell.try_borrow_mut() {           // no panic path; re-entrancy falls back
        Ok(mut buf) => { write_into(&mut buf, value); if buf.capacity() > TRACE_BUF_RETAIN_MAX { *buf = Vec::new(); } }
        Err(_) => write_into(&mut Vec::new(), value),
    });
}
fn write_into<T: serde::Serialize>(buf: &mut Vec<u8>, value: &T) {
    buf.clear();
    match serde_json::to_writer(&mut *buf, value) {
        Ok(()) => match std::str::from_utf8(buf) {                 // serde_json emits UTF-8; no unsafe
            Ok(json) => log::info!(target: "paladin::trace", "{json}"),
            Err(error) => log::error!(target: "paladin::trace", "LogTraceSink produced non-UTF-8 JSON: {error}"),
        },
        Err(error) => log::error!(target: "paladin::trace", "LogTraceSink failed to serialize a TraceRecord: {error}"),
    }
}
// on_event: the enablement guard goes HERE, not in write_trace_line, so log_sink_never_returns_err_and_logs_diagnostic
// (which calls write_trace_line directly with an unserialisable value) keeps exercising the error path.
if !log::log_enabled!(target: "paladin::trace", log::Level::Info) { return Ok(()); }
```
A `thread_local!` adds no field, keeps `#[derive(Debug, Default, Clone, Copy)]` and the unit-struct literal `LogTraceSink`, so `make api-surface` is unchanged (`.project/current-exports.txt` lines 7657-7666 list a `Copy` unit struct with `new()`, `Clone`, `Default`, `Debug`). Adding any field, even private, breaks the unit-struct expression and drops `Copy`. The borrow is never held across an `.await` (`on_event` is `async fn` but `write_trace_line` is synchronous), so a task hopping threads is safe.

### Anti-Patterns to Avoid

- **Asserting a `RunStarted`-derived wire event.** `map_trace_event` returns `None` for `RunStarted`, `NodeProgress`, `EdgeEvaluated`, `WaypointSaved`, `FallbackHop`, `MiddlewareEvent` (events.rs 230-236). D-16's sibling test must assert `done` (and `node_started`/`node_finished` only if the pair is emitted).
- **Hand-editing `WINDOWS.md`.** The plan forbids it (40-06 precedent); use the tool (Pitfall 9).
- **`:latest` or a `preview`/`rc` RustFS tag**; **`command:` on the RustFS service** (unneeded; also removes the actionlint suppression need).
- **Reading the ETag as MD5** - composite for multipart; keep `md5_hash` as an opaque label.
- **Putting the enablement guard inside `write_trace_line`** (breaks the error-path test) or inside `TraceDispatcher` (would leak a sink's log config into the battalion crate).
- **Relying on RustFS's `MINIO_*` env compat shim** - it exists but leaves `MINIO` literals in live config and is not a documented contract.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Bucket bootstrap | An `rc`/`mc` init container or shell loop | `MinioAdapter::new` -> `ensure_bucket_exists` (after the `create_with_path_style` fix) | Exercised by the suite itself; one fewer pinned image |
| Multipart | A hand-rolled SigV4 multipart client | `rust-s3 0.35.1` `initiate_multipart_upload` / `put_multipart_chunk` / `complete_multipart_upload` / `abort_upload` | Already a dependency; verified against RustFS |
| Presigned URLs | Custom signing | `Bucket::presign_put` / `presign_get` | Already used; verified with real HTTP against RustFS |
| Webhook delivery / event mapping | A second enqueue path or a hand-published `done` | `webhook_delivery_for_outcome`, `map_trace_event`, `RunEventBusSink`, `build_run_sink` | House rule: one helper per rule (D-00e, D-14, D-15) |
| Ledger transitions | Editing `WINDOWS.md` by hand | `gsd-tools windows append|waive|fixed` | Hand edits forbidden (40-06); tool refuses `waived -> fixed` |
| JSON fast paths | `sonic-rs` / `simd-json` | `serde_json::to_writer` into a reused buffer | D-17: no new dependency |
| Trace-line enablement | A config flag mirror of the log filter | `log::log_enabled!(target: ..., Level::Info)` | Reads the operator's actual `RUST_LOG` |

**Key insight:** every piece this phase needs already exists in the tree or in `rust-s3`; the work is making the suite real, deleting bootstrap machinery, and routing the agent path through the existing shared helpers.

## Runtime State Inventory

MinIO -> RustFS is a migration, so all five categories are answered explicitly.

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | Named volumes `minio_data` (`docker/docker-compose.yml`) and `minio-data` (`.devcontainer/docker-compose.yml`) hold MinIO-format data; the default RustFS build cannot read MinIO on-disk data (`rio-v2` feature only; MinIO-SSE objects unreadable). Dev/test data only (test compose uses `tmpfs`). The anonymous-public policy on `paladin-files` set by `mc anonymous set public` lives in that volume | **Data migration: none** (dev data; state it). **Code/config edit:** use NEW volume names (`rustfs_data`, `rustfs-data`) so RustFS never mounts a MinIO-formatted volume; note in CHANGELOG/`k8s/README.md` that old volumes are not migrated and can be removed with `docker volume rm` |
| Live service config | None outside git: no UI-configured services. The k8s reference deployment reads `minio.endpoint` from `k8s/configmap.yaml` (in git) | Code edit: `configmap.yaml` `minio.endpoint: "paladin-rustfs:9000"` |
| OS-registered state | None - verified: no launchd/systemd/Task Scheduler references to MinIO in the repo (`git grep -i minio` finds only Docker/CI/docs/scripts) | None |
| Secrets / env vars | Compose interpolation `MINIO_ROOT_USER`/`MINIO_ROOT_PASSWORD` (+ MinIO-only `MINIO_BROWSER_REDIRECT_URL`) in `docker/docker-compose.yml`, `docker-compose.dev.yml`, `.env.example:158-161`, and written by the E2E job into `docker/.env` (`ci.yml:1899-1900`); k8s secret keys `MINIO_ROOT_USER`/`MINIO_ROOT_PASSWORD` (store side) in `ci.yml:1805-1806`, `k8s/minio.yaml`, `k8s/secret.yaml.example:34-35`; app-side `APP_MINIO_*`, `MINIO_ACCESS_KEY`/`MINIO_SECRET_KEY`, `TEST_MINIO_*` (D-04: unchanged). Developer `.env` files outside git may still carry the old names | Code edit: store-side names become `RUSTFS_ACCESS_KEY`/`RUSTFS_SECRET_KEY` (compose interpolation and the k8s secret keys `RUSTFS_ACCESS_KEY`/`RUSTFS_SECRET_KEY` mapped into the container env of the same name); keep the app-side pair; drop `MINIO_BROWSER_REDIRECT_URL`; document in CHANGELOG that a local `.env` must be renamed. Do NOT use the default value `rustfsadmin` (entrypoint warns) |
| Build artifacts / installed packages | `Cargo.lock` still lists `testcontainers-modules` after the `Cargo.toml` edit; no compiled artefact carries the old name | Refresh `Cargo.lock` in the same commit (`cargo metadata` / `cargo build --offline`); CI may use `--locked` |

**The canonical question:** after every repo file is updated, the runtime systems still holding the old string are (a) any developer's existing `minio_data`/`minio-data` volume and (b) any operator's stored `paladin-secrets` with `MINIO_ROOT_*` keys and `kubectl apply -f k8s/minio.yaml` muscle memory. Neither is migratable by code; both are called out in the CHANGELOG and `k8s/README.md`.

## MinIO Reference Inventory (live config = MUST change; historical = MUST NOT change)

**Live - image references and service-side env/health**
- `.github/workflows/ci.yml`: `integration-tests` job service block lines 697-723 (image 706; env 711-712; health 714; `command` 723; console port 9011); steps `Install MinIO Client` 743-753, `Setup MinIO buckets` 755-760, wait loop 773-778 (`/minio/health/live` at 775); `coverage` job service block 1409-1435 (image 1418; env 1423-1424; health 1426; `command` 1435), `Install MinIO Client` 1462-1472, `Setup MinIO buckets` 1474-1479, wait loop 1492-1497 (1494); `docker-integration` job 859-861 (`up -d redis-test minio-test minio-test-init` + `docker inspect paladin-minio-test-init` wait); smoke test secret 1805-1808, `kubectl apply -f k8s/minio.yaml` 1815, `kubectl wait -l app=minio` 1821, log dump 1878-1879; E2E `docker/.env` 1899-1900 and `curl -f .../minio/health/live` 1916 (comment 1915, prose 1923).
- `docker/docker-compose.test.yml`: `minio-test` 26-47 (image 27, `container_name` 28, env 33-35, `command` 37, `tmpfs` 38-39, health 42), `minio-test-init` 49-70 (image `quay.io/minio/mc:...` 49), `integration-tests` service `depends_on` 160-165 and `APP_MINIO_*` 171-174, command 176.
- `docker/docker-compose.yml`: `minio` 22-45 (image 23; env 28-30 incl. `MINIO_BROWSER_REDIRECT_URL`; `command` 31; volume 33; health 36), `minio-init` 46-70 (image 47; creates `paladin-files`, `paladin-analysis`, `paladin-reports`, `paladin-backups` and `mc anonymous set public minio/paladin-files`), `paladin-app` `depends_on` 83-88 and `APP_MINIO_*` 100-108, volume `minio_data` declaration further down.
- `docker/docker-compose.dev.yml`: `minio` env 9-16 (`MINIO_ROOT_*` overrides; no `image:` of its own - inherits) and `minio-init` env 17-21, `APP_MINIO_*` 34-35.
- `.devcontainer/docker-compose.yml`: service `minio` 90-108 (image 92, env 97-98, `command`, volume `minio-data` 100/138, health 105), `depends_on` 67, env 37-39 (app-side, unchanged); `.devcontainer/validate.sh:114-115` (`docker ps | grep -q minio`, `nc -z minio 9000`); `.devcontainer/devcontainer.json:11-12,28-32` (port labels).
- `k8s/minio.yaml` (whole file: image 24, `args` 25-28, env 29-38, probes 55-61 `/minio/health/live|ready`, Service `paladin-minio` 77); `k8s/deployment.yaml:58-68` (`wait-for-minio`, `nc -z paladin-minio 9000`) and env 120-129 (app side, unchanged); `k8s/configmap.yaml:55-57` (`minio.endpoint: "paladin-minio:9000"`); `k8s/secret.yaml.example:33-37`; `k8s/README.md:90-95, 279-293, 400`.
- `Makefile`: help text 50, 67, 113; `test-integration-minio` 164-167; `minio-console` 519-523; `storage-reset` 534-537 (`exec minio rm -rf /data/*`, `restart minio minio-init`); `health` 689-690; devcontainer `up -d redis minio mysql` 726; comment 332.
- `scripts/coverage.sh:53-95` (probe `/minio/health/live`, `minio:9000`, `minio` defaults `minioadmin`), `scripts/run_integration_tests.sh:191,246-253` (compose service names, health URL), `tests/integration/cli_real_services_test.rs:105` (`/minio/health/live` - RustFS aliases it, but change to `/health` so no MinIO URL remains), `.env.example:144-181`.
- `.github/actionlint.yaml:14-27` - the `command:` suppression and its `minio/minio` comment; after the swap **no `services.<id>.command` key remains anywhere** (`git grep "^\s*command:" .github/workflows` finds only ci.yml:723 and :1435), so delete the `paths:` block (keep `self-hosted-runner`).
- Root `Cargo.toml:244` (`testcontainers-modules ... features = ["minio"]`) and `tests/integration/file_storage_integration_tests.rs:6-7,16-21,79-105`.

**Live - docs that quote the pin, health URL, `mc` steps or the manifest path (docs-currency scope)**
`docs/src/appendix/integration-tests.md` (203 image row; 136, 231-232 service/container names), `docs/src/appendix/minio-file-repository-setup.md` (57, 364, 506, 820, 869-873 quote pin/health/compose; 115 MinIO mentions total; duplicated Quick Start at 26 and 475), `docs/src/contributing/testing-guide.md` (295-320 the `GenericImage` example; Code Coverage section 458-494), `docs/src/contributing/branching-model.md:49` (pin in the `ci.yml` row prose), `docs/src/deployment/cicd.md` (200, 205, 220), `docs/src/deployment/docker.md` (419-431), `docs/src/deployment/kubernetes.md` (35 `k8s/minio.yaml`; 884 `app: minio`), `docs/src/operations/troubleshooting.md:497`, `.devcontainer/CI-CD.md` (86, 93, 232, 327, 508), `.devcontainer/README.md` (29, 233, 249-250, 310-312), `QUICKSTART.md` (46, 82, 169), `FILES.md:51`, `SETUP_COMPLETE.md` (10, 50, 126), `docs/src/SUMMARY.md:115` (link text).

**Historical / MUST NOT change:** `CHANGELOG.md` history (`[0.10.x]` sections), `.planning/**` (phases, milestones, todos, research, intel), `.project/**`, `lcov.info`, `final-api.txt`/`api_surface_current.txt`/`.project/current-exports.txt` (`FileStorageConfig::minio_*` are unchanged public config fields per D-04/D-10). Rows 31/35 text in `WINDOWS.md` is tool-managed.

## Adapter and suite fix set (verified against RustFS 1.0.0)

Everything below was applied to a scratch copy of the tree, run against a fresh RustFS 1.0.0 data directory (so the adapter bootstrapped `integration-tests` itself), and reverted; the repo tree is untouched. Result: `test result: ok. 9 passed` (the 8 existing cases + one multipart lifecycle case).

| # | Defect (evidence) | Fix |
|---|-------------------|-----|
| 1 | `MinioAdapter::new` on a missing bucket -> `Bucket::create` (virtual-hosted) -> DNS failure; with `*.localhost` resolvable, RustFS returns `501 NotImplemented`. Real MinIO only worked because `mc` pre-created buckets | `Bucket::create_with_path_style` in `create_bucket` (minio.rs:163) |
| 2 | `copy_file`: `copy_source = "<bucket>/<key>"` -> rust-s3 builds `<bucket>/<bucket>/<key>` -> `404 NoSuchKey` (minio.rs:529-541) | pass `&source_object` (bare key) |
| 3 | `upload_part`/`complete_multipart_upload`/`abort_multipart_upload` return `Unknown("Multipart upload not fully implemented")` (minio.rs:910-938) | Pattern 1 |
| 4 | `TestEnvironment::check_service_availability` does `format!("{host}:{port}").parse::<SocketAddr>().unwrap()` (`tests/integration/mod.rs:138`) -> panics on `localhost`; also runs in `detect_external_services()` on any dev box without `CI` set, so local mode panics too | resolve with `(host, port).to_socket_addrs()` (take the first) and return `false` on error; no `unwrap` |
| 5 | `TestEnvironment::new` runs `env_logger::init()` (`mod.rs:95-97`), then `SystemLogAdapter::new` runs `env_logger::init()` again (`system_log_adapter.rs:127-155`) -> `SetLoggerError` panic in every case | drop the `SystemLogAdapter` from the suite and pass `None` to `MinioAdapter::new` (the log adapter is not under test) |
| 6 | `s3-storage` is not enabled by any CI command: `ci.yml:~790` (`cargo test file_storage_integration_tests --release -- --ignored`), `~817` (`--workspace --features integration-tests`), `scripts/coverage.sh:100` (`--features integration-tests,llm-all`), `docker-compose.test.yml:176`, E2E `~1924`. Verified with `cargo test --test lib --features integration-tests -- --list --ignored`: **zero** `file_storage` tests listed; with `,s3-storage` the 8 tests appear. `scripts/run_integration_tests.sh:321` alone passes `s3-storage` | add `--features s3-storage` (or `integration-tests,s3-storage`) to the `file_storage_integration_tests` steps in the Integration Tests job and the E2E step; decide `coverage.sh` separately (Pitfall 7) |
| 7 | Presigned test only asserts the URL contains the host and `println!`s the whole signed URL to CI logs | real `PUT` via the upload URL, real `GET` via the download URL (`reqwest` is a root dependency), and print only the URL without its query string |
| 8 | Suite has no multipart or ETag case | Multipart lifecycle + abort; ETag: quote-stripped equality between `get_file_info` and `list_files`, changes after overwrite, `!= content MD5` is NOT asserted for multipart |

Additional latent adapter quirks seen while reading (do not need fixing for the gate, but do not be surprised): `list_files` passes `limit` as rust-s3's **delimiter** argument (minio.rs:495) and `health_check` passes `Some("1")` as a delimiter; `list_files` reports `has_more = false` always. None affects the eight cases.

## Common Pitfalls

### Pitfall 1: D-05's premise is false until `create_with_path_style`
**What goes wrong:** `MinioAdapter::new` fails on a fresh RustFS with "dns error" (or `501 NotImplemented` where `*.localhost` resolves) and every CI job that no longer pre-creates buckets goes red.
**Why:** `Bucket::create` builds a virtual-hosted request (`bucket.host:port`); only `create_with_path_style` is path-style; the adapter's own bucket handle is path-style but `create_bucket` builds a separate one.
**How to avoid:** fix #1 above, and keep a unit/integration case that constructs the adapter against an empty store. **Warning signs:** `ConnectionError("Failed to create bucket: ... dns error")`.

### Pitfall 2: the contract suite is currently vacuous in CI
**What goes wrong:** "suite green" would pass with zero tests executed.
**How to avoid:** add the feature (fix #6) and make the job log the executed test names; a plan acceptance check should assert `--list` shows the 8+ cases with the CI feature set. **Warning signs:** `running 0 tests`.

### Pitfall 3: hostname endpoints panic the harness
`SocketAddr::from_str` accepts IP literals only. CI sets `TEST_REDIS_HOST=localhost`. Use `ToSocketAddrs`. (Also required for the Docker path: `rustfs-test:9000`.)

### Pitfall 4: double `env_logger::init()` (fix #5)
Any test that builds a `SystemLogAdapter` after `TestEnvironment::new()` panics. Remove the adapter from the suite.

### Pitfall 5: `copy_object_internal` already prefixes the bucket (fix #2)
Passing `"<bucket>/<key>"` yields a doubled prefix. Whether real MinIO tolerated it is unverified [ASSUMED it did not], but the bare key is correct for every S3 server.

### Pitfall 6: stdout `WaitFor` is useless for RustFS
The image logs to `/logs` at `warn`; stdout has only the entrypoint's `Starting: ...` echo printed *before* the server is ready. Use an HTTP poll of `/health/ready` (or the `http_wait` feature).

### Pitfall 7: adding `s3-storage` to the coverage command moves the 82 % denominator
It compiles `minio.rs` (1,198 lines) and the `s3-storage`-gated `src/config/setup/service_runner.rs` / `settings.rs` paths, which have their own coverage profile. The effect on the floor is **unmeasured** [ASSUMED neutral-to-positive]. Recommendation: put the suite in the Integration Tests job first (STORE-02 evidence), and change `scripts/coverage.sh` only with a CI-measured figure recorded in the SUMMARY; do not let the floor shift silently.

### Pitfall 8: `RunStarted` is not a wire event; `RunFinished{Failed}` carries `message: null`
D-16's sibling test must assert `done`. The graph path's `error` from `RunFinished` has `message: null`; the direct publish in `record_engine_failure` carries the text. Choose the agent path's parity with the engine-emitted form (Pattern 3) and say so in the rustdoc.

### Pitfall 9: the ledger tool cannot close rows 31 and 35
`gsd-tools windows fixed <id>` calls `assertOpen`; both rows are already `waived` (rows 31 and 35, `WINDOWS.md` lines 48 and 52), so it throws `WINDOWS_ALREADY_RESOLVED`. There is no amend/reopen verb (`status|append|waive|fixed` only, `gsd-tools.cjs:1507-1523`). Phase 40 plan 40-06 hit exactly this for row 32 and left it `waived`, recording "closing condition met" in CHANGELOG + ADR (40-06-SUMMARY deviation 3). CONTEXT's "closed through the ledger tool" therefore needs a decision. **Recommended:** follow the 40-06 precedent for row 31 (leave `waived`, record the met closing condition with the inverted test name in CHANGELOG, ADR-0055 or the SUMMARY); if the maintainer wants a `fixed` row, `gsd-tools windows append --kind deviation --phase 45 --description "Row 31 closing condition met by <test>/<commit>"` then `windows fixed <newid>`. For row 35's D-19 "amend" branch, the tool-sanctioned form is `windows append` (new row carrying the new measured figure and closing condition) followed by `windows waive <newid> "<reason + UAT acceptance>"`; never hand-edit. Also: row 35's `file` column points at the old `.planning/phases/28-.../28-BENCH-EVIDENCE.md`; the file now lives under `.planning/milestones/v0.10.0-phases/28-observability-tooling/`.

### Pitfall 10: there is no `make docs`
Docs gate = `.github/workflows/docs.yml` (`mdbook build docs/` + mdbook-linkcheck, CI-only; `mdbook` is not installed locally) plus `make check-doc-config` (fenced YAML must parse) and `make check-doc-examples`. `scripts/check-workflow-triggers.sh` cross-checks the `docs/src/contributing/branching-model.md` workflow table - edit only the prose cell (branching-model.md:49), not the trigger columns. The "Phase 34 docs-currency gate" has no dedicated script in the tree; treat "docs quote the live file" as a manual acceptance grep (see Validation Architecture).

### Pitfall 11: compose `tmpfs: /data` and the non-root user
RustFS runs as uid 10001. Docker tmpfs mounts default to mode 1777 [ASSUMED]; be explicit: `tmpfs: - /data:mode=1777`. Named volumes are populated from the image's pre-owned `/data`; bind mounts are not (chown to 10001).

### Pitfall 12: old MinIO named volumes
RustFS cannot read them; reuse of the old volume name would mount MinIO-format data. Rename volumes (Runtime State Inventory).

### Pitfall 13: dropping `minio-init` drops three unused buckets and a public-read policy
`paladin-analysis`, `paladin-reports`, `paladin-backups` are referenced by no code (only a real-S3 example in `sanctum-deployment.md:684`), and nothing depends on the anonymous `public` policy on `paladin-files` (`git grep` finds no reader). Dropping both is a security-positive change; record it under CHANGELOG `Removed`.

### Pitfall 14: the tracing bar is probably unreachable with the target enabled
Arithmetic: 3 % of the 110 microsecond baseline is 3.3 microseconds for ~22 records/run (`SuperstepStarted`, 8x`NodeStarted`, 8x`NodeFinished`, `DeltaMerged`, `WaypointSaved`?, `RunStarted`, `RunFinished`), i.e. ~150 ns per record including dispatch, clones, a `Mutex`, a doorbell `try_send`, an `async_trait` box, `catch_unwind` and (enabled) JSON serialisation of a ~250-byte record. That budget is below a typical `serde_json` cost for such a record [ASSUMED], so plan for the D-19 "amend" branch, and treat the guard's benefit as visible only in the target-disabled row. The `none` variant is the only variant with `inner: None` (no queue, no consumer task, no clones). A per-run `tokio::spawn` of the consumer task is also a fixed cost inside the 110 microsecond window.

### Pitfall 15: the bench installs no logger
`benches/engine_benchmarks.rs` never calls `log::set_logger`, so `log::max_level()` is `Off`: Phase 28's `log_sink` +22 % was serialisation whose output was discarded by the `log!` macro's static check, i.e. the "target disabled but still serialising" case. D-18's "enabled" measurement requires installing a logger (see Code Examples). To keep run-to-run comparability keep the original bench IDs for the enabled rows and add distinctly named target-off rows.

## Code Examples

### GitHub Actions service block (replaces both `minio:` blocks; no `command:`)
```yaml
      rustfs:
        # Pinned rather than floating: the community MinIO images are gone (Docker Hub deleted them 2026-09-12,
        # quay.io locked anonymous pulls 2026-09-24). RustFS is Apache-2.0 and multi-arch.
        # Manifest-list digest: sha256:8cc9801755448b71a786705ce76692c77e14936cccd87cf2fc31842e58f4d1ff
        image: rustfs/rustfs:1.0.0
        ports:
          - 9010:9000
        env:
          RUSTFS_ACCESS_KEY: testuser
          RUSTFS_SECRET_KEY: testpass123
          RUSTFS_CONSOLE_ENABLE: "false"
        options: >-
          --health-cmd "curl -f http://localhost:9000/health/ready"
          --health-interval 5s
          --health-timeout 5s
          --health-retries 12
          --health-start-period 5s
```
Wait loops become `curl -f http://localhost:9010/health/ready`; the `Install MinIO Client` and `Setup MinIO buckets` steps are deleted. (CI-attributed: container start under Actions, `/data` ownership, `curl` in the image.)

### Compose test service (`docker/docker-compose.test.yml`)
```yaml
  rustfs-test:
    image: rustfs/rustfs:1.0.0   # digest sha256:8cc98017...f1ff in a comment, same as ci.yml
    container_name: paladin-rustfs-test
    ports: ["9010:9000"]
    environment:
      RUSTFS_ACCESS_KEY: testuser
      RUSTFS_SECRET_KEY: testpass123
      RUSTFS_CONSOLE_ENABLE: "false"
    tmpfs:
      - /data:mode=1777
    networks: [paladin-test-network]
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/health/ready"]
      interval: 5s
      timeout: 3s
      retries: 10
      start_period: 5s
```
The `integration-tests` service then `depends_on: rustfs-test: {condition: service_healthy}` only (no `-init`), and if the suite runs inside compose it needs `USE_EXTERNAL_TEST_SERVICES=true`, `TEST_REDIS_HOST=redis-test`, `TEST_REDIS_PORT=6379`, `TEST_MINIO_ENDPOINT=rustfs-test:9000` plus keys (harness hostname fix #4 is a prerequisite). Renaming services (`minio-test` -> `rustfs-test`, `minio` -> `rustfs`) is recommended for honesty; the full reader list is the inventory above (Makefile 536-537/726, `ci.yml` 859-861, `run_integration_tests.sh:191`, `coverage.sh:81-83`, `validate.sh:114-115`, docs). Dev compose (`docker/docker-compose.yml`): keep the console mapping (`make minio-console` stays as a target name pointing at `http://localhost:9001/rustfs/console/index.html`), drop `minio-init` and the `paladin-app` `depends_on: minio-init`, volume `rustfs_data:/data`.

### Kubernetes (`k8s/rustfs.yaml`, shape only)
```yaml
      containers:
        - name: rustfs
          image: rustfs/rustfs:1.0.0     # digest comment
          env:
            - { name: RUSTFS_ACCESS_KEY, valueFrom: { secretKeyRef: { name: paladin-secrets, key: RUSTFS_ACCESS_KEY } } }
            - { name: RUSTFS_SECRET_KEY, valueFrom: { secretKeyRef: { name: paladin-secrets, key: RUSTFS_SECRET_KEY } } }
          ports: [ {name: api, containerPort: 9000}, {name: console, containerPort: 9001} ]
          livenessProbe:  { httpGet: { path: /health,       port: api }, initialDelaySeconds: 10, periodSeconds: 30 }
          readinessProbe: { httpGet: { path: /health/ready, port: api }, initialDelaySeconds: 5,  periodSeconds: 10 }
          volumeMounts: [ { name: data, mountPath: /data } ]     # emptyDir sizeLimit 10Gi as today
```
Service `paladin-rustfs` (ports 9000/9001), labels `app: rustfs`, `component: storage`; no `args` (entrypoint default). For the reference deployment consider `RUSTFS_CONSOLE_ENABLE: "false"` (smaller surface; image default CORS origin is `*`) - a documented choice for ADR-0055, not a requirement. `wait-for-minio` init container becomes `wait-for-rustfs` with `nc -z paladin-rustfs 9000` (Alpine/busybox `nc -z` is what it uses today).

### Local-mode `GenericImage` (replaces `MinIO::default()`; D-07)
```rust
use testcontainers::{GenericImage, ImageExt, core::IntoContainerPort, runners::AsyncRunner};

let container = GenericImage::new("rustfs/rustfs", "1.0.0")
    .with_exposed_port(9000.tcp())
    .with_env_var("RUSTFS_ACCESS_KEY", env.minio_access_key.clone())
    .with_env_var("RUSTFS_SECRET_KEY", env.minio_secret_key.clone())
    .with_env_var("RUSTFS_CONSOLE_ENABLE", "false")
    .start()
    .await?;
let port = container.get_host_port_ipv4(9000).await?;
env.minio_endpoint = format!("localhost:{port}");
wait_until_ready(&format!("http://{}/health/ready", env.minio_endpoint), Duration::from_secs(60)).await?; // reqwest poll, no fixed sleep
```
`GenericImage::new(name, tag)` takes two `Into<String>`s of the same type; `ImageExt` must be imported for `with_env_var`. The suite's local credentials are currently `minioadmin`/`minioadmin` (`mod.rs:174-181`): keep them coherent by feeding the env vars from `env`, and prefer a neutral literal (`testuser`/`testpass123`) over `rustfsadmin` (entrypoint warns on the default). Remove `testcontainers-modules` from `[dev-dependencies]` (and its `use`), update `docs/src/contributing/testing-guide.md:295-320` to the same image/tag/env/health-poll.

### Bench logger for the enabled rows (D-18)
```rust
struct DiscardLogger;
impl log::Log for DiscardLogger {
    fn enabled(&self, m: &log::Metadata) -> bool { m.level() <= log::Level::Info }
    fn log(&self, r: &log::Record) {                 // format like a real logger would, discard the bytes
        if self.enabled(r.metadata()) { use std::io::Write; let _ = write!(std::io::sink(), "{}", r.args()); }
    }
    fn flush(&self) {}
}
static LOGGER: DiscardLogger = DiscardLogger;
fn set_trace_target(enabled: bool) {
    let _ = log::set_logger(&LOGGER);                // once per process; ignore "already set"
    log::set_max_level(if enabled { log::LevelFilter::Info } else { log::LevelFilter::Off });
}
```
Call `set_trace_target(true)` before the existing three IDs and `set_trace_target(false)` before three new `..._target_off` IDs (criterion runs `bench_function`s sequentially). `root [dependencies] log` is already available to benches; no fixture change (`build_width_graph(8)`).

### Cheap `CompositeSink` improvement (paladin-ports, internal)
```rust
// in on_event: clone for every child except the last, hand the owned record to the last one
let last = self.sinks.len() - 1;
for (i, sink) in self.sinks.iter().enumerate() {
    let rec = if i == last { record_owned_take() } else { record.clone() };   // e.g. Option<TraceRecord>::take()
    ...
}
```
The `thread_id`/`run_id` `String` clones in `TraceDispatcher::emit` are **not** avoidable without changing `TraceRecord`'s field types (owned `ThreadId(String)`/`RunId(String)`); the `usage.clone()` is stack-only (`TokenUsage` is plain integers/`Option<u32>`; `AddAssign` takes ownership). The deep `record.clone()` per `CompositeSink` child is the avoidable allocation.

### Ledger commands (when the planner decides to use them)
```bash
gsd-tools windows append --kind deviation --phase 45 --description "..."   # returns the new id
gsd-tools windows fixed <new-id>                                          # or: windows waive <new-id> "<reason>"
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `minio/minio` community image on Docker Hub / quay pin | `rustfs/rustfs:1.0.0` exact tag | Docker Hub deletion 2026-09-12; quay lockout 2026-09-24; RustFS 1.0.0 GA 2026-09-16 | The whole phase |
| `mc` binary/`quay.io/minio/mc` for bucket creation | Adapter-owned `Bucket::create_with_path_style` | this phase | One fewer pinned image; bootstrap exercised by the suite |
| MinIO `/minio/health/live` | RustFS `/health`, `/health/ready` (MinIO aliases still served) | RustFS 1.0.0 | Live/ready split preserved for k8s |
| MinIO env `MINIO_ROOT_USER/PASSWORD` | `RUSTFS_ACCESS_KEY/RUSTFS_SECRET_KEY` | RustFS 1.0.0 | Store-side only; app-side names unchanged (D-04) |

**Deprecated/outdated:** `testcontainers_modules::minio::MinIO` (pins `RELEASE.2025-02-28...` inside the crate) - removed; the `actionlint` `command:` suppression - dead after the swap.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Real MinIO also failed the double-prefixed `copy_object_internal` (so the copy defect is pre-existing, not RustFS-specific) | Pitfall 5 | Low; the bare-key fix is correct either way |
| A2 | Docker tmpfs mounts default to mode 1777, so uid 10001 can write `/data` on `tmpfs` | Pitfall 11 | Compose test service would fail to start; mitigated by the explicit `mode=1777` |
| A3 | RustFS behaves identically in the Docker image and as the native release binary I ran (the Dockerfile packages that zip) | RustFS facts | Low; container start/health/entrypoint are still CI-attributed |
| A4 | A <= 3 % bar is unreachable with the target enabled (per-record budget ~150 ns) | Pitfall 14 | Plan would over-invest in micro-optimisation; the measurement decides regardless |
| A5 | Adding `s3-storage` to `scripts/coverage.sh` is neutral-to-positive for the 82 % floor | Pitfall 7 | Coverage job could drop below the floor; mitigated by making that change separate and CI-measured |
| A6 | `serde_json` cost for a ~250-byte record exceeds the per-record budget | Pitfall 14 | Same as A4 |
| A7 | GH Actions service containers start `rustfs/rustfs:1.0.0` correctly with no `command:` (entrypoint default) | Code Examples | CI red on first push; fallback: `command: rustfs /data` |
| A8 | Docker Hub tag `1.0.0` is immutable; the digest comment is the audit trail if it is ever repushed | Standard Stack | Low |

## Open Questions (RESOLVED)

All six questions were resolved at planning time (2026-09-29); each question below carries a resolution line naming the plan and decision that settles it.

1. **Ledger closure form for rows 31/35** (Pitfall 9)
   - Known: `windows fixed` refuses `waived` rows; Phase 40 left row 32 `waived` and recorded the met condition elsewhere; CONTEXT D-16/D-19 say "closed/amended through the ledger tool".
   - Unclear: whether the maintainer wants an appended-then-fixed closure row or the 40-06 precedent.
   - Recommendation: default to the 40-06 precedent; offer the append+fix row as the optional tool-sanctioned alternative; for the D-19 amend branch use append + waive.
   - RESOLVED: the 40-06 precedent plus an appended-then-fixed closure row, never a hand edit (D-16, D-19). Plan 45-02 leaves row 31 `waived` and closes it with one `gsd-tools windows append` row that is then `windows fixed`; plan 45-07 leaves row 35 `waived` and appends one row: `fixed` on the D-19 meet branch, left open on the amend branch until the maintainer's UAT acceptance, then `windows waive` (append + waive).

2. **Should `scripts/coverage.sh` gain `s3-storage` in this phase?** (Pitfall 7) - Recommendation: no; STORE-02's evidence comes from the Integration Tests job, and the coverage change needs its own CI-measured figure.
   - RESOLVED: no. Plan 45-04 (Task 1 step 3) leaves `s3-storage` out of the Coverage job and records the choice in its SUMMARY; plan 45-05 (Task 3) keeps `scripts/coverage.sh`'s `exec cargo llvm-cov` line byte-identical, and both plans gate `grep -c "s3-storage" scripts/coverage.sh` at 0.

3. **Rename compose service names (`minio` -> `rustfs`)?** - Recommendation: yes (honest naming; D-11 already renames k8s), applying the reader list above in one commit; the alternative (keep names, change image only) satisfies "no MinIO image" but leaves a misleading `minio` service running RustFS.
   - RESOLVED: yes. Plan 45-04 (Task 2) renames the test-compose service to `rustfs-test` (container `paladin-rustfs-test`); plan 45-05 (Tasks 1-2) renames the dev-compose and devcontainer services to `rustfs` (container `paladin-rustfs`), with every reader updated in the same plan.

4. **`RUSTFS_CONSOLE_ENABLE=false` in the k8s reference manifest?** - Recommendation: disable in CI/test blocks; leave the console on in the dev compose; for k8s pick per ADR-0055 (smaller surface vs. operator convenience).
   - RESOLVED: console OFF (`RUSTFS_CONSOLE_ENABLE: "false"`) in CI, the test compose and `k8s/rustfs.yaml` (plan 45-04, Tasks 1-3; the k8s Service exposes the API port only); console ON in the dev compose and the devcontainer (plan 45-05, Tasks 1-2) for operator convenience; ADR-0055 (plan 45-06) records the choice.

5. **Emit a `NodeStarted`/`NodeFinished` pair for the agent call?** - Recommendation: yes; without it `dispatcher.total_usage()/total_cost()` are zero (they only sum `NodeFinished`), so the wire `done.usage`, the Herald summary and `run_traces` would misreport an agent run's spend.
   - RESOLVED: yes. Plan 45-02 (Task 1) has `run_agent` emit `RunStarted` -> `NodeStarted` -> `NodeFinished` -> `RunFinished` (seq 1..=4, no gap), with `NodeFinished` carrying the `PaladinResult` usage and cost so `total_usage()`/`total_cost()` are non-zero (D-15).

6. **Executor sandbox for the benchmark:** the researcher sandbox had ~1-2 GB free disk (98 % used) and 4 cores, too small for a release bench build; `45-BENCH-EVIDENCE.md` must be produced where a release `cargo bench` can run (maintainer machine or a CI dispatch) and the machine block recorded as Phase 28 did.
   - RESOLVED: plan 45-03 builds the three commit points and the `45-BENCH-EVIDENCE.md` skeleton in the sandbox (criterion test mode only, no release bench); plan 45-07 Task 1 is a `checkpoint:human-action` (`gate="blocking-human"`) where the maintainer runs the release bench at points A/B/C on one machine in one session and pastes the output and machine block (D-18).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Docker daemon | compose/k8s/testcontainers verification | No (client 29.3.1 present, no daemon; registry pulls 403) | - | Native RustFS release binary for the suite loop; container specifics are CI-attributed |
| Native `rustfs` 1.0.0 binary | Local suite red/green | Yes (downloadable ~186 MB from GitHub releases; ran fine, ~175 MB RSS) | 1.0.0 | - |
| Rust toolchain / cargo | all code work | Yes | 1.97.1 | - |
| `cargo-public-api` | `make api-surface` | Yes | 0.52.0 | - |
| `cargo-llvm-cov` | coverage floor | No | - | CI-attributed (coverage is CI-attributed per STATE.md) |
| `mdbook`, `actionlint`, `kubectl`, `yamllint` | docs build, workflow lint, k8s apply | No | - | CI jobs; locally `python3` + PyYAML 6.0.1 for `make check-doc-config` and YAML parse of compose/k8s files |
| Redis | `TestEnvironment` external mode waits on `TEST_REDIS_HOST:PORT` | No | - | A TCP listener on 6380 is enough for the suite's reachability check |
| Disk | benchmark release build | ~1-2 GB free at research time | - | Run the bench on the maintainer machine/CI |

**Missing dependencies with no fallback:** none blocking planning.
**Missing dependencies with fallback:** everything above; the CI-attributed set is: container start/health under Actions and compose, k8s smoke, `mdbook build`, actionlint, coverage floor.

## Validation Architecture

Nyquist validation is enabled (`workflow.nyquist_validation` absent in `.planning/config.json`).

### Test Framework

| Property | Value |
|----------|-------|
| Framework | `cargo test` (libtest + `tokio::test`), `criterion` 0.5 for the bench, shell/`python3` grep gates |
| Config file | root `Cargo.toml` (`s3-storage`, `integration-tests` features; `[[bench]] engine_benchmarks harness = false`); `tests/lib.rs` is the `lib` integration target that includes `tests/integration/mod.rs` |
| Quick run command | `cargo test --lib services::run::worker` (worker + `worker_tests`); `cargo test --lib telemetry::log_sink` |
| Full suite command | `cargo test` then `make clean-code` and `make api-surface` (per CLAUDE.md), `make security` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| STORE-01 | No MinIO image/`mc` in live config; RustFS pinned to exact tag; services healthy | static grep gate + CI | `git grep -n -i "quay.io/minio\|minio/minio\|minio/mc\|MC_RELEASE\|mc alias\|MINIO_ROOT_" -- .github docker k8s .devcontainer scripts Makefile .env.example` returns nothing; `git grep -n "rustfs/rustfs:" -- .github docker k8s .devcontainer tests docs` shows one tag everywhere; YAML parse of compose/k8s via `python3 -c "import yaml,sys;[list(yaml.safe_load_all(open(f))) for f in sys.argv[1:]]" ...`; jobs Coverage / Integration Tests / Docker Integration Tests / Kubernetes Smoke Test green (CI-attributed, run ids recorded in UAT) | Wave 0 (gate script inline in plan acceptance) |
| STORE-01 | Bucket bootstrap by the adapter on an empty store | integration | `TEST_MINIO_ENDPOINT=127.0.0.1:9010 ... cargo test --test lib --features integration-tests,s3-storage file_storage_integration_tests -- --ignored --test-threads=1` against a fresh native RustFS data dir | Needs the harness fixes |
| STORE-02 | 8 existing + multipart + ETag + real presign cases pass on RustFS | integration (`#[ignore]`) | same command; local via native binary, authoritative run in CI **Integration Tests** job with `--features ...s3-storage` | Partly Wave 0 (new cases, harness fixes) |
| STORE-02 | The suite is actually compiled/executed in CI | listing check | `cargo test --test lib --features integration-tests,s3-storage -- --list --ignored \| grep -c file_storage_integration_tests` >= 10 (and the CI step's own log shows non-zero `running N tests`) | Wave 0 |
| STORE-02 | Adapter unit behaviour of `split_token` (round trip, malformed, forged key with `..`) | unit | `cargo test -p paladin-storage --features s3 minio` | Wave 0 (new unit tests in `minio.rs`) |
| STORE-03 | ADR-0055 exists, PROMOTION.md advanced to 0056, docs cite it, production-manifest decision stated | file/grep | `ls .planning/decisions/0055-*.md`; `grep -n "Next free ADR number: 0056" .planning/decisions/PROMOTION.md`; `grep -n "ADR-0055" docs/src/appendix/minio-file-repository-setup.md`; `make check-doc-config`; `mdbook build docs/` (CI `docs.yml`) | Wave 0 |
| PLAT-08 | Agent run with webhook enqueues one `Pending` `Completed` delivery (payload run id, status, assistant) | unit | `cargo test --lib services::run::worker_tests::agent_kind_run_with_a_webhook_enqueues_a_delivery` | Invert existing test (worker_tests.rs:939) |
| PLAT-08 | Agent run SSE subscriber receives `done` (Live), no second terminal event; `node_started/finished` if emitted | unit | `cargo test --lib services::run::worker_tests::agent_kind_run_streams_done_live` and `..._emits_exactly_one_terminal_event` (failing port variant) | Wave 0 (new) |
| PLAT-08 | Graph run failing through `record_engine_failure` enqueues exactly one `Failed` delivery (corrupt `fork_from` vehicle) | unit | `cargo test --lib services::run::worker_tests::graph_engine_failure_enqueues_failed_delivery` | Wave 0 (new) |
| PLAT-08 | `RunFinished{total_supersteps:0}` + fingerprint `"agent"` tolerated by sinks | unit | existing `otel_sink` test (line ~700) already covers `total_supersteps: 0`; add a `PersistingTraceSink`/`HeraldTraceSink` agent-shaped record test | Wave 0 (small) |
| OBS-05 | `LogTraceSink::on_event` skips serialisation when the target is filtered | unit | `cargo test --lib telemetry::log_sink` (new test: capturing logger with `max_level` Off / target filtered -> no serde call, e.g. a value whose `Serialize` panics or counts) plus the existing `log_sink_never_returns_err_and_logs_diagnostic` unchanged | Wave 0 (new) |
| OBS-05 | Buffer reuse does not change output bytes or truncate/leak across records | unit | new test: two consecutive records produce two exact JSON lines; oversize record then small record | Wave 0 (new) |
| OBS-05 | Gapless `seq` and `Replay` mode intact (hard gate, D-00f) | existing suites | `cargo test -p paladin-battalion` (dispatcher/hook tests) and `cargo test --lib services::run::events services::run::stream` | Exists |
| OBS-05 | Overhead re-measured; <= 3 % or new figure recorded | benchmark (NOT sandbox-runnable) | `cargo bench --bench engine_benchmarks -- bench_superstep_cost --warm-up-time 1 --measurement-time 3` before/after each fix, on a machine with a release build; output pasted into `45-BENCH-EVIDENCE.md` | Evidence file Wave 0 |
| OBS-05 | Public surface unchanged | gate | `make api-surface` (expects "API surface unchanged": thread-local buffer keeps `Copy`) | Exists |

### Sampling Rate
- **Per task commit:** the targeted `cargo test --lib ...` filter for the file touched, `cargo fmt --check`, `cargo clippy -p <crate> -- -D warnings`.
- **Per wave merge:** `cargo test`, `make clean-code`, `make api-surface`, `make security`, `make check-doc-config check-doc-examples check-api-examples`.
- **Phase gate:** CI green on the four named jobs plus Coverage floor (CI-attributed), then `/gsd-verify-work`; afterwards `/gsd-verify-work 40` (D-02).

### CI-attributed evidence (sandbox has no Docker)
Record the CI run id for: RustFS service containers healthy and the suite executing non-zero tests (Integration Tests job), Docker Integration Tests job and E2E (main-only) job with the compose RustFS service, Kubernetes Smoke Test with `k8s/rustfs.yaml` (`kubectl wait -l app=rustfs`), Coverage job (floor + timing), `actionlint`, `mdbook build`/linkcheck, and `make api-surface` on the CI toolchain. Locally verifiable without Docker: all worker/agent tests, the log-sink tests, `split_token` unit tests, the YAML parse and grep gates, and the full contract suite against the native RustFS binary.

### Wave 0 Gaps
- [ ] Harness fixes in `tests/integration/mod.rs` (`ToSocketAddrs`) and `file_storage_integration_tests.rs` (no `SystemLogAdapter`, `GenericImage` local mode, multipart/ETag/presign cases, no signed-URL printing)
- [ ] `minio.rs` fixes + `split_token` unit tests
- [ ] CI command feature flags (`s3-storage`) and the "tests actually ran" check
- [ ] New worker tests (agent SSE, terminal-once, graph-Err webhook), log-sink guard/buffer tests
- [ ] `45-BENCH-EVIDENCE.md` skeleton and the bench logger rows

## Security Domain

`security_enforcement` is not set to `false` in `.planning/config.json` (absent = enabled).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | yes (object-store root credentials) | Throwaway CI/test literals; never the RustFS default `rustfsadmin` (entrypoint warns); no real credential in any log/CI output (D-00a); k8s reference reads them from `paladin-secrets` |
| V3 Session Management | no | - |
| V4 Access Control | yes | Drop the anonymous `public` bucket policy previously set by `mc anonymous set public`; run containers as the image's non-root uid 10001; console disabled where not needed (`RUSTFS_CONSOLE_ENABLE=false`); image default console CORS origin is `*` |
| V5 Input Validation | yes | The multipart upload-id token is caller-supplied: `split_token` must reject malformed tokens and the decoded key must pass `validate_path` again (path traversal, e.g. `../`) |
| V6 Cryptography | no new use | SigV4 signing stays in `rust-s3`; do not hand-roll |
| V7 Error Handling and Logging | yes | Do not print presigned URLs (they embed the access key id and a signature valid for the URL's TTL) to CI logs; keep redact-then-truncate for any embedded S3 error body (`security.instructions.md`); the new `LogTraceSink` error paths log only the serializer error |
| V10 Malicious Code / Supply Chain | yes | Exact tag + digest comment; the checksum-verified `mc` download and a third-party image are removed; no new crate; `deny.toml` needs no change |
| V14 Configuration | yes | Explicit credentials in every block; no `:latest`; document that the k8s reference is a single-node `emptyDir` store |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Forged multipart upload id pointing at another key | Tampering | Re-validate the decoded key with `validate_path`; the port already grants arbitrary-key writes, so this adds no new authority but must not bypass the traversal check |
| Credentials or signed URLs in CI logs | Information disclosure | Throwaway literals only; strip the query string from printed presigned URLs; no `set -x` around credential steps |
| Unpinned/moving image tag | Tampering / supply chain | Exact tag + digest comment; deliberate re-pin only |
| Anonymous-readable dev bucket | Information disclosure | Removed with `minio-init` (Pitfall 13) |
| Console exposed with wildcard CORS | Elevation / disclosure | Disable console in CI/test/k8s reference, or document it (ADR-0055) |
| Webhook SSRF via the newly wired agent path | Spoofing / SSRF | Unchanged guard at submit and send time; the agent path enqueues through the same `webhook_delivery_for_outcome` -> `WebhookDeliveryService` |
| Trace line leaking values | Information disclosure | Unchanged: `DeltaMerged` carries names only by default (D-05); the guard/buffer change touches no field content |

The manual credential-handling review (CodeQL is advisory; no Snyk) applies to `minio.rs` (credentials handling unchanged; verify no new log line interpolates a key), the CI YAML (secret literals), and the new tests (signed URLs).

## Sources

### Primary (HIGH confidence)
- RustFS 1.0.0 source tree (`git clone --branch 1.0.0`): `Dockerfile`, `entrypoint.sh`, `docker-compose.yml`, `README.md`, `docs/architecture/s3-compatibility-matrix.md`, `rustfs/src/server/health.rs`, `prefix.rs`, `startup_preflight.rs`, `crates/ecstore/src/set_disk/{mod.rs,ops/multipart.rs}`, `crates/credentials`, `crates/config`
- Native execution of the RustFS 1.0.0 release binary (`github.com/rustfs/rustfs/releases/download/1.0.0/rustfs-linux-x86_64-musl-v1.0.0.zip`) against the repo's `MinioAdapter`, `rust-s3 0.35.1` and the repo's contract suite (scratch copy, reverted)
- Docker Hub v2 API (`hub.docker.com/v2/repositories/rustfs/rustfs[/tags]`, `rustfs/rc`): tags, digests, architectures, pull counts, dates
- `~/.cargo/registry/src/*/rust-s3-0.35.1` (`bucket.rs`, `serde_types.rs`, `request/tokio_backend.rs`, `bucket_ops.rs`), `testcontainers-0.24.0`, `testcontainers-modules-0.12.1`, `log-0.4.30`
- Repo files read in full or in the cited ranges: `crates/paladin-storage/src/minio.rs`, `tests/integration/{mod.rs,file_storage_integration_tests.rs}`, `src/application/services/run/{worker.rs,events.rs,worker_tests.rs}`, `src/infrastructure/telemetry/*`, `crates/paladin-battalion/src/engine/hooks.rs`, `crates/paladin-ports/src/output/trace_sink_port.rs`, `crates/paladin-core/src/platform/container/trace.rs`, `benches/engine_benchmarks.rs`, `.github/workflows/ci.yml`, `docker/*.yml`, `k8s/*`, `.devcontainer/*`, `Makefile`, `scripts/*`, `.planning/{WINDOWS.md,decisions/PROMOTION.md,decisions/0054-*.md}`, `28-BENCH-EVIDENCE.md`, `40-06-SUMMARY.md`
- `/workspace/.claude/gsd-core/bin/lib/broken-windows.cjs` (ledger verbs and `assertOpen`)

### Secondary (MEDIUM confidence)
- `github.com/rustfs/rustfs/releases` page (GA status of 1.0.0, previews after it) via WebFetch summary

### Tertiary (LOW confidence)
- Assumptions A2, A4-A7 above (Docker tmpfs mode, tracing arithmetic, coverage effect, Actions service start without `command:`)

## Metadata

**Confidence breakdown:**
- Standard stack (RustFS behaviour, `rust-s3` calls): HIGH - executed against the real binary and read from source
- Architecture (agent wiring, sink tolerance, single terminal event): HIGH for the code reading; MEDIUM until the tests exist
- Pitfalls: HIGH for Pitfalls 1-6, 8-10, 12-13, 15 (reproduced or read); MEDIUM for 7, 11, 14 (predicted)
- Container-runtime behaviour (compose/Actions/k8s): MEDIUM - CI-attributed, cannot be run in the sandbox

**Research date:** 2026-09-29
**Valid until:** 2026-10-06 for the image tag (RustFS pushes previews near-daily; re-run the tag query at execution and take the newest GA); 30 days for the repo findings
