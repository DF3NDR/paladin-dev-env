---
phase: 54-first-run-usability
plan: 06
subsystem: infra
tags: [kubernetes, secret, config, makefile, docker, changelog, migration, first-run]
status: complete

requires:
  - phase: 54-first-run-usability
    provides: "54-01 refusal of unexpanded ${...} API keys, M-B-29, MIGRATION 11.5/11.8 entries"
provides:
  - "k8s/server/secret.yaml.example carrying the whole server config.yml (fail-closed, empty keys) and the provider key"
  - "deployment.yaml and worker-deployment.yaml mounting config.yml from paladin-server-secrets via secret volume items, defaultMode 0440"
  - "k8s/server/configmap.yaml deleted; per-caller API-key env injections and Secret keys removed"
  - "k8s version labels (v0.11.0) and paladin-server image tags (0.11.0) at the workspace version"
  - "make docker-build-server tags :latest and :<workspace version> (WORKSPACE_VERSION read offline with tomllib)"
  - "k8s/README.md copy-paste server quick start; CHANGELOG [Unreleased] ### Changed; MIGRATION 11.5 bullet and 11.8 step 5"
affects: [54-07 version-strings gate, 54-09 evidence and Phase 57 residual]

tech-stack:
  added: []
  patterns:
    - "Config holding literal secrets travels only in a Secret, projected with items so sibling keys are not exposed as files"
    - "Image tag derived from the workspace manifest by the build recipe, not hard-coded in the Makefile"

key-files:
  created: []
  modified:
    - k8s/server/secret.yaml.example
    - k8s/server/deployment.yaml
    - k8s/server/worker-deployment.yaml
    - k8s/deployment.yaml
    - k8s/README.md
    - Makefile
    - CHANGELOG.md
    - MIGRATION.md
  deleted:
    - k8s/server/configmap.yaml

key-decisions:
  - "PD-06-1: same Secret, config.yml stringData key, items projection (not subPath), defaultMode 0440; configmap.yaml deleted rather than left as a pointer"
  - "PD-06-2: WORKSPACE_VERSION is recursively expanded so only recipes that use it run python3"
  - "PD-06-3: README quick start applies secret.yaml, deployment.yaml, service.yaml explicitly, never the whole k8s/server/ directory"

requirements-completed: [FIRST-01, FIRST-04]

duration: ~20min
completed: 2026-11-01
---

# Phase 54 Plan 06: k8s Secret-delivered server config and workspace-version tags Summary

**The `paladin-server` config, with its literal API keys, now reaches both Kubernetes Deployments only as the `config.yml` key of `paladin-server-secrets` (an unfilled template fails closed), and every k8s label and first-party image tag reads the workspace version that `make docker-build-server` now tags.**

## Performance

- **Plan-start SHA:** `3ad89db2faf986a0ad26c3c4550f61291755306c`
- **Tasks:** 3 (1 tracer, 2 auto)
- **Files modified:** 8, 1 deleted

## Accomplishments

- Secret template: `stringData` is exactly `openai-api-key` and `config.yml`; `http.auth.enabled: true`, both `api_keys[].key` are `""`, `llm_api_key: ""`, and `llm.openai.api_key: "${OPENAI_API_KEY}"` (the working exact-name mechanism). Comments say keys are literal and nothing in `api_keys` is expanded.
- Both Deployments mount the Secret (`items: config.yml`, `defaultMode: 0440`) at `/etc/paladin` read-only; server env is exactly `PALADIN_CONFIG` and `OPENAI_API_KEY`; the worker's `APP_*` env, labels and mount are untouched by Task 1.
- Six `version:` labels read `v0.11.0`; both images read `paladin-server:0.11.0` with `IfNotPresent`; `k8s/deployment.yaml` changed only its two `version:` lines (`paladin:test`, `imagePullPolicy: Never` intact).
- `make -n docker-build-server` prints one build line with `-t paladin-server:latest -t paladin-server:0.11.0`; an empty `WORKSPACE_VERSION` fails the recipe with the named message (tried with `WORKSPACE_VERSION=`).
- README quick start is a copy-paste sequence naming the tag only as `<workspace version>`; root `k8s/configmap.yaml` lines kept (count 2, unchanged).

## Task Commits

| Task | Commit | Description |
| ---- | ------ | ----------- |
| 1 (tracer) | `1c16deee3` | Secret carries config.yml; both Deployments mount it; configmap.yaml deleted |
| 2 | `1a4ddfd47` | Labels/images at workspace version; Makefile WORKSPACE_VERSION and second tag |
| 3 | `b4e5222b8` | README quick start, CHANGELOG `### Changed`, MIGRATION 11.5 and 11.8 |

## Prove-then-lock outputs

Task 1 before (hits in `k8s/server`): `worker-deployment.yaml:147` and `deployment.yaml:102` name `paladin-server-config`; `deployment.yaml:66-75` inject `PALADIN_API_KEY_CI`/`_APP` from `api-key-ci`/`api-key-app`; `secret.yaml.example:7-8,19-20` carry the two keys; `configmap.yaml:4,8,36,40` carry the expansion claim, the ConfigMap name and the two `key: "${...}"` entries. After: the same grep over `k8s/server` exits 1 (no hits), and `grep -rnE '(^\s*|[{,]\s*|-\s+)key:\s*"?\$\{' k8s` exits 1.

Task 2 before: labels `v0.10.0` (worker, l.20/35), `v0.6.0` (server, l.13/28), `v0.1.0` (root, l.8/23); images `paladin-server:0.10.0`, `paladin-server:0.6.0`, `paladin:test`; `make -n` showed a single `-t paladin-server:latest`. After: all labels `v0.11.0`, both server images `0.11.0`, `make -n` shows both `-t` flags.

Task 3 before: `k8s/README.md` hits at l.10 (`server/configmap.yaml` bullet), l.12 (ConfigMap mount), l.21 (`-f k8s/server/` apply), l.25 (scaling note), l.165 (link); root `k8s/configmap.yaml` lines at l.106 and l.136. After: no `server/configmap.yaml`, no directory apply, no `paladin-server:<digits>` literal.

## PyYAML results

All three tasks' `<verify>` commands exit 0 (PyYAML 6.0.1): Secret kind/name/keys, `config.yml` parse with the required legacy fields, empty `api_keys[].key`, server env exactly `['PALADIN_CONFIG', 'OPENAI_API_KEY']`, worker `config` volume equal to the server's (`secretName`, `items`, `defaultMode == 0o440`), labels/images at `0.11.0`, `paladin:test` still present. `make check-migration-allowlist` exits 0; `grep -c TBD MIGRATION.md` is 0. No `kubectl`, `kind` or docker daemon available, so only manifest shape was checked.

## Deviations from Plan

None - plan executed as written. The tracer gate (auto mode active) re-ran the Task 1 verify, which passed, before Tasks 2 and 3.

## Decisions Made

See `key-decisions` above (PD-06-1, PD-06-2, PD-06-3 applied as planned).

## Untested assumption: Secret file mode (RESEARCH A1)

- `defaultMode: 0440` with the pod's `fsGroup: 65532` was not exercised on a cluster (no `kubectl` or `kind` here). The PyYAML checks prove manifest shape only, not that the non-root container can read `/etc/paladin/config.yml`.
- Fallback, if a pod fails to start with a permission error reading that file: set `defaultMode: 0444` on the `config` volume of both `k8s/server/deployment.yaml` and `k8s/server/worker-deployment.yaml`. `items` still projects only `config.yml`, so the provider key is not exposed as a file.
- Plan 54-09 carries this as a known residual for Phase 57.

## Known Stubs

None. `REPLACE_ME` in `openai-api-key` and the empty `key: ""` entries are the intentional unfilled-template placeholders; the empty keys are refused at boot.

## Threat Flags

None. The change moves existing material from a ConfigMap to a Secret and adds no new endpoint, auth path or trust boundary.

## Issues Encountered

None. Note for 54-07: `k8s/README.md` still has a heading `### Worker replicas (Platform API, v0.10)`, a version string in prose that the version-strings gate may need to classify.

## Self-Check: PASSED

- `k8s/server/configmap.yaml` absent; `k8s/server/secret.yaml.example`, `deployment.yaml`, `worker-deployment.yaml`, `k8s/README.md` present.
- Commits `1c16deee3`, `1a4ddfd47`, `b4e5222b8` exist on `claude/laughing-dirac-e0h2ax`.
