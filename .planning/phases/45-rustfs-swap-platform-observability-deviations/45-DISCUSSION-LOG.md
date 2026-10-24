# Phase 45: RustFS Swap & Platform/Observability Deviations - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-29
**Phase:** 45-rustfs-swap-platform-observability-deviations
**Mode:** `--auto` — every question was answered with the recommended option without a user
prompt; the user reviews the choices here and in CONTEXT.md.
**Areas discussed:** Sequencing & interim pin, RustFS pin & bucket bootstrap, Contract-suite gate &
multipart gap, Production manifest & decision record, Legacy-agent SSE/webhook wiring,
Tracing-overhead fix & acceptance record

---

## Sequencing & interim pin

| Option | Description | Selected |
|--------|-------------|----------|
| Phase 45 now; interim re-pin superseded | Run 45 ahead of 41-44; never re-pin a MinIO image; close both MinIO todos via this phase's SUMMARY; re-verify Phase 40 UAT test 4 afterwards | ✓ |
| Interim re-pin first (quick task) | Land the 2026-09-29 todo's Chainguard/ghcr/authenticated-quay re-pin to restore CI, then run 41-44, then 45 | |
| Pull only the image-swap slice ahead | Split STORE-01 out as a quick task; leave STORE-02/03, PLAT-08, OBS-05 sequenced last | |

**Auto choice:** Phase 45 now; interim re-pin superseded (recommended).
**Notes:** The maintainer invoking `/gsd-discuss-phase 45` with 41-44 unplanned is read as the
roadmap decision the Phase 40 UAT session deferred. The "Depends on: Phase 44" line is a
diff-conflict preference with nothing to conflict against yet. → D-01, D-02.

---

## RustFS pin & bucket bootstrap

| Option | Description | Selected |
|--------|-------------|----------|
| Exact GA tag + digest comment; adapter-owned bucket creation; `mc` machinery deleted | `rustfs/rustfs:<tag>` with the manifest digest in a comment (house pattern); `ensure_bucket_exists` is the bootstrap; drop init containers and CI `mc` steps | ✓ |
| `@sha256` digest in the image reference | Pin by digest in every image ref | |
| `rustfs/rc` init containers replace `mc` | Keep the init-container shape with RustFS's own client image | |
| `curl`-based S3 `PUT /bucket` in an alpine container | Shell-only bootstrap (needs SigV4 signing in shell) | |

**Auto choice:** Exact tag + digest comment; adapter-owned bootstrap; `mc` deleted (recommended).
**Notes:** One fewer third-party image to pin; the bootstrap is exercised by the suite. Fallback
to a pinned `rc` container only if RustFS rejects the adapter's bucket create. Service-side env
names follow RustFS; application-side names unchanged. → D-03…D-07.

---

## Contract-suite gate & multipart gap

| Option | Description | Selected |
|--------|-------------|----------|
| Fill the three multipart stubs with `rust-s3 0.35.1`; add multipart + opaque-ETag + exercised-presign cases; gate = suite green in CI | Make the roadmap's "including multipart uploads and ETags" true before using the suite as the reuse-vs-new-adapter gate | ✓ |
| Run the suite as-is; record multipart as untested | Cheapest; leaves criterion 2 unmeasured for multipart | |
| Write the second adapter up front behind a feature flag | The 2026-09-13 todo's original sketch | |
| Assert ETag == MD5 of content | Strict content-integrity check | |

**Auto choice:** Fill the stubs, complete the suite, CI is the gate (recommended).
**Notes:** ETag is treated as an opaque stable token (Pitfall 11: multipart ETags are composite
and may differ on RustFS). No adapter/config renames in this phase. → D-08…D-10.

---

## Production manifest & decision record

| Option | Description | Selected |
|--------|-------------|----------|
| One manifest renamed `k8s/rustfs.yaml` serves smoke test and reference; ADR-0055 records it | The smoke test applies the reference manifest, so it follows; storage doc keeps its path and cites the ADR | ✓ |
| Split a smoke-only RustFS manifest; keep `k8s/minio.yaml` as the production reference | Two manifests; the reference stays on an unpullable image | |
| Delete the bundled object-store manifest; reference deployments use external S3 only | Removes the reference store entirely | |
| Record the decision in docs only, no ADR | Lighter bookkeeping | |

**Auto choice:** One renamed manifest + ADR-0055 (recommended). → D-11…D-13.

---

## Legacy-agent SSE/webhook wiring

| Option | Description | Selected |
|--------|-------------|----------|
| Same machinery: `build_run_sink` + `TraceDispatcher` + `RunEventBusSink`; `run_agent` emits `RunStarted`/`RunFinished`; webhook via `webhook_delivery_for_outcome` after ack | Wire-level parity through the ONE `map_trace_event`; the agent loop's own trace events reach the bus | ✓ |
| Minimal: `bus.bind` + direct `publish(done/error)` + `enqueue` | Smallest diff; hand-built `done` payload diverges from graph runs | |
| Route agent runs through `WarEngine` as a one-node graph | Full parity by construction; a much larger change with ledger-scope implications | |

**Auto choice:** Same machinery (recommended).
**Notes:** Exactly one terminal wire event per agent run (test-pinned); `record_engine_failure`
gains the `Failed` webhook enqueue, which also fixes the graph path's `Err` branch; PLAT-09
(cancel/halt `done` status) stays Phase 42's. → D-14…D-16.

---

## Tracing-overhead fix & acceptance record

| Option | Description | Selected |
|--------|-------------|----------|
| `log_enabled!` guard + `to_writer` buffer reuse; same bench fixture/command; `45-BENCH-EVIDENCE.md`; miss → new accepted figure on row 35 | The research's two no-new-dependency fixes, measured comparably to Phase 28 | ✓ |
| Also add an I/O-bound superstep bench variant now | Phase 29's deferred re-scope, done in this phase | |
| Replace `serde_json` with `sonic-rs`/`simd-json` in the sink | SIMD JSON | |
| Buffer/sample trace events ahead of the sink | Throughput win at the cost of gaplessness | |

**Auto choice:** Two fixes, same bench, record either outcome (recommended). → D-17…D-19.

---

## Claude's Discretion

- `GenericImage` wait strategy; console port mappings.
- `LogTraceSink` `Copy` vs. `Mutex` buffer (prefer thread-local to keep the public shape).
- Agent-run `RunStarted` payload sentinel; optional per-call `NodeStarted`/`NodeFinished`.
- E2E `.env` variable names; `docker-compose.dev.yml` image inheritance.
- Multipart/ETag test topology and part size.
- Plan split and wave order (RustFS verification first; agent wiring and tracing independent).

## Deferred Ideas

- S3-neutral rename of `MinioAdapter`/`MinioConfig`/`minio:`/`APP_MINIO_*`/`TEST_MINIO_*` (X-03 break).
- I/O-bound re-scope of the tracing bar (Phase 29 D-16 follow-up) — only if D-19 records a miss.
- `rust-s3` 0.35.1 → 0.37.x housekeeping bump.
- Webhook SSRF DNS-rebinding pinning (FUT-13).
- `rustfs` feature / native SDK adapter — only if the D-08 gate fails.
- arm64 CI leg for the RustFS image.
