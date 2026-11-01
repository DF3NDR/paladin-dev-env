---
phase: 54-first-run-usability
plan: 02
subsystem: docker-docs
tags: [docker, compose, docs, first-run, api-keys, kubernetes]

requires:
  - phase: 54-first-run-usability
    provides: "54-01 refusal of a ${...} HTTP API key and config.example.yml with api_keys: []"
provides:
  - "docker compose first run that mounts the operator's config.yml read-only"
  - "Dockerfile.server header run snippet that mounts a config and explains the refusal"
  - "http-service-host.md with literal-key sample, the one rule about ${...}, and honest binary, Docker and Kubernetes snippets"
affects: [54-06 k8s secret config, 54-09 evidence]

tech-stack:
  added: []
  patterns:
    - "Docs never show an HTTP API key as an environment variable or a ${...} placeholder; sample uses key: \"\" so a verbatim copy never authenticates"

key-files:
  created: []
  modified:
    - docker/docker-compose.server.yml
    - Dockerfile.server
    - docs/src/deployment-topologies/http-service-host.md

key-decisions:
  - "Image tag in Docker material is paladin-server:latest, the tag make docker-build-server produces; no version string in docker/ or Dockerfile* (PD-02-1)"
  - "Config sample uses key: \"\" with a paste-a-literal-secret comment (PD-02-2)"
  - "Kubernetes snippet names the three server files explicitly and copies secret.yaml.example (built by 54-06) (PD-02-3)"

requirements-completed: [FIRST-01]

coverage:
  - id: D1
    description: "compose file mounts ../config.yml:/etc/paladin/config.yml:ro, uses paladin-server:latest, carries only PALADIN_CONFIG and OPENAI_API_KEY, and renders"
    requirement: FIRST-01
    verification:
      - kind: other
        ref: "PyYAML assertion on docker/docker-compose.server.yml plus docker compose -f docker/docker-compose.server.yml config"
        status: pass
    human_judgment: false
  - id: D2
    description: "Dockerfile.server diff is comment-only; no PALADIN_API_KEY_ string or version tag in Docker material"
    requirement: FIRST-01
    verification:
      - kind: other
        ref: "git diff of Dockerfile.server filtered for non-comment lines prints nothing; grep PALADIN_API_KEY_ and paladin-server:[0-9] find nothing"
        status: pass
    human_judgment: false
  - id: D3
    description: "http-service-host.md carries no ${...} HTTP key, states the boot refusal and the llm-only expansion, applies three k8s files explicitly"
    requirement: FIRST-01
    verification:
      - kind: other
        ref: "make check-doc-config (161 blocks, 0 failed) plus the Task 2 grep assertions"
        status: pass
    human_judgment: false
---

# Phase 54 Plan 02: Docker first run honest after FIRST-01 Summary

**The compose file, Dockerfile header and service-host guide now all start the server from a mounted `config.yml` holding a literal API key; the unread per-caller key environment variables are gone and the guide states the one `${...}` rule.**

## Performance

- **Duration:** about 10 min
- **Tasks:** 2 (1 tracer, 1 auto)
- **Files modified:** 3
- **Plan-start SHA:** `0d00681dc57e248002bf415d250177773d3d3059`

## Accomplishments

- `docker/docker-compose.server.yml`: `image: paladin-server:latest`, the `volumes:` block uncommented to mount `../config.yml:/etc/paladin/config.yml:ro`, environment reduced to `PALADIN_CONFIG` and `OPENAI_API_KEY`, header rewritten as a three-step first run.
- `Dockerfile.server`: header comment only; run snippet mounts `config.yml`, passes only `OPENAI_API_KEY`, and states that the baked `config.example.yml` ships `api_keys: []` so the container refuses to start until a config is mounted. `COPY`, `ENV` and build stages untouched.
- `http-service-host.md`: block-form `key: ""` sample entry, a "API key values are literal" paragraph (no `${VAR}` expansion in `http.auth.api_keys`, key containing `${` refused at boot naming the entry, only `llm.<provider>.api_key` resolves `"${NAME}"`), binary and Docker snippets that use a mounted config, and a Kubernetes paragraph (Deployment + Service, config as the `config.yml` key of a Secret) applying `secret.yaml`, `deployment.yaml`, `service.yaml` explicitly and pointing to `k8s/README.md` for the worker.

## Prove-then-lock (D-00f)

- Task 1 step 1, `grep -n 'PALADIN_API_KEY_' docker/docker-compose.server.yml Dockerfile.server` at the plan-start SHA:
  - `docker/docker-compose.server.yml:6` (header run line)
  - `docker/docker-compose.server.yml:21` (`PALADIN_API_KEY_CI`)
  - `docker/docker-compose.server.yml:22` (`PALADIN_API_KEY_APP`)
  - `Dockerfile.server:9` (header `docker run`)
  After the edits the same grep finds nothing.
- Task 2 step 1, `grep -nE '(^\s*|[{,]\s*|-\s+)key:\s*"?\$\{|PALADIN_API_KEY_|ConfigMap|k8s/server/$' docs/src/deployment-topologies/http-service-host.md`:
  - l.82 (inline `key: "${PALADIN_API_KEY_CI}"`), l.137 and l.146 (`PALADIN_API_KEY_CI` in the binary and Docker snippets), l.151 ("Deployment + Service + ConfigMap"), l.155 (`kubectl apply -f k8s/server/secret.yaml -f k8s/server/`).
  After the edit the key and `PALADIN_API_KEY_` greps find nothing, `ConfigMap` is absent, and no whole-directory apply remains.

## Task Commits

1. **Task 1: tracer, compose file and Dockerfile header** - `9efc3d284` (docs)
2. **Task 2: http-service-host.md** - `c30ab7807` (docs)

The tracer feedback gate ran in auto mode: the tracer `<verify>` (PyYAML assertion, `PALADIN_API_KEY_` grep, `docker compose config`) passed before the expansion task. The precondition (`docker compose version`) was met (v5.6.0), so `docker compose config` was the check, not only PyYAML.

## Deviations from Plan

None - plan executed exactly as written. One note: `docker compose config` renders with `../config.yml` absent (bind-mount sources are not checked at render time), so the verification does not require a local `config.yml`.

## Issues Encountered

None.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or trust-boundary surface; T-54-05, T-54-06 and T-54-07 are mitigated as the register states (baked example ships no key, only the provider key travels through the environment, the Kubernetes snippet applies the three server files and refers to `k8s/README.md` for the worker).

## Next Phase Readiness

- The Kubernetes paragraph copies `k8s/server/secret.yaml.example`, which plan 54-06 creates; until it lands the snippet names a file that does not exist yet.

## Self-Check: PASSED

- Files `docker/docker-compose.server.yml`, `Dockerfile.server`, `docs/src/deployment-topologies/http-service-host.md` modified and committed.
- Commits `9efc3d284` and `c30ab7807` present in `git log`.
- `grep -rn 'PALADIN_API_KEY_' docker Dockerfile.server docs/src` finds nothing; `make check-doc-config` exits 0.
