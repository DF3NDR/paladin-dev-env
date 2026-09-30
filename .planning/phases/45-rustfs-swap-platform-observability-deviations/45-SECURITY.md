---
phase: 45
slug: rustfs-swap-platform-observability-deviations
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-09-30
---

# Phase 45 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time — every one of the seven PLAN.md files (45-01 through 45-07) carries a `<threat_model>` block. Verification depth: ASVS L1 (grep-level presence checks against the implementation and phase artifacts), per the short-circuit rule for a plan-time register with `threats_open: 0`. Every SUMMARY.md `## Threat Flags` section reports "None" beyond the plan register.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| port caller -> adapter | multipart upload id is caller-supplied and forgeable | object keys (path-validated) |
| adapter -> S3 endpoint | response bodies, including a 200 with an `<Error>` body, are untrusted text | S3 XML; only `<Code>` (<= 64 chars) re-emitted |
| test harness -> CI log | anything printed by the contract suite lands in public CI logs | presigned URLs (query string stripped) |
| sandbox -> GitHub release | native RustFS binary is a downloaded executable used only as a local fixture | binary, sha256 recorded |
| run submitter -> worker | run `webhook` URL and subscribed events are caller-controlled | webhook URL (SSRF-guarded) |
| worker -> SSE subscriber | wire events leave the process to tenant-scoped readers | ids, status, usage, cost; no failure text |
| worker -> webhook receiver | delivery service sends to a caller-chosen URL with an HMAC header | signed payload |
| trace record -> log output | record content leaves the process through the operator's logger | serialised trace records |
| sink panic -> dispatcher | a panicking child sink must not take down siblings or the run | none (isolation) |
| registry -> CI / compose / cluster | third-party image pulled by tag into every environment | `rustfs/rustfs:1.0.0` |
| workflow -> public CI log | step output and env are visible to anyone who can read the run | throwaway test credentials |
| cluster network -> object store | in-cluster clients reach the store's S3 API | S3 traffic; console disabled |
| developer host -> dev stack | dev compose publishes the S3 API and console on host ports | dev-only credentials |
| developer `.env` -> compose | untracked local env files feed store and app credentials | dev credentials |
| repo docs -> operators | quoted config and credentials in the book are copied into real deployments | placeholders only |
| planning record -> future phases | the ADR and closed todos are what later phases trust | commit references |
| maintainer machine -> planning record | measured numbers enter the record by paste | raw criterion output |
| ledger -> ship gate | an open `WINDOWS.md` row blocks `/gsd-ship` | row disposition |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-45-01 | Tampering | `decode_multipart_token` / multipart methods | high | mitigate | Decoded keys re-validated; tests `split_token_rejects_malformed_tokens` and `decode_multipart_token_rejects_a_traversal_key` present in `crates/paladin-storage/src/minio.rs` | closed |
| T-45-02 | Information disclosure | `test_presigned_urls` output | medium | mitigate | `tests/integration/file_storage_integration_tests.rs` prints only `url.split('?').next()` for download and upload URLs | closed |
| T-45-03 | Information disclosure | `complete_multipart_upload` error path | low | mitigate | `s3_error_code` in `minio.rs` extracts only `<Code>`, bounded to 64 chars, and is what the error path embeds | closed |
| T-45-04 | Tampering (supply chain) | `GenericImage` local-mode pin | medium | mitigate | `RUSTFS_IMAGE` pinned to `rustfs/rustfs` tag `1.0.0` with manifest-list digest comment; no `:latest`/preview tag anywhere in the tree | closed |
| T-45-05 | Spoofing | harness credentials | low | accept | Throwaway `testuser`/`testpass123` in CI and local mode; the RustFS default credential literal does not appear in the tree | closed |
| T-45-06 | Tampering | native RustFS fixture binary | low | accept | Sandbox-only fixture; sha256 recorded in 45-01-SUMMARY.md; never committed | closed |
| T-45-07 | Spoofing (SSRF) | agent-run webhook deliveries | high | mitigate | Agent runs enqueue only via `webhook_delivery_for_outcome` in `worker.rs`; `WebhookDeliveryService::process` re-runs `SsrfGuard::check_url` at send time; `ssrf.rs`, `service.rs`, `client.rs` have no diff across the phase | closed |
| T-45-08 | Information disclosure | agent failure `error` wire event | medium | mitigate | `agent_kind_run_emits_exactly_one_terminal_event` in `worker_tests.rs` asserts the terminal event with null message | closed |
| T-45-09 | Information disclosure | agent-run trace records | medium | mitigate | `worker_tests.rs` asserts trace payloads carry `node_id`, `outcome`, `status`, `usage` only; no input/output text keys | closed |
| T-45-10 | Repudiation / integrity | duplicate terminal events or deliveries | low | mitigate | Exactly-once tests: `agent_kind_run_emits_exactly_one_terminal_event`, `agent_kind_run_with_a_webhook_enqueues_a_delivery`, `agent_kind_run_failure_enqueues_a_failed_delivery` | closed |
| T-45-11 | Denial of service | bus channel left bound on a repository error | low | mitigate | `run_agent` captures the persistence result, sleeps `TRACE_DRAIN_GRACE_PERIOD`, then always calls `bus.unbind` before returning (`worker.rs`, comment cites T-45-11) | closed |
| T-45-12 | Denial of service | thread-local `TRACE_BUF` | low | mitigate | `TRACE_BUF_RETAIN_MAX = 64 KiB` release in `log_sink.rs`; test `log_sink_writes_an_oversize_record_then_a_small_record_exactly` asserts capacity bound | closed |
| T-45-13 | Information disclosure | `LogTraceSink` error paths | low | mitigate | `log_sink.rs` `error!` lines name only the serializer/UTF-8 error, never record content (doc comment and both call sites) | closed |
| T-45-14 | Tampering (integrity) | `TraceDispatcher::emit` | medium | mitigate | Phase diff on `crates/paladin-battalion/src/engine/hooks.rs` is 13 doc-comment lines only; no enablement guard, `seq` stamp and tallies untouched | closed |
| T-45-15 | Denial of service | `RefCell` re-entrancy / composite child panic | low | mitigate | `try_borrow_mut` with fresh-buffer fallback in `log_sink.rs`; each composite child wrapped in `catch_unwind` in `trace_sink_port.rs` | closed |
| T-45-16 | Tampering (supply chain) | `rustfs/rustfs` image in CI, compose, k8s | high | mitigate | Exact tag `1.0.0` in `ci.yml` (2 jobs), `docker-compose.yml`, `docker-compose.test.yml`, `k8s/rustfs.yaml`; digest re-verified in 45-04; CI pod Image ID matches the recorded digest (UAT test 1) | closed |
| T-45-17 | Repudiation (false assurance) | CI contract-suite steps | high | mitigate | `ci.yml` uses `set -o pipefail` + `tee`, parses the passed count and fails if `< 11`; UAT test 1 recorded "11 passed" | closed |
| T-45-18 | Information disclosure | CI credentials and logs | medium | mitigate | Throwaway literals only; no `set -x` in any workflow; suite prints no presigned query string (T-45-02) | closed |
| T-45-19 | Elevation of privilege | RustFS console with wildcard CORS | medium | mitigate | `RUSTFS_CONSOLE_ENABLE: "false"` in `ci.yml` (both jobs), `docker-compose.test.yml`, `k8s/rustfs.yaml`; k8s exposes only `containerPort: 9000` | closed |
| T-45-20 | Elevation of privilege | k8s RustFS pod | low | mitigate | `k8s/rustfs.yaml`: `runAsNonRoot: true`, uid/gid/fsGroup `10001`, `allowPrivilegeEscalation: false` | closed |
| T-45-21 | Information disclosure | `k8s/secret.yaml.example` | low | mitigate | Base64 placeholders `paladin-store` / `change-me-store-secret` with a PLACEHOLDERS replace-me comment | closed |
| T-45-22 | Denial of service | tmpfs `/data` ownership vs uid 10001 | low | mitigate | `docker-compose.test.yml` mounts `tmpfs: - /data:mode=1777` | closed |
| T-45-SC | Tampering | package installs (npm/pip/cargo) | high | mitigate | No package added: the only lockfile change in the phase (commit a2b22b1d) is a 10-line removal of `testcontainers-modules`; the pinned image is covered by T-45-16 | closed |
| T-45-23 | Information disclosure | anonymous-readable `paladin-files` bucket | medium | mitigate | No init container or public/anonymous policy string remains in `docker/` or any compose file (grep empty) | closed |
| T-45-24 | Tampering / integrity | old MinIO-formatted named volumes | low | mitigate | Dev compose uses new volume `rustfs_data`; k8s uses an `emptyDir` (`data`, 10Gi); storage page states old volumes are not migrated | closed |
| T-45-25 | Spoofing | default dev credentials | low | accept | `.env.example` carries `paladin-dev` / `paladin-dev-secret`, overridable from `.env`; dev stack only; not the image default | closed |
| T-45-26 | Elevation of privilege | RustFS console on host port 9001 | low | accept | Dev-only (`RUSTFS_CONSOLE_ENABLE: "true"` only in `docker/docker-compose.yml`); disabled in CI, test compose and k8s (T-45-19); recorded in ADR-0055 | closed |
| T-45-27 | Information disclosure | credentials quoted in docs and CHANGELOG | low | mitigate | Storage page quotes only `.env` placeholders, tells operators to use a real secret store, a Kubernetes Secret and a least-privilege IAM user | closed |
| T-45-28 | Repudiation | todo closures and "no MinIO left" claim | low | mitigate | 45-06-SUMMARY.md records the removing commits, the phase-wide grep and the one-tag invariant as re-runnable evidence | closed |
| T-45-29 | Repudiation | benchmark figures | medium | mitigate | Raw criterion output (`45-07-bench-A/B/C/C-run1.txt`), `45-07-machine-block.txt` and per-point SHAs recorded verbatim in `45-BENCH-EVIDENCE.md`; `blocking-human` gate | closed |
| T-45-30 | Repudiation | WINDOWS.md row 35 disposition | medium | mitigate | Row 35 stays waived; amend row 61 stayed open until the maintainer accepted at UAT test 4 (2026-09-30) and was then waived through `gsd-tools windows waive` with the acceptance text | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-45-01 | T-45-05 | Test-harness credentials are throwaway literals shared with CI; never the image default, never a real key (D-00a) | 45-01-PLAN threat model | 2026-09-30 |
| R-45-02 | T-45-06 | Native RustFS binary is an executor-sandbox-only fixture, never committed or shipped; sha256 recorded in 45-01-SUMMARY.md | 45-01-PLAN threat model | 2026-09-30 |
| R-45-03 | T-45-25 | Dev-stack default credentials are throwaway literals overridable from `.env`; dev compose only | 45-05-PLAN threat model | 2026-09-30 |
| R-45-04 | T-45-26 | RustFS console on host port 9001 is a dev-only convenience with non-default credentials; disabled in CI, test compose and k8s; ADR-0055 | 45-05-PLAN threat model | 2026-09-30 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-30 | 31 | 31 | 0 | /gsd-secure-phase (L1 grep-depth, orchestrator; dispatched from /gsd-verify-work 45) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-30
