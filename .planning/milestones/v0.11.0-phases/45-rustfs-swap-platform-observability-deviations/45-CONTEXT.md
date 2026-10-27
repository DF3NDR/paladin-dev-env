# Phase 45: RustFS Swap & Platform/Observability Deviations - Context

**Gathered:** 2026-09-29
**Status:** Ready for planning
**Mode:** `--auto` (every question below was answered with the recommended option; the
alternatives considered are in `45-DISCUSSION-LOG.md`)

<domain>
## Phase Boundary

Three independent deliveries, sequenced together because none of them touches the Treasurer
feature surface (Phases 41-43) and all three close standing deviations:

1. **Object store (STORE-01, STORE-02, STORE-03).** The dev/test compose stack, the devcontainer
   compose, the Coverage / Integration Tests / Docker Integration Tests / Kubernetes Smoke Test CI
   jobs and the End-to-End job run against **RustFS pinned to an exact tag**. Bucket bootstrap no
   longer shells out to `mc`. No MinIO image reference remains in any live configuration (CI,
   compose, k8s, devcontainer, `testcontainers` local mode). The `FileStoragePort` contract suite
   (presigned URLs, multipart uploads, ETags) passes against RustFS using the **existing**
   `rust-s3`-based adapter in `crates/paladin-storage/src/minio.rs`; a new adapter behind a feature
   flag is written only if that suite fails. Storage docs are updated and the production-manifest
   decision is recorded.

2. **Legacy `Runnable::Agent` runs (PLAT-08, `WINDOWS.md` row 31).** A run against a
   code-registered agent emits SSE live events and enqueues webhook deliveries the same way a graph
   run does. The pinning test `agent_kind_run_with_a_webhook_enqueues_no_delivery` is inverted and
   row 31 is closed.

3. **Tracing overhead (OBS-05, `WINDOWS.md` row 35, D-16).** `LogTraceSink` and
   `TraceDispatcher::emit` skip serialisation when the `paladin::trace` log target is disabled and
   reuse their buffers; `bench_superstep_cost_sink_variants` is re-measured with the Phase 28
   fixture and command, and the result either meets PRD 07 criterion 6's ≤ 3 % bar or a new
   accepted figure is written on the record.

**Not in this phase:** the SSE `done` status for a caller-cancelled or Treasurer-halted run
(PLAT-09, Phase 42); any Treasurer, allowance or pacing work (41-43); the legacy Battalion
removals (44); the Treasurer mdBook page and the v0.10 → v0.11 `MIGRATION.md` guide (46);
renaming `MinioAdapter`/`MinioConfig`/the `minio:` config section/`APP_MINIO_*` (see Deferred);
an I/O-bound re-scoping of the tracing bar (see Deferred); the user-owned local `make coverage`
walk on a Docker-capable machine (its docs half is folded, its walk is not).

</domain>

<decisions>
## Implementation Decisions

### Carried forward (locked by earlier phases and milestone-level decisions — not re-asked)

- **D-00a:** Shipped tree outranks any document (Phase 34 D-00g and successors). 82 % workspace
  line-coverage floor (ADR-0006). `make clean-code`, `make api-surface` (+ `make
  api-surface-update` and a CHANGELOG entry for an intentional public-surface change), `make
  security`, `make check-api-examples` and the manual credential-handling review in
  `.github/instructions/security.instructions.md` gate every commit. No log line, error, response
  body or CI log ever carries a real credential; the RustFS root credentials used in CI and compose
  are throwaway test literals exactly as the MinIO ones were.
- **D-00b:** X-03 governs public API: any break needs a `MIGRATION.md` §9.2 row and, when marked
  `Y`, a `.cargo/semver-checks-allowlist.toml` entry naming one crate and one lint (Phase 38
  D-00g, Phase 40 D-00c). This phase is designed to be **additive on the Rust surface** — see
  D-08 and D-17 — so no §9.2 row is expected; if the planner finds one is needed, it is written
  in the same commit as the code.
- **D-00c:** ADRs take the next free number from `.planning/decisions/PROMOTION.md` (currently
  **0055**) and advance that line in the same commit (Phase 38 D-00f).
- **D-00d:** Vocabulary: Medieval-military words for roles, plain words for units and
  identifiers (Phase 30 D-01). "RustFS", "bucket", "object store", "trace sink" are plain
  identifiers; no new officer is minted in this phase.
- **D-00e:** Phase 27 D-24 (the live SSE path is a `TraceSink` adapter — `RunEventBusSink` —
  feeding a per-run broadcast bus, `map_trace_event` is the ONE trace-to-wire mapping) and D-40
  (webhook delivery is a persisted queue drained by `WebhookDeliveryService`, enqueued strictly
  after the run's own status write and ack, never rolled back) are the shape this phase wires the
  legacy agent path into. Phase 28 D-03 (one `TraceDispatcher` per run; below-engine producers
  reach it through the `RUN_TRACE_EMITTER` task-local) is how the agent loop's own trace events
  reach that sink.
- **D-00f:** Phase 28 D-16 / STATE.md D-37 (the ≤ 3 % bar measured at +22.18 % log sink /
  +18.46 % composite on `bench_superstep_cost_sink_variants`, ACCEPTED for v0.10.0) is the
  baseline this phase re-measures against, on the same fixture, same command, same three variants
  (`none`, `log_sink`, `composite`). Any optimisation must leave the gapless-`seq` guarantee and
  `RunStreamMode::Replay` intact — the existing observability/replay suite is a hard gate, not
  the throughput number alone (research Pitfall 13).
- **D-00g:** Phases 39-05/39-07 and 40-04 (D-15/D-16) put a `RunScope` carrying the Platform run
  id and the ledger scope on `run_agent`'s `execute_scoped` call. That call and its scope are
  preserved verbatim by the agent-path wiring below; nothing in this phase changes what the
  ledger settles for an agent run.
- **D-00h:** `security.instructions.md`'s redact-then-truncate rule and the SSRF guard on webhook
  URLs (checked at submit and at send) are untouched; an agent run's webhook delivery goes
  through the identical `WebhookDeliveryService` path a graph run's does.

### Sequencing and the interim MinIO re-pin

- **D-01:** Phase 45 executes **now, ahead of Phases 41-44**. The roadmap's "Depends on: Phase
  44" is a diff-conflict-avoidance preference, not a functional dependency; with 41-44 unplanned
  there is nothing to conflict with, and every later phase's Coverage / Integration / Docker
  Integration / Kubernetes Smoke evidence is red until the object store is replaced (Phase 40 UAT
  test 4 is `blocked` on exactly this). The first plan amends the Phase 45 `Depends on` line in
  `.planning/ROADMAP.md` to record the resequencing (one line, docs only).
- **D-02:** The interim quick task in
  `.planning/todos/pending/2026-09-29-interim-minio-image-repin-quay-locked.md` (re-pin the MinIO
  images at Chainguard / a ghcr mirror / authenticated quay) is **superseded, not executed**: no
  MinIO image is re-pinned anywhere. Both MinIO todos (`2026-09-13-…` and `2026-09-29-…`) are
  closed by this phase's SUMMARY with a pointer to the commits that removed the last reference,
  never silently. After the swap lands green, `/gsd-verify-work 40` is re-run so Phase 40 UAT
  test 4 flips from `blocked` to `pass` (a follow-up noted in this phase's SUMMARY, not a Phase 45
  success criterion).

### RustFS image pin, service shape and bucket bootstrap

- **D-03:** The image is `rustfs/rustfs` at the **newest exact GA tag the researcher verifies is
  pullable at plan time** (research says `1.0.0`, released 2026-09-16, Apache-2.0, multi-arch),
  written as `image: rustfs/rustfs:<tag>` with the resolved manifest digest recorded in an
  adjacent comment — the same house shape the MinIO pin used. Never `:latest`, never a floating
  or `-alpha` tag. The same tag is used in every live configuration (both `ci.yml` service
  blocks, `docker/docker-compose.test.yml`, `docker/docker-compose.yml`,
  `docker/docker-compose.dev.yml` if it declares its own image, `.devcontainer/docker-compose.yml`,
  the Kubernetes manifest, and the `testcontainers` `GenericImage` in the contract suite's local
  mode) — one pin, one place per file, one CHANGELOG line naming it. A later re-pin is a
  deliberate commit, never automatic.
  — **Reversibility:** reversible — a tag is one string per file; the digest comment makes any
  later re-pin auditable.
- **D-04:** Service-side environment variables follow the RustFS image's own documented names
  (research: `RUSTFS_ACCESS_KEY` / `RUSTFS_SECRET_KEY`, S3 API on 9000, console on 9001, health at
  `GET /health` on the API port — the researcher confirms the exact names, ports and health path
  from RustFS's docs or image before the planner writes them). Application-side names are
  **unchanged**: the `minio:` config section (`config.example.yml`, `k8s/configmap.yaml`),
  `APP_MINIO_ACCESS_KEY`/`APP_MINIO_SECRET_KEY`, `MinioConfig`, and the test harness's
  `TEST_MINIO_ENDPOINT`/`TEST_MINIO_ACCESS_KEY`/`TEST_MINIO_SECRET_KEY` all keep their names so no
  operator config, no `MIGRATION.md` row and no docs-quoted config snippet changes meaning. The
  Kubernetes secret keys created by the smoke test and consumed by `k8s/deployment.yaml`
  (`MINIO_ROOT_USER`/`MINIO_ROOT_PASSWORD` for the store, `MINIO_ACCESS_KEY`/`MINIO_SECRET_KEY`
  for the app) are renamed on the store side only if the RustFS container needs different names;
  the app-side pair stays.
- **D-05:** **Bucket bootstrap is the adapter's own `ensure_bucket_exists` → `Bucket::create`
  path, which already ships in `minio.rs` and runs on every `MinioAdapter::new`.** The compose
  `minio-init` / `minio-test-init` `mc` containers, the CI "Install MinIO Client" and "Setup MinIO
  buckets" steps, and the checksum-verified `mc` binary download are **deleted, not replaced**
  with an `rc` container — one fewer third-party image to pin, and the bootstrap is exercised by
  the contract suite itself (STORE-01 "bucket bootstrap replaces `mc`"). The suite's own
  `test_bucket()` names and the E2E job's buckets are therefore created by the code under test.
  Fallback, taken only if the suite proves RustFS rejects the adapter's `PUT /bucket` with a
  default `BucketConfiguration`: a pinned `rustfs/rc` init container using the same tag
  discipline as D-03, recorded as a deviation in the SUMMARY.
- **D-06:** Health checks move from `/minio/health/live|ready` to RustFS's health endpoint in
  every place they appear (both `ci.yml` service `--health-cmd`s, all compose `healthcheck`s,
  the k8s liveness/readiness probes, the E2E job's `curl -f` check, and the Makefile/devcontainer
  preflights). `k8s/deployment.yaml`'s `wait-for-minio` init container keeps its `nc -z <svc>
  9000` shape against the renamed service.
- **D-07:** Local mode of the contract suite (`FileStorageTestContext::new_local`) swaps
  `testcontainers_modules::minio::MinIO` for a `testcontainers::GenericImage` of the pinned RustFS
  tag with the D-04 env vars and a wait-for condition (log line or health), and the `minio`
  feature is removed from the root `Cargo.toml`'s `testcontainers-modules` dependency. The
  `testing-guide.md` `GenericImage::new(...)` example is updated to the same pin (it is a
  docs-currency-gated quote).

### Contract-suite gate, multipart gap and adapter reuse

- **D-08:** **Reuse first.** No new adapter file, no new Cargo feature and no new crate unless the
  contract suite fails against RustFS. The decision gate is the suite green in CI's **Docker
  Integration Tests** job (and the Coverage / Integration Tests service-container jobs) on the
  phase's tree — this sandbox has no container runtime and cannot pull images (proxy 403), so the
  evidence is CI-attributed exactly as Phase 40's was. Only a suite failure that traces to a
  RustFS S3-surface gap (SigV4 presign validation, a header `rust-s3` sends that RustFS rejects,
  ETag shape) opens the second-adapter path: a new file beside `minio.rs`, behind its own
  `paladin-storage` feature, sharing the one `rust-s3` dependency (the `sqlite`/`mysql`/`postgres`
  precedent in the same crate), with the failure quoted in the SUMMARY.
- **D-09:** **The suite is completed before it is used as the gate.** Today the suite has no
  multipart case and no ETag-format assertion, and the adapter's `upload_part`,
  `complete_multipart_upload` and `abort_multipart_upload` are stubs returning
  `FileStorageError::Unknown("Multipart upload not fully implemented with rust-s3")` — so
  "including multipart uploads and ETags passes" cannot be true as the tree stands. This phase
  implements the three stubbed methods with `rust-s3 0.35.1`'s existing
  `put_multipart_chunk` / `complete_multipart_upload` / `abort_upload` (no version bump, no new
  dependency — `rust-s3` stays at `0.35.1`), and adds to the suite: (a) a multipart lifecycle
  case (create → ≥ 2 parts → complete → download equals the concatenation; and create → abort →
  object absent), (b) an ETag assertion that treats ETag as an **opaque, quote-stripped, stable
  token** (same value from `get_file_info` and `list_files` for the same object; a re-upload of
  different bytes changes it) — never as an MD5 of the content, since multipart ETags are
  composite on MinIO/AWS and possibly different on RustFS (research Pitfall 11), and (c) the
  presigned-URL case exercises the URLs (a real `PUT` through the upload URL and a real `GET`
  through the download URL), not just their generation. The adapter's existing `md5_hash =
  ETag` population stays as-is — it is a label, not a verified digest — and its rustdoc says so.
  — **Reversibility:** reversible — additive methods on a `#[doc(hidden)]` adapter and new test
  cases; nothing public changes shape.
- **D-10:** Adapter identity is unchanged: `MinioAdapter`, `MinioConfig`, the `s3` feature in
  `paladin-storage` and the `s3-storage` facade feature keep their names; the adapter's and the
  docs' prose is corrected to "S3-compatible store (RustFS in dev/test/CI; MinIO, AWS S3,
  DigitalOcean Spaces or any SigV4 S3 endpoint in production)". A rename to an S3-neutral name is
  a public break and is deferred (see Deferred Ideas).

### Production manifest and the decision record (STORE-03)

- **D-11:** `k8s/minio.yaml` is **one manifest that serves both the Kubernetes Smoke Test and the
  reference deployment**, and STORE-01 requires the smoke test to run RustFS — so the reference
  manifest follows: the file is renamed `k8s/rustfs.yaml` (Deployment + Service `paladin-rustfs`,
  labels `app: rustfs`, `component: storage`), with the pinned image, D-04 env, D-06 probes and
  the same `emptyDir`/resource shape. Every reader is updated in the same commit: `ci.yml`'s
  `kubectl apply`/`kubectl wait`/log-dump lines, `k8s/deployment.yaml`'s init container and
  service name, `k8s/configmap.yaml`'s `minio.endpoint`, `k8s/README.md`. No split into a
  smoke-only manifest: a terminal, unpullable MinIO image cannot be a production reference
  either, and the docs already tell operators how to point the adapter at AWS S3 or another
  managed endpoint instead of the bundled store.
  — **Reversibility:** costly — operators who `kubectl apply -f k8s/minio.yaml` by path lose
  that path; the CHANGELOG entry and `k8s/README.md` name the rename explicitly.
- **D-12:** The decision is recorded as **ADR-0055 "Dev/test and reference object store is
  RustFS"** in `.planning/decisions/0055-*.md` (Context: the 2026-09-12 Docker Hub deletion, the
  terminal quay pin, the 2026-09-24 quay lockout; Decision: RustFS exact-tag pin everywhere,
  adapter reuse proven by the contract suite, reference manifest follows, bootstrap by the
  adapter; Consequences: one vendor image to re-pin deliberately, the re-pin trigger is a RustFS
  security release or a suite regression), cited from
  `docs/src/appendix/minio-file-repository-setup.md` (the "storage docs" STORE-03 names — the
  file keeps its path so the book's links and `SUMMARY.md` entry hold; its title and body change
  to the S3-compatible framing of D-10, with a RustFS quick-start section replacing the MinIO one
  and the MinIO/AWS/Spaces compatibility notes kept). The PROMOTION.md line advances to 0056.
- **D-13:** Every doc that quotes the MinIO pin is updated in the same plan as the config it
  quotes, so the Phase 34 docs-currency gate stays green: `docs/src/appendix/integration-tests.md`,
  `docs/src/contributing/testing-guide.md`, `docs/src/contributing/branching-model.md`,
  `docs/src/deployment/cicd.md`, `docs/src/deployment/docker.md`, `.devcontainer/CI-CD.md`,
  `.devcontainer/README.md`/`QUICKSTART.md`/`FILES.md`/`SETUP_COMPLETE.md` where they name the
  image, `k8s/README.md`, and the Makefile help text / `minio-console` target (kept as a target
  name, pointed at RustFS's console). `make docs` (the mdBook build) runs before the commit.

### Legacy `Runnable::Agent` SSE and webhook parity (PLAT-08, row 31)

- **D-14:** **Same machinery, not a hand-published `done`.** `run_agent` gains the graph path's
  per-run assembly: `build_run_sink(&self.trace_config, bus_sink, self.run_trace_port.clone())`
  (+ the `HeraldTraceSink` when a herald is wired, composed exactly as `run_once` does), one
  `TraceDispatcher::with_capacity` bound to this run, `event_bus.bind` before dispatch, the
  `execute_scoped` call wrapped in `with_run_trace_scope(&emitter, …)` so the agent loop's own
  `MiddlewareEvent`/`NodeProgress`/`FallbackHop` records reach the bus through
  `RunEventBusSink`/`map_trace_event`, and `event_bus.unbind` after the same
  `TRACE_DRAIN_GRACE_PERIOD` the graph path waits. Because no engine runs, `run_agent` itself emits
  `TraceEvent::RunStarted` before the call and `TraceEvent::RunFinished` after it
  (`status: Completed` or `Failed`, `total_supersteps: 0`, `usage`/`cost` read from the
  dispatcher's `total_usage()`/`total_cost()`, `duration_ms` measured, `trace_dropped_total`
  overwritten by the dispatcher as today) so the wire `done`/`error` events come from the ONE
  `map_trace_event` mapping and carry the same payload shape a graph run's do (D-24). A
  `NodeFinished`-style per-call record for the single agent call is the planner's choice, not a
  requirement.
  — **Reversibility:** reversible — additive wiring inside one private method; the pinning test
  and row 31 pin the new behaviour.
- **D-15:** **Webhook enqueue uses the shared helper.** On the success path, strictly after
  `update_status` → `record_outcome` → `ack` succeed, `run_agent` calls
  `webhook_delivery_for_outcome(&run, RunEventKind::Completed, RunStatus::Completed, None, now)`
  and enqueues it, logging (never propagating) an enqueue error — prohibition P2 verbatim. On the
  failure path `record_engine_failure` (which already publishes the bus `error` event and unbinds)
  gains the same enqueue for `RunEventKind::Failed` after its own status write and ack, so the
  graph path's `Err` branch — which also goes through `record_engine_failure` — gets the webhook
  it was previously missing too; the planner confirms with a test that a graph run failing
  through `record_engine_failure` now enqueues exactly one `Failed` delivery (today `run_once`'s
  `Transition` arm enqueues only on the `Ok` path). `RunFinished{Failed}` from D-14 and
  `record_engine_failure`'s existing direct publish must not produce two `error` wire events —
  the planner picks one (prefer: `record_engine_failure` keeps its publish for the graph path's
  engine-error case and `run_agent` routes its failure through `RunFinished{Failed}` before
  calling it with the bus publish skipped, or the reverse), pinned by a test asserting exactly
  one terminal event per agent run.
- **D-16:** `agent_kind_run_with_a_webhook_enqueues_no_delivery` is inverted to
  `agent_kind_run_with_a_webhook_enqueues_a_delivery` (asserting one `Pending` delivery whose
  payload carries the run id, `Completed`, and the assistant fields), plus a sibling asserting the
  SSE subscriber of an agent run receives `RunStarted`-derived and `done` events in `Live` mode.
  `WINDOWS.md` row 31 is closed through the ledger tool with the inverted test named as the
  closing evidence; the field docs on `event_bus`/`webhook_deliveries`, `run_agent`'s own
  rustdoc, `webhook/mod.rs`'s `WebhookPayload` carve-out and
  `docs/src/api-reference/platform-api.md`'s "A run against a code-registered agent never fires a
  webhook" limitation are all rewritten or deleted in the same commit. Cancel/halt `done` status
  for agent runs is **not** touched (PLAT-09, Phase 42).

### Tracing overhead (OBS-05, row 35)

- **D-17:** Two no-new-dependency fixes, in this order, each measured: (1) an **enablement
  guard** — `LogTraceSink::on_event` checks `log::log_enabled!(target: "paladin::trace",
  log::Level::Info)` before serialising and returns `Ok(())` without touching `serde_json` when
  the target is filtered (the requirement's "skips serialisation when logging is disabled"); the
  same guard shape is applied inside `TraceDispatcher`'s consumer only where the planner can show
  it is a sink-independent no-op (the dispatcher must never learn a sink's log configuration);
  (2) **buffer reuse** — `write_trace_line` serialises with `serde_json::to_writer` into a reused
  `Vec<u8>` and logs `str::from_utf8(&buf)` rather than allocating a fresh `String` per record,
  and `TraceDispatcher::emit`'s per-event `clone()`s (`thread_id`, `run_id`, `usage`) are
  reduced where an `Arc`/borrow is equivalent, without changing `TraceRecord`'s public shape.
  `sonic-rs`/`simd-json` are **not** adopted (research "What NOT to Use"). No unbounded buffering
  ahead of the sink; the drop-oldest bounded queue stays the only backpressure (Pitfall 13).
  — **Reversibility:** reversible — internal to two private functions; the bench and the replay
  suite pin the observable contract.
- **D-18:** **Same fixture, same command, comparable numbers.** `cargo bench --bench
  engine_benchmarks -- bench_superstep_cost --warm-up-time 1 --measurement-time 3` on
  `bench_superstep_cost_sink_variants` (`build_width_graph(8)`, variants `none`/`log_sink`/
  `composite`), recorded in `45-BENCH-EVIDENCE.md` with the machine/toolchain/free-memory block
  and the raw criterion output exactly as `28-BENCH-EVIDENCE.md` does, run **before and after**
  each of D-17's two fixes on the same machine in the same session. The bar is PRD 07 criterion
  6's ≤ 3 % for BOTH `log_sink` and `composite` against `none`. The measurement is taken with the
  `paladin::trace` target **enabled** (an `Info`-level logger installed, output discarded), so
  the guard's cheap path is not what is measured; a second row with the target disabled is
  recorded for information.
- **D-19:** **Outcome recording.** If both variants meet ≤ 3 %: row 35 is closed (`fixed`) with
  the evidence file named; the "Known limitations" bullet in
  `docs/src/operations/observability.md` and the `[0.10.0]`-era CHANGELOG limitation get a
  `[Unreleased]` "Fixed" entry citing the new figures. If either still misses: row 35 is
  **amended, not re-waived silently** — its closing condition is rewritten to the new measured
  figure ("accepted at +N %/+M % after the D-17 fixes, 2026-09-xx, `45-BENCH-EVIDENCE.md`"),
  the same figure replaces +22 %/+18 % in `observability.md`, `PROJECT.md`'s known-deviation
  lines and the CHANGELOG, and the maintainer's acceptance is requested at this phase's UAT (the
  Phase 28 D-37 precedent). No I/O-bound bench variant is added in this phase either way (see
  Deferred Ideas).

### Claude's Discretion

- Exact `GenericImage` wait strategy for RustFS in local mode (stdout message vs. HTTP health
  poll); whether the compose files keep a console port mapping at all.
- Whether `LogTraceSink` keeps `#[derive(Copy)]` with a thread-local buffer, or drops `Copy` for
  a `Mutex<Vec<u8>>` field — dropping `Copy` on a public facade type is an api-surface change that
  needs `make api-surface-update` and a CHANGELOG line, so prefer the thread-local unless it is
  measurably worse.
- The exact `RunStarted` payload for an agent run (`graph_fingerprint` is meaningless without a
  graph — an empty/`"agent"` sentinel is acceptable if `map_trace_event` and the OTel/herald sinks
  tolerate it; add a test) and whether a per-call `NodeStarted`/`NodeFinished` pair is emitted.
- How the E2E job's `docker/.env` is written (variable names follow D-04) and whether
  `docker-compose.dev.yml` needs its own image line or inherits.
- Test topology for the multipart/ETag cases (new `#[ignore]`d `#[tokio::test]`s in
  `file_storage_integration_tests.rs` registered in `run_all_file_storage_tests`, matching the
  existing shape) and the part size used (≥ 5 MiB per non-final part is the S3 minimum — verify
  RustFS's own minimum).
- Plan split and wave order — the researcher's RustFS verification is the natural first plan;
  the agent-wiring and tracing plans are independent of it and of each other.

### Folded Todos

- **Evaluate replacing MinIO with RustFS in the dev/test stack**
  (`.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`, score 0.6,
  `resolves_phase: 45`). The four open questions it lists (S3-surface coverage vs. what
  `paladin-storage` calls; multi-arch image and cadence; licence under `cargo-deny`; whether the
  production manifest follows) are answered by D-08/D-09, D-03, the research (Apache-2.0, already
  allow-listed) and D-11/D-12 respectively. Closed by this phase's SUMMARY.
- **Interim re-pin of the MinIO server/client images — quay.io locked anonymous pulls**
  (`.planning/todos/pending/2026-09-29-interim-minio-image-repin-quay-locked.md`, score 0.6,
  `superseded_by: Phase 45`). Superseded by D-01/D-02; its file inventory (7 server-image
  occurrences, 2 `mc`-image occurrences, 11 docs that quote the pin) is the checklist D-03/D-13
  must empty. Closed by this phase's SUMMARY.
- **Verify local `make coverage` reproduces CI's figure**
  (`.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md`, score 0.6, user-owned).
  Folded in its **docs-currency sense only**: the testing guide's Code Coverage section documents
  `make services-up` → `make coverage`, and `services-up` now brings up RustFS, so the section
  must stay accurate after the swap. The end-to-end walk on a Docker-capable machine remains the
  maintainer's; the todo stays open with its 2026-10-16 re-check.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Milestone scope and requirements
- `.planning/ROADMAP.md` §"Phase 45: RustFS Swap & Platform/Observability Deviations" — goal,
  five success criteria, the research flag (RustFS parity is MEDIUM-confidence; close it via the
  contract suite first). Also §Phase 42 (owns PLAT-09) and §Phase 46 (owns the Treasurer docs).
- `.planning/REQUIREMENTS.md` — STORE-01, STORE-02, STORE-03, PLAT-08, OBS-05 (the phase's five
  requirements); PLAT-09 and CURR-22…25 are explicitly other phases'.
- `.planning/PROJECT.md` — *Current Milestone* bullets "RustFS (FUT-10)", "Platform deviations
  (row 31)", "Tracing overhead (D-16, row 35)"; the Phase 28 close note recording +22.18 %/+18.46 %.
- `.planning/STATE.md` §Blockers/Concerns — "Terminal MinIO pin", "Tracing overhead", "Coverage
  is CI-attributed"; §Pending Todos.

### Research (read before planning; the RustFS rows are MEDIUM-confidence and must be verified)
- `.planning/research/SUMMARY.md` — Phase 45 rationale, research flags, "Gaps to Address"
  (RustFS parity; the two tracing fixes).
- `.planning/research/STACK.md` §"Tracing overhead (D-16, row 35)" (root cause in
  `write_trace_line` and `TraceDispatcher::emit`; the two no-new-dependency fixes; `sonic-rs`
  rejected) and §"RustFS (FUT-10)" (tag `1.0.0`, Apache-2.0, multi-arch, `GET /health`, `rc`
  client, reuse-first shape) and §"What NOT to Use" (`:latest`).
- `.planning/research/PITFALLS.md` — Pitfall 11 (RustFS is not a drop-in: ETag, `mc`, presign,
  multipart, cadence), Pitfall 13 and the "Integration Gotchas" row on the Phase 28 trace stream
  (gaplessness/replay is a hard gate), "Performance Traps" row on unbounded trace buffers.

### Standing decisions and deviation records (cite, do not re-open)
- `.planning/WINDOWS.md` — row 31 (agent runs excluded from bus and webhook; closing condition:
  wire both hooks into `run_agent`'s two return paths and invert the pinning test) and row 35
  (the accepted tracing-overhead FAIL; closing condition and where the same record lives).
- `.planning/milestones/v0.10.0-phases/28-observability-tooling/28-BENCH-EVIDENCE.md` — the
  baseline command, fixture, machine block and raw criterion output D-18 must mirror.
- `.planning/milestones/v0.10.0-phases/28-observability-tooling/28-CONTEXT.md` — D-03 (one
  dispatcher per run; `RUN_TRACE_EMITTER`), D-05 (`state_values` off by default), D-07/D-08
  (drop-oldest, panic-isolated sinks).
- `.planning/milestones/v0.10.0-phases/29-*/29-CONTEXT.md` — D-16 (the acceptance and the
  deferred "re-scope to an I/O-bound superstep" follow-up).
- `.planning/milestones/v0.10.0-phases/27-*/27-CONTEXT.md` — D-24 (live SSE via
  `RunEventBusSink`), D-40 (persisted webhook queue; enqueue after status write and ack), D-14
  (SSE `done` collapse — Phase 42's to fix).
- `.project/v0.10.0/07-observability-tooling.md` — criterion 6 (≤ 3 % superstep overhead vs.
  sink-disabled, in `benches/`).
- `.planning/decisions/PROMOTION.md` — ADR numbering (next free: 0055 → this phase advances to 0056).
- `.planning/decisions/0006-coverage-gate.md` (ADR-0006) — 82 % floor.
- `.planning/phases/40-tenant-identity-run-read-scoping/40-CONTEXT.md` — D-15/D-16 (the
  `RunScope`/ledger scope `run_agent` carries; preserved by D-00g) and D-00a…g (house
  conventions carried into this phase's D-00a…d).
- `.planning/phases/40-tenant-identity-run-read-scoping/40-UAT.md` — test 4 `blocked` on the
  MinIO pull; the re-verify D-02 schedules.
- `.github/instructions/security.instructions.md` — manual credential-handling review;
  redact-then-truncate; webhook SSRF guard (untouched, but the agent path now reaches it).

### Todos folded (full text is the requirement)
- `.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`
- `.planning/todos/pending/2026-09-29-interim-minio-image-repin-quay-locked.md` — its file/line
  inventory is the removal checklist.
- `.planning/todos/pending/2026-08-13-verify-local-coverage-reproduction.md` — docs half only.

### Docs the phase must keep current (Phase 34 docs-currency gate)
- `docs/src/appendix/minio-file-repository-setup.md` — the STORE-03 "storage docs"; retitled and
  reframed per D-10/D-12, path kept.
- `docs/src/operations/observability.md` §"Known limitations" — the overhead bullet D-19 rewrites.
- `docs/src/api-reference/platform-api.md` §Webhooks "Known limitations" — the agent carve-out
  D-16 deletes.
- `docs/src/appendix/integration-tests.md`, `docs/src/contributing/testing-guide.md` (incl. the
  `GenericImage` example and the Code Coverage section), `docs/src/contributing/branching-model.md`,
  `docs/src/deployment/cicd.md`, `docs/src/deployment/docker.md`, `.devcontainer/CI-CD.md`,
  `k8s/README.md` — every file that quotes the MinIO pin or the `mc` steps.
- `CHANGELOG.md` `[Unreleased]` — Changed (RustFS pin, manifest rename), Fixed (row 31, row 35
  or its new figure), Removed (`mc` bootstrap, MinIO images).

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crates/paladin-storage/src/minio.rs` — `MinioConfig` (endpoint/region/path-style/secure),
  `MinioAdapter::new` → `ensure_bucket_exists` → `Bucket::create` (the D-05 bootstrap, already
  shipped), `presign_put`/`presign_get`, ETag → `md5_hash` population, and the three multipart
  stubs D-09 fills. `rust-s3 0.35.1` already exposes `initiate_multipart_upload`,
  `put_multipart_chunk`, `complete_multipart_upload`, `abort_upload`.
- `crates/paladin-ports/src/output/file_storage_port.rs` — `FileStoragePort`,
  `AdvancedFileStoragePort` (presign + multipart), `BatchFileStoragePort`, `FileVersioningPort`;
  unchanged by this phase.
- `tests/integration/file_storage_integration_tests.rs` + `tests/integration/mod.rs` — the contract
  suite (`run_all_file_storage_tests`, eight cases, `#[ignore]`d `#[tokio::test]`s),
  `TestEnvironment` (external mode via `TEST_MINIO_*` + `CI`/`USE_EXTERNAL_TEST_SERVICES`; local
  mode via `testcontainers_modules::minio::MinIO` — the D-07 swap point).
- `src/application/services/run/worker.rs` — `run_once`'s per-run sink assembly (`build_run_sink`,
  `HeraldTraceSink`, `TraceDispatcher::with_capacity`, `with_run_trace_scope`, `event_bus.bind`/
  `unbind`, `TRACE_DRAIN_GRACE_PERIOD`), `webhook_delivery_for_outcome`,
  `run_status_to_event_kind`, `record_engine_failure` — everything D-14/D-15 reuse.
- `src/application/services/run/events.rs` — `RunEventBus`, `RunEventBusSink`, `map_trace_event`
  (`RunFinished{completed|halted|awaiting_input}` → `done`, `{failed}` → `error`).
- `src/infrastructure/telemetry/{mod,log_sink}.rs` — `build_run_sink`, `LogTraceSink`,
  `write_trace_line` (the D-17 site); `crates/paladin-battalion/src/engine/hooks.rs` —
  `TraceDispatcher::emit` and its consumer.
- `benches/engine_benchmarks.rs` — `bench_superstep_cost_sink_variants` (D-18's fixture, unchanged).
- `src/application/services/run/worker_tests.rs` — `agent_kind_run_with_a_webhook_enqueues_no_delivery`,
  `submit_with_webhook`, `AlwaysSucceedsPaladinPort`, `AgentOnlyResolver` — the D-16 inversion
  builds on these.

### Established Patterns
- **Exact-tag pin + manifest-digest comment** for every third-party image (the MinIO block in
  `ci.yml`/compose); `actionlint` suppression for `services.command` in `.github/actionlint.yaml`
  (the RustFS service block needs the same treatment if it needs a `command`).
- **One shared helper per rule** (`webhook_delivery_for_outcome`, `map_trace_event`, the SSRF
  guard) — the agent path must call these, never re-implement them.
- **Diagnostics-only sinks**: `LogTraceSink` never fails a run; a serialisation error logs one
  `error!` line — D-17's guard preserves this contract and its existing test
  (`log_sink_never_returns_err_and_logs_diagnostic`).
- **Evidence files with machine blocks** (`28-BENCH-EVIDENCE.md`) and **WINDOWS rows closed
  through the ledger tool** with the test/evidence named.
- **Docs-currency gate**: every quoted config snippet in `docs/src` and `.devcontainer` must
  match the live file (Phase 34); `make docs` before commit.
- **CI-attributed evidence**: the authoring sandbox has no Docker and cannot pull images; UAT
  records CI run ids as Phase 40 did.

### Integration Points
- `.github/workflows/ci.yml` lines ~697-760 and ~1409-1480 (the two `minio` service blocks, `mc`
  install and bucket steps), ~859-861 (compose services `minio-test minio-test-init`), ~1805-1821
  (smoke-test secret + `kubectl apply -f k8s/minio.yaml` + wait), ~1899-1916 (E2E `.env` and
  health curl).
- `docker/docker-compose.test.yml` (`minio-test`, `minio-test-init`), `docker/docker-compose.yml`
  (`minio`, `minio-init`), `docker/docker-compose.dev.yml`, `.devcontainer/docker-compose.yml:92`.
- `k8s/minio.yaml` → `k8s/rustfs.yaml`; `k8s/deployment.yaml` (init container, env),
  `k8s/configmap.yaml` (`minio.endpoint`), `k8s/README.md`.
- Root `Cargo.toml` `testcontainers-modules` features; `deny.toml` needs no change (Apache-2.0
  is allow-listed; no new crate).
- `Makefile` targets `test-integration-minio`, `services-up`, `minio-console`, help text.
- `WINDOWS.md` rows 31 and 35; `.planning/decisions/PROMOTION.md`; `CHANGELOG.md`.

</code_context>

<specifics>
## Specific Ideas

- The phase's **first work item is the RustFS verification**: pin, bring the contract suite up
  against it in CI, and read the result before any adapter or second-adapter work — the
  roadmap's research flag and the research summary both say so, and D-08 makes the CI run the
  gate.
- "Bucket bootstrap replaces `mc`" is satisfied by **code that already exists**
  (`ensure_bucket_exists`), so the phase deletes bootstrap machinery rather than porting it.
- Parity for agent runs means **the same `map_trace_event` output**, so a subscriber cannot tell
  an agent run's `done` from a graph run's except by payload content.
- The tracing re-measure must be **comparable to Phase 28's numbers**: same bench, same flags,
  same three variants, raw criterion output pasted, machine block included.

</specifics>

<deferred>
## Deferred Ideas

- **Rename `MinioAdapter`/`MinioConfig`/the `minio:` config section/`APP_MINIO_*`/`TEST_MINIO_*`
  to an S3-neutral name** — a public API and operator-config break (X-03; `MIGRATION.md` rows, a
  docs sweep). Belongs to a future clean-break phase, not this swap.
- **Re-scope the tracing bar to an I/O-bound superstep** (Phase 29 D-16's recorded follow-up) —
  an added bench variant with a simulated LLM latency; only worth doing if D-19 records a miss.
- **`rust-s3` 0.35.1 → 0.37.x housekeeping bump** — independent of this phase; its own change
  with its own breaking-change check.
- **Resolve-then-connect address pinning for the webhook SSRF guard** (FUT-13) — untouched.
- **A `paladin-storage` `rustfs` feature / native RustFS SDK adapter** — only if the D-08 gate
  fails; otherwise not built.
- **Multi-arch verification of the RustFS image in the release-image workflow** — the researcher
  confirms the image is multi-arch; adding an arm64 CI leg is out of scope.

### Reviewed Todos (not folded)
None — all three matched todos were folded (the coverage todo in its docs sense only; its
Docker-capable walk stays user-owned and open).

</deferred>

---

*Phase: 45-rustfs-swap-platform-observability-deviations*
*Context gathered: 2026-09-29*
