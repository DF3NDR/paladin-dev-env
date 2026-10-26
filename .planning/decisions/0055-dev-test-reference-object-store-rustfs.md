# ADR-0055: Dev/test and reference object store is RustFS

## Status

Accepted

**Date:** 2026-09-30

## Context

On 2026-09-12 Docker Hub deleted the community MinIO server and client repositories. Every CI job
that started a MinIO container (Coverage, Integration Tests, Docker Integration Tests, Kubernetes
Smoke Test) failed at container initialisation with no first-party code at fault. Quick task
260913-15w restored green by pinning every live configuration to the last community release still
served by quay.io — a terminal pin, since no newer community tag will ever be published and the
image receives no security fixes.

The terminal pin failed the way the FUT-10 todo (`2026-09-13-evaluate-rustfs-replacement-for-minio`)
predicted it could: from about 2026-09-24 `quay.io` refused anonymous pulls of both images. Coverage,
Integration Tests, Docker Integration Tests and Kubernetes Smoke Test went red on every push, and
Phase 40 UAT test 4 (CI-only evidence) was `blocked` on it. The interim todo
(`2026-09-29-interim-minio-image-repin-quay-locked`) proposed re-pinning to a Chainguard rebuild, a
ghcr mirror or an authenticated quay pull, each of which keeps a third-party MinIO image alive that
this project does not control.

The FUT-10 todo carried four open questions: how much of the S3 surface `paladin-storage` calls
RustFS covers, whether the image is multi-arch and maintained, whether the licence passes the
`cargo-deny` policy, and whether the production `k8s/minio.yaml` manifest follows the dev/test stack.
Phase 45 (STORE-01, STORE-02, STORE-03) answers them; the planning decisions are D-01 through D-13 in
`45-CONTEXT.md`.

## Decision

1. **One pinned image everywhere (D-03).** Every live configuration — CI service containers, the
   test and dev compose files, the devcontainer, the Kubernetes manifest and the contract suite's
   local mode — runs `rustfs/rustfs:1.0.0`, the exact tag with the manifest-list digest
   (`sha256:8cc9801755448b71a786705ce76692c77e14936cccd87cf2fc31842e58f4d1ff`, re-verified against
   the Docker Hub tags API on 2026-09-30) in a comment beside each `image:` line. Never `latest`. A
   re-pin is a deliberate act, not drift.

2. **Store-side credentials are `RUSTFS_ACCESS_KEY` / `RUSTFS_SECRET_KEY` (D-04).** The
   application-side names (`APP_MINIO_*`, `MINIO_*`, `TEST_MINIO_*`) are unchanged, so no operator
   application configuration moves.

3. **The bucket bootstrap is the adapter's `ensure_bucket_exists`, path-style (D-05).** The `mc`
   client, its init containers, its CI steps and the checksum-verified client download are deleted,
   not replaced. The adapter's bucket creation was made path-style, which works on RustFS and on
   MinIO without domain configuration.

4. **Health is `/health` (liveness) and `/health/ready` (readiness) (D-06).** The MinIO health
   paths are gone from every probe.

5. **The existing adapter is reused; the contract suite proves it (D-08, D-09).** The `rust-s3`
   adapter passes the completed 11-case `FileStoragePort` contract suite — including multipart,
   opaque ETags and exercised presigned URLs — against a native RustFS 1.0.0 on an empty data
   directory (45-01: `11 passed; 0 failed`). The suite runs in CI in the Integration Tests and Docker
   Integration Tests jobs behind a compiled-in count and a passed count that must both reach 11, so
   it cannot pass vacuously (45-04). No RustFS S3-surface gap was found; the second-adapter fallback
   stays closed.

6. **Identity nouns are unchanged (D-10).** `MinioAdapter`, `MinioConfig`, the `minio:` config
   section and the `s3` / `s3-storage` features keep their names; a rename to an S3-neutral noun is a
   public break deferred by the phase. Only prose is corrected: the adapter is a generic SigV4 S3
   client, and MinIO, AWS S3, DigitalOcean Spaces or any compatible endpoint remain supported
   production targets.

7. **`k8s/rustfs.yaml` is the one manifest (D-11).** It serves both the Kubernetes Smoke Test and
   the reference deployment, renamed from `k8s/minio.yaml`. It is single-node with `emptyDir`
   storage, console disabled, and runs non-root (uid 10001). It is not highly available; production
   points the same adapter at AWS S3, a managed S3 endpoint, MinIO or another SigV4 store.

8. **The console is enabled only in the dev compose and devcontainer (discretion).** CI, the test
   compose and Kubernetes run with the console off, which shrinks their exposed surface.

The FUT-10 questions resolve as: S3 surface — the contract suite is green with the reused adapter;
image — `rustfs/rustfs:1.0.0` is multi-arch and maintained (a newer GA tag has not been published;
`1.0.1-preview.11` is a preview and was not adopted); licence — Apache-2.0, already allow-listed;
production manifest — follows the dev/test stack, as item 7.

## Considered Options

- **Interim MinIO re-pin via Chainguard, a ghcr mirror or an authenticated quay pull** (superseded,
  D-02) — restores green but keeps an unmaintained third-party MinIO image on the critical path and
  leaves the `mc` client dependency in place; Phase 45 replaces it outright, so the interim todo is
  closed as superseded, not executed.
- **A pinned `rustfs/rc` init container in place of `mc`** (rejected, D-05) — one more third-party
  image for a job the adapter already does; the fallback was not needed because the adapter creates
  its own bucket.
- **A new RustFS-native adapter behind its own Cargo feature** (rejected, D-08) — the existing
  adapter passed the contract suite, so a second adapter would be unjustified code with its own
  coverage obligation.
- **A smoke-test-only manifest split from a reference manifest** (rejected, D-11) — two manifests
  for one store would drift; one manifest is exercised on every CI run.
- **Keeping `minio`-named compose services and files running RustFS** (rejected) — misleading names
  on a store that is no longer MinIO; services are `rustfs` / `rustfs-test` and the manifest is
  `k8s/rustfs.yaml`.

## Code Locations

- `crates/paladin-storage/src/minio.rs` — `MinioAdapter`: path-style `create_bucket`, the multipart
  trio behind a stateless validated upload token, bare-key `copy_file`
- `tests/integration/file_storage_integration_tests.rs` — the 11-case contract suite, `RUSTFS_IMAGE`,
  `RUSTFS_TAG`, `wait_until_ready`
- `.github/workflows/ci.yml` — the two `rustfs` service containers (Integration Tests, Coverage) and
  the contract-suite steps with the compiled-in and passed counts
- `docker/docker-compose.yml` — the dev `rustfs` service (console on, adapter-owned bucket)
- `docker/docker-compose.test.yml` — `rustfs-test`, the external-mode `integration-tests` service
- `.devcontainer/docker-compose.yml` — the devcontainer `rustfs` service and `rustfs-data` volume
- `k8s/rustfs.yaml` — the single Kubernetes manifest (Service `paladin-rustfs`)
- `docs/src/appendix/minio-file-repository-setup.md` — the storage page: RustFS quick start and the
  production-manifest decision

## Code Conformance

conforms

Landed in Phase 45 (plans 45-01, 45-04, 45-05; recorded by 45-06). Evidence: the phase-wide grep
finds no MinIO image, `mc` step, MinIO health path, anonymous-public policy or
`testcontainers-modules` reference in any live configuration; exactly one RustFS tag,
`rustfs/rustfs:1.0.0`, appears across `.github docker k8s .devcontainer tests docs`; the E2E job's
generated environment file renders a `rustfs` service and a `paladin-app` that share credentials and
a `/health/ready` healthcheck; and `--list --ignored` shows 11 contract-suite cases. CI runtime
behaviour (service containers healthy, the suite reporting at least 11 passed) is CI-attributed and
recorded by the phase's UAT run id.

## Downstream Consumers

- **The next RustFS re-pin** — triggered by a RustFS security release or a contract-suite
  regression; it changes the tag and digest in every file in Code Locations together, and the
  one-tag grep in the 45-06 gate must still print exactly one tag.
- **`/gsd-verify-work 40`** — once CI is green on the phase's tree, this flips Phase 40 UAT test 4
  from `blocked` to `pass` (D-02 follow-up).
- **Phase 46 (docs currency)** — the release documentation carries the operator-facing changes:
  the renamed store-side variables, the renamed manifest, the unmigrated old MinIO volumes.
- **The deferred S3-neutral rename** — `MinioAdapter`, `MinioConfig`, the `minio:` section and
  `APP_MINIO_*` keep their names until a rename is scheduled as a deliberate public break.
