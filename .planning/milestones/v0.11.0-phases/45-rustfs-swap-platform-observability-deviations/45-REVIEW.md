---
phase: 45-rustfs-swap-platform-observability-deviations
reviewed: 2026-09-30T00:00:00Z
depth: standard
files_reviewed: 46
files_reviewed_list:
  - .devcontainer/CI-CD.md
  - .devcontainer/FILES.md
  - .devcontainer/QUICKSTART.md
  - .devcontainer/README.md
  - .devcontainer/SETUP_COMPLETE.md
  - .devcontainer/devcontainer.json
  - .devcontainer/docker-compose.yml
  - .devcontainer/post-create.sh
  - .devcontainer/validate.sh
  - .github/actionlint.yaml
  - .github/workflows/ci.yml
  - benches/engine_benchmarks.rs
  - crates/paladin-battalion/src/engine/hooks.rs
  - crates/paladin-ports/CHANGELOG.md
  - crates/paladin-ports/src/output/trace_sink_port.rs
  - crates/paladin-storage/CHANGELOG.md
  - crates/paladin-storage/src/minio.rs
  - docker/docker-compose.dev.yml
  - docker/docker-compose.test.yml
  - docker/docker-compose.yml
  - docs/src/SUMMARY.md
  - docs/src/api-reference/platform-api.md
  - docs/src/appendix/integration-tests.md
  - docs/src/appendix/minio-file-repository-setup.md
  - docs/src/contributing/branching-model.md
  - docs/src/contributing/testing-guide.md
  - docs/src/deployment/cicd.md
  - docs/src/deployment/docker.md
  - docs/src/deployment/kubernetes.md
  - docs/src/operations/observability.md
  - docs/src/operations/troubleshooting.md
  - k8s/README.md
  - k8s/configmap.yaml
  - k8s/deployment.yaml
  - k8s/rustfs.yaml
  - k8s/secret.yaml.example
  - scripts/coverage.sh
  - scripts/run_integration_tests.sh
  - src/application/services/run/webhook/mod.rs
  - src/application/services/run/worker.rs
  - src/application/services/run/worker_tests.rs
  - src/infrastructure/telemetry/herald_sink.rs
  - src/infrastructure/telemetry/log_sink.rs
  - tests/integration/cli_real_services_test.rs
  - tests/integration/file_storage_integration_tests.rs
  - tests/integration/mod.rs
findings:
  critical: 1
  warning: 6
  info: 8
  total: 15
status: issues_found
---

# Phase 45: Code Review Report

**Reviewed:** 2026-09-30
**Depth:** standard
**Files Reviewed:** 46 (the config lists 46 paths; the frontmatter count matches the list above)
**Status:** issues_found

## Summary

Rust attention went to `minio.rs`, `worker.rs` (the `run_agent` rewrite plus `persist_failure` and
`enqueue_webhook_delivery`), `log_sink.rs`, `trace_sink_port.rs`, `hooks.rs`, `herald_sink.rs`, both
integration test files and the bench. YAML, shell and docs were checked at pattern-match depth.

Checks that passed:
- `decode_multipart_token` re-validates the decoded key through `validate_path`. `split_token` uses
  `str::get`, so a cut UTF-8 boundary or an oversized length returns `InvalidPath` and never panics.
- `LogTraceSink` is still a `Copy` unit struct. The thread-local buffer is borrowed with
  `try_borrow_mut`, is never held across an `.await`, and has a bounded retention of 64 KiB.
- Gapless `seq` is preserved. `emit` remains the single stamping point, and the `RunFinished`
  `trace_dropped_total: 0` hardcode in `run_agent` is overwritten by `TraceDispatcher::emit`, so it is
  not a defect.
- The `rustfs/rustfs:1.0.0` tag is consistent across ci.yml (two service blocks), all compose files,
  k8s/rustfs.yaml, the devcontainer and the tests. Health paths are `/health` and `/health/ready`.
  No live `minio` image or `mc` reference remains. The remaining `minio:` hits are application-side
  config key names (D-04), not live image or `mc` references.
- The `copy_object_internal` fix is correct against rust-s3 0.35.1: the crate prepends `<bucket>/` itself.

Concerns are concentrated in three areas:
1. The multipart error handling makes claims about rust-s3 behaviour that the crate source contradicts.
2. One pre-existing `list_files` bug sits in the same adapter the phase "completed".
3. The agent-run failure path persists and logs raw error text, and a new test endorses that.

## Critical Issues

### CR-01: `list_files` passes `limit` as the S3 *delimiter*, so `limit` is ignored and keys are silently dropped

**File:** `crates/paladin-storage/src/minio.rs:550-556` (also `:677`)
**Issue:** `Bucket::list(prefix, delimiter)` in rust-s3 0.35.1 takes a **delimiter** as its second
argument (`bucket.rs:2085`), not `max_keys`. `list_files` does
`max_keys = options.limit.map(|l| l.to_string())` and passes it there.
- With `limit = Some(5)` the request becomes `delimiter=5`.
- Every object whose key contains `5` after the prefix is rolled up into `CommonPrefixes` and vanishes
  from `contents`.
- The limit itself is never applied, because `list` pages through everything.
- `health_check` has the same mistake (`Some("1".to_string())`). It is harmless there only because it
  ignores the result.

This is pre-existing, not introduced by this phase. It is still incorrect behaviour in a
`FileStoragePort` method the phase claims to have brought under a complete contract suite.
**Fix:**
```rust
// Use list_page, which exposes max_keys, and honour the limit explicitly.
let (page, _) = self
    .execute_with_retry(|| {
        self.bucket.list_page(prefix.clone(), None, None, None, options.limit)
    })
    .await
    .map_err(|e| FileStorageError::IoError(format!("Failed to list files: {}", e)))?;
let results = vec![page];
```
Also add a contract test that uploads `a5`, `b5` and `c` and lists with `limit: Some(2)`.

## Warnings

### WR-01: Multipart docs claim rust-s3 auto-aborts on a rejected part; with default features it does not, so uploads leak

**File:** `crates/paladin-storage/src/minio.rs:976-980, 1004-1009, 1041-1045`
**Issue:** The doc comments on `upload_part`, `complete_multipart_upload` and `abort_multipart_upload`
say `rust-s3` aborts the whole upload itself when a part is rejected, so a later abort reports
`FileNotFound`.

In rust-s3 0.35.1 (default features include `fail-on-err`), `request.response()` returns
`Err(S3Error::HttpFailWithBody(..))` for any non-2xx status (`tokio_backend.rs:155`). In
`put_multipart_chunk`, `response_data(true)?` propagates that error **before** the
`!(200..300).contains(..)` branch that calls `abort_upload` (`bucket.rs:1510-1521`). The auto-abort
branch is therefore unreachable.

Consequences:
- A failed part leaves the multipart upload open.
- Callers who trust the doc will skip `abort_multipart_upload`, leaking incomplete-upload storage.
- The `status_code() >= 300` check in `complete_multipart_upload` is also unreachable for the same reason.

**Fix:** Correct the docs to say a rejected part leaves the upload open and the caller should abort.
Only keep the auto-abort claim if a test against RustFS proves it. Drop or justify the
`status_code() >= 300` branch.

### WR-02: Multipart error paths embed the raw S3 response body; the doc says "never the body"

**File:** `crates/paladin-storage/src/minio.rs:999, 1026-1028, 1056-1059, 969-971`
**Issue:** `complete_multipart_upload`'s doc says "only the bounded S3 error code is reported, never
the body", and `s3_error_code` exists for that purpose. But every non-2xx response reaches the caller
as `S3Error::HttpFailWithBody(status, body)`, and the `map_err(|e| ... {e})` closures format it with
`Display`. That prints the full body, e.g. `Http request returned 403 with error message: <Error>...`.
- The `s3_error_code` path therefore only covers the rare "200 with an `<Error>` body" case.
- The `abort_multipart_upload` fallback and `upload_part` embed bodies unbounded and unredacted.
- This contradicts the project's redact-then-truncate rule for response bodies embedded in errors.
  `SignatureDoesNotMatch` and `InvalidAccessKeyId` bodies echo the access key id and request details.

The same pattern pre-exists across the adapter (`upload_file`, `download_file`, and so on), so the new
methods copy it.
**Fix:** Add one helper, e.g. `fn s3_err_summary(e: &S3Error) -> String`.
- Return `HTTP <status> <code>` for `HttpFailWithBody(status, body)`, using `s3_error_code(body)`.
- Return a fixed string for other variants.

Use it in all multipart `map_err` closures, then in the remaining adapter sites.

### WR-03: Integration test can print presigned URLs on failure through `reqwest::Error`

**File:** `tests/integration/file_storage_integration_tests.rs:407, 423-428, 435`
**Issue:** The test is careful to print only the URL up to `?`. But `http.get(&download_url).send().await?`
converts a `reqwest::Error` into `Box<dyn Error>`. `reqwest::Error`'s `Display` includes the full URL,
e.g. `error sending request for url (http://...?X-Amz-Credential=...&X-Amz-Signature=...)`. A
connection failure or timeout therefore prints the live signed URL (access key id and signature) into
CI logs, contradicting the "presigned URLs never printed" rule.
**Fix:**
```rust
let response = http
    .get(&download_url)
    .send()
    .await
    .map_err(|e| e.without_url())?;
```
Apply the same to the PUT and the read-back GET. Optionally wrap this in a small `send_redacted` helper.

### WR-04: `run_agent` publishes the terminal wire event before the durable status write; a failed write or nack leaves a false terminal event

**File:** `src/application/services/run/worker.rs:1367-1442`
**Issue:** `RunFinished { Completed }` or `RunFinished { Failed }` is emitted into the dispatcher, and
so onto the SSE bus and the persisted trace, before `update_status`, `record_outcome` and `ack`.
- If the repository write fails on the success path, `?` returns a `WorkerError`. Subscribers already
  saw `done` and the trace says `Completed`, but the run row is still `Running`.
- On the `persist_failure` repository-error branch the run is nacked and retried, yet `error` was
  already emitted as terminal.
- A retry attempt builds a fresh dispatcher whose `seq` restarts at 1 on the same `thread_id`, so
  persisted `run_traces` can collide or duplicate.

The graph path has the same structural ordering because the engine emits `RunFinished` itself. The new
code is, however, the first place where the ordering is entirely under the worker's control.
**Fix:** Emit the terminal `RunFinished` after the status write succeeds. On a nack, do not emit a
terminal event (emit a non-terminal marker, or nothing) and unbind. At minimum, document the window,
and have the retry path bind a new `run_id`/attempt-scoped trace key.

### WR-05: Agent failure text is persisted and logged unredacted, and a new test pins that as a contract

**File:** `src/application/services/run/worker.rs:1442, 1530, 1547-1550`;
`src/application/services/run/worker_tests.rs:1128, 1250`
**Issue:** `persist_failure(leased, run, error.to_string())` stores the `PaladinError` text verbatim on
the run row. On a repository error it also logs `(original error: {error_text})` at `warn`.
- LLM-adapter errors can embed provider response bodies, and project rules require redact-then-truncate
  before embedding them.
- The new test uses a secret-shaped marker (`sk-agent-secret-marker`) and asserts the raw text is
  readable through `GET /runs/{id}` (`Some(format!("Execution error: {AGENT_FAILURE_TEXT}"))`). That
  locks unredacted persistence in as intended behaviour.
- The wire event and webhook are correctly clean (`message: null`). The run row and the log line are not.

**Fix:** Pass `error_text` through the `paladin-llm` redaction helper (`crates/paladin-llm/src/redaction.rs`)
before persisting and logging. Redact first, then bound length. Update the test to assert the marker is
**not** present in `failed.error`. Drop the error text from the `warn!` line, or log only a redacted
form.

### WR-06: Failed agent run reports zero usage and cost to `RunFinished`, the herald and the trace

**File:** `src/application/services/run/worker.rs:1422-1441`
**Issue:** On `Err`, `NodeFinished` is emitted with `usage: TokenUsage::default()` and `cost: None`,
and `RunFinished` then reports `dispatcher.total_usage()`, which is therefore zero.
- An agent loop that failed on its third iteration has still consumed tokens and money.
- The Herald summary and persisted trace for failed agent runs will under-report spend.
- This diverges from the graph path, where per-node usage is tallied as it accrues.
- The test only covers the success shape.

**Fix:** Have the execution path surface partial usage on error, or tally it from the
`RUN_TRACE_EMITTER` events the agent loop already emits. If that is out of scope, document that failed
agent runs report zero usage. Add a test asserting the documented behaviour.

## Info

### IN-01: `execute_with_retry` retries permanent errors; `download_file` maps every failure to `FileNotFound`

**File:** `crates/paladin-storage/src/minio.rs:361-391, 454-459, 511-514`
**Issue:** 404, 403 and 400 responses are retried up to `max_retries` times with backoff. A missing
object costs about 1 s of sleeps in `get_file_info` and `download_file`. Timeouts, auth failures and
transport errors are all reported as `FileNotFound`. This is pre-existing.
**Fix:** Classify `HttpFailWithBody(4xx, _)` as non-retryable and map the status to the matching
`FileStorageError` variant.

### IN-02: `MinioConfig::path_style` is dead; `Bucket::with_path_style()` is unconditional

**File:** `crates/paladin-storage/src/minio.rs:161-165`
**Issue:** The config field is never read, although the docs tell operators to "keep `path_style: true`".
Setting it to `false` silently still uses path style.
**Fix:** Either honour it (`if config.path_style { bucket.with_path_style() }`) or document that it is
ignored and remove it.

### IN-03: `expires_in.as_secs() as u32` wraps silently

**File:** `crates/paladin-storage/src/minio.rs:917, 938`
**Issue:** A `Duration` above `u32::MAX` seconds wraps modulo 2^32, which can yield a near-zero expiry
instead of an error. S3 caps presigned expiry at 7 days anyway.
**Fix:** `u32::try_from(expires_in.as_secs()).map_err(|_| FileStorageError::InvalidPath(..))?`, and
optionally reject values over 604800.

### IN-04: `upload_files` comment says "concurrently" but the futures are awaited sequentially

**File:** `crates/paladin-storage/src/minio.rs:791-812`
**Issue:** The `Vec` of lazy futures is awaited in order, so nothing runs concurrently. Pre-existing.
**Fix:** Use `futures::future::try_join_all` or fix the comment.

### IN-05: `composite_sink_hands_an_equal_record_to_its_last_child` cannot detect a clone versus a move

**File:** `crates/paladin-ports/src/output/trace_sink_port.rs:477-507`
**Issue:** The test is named and documented as proving the last child receives the record by move. It
only asserts record equality, which passes identically if the implementation cloned for every child.
The "no clone at all for one child" claim is therefore untested.
**Fix:** Either reword the test to "every child sees an equal record", or assert via a sink that
inspects a pointer or allocation. For example, compare the `graph_fingerprint` string heap pointer
before and after for the single-child case.

### IN-06: `LogTraceSink` buffer reuse adds a full UTF-8 validation pass

**File:** `src/infrastructure/telemetry/log_sink.rs:58-71`
**Issue:** `serde_json::to_writer` into a `Vec<u8>` followed by `str::from_utf8(buf)` re-validates
bytes `serde_json` guarantees are valid. The non-UTF-8 arm is unreachable, and `log::info!` then
formats into its own `String` anyway. The phase's own bench evidence shows no measurable win, so the
buffer mainly adds a thread-local and an unreachable error branch.
**Fix:** Keep it if the maintainers want the retention bound. Otherwise simplify back to
`serde_json::to_string`. If kept, use `String` with `serde_json::to_writer` via a `Write` adapter, or
`from_utf8_unchecked` with a `// SAFETY:` comment.

### IN-07: Dev compose publishes the RustFS console and API on all interfaces with well-known default credentials

**File:** `docker/docker-compose.yml:36-44`, `.devcontainer/docker-compose.yml:97-110`
**Issue:**
- `"9000:9000"` and `"9001:9001"` bind to `0.0.0.0` with `paladin-dev` / `paladin-dev-secret` defaults
  (`docker-compose.dev.yml` uses `devuser` / `devpassword123`) and `RUSTFS_CONSOLE_ENABLE: "true"`.
  k8s/rustfs.yaml itself notes the console CORS default is `*`.
- The prior MinIO setup had the same shape, so this is not a regression. It is still a LAN-exposed
  admin console with guessable credentials.

**Fix:** Bind to loopback, `"127.0.0.1:9000:9000"` and `"127.0.0.1:9001:9001"`, or document the exposure.

### IN-08: Docs and CI small defects

**Issue:**
- `docs/src/appendix/minio-file-repository-setup.md:96` tells readers to run
  `docker compose -f docker/docker-compose.test.yml up --build test-runner`. No `test-runner` service
  exists; the compose service is `integration-tests`. The command was carried through the rewrite.
- `docs/src/operations/troubleshooting.md:505` suggests `kubectl create secret generic paladin-secrets`
  with only four RUSTFS_*/MINIO_* keys. `paladin-secrets` already holds the LLM API keys, so the
  command either fails with AlreadyExists or, if replaced, drops the API keys. Use
  `kubectl patch secret paladin-secrets` or `kubectl create secret ... --dry-run=client -o yaml | kubectl apply -f -`
  with the full key set.
- `docs/src/deployment/kubernetes.md:127` still draws "StatefulSet" for the object store, but
  `k8s/rustfs.yaml` is a single-replica Deployment with `emptyDir`.
- `.github/workflows/ci.yml:1927` collects logs only from `docker/docker-compose.yml`, but the e2e
  failure surface is the `docker-compose.test.yml` `integration-tests`/`rustfs-test` stack. Cleanup
  (`down -v` on the main file) never tears the test stack down. Collect and clean both files.
- The compose and CI health checks (`curl -f http://localhost:9000/health/ready`) assume `curl` exists
  inside the `rustfs/rustfs:1.0.0` image. That was not verifiable from source. If it is absent, every
  `up --wait` and service-container start fails identically. Confirm once with
  `docker run --rm --entrypoint sh rustfs/rustfs:1.0.0 -c 'command -v curl'`.

---

_Reviewed: 2026-09-30_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
