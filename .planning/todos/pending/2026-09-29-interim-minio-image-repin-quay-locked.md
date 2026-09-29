---
created: 2026-09-29T20:10:00Z
title: Interim re-pin of the MinIO server/client images — quay.io locked anonymous pulls
area: infrastructure
severity: blocker
kind: quick-task
blocks:
  - Phase 40 UAT test 4 (CI-only evidence) — .planning/phases/40-tenant-identity-run-read-scoping/40-UAT.md
  - every later phase's Coverage / Integration / Docker Integration / Kubernetes Smoke evidence
superseded_by: Phase 45 (RustFS swap) — .planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md
files:
  - .github/workflows/ci.yml
  - docker/docker-compose.test.yml
  - docker/docker-compose.yml
  - k8s/minio.yaml
  - .devcontainer/docker-compose.yml
  - .devcontainer/CI-CD.md
  - docs/src/appendix/integration-tests.md
  - docs/src/appendix/minio-file-repository-setup.md
  - docs/src/contributing/testing-guide.md
  - docs/src/deployment/cicd.md
  - docs/src/deployment/docker.md
owner: repo maintainer
recheck_by: 2026-10-06
---

## Problem

Since roughly 2026-09-24 `quay.io/minio/minio` and `quay.io/minio/mc` refuse anonymous pulls:

```
docker pull quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772
Error response from daemon: unauthorized: access to the requested resource is not authorized
```

This is the outage the 2026-09-13 RustFS todo predicted ("quay.io could itself retire the
repository at any time ... with no remaining fallback registry to pin against"). It is vendor-side
and repo-wide: the pin is byte-identical on `main`, whose last CI run (2026-09-23, run 374) was
green, and many unrelated open-source projects report the same error on the same images in the
same window.

Measured on this branch (`claude/laughing-dirac-e0h2ax`), CI runs 442 and 444, both on the Phase
40 final code tree:

| Job | Step that dies | Consequence |
|---|---|---|
| Coverage | Initialize containers (`minio` service) | no `lcov.info`; 82 % floor not measured |
| Integration Tests | Initialize containers (`minio` service) | web-server integration job never runs |
| Docker Integration Tests | Start infrastructure services (compose pulls `minio-test` + `minio-test-init`) | skipped test body |
| Kubernetes Smoke Test | Wait for dependencies (`kubectl apply -f k8s/minio.yaml`, pod never Ready) | skipped |
| End-to-End Tests, Publish Dry Run | `needs:` the above | skipped |

Everything else in those runs is green (postgres-integration contract suites, Redis and Ollama
integration jobs, cargo-audit, cargo-deny, API-surface, actionlint, License & Dependency Policy).
No first-party code is at fault. The `mc` **binary** install in CI (checksum-verified GitHub
release asset from the archived `minio/mc` repo, quick task 260913-h7l) is unaffected; only the
`quay.io/minio/mc` **image** used by the compose `*-init` services is.

## What needs to be done (interim, quick-task scope)

Goal: restore green CI without pre-empting Phase 45. Keep the change to image references and the
docs that quote them; no adapter code, no port changes.

1. **Pick a still-anonymously-pullable MinIO source and verify it before editing anything.**
   The sandbox that diagnosed this cannot reach any registry (proxy 403), so the candidates below
   are unverified. Verify with a plain `docker pull` from a runner-like environment first:
   - `cgr.dev/chainguard/minio` (Chainguard's rebuild of the same binaries; the free tier usually
     serves only a rolling tag, so pin by **digest**, not tag) — the replacement most other projects
     adopted.
   - Mirror the last known-good server + client images into this org's own registry
     (`ghcr.io/df3ndr/minio`, `ghcr.io/df3ndr/mc`) if any registry still serves them to an
     authenticated account, then pin by digest. This is the only option that makes the pin
     ours to keep and is the recommended shape if the bits can be obtained.
   - An authenticated quay.io pull (`docker login quay.io` with a registry secret in the four
     jobs) only if quay.io actually serves the image to a logged-in account — not confirmed.
   Whatever is chosen: exact tag **and** `@sha256:` digest, never `:latest` (the 2026-09-13 todo
   exists because a floating tag disappeared).

2. **Replace the server image in every live configuration** (7 occurrences):
   - `.github/workflows/ci.yml` lines 706 and 1418 (the `minio:` service container in the
     Coverage and Integration Tests jobs; update the comment block above each that still says
     "quay.io still serves this last known-good community release").
   - `docker/docker-compose.test.yml` line 27 (`minio-test`), `docker/docker-compose.yml`
     line 23 (`minio`), `.devcontainer/docker-compose.yml` line 92.
   - `k8s/minio.yaml` line 24 (drives the Kubernetes Smoke Test via `kubectl apply` at
     ci.yml line 1815).
   Keep `server /data --console-address ":9001"`, the `/minio/health/live|ready` probes and the
   `MINIO_ROOT_USER/PASSWORD` env unless the replacement image documents a different entrypoint.

3. **Replace or eliminate the `mc` client image** (2 occurrences): `docker/docker-compose.test.yml`
   line 49 and `docker/docker-compose.yml` line 47. Either point the `*-init` services at the
   same source chosen in step 1, or drop the image and run bucket bootstrap with the
   checksum-verified `mc` binary / a `curl`-based S3 `PUT bucket` in a plain `alpine` container —
   the latter also pre-does Phase 45's "bucket bootstrap replaces `mc`" criterion.

4. **Update the docs that quote the pin** so the docs-currency gate stays green (the Phase 34
   audit tracked every quoted occurrence): `.devcontainer/CI-CD.md` (lines 86, 232, 327, 508),
   `docs/src/appendix/integration-tests.md` 203, `docs/src/appendix/minio-file-repository-setup.md`
   (57, 506, 869, 873), `docs/src/contributing/testing-guide.md` 302 (a `testcontainers`
   `GenericImage::new(...)` example), `docs/src/deployment/cicd.md` 200,
   `docs/src/deployment/docker.md` 419. Run `make docs` (or the mdBook build the Phase 34 gate
   uses) afterwards.

5. **Verify**: push, confirm ci.yml run is green on Coverage, Integration Tests, Docker Integration
   Tests, Kubernetes Smoke Test, End-to-End Tests and Publish Dry Run; note the run id here and in
   the CHANGELOG `[Unreleased]` → Fixed. Then re-run `/gsd-verify-work 40` so UAT test 4 flips from
   `blocked` to `pass` and Phase 40 can transition.

6. **Do not** touch `crates/paladin-storage` or the `FileStoragePort` contract suite; the
   `rust-s3`-based adapter is unchanged by an image swap. Phase 45 remains the real fix and this
   file is closed by that phase's SUMMARY, not silently.

## Roadmap note

Phase 45 is sequenced last in v0.11.0. Until this interim pin lands, Phases 41–44 will also lack
coverage and integration evidence on every push. The alternative to this quick task is pulling
Phase 45 (or just its image-swap slice) ahead of Phase 41; that is a roadmap decision for the
maintainer, recorded in the Phase 40 UAT session of 2026-09-29.
