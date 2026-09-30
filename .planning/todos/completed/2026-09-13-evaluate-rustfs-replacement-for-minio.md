completed: 2026-09-30
---
created: 2026-09-13T00:00:00Z
title: Evaluate replacing MinIO with RustFS in the dev/test stack
area: infrastructure
resolves_phase: 45
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

## Resolution (Phase 45)

Resolved by Phase 45 (RustFS swap) on 2026-09-30; recorded as ADR-0055
(`.planning/decisions/0055-dev-test-reference-object-store-rustfs.md`). Nothing here was closed
silently: the four open questions are answered below and the commits that removed the last MinIO
reference are named.

**The four open questions.**

- *S3 API surface:* the existing `rust-s3` adapter passes the completed 11-case `FileStoragePort`
  contract suite (multipart, opaque ETag and exercised presigned URLs included) against RustFS
  1.0.0 on an empty data directory (45-01), and the suite now runs in CI behind a compiled-in and a
  passed count. No S3-surface gap was found, so no second adapter or feature was built (D-08); the
  Solution sketch's separate-adapter route was not needed.

- *Image and cadence:* `rustfs/rustfs:1.0.0`, multi-arch, pinned by exact tag with the manifest-list
  digest in a comment beside every `image:` line (D-03). A newer GA tag had not been published at
  the time; `1.0.1-preview.11` is a preview and was not adopted.

- *Licence:* Apache-2.0, already on the `cargo-deny` allow-list.
- *Production manifest:* follows the dev/test stack. `k8s/rustfs.yaml` is the one manifest for the
  smoke test and the reference deployment (single-node, `emptyDir`, console off, non-root);
  production points the same adapter at AWS S3, a managed endpoint or MinIO (D-11).

**Commits that removed the last MinIO reference from live configuration.**

- `7dfb48a9` (45-04): CI Integration Tests and Coverage service containers run the pinned RustFS
  image; the `mc` client install and bucket-setup steps and the checksum-verified client download
  are deleted

- `870a491c` (45-04): `docker/docker-compose.test.yml` runs `rustfs-test`; the `mc` init container
  is deleted; the Docker Integration and E2E jobs are rewired

- `ed3fb5b8` (45-04): `k8s/minio.yaml` renamed to `k8s/rustfs.yaml`
- `48e4ef23` (45-05): `docker/docker-compose.yml` runs `rustfs`; the `mc` init container, the
  anonymous-public policy and the three unused buckets are deleted

- `23d03465` (45-05): the devcontainer object store is RustFS (the last MinIO image reference in
  live configuration)

- `a2b22b1d` (45-01): `testcontainers-modules` removed; the contract suite's local mode runs the
  pinned RustFS image

The phase-wide grep for MinIO images, `mc` steps, MinIO health paths, the anonymous-public policy
and `testcontainers-modules` over `.github docker k8s .devcontainer scripts Makefile .env.example
tests Cargo.toml Cargo.lock` prints nothing, and exactly one RustFS tag appears across
`.github docker k8s .devcontainer tests docs` (plan 45-06). See
`.planning/phases/45-rustfs-swap-platform-observability-deviations/45-06-SUMMARY.md`.
