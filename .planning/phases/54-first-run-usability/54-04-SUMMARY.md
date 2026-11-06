---
phase: 54-first-run-usability
plan: 04
subsystem: docs
tags: [docs, versioning, first-run, pins, deployment, images]

requires:
  - phase: 54-first-run-usability
    provides: "54-03 sweep of README, quickstart, installation, crate-map, feature-flags and sentinel pins"
provides:
  - "User-guide and contributor-guide workspace pins at 0.11.0"
  - "Version-free present-tense banners in four user guides"
  - "Version-free copyable image tags in deployment/docker.md and deployment/kubernetes.md"
affects: [54-07 version-string gate, 54-09 evidence]

tech-stack:
  added: []
  patterns:
    - "Prose that names the current release is made version-free; only the four gated classes carry a literal workspace version"
    - "Deployment-guide image tags are X.Y.Z / X.Y / X placeholders, the form release.yml's docker/metadata-action pushes"

key-files:
  created: []
  modified:
    - docs/src/user-guides/paladin-agents.md
    - docs/src/user-guides/battalion-patterns.md
    - docs/src/user-guides/maneuver-flow-dsl.md
    - docs/src/contributing/architecture-decisions.md
    - docs/src/user-guides/orchestration.md
    - docs/src/user-guides/content-processing.md
    - docs/src/user-guides/superstep-engine.md
    - docs/src/user-guides/agent-orchestrator-bridge.md
    - docs/src/deployment/docker.md
    - docs/src/deployment/kubernetes.md

key-decisions:
  - "Banners and the content-processing 'not yet implemented in v0.10.0' claim made version-free rather than bumped (PD-04-1)"
  - "architecture-decisions.md is swept, not exempted; its paladin-custom-llm 0.1.0 and 0.1 lines stay (PD-04-2)"
  - "Deployment-guide image tags become version-free placeholders, not bumped, so D-07's class 3 stays k8s/ only (PD-04-3)"

requirements-completed: [FIRST-02]

status: complete
---

# Phase 54 Plan 04: Guide pins, banners and image tags Summary

**Ten doc pages no longer name a stale release: four copyable `paladin-ai`/`paladin-battalion` pins read 0.11.0 (including the contributor adapter-author manifest, which was `0.5`), five "current v0.10.0" claims and one footer version line are version-free, and every `docker run`/`image:`/Helm `tag:` in the deployment guides is an `X.Y.Z` placeholder.**

## Facts

- Plan-start SHA: `8ffa80563aebcf690ce8cdb2bf1ee70c2a652953`
- Computed `V` (root `[package].version`, via `tomllib`): `0.11.0`
- 3 tasks, 10 files, docs only; no code, no tests changed.

## Prove-then-lock hit lists (re-measured before editing)

Task 1 (`grep -nE '"0\.(5|10\.0)"|Version\*\*: 0\.10\.0'` over the four files):
`paladin-agents.md:36`, `battalion-patterns.md:51` (both `paladin-ai ... "0.10.0"`), `maneuver-flow-dsl.md:55` (`paladin-battalion ... "0.10.0"`) and `:1154` (`**Version**: 0.10.0`), `architecture-decisions.md:645` (`paladin-ai ... "0.5"`). Matches the plan's expected set.

Task 2 (`grep -n 'current \*\*v0\.10\.0\*\*' docs/src/user-guides/*.md`):
`agent-orchestrator-bridge.md:14`, `content-processing.md:9`, `orchestration.md:14`, `superstep-engine.md:7`. The `in$` grep on `content-processing.md` hit l.161 (`...**not yet implemented** in`), with `v0.10.0. To keep this guide honest:` on l.162.

Task 3 (`grep -nE 'paladin:v?[0-9]|tag: "v?[0-9]|v0\.[0-9]'` over docker.md and kubernetes.md):
`docker.md` l.91, 105, 515, 549, 551, 552, 553, 566, 664 (all `v0.8.0` / `v0.8` / `v0`); `kubernetes.md` l.206 (`paladin:v0.8.0`), l.314 (`### Worker replicas (Platform API, v0.10)`, kept), l.518 (`tag: "v0.8.0"`).

`release.yml` l.221-223 `docker/metadata-action@v5` tag patterns (unchanged from the plan's measurement): `type=semver,pattern={{version}}`, `type=semver,pattern={{major}}.{{minor}}`, `type=semver,pattern={{major}}`, plus `type=raw,value=latest,enable={{is_default_branch}}`. The `X.Y.Z` / `X.Y` / `X` placeholders therefore match what the release workflow pushes (no leading `v`).

## Task Commits

1. Task 1 (tracer): `d1325ebc6` docs(54-04): user and contributor guide pins read the workspace version
2. Task 2: `a0b5bc8ed` docs(54-04): user-guide banners stop naming a stale current version
3. Task 3: `437bafeae` docs(54-04): deployment-guide image tags are version-free

Tracer gate: Task 1's `<verify>` passed immediately after its commit, so expansion proceeded.

## Verification

- All three tasks' `<verify>` commands pass (the Task 3 chain ends in `make check-doc-config`: 161 YAML blocks checked, 0 failed).
- `git diff --stat 8ffa80563 HEAD -- docs/` lists exactly the ten `files_modified` (21 insertions, 22 deletions); `docs/src/api-reference/migration-guide.md` has an empty diff.
- Task 3 changed only the eleven listed lines (nine in `docker.md`, two in `kubernetes.md`); the comment column of the `Tagging Strategy` block was re-aligned with the `:latest` line.
- Kept as designed: `**Last Updated**` and `**Status**` footer lines, `superstep-engine.md` `**Since:** v0.10.0 (Phase 22)`, the adapter's `version = "0.1.0"` and `paladin-custom-llm = "0.1"`, `:latest`/`:main`/`:dev-*`, `ghcr.io/your-org`, `imagePullPolicy: IfNotPresent`, and the `(Platform API, v0.10)` heading.

## Deviations from Plan

None - plan executed exactly as written. `content-processing.md`'s edited sentence was re-wrapped by one line break (`**not yet implemented**.` then `To keep this guide honest:`) to keep both lines short.

## Known Stubs

None.

## Threat Flags

None. Documentation edits only; no new endpoint, auth path or trust-boundary surface.

## Residuals for plan 54-09

- The illustrative `ghcr.io/your-org` registry owner in the deployment guides is not a version and stays.
- The deployment pages sit outside the 54-07 gate's four pattern classes, so their placeholders are ungated by design (T-54-31).

## Self-Check: PASSED

All ten files exist and commits `d1325ebc6`, `a0b5bc8ed`, `437bafeae` are present in `git log`.
