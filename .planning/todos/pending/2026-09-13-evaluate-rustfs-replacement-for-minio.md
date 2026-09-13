---
created: 2026-09-13T00:00:00Z
title: Evaluate replacing MinIO with RustFS in the dev/test stack
area: infrastructure
severity: major
files:
  - docker/docker-compose.test.yml
  - docker/docker-compose.yml
  - .github/workflows/ci.yml
  - k8s/minio.yaml
  - crates/paladin-storage
---

## Problem

Docker Hub deleted the community MinIO server and client repositories on 2026-09-12. This
red-lined four CI jobs at the container-initialization step (Coverage, Integration Tests, Docker
Integration Tests, Kubernetes Smoke Test) with `pull access denied ... repository does not exist`,
with no first-party code at fault. Quick task 260913-15w restored green by pinning every live
configuration and the docs that quote it to the last known-good community release still served by
quay.io (`quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772` and
`quay.io/minio/mc:RELEASE.2025-08-13T08-35-41Z`).

That pin is terminal, not a maintenance path. No newer community MinIO tag will ever be
published, so the dev/test stack and the Kubernetes smoke test now depend on a frozen,
unmaintained third-party image that will drift further from upstream and will never receive a
security fix. quay.io could itself retire the repository at any time, which would reproduce the
exact same outage this quick task just fixed, with no remaining fallback registry to pin against.

## Solution

Evaluate RustFS — an S3-compatible, Rust-native object store — as the dev/test object-storage
backend, to remove the CI dependency on an unmaintained community image entirely.

Sketch of the shape:

- Add a new file-storage adapter in `crates/paladin-storage` implementing the existing
  `FileStoragePort`, alongside the existing MinIO/S3 adapter, behind its own Cargo feature flag so
  neither backend is forced on consumers.
- Add adapter-parity integration tests exercised against a RustFS service container, proving the
  same `FileStoragePort` contract suite passes unchanged against both backends.
- Once parity holds, swap the compose (`docker/docker-compose.test.yml`, `docker/docker-compose.yml`)
  and CI (`.github/workflows/ci.yml`) service definitions over to RustFS for dev/test.

This is an infrastructure-adapter change only and must not reach into core or ports — dependencies
flow inward only, per the hexagonal architecture rule in `CLAUDE.md`. The new adapter is subject to
the same TDD/coverage obligation as any other adapter: 82% workspace line coverage floor
(ADR-0006).

Open questions to answer during evaluation:

- RustFS's S3 API surface coverage versus what `paladin-storage` actually calls (bucket
  create/list, object put/get/delete, presigned URLs, multipart uploads if used).
- Multi-arch container availability and release cadence — does RustFS publish a maintained,
  multi-arch image with a healthy release cadence, avoiding the exact failure mode this todo
  exists to prevent?
- Project maturity and licence acceptability under the `cargo-deny` policy.
- Whether the production `k8s/minio.yaml` manifest should follow the dev/test stack onto RustFS,
  or stay on a separately sourced, production-grade S3-compatible service.

This item deliberately carries no `resolves_phase` tag — it is expected to outlive the current
milestone and must not be silently closed. Owner: repo maintainer.
