# Phase 45: RustFS Swap & Platform/Observability Deviations - Pattern Map

**Mapped:** 2026-09-29
**Files analyzed:** 22 (new + modified)
**Analogs found:** 20 / 22 (two are pure edits with no analog needed)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `.planning/decisions/0055-*.md` | ADR doc | n/a | `.planning/decisions/0054-tenant-scoped-run-reads.md` | exact |
| `.planning/decisions/PROMOTION.md` | index | n/a | its own line 74 (0054 row) + line 76 "Next free ADR number" + dated notes at 78/85 | exact |
| `k8s/rustfs.yaml` (new; replaces `k8s/minio.yaml`) | k8s manifest | request-response | `k8s/minio.yaml` (Deployment+Service, probes, secretKeyRef) | exact |
| `45-BENCH-EVIDENCE.md` | evidence doc | batch | `.planning/milestones/v0.10.0-phases/28-observability-tooling/28-BENCH-EVIDENCE.md` | exact |
| `crates/paladin-storage/src/minio.rs` | adapter | file-I/O | itself: `create_multipart_upload` (~892) plus the stubs (910-940) | exact |
| `tests/integration/file_storage_integration_tests.rs` | test | file-I/O | itself, plus `tests/integration/redis_queue_integration_test.rs:60-90` | exact |
| `tests/integration/mod.rs` | test harness | n/a | its own header comment and `#[cfg(feature)]` mod gates | role-match |
| `src/application/services/run/worker.rs` (`run_agent`) | service | event-driven | `run_once` sink assembly (987-1050) and webhook enqueue (1160-1185) | exact |
| `src/application/services/run/worker_tests.rs` | test | event-driven | `agent_kind_run_with_a_webhook_enqueues_no_delivery` (865-975) | exact |
| `src/application/services/run/stream_tests.rs` | test | streaming | `live_stream_yields_progress_then_done` (234) and `terminal_run_with_rows_replays` (778) | role-match |
| `src/infrastructure/telemetry/log_sink.rs` (+tests) | sink | streaming | itself (`write_trace_line` 40-56, test module 65+) | exact |
| `crates/paladin-ports/src/output/trace_sink_port.rs` | port and composite | streaming | itself (`CompositeSink` 147-190) | exact |
| `crates/paladin-battalion/src/engine/hooks.rs` (`TraceDispatcher::emit`) | dispatcher | streaming | itself (305+) | exact |
| `benches/engine_benchmarks.rs` | bench | batch | `bench_superstep_cost_sink_variants` (318) | exact |
| `.github/workflows/ci.yml` | CI | n/a | its own `minio:` service blocks (697-790, 859-861, 1409+) | exact |
| `docker/docker-compose.test.yml`, `docker/docker-compose.yml`, `docker/docker-compose.dev.yml`, `.devcontainer/docker-compose.yml` | compose | n/a | the current `minio*` services in each | exact |
| `k8s/deployment.yaml`, `k8s/configmap.yaml`, `Makefile`, root `Cargo.toml` (line 244), docs and CHANGELOG | config/docs | n/a | in-place edits | n/a |

## Pattern Assignments

### `crates/paladin-storage/src/minio.rs` (adapter, file-I/O)

**Analog:** the same file. `create_multipart_upload` already works (rust-s3 `initiate_multipart_upload`). The three other multipart methods are stubs returning `FileStorageError::Unknown("Multipart upload not fully implemented with rust-s3")`. Trait signatures to keep (lines ~892-940):
```rust
async fn create_multipart_upload(&self, path: &Path, _options: Option<UploadOptions>) -> FileStorageResult<String>
async fn upload_part(&self, _upload_id: &str, _part_number: u32, _content: &[u8]) -> FileStorageResult<String>
async fn complete_multipart_upload(&self, _upload_id: &str, _parts: Vec<(u32, String)>) -> FileStorageResult<FileItem>
async fn abort_multipart_upload(&self, _upload_id: &str) -> FileStorageResult<()>
```
Error idiom to copy (line ~885-890):
```rust
.map_err(|e| FileStorageError::IoError(format!("Failed to initiate multipart upload: {}", e)))?;
```
Gap for the planner: `upload_part` and the two below it receive only `upload_id`, not the object path. rust-s3 needs the path for `put_multipart_chunk` / `complete_multipart_upload` / `abort_upload`. The port trait has no path, so the "stateless token helper" idea is to encode the path into the returned upload id. No existing token helper in the crate was found (the closest small helper is `path_to_object_name`, line 221, which validates a `&Path` and returns `FileStorageResult<String>`). Model the new encode/decode pair on it: a private `fn`, `FileStorageResult` return, `FileStorageError::InvalidPath`-style rejection for malformed input. Put `#[cfg(test)]` round-trip unit tests in the same file.

Imports already present (lines 1-24): `s3::Bucket`, `s3::serde_types::Object`, `uuid::Uuid`, `paladin_ports::output::file_storage_port::{...}`. Bucket bootstrap (D-05): `ensure_bucket_exists` (133) and `create_bucket` (157) run in `MinioAdapter::new` (71), so no `mc` is needed.

### `tests/integration/file_storage_integration_tests.rs` (test, file-I/O)

**Env-gating idiom** (lines 25-35): `TestEnvironment::new()`, then `if env.use_external_services { new_external } else { new_local }`. Every test is `#[ignore]` (lines 192, 207, ...). The external config uses `env.minio_endpoint/minio_access_key/minio_secret_key/test_bucket()` with `path_style: true`, `region: Some("us-east-1")`.

**Line 77 to replace** (`MinIO::default().start()`, `get_host_port_ipv4(9000)`, then a 3-second sleep). Analog for `GenericImage` (`tests/integration/redis_queue_integration_test.rs:68-76`):
```rust
let container = GenericImage::new("redis", "7.2.4")
    .with_exposed_port(6379.tcp())
    .with_wait_for(WaitFor::message_on_stdout("Ready to accept connections"))
    .start().await.expect("Failed to start Redis");
let port = container.get_host_port_ipv4(6379).await?;
```
For RustFS use `GenericImage::new("rustfs/rustfs", "<pinned>")`, `.with_exposed_port(9000.tcp())`, a wait-for on a log line (confirm at plan time), `.with_env_var("RUSTFS_ACCESS_KEY", ..)` and `.with_env_var("RUSTFS_SECRET_KEY", ..)`. Change the `container: Option<ContainerAsync<MinIO>>` field (line 20) to `ContainerAsync<GenericImage>` and drop the `use testcontainers_modules::minio::MinIO;` import (line 7). Drop the `minio` feature at root `Cargo.toml:244`: `testcontainers-modules = { version = "0.12.1", features = ["minio"] }`. Check that other features are not needed before removing the dependency wholesale.

`tests/integration/mod.rs` gates feature-scoped modules with `#[cfg(feature = "...")]` `pub mod ...;`. Register any new module there. Its header comment documents the `#[ignore]` plus feature double gate.

### `src/application/services/run/worker.rs` (`run_agent`, service, event-driven)

**Analog:** `run_once`, in the same file.

Sink assembly to copy (987-1040):
```rust
let bus_sink = self.event_bus.as_ref().map(|bus| {
    Arc::new(RunEventBusSink::new(Arc::clone(bus))) as Arc<dyn TraceSink>
});
let base_sink = build_run_sink(&self.trace_config, bus_sink, self.run_trace_port.clone());
let herald_sink = self.herald.as_ref().map(|herald| {
    Arc::new(HeraldTraceSink::new(Arc::clone(herald), run_model_label(&graph))) as Arc<dyn TraceSink>
});
let composed_sink = match (base_sink, herald_sink) {
    (None, None) => None,
    (Some(sink), None) | (None, Some(sink)) => Some(sink),
    (Some(base), Some(herald)) => Some(Arc::new(CompositeSink::new(vec![base, herald])) as Arc<dyn TraceSink>),
};
let dispatcher = Arc::new(TraceDispatcher::with_capacity(
    run.thread_id.clone(), Some(run.run_id.clone()), Some(sink.clone()),
    self.trace_config.channel_capacity));
```
`run_model_label(&graph)` needs an agent equivalent (a `Paladin` has `PaladinData.model`). `RUN_TRACE_EMITTER` (imported line 63) is the task-local through which below-engine producers reach the dispatcher (D-00e, Phase 28 D-03). Scope the `execute_scoped` call inside `RUN_TRACE_EMITTER.scope(emitter, ...)`. Find the existing scope call site in `run_once` with `grep -n RUN_TRACE_EMITTER` and copy it.

Bus lifecycle to copy from `run_once`: bind on start, and the end-of-run block (~1188-1210):
```rust
tokio::time::sleep(TRACE_DRAIN_GRACE_PERIOD).await;
bus.unbind(&run.thread_id).await;
```
Keep the existing `run_agent` `RunScope` (lines ~1265-1275) verbatim (D-00g):
```rust
let run_scope = RunScope::default()
    .with_run_id(run.run_id.clone())
    .with_ledger_scope(LedgerScope::from_attribution(run.submitted_by.as_ref()));
paladin_port.execute_scoped(paladin.as_ref(), &input_text, &HeartbeatHandle::new(), &run_scope)
```

**Webhook call site to reuse exactly** (`run_once`, worker.rs 1160-1185; the ordering is status write, `record_outcome`, `queue.ack`, and only then the enqueue):
```rust
if let Some(deliveries) = &self.webhook_deliveries
    && let Some(kind) = run_status_to_event_kind(to)
{
    let parleys = match &outcome { RunOutcome::AwaitingInput { parleys, .. } => Some(parleys.as_slice()), _ => None };
    if let Some(delivery) = webhook_delivery_for_outcome(&run, kind, to, parleys, chrono::Utc::now())
        && let Err(error) = deliveries.enqueue(delivery).await
    {
        log::warn!("run worker: failed to enqueue webhook delivery for run {}: {error}", run.run_id);
    }
}
```
Signature (371): `fn webhook_delivery_for_outcome(run: &Run, kind: RunEventKind, status: RunStatus, parleys: Option<&[ParleyRequest]>, now: DateTime<Utc>) -> Option<WebhookDelivery>`. In `run_agent` pass `parleys = None` and `to = RunStatus::Completed` on `Ok`. The `Err` arm goes through `record_engine_failure` (~1295), which also needs the enqueue for `Failed` (check whether that method already enqueues for graph runs, since it is shared).

Docs to update when wiring: the carve-out doc comments at worker.rs ~529, ~549 and ~1239, plus `src/application/services/run/webhook/mod.rs:57`, all naming `agent_kind_run_with_a_webhook_enqueues_no_delivery` and ledger row 31. `WINDOWS.md` row 31 is closed in the same commit.

### `src/application/services/run/worker_tests.rs` (test)

**Analog:** lines 865-975, the WR-02 pinning test to invert. Reusable fixtures: `AgentOnlyResolver` (868), `AlwaysSucceedsPaladinPort` (895), `submit_with_webhook(&repository, &queue, "code-agent", webhook)`, `InMemoryWebhookDeliveryRepository`, and the pool builder:
```rust
RunWorkerPool::new(engine, store, repository.clone(), queue.clone(), resolver, Duration::from_secs(30))
    .with_paladin_port(Arc::new(AlwaysSucceedsPaladinPort))
    .with_webhook_deliveries(Arc::clone(&deliveries));
assert!(worker.run_once().await.unwrap());
let page = deliveries.list_for_run(&run_id, 10, None).await.unwrap();
```
Invert the assertion to `page.items.len() == 1`, and rename it (for example `agent_kind_run_with_a_webhook_enqueues_a_delivery`). Add a Failed-path case using a failing port. For the trace-emitter case, `build_traced_pool(trace_config, event_bus)` (line ~979) returns a pool wired with an `engine_factory`, `trace_config` and `event_bus`. Extend it, or copy it, for an agent-kind pool.

### `src/application/services/run/stream_tests.rs` (test, streaming)

**Analog:** `live_stream_yields_progress_then_done` (234) for a live SSE subscriber, and `replay_and_live_produce_the_same_wire_sequence` (855) for the gapless-`seq` / Replay gate (D-00f). Helpers: `submit` (188), `temp_sqlite_url`/`cleanup` (218/227). Its own `PaladinPort` stub (line 60, `execute`/`execute_stream`) can serve as the agent stub. Add an agent-kind live-stream test in the same shape.

### `src/infrastructure/telemetry/log_sink.rs` (sink, streaming)

**Current code to change** (lines 40-63):
```rust
fn write_trace_line<T: serde::Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(json) => { log::info!(target: "paladin::trace", "{json}"); }
        Err(error) => { log::error!(target: "paladin::trace", "LogTraceSink failed to serialize a TraceRecord: {error}"); }
    }
}
#[async_trait]
impl TraceSink for LogTraceSink {
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
        write_trace_line(&record); Ok(())
    }
}
```
Target design: an early return guarded by `log::log_enabled!(target: "paladin::trace", log::Level::Info)`, plus `serde_json::to_writer` into a reused buffer, such as a thread-local `Vec<u8>` cleared per call.

**Test idioms to copy** (test module at line 65): `install_capturing_logger()` (111), `drain_records(&logger)` (145), `sample_record()` (149), and the serial guard:
```rust
#[tokio::test]
#[serial_test::serial]
async fn log_sink_writes_one_json_line_per_record() {
    let logger = install_capturing_logger();
    drain_records(&logger);
    let sink = LogTraceSink::new();
    sink.on_event(sample_record()).await.unwrap();
    let records = drain_records(&logger);
    ...filter(|(target, _, _)| target == "paladin::trace")
```
`CapturingLogger::enabled` (line 91) controls the disabled case. For a "skips serialisation when disabled" test, reuse the failing-`Serialize` value at line ~215 (`fn serialize(...) -> Err`), which never runs when the target is disabled. Existing tests `log_sink_never_returns_err_and_logs_diagnostic` (229) and `on_event_always_returns_ok` (255) must stay green.

### `crates/paladin-ports/src/output/trace_sink_port.rs` (`CompositeSink`)

**Current** (147-190): `on_event` clones the record per child inside `catch_unwind`:
```rust
for sink in &self.sinks {
    let outcome = AssertUnwindSafe(sink.on_event(record.clone())).catch_unwind().await;
    match outcome { Ok(Ok(())) => all_failed = false, Ok(Err(_)) => {}, Err(_panic) => { log::error!(target: "paladin::trace", ...) } }
}
```
Keep the D-08 panic isolation and the "Ok unless every child failed" contract. Optimisations (for example, skipping the clone for the last child) must keep both. The public surface is additive only (D-00b), so run `make api-surface`.

### `crates/paladin-battalion/src/engine/hooks.rs` (`TraceDispatcher::emit`, line 305)

`emit` stamps `seq` via `queue.seq.fetch_add(1, SeqCst) + 1` (gapless, so it must remain the sole stamp point), tallies superstep/usage/cost in the `match &event` block, then enqueues drop-oldest. Guard any "skip when disabled" logic so it never skips the seq stamp or tallies, or replay breaks (Pitfall 13). The evidence test is `replay_and_live_produce_the_same_wire_sequence`.

### `benches/engine_benchmarks.rs`

`bench_superstep_cost_sink_variants` (line 318): the three variants `none`, `log_sink`, `composite`, reusing `build_width_graph(8)`. Do not change the fixture. Command (from 28-BENCH-EVIDENCE.md):
```bash
cargo bench --bench engine_benchmarks -- bench_superstep_cost --warm-up-time 1 --measurement-time 3
```

### `45-BENCH-EVIDENCE.md`

**Analog:** 28-BENCH-EVIDENCE.md. Section shape: title `# 28-06 Task 3: ... overhead evidence`; `**Purpose:**` (PRD 07 acceptance 6, ≤3 %); `## Command`; `## Machine and toolchain` (CPU, `nproc`, `free -h`, note on a shared machine and outlier counts); then results per variant and a verdict. Baseline to compare: +22.18 % log sink and +18.46 % composite (D-00f). The verdict either meets ≤3 % or records a new accepted figure.

### ADR `.planning/decisions/0055-<slug>.md`

**Analog:** 0054 shape:
```
# ADR-0054: <one-line decision>
## Status
Accepted
**Date:** 2026-09-29
## Context
## Decision
1. **...(D-xx).** ...
```
Then Consequences, if 0054 has it (check its tail). Next number: 0055 (`PROMOTION.md:76` "**Next free ADR number: 0055**"). In the same commit add a table row like line 74, `| 0055 | \`slug\` | summary (D-.., Phase 45, plan 45-xx) |`, change line 76 to 0056, and add a dated note in the style of lines 78-85: "the line advances by **one**, from 0055 to 0056, because ...".

### `k8s/rustfs.yaml` (from `k8s/minio.yaml`)

Copy the two-document shape (Deployment, then Service, both named `paladin-minio` in namespace `paladin`). Rename in the Service and Deployment consistently with `k8s/deployment.yaml`'s `wait-for-minio` init (`nc -z <svc> 9000`, D-06), or keep the service name `paladin-minio` to avoid touching the app manifests, and decide in the plan. Container excerpt to adapt (`k8s/minio.yaml` lines 21-70):
```yaml
containers:
  - name: minio
    image: quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772   # -> rustfs/rustfs:<tag> + digest comment
    args: [server, /data, --console-address, ":9001"]
    env:
      - name: MINIO_ROOT_USER
        valueFrom: { secretKeyRef: { name: paladin-secrets, key: MINIO_ROOT_USER } }
    ports: [{name: api, containerPort: 9000}, {name: console, containerPort: 9001}]
    livenessProbe:  { httpGet: { path: /minio/health/live,  port: api }, initialDelaySeconds: 30, periodSeconds: 30 }
    readinessProbe: { httpGet: { path: /minio/health/ready, port: api }, initialDelaySeconds: 10, periodSeconds: 10 }
    volumeMounts: [{name: data, mountPath: /data}]
volumes: [{name: data, emptyDir: {sizeLimit: 10Gi}}]
```
Change the probe paths to RustFS `/health` (confirm from RESEARCH.md) and the env names to `RUSTFS_ACCESS_KEY` / `RUSTFS_SECRET_KEY` (D-04, keep the secretKeyRef keys unless RESEARCH says otherwise). `k8s/redis.yaml` is the stylistic sibling (not read here).

### Compose and CI (delete `mc`, swap image, swap health path)

- `docker/docker-compose.test.yml`: service `minio-test` (18-47; image line 27, health test line 42 `curl -f http://localhost:9000/minio/health/live`). Delete `minio-test-init` (48-66) and remove its `depends_on` at lines 160-164; `APP_MINIO_ENDPOINT=minio-test:9000` (171) stays. Update the 19-26 pin comment.
- `docker/docker-compose.yml`: `minio` (22-43; env `MINIO_ROOT_USER/PASSWORD` at 29-30), `minio-init` (46-68, includes `mc anonymous set public minio/paladin-files`; that public-read is a behaviour to record or drop in the SUMMARY), `depends_on` at 84/86, `APP_MINIO_*` at 99-102, volume `minio_data` (206).
- `docker/docker-compose.dev.yml`: overrides `minio` env and ports and `minio-init` env (lines 9-22); delete the `minio-init` override and change env names.
- `.devcontainer/docker-compose.yml`: `minio` (91-105; command `server /data --console-address ":9001"`, health line ~103), `depends_on: - minio` (67), env `MINIO_ENDPOINT/ACCESS_KEY/SECRET_KEY` (37-39, app-side, unchanged).
- `.github/workflows/ci.yml`: service block 697-720 (image 706, env 711-712, `--health-cmd` 714, the explicit `server` command note 718), "Install MinIO Client" 743-753 and "Setup MinIO buckets" 755-760 (delete), the health wait 773-776, `TEST_MINIO_*` at 787-789 and 813-815 (names unchanged), the docker-compose job 859-861 (drop `minio-test-init` and the `docker inspect ... exited` wait), a second block at 1409+ (E2E). Also check `.github/workflows/integration-tests.yml`, which line 719 says has an identical block.
- `Makefile` and docs quoting the pin: `docs/src/appendix/minio-file-repository-setup.md`, `testing-guide.md` (the `GenericImage::new(...)` example is docs-currency gated). Locate every remaining reference before finishing with `grep -rn "quay.io/minio\|minio/mc\|minio/health"` across the repo.

## Shared Patterns

### Pin discipline (D-03)
One exact tag `rustfs/rustfs:<tag>` plus an adjacent digest comment, identical in every file. Mirror the existing MinIO pin comment block (`docker/docker-compose.test.yml` 19-26, `ci.yml` 698-705), rewritten to record the RustFS choice.

### Test credential literals
CI uses throwaway `testuser` / `testpass123` (`ci.yml` 711-712). Compose defaults use `minioadmin`. Do not put real credentials anywhere (D-00a).

### Trace fan-out and ordering (D-00e)
Any new code emitting run events goes through `build_run_sink` -> `CompositeSink` -> `TraceDispatcher` (one per run), and the SSE mapping stays in `map_trace_event` only. Webhook enqueue is strictly after the status write and ack, and a failure is `log::warn!` only (worker.rs 1160-1185).

### Error style
Adapter errors: `FileStorageError::IoError(format!("Failed to ...: {}", e))` (`minio.rs`). No `unwrap`/`expect` in library code. Tests use `#[tokio::test]` plus `#[serial_test::serial]` where they install the global logger.

### Gates
`make clean-code`, `make api-surface`, `make security`, `make check-api-examples`, 82 % coverage floor, and a CHANGELOG entry naming the RustFS pin.

## No Analog Found

| File | Role | Reason |
|---|---|---|
| Multipart upload-id-to-path token helper in `minio.rs` | utility | No existing stateless token encode/decode in the crate; use `path_to_object_name` (221) only as a style guide. |
| RustFS log-line wait-for string for `GenericImage` | test harness | Not derivable from the repo; take it from RESEARCH.md or from the image at plan time. |

## Metadata

**Analog search scope:** `src/application/services/run/`, `src/infrastructure/telemetry/`, `crates/paladin-storage`, `crates/paladin-ports`, `crates/paladin-battalion/src/engine/hooks.rs`, `tests/integration/`, `docker/`, `.devcontainer/`, `k8s/`, `.github/workflows/ci.yml`, `.planning/decisions/`, the Phase 28 evidence doc.
**Not read in full:** the 45-RESEARCH.md body (only CONTEXT.md D-01..D-07 were read closely), `k8s/redis.yaml`, `k8s/deployment.yaml`, `Makefile`, docs. Planner should confirm RustFS env names, health path and wait-for string from RESEARCH.md.
**Pattern extraction date:** 2026-09-29
