---
phase: 45-rustfs-swap-platform-observability-deviations
plan: 05
subsystem: infra
tags: [rustfs, docker-compose, devcontainer, makefile, coverage, dev-stack]

requires:
  - phase: 45-rustfs-swap-platform-observability-deviations
    provides: "45-01: adapter bootstraps its own bucket; 45-04: pinned image, digest, health path and E2E env contract"
provides:
  - "docker/docker-compose.yml runs rustfs (rustfs/rustfs:1.0.0, digest comment, /health/ready, volume rustfs_data) with paladin-app wired to rustfs:9000; bucket-init service deleted"
  - "docker/docker-compose.dev.yml overrides rustfs credentials only and declares no image (inherits)"
  - "Store-side RUSTFS_ACCESS_KEY / RUSTFS_SECRET_KEY in .env.example and compose interpolation (defaults paladin-dev / paladin-dev-secret); application-side APP_MINIO_* / MINIO_* / TEST_MINIO_* names unchanged"
  - ".devcontainer compose service rustfs with new volume rustfs-data, validate.sh probe, port labels and docs"
  - "Makefile health / minio-console (name kept) / storage-reset / devcontainer targets, scripts/coverage.sh probe_object_store, testing-guide Code Coverage section and docker.md quote the live values"
affects: [45-06, 45-07]

tech-stack:
  added: []
  patterns:
    - "Dev compose inherits the pinned image in the .dev.yml override rather than repeating it (one pin per file family)"
    - "coverage.sh resolution order: validated preset -> rustfs:9000 (devcontainer creds) -> localhost:9010 (test compose creds)"

key-files:
  created: []
  modified:
    - docker/docker-compose.yml
    - docker/docker-compose.dev.yml
    - .env.example
    - docs/src/deployment/docker.md
    - .devcontainer/docker-compose.yml
    - .devcontainer/validate.sh
    - .devcontainer/devcontainer.json
    - .devcontainer/post-create.sh
    - .devcontainer/README.md
    - .devcontainer/QUICKSTART.md
    - .devcontainer/FILES.md
    - .devcontainer/SETUP_COMPLETE.md
    - Makefile
    - scripts/coverage.sh
    - docs/src/contributing/testing-guide.md
    - tests/integration/cli_real_services_test.rs

key-decisions:
  - "docker-compose.dev.yml declares no image and inherits rustfs/rustfs:1.0.0 from the base file (discretion choice)"
  - "Console stays enabled on 9001 in the dev and devcontainer stacks (RUSTFS_CONSOLE_ENABLE true); it is off in CI, the test compose and k8s (45-04)"
  - "The coverage.sh log line keeps its `minio=` label: it is quoted in archived CI evidence and is a log format, not a config name; the exported TEST_MINIO_* / MINIO_* names and the exec cargo llvm-cov line are unchanged (D-04, Pitfall 7)"

patterns-established:
  - "Docs that must describe the pre-Phase-45 state (volume left over, renamed variables) avoid the retired literals so the plan-scoped STORE-01 grep gate stays empty"

requirements-completed: []

duration: ~40min
completed: 2026-09-30
status: complete
---

# Phase 45 Plan 05: RustFS in the developer stack Summary

**The dev compose, devcontainer, Makefile, `.env.example`, `scripts/coverage.sh` and the docs that quote them now run and describe `rustfs/rustfs:1.0.0`; the `mc` init container is deleted, so the app's own adapter creates `paladin-files`.**

## Task Commits

1. **Task 1 (tracer): dev compose runs RustFS with adapter-owned bucket bootstrap** - `48e4ef23` (feat)
2. **Task 2: devcontainer object store is RustFS** - `23d03465` (feat)
3. **Task 3: Makefile, coverage probe and testing guide point at RustFS** - `d53be442` (chore)

## What changed

- **Dev compose:** `minio` becomes `rustfs` (container `paladin-rustfs`, ports 9000 and 9001, healthcheck `curl -f http://localhost:9000/health/ready`, no `command:`), pin and manifest-list digest comment identical to `ci.yml`. `paladin-app` depends only on `redis` and `rustfs`, with `APP_MINIO_ENDPOINT=rustfs:9000` and the credentials interpolated from `RUSTFS_ACCESS_KEY` / `RUSTFS_SECRET_KEY`.
- **Dropped with the init container:** the anonymous-public policy on `paladin-files` (T-45-23, Pitfall 13) and the three buckets nothing in the tree reads: `paladin-analysis`, `paladin-reports`, `paladin-backups`.
- **Volume renames (Pitfall 12):** `minio_data` becomes `rustfs_data` (docker) and `minio-data` becomes `rustfs-data` (devcontainer), so RustFS never mounts a MinIO-formatted volume. Old volumes are not migrated; `docker.md` says to remove them with `docker volume rm`.
- **Local `.env` files must rename the two store-side variables** (old root-user variable to `RUSTFS_ACCESS_KEY`, old root-password variable to `RUSTFS_SECRET_KEY`); the MinIO-only browser-redirect variable is gone. `.env.example` and `docker.md` both say so.
- **Makefile:** `health` probes `/health/ready`; `minio-console` (target name kept) prints `http://localhost:9001/rustfs/console/index.html` and the dev credentials; `storage-reset` execs `rm -rf /data/*` in `rustfs` then restarts it (app recreates the bucket, no init); devcontainer services line is `up -d redis rustfs mysql`; help text names RustFS.
- **`scripts/coverage.sh`:** `probe_object_store` probes `/health/ready` (TCP fallback unchanged); order preset, `rustfs:9000` (`paladin-dev` / `paladin-dev-secret`), `localhost:9010` (`testuser` / `testpass123`). `s3-storage` is not added (Pitfall 7, as recorded by 45-04).
- **Docs:** `docker.md` (quoted service block, console URL, adapter-created bucket, volume and `.env` migration notes), the five `.devcontainer` docs, and the testing guide's Code Coverage section (procedure `make services-up` then `make coverage` unchanged; the `exec` excerpt still matches the script byte for byte).

## Verification

- **Tracer (E2E contract env):** `docker compose --env-file <e2euser env> -f docker/docker-compose.yml config --format json` (Compose v5.1.1 rendered without a daemon; no PyYAML fallback needed) shows service `rustfs` on `rustfs/rustfs:1.0.0` with `RUSTFS_ACCESS_KEY=e2euser`, a `/health/ready` healthcheck, `command` null, volume `rustfs_data`, `paladin-app` with `APP_MINIO_ENDPOINT=rustfs:9000` and `APP_MINIO_ACCESS_KEY=e2euser`, services exactly `paladin-app`, `redis`, `rustfs`. Layered with `docker-compose.dev.yml` the store credential is `devuser`. Tracer verified end to end before expansion (auto mode).
- **Devcontainer render:** image pinned, `command` null, `rustfs-data` declared, no `minio` service.
- `bash -n` on `validate.sh`, `post-create.sh`, `coverage.sh`; `make lint-shell` clean; `make check-doc-config` (151 YAML blocks, 0 failed); `cargo check --tests --features cli` finished; `make -n minio-console health storage-reset` expands.
- Every acceptance grep in the three tasks passes. The plan-scoped STORE-01 grep (`quay.io/minio|minio/minio|minio/mc|MC_RELEASE|mc alias|MINIO_ROOT_|/minio/health|minio-init|anonymous set public`, comment lines filtered) over all 16 files prints nothing; `git grep rustfsadmin` over the dev stack prints nothing (T-45-25: only throwaway literals, never the image default credential).

## CI-attributed / maintainer-owned (not run here)

No Docker daemon exists in this sandbox, so no container was started. Recorded as backstop, not as a Phase 45 criterion:
- `make services-up` then `make coverage` on a Docker-capable machine reproducing CI's coverage figure against the RustFS dev stack (user-owned todo 2026-08-13, docs half folded and done here; the todo stays open).
- RustFS answering `/health/ready` healthy under the dev and devcontainer compose files and the adapter's first-start bucket creation against a persistent (named-volume) `/data` owned by uid 10001 (RESEARCH A2 does not apply to named volumes, which inherit the image's pre-owned `/data`; still unobserved here).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical / docs-currency] `.devcontainer/post-create.sh` hint text**
- **Found during:** Task 2
- **Issue:** the `pd-dev` help echo still said "Redis, MinIO, etc." and the file is not in the plan's `files_modified`; leaving it would put a stale service name in front of every devcontainer user.
- **Fix:** one-word change to "RustFS". **Files:** `.devcontainer/post-create.sh`. **Commit:** `23d03465`.

**2. [Rule 1 - Bug in own draft] Migration notes tripped the plan's STORE-01 grep gate**
- **Found during:** Task 1 acceptance
- **Issue:** the first draft of the `docker.md` migration note quoted the retired variable names and volume name literally, which the gate forbids outside comment lines.
- **Fix:** reworded to "the MinIO volume from before Phase 45" and "its two store-side root-credential variables"; the literal old names remain only in `.env.example` comment lines, where the gate filters them. **Commit:** `48e4ef23`.

### Additions

- `docker.md`'s quoted "Multi-Container Setup" compose example, its `file_storage` config snippet, its environment-variable block and two troubleshooting lines were also moved to RustFS values (the plan named the quoted dev compose block; these were further stale quotes in the same file).
- `docker.md`'s Docker Compose example uses the live pin string but not the digest comment; the digest lives in the compose files themselves.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-45-23 (init container and public policy deleted, grep gate empty), T-45-24 (new volume names, docs and `.env.example` say old volumes are not migrated), T-45-25 and T-45-26 (accepted: throwaway dev literals, console dev-only) are as planned.

## Requirements

STORE-01 is not marked complete here: its CI-runtime edges are backstop truths pending the recorded CI run id (45-07), and the phase-wide grep gate and E2E cross-check run in 45-06.

## Self-Check: PASSED

- Commits `48e4ef23`, `23d03465`, `d53be442` present in `git log`; each body ends with the two required trailers.
- `docker/docker-compose.yml` and `.devcontainer/docker-compose.yml` contain `rustfs/rustfs:1.0.0`; `scripts/coverage.sh` contains `probe_object_store`; `docker/docker-compose.dev.yml` has no `image:` line.
