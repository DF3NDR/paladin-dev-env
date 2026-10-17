---
phase: 45-rustfs-swap-platform-observability-deviations
plan: 04
subsystem: infra
tags: [rustfs, ci, github-actions, docker-compose, kubernetes, contract-suite, actionlint]

requires:
  - phase: 45-rustfs-swap-platform-observability-deviations
    provides: "45-01: 11-case FileStoragePort contract suite green on native RustFS 1.0.0, adapter bootstraps its own bucket, RustFS local mode"
provides:
  - "Integration Tests and Coverage jobs run rustfs/rustfs:1.0.0 service containers (digest comment, /health/ready, console off, no command key)"
  - "Contract-suite steps that cannot pass vacuously: compiled-in count >= 11, then passed count >= 11 (Integration Tests job and Docker Integration Tests job)"
  - "MinIO client install and bucket-setup steps and the bucket-init compose service deleted (D-05)"
  - "docker/docker-compose.test.yml rustfs-test service; integration-tests service in external mode"
  - "E2E job wired to the dev compose contract (RUSTFS_* env, /health/ready) and running the suite with s3-storage"
  - "k8s/minio.yaml renamed to k8s/rustfs.yaml with every reader, the smoke job and the k8s docs (D-11)"
  - "actionlint config reduced to its self-hosted-runner block"
affects: [45-05, 45-06, 45-07]

tech-stack:
  added: []
  patterns:
    - "Two-step vacuous-pass guard: `--list --ignored` count, then `test result: ok. N passed` parse under pipefail + tee"
    - "Store-side names RUSTFS_*, application-side names MINIO_* / TEST_MINIO_* / APP_MINIO_* unchanged (D-04)"

key-files:
  created:
    - k8s/rustfs.yaml
  modified:
    - .github/workflows/ci.yml
    - .github/actionlint.yaml
    - docker/docker-compose.test.yml
    - scripts/run_integration_tests.sh
    - k8s/deployment.yaml
    - k8s/configmap.yaml
    - k8s/secret.yaml.example
    - k8s/README.md
    - docs/src/deployment/cicd.md
    - docs/src/deployment/kubernetes.md
    - docs/src/contributing/branching-model.md
    - docs/src/operations/troubleshooting.md
    - docs/src/appendix/integration-tests.md
    - .devcontainer/CI-CD.md
  deleted:
    - k8s/minio.yaml (renamed to k8s/rustfs.yaml, git records the rename)

key-decisions:
  - "Pin stays rustfs/rustfs:1.0.0 with manifest-list digest sha256:8cc9801755448b71a786705ce76692c77e14936cccd87cf2fc31842e58f4d1ff (re-verified this run; see Evidence)"
  - "s3-storage is NOT added to the Coverage job or scripts/coverage.sh (Pitfall 7): it would move the 82 percent workspace-line denominator without a CI-measured figure. The Coverage job still gets the RustFS service because its script probes the object store"
  - "k8s RustFS runs with RUSTFS_CONSOLE_ENABLE false and exposes only the api port (smaller surface; image console CORS default is *)"
  - "Docker-job contract-suite step uses `run -T` so the compose run does not allocate a TTY under `| tee`"

patterns-established:
  - "Contract-suite CI step shape: set -o pipefail; cargo test ... | tee <name>.log; sed the passed count; ::error:: and exit 1 below 11"

requirements-completed: []

duration: ~95min (includes a killed first run that left Task 1 edits uncommitted)
completed: 2026-09-30
status: complete
---

# Phase 45 Plan 04: RustFS in CI, test compose and Kubernetes Summary

**Every CI job STORE-01 names, the test compose stack and the Kubernetes reference manifest now run the pinned `rustfs/rustfs:1.0.0`; the `mc` bootstrap is gone; and the contract suite executes in CI behind a compiled-in count and a passed count that both must reach 11, so it can no longer pass vacuously.**

## Task Commits

1. **Task 1 (tracer): Integration Tests and Coverage RustFS service blocks, contract-suite steps, actionlint cleanup, CI docs** - `7dfb48a9` (ci)
2. **Task 2: rustfs-test compose service, Docker Integration and E2E jobs, script, docs** - `870a491c` (ci)
3. **Task 3: k8s/minio.yaml renamed to k8s/rustfs.yaml with readers, smoke job, docs** - `ed3fb5b8` (feat!)


## Evidence

**Interrupted-run recovery.** A prior executor was killed mid-Task-1 and left six files dirty. I read the full `git diff` of all six against the Task 1 action and acceptance criteria before doing anything else: the service blocks, deleted steps, wait loops, two new contract-suite steps, actionlint cleanup and doc edits were complete and consistent with the plan, so I adopted them (no `git checkout`) and committed them only after the verify below passed.

**Digest re-verification (D-03, T-45-16).** Docker Hub v2 tags API (`hub.docker.com/v2/repositories/rustfs/rustfs/tags`, via the proxy with the CA bundle), queried 2026-09-30: tag `1.0.0` (last updated 2026-09-16) reports `sha256:8cc9801755448b71a786705ce76692c77e14936cccd87cf2fc31842e58f4d1ff`, identical to the recorded value. The newest GA tag is still `1.0.0`; `1.0.1-preview.11` (2026-09-29) is a preview and was not adopted. (Note: the API reports the same manifest-list digest for `latest`; the pin uses the exact tag, never `latest`.)

**Tracer, local simulation (D-08).** Re-downloaded `rustfs-linux-x86_64-musl-v1.0.0.zip` (sha256 `c30a95b7...21c8`, identical to the 45-01 record; still no published checksum to compare against), ran it on a fresh empty data dir at 127.0.0.1:9010 with `testuser`/`testpass123` and console disabled, plus a throwaway listener on 6380. Extracted the two step scripts from `jobs.integration-tests.steps` with PyYAML by name and ran each with `bash -eo pipefail` under its own `env:` mapping:
- "Assert the file storage contract suite is compiled in": printed `cases compiled in: 11`, exit 0.
- "Run file storage contract suite (RustFS)": `test result: ok. 11 passed; 0 failed`, printed `passed: 11`, exit 0. The tee'd log contained no `X-Amz-Signature` or `X-Amz-Credential` string (T-45-18).
The native binary, zip, data dir, logs and the listener process were deleted afterwards. Tracer verified end to end before expansion (auto mode).

**Other gates that passed.** PyYAML parse of `ci.yml`, `docker-compose.test.yml`, `k8s/rustfs.yaml`, `deployment.yaml`, `configmap.yaml`, `secret.yaml.example`; the Task 1 verify one-liner (no `services.<id>.command` anywhere, both RustFS images equal the pin); `docker compose -f docker/docker-compose.test.yml config --quiet`; `make lint-shell` (shellcheck clean); `make check-gates` after each task; every acceptance grep in the three tasks; the plan-scoped STORE-01 grep (`quay.io/minio|minio/minio|minio/mc|MC_RELEASE|mc alias|/minio/health|minio-test`, comment lines filtered) over all of this plan's files prints nothing; `git grep -i rustfsadmin -- k8s .github docker` prints nothing; the k8s rename is recorded by git as `k8s/{minio.yaml => rustfs.yaml}`.

**CI-attributed, not run here (backstop truths).** RustFS service containers healthy, Integration Tests / Coverage / Docker Integration Tests / Kubernetes Smoke Test green with the suite reporting >= 11 passed, the actionlint job passing with the `paths:` suppression removed (`actionlint` is not installed in the sandbox), E2E on `main`, and RESEARCH assumption A7 (Actions starts the image with no `command:`). No Docker daemon exists here. **CI run id to collect at UAT:** the first `ci.yml` run on this branch after push, covering all five jobs; record it in the 45-07 evidence. If A7 proves wrong, the documented fallback is `command: rustfs /data` plus the actionlint suppression, recorded as a deviation.

## Decisions Made

- **Pitfall 7 choice (explicit).** `s3-storage` is not added to the Coverage job or `scripts/coverage.sh` (`grep -c s3-storage scripts/coverage.sh` prints 0). Adding it would change the 82 percent line-coverage denominator with no CI-measured figure to justify a new floor. The contract suite's coverage contribution is therefore unchanged; the suite is gated by its own passed-count steps instead.
- **k8s hardening beyond the swap:** pod `runAsNonRoot`, uid/gid/fsGroup 10001, `allowPrivilegeEscalation: false`, console off and console port dropped from container and Service (T-45-19, T-45-20).
- **Secret example values** are placeholders (`paladin-store`, `change-me-store-secret`) with a replace-me comment; the smoke job uses `smokeadmin`/`smokepass123`; CI/compose use `testuser`/`testpass123` and `e2euser`/`e2epassword123`. The image default credential literal appears nowhere (T-45-18, T-45-21).
- The three deleted buckets and the dropped public policy from the dev compose do NOT apply here; that is 45-05.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking/Clarity] Task 2 acceptance "no service name containing `init`" is over-broad**
- **Found during:** Task 2 verify
- **Issue:** the plan's literal check `not any('init' in k for k in s)` fails on the pre-existing, unrelated `ollama-test-init` service (Phase 17-07). The intent was that the MinIO bucket-init service is gone.
- **Fix:** no code change; verified the intent instead (`minio-test-init` absent; `integration-tests` `depends_on` is exactly `{rustfs-test, redis-test}`). The only remaining `init` service is `ollama-test-init`.

**2. [Rule 2 - Missing critical] `-T` on the Docker contract-suite `compose run`**
- **Found during:** Task 2
- **Issue:** the plan's command has no `-T`; under `| tee` a TTY allocation can merge streams or fail on some runners.
- **Fix:** `docker compose ... run --no-deps --rm -T integration-tests ...` in the new step. **Files:** `.github/workflows/ci.yml`. **Commit:** Task 2.

**3. [Rule 1 - Bug] Rename-detection of `k8s/rustfs.yaml`**
- **Found during:** Task 3 commit
- **Issue:** the first full rewrite fell just under git's 50 percent similarity, so the commit would have recorded delete+add and broken `git log --follow` (plan acceptance requires a recorded rename).
- **Fix:** shortened the header comment block and moved the pin rationale next to the `image:` line; git now records `k8s/{minio.yaml => rustfs.yaml}`.

### Additions

- `scripts/run_integration_tests.sh` starts the stack with `up -d --wait redis-test rustfs-test` (the plan said "bring up"; `--wait` is what makes the removed init wait unnecessary) and its `wait_for_services` log lines now say RustFS.
- `docs/src/appendix/integration-tests.md` Option B example passes `integration-tests,s3-storage` and states the External-mode env used inside the compose network.
- `k8s/README.md` line 8 reworded from "no Redis/MinIO required" to "no Redis or object store required".

## Known Stubs

None.

## Deferred / Out of Scope

Remaining MinIO references outside this plan's files (recorded, not touched): `docker/docker-compose.yml` (`paladin-minio`, `paladin-minio-init`) and `docs/src/deployment/docker.md` (owned by 45-05 and its docs), `.project/Milestone_1-MVP/...` (historical), `k8s/server/deployment.yaml` comment "no Redis/MinIO is required" (generic, not a config quote), the `Makefile` target label `test-integration-minio` (45-05 owns the Makefile). The phase-wide gate runs in 45-06.

## Threat Flags

None beyond the plan's register. T-45-16 (digest re-verified, exact tag), T-45-17 (compiled-in and passed counts asserted, pipefail + tee), T-45-18 (throwaway literals, no `set -x`, no presigned query string in the log), T-45-19 (console off in CI, compose and k8s), T-45-20 (non-root pod, uid 10001), T-45-21 (placeholder secret values), T-45-22 (`tmpfs /data:mode=1777`, CI-attributed) are all mitigated in the files above.

## Requirements

STORE-01 and STORE-02 are not marked complete: STORE-02's CI run and STORE-01's CI-runtime edges are backstop truths pending a recorded run id (UAT, 45-07), and STORE-01's dev compose, devcontainer and Makefile edges belong to 45-05.

## Self-Check: PASSED

- Commits `7dfb48a9`, `870a491c`, `ed3fb5b8` present in `git log`.
- `k8s/rustfs.yaml` exists, `k8s/minio.yaml` does not; `docker/docker-compose.test.yml` contains `rustfs-test:`; `.github/workflows/ci.yml` contains `rustfs/rustfs:1.0.0` (2 service blocks) and the three new step names.
- All three commit bodies end with the two required trailers.
