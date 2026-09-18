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
owner: repo maintainer
deferred_past: v0.10.0
recheck_by: 2026-10-16
dispositioned_by: Phase 36.1 (2026-09-18)
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

MinIO's community binary downloads are gone as well as its Docker Hub images — the former
download host now returns 410 Gone for the client binary — so quick task 260913-h7l additionally
pinned the CI client install to a checksum-verified release asset on the archived
`github.com/minio/mc` repository.

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

## Disposition (Phase 36.1, 2026-09-18)

**What was verified now.** The documentation-currency half of this item — the live configuration
and the docs that quote it staying pinned to the last known-good `quay.io` MinIO release, and the
checksum-verified `mc` client install — was settled by an earlier sweep (quick tasks 260913-15w
and 260913-h7l, both cited in the Problem section above) and remains in force; this phase found no
further documentation drift to fix here.

**What remains blocked, and why.** The evaluation itself — building a RustFS `FileStoragePort`
adapter behind its own Cargo feature flag, proving parity against the existing MinIO/S3 adapter
with adapter-parity integration tests, and only then swapping the dev/test compose and CI service
definitions — is an adapter build with its own TDD and coverage obligation (82% workspace line
coverage floor, ADR-0006). That is past this milestone by the item's own text, and adding a v2
requirement line does not shrink it to something this closing phase could absorb.

**A v2 requirement line has been added** — see `.planning/REQUIREMENTS.md` `## v2 Requirements`
→ `### Platform & Tooling`, naming this evaluation as a v0.11.0 candidate and pointing back at
this file, so the milestone backlog carries it as well as the todo directory.

**Re-check trigger.** 2026-10-16, or the v0.11.0 planning kickoff, whichever comes first.
