---
phase: 54
slug: first-run-usability
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-11-01
verified_head: 7ff7a9b5e
---

# Phase 54 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time. All nine PLAN files (54-01 through 54-09) carry a
`<threat_model>` block (32 threats: T-54-01 through T-54-31 plus T-54-SC); all nine SUMMARY files
report `## Threat Flags: None` (the phase adds no network endpoint, auth path or trust boundary --
it tightens an existing validator, moves k8s config from a ConfigMap to a Secret, adds a check-only
version-string gate and corrects documentation and changelogs). The verification below is L1
grep-depth against the tree at `7ff7a9b5e`, which the ASVS level 1 configuration accepts as
sufficient when no threat at or above the `high` block threshold remains open. Each "mitigate" row
names the function, file or test that was confirmed present; each "accept" row names where the
acceptance is recorded.

The one human decision the phase routed through UAT (54-REVIEW WR-02, the provider-key
`REPLACE_ME` placeholder in the k8s Secret template) was passed by the maintainer in `54-UAT.md`
test 1 and is recorded below as accepted risk AR-54-03. It does not reopen T-54-16, whose scope
is `http.auth.api_keys[].key`.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| operator config file -> boot | YAML text becomes the set of accepted `X-API-Key` credentials | API key literals; `${VAR}` placeholders (refused) |
| boot error -> operator logs | refusal messages are printed to stderr and collected by log pipelines | entry names only, never key values |
| operator shell -> container | how the HTTP API keys and the provider key reach the server process | `config.yml` (read-only mount); `OPENAI_API_KEY` env |
| documentation -> operator / user `Cargo.toml` | copy-paste snippets, pins and image tags become production builds and deployments | crate versions, image tags, apply commands |
| published records -> operator decisions | changelogs and ADRs decide upgrade order and allowance sizing | migration order (M-B-12), allowance race posture (ADR-0057) |
| cluster API -> pod filesystem | the Secret's `config.yml` becomes `/etc/paladin/config.yml` | whole server config including literal keys |
| ConfigMap vs Secret | ConfigMaps are readable by a wider set of principals and are not treated as secret material | (retired: `configmap.yaml` deleted) |
| repository tree -> gate verdict | a green `check-version-strings.sh` is read as "the first-run material is current" | version strings across 128 files |
| fixture seam -> gate inputs | `VERSION_STRINGS_ROOT` relocates the scanned tree (test-only) | scan root path |
| changelog / release config -> release tooling | `make release` and the release workflow read these files | section extraction, compare links |
| evidence file -> later phases' audits | Phase 57 and Phase 58 accept claims recorded in `54-FIRST-EVIDENCE.md` | test result lines, head SHA, captured stderr |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-54-01 | Spoofing / Elevation of privilege | `AuthConfig::validate`: an unexpanded `${VAR}` loaded as a valid key | high | mitigate | `entry.key.contains("${")` refusal at `src/config/agents.rs:221`; boot test `build_auth_config_refuses_an_unexpanded_placeholder_key` (`src/bin/paladin-server.rs:824`) refuses before `TcpListener::bind`; process-level run recorded in `54-FIRST-EVIDENCE.md` "Boot refusal (SC1)" | closed |
| T-54-02 | Information disclosure | refusal message echoing the key or the variable name | high | mitigate | Message at `agents.rs:223` interpolates only `{name}`; refusal tests assert `!err.contains("sk-ci")` (`agents.rs:696,709,775`); manual credential review in `54-FIRST-EVIDENCE.md` "Manual credential-handling review" | closed |
| T-54-03 | Elevation of privilege | refusal skipped while `enabled: false`, then auth re-enabled | medium | mitigate | `validate_refuses_a_placeholder_key_while_auth_is_disabled` (`agents.rs:1034`) | closed |
| T-54-04 | Denial of service | the shipped example failing its own rule or loading a key by accident | low | mitigate | `shipped_example_http_auth_validates_and_configures_no_key` (`agents.rs:1093`), `shipped_example_api_key_entries_uncommented_are_refused_as_empty` (`agents.rs:1117`) | closed |
| T-54-SC | Tampering | package installs | low | accept | Phase installs no package (RESEARCH Package Legitimacy Audit empty by construction); see AR-54-01 | closed |
| T-54-05 | Spoofing | Docker first run on the baked example config | high | mitigate | `config.example.yml` ships `api_keys: []` with commented `key: ""` entries; `docker/docker-compose.server.yml:28` mounts `../config.yml:/etc/paladin/config.yml:ro`; zero `PALADIN_API_KEY` hits in `docker/`, `README.md`, `docs/src` | closed |
| T-54-06 | Information disclosure | API keys on the `docker run` command line or shell environment | low | mitigate | Compose `environment:` carries only `PALADIN_CONFIG` and `OPENAI_API_KEY` (`docker-compose.server.yml:19-24`); HTTP keys live in the gitignored, read-only mounted file | closed |
| T-54-07 | Denial of service | the Kubernetes quick start applying the worker without its Secret, database and queue | low | mitigate | `docs/src/deployment-topologies/http-service-host.md:170` applies `secret.yaml`, `deployment.yaml`, `service.yaml` explicitly and l.174 points to `k8s/README.md` for the worker | closed |
| T-54-08 | Tampering (supply-chain drift) | stale pins steering users to `0.5`/`0.10.0` | low | mitigate | Every workspace pin in README, `docs/src`, `crates/*/README.md` reads `0.11.0` except the exempt `migration-guide.md`; gate wired at `Makefile:268` and `ci.yml:182` | closed |
| T-54-09 | Tampering | the sweep changing a non-workspace pin or a historical row | low | mitigate | `paladin-ai = "0.4"` rows appear only in `docs/src/api-reference/migration-guide.md` (the single `EXEMPT` entry, D-08) | closed |
| T-54-10 | Tampering (supply-chain drift) | adapter authors pinning `0.5` from the contributor guide | low | mitigate | Pins read `0.11.0`; the file is inside the gate scan (not in `EXEMPT`) | closed |
| T-54-11 | Repudiation (false assurance) | "current v0.10.0" banners mis-stating what the examples were checked against | low | mitigate | Zero `current v0.` hits in `docs/src`, `README.md`, `CONTRIBUTING.md` | closed |
| T-54-31 | Tampering (deployment drift) | an operator pinning a stale release's image from `docker.md` / `kubernetes.md` | low | mitigate | `docs/src/deployment/docker.md:91,105,515` use the version-free `X.Y.Z` placeholder; no `paladin-server:0.` literal remains in `docs/src/deployment` | closed |
| T-54-12 | Tampering (data loss) | an operator trusting "No functional change" and mixing v0.10/v0.11 readers | medium | mitigate | `crates/paladin-memory/CHANGELOG.md:8-12` and `crates/paladin-content/CHANGELOG.md:12` carry `## [Unreleased] / ### Fixed` corrections naming the one-way key change and `MIGRATION.md` 10.1 `M-B-12` | closed |
| T-54-13 | Elevation of privilege (overspend) | an operator sizing allowances believing Phase 42 closed the race | medium | mitigate | `.planning/decisions/0056-allowance-admission-model.md:13,65,77,128` cite ADR-0057 check-only | closed |
| T-54-14 | Repudiation (false assurance) | `node_cache.enabled: true` believed to cache on the shipped server | low | mitigate | `docs/src/user-guides/fault-tolerance.md:381` "Node cache on the shipped server", l.394 "Cache key version and rotation" | closed |
| T-54-15 | Information disclosure | literal API keys in a ConfigMap | high | mitigate | `k8s/server/configmap.yaml` deleted (`ls k8s/server/` shows only `deployment.yaml`, `secret.yaml.example`, `service.yaml`, `worker-deployment.yaml`); the only `ConfigMap` mentions under `k8s/` are comments explaining the Secret choice | closed |
| T-54-16 | Spoofing | an unfilled template loading placeholder text as a credential (`http.auth.api_keys[].key`) | high | mitigate | `k8s/server/secret.yaml.example:65` `key: ""` (refused as empty at boot); `${…}` keys refused by T-54-01. Provider-key placeholder (`openai-api-key: "REPLACE_ME"`, l.31) is out of this threat's scope and is AR-54-03 | closed |
| T-54-17 | Information disclosure | the provider key projected as a world-readable file | medium | mitigate | `k8s/server/deployment.yaml:39,98-101` and `worker-deployment.yaml:49,151-153`: `fsGroup: 65532`, `defaultMode: 0440`, `items: - key: config.yml` | closed |
| T-54-18 | Denial of service | the quick start applying the worker without its Secret, database and queue | low | mitigate | Three-file explicit apply (`http-service-host.md:170`); worker keeps its own documented apply in `k8s/README.md` | closed |
| T-54-19 | Tampering | breaking the CI kind e2e job's root manifests | low | mitigate | `k8s/deployment.yaml` changes only its two `version: v0.11.0` labels (l.8, l.23) | closed |
| T-54-20 | Repudiation (false assurance) | a gate that scans or matches nothing and passes | high | mitigate | `ZERO_FILES`, `CLASS_UNMATCHED`, `STALE_EXEMPTION`, `NO_VERSION` named failures in `scripts/check-version-strings.sh:254-264`; harness `tests/scripts/check-version-strings_test.sh` has 26 `assert_fire` cases including each class (l.209, 262, 267, 272); real-tree red run recorded in `54-FIRST-EVIDENCE.md` "Gate red run (SC3)" | closed |
| T-54-21 | Tampering | the gate rewriting files it scans | medium | mitigate | Check-only (D-12), no option parsing; `cksum` no-mutation assertions on fixture and real tree (`check-version-strings_test.sh:14,41,46,102`) | closed |
| T-54-22 | Elevation of privilege (bypass) | the exemption list growing to hide stale pins | low | mitigate | `EXEMPT` dict holds exactly one entry (`migration-guide.md`) with a reason | closed |
| T-54-23 | Tampering | `VERSION_STRINGS_ROOT` set in CI to point the gate elsewhere | low | accept | `ci.yml:182` step and `Makefile:226` target set no environment; variable documented test-only at `check-version-strings.sh:60`; see AR-54-02 | closed |
| T-54-24 | Tampering | footer deletion breaking section extraction or release notes | medium | mitigate | `tests/scripts/extract-changelog-section_test.sh`, `finalize-crate-changelogs_test.sh`, `check-release-consistency_test.sh` present; `make test-shell-guards` 85 assertions passed (54-VERIFICATION) | closed |
| T-54-25 | Spoofing | a changelog link sending readers to another repository's compare page | low | mitigate | No `[Unreleased]:` definition remains in `CHANGELOG.md` (deleted, not retargeted, D-17) | closed |
| T-54-26 | Tampering | a comment edit changing script behavior | low | mitigate | `make lint-shell` (shellcheck, `Makefile:395`) and the shell harnesses; diff filter recorded in 54-08-SUMMARY | closed |
| T-54-27 | Repudiation | WINDOWS row 62 closed without proof | medium | mitigate | `54-FIRST-EVIDENCE.md` "WINDOWS row 62 closing evidence" records re-run result lines; `.planning/WINDOWS.md:79` row 62 reads `fixed` | closed |
| T-54-28 | Tampering | the API snapshot hand-edited or padded with a `pub use` | medium | mitigate | `git diff` of `src/lib.rs` and `.project/current-exports.txt` is empty across the phase's commits; FIRST-08 wording reconciled under D-18 instead | closed |
| T-54-29 | Information disclosure | a real key captured into the evidence by a process-level run | medium | mitigate | Zero matches for `sk-[A-Za-z0-9]{20,}` in `54-FIRST-EVIDENCE.md`; only `sk-dummy-not-a-real-key` used (l.61, l.85) | closed |
| T-54-30 | Repudiation (false assurance) | a gate recorded as passing that was not run | medium | mitigate | `54-FIRST-EVIDENCE.md` "Phase-head gates" records each gate's own result line with the head SHA and labels CI-only jobs | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

**Totals:** 32 threats; 30 mitigate (all closed); 2 accept (all documented, closed); 0 open.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-54-01 | T-54-SC | The phase installs and adds no package; the RESEARCH Package Legitimacy Audit is empty by construction, so there is no supply-chain surface to verify. | plan 54-01 | 2026-11-01 |
| AR-54-02 | T-54-23 | `VERSION_STRINGS_ROOT` is a test-only seam. Neither the CI step (`ci.yml:182`) nor the Makefile target (`Makefile:226`) sets any environment, so a redirected scan root would require a reviewed workflow edit. | plan 54-07 (D-12) | 2026-11-01 |
| AR-54-03 | T-54-16 (adjacent; 54-REVIEW WR-02) | `k8s/server/secret.yaml.example:31` keeps `openai-api-key: "REPLACE_ME"`, a provider-key placeholder the server treats as a live bearer if the operator fills the HTTP keys but forgets the provider key. The HTTP-key `key: ""` entries still refuse boot, so a fully unfilled template fails closed; a half-filled one reaches the provider with `REPLACE_ME`, which the provider rejects. SC1 and FIRST-01 are scoped to `http.auth.api_keys[].key`. The maintainer reviewed the trade-off and passed it in `54-UAT.md` test 1. | maintainer (54-UAT.md test 1) | 2026-11-01 |

*Accepted risks do not resurface in future audit runs.*

---

## Review-Surfaced Residuals (not in the plan register)

`54-REVIEW.md` (gsd-code-reviewer, 1 critical / 5 warning / 4 info) and `54-VERIFICATION.md`
classed each of these as a documented residual, a carry-forward or an advisory follow-up. None
reopens a register threat and none introduces a new trust boundary. They are listed so the
security record is complete; the gate above does not count them.

| Ref | Relates to | Summary | Where it stands |
|-----|------------|---------|-----------------|
| CR-01 | T-54-20 | `make release` commits and tags without running the check-only gate, so a tag can be pushed before CI turns red on stale strings. CONTEXT D-12 chose check-only deliberately. | Carried to Phase 58 (bump PR must carry the string edits and run `check-version-strings.sh` before the tag); recorded in 54-VERIFICATION `deferred`. |
| WR-01 | T-54-08, T-54-10 | Copy-paste Cargo snippets name features that do not exist (`storage-sqlite`, `llm-openai`); pre-existing, the gate checks versions only. | Advisory follow-up for the next doc pass. |
| WR-02 | T-54-16 | Provider-key `REPLACE_ME` placeholder in the Secret template. | Accepted: AR-54-03 (54-UAT.md test 1 passed). |
| WR-03 | T-54-05 | Compose step 2 before step 1 makes Docker create a root-owned `config.yml/` directory and crash-loop. Availability on the operator's own host, not disclosure. | Maintainer verified with Docker and passed `54-UAT.md` test 3. |
| WR-04 | T-54-01, T-54-16 | Only one config is pinned by test; the Treasurer webhook secret has the same placeholder shape. | Known residual 4 in `54-FIRST-EVIDENCE.md`; webhook secret out of scope per CONTEXT Deferred Ideas. |
| WR-05 | T-54-20 | Python < 3.11 gives an unreadable traceback; the gate still fails closed (non-zero). | Advisory. |
| IN-02 | T-54-17 | Worker pods mount the full config including admin API keys they never use. | Advisory; a worker-trimmed Secret is optional hardening. |
| IN-01, IN-03, IN-04 | — | MIGRATION step ordering; gate parser edge shapes; duplicate `ci.yml` comment and `X.Y.Z` literal in copy-paste manifests. | Advisory; none affects a control above. |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-01 | 32 | 32 | 0 | /gsd-secure-phase 54 (orchestrator, L1 grep-depth; auditor short-circuited per ASVS L1 with a plan-authored register and threats_open: 0) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-01
