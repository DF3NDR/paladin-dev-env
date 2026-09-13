---
phase: quick-260913-h7l
plan: 01
subsystem: infra
tags: [ci, github-actions, minio, mc, curl, sha256, yaml]

requires:
  - phase: quick-260913-15w
    provides: quay.io MinIO/mc image pins (RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772 / RELEASE.2025-08-13T08-35-41Z) that this plan's checksum target is derived from
provides:
  - Both "Install MinIO Client" steps in ci.yml (integration-tests, coverage jobs) fetch mc RELEASE.2025-08-13T08-35-41Z from the archived github.com/minio/mc release assets with sha256sum verification instead of the retired dl.min.io host
affects: [ci-pipeline, minio-rustfs-evaluation]

tech-stack:
  added: []
  patterns: ["checksum-verified curl fetch of a pinned third-party binary in CI (curl -fsSL + sha256sum -c -, no credential header so -L is safe)"]

key-files:
  created: []
  modified:
    - .github/workflows/ci.yml
    - .planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md

key-decisions:
  - "Sourced mc from the archived github.com/minio/mc repository's release assets (still downloadable) rather than re-hosting the binary or vendoring it, per the plan's closed decision"
  - "Checksum verification (sha256sum -c -) is load-bearing, not advisory — a mismatch fails the step before chmod +x/mv, per T-h7l-01 mitigation"

requirements-completed: [QUICK-260913-h7l]

coverage:
  - id: D1
    description: "Both Install MinIO Client steps (integration-tests, coverage jobs) fetch mc RELEASE.2025-08-13T08-35-41Z from github.com/minio/mc release assets and verify sha256 01f866e9c5f9b87c2b09116fa5d7c06695b106242d829a8bb32990c00312e891 before installing to /usr/local/bin/mc"
    requirement: "QUICK-260913-h7l"
    verification:
      - kind: other
        ref: "python3 -c \"import yaml,sys; list(yaml.safe_load_all(open('.github/workflows/ci.yml'))); print('YAML_OK')\" -> YAML_OK"
        status: pass
      - kind: other
        ref: "grep -cF '01f866e9c5f9b87c2b09116fa5d7c06695b106242d829a8bb32990c00312e891' .github/workflows/ci.yml -> 2"
        status: pass
      - kind: other
        ref: "grep -cF 'github.com/minio/mc/releases/download' .github/workflows/ci.yml -> 2"
        status: pass
      - kind: other
        ref: "PyYAML-extracted run bodies of both 'Install MinIO Client' steps: bash -n exits 0 on both, diff reports identical"
        status: pass
    human_judgment: false
  - id: D2
    description: "Zero references to the retired dl.min.io download host remain outside .planning/, and the diff touches only ci.yml and the RustFS todo with no changed line carrying an image reference, port, credential, or bucket command"
    requirement: "QUICK-260913-h7l"
    verification:
      - kind: other
        ref: "grep -rnIE --exclude-dir=.git --exclude-dir=.planning --exclude-dir=target --exclude-dir=book 'dl\\.min\\.io' . | wc -l -> 0"
        status: pass
      - kind: other
        ref: "git diff --name-only | sort -> exactly .github/workflows/ci.yml and .planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md"
        status: pass
      - kind: other
        ref: "git diff -U0 -- .github/workflows/ci.yml | grep -E '^[+-]' | grep -vE '^(\\+\\+\\+|---)' | grep -cE 'image:|9010|9011|MINIO_ROOT|testuser|testpass123|mc alias set|mc mb ' -> 0"
        status: pass
    human_judgment: false
  - id: D3
    description: "Real end-to-end verification — CI Coverage and Integration Tests jobs clear 'Install MinIO Client' and 'Setup MinIO buckets' on the post-push run"
    verification: []
    human_judgment: true
    rationale: "Docker is unavailable in this environment; static verification (YAML parse, grep counts, bash -n, diff) is the strongest check available locally. The orchestrator owns the post-push CI run that provides the real end-to-end proof."

duration: ~10min
completed: 2026-09-13
status: complete
---

# Quick Task 260913-h7l: Replace dl.min.io mc download in ci.yml Summary

**Both CI "Install MinIO Client" steps now fetch `mc` RELEASE.2025-08-13T08-35-41Z from the archived `github.com/minio/mc` GitHub release assets with mandatory sha256 verification, replacing the `dl.min.io` host that now returns HTTP 410 Gone.**

## Performance

- **Duration:** ~10min
- **Completed:** 2026-09-13
- **Tasks:** 1 completed
- **Files modified:** 2

## Accomplishments

- Rewrote both `Install MinIO Client` steps in `.github/workflows/ci.yml` (integration-tests job and coverage job) to fetch the pinned `mc` release from `github.com/minio/mc`'s archived-repository release assets instead of the retired `dl.min.io` community download host, with `curl -fsSL` + `sha256sum -c -` verification gating `chmod +x`/`sudo mv` so a checksum mismatch or non-2xx response fails the step rather than installing a bad binary.
- Added a six-line inline comment above each step explaining why the fetch source changed, without writing the retired host's bare domain literal into the file (kept the completion-gate grep for `dl.min.io` truthful).
- Appended one sentence to the RustFS replacement todo's `## Problem` section recording that MinIO's community binary downloads are gone as well as its Docker Hub images, and that this quick task pinned the CI client install accordingly.
- Left both `Setup MinIO buckets` steps, all service blocks, ports, credentials, and bucket-creation commands byte-identical.

## Task Commits

1. **Task 1: Fetch the pinned mc release from GitHub in both CI jobs, checksum-verified** - `9d0aa7a0` (fix)

**Plan metadata:** commit pending (orchestrator-owned docs commit)

## Files Created/Modified

- `.github/workflows/ci.yml` - Both `Install MinIO Client` steps (integration-tests job, coverage job) rewritten to fetch and checksum-verify `mc` from GitHub release assets instead of `dl.min.io`; six-line rationale comment added above each
- `.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md` - One sentence appended to `## Problem` recording that MinIO's client-binary downloads are gone too, not just server/client images, and how this quick task worked around it

## Decisions Made

- Sourced `mc` from the archived `github.com/minio/mc` repository's release assets — the plan's decision was closed and not re-opened; release assets on archived repos remain downloadable, unlike the deleted Docker Hub community images.
- `curl -fsSL` (not `wget`) was used per the plan's explicit spec: `-f` fails the step on any non-2xx response so an HTML error page can never be executed, and `-L` is safe here because no credential header is attached to this request (unlike the LLM adapters' credential-bearing clients, which refuse redirects per `security.instructions.md`).

## Deviations from Plan

None - plan executed exactly as written. The six-line comment content, seven-line `run:` body shape, and todo-file sentence all match the plan's `<action>` specification verbatim.

## Issues Encountered

None. The plan's own `<read_first>` warning about the two step bodies being byte-identical (making a naive single-match edit ambiguous) was addressed by using a Python script that asserted exactly 2 occurrences of the old block before replacing both — avoiding the single-match pitfall the prior quick task (260913-15w) had already hit and documented.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Static verification is complete and green (`CI_MC_PINNED_OK`): YAML parses, zero `dl.min.io` references remain outside `.planning/`, the pinned sha256 and GitHub release URL each appear exactly twice, both step bodies are byte-identical and `bash -n`-clean, and the diff is scoped to exactly the two expected files with no image/port/credential/bucket-command lines touched.

Real end-to-end verification — confirming the Coverage and Integration Tests jobs clear "Install MinIO Client" and "Setup MinIO buckets" — is owned by the orchestrator via the post-push CI run. `actionlint` was not installed in this environment and is recorded as SKIPPED, not passed.

No blockers for the RustFS replacement todo, which now also tracks the client-binary half of the MinIO retirement problem.

---
*Quick task: 260913-h7l*
*Completed: 2026-09-13*

## Self-Check: PASSED

- FOUND: `.github/workflows/ci.yml`
- FOUND: `.planning/todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`
- FOUND: `.planning/quick/260913-h7l-replace-dl-min-io-mc-download-in-ci-yml-/260913-h7l-SUMMARY.md`
- FOUND commit: `9d0aa7a0`
