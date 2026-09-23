---
phase: 29
slug: program-gates-release
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-09-10
---

# Phase 29 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time — all nine `29-0N-PLAN.md` files carry a `<threat_model>` block (37 threats). Every `29-0N-SUMMARY.md` that has a `## Threat Flags` section reports "None" (no new attack surface: the phase is release gating, tests, CI configuration and documentation over already-reviewed code). Verification depth: ASVS L1 grep-depth, per the short-circuit rule (`threats_open: 0`, `register_authored_at_plan_time: true`, `asvs_level: 1`); the CI allowlist gate (T-29-03-01/03) was additionally executed locally from the `ci.yml` step body rather than only grepped.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| v0.9 operator config file → v0.10 composition root | An upgrade must not silently widen the exposed HTTP surface | Operator config; route enablement |
| CI runner environment → boot-test config resolution | Ambient `APP_*` vars must not change the boot test's verdict | Environment variables |
| v0.9 API client → v0.10 HTTP surface | Pre-existing paths and schemas must be unchanged for un-opted-in clients | OpenAPI document; auth schemes |
| Frozen baseline fixtures → test verdicts | A HEAD-sourced fixture makes the compat checks tautological | Config and OpenAPI blobs with recorded SHAs |
| `MIGRATION.md` §9.2 register → semver-checks allowlist | An unregistered suppression silences a real public-API break | crate\|type pairs |
| `make publish-dry-run` exit status → release decision | A target that cannot fail proves nothing | Exit codes; crate count |
| Prior VERIFICATION/VALIDATION claims → acceptance audit verdicts | A restated claim must not be laundered into a release gate | Audit evidence (commands, counts, SHAs) |
| Audit finding → production code | X-03 forbids closing a finding by changing behavior in a release-gate phase | `.rs` source |
| `MIGRATION.md` upgrade instructions / published Upgrading page → operator cluster | Nonexistent commands or a missing grace-period step break a real upgrade | Operational checklist |
| Agent verdict → maintainer sign-off | Judgment-tier safety/privacy items must be ticked by a human | Sign-off checkboxes |
| Defect register → `/gsd-ship` gate | Emptying the register defeats the gate | WINDOWS.md rows and counts |
| Working tree version → crates.io; branch → release tag | Published versions are irreversible; tags must come from `main` | Version strings; git tag |
| Recorded CI evidence → human merge decision | An unrun gate reported green is worse than a blank record | Run identifiers and job names |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-29-01-01 | Information disclosure | v0.10 platform surface after a v0.9→v0.10 upgrade | high | mitigate | `tests/integration/v0_9_config_boot_test.rs` asserts `501` (never `404`) for `/v1/runs`, `/v1/threads/*/history`, `/v1/assistants`, `/v1/schedules` from the frozen v0.9 config (10 NOT_IMPLEMENTED refs, 4-family table at lines 482-485); run in CI at `ci.yml:1295` | closed |
| T-29-01-02 | Tampering | Boot test verdict under ambient `APP_*` env | medium | mitigate | `apply_env_overrides_is_a_no_op_with_no_app_vars` (`v0_9_config_boot_test.rs:341-343`) is `#[serial]`, clears 26 `APP_*` vars via `remove_var` and restores them (lines 350, 430-431) | closed |
| T-29-01-03 | Repudiation | Frozen config fixtures' provenance | medium | mitigate | `tests/fixtures/config/README.md` records tag commit `8b9bef89…` and both blob SHAs with the `git hash-object` re-derivation command | closed |
| T-29-01-04 | Spoofing | `AgentAuthConfig::default()` in test composition | low | accept | Auth unchanged by this phase (X-03); covered by `thread_routes_share_the_agent_auth_middleware` (`src/bin/paladin-server.rs:1013`). See Accepted Risks R-29-01. | closed |
| T-29-02-01 | Tampering | `openapi-v0.9.0.json` baseline provenance | high | mitigate | `crates/paladin-web/tests/fixtures/README.md` records tag `v0.9.0`, commit `8b9bef89…`, and the `git show v0.9.0:… \| git hash-object --stdin` re-derivation | closed |
| T-29-02-02 | Information disclosure | Schema reachable only via a pre-existing path changing | medium | mitigate | `openapi_golden_v0_9.rs` compares the transitive `$ref` closure, not just operation objects | closed |
| T-29-02-03 | Spoofing | `components.securitySchemes` renamed or weakened | high | mitigate | `openapi_golden_v0_9.rs:309-320` deep-compares `securitySchemes` in full, unrestricted | closed |
| T-29-02-04 | Repudiation | Vacuous pass from an empty path restriction | medium | mitigate | `v0_9_path_restriction_is_non_empty` (line 224) and `ref_closure_is_non_empty_and_fully_resolved` (line 289) | closed |
| T-29-03-01 | Tampering | Allowlist entry with no §9.2 row | high | mitigate | `ci.yml:377-450` builds both `crate \| type` sets with `sort -u` and `diff -u`, exit 1 on mismatch; executed locally from the step body: 9 = 9 pairs, set-equal, exit 0 | closed |
| T-29-03-02 | Repudiation | `make publish-dry-run` succeeding while sub-commands fail | high | mitigate | Target body is a single `cargo publish --workspace --dry-run`; zero `\|\| true` in the target | closed |
| T-29-03-03 | Denial of service | False CI failure from a row-count comparison | medium | mitigate | Pair-SET comparison; marker scan iterates `k = 6..NF` (handles the embedded-pipe row); local run shows a clean diff at HEAD | closed |
| T-29-03-04 | Tampering | Version-bump edit rewriting the semver baseline pins | high | mitigate | `--baseline-version 0.9.0` pins in `ci.yml` (lines 276, 334, 357) untouched by every Phase 29 commit; `git log -S` shows last change at Phase 28 (`3b6b0a3e`). Note: the plan asserted a literal count of 5; the measured literal count is 3 and has been 3 throughout — a plan-text precision finding recorded in `29-03-SUMMARY.md` and `29-09-SUMMARY.md`, not a pin change | closed |
| T-29-04-01 | Repudiation | `PASS` verdict without re-run evidence | high | mitigate | `.project/v0.10.0/09-program-acceptance-audit.md` records commands and observed output per section; `29-VERIFICATION.md` independently re-ran the E2E targets, eval harness and BUG-01 grep | closed |
| T-29-04-02 | Tampering | Production behavior change to close an audit finding | high | mitigate | `git log --diff-filter=AM -- '*.rs'` over every `(29)`, `29-04` and `29-07` commit returns no entries; `29-VERIFICATION.md` confirms zero non-test `.rs` changes across Phase 29 | closed |
| T-29-04-03 | Repudiation | Finding silently omitted | medium | mitigate | 10 numbered sections, 10 `Findings:` headings, `- none` markers where empty | closed |
| T-29-04-04 | Information disclosure | Audit quoting credentials or response bodies | low | mitigate | Credential-shaped pattern grep over the audit returns 0; evidence is commands, counts, SHAs and test names | closed |
| T-29-05-01 | Repudiation | §9.5/§9.6 compat claims with no test behind them | high | mitigate | `MIGRATION.md:321,436,582,662` cite `v0_9_config_boot` and `openapi_golden_v0_9` by path; boot test runs in CI (`ci.yml:1295`) | closed |
| T-29-05-02 | Denial of service | §9.8 naming a nonexistent CLI command | high | mitigate | `grep -c 'paladin-cli health\|graph validate' MIGRATION.md` → 0 | closed |
| T-29-05-03 | Denial of service | Deploying without raising `terminationGracePeriodSeconds` | high | mitigate | `MIGRATION.md:19,75,100` name the value (60), `APP_ENGINE_SHUTDOWN_GRACE_SECS`, and both `k8s/` manifests | closed |
| T-29-05-04 | Tampering | Future edit reintroducing a placeholder | medium | mitigate | Read-only grep gate in the `semver` job (`ci.yml:362-377` comments record why) | closed |
| T-29-05-05 | Repudiation | CI boot step green having selected zero tests | high | mitigate | Guard step `ci.yml:1301-1313` fails when passed count < 9 (hardened against zero-match grep by WR-03, `59e4e5f8`) | closed |
| T-29-06-01 | Denial of service | Docs build failing on a relative repo-root link | high | mitigate | `docs/src/api-reference/upgrading.md` has 0 `{{#include}}`, links `MIGRATION.md` by repository URL (line 9); `mdbook build docs/` green per `29-CI-EVIDENCE.md` row 15 | closed |
| T-29-06-02 | Repudiation | Upgrading checklist drifting from §9.8 | medium | mitigate | Page names `terminationGracePeriodSeconds: 60` (lines 22, 46, 49) and the real `paladin-cli setup-check` subcommand (line 58) | closed |
| T-29-06-03 | Tampering | Errata growing into a §4 restructuring | medium | mitigate | `ada9a6f4` touches `00-program-overview.md` with 2 insertions, 0 deletions; §4 table still 12 term rows; errata is a footnote below the table | closed |
| T-29-06-04 | Information disclosure | Upgrading page publishing a credential-shaped example | low | mitigate | Page carries env var names and file paths only; no key values | closed |
| T-29-07-01 | Tampering | Public-API change with no §9.2 row and no allowlist entry | high | mitigate | Audit §6 reconciles both directions; CI gate (T-29-03-01) makes it durable; `cargo semver-checks` 11/11 clean per `29-CI-EVIDENCE.md` row 10 | closed |
| T-29-07-02 | Repudiation | Agent ticking a judgment-tier prohibition | high | mitigate | "Maintainer sign-off" section authored with all 7 boxes unchecked (0 `- [x]` on disk) and an explicit "this audit does not, and must not, tick them itself" rule; UAT Test 1 routes ticking to a human | closed |
| T-29-07-03 | Repudiation | `lint` job recorded green while its doc step is red | medium | mitigate | Audit §8 records the re-measured `warning:` count and the exact `cargo doc` command (lines 75-79); tracked, not papered over | closed |
| T-29-07-04 | Information disclosure | Credential quoted alongside audit/deny output | low | mitigate | §8 records advisory identifiers and counts only; credential-shaped grep → 0 | closed |
| T-29-07-05 | Denial of service | Tracing overhead accepted without recording bounded exposure | medium | accept | Recorded in the audit, `docs/src/operations/observability.md`, `CHANGELOG.md` [0.10.0] Known limitations, and WINDOWS.md row 35; sinks opt-in, `trace.state_values` defaults off. Reaffirmed at Phase 29 UAT Test 3 (2026-09-10). See Accepted Risks R-29-02. | closed |
| T-29-08-01 | Repudiation | Row waived generically or deleted to reach `open_count` 0 | high | mitigate | WINDOWS.md: 35 numbered rows = `total_count: 35`; 0 waived rows with an empty reason; each waiver cites a specific record | closed |
| T-29-08-02 | Tampering | Direct edits drifting counts from rows | high | mitigate | Frontmatter `open 0 + waived 26 + fixed 9 = total 35` reconciles with the row count; transitions via `gsd-tools windows` | closed |
| T-29-08-03 | Repudiation | Docker-gated row marked fixed against a run lacking the job | high | mitigate | Rows cite job name alongside run identifier `34365812871`; evidence file lists all 33 jobs of that run | closed |
| T-29-08-04 | Repudiation | Coverage row closed with a local figure | medium | mitigate | Row 27 closed against the canonical `coverage` job (run `34365812871`, job `102514106872`, 90.28%) | closed |
| T-29-08-05 | Tampering | Waiving a deviation the audit found to be an unmet FR | high | mitigate | Row 35 (D-16 tracing overhead) waived with the deviation's full citation, matching the audit's own recorded disposition, not a silent absorption | closed |
| T-29-09-01 | Tampering | `ci.yml` baseline pins rewritten during the bump | high | mitigate | Same evidence as T-29-03-04: pins at lines 276, 334, 357 untouched; `git diff --stat 5e0c979a..HEAD -- .github/workflows/ci.yml` empty for the bump plan | closed |
| T-29-09-02 | Repudiation | Evidence record reporting an unrunnable gate as passing | high | mitigate | `29-CI-EVIDENCE.md` records `docs.yml` as **not run** and owed to the PR, Docker/Java suites as read from the cited run rather than claimed local, with real run identifiers | closed |
| T-29-09-03 | Repudiation | Dry-run publish that packaged nothing reading green | high | mitigate | Evidence row 14: twelve crates in dependency order (`paladin-ai-core` first, `paladin-ai` last), `paladin-doc-examples` correctly absent, `grep -c Uploading` = 12 | closed |
| T-29-09-04 | Denial of service | `create-release` failing on a missing changelog heading | medium | mitigate | `## [0.10.0] - 2026-09-10` present in root and all 11 crate changelogs (WR-01 populated the empty per-crate sections); `check-release-consistency.sh --tag v0.10.0` OK | closed |
| T-29-09-05 | Tampering | Tag cut from this branch instead of `main` | high | mitigate | `git tag -l v0.10.0` is empty; `release.yml` `verify-tag-source` job (line 29) enforces `git merge-base --is-ancestor "$SHA" origin/main` (line 76) | closed |
| T-29-09-06 | Tampering | New dependency entering under cover of the bump | medium | mitigate | `29-09-SUMMARY.md` records the bump diff scope (version fields, pins, lockfile, `openapi.json` `info.version`); `make security` (cargo-deny + cargo-audit) exit 0 per evidence row 12 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-29-01 | T-29-01-04 | The boot test asserts route registration and `501` status, not authorization. Auth behavior is unchanged by Phase 29 (X-03) and remains covered by the pre-existing `thread_routes_share_the_agent_auth_middleware` test. Low severity. | Plan 29-01 (plan-time disposition) | 2026-09-10 |
| R-29-02 | T-29-07-05 | Tracing overhead (+22.18% log-sink / +18.46% composite vs the ≤3% PRD 07 acceptance-6 bar) is accepted for v0.10.0 as documented deviation D-16: sinks are opt-in and `trace.state_values` defaults off, so a default deployment does not pay it. Cross-referenced in the acceptance audit, observability docs, CHANGELOG Known limitations, and WINDOWS.md row 35. Maintainer reaffirmed at Phase 29 UAT Test 3. | Maintainer (STATE.md D-37, reaffirmed 2026-09-10) | 2026-09-10 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-10 | 37 | 37 | 0 | /gsd-secure-phase 29 (orchestrator, L1 short-circuit; CI allowlist gate executed locally) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-10
