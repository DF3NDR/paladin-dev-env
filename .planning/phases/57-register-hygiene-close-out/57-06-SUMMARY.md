---
phase: 57-register-hygiene-close-out
plan: 06
subsystem: storage-s3-adapter
tags: [s3, rustfs, list-objects, credential-redaction, multipart, windows-ledger, d-14]
status: complete

requires:
  - phase: 57-register-hygiene-close-out
    provides: "the Phase 57 sweep's plan-time liveness check that found 45-REVIEW CR-01, WR-01, WR-02 and WR-03 still live in the S3 adapter and its RustFS contract suite"
provides:
  - "list_files honours ListOptions.limit as the S3 page size (max-keys) and pages with continuation_token; no key is dropped and has_more / continuation_token come from the store's truncation state"
  - "s3_err_summary: every place the S3 adapter renders an S3Error now reports only 'HTTP <status> <code>' or a fixed label, never a response body"
  - "multipart rustdoc that says a rejected part leaves the upload open and the caller must abort"
  - "a presigned-URL integration test that cannot print a signed URL"
  - "WINDOWS rows 90 (CR-01), 91 (WR-02), 92 (WR-01) and 93 (WR-03), all open, for 57-10 and 57-11 to close"
affects: [57-10, 57-11]

tech-stack:
  added: []
  patterns:
    - "pure request-mapping seam (list_page_request) so an adapter's wire arguments are unit-tested without a store"
    - "one bounded error-summary helper for every credential-adjacent upstream error, reusing the existing s3_error_code bound"

key-files:
  created: []
  modified:
    - .planning/WINDOWS.md
    - crates/paladin-storage/src/minio.rs
    - tests/integration/file_storage_integration_tests.rs
    - crates/paladin-storage/CHANGELOG.md
    - MIGRATION.md
    - CHANGELOG.md

key-decisions:
  - "PD-06-2 seam: private ListPageRequest and list_page_request(&ListOptions); introduced first with today's (wrong) values so the unit test failed, then fixed. No public item changes."
  - "PD-06-3: has_more and continuation_token come from the page; the filter runs after the fetch, so a filtered page may hold fewer than limit items while has_more is true. Stated in the list_files rustdoc and in M-B-40."
  - "The status >= 300 branch in complete_multipart_upload is kept (defensive), with a comment citing fail-on-err; the <Error> body branch is live because S3 can answer 200 with an error document."
  - "execute_with_retry's exhaustion message was a further raw S3Error rendering the plan did not list; it now goes through s3_err_summary, which also bounds every post-retry map_err site downstream of it."

patterns-established:
  - "An upstream error that can echo a response body is summarised (status and code, or a fixed label) at the point it is first rendered, not at each caller"

requirements-completed: [REG-05, REG-02]

duration: 28min
completed: 2026-11-04
---

# Phase 57 Plan 06: S3 adapter list, error-text and presigned-test hygiene Summary

**`list_files` now sends `limit` as `max-keys` with continuation (never as a delimiter that dropped keys), every S3 error is bounded to `HTTP <status> <code>` via one `s3_err_summary`, the multipart docs say a rejected part must be aborted, and the presigned-URL test can no longer print a signed URL.**

## Performance

- **Duration:** ~28 min
- **Started:** 2026-11-04T16:13Z (first WINDOWS row minted 16:13:05; plan-start SHA `2df4b0fcc`)
- **Completed:** 2026-11-04T16:35Z
- **Tasks:** 3 (1 tracer, 2 auto)
- **Files modified:** 6

## Accomplishments

- 45-REVIEW CR-01 (correctness): `list_files` issues one `list_page(prefix, None, token, None, limit)` when a limit or token is given, returns at most `limit` files, and reports `has_more` / `continuation_token` from the page; with neither it lists every page with no delimiter. `health_check` lists with `max_keys` 1.
- 45-REVIEW WR-02 (credential handling): 12 `s3_err_summary(&e)` call sites; no `S3Error` is formatted with `Display` any more.
- 45-REVIEW WR-01: the three multipart doc blocks no longer promise an auto-abort.
- 45-REVIEW WR-03: five `reqwest` send / body-read sites in `test_presigned_urls` map their error with `.without_url()`.
- Public API surface unchanged; MIGRATION 11.1 gains `M-B-40`; both CHANGELOGs updated.

## Task Commits

1. **Task 1 (tracer): 45 CR-01 end to end** - `b5ba82365` (fix)
2. **Task 2: 45 WR-02 and WR-01** - `6d0de8905` (fix)
3. **Task 3: 45 WR-03, records and gates** - `ac7841931` (fix)

## WINDOWS rows minted

All kind `deviation`, status `open` (57-11 marks them `fixed` on the evidence below). Each was minted before the commit that carries its fix.

| Id | Description starts | File:line |
|----|--------------------|-----------|
| 90 | `45-REVIEW CR-01 (found live by the Phase 57 sweep, D-14): list_files passes ListOptions.limit ... as the delimiter` | `crates/paladin-storage/src/minio.rs:551` |
| 91 | `45-REVIEW WR-02 (found live by the Phase 57 sweep, D-14): every non-2xx S3 response reaches callers as HttpFailWithBody ...` | `crates/paladin-storage/src/minio.rs:999` |
| 92 | `45-REVIEW WR-01 (found live by the Phase 57 sweep, D-14): upload_part / complete_multipart_upload / abort_multipart_upload rustdoc claims rust-s3 auto-aborts ...` | `crates/paladin-storage/src/minio.rs:1008` |
| 93 | `45-REVIEW WR-03 (found live by the Phase 57 sweep, D-14): the presigned-URL test converts reqwest::Error with ? ...` | `tests/integration/file_storage_integration_tests.rs:407` |

## map_err sites

Line numbers are in `crates/paladin-storage/src/minio.rs` at HEAD `ac7841931`.

**Rendered an `S3Error`; now `s3_err_summary(&e)` (12 sites):**

| Line | Site | Error type |
|------|------|-----------|
| 215 | `Bucket::new` "Failed to create bucket" | `S3Error` |
| 258 | `ensure_bucket_exists` "Failed to check bucket" | `S3Error` (match arm, not a `map_err`) |
| 301 | `create_bucket` "Failed to create bucket" | `S3Error` (match arm) |
| 451 | `execute_with_retry` exhaustion "Operation failed after N retries" | `S3Error` (last error) |
| 565 | `file_exists` "Failed to check file existence" | `S3Error` (match arm) |
| 775 | `health_check` "MinIO health check failed" | `S3Error` (match arm; this text is also logged and returned in `StorageHealth.error`) |
| 1007 | `generate_upload_url` (`presign_put`) | `S3Error` |
| 1031 | `generate_download_url` (`presign_get`) | `S3Error` |
| 1058 | `create_multipart_upload` | `S3Error` |
| 1094 | `upload_part` | `S3Error` |
| 1125 | `complete_multipart_upload` | `S3Error` |
| 1161 | `abort_multipart_upload` | `S3Error` |

**Unchanged `map_err` sites, with the reason:**

| Line | Site | Error type | Why unchanged |
|------|------|-----------|---------------|
| 188 | `Credentials::new` "Invalid credentials" | `CredentialsError` (aws-creds) | not an `S3Error`; local config validation, no response body |
| 276 | `bucket.credentials()` "Failed to get credentials" | `CredentialsError` (aws-creds) | not an `S3Error`; no response body |
| 492, 521, 544, 577, 640, 649, 681 | `upload_file`, `download_file`, `delete_file`, `get_file_info`, `list_files` (both branches), `copy_file` | `FileStorageError` | these wrap the result of `execute_with_retry`, whose exhaustion text is now bounded (line 451), so they render already-summarised text |

The plan's interface list named `:429, :457, ...` sites; the post-retry ones turned out to wrap `FileStorageError`, not `S3Error`, which is why the bound had to go in `execute_with_retry` rather than at each closure.

## Red-green

| Test | RED observed | GREEN |
|------|--------------|-------|
| `minio::tests::list_request_sends_the_limit_as_max_keys_never_as_a_delimiter` | Runtime failure at `minio.rs:1192` with the seam returning today's request: `assertion left == right failed: the limit must never be a delimiter`, `left: Some("5")`, `right: None` | passes |
| `minio::tests::s3_err_summary_never_echoes_the_response_body` | Compile failure E0425: `cannot find function s3_err_summary in this scope` (the helper did not exist; a second E0308 in my first draft of the test was fixed before GREEN) | passes |
| `minio::tests::retry_exhaustion_text_never_echoes_the_response_body` | Added after the helper, as a regression guard for the `execute_with_retry` site the plan did not list; passes on first run, so no separate RED was observed | passes |

## Gates

- `cargo test -p paladin-storage --features s3`: `test result: ok. 320 passed; 0 failed` (the 17 `minio::` tests are among them).
- Tracer `<verify>` re-run after the tracer commit: unit test, all `minio::` tests, `--list` showing the new case and 12 cases, `list_page(` grep, WINDOWS row grep: all passed.
- `cargo test --test lib --features integration-tests,s3-storage file_storage_integration_tests -- --list`: `12 tests, 0 benchmarks`, including `integration::file_storage_integration_tests::tests::test_list_files_honours_limit_and_drops_no_keys: test`. CI's at-least-11 guard is satisfied.
- `cargo test --test lib --features integration-tests,s3-storage --no-run`: compiles.
- `cargo clippy -p paladin-storage --features s3 --all-targets -- -D warnings`: no warnings.
- `cargo clippy --test lib --features integration-tests,s3-storage -- -D warnings`: no warnings.
- `RUSTDOCFLAGS="-D warnings" cargo doc -p paladin-storage --features s3 --no-deps --document-private-items`: clean.
- `cargo fmt --all -- --check`: clean.
- `make api-surface`: `API surface unchanged` (4331 items).
- `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-storage`: `223 checks: 223 pass, 31 skip`; `1 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`.
- `make check-gates`: exit 0 (`OK: scanned 626 file(s) (19 test-only), 0 finding(s)`).
- Task 3 `<verify>` doc greps (storage CHANGELOG `CR-01`, MIGRATION 11.1 `45-REVIEW CR-01`, `without_url()` count, WINDOWS `WR-03`): all pass.
- Not run: the workspace-wide `cargo test`; this plan touches only `paladin-storage` and a root integration test file, and the whole `paladin-storage` suite plus the root integration target's compile were run instead. Disk was the constraint (about 5 GB free; builds used `CARGO_INCREMENTAL=0`).

## Evidence

Test names for 57-10 and 57-11 to cite:

- 45 CR-01: `minio::tests::list_request_sends_the_limit_as_max_keys_never_as_a_delimiter` (local) and `integration::file_storage_integration_tests::tests::test_list_files_honours_limit_and_drops_no_keys` (RustFS, `#[ignore]`).
- 45 WR-02: `minio::tests::s3_err_summary_never_echoes_the_response_body` and `minio::tests::retry_exhaustion_text_never_echoes_the_response_body`.
- 45 WR-01: the rustdoc on `upload_part`, `complete_multipart_upload` and `abort_multipart_upload` (docs only; `cargo doc -D warnings` clean).
- 45 WR-03: `test_presigned_urls` in `tests/integration/file_storage_integration_tests.rs` (5 `.without_url()` sites).

**SKIP, recorded honestly:** the RustFS contract case `test_list_files_honours_limit_and_drops_no_keys` did **not run** in this sandbox. The `docker` client is installed but the daemon is absent (`failed to connect to the docker API at unix:///var/run/docker.sock: ... no such file or directory`), and every case in that suite needs a RustFS container. It compiles and is listed, so CI's `File Storage` (RustFS) job runs it on the pushed head; **57-11 must read that job's result before marking rows 90 and 93 `fixed`.** The local proof for CR-01 is therefore the request-seam unit test only; the end-to-end behaviour against a real store (page 1 of 2 with `has_more` and a token, page 2 of 1 with `has_more` false, and all 3 keys, including `a5` and `b5`, with no limit) is unproven until that CI job reports.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical functionality] `execute_with_retry` and three further sites rendered a raw `S3Error`**
- **Found during:** Task 2
- **Issue:** the plan's WR-02 site list was `map_err` closures, but the retry-exhaustion message in `execute_with_retry` (`last_error.map(|e| e.to_string())`) and three match arms (`ensure_bucket_exists`, `create_bucket`, `file_exists`, plus `health_check`) also formatted an `S3Error` with `Display`. Left alone, the seven post-retry `map_err` sites would still have carried the raw body, defeating P-06-1.
- **Fix:** routed each through `s3_err_summary(&e)` and added `retry_exhaustion_text_never_echoes_the_response_body`.
- **Files modified:** `crates/paladin-storage/src/minio.rs`
- **Commit:** `6d0de8905`

### Notes

**1. `ensure_bucket_exists` still lists with the delimiter `"/"`.** It uses `Bucket::list("", Some("/"))` as an accessibility probe. That is a real delimiter used deliberately, not the CR-01 mistake, and it pages the whole bucket root; switching it to a one-key `list_page` would be a performance improvement outside this plan's findings, so I left it. It is a candidate for a later cleanup.

**2. `status_code() >= 300` branch kept.** The plan allowed dropping it only if provably unreachable. `fail-on-err` is a rust-s3 default feature this crate leaves on, so the branch is unreachable in practice, but I kept it as defence and documented why in a comment; the `<Error>` body test beside it is live.

**3. Gate scope.** See Gates: workspace-wide `cargo test` was not re-run for this storage-only plan.

### Tracer gate

tracer gate: verify re-run passed; orchestrator-approved (unattended session), not owner-approved. The Task 1 `<verify>` was re-run after the tracer commit and passed before the expansion tasks started.

## Threat Flags

None. T-57-21 (S3 body in errors) and T-57-22 (presigned URL in CI logs) are mitigated and tested as above; T-57-23 (dropped keys / hidden truncation) is mitigated by the seam test and, pending CI, the RustFS case; T-57-24 (open multipart uploads) is mitigated by the docs.

## Known Stubs

None.

## Issues Encountered

None. Disk stayed above 5 GB; no ENOSPC retry was needed.

## Next Phase Readiness

57-10 can record 45-REVIEW CR-01, WR-01, WR-02 and WR-03 as closed pointing at this plan, and 57-11 can mark WINDOWS rows 90 to 93 `fixed` once CI's `File Storage` job has run `test_list_files_honours_limit_and_drops_no_keys` on the pushed head.

## Self-Check: PASSED

- FOUND: `crates/paladin-storage/src/minio.rs` contains `fn s3_err_summary`, `fn list_page_request` and `list_page(`
- FOUND: `tests/integration/file_storage_integration_tests.rs` contains `test_list_files_honours_limit_and_drops_no_keys`
- FOUND: MIGRATION 11.1 row `M-B-40`
- FOUND commits: `b5ba82365`, `6d0de8905`, `ac7841931`
- FOUND: WINDOWS rows 90, 91, 92 and 93, status `open`
