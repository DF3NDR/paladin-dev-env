---
status: testing
phase: 54-first-run-usability
source: [54-VERIFICATION.md]
started: 2026-11-01T02:35:00Z
updated: 2026-11-01T03:05:00Z
---

## Current Test

number: 2
name: Apply the filled Secret and both Deployments on a real cluster (kind)
expected: |
  The `paladin-server` and `paladin-worker` pods read `/etc/paladin/config.yml`, mounted from the
  `paladin-server-secrets` Secret (`items: config.yml`, `defaultMode: 0440`, `fsGroup: 65532`), and boot;
  `/ready` answers. If a pod cannot read the file, the documented fallback is `defaultMode: 0444` on both
  Deployments' `config` volume (54-06 SUMMARY, RESEARCH A1).
awaiting: user response

## Tests

### 1. Decide WR-02 — the k8s Secret template's provider key placeholder
expected: The maintainer decides whether `openai-api-key: "REPLACE_ME"` in `k8s/server/secret.yaml.example` becomes `""` (refused at boot, header reworded) or stays as an accepted residual
result: pass

### 2. Apply the filled Secret and both Deployments on a real cluster (kind)
expected: The `paladin-server` and `paladin-worker` pods read `/etc/paladin/config.yml`, mounted from the `paladin-server-secrets` Secret (`items: config.yml`, `defaultMode: 0440`, `fsGroup: 65532`), and boot; `/ready` answers. If a pod cannot read the file, the documented fallback is `defaultMode: 0444` on both Deployments' `config` volume (54-06 SUMMARY, RESEARCH A1)
result: [pending]

### 3. Docker compose first run without step 1 (no `config.yml` yet)
expected: Following `docker/docker-compose.server.yml` header step 2 before step 1 either fails with a clear error, or the recipe prevents Docker from creating a root-owned `config.yml/` directory and a `restart: unless-stopped` loop (54-REVIEW WR-03)
result: [pending]

## Summary

total: 3
passed: 1
issues: 0
pending: 2
skipped: 0
blocked: 0

## Gaps
