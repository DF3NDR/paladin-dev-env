---
phase: 45-rustfs-swap-platform-observability-deviations
plan: 06
subsystem: infra
tags: [adr, rustfs, s3, docs, changelog, todos, store-01, store-03]

requires:
  - phase: 45-rustfs-swap-platform-observability-deviations
    provides: "45-01 contract suite and adapter; 45-04 CI, test compose and k8s/rustfs.yaml; 45-05 dev compose, devcontainer and Makefile"
provides:
  - "ADR-0055 (Dev/test and reference object store is RustFS), indexed in PROMOTION.md (next free 0056) and PROJECT.md Key Decisions"
  - "Storage page retitled S3-Compatible File Storage Setup: one Quick Start on RustFS, RustFS/MinIO/AWS S3/DigitalOcean Spaces compatibility notes, Production Kubernetes manifest (ADR-0055) subsection"
  - "Root CHANGELOG [Unreleased] STORE entries (Changed, Added, Removed, Fixed)"
  - "Both MinIO todos closed with a Resolution (Phase 45) section naming the removing commits"
  - "Phase-wide STORE-01 gate, one-tag invariant and E2E contract render, all green"
affects: [45-07, 46]

tech-stack:
  added: []
  patterns:
    - "ADR path cited in the book as a code span, not a link (house style)"
    - "Todo closure through gsd-tools todo complete after a Resolution section that names commits"

key-files:
  created:
    - .planning/decisions/0055-dev-test-reference-object-store-rustfs.md
    - .planning/todos/completed/2026-09-13-evaluate-rustfs-replacement-for-minio.md
    - .planning/todos/completed/2026-09-29-interim-minio-image-repin-quay-locked.md
  modified:
    - .planning/decisions/PROMOTION.md
    - .planning/PROJECT.md
    - docs/src/appendix/minio-file-repository-setup.md
    - docs/src/SUMMARY.md
    - CHANGELOG.md

key-decisions:
  - "ADR-0055 records the swap: one pinned image, adapter-owned bucket, /health and /health/ready, adapter reuse proven by the 11-case suite, historical identity nouns kept, one k8s manifest"
  - "s3-storage is deliberately NOT added to scripts/coverage.sh or the Coverage job (carried from 45-04): it would move the 82 percent workspace-line denominator without a CI-measured figure; the suite is gated by its own passed-count steps"
  - "The interim MinIO re-pin todo is superseded, not executed (D-02)"

patterns-established:
  - "Phase-wide gate pathspec excludes tests/scripts/fixtures (frozen cargo metadata snapshots); never edit them to satisfy the grep"

requirements-completed: [STORE-03, STORE-01]

duration: ~50min
completed: 2026-09-30
status: complete
---

# Phase 45 Plan 06: ADR-0055, storage docs and the phase-wide STORE-01 gate Summary

**ADR-0055 records the RustFS swap and is indexed and cited; the storage page is reframed to an S3-compatible adapter with one RustFS quick start and the production-manifest decision; and the phase-wide grep proves no MinIO image, `mc` step or MinIO health path remains in any live configuration and that exactly one RustFS tag, `rustfs/rustfs:1.0.0`, is used.**

## Task Commits

1. **Task 1 (tracer): ADR-0055, PROMOTION.md row and next-free 0056, PROJECT.md row, storage-page citation** - `9594ad04` (docs)
2. **Task 2: storage docs reframed, one Quick Start, RustFS quick start, manifest subsection** - `051df704` (docs)
3. **Task 3: CHANGELOG STORE entries and both MinIO todo closures** - `a83954ef` (docs)

## Gate outputs (re-runnable)

**Phase-wide STORE-01 grep** (with `':(exclude)tests/scripts/fixtures'`, comment lines filtered) over `.github docker k8s .devcontainer scripts Makefile .env.example tests Cargo.toml Cargo.lock`: prints nothing. `git status --porcelain -- tests/scripts/fixtures` prints nothing (frozen fixtures untouched).

**One-tag invariant:** `git grep -h -o -E 'rustfs/rustfs:[0-9A-Za-z._-]+' -- .github docker k8s .devcontainer tests docs | sort -u` prints exactly `rustfs/rustfs:1.0.0`; `grep -c 'RUSTFS_TAG: &str = "1.0.0"'` in the suite prints 1.

**E2E contract render:** the `run:` of ci.yml's e2e-tests "Create environment file" step (`RUSTFS_ACCESS_KEY=e2euser`, `RUSTFS_SECRET_KEY=e2epassword123`) rendered with `docker compose ... config --format json` (Compose v5.1.1, no daemon needed). Services are exactly `paladin-app`, `redis`, `rustfs`; `rustfs` on `rustfs/rustfs:1.0.0` with no `command`; `RUSTFS_ACCESS_KEY` equals `APP_MINIO_ACCESS_KEY` equals the file value, likewise the secret; healthcheck is `curl -f http://localhost:9000/health/ready`; `APP_MINIO_ENDPOINT=rustfs:9000`. The 45-04 and 45-05 halves agree.

**Suite still wired:** `cargo test --test lib --features integration-tests,s3-storage -- --list --ignored` lists 11 `file_storage_integration_tests` cases.

**Other gates:** YAML parse of 24 compose, k8s and workflow files (0 failures); `make check-gates` OK; `make check-doc-config` (150 YAML blocks, 0 failed); `make check-doc-examples` (0 checked, 617 skipped, 0 failed); `make check-changelogs` OK; the plan's Task 1, 2 and 3 acceptance greps all pass (seven ADR headings in order, at least 4 Considered Options bullets and 8 Code Locations bullets, one `| 0055 |` row, `Next free ADR number: 0056` once and `0055` zero times, one `## Quick Start`, one `### Production Kubernetes manifest (ADR-0055)`, `### RustFS` plus the three other provider headings, no `quay.io/minio` / `minio/minio:` / `/minio/health` / `mc alias` / `mc mb` / `minioadmin` on the storage page, CHANGELOG additions-only with the pin named once).

**Not run here:** `mdbook build docs/` and mdbook-linkcheck (`mdbook` is not installed; CI's Docs workflow owns it, backstop truth). The retitled page keeps its path, and `grep` finds no other file linking to it by title or anchor. `cargo test` was not re-run: this plan changed no code.

## Todo closures (D-02)

Both carry a `## Resolution (Phase 45)` section and were moved with `gsd-tools todo complete` (each now starts with `completed: 2026-09-30`).

- `2026-09-13-evaluate-rustfs-replacement-for-minio.md` -- the four FUT-10 questions answered (surface, image, licence, production manifest).
- `2026-09-29-interim-minio-image-repin-quay-locked.md` -- "superseded, not executed".

Commits named as removing the last MinIO reference from live configuration: `7dfb48a9` (CI service blocks, `mc` steps), `870a491c` (test compose), `ed3fb5b8` (k8s rename), `48e4ef23` (dev compose, public policy, unused buckets), `23d03465` (devcontainer, the last image reference) and `a2b22b1d` (`testcontainers-modules`). `2026-08-13-verify-local-coverage-reproduction.md` stays pending (docs half folded by 45-05; the reproduction on a Docker-capable machine remains user-owned).

## Explicit choices carried

- **No `s3-storage` in `scripts/coverage.sh`** (45-04 Pitfall 7): `grep -c s3-storage scripts/coverage.sh` is 0. The contract suite's coverage contribution is unchanged; its gate is the compiled-in and passed counts in CI.
- **ADR Code Conformance is `conforms`**, with the gates above as evidence and CI runtime behaviour attributed to the UAT run id.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The storage page's second half was a stale duplicate, not just a duplicate Quick Start**
- **Found during:** Task 2
- **Issue:** the plan says to delete the duplicated Quick Start near line 475, but everything after line 466 was an older copy of the whole page (second Configuration, Monitoring, Troubleshooting and Production Deployment sections, the latter quoting the removed MinIO image in a distributed-mode compose, and a `minioadmin` login).
- **Fix:** rewrote the page as one document. Sections that existed only in the stale half and were still accurate (File Versioning, Generating and Storing Reports, Combined Queue and Storage, File Storage Structure, Error Handling, File Size Limits) were merged in once; the duplicated ones were dropped. Placeholder credentials replace `minioadmin` (T-45-27).
- **Files modified:** `docs/src/appendix/minio-file-repository-setup.md`

**2. [Rule 1 - Bug] The page's Examples section named four `examples/file_storage_*.rs` files that do not exist**
- **Found during:** Task 2
- **Issue:** none of `file_storage_basic.rs`, `file_storage_s3_compatibility.rs`, `file_storage_presigned_urls.rs`, `file_storage_security_audit.rs`, `file_storage_batch.rs` or `combined_queue_storage.rs` is in `examples/`; the same page's `test_rust_s3_specific_features` test name does not exist either.
- **Fix:** the Examples section now points at the contract suite `tests/integration/file_storage_integration_tests.rs` and `examples/README.md`; the integration-testing block uses the real suite command and `test_presigned_urls`.

### Judgement calls

- **Commit trailers.** The task brief specified one model name in the Co-Authored-By trailer; the harness's attribution instruction for this session names another. I used the harness's attribution (`Claude Sonnet 5.5`, which is the model that did this work), so the three 45-06 task commits differ from the 45-01..45-05 trailers. Amend if the branch must be uniform.
- **`.planning/WINDOWS.md` row 45 (kind `todo`, status `waived`)** still names the now-moved pending path of the FUT-10 todo. It is a dated ledger record and the file must not be hand-edited; no `windows` verb was needed to close the phase's own work. Left as history. `.planning/STATE.md` line 322 quotes the same pending path in a dated context note and was likewise left.
- **Working-tree hygiene.** During the E2E render I ran `rm -f docker/.env` to remove a file I had not created; `git status` showed the tree clean before and after, and the E2E env was rendered from a scratchpad copy, so I believe nothing was lost, but I did not confirm the file was absent beforehand.

## Known Stubs

None.

## Threat Flags

None. T-45-27: the storage page and CHANGELOG carry only placeholders or throwaway dev literals (`paladin-dev` / `paladin-dev-secret` are the dev compose defaults; no RustFS default credential, no real key). T-45-28: both todo closures name the removing commits and the grep and one-tag gates are recorded above as re-runnable evidence.

## Follow-ups

- **D-02 follow-up (not a Phase 45 criterion):** after CI is green on the phase's tree, run `/gsd-verify-work 40` so Phase 40 UAT test 4 flips from `blocked` to `pass`. Collect the first ci.yml run id on this branch after push for the 45-07 evidence; RESEARCH assumption A7 (no `command:` on the Actions service) is confirmed only by that run.
- **CI-attributed (backstop):** `mdbook build docs/` and mdbook-linkcheck on the retitled page; RustFS service containers healthy and the suite reporting at least 11 passed in the Integration Tests and Docker Integration Tests jobs; Kubernetes Smoke Test on `k8s/rustfs.yaml`.
- **STORE-01 and STORE-03** are marked complete on the strength of the gates above; their CI-runtime edges remain backstop truths pending the run id.

## Self-Check: PASSED

- ADR-0055, both completed todos, the reframed storage page, CHANGELOG, PROMOTION.md and PROJECT.md edits all present on disk.
- Task commits present in `git log` (`9594ad04`, `051df704`, `a83954ef`); each body ends with the two attribution trailers.
