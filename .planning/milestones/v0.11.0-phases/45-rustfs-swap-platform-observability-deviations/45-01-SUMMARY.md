---
phase: 45-rustfs-swap-platform-observability-deviations
plan: 01
subsystem: storage
tags: [rustfs, s3, rust-s3, multipart, presigned-urls, etag, testcontainers, contract-suite, advisory-register]

requires:
  - phase: 40-tenant-identity-run-read-scoping
    provides: the CI evidence discipline (sandbox has no Docker; container evidence is CI-attributed)
provides:
  - "MinioAdapter path-style bucket bootstrap (adapter creates the bucket on an empty RustFS, no mc)"
  - "MinioAdapter multipart trio implemented behind a stateless, self-describing upload token"
  - "copy_file passes the bare source key (no double bucket prefix)"
  - "FileStoragePort contract suite of 11 cases (multipart lifecycle, multipart abort, opaque ETag, exercised presigned URLs) green against native RustFS 1.0.0"
  - "RustFS GenericImage local mode (rustfs/rustfs:1.0.0, /health/ready poll); testcontainers-modules removed"
  - "RUSTSEC-2025-0111 register entry re-pointed at tokio-tar -> testcontainers"
  - "D-01 resequencing recorded on the ROADMAP Phase 45 Depends on line"
affects: [45-04, 45-05, 45-06]

tech-stack:
  added: []
  patterns:
    - "Stateless multipart token: <key byte length>:<key><S3 upload id>, re-validated with validate_path on every decode"
    - "S3 error bodies never propagate: only the <Code> value, bounded to 64 chars, is embedded in an error"
    - "HTTP readiness poll (/health/ready) instead of a stdout wait for an image that logs to files"

key-files:
  created: []
  modified:
    - crates/paladin-storage/src/minio.rs
    - crates/paladin-storage/CHANGELOG.md
    - tests/integration/mod.rs
    - tests/integration/file_storage_integration_tests.rs
    - Cargo.toml
    - Cargo.lock
    - SECURITY-EXCEPTIONS.md
    - docs/src/contributing/testing-guide.md
    - .planning/ROADMAP.md

key-decisions:
  - "D-08 reuse-first holds: the existing rust-s3 adapter passes the whole contract suite on RustFS 1.0.0, so no second adapter, feature or crate was added"
  - "Readiness wait is an HTTP poll of /health/ready written as a suite-local helper (no testcontainers http_wait feature, no Cargo.toml change)"
  - "RUSTSEC-2025-0111 stays suppressed: testcontainers 0.24.0 still pulls tokio-tar 0.3.1, so the register row is re-pointed, not deleted"
  - "STORE-01 and STORE-02 are NOT marked complete by this plan: STORE-02's CI run is a backstop truth and STORE-01's CI/compose/k8s edges belong to 45-04, 45-05 and 45-06"

patterns-established:
  - "The 45-01 native RustFS loop: unzip the release binary into the session scratchpad, run it with RUSTFS_ACCESS_KEY=testuser RUSTFS_SECRET_KEY=testpass123 RUSTFS_CONSOLE_ENABLE=false ./rustfs --address 127.0.0.1:9010 <empty-dir>, poll /health/ready, run a throwaway listener on 127.0.0.1:6380 for the harness Redis check"

requirements-completed: []

duration: 55min
completed: 2026-09-29
status: complete
---

# Phase 45 Plan 01: RustFS-proven FileStoragePort contract suite Summary

**The existing rust-s3 adapter now bootstraps its own bucket path-style, implements multipart behind a stateless validated token, and passes an 11-case contract suite (multipart, ETag, exercised presigned URLs) on a native RustFS 1.0.0 with an empty data directory; local mode runs the pinned RustFS image and `testcontainers-modules` is gone.**

## Performance

- **Tasks:** 3 of 3 (Task 1 was the tracer)
- **Files modified:** 9 (no file created or deleted)

## Task Commits

1. **Task 1: Tracer, harness hostname fix, single logger, path-style bootstrap** - `d6ac370d` (test)
2. **Task 2: Multipart via stateless token, bare-key copy, ETag and presign cases** - `71014c33` (feat)
3. **Task 3: RustFS GenericImage local mode, drop testcontainers-modules, D-01 line** - `a2b22b1d` (chore)

## Evidence

**Native binary (T-45-06).** `rustfs-linux-x86_64-musl-v1.0.0.zip` from `github.com/rustfs/rustfs/releases/download/1.0.0/`, sha256 `c30a95b76546f25122c9ca387090ddb30c391ca5605621b0d7c881703c0f21c8`. No published checksum asset exists for it (`.sha256`, `.sha256sum` and `.sha512` all answered 404 and the GitHub API is not enabled for this session), so the hash is recorded but not verified against a published value. The binary reported `rustfs 1.0.0`, build time 2026-09-16. It ran only in the sandbox scratchpad and was never committed. Both it and the unused `rustfs-cli` were deleted afterwards (see Issues).

**Tracer (D-05, D-08).** Started on an empty data directory, `test_file_storage_health_check` and `test_file_upload_download_lifecycle` passed with `TEST_REDIS_HOST=localhost`; afterwards the `integration-tests` bucket answered 200, so the adapter created it (nothing else did). Tracer verified end to end before expansion.

**Full suite.** On a second fresh data directory: `test result: ok. 11 passed; 0 failed`, `--list --ignored` shows 11 `file_storage_integration_tests` cases. The cases:

`test_batch_operations`, `test_end_to_end_security_audit_workflow`, `test_error_handling`, `test_etag_is_an_opaque_stable_token`, `test_file_operations_full_cycle`, `test_file_storage_health_check`, `test_file_upload_download_lifecycle`, `test_multipart_abort_leaves_no_object`, `test_multipart_upload_lifecycle`, `test_presigned_urls`, `test_storage_statistics`.

**RustFS S3-surface gaps: none.** Every failure met on the way was a defect in this repo (harness or adapter), not in RustFS. The one suite failure while writing the new cases was my own `test_etag_is_an_opaque_stable_token` re-uploading without `overwrite: true` (fixed in the test, see Deviations). D-08's second-adapter path stays closed.

**Unit tests.** `cargo test -p paladin-storage --features s3 --lib minio::tests`: 14 passed, including `multipart_token_round_trips`, `split_token_rejects_malformed_tokens`, `decode_multipart_token_rejects_a_traversal_key`, `decode_multipart_token_rejects_an_empty_upload_id` (red first: they failed to compile before the helpers existed).

**Gates that passed.** `cargo fmt --check`; `cargo clippy -p paladin-storage --features s3 --all-targets -- -D warnings`; `cargo clippy --tests --features integration-tests,s3-storage -- -D warnings`; `make security` (advisories, bans, licenses, sources ok); `make check-gates` (register 11 rows against `deny.toml` and `.cargo/audit.toml`, all clauses satisfied); `PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` reports the surface unchanged (4044 items); `make check-doc-config`; `make check-changelogs`.

## Decisions Made

- **Token shape** is `<key byte length>:<key><S3 upload id>` (D-09 a). `split_token` slices with `str::get`, so a bad length or a cut UTF-8 character is `FileStorageError::InvalidPath`, never a panic. `decode_multipart_token` also rejects an empty S3 upload id and re-runs `validate_path` on the decoded key, so a forged token cannot smuggle `..` (T-45-01).
- **Complete-multipart** checks both the HTTP status and an `<Error>` body: `rust-s3`'s `complete_multipart_upload` does not check status itself. The error carries only the S3 `<Code>`, capped at 64 chars (T-45-03).
- **Abort** maps `404 NoSuchUpload` to `FileNotFound("multipart upload not found or already finished")`.
- **Presigned URLs** are exercised with a real `PUT` and `GET`; only `url.split('?').next()` is printed (T-45-02).
- **Advisory register (RUSTSEC-2025-0111):** `path`, `why_present` and `revisit_condition` re-pointed at `tokio-tar -> testcontainers`. The advisory remains suppressed because `testcontainers` 0.24.0 still depends on `tokio-tar` 0.3.1 (`Cargo.lock` still holds `tokio-tar`; `testcontainers-modules` is gone from `Cargo.toml` and `Cargo.lock`). `deny.toml` and `.cargo/audit.toml` untouched.
- **D-01:** the ROADMAP Phase 45 `**Depends on**` line now records the 2026-09-29 resequencing ahead of Phases 41-44; no other ROADMAP line touched by the task (the state tooling updates the progress table separately).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] ETag case re-uploaded without overwrite**
- **Found during:** Task 2, first full-suite run
- **Issue:** `test_etag_is_an_opaque_stable_token` uploaded twice to `etag/a.txt` with default options (`overwrite: false`); the second upload failed with `InvalidPath("File already exists")`.
- **Fix:** both uploads use `ctx.create_upload_options(...)` (overwrite true), which also makes the case re-runnable after a failed run.
- **Files modified:** `tests/integration/file_storage_integration_tests.rs`
- **Commit:** `71014c33` (fixed before the commit)

### TDD note

The four unit tests were written first and failed to compile (red) before the helpers existed. The three new integration cases and the rewritten presign case were written after the adapter methods (they needed a live RustFS to say anything), so their red step is the pre-plan tree, where the multipart trio returned `Unknown("... not fully implemented ...")`.

### Scope adherence

No second adapter, feature or crate; no existing suite case weakened or removed; the eight existing cases keep their assertions; no credential other than the throwaway `testuser`/`testpass123` literals; `rust-s3` stays at 0.35.1; no public item changed.

## Issues Encountered

**Disk exhaustion.** Free space fell from 1.7 GB to 0 during Task 3 (the second `integration-tests` test binary is about 170 MB and clippy writes an incremental cache). To recover I deleted only files this run created: the native RustFS binary and its `rustfs-cli` sibling in the session scratchpad, its data directories and logs, and one superseded test binary (`target/debug/deps/lib-e2bafdc7bc1edafd`, the pre-Task-3 build of `tests/lib.rs`). Nothing pre-existing in `target/` was touched. Consequences: (a) the native RustFS is no longer available here, so a later plan must re-download it (the procedure is above); (b) the workspace-wide `cargo test` and `make check-doc-examples` were not run, only the targeted commands listed under Evidence. Free space at hand-off is about 165 MB, so the next plan that compiles anything large will hit the same wall.

**Local mode is CI-attributed.** `new_local` (GenericImage plus the `/health/ready` poll) compiles and lints clean but was never executed: no Docker daemon exists here. Only external mode ran, against the native binary.

## Known Stubs

None. The three multipart stubs this plan targeted are implemented.

## Threat Flags

None. No new network endpoint, auth path or trust-boundary schema. The new multipart token is the trust boundary the plan's threat model already covers (T-45-01).

## Follow-ups for the orchestrator

- STORE-02's "green on the RustFS service container in CI" is a `backstop` truth (D-08 gate); it needs plan 45-04's CI wiring plus a recorded run id at UAT.
- The CI, compose, k8s and devcontainer wiring still references MinIO images; that is 45-04 and 45-05, not this plan.
- Coverage note for 45-05: the suite's local-mode helper and `wait_until_ready` are compiled only under the `integration-tests` feature.

## Self-Check: PASSED

- `d6ac370d`, `71014c33`, `a2b22b1d` present in `git log`.
- All 9 modified files present in the tree; `crates/paladin-storage/src/minio.rs` contains `fn split_token(`, `fn encode_multipart_token(`, `fn decode_multipart_token(`; `tests/integration/mod.rs` contains `to_socket_addrs`.
- Acceptance greps all match the plan's expected values (0 `SystemLogAdapter`, 0 `not fully implemented`, 0 `copy_source`, 0 `testcontainers-modules` in `Cargo.toml` and `Cargo.lock`, 1 `tokio-tar` in `Cargo.lock`, `>= 11` listed suite cases).
