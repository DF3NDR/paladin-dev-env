---
status: testing
phase: 54-first-run-usability
source: [54-VERIFICATION.md]
started: 2026-11-01T02:35:00Z
updated: 2026-11-01T02:35:00Z
---

## Current Test

number: 1
name: Decide WR-02 — the k8s Secret template's provider key placeholder
expected: |
  `k8s/server/secret.yaml.example` carries `openai-api-key: "REPLACE_ME"`, which the server accepts as a live
  provider key, so a half-filled Secret (API keys pasted, provider key forgotten) starts healthy and sends
  `REPLACE_ME` as the provider bearer. Either change it to an empty value, which is refused at boot, and fix the
  file header's "unfilled template fails closed" wording, or accept it. FIRST-01 and SC1 cover
  `http.auth.api_keys[].key`, which is fixed; the phase goal's "no example loads a placeholder as a credential"
  is the wording at stake.
awaiting: user response

## Tests

### 1. Decide WR-02 — the k8s Secret template's provider key placeholder
expected: The maintainer decides whether `openai-api-key: "REPLACE_ME"` in `k8s/server/secret.yaml.example` becomes `""` (refused at boot, header reworded) or stays as an accepted residual
result: [pending]

### 2. Apply the filled Secret and both Deployments on a real cluster (kind)
expected: The `paladin-server` and `paladin-worker` pods read `/etc/paladin/config.yml`, mounted from the `paladin-server-secrets` Secret (`items: config.yml`, `defaultMode: 0440`, `fsGroup: 65532`), and boot; `/ready` answers. If a pod cannot read the file, the documented fallback is `defaultMode: 0444` on both Deployments' `config` volume (54-06 SUMMARY, RESEARCH A1)
result: [pending]

### 3. Docker compose first run without step 1 (no `config.yml` yet)
expected: Following `docker/docker-compose.server.yml` header step 2 before step 1 either fails with a clear error, or the recipe prevents Docker from creating a root-owned `config.yml/` directory and a `restart: unless-stopped` loop (54-REVIEW WR-03)
result: [pending]

## Summary

total: 3
passed: 0
issues: 0
pending: 3
skipped: 0
blocked: 0

## Gaps
