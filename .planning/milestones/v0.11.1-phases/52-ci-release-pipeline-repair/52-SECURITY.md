---
phase: 52
slug: ci-release-pipeline-repair
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
created: 2026-10-30
---

# Phase 52 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register origin: authored at plan time. All eight PLAN files (52-01 through 52-08) carry a
`<threat_model>` block; all eight SUMMARY files report `## Threat Flags: None` (no new network
endpoint, auth path, schema or trust-boundary file access beyond the planned register). The
verification below is L1 grep-depth, which the ASVS level 1 configuration accepts as sufficient
when no threat at or above the `high` block threshold remains open.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| PR author -> ci.yml | A pull request's tree decides what the semver and allowlist gates see | Checked-out source tree; gate verdict |
| crates.io -> cargo-semver-checks | The `0.11.0` baseline crates are downloaded from the registry at run time | Published crate sources (public) |
| tool output -> script parser | `check-semver.sh` parses free text printed by a third-party tool | Tool stdout (`N checks:` line) |
| CHANGELOG.md text -> release payload | Commit-subject-derived text becomes the body of a token-authenticated API request | Changelog text (low sensitivity, attacker-influenceable via commit subjects) |
| release script -> GitHub API | `gh api` sends the runner's `GITHUB_TOKEN` (contents: write) with the payload | Release body; `GITHUB_TOKEN` (secret, never logged) |
| env vars -> size policy | `PALADIN_RELEASE_BODY_LIMIT` / `_RESERVE` steer truncation | Integer limits |
| maintainer paste -> evidence file | Alert data pasted by a human crosses into a committed file | Alert metadata; must contain no token-shaped strings |
| sandbox -> GitHub API | Unauthenticated reads; write attempts with an invalid sandbox token | Public alert / issue data |
| register -> alert store | `CODEQL-DISMISSALS.md` rows versus platform dismissals, reconciled by a manual command | Dismissal reasons (public) |
| live release body -> finalize | The body read back from GitHub (possibly hand-edited) is re-composed | Release body text |
| finalize -> GitHub API | `gh release edit` with the runner's `GITHUB_TOKEN` | Release body; `GITHUB_TOKEN` |
| manifest metadata -> cargo-semver-checks | Crate-wide allows decide which lints the tool evaluates | `[package.metadata.cargo-semver-checks]` tables |
| register -> gate | MIGRATION.md §11.2 and the allowlist decide which breaks are "deliberate" | crate\|type pairs |
| PR labels -> workflow conditions | A PR-controlled label set decides whether E2E runs | Label names (PR-controlled) |
| committed ruleset record -> live ruleset | The JSON in git versus the ruleset GitHub enforces | Required-check names |
| fork PRs -> CI | `pull_request` events (not `pull_request_target`) from forks | Untrusted PR tree; read-only token, no secrets |
| sandbox -> origin | A push from this environment is a write to the shared repository | Scratch branch commits |
| scratch branch -> main | A deliberately broken commit existed on origin for the duration of the red proof | Deliberate semver break (never merged) |
| maintainer reply -> committed record | Human-reported outcomes become phase evidence | Hand-off read-backs; redacted of token-shaped strings |
| executor -> repository settings | The executor must not cross into maintainer-only settings | None (commands printed, not run) |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-52-01 | Spoofing (assurance) | scripts/check-semver.sh verdict | high | mitigate | Zero-check tripwire: `reported no check count` (check-semver.sh:149) and `evaluated zero checks` (:154) are named failures; harness `check-semver_test.sh:269-279` pins `0 checks` fails and `1 checks` passes | closed |
| T-52-02 | Tampering | package coverage | medium | mitigate | Package list read from `scripts/publish-crates.sh` CRATES via the `PUBLISH_CRATES_LIB_ONLY=1` sourcing seam (check-semver.sh:194); empty/missing list is the named failure `CRATES_EMPTY`; no second hand-kept list | closed |
| T-52-03 | Tampering (supply chain) | cargo-semver-checks install | medium | mitigate | `taiki-e/install-action@v2` with `cargo-semver-checks@0.50.0` (ci.yml:414-416); no new action or tool introduced | closed |
| T-52-04 | Elevation of privilege | workflow `run:` blocks | medium | mitigate | YAML walk: no `${{ }}` expression in any `run:` of the `semver` or `cargo-deny` jobs (all 22 steps clean); the only `${{ }}` in `run:` workspace-wide are pre-existing `matrix.*` / `steps.*` uses in `crate-isolation` and `docker`, none event-data | closed |
| T-52-05 | Repudiation | allowlist gate placement | high | mitigate | Step `Check semver allowlist is set-equal to the MIGRATION.md register` lives in the `cargo-deny` job (`License & Dependency Policy`, a required check) with `if: None`; YAML assertion confirms | closed |
| T-52-06 | Tampering | `_cor_build_payload` | high | mitigate | Body travels file -> `jq -n --rawfile body` (create-or-reuse-release.sh:127) -> stdin -> `gh api --input -` (:94); never interpolated into a command; harness cases 8 and 10 keep it data | closed |
| T-52-07 | Denial of service | release create on a long changelog | high | mitigate | argv path removed (131,072-byte kernel cap documented at :26); over-limit bodies fitted by `release-body-fit.sh` before create (cases 11-12) | closed |
| T-52-08 | Repudiation | silent truncation | medium | mitigate | stderr size log, `::warning` annotation under Actions (:49, :230), sentinel footer naming CHANGELOG.md at the tag; harness case 11 asserts prefix-at-line-boundary + sentinel footer | closed |
| T-52-09 | Information disclosure | size-guard logging | medium | mitigate | Helper logs sizes only, "never the body text" (:48-49); no header or token echoed; payload fed on stdin | closed |
| T-52-10 | Tampering | limit env vars | low | mitigate | `release-body-fit.sh:78,90`: non-integer, zero, negative or inconsistent LIMIT/RESERVE are named failures before any API call (case 14) | closed |
| T-52-11 | Repudiation | CODEQL-DISMISSALS.md rows | high | mitigate | Register carries 37 governed rows; `check-codeql-dismissals.sh` (declared-count vs payload drift guard) exits 0 locally; H-04 done: all 37 PATCH calls dismissed, register vs platform `diff` empty, 0 CodeQL alerts open | closed |
| T-52-12 | Information disclosure | evidence file | medium | mitigate | Negative grep for `gh[pousr]_…` / `github_pat_` in 52-03 Task 1 verify; only hits in the phase directory are the grep patterns themselves and the "found none" records; raw API JSON kept in scratchpad | closed |
| T-52-13 | Spoofing (assurance) | CodeQL posture | medium | mitigate | `codeql.yml` last touched in phase 18 (`cc748d97e`), no phase-52 change; no `snyk` string in any workflow; security.instructions.md still states CodeQL is advisory-only | closed |
| T-52-14 | Tampering | issue #41 comment | low | mitigate | 52-CARRYOVER-EVIDENCE.md records each fact re-read from the public API with source URL and time; comment saved verbatim before posting (52-03 SUMMARY) | closed |
| T-52-15 | Elevation of privilege | write attempts | low | accept | See Accepted Risks Log AR-52-02 | closed |
| T-52-16 | Denial of service | `Finalize Release Body` job on a tag | high | mitigate | `finalize-release-body.sh` re-fits the composed body under the limit before `gh release edit` (:77-89); artifact-only overflow is a named failure (`the release artifact sections alone…`, :89) | closed |
| T-52-17 | Repudiation | artifact inventory | medium | mitigate | Only the curated section is truncated (:82); artifact tail asserted byte-identical in `finalize-release-body_test.sh` (cases 6-8, 26) | closed |
| T-52-18 | Tampering | footer duplication on re-run | low | mitigate | Helper strips an existing footer before re-fitting (:86-87); two-run byte-identity asserted (`finalize-release-body_test.sh:314,389,419`) | closed |
| T-52-19 | Spoofing (assurance) | crate-wide allows | high | mitigate | No `[package.metadata.cargo-semver-checks]` table remains in any crate manifest (grep empty, 28 lines removed); crate-wide-allow mirror in the gate fails any future table lacking an allowlist entry | closed |
| T-52-20 | Tampering | register window | high | mitigate | `check-migration-allowlist.sh:70-79` scopes to §11.2 and requires the `## 11.3 ` terminator; MIGRATION.md:24 carries it; harness cases 12a-12f pin frozen §10.2/§9.2 and out-of-window rows | closed |
| T-52-21 | Repudiation | frozen §9/§10 records | medium | mitigate | Byte-identity of MIGRATION.md from `# 10.` (line 48) to EOF asserted in 52-05 Task 2 verify; CI step `Verify MIGRATION.md carries no unfilled placeholder marker` is green on the final head (H-08) | closed |
| T-52-22 | Denial of service | local measurement disk use | low | mitigate | `SEMVER_PRUNE_LOCAL_CACHE=1` deletes only `target/semver-checks/local-*` (check-semver.sh:70,102-105) | closed |
| T-52-23 | Tampering (injection) | e2e-tests `if:` | high | mitigate | Label used only inside `contains(github.event.pull_request.labels.*.name, 'release')` in the job-level `if:`; YAML walk finds no `run:` referencing `pull_request.labels` | closed |
| T-52-24 | Elevation of privilege | `labeled` trigger | medium | mitigate | Trigger is `pull_request` with types `[opened, synchronize, reopened, labeled]`; `pull_request_target` absent from ci.yml; e2e job permissions unchanged (none declared) | closed |
| T-52-25 | Repudiation | ruleset record vs live | medium | mitigate | `.github/rulesets/protect-main-branch.json` committed; H-01 done: maintainer PUT at 2026-10-29T19:33:35Z, read-backs 3 rulesets / 46 contexts, live context set identical to the record (sorted `diff` empty) | closed |
| T-52-26 | Denial of service | `labeled` fan-out | low | accept | See Accepted Risks Log AR-52-03 | closed |
| T-52-27 | Spoofing (assurance) | required-but-skipped E2E | medium | mitigate | `release` label documented as a pre-review step in `docs/src/appendix/release-checklist.md` and `release-automation.md`; H-02 done: labelled PR #61 run 40932320347 ran `End-to-End Tests` to success | closed |
| T-52-28 | Tampering | scratch break | high | mitigate | Break lived only on `scratch/52-semver-red-proof`; ancestry check exit 1 and subject grep empty on main (52-07 SUMMARY); H-03 done: remote branch deleted, `git ls-remote` empty; no `scratch/*` remote branch after `fetch --prune` | closed |
| T-52-29 | Spoofing (assurance) | red proof | high | mitigate | 52-SEMVER-RED-EVIDENCE.md: primary target detected (conditions 3 and 4 hold), item not `#[doc(hidden)]`, visible in rustdoc JSON under `--default-features`; fallback not needed | closed |
| T-52-30 | Elevation of privilege | push / PR creation | medium | mitigate | 52-07 Task 2 was a blocking `checkpoint:decision`; maintainer reply recorded verbatim before any push; only sandbox credentials used once; refusals became hand-offs | closed |
| T-52-31 | Denial of service | CI minutes | low | accept | See Accepted Risks Log AR-52-04 | closed |
| T-52-32 | Repudiation | hand-off items | high | mitigate | 52-HANDOVER.md status block: H-01 applied, H-02 observed, H-03 done, H-04 done, H-05 ack, H-06 deferred (owner: maintainer, resolves Phase 56), H-07 ack, H-08 green; 8 of 8 carry a resolved status with a read-back, 0 open | closed |
| T-52-33 | Elevation of privilege | ruleset / labels / dismissals | high | mitigate | 52-08 PLAN Task 2 action: "Perform none of them from the executor"; HANDOVER.md:11 records "performed no ruleset write, no label"; H-01/H-02/H-04 read-backs show the maintainer ran them | closed |
| T-52-34 | Spoofing (assurance) | coverage reproduction claim | medium | mitigate | 52-08 SUMMARY: H-06 deferred, no local coverage figure claimed, todo stays pending (`resolves_phase: 56`), no gate or threshold changed | closed |
| T-52-35 | Information disclosure | recorded maintainer reply | medium | mitigate | HANDOVER.md:358 and 52-08 SUMMARY:129: token-shaped-string scan (`ghp_`, `github_pat_`, `AKIA`, `sk-`, bearer) run before recording, found none | closed |
| T-52-SC | Tampering | npm/pip/cargo installs | low | accept | See Accepted Risks Log AR-52-01 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-52-01 | T-52-SC | Plan 52-01 installs no new package; the existing `cargo-semver-checks@0.50.0` pin is unchanged, so no slopcheck checkpoint applies | Plan 52-01 threat model; confirmed by secure-phase | 2026-10-30 |
| AR-52-02 | T-52-15 | Only the sandbox's own token is tried, once; a refusal becomes a maintainer hand-off (H-01/H-04); no credential is sought or stored | Plan 52-03 threat model; confirmed by secure-phase | 2026-10-30 |
| AR-52-03 | T-52-26 | Any `labeled` event re-runs the full pipeline (about 63 min Docker build); single-maintainer repository, rare label events, `cancel-in-progress` on non-main refs (PD-06-3) | Plan 52-06 threat model; confirmed by secure-phase | 2026-10-30 |
| AR-52-04 | T-52-31 | One scratch push ran the whole pipeline once for the phase's central red proof; the branch is deleted (H-03) so the cost cannot recur | Plan 52-07 threat model; confirmed by secure-phase | 2026-10-30 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-30 | 36 | 36 | 0 | gsd-secure-phase orchestrator (L1 grep-depth; short-circuit: plan-time register, ASVS 1, threats_open 0) |

Notes:
- 36 register rows = 35 numbered threats (T-52-01..T-52-35) plus T-52-SC.
- Closed = 32 mitigated with implementation evidence + 4 accepted (AR-52-01..04).
- No `## Threat Flags` entry in any SUMMARY added unregistered attack surface.
- Pre-existing, out-of-scope finding carried forward (not a phase 52 threat): D-52-03-01 in
  `deferred-items.md`, recorded by 52-03 SUMMARY as predating this phase.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-30
