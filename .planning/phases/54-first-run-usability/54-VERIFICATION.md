---
phase: 54-first-run-usability
verified: 2026-11-01T02:30:00Z
status: human_needed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification: false
deferred:
  - truth: "make release pushes a tag on a tree the version-string gate would reject (REVIEW CR-01)"
    addressed_in: "Phase 58"
    evidence: "Phase 58 SC1: every user-facing version string reads 0.11.1 'because the gate demands it, not because a sweep remembered'; the Phase 54 scope note and 54-FIRST-EVIDENCE.md 'Carry-forward to Phase 58' item 2 record that the gate goes red at the bump and the bump commit must carry the 52 string edits. CONTEXT D-12 chose check-only on purpose."
human_verification:
  - test: "Decide WR-02: change k8s/server/secret.yaml.example `openai-api-key: \"REPLACE_ME\"` to an empty value (refused at boot) or accept it"
    expected: "A half-filled Secret (API keys pasted, provider key forgotten) fails at boot rather than starting healthy with REPLACE_ME as the provider bearer"
    why_human: "The goal text says 'no example loads a placeholder as a credential'. SC1 and FIRST-01 are scoped to http.auth.api_keys[].key, which is fixed. This provider-key placeholder is pre-existing, so it is a scope call for the maintainer."
  - test: "Apply k8s/server/secret.yaml (filled) plus both Deployments on a real cluster (kind) and read /ready"
    expected: "Pods read /etc/paladin/config.yml mounted from the Secret (defaultMode 0440, fsGroup 65532) and boot"
    why_human: "No kubectl, kind or Docker daemon in this container. The phase proved manifest shape only (54-FIRST-EVIDENCE.md Known residuals item 6)."
  - test: "Follow docker/docker-compose.server.yml header step 2 without step 1 (no config.yml yet)"
    expected: "Compose errors clearly, or the header recipe prevents the root-owned config.yml/ directory and restart loop (WR-03)"
    why_human: "Needs a Docker daemon, which is unavailable here."
---

# Phase 54: First-Run Usability Verification Report

**Phase Goal:** A new user who copies from the README, the docs, `config.example.yml` or `k8s/` gets a working first run: no example loads a placeholder as a credential, every install snippet and version line reads the current release, and a gate, not a one-off sweep, keeps it that way.
**Verified:** 2026-11-01
**Status:** human_needed
**Re-verification:** No, initial verification

I did not rely on SUMMARY.md. Every claim below was re-checked against the tree or by re-running the command. The working tree stayed clean (the stale-pin test restored `README.md`; `git status` is empty).

## Goal Achievement

### Observable Truths (ROADMAP success criteria, the contract)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A `${…}` `http.auth.api_keys[].key` exits at config load naming the key; both examples carry no loadable placeholder; the configmap header no longer claims expansion | VERIFIED | `AuthConfig::validate` (`src/config/agents.rs:218-223`) refuses any `key` containing `${`, with a name-only message. `cargo test --lib config::agents` gave 36 passed, including `shipped_example_http_auth_validates_and_configures_no_key` and `shipped_example_api_key_entries_uncommented_are_refused_as_empty`. `cargo test --features web-server --bin paladin-server placeholder` passed `build_auth_config_refuses_an_unexpanded_placeholder_key`. `config.example.yml:187` is `api_keys: []` with commented `key: ""` entries. `k8s/server/configmap.yaml` is deleted; the config moved into the Secret `k8s/server/secret.yaml.example` with `key: ""`. Process-level refusal on the real binary is in 54-FIRST-EVIDENCE.md, and the reviewer re-ran it independently. See WR-02 for the provider-key caveat. |
| 2 | Every install snippet and version line reads the workspace version (README `"0.5"` and `Current version`, sentinel, crate-map, feature-flags, k8s labels); migration-guide historical rows exempt | VERIFIED | `Cargo.toml` is 0.11.0. `README.md:185` reads `Current version: **0.11.0**`. A grep for non-`0.11.0` `paladin-*` pins across README, `docs/src` and `crates/*/README.md` hits only `migration-guide.md` (the exempt historical 0.4 rows). All `k8s/` `version:` labels are `v0.11.0` and the server images are `paladin-server:0.11.0`. `docs/src/deployment/*` tags are `X.Y.Z` placeholders (see IN-04). |
| 3 | `make check-gates` and CI fail when any user-facing version string lags; shown red by a deliberate stale string | VERIFIED | `Makefile:268` lists `check-version-strings` in `check-gates`, and `.github/workflows/ci.yml:182` runs the script. I ran the gate on the real tree: exit 0, 128 files scanned, all four classes non-zero (42 / 1 / 8 / 1). I then set `README.md:108` back to `"0.5"`, and the gate printed `README.md:108: found '0.5', want '0.11.0'` and exited 1. File restored. `make check-gates` exit 0. `make test-shell-guards` exit 0, 85 assertions passed, including the gate's harness. |
| 4 | memory and content `[0.11.1]` changelog entries correct "No functional change" with the `paladin.node` to `paladin.data` change; ADR-0056 and treasurer tests cite ADR-0057; "ten crates", `cargo-release` version and the dangling `[Unreleased]` link corrected | VERIFIED | Both crate changelogs carry `## [Unreleased]` with a `### Fixed` correction citing `MIGRATION.md` 10.1 M-B-12. The `[0.11.0]` text is untouched, and `finalize-crate-changelogs.sh` stamps the anchor as `[0.11.1]` at release. The correction is true: `crates/paladin-memory/src/citadel/file_citadel.rs` exists. `.planning/decisions/0056-allowance-admission-model.md` lines 13-14, 77 and 223-232 cite ADR-0057 check-only. `treasurer/tests.rs:17-19` cites ADR-0057 (already correct before the phase). `release.toml:48-50` records `cargo-release 1.1.7`. `finalize-crate-changelogs.sh` and the `ci.yml` comments are count-free or say "twelve". The `CHANGELOG.md` tail shows only the `[0.2.0]` and `[0.1.0]` definitions, with no `[Unreleased]:` footer. |
| 5 | `current-exports.txt` regenerated and carrying `UnsupportedParameter` / `check_reasoning`; node-cache guide says the server attaches no cache, documents the `k1` key version and the `key_prefix` rotation | VERIFIED (reconciled) | The guide, `fault-tolerance.md:381-406`, has "Node cache on the shipped server" and "Cache key version and rotation". It names `k1` (matching `cache_key.rs:69`) and the `key_prefix` rotation. The snapshot clause was reconciled in-phase under locked decision D-18, which anticipated it. I confirmed the premise: `grep -c 'UnsupportedParameter\|check_reasoning' .project/current-exports.txt` gives 0; `src/lib.rs` has no `pub use paladin_ports`; both symbols live in `crates/paladin-ports/src/output/llm_port.rs`. ROADMAP SC5 and REQUIREMENTS FIRST-08 carry a dated reconciliation note citing WINDOWS row 59 (waived) and `make check-semver`. The requirement as originally worded is not literally met, only reconciled. |

**Score:** 5/5 truths verified, 0 behavior-unverified. The behavior-dependent truths are SC1's boot refusal and SC3's red-on-stale. Both have a passing named test or a run I executed.

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| FIRST-01 | 54-01, 54-02, 54-03, 54-06 | SATISFIED (see WR-02) | Refusal in `validate`, examples fixed, Secret-delivered k8s config, Docker and README literal-key paths. |
| FIRST-02 | 54-03, 54-04, 54-07 | SATISFIED | Pins and `Current version` lines are at 0.11.0, and the gate holds them there. |
| FIRST-03 | 54-07 | SATISFIED | Gate wired into the Makefile and CI; red run observed. |
| FIRST-04 | 54-06, 54-07 | SATISFIED | k8s labels and image tags are at 0.11.0 and gate class 3 covers them. `docker-build-server` tags the workspace version. |
| FIRST-05 | 54-05 | SATISFIED | Both crate changelogs. |
| FIRST-06 | 54-05 | SATISFIED | ADR-0056 amended; tests doc comment already cited ADR-0057. |
| FIRST-07 | 54-07, 54-08 | SATISFIED | Counts, `cargo-release 1.1.7`, dangling link. |
| FIRST-08 | 54-09 | SATISFIED (reconciled) | Snapshot current (evidence: `make api-surface` unchanged, 4325 items). Wording reconciled as D-18 allowed. |
| FIRST-09 | 54-05 | SATISFIED | Guide sections above. |

All nine IDs are claimed by plan frontmatter, and all nine are `[x]` and "Complete" in REQUIREMENTS.md. No orphaned requirements.

### Required Artifacts and Key Links

| Artifact | Status | Details |
|----------|--------|---------|
| `scripts/check-version-strings.sh` | VERIFIED, wired | 306 lines. Makefile `check-gates` prerequisite plus the `ci.yml` step. Its harness `tests/scripts/check-version-strings_test.sh` is auto-discovered by `test-shell-guards`. |
| `AuthConfig::validate` `${` refusal | VERIFIED, wired | Reached from `build_auth_config` in `paladin-server.rs`, before the listener binds. |
| `k8s/server/secret.yaml.example` | VERIFIED, wired | Both Deployments mount it. The reviewer confirmed the embedded config parses. |
| `config.example.yml`, `docker-compose.server.yml`, `Dockerfile.server` | VERIFIED | Literal-key paths, no loadable placeholder. |

### Anti-Patterns

I found no unreferenced debt markers (TBD/FIXME/XXX) in the files this phase changed. The one match, `ci.yml:1698`, is pre-existing prose, not a marker.

## 54-REVIEW.md Findings Against the Goal

| Finding | Verdict | Reasoning |
|---------|---------|-----------|
| CR-01 `make release` vs check-only gate | Not a Phase 54 gap, carried to Phase 58. WARNING | CONTEXT D-12 deliberately chose check-only: "`make release` is unchanged ... Phase 58's bump PR edits them". The ROADMAP scope note says the same gate "forces the lockstep bump ... in Phase 58". `Makefile` lines 705-723 confirm `release` does `git add -u` and never calls the gate. The reviewer's concrete risk is that the tag is pushed before CI turns red. It is not recorded in the Phase 58 carry-forward, which says only that the bump commit must carry the edits. Add "run `check-version-strings.sh` before the tag" to Phase 58's plan. SC3 asks that the gate exist, be wired and go red, and it does. |
| WR-02 `openai-api-key: "REPLACE_ME"` | Goal-wording gap, partial. WARNING, human decision | I confirmed that `REPLACE_ME` is a non-empty provider-key placeholder that the server treats as a live key. It is pre-existing: the pre-phase file had the same line, plus two `REPLACE_ME` API keys that this phase did fix. The secret's header says "an unfilled template fails closed". That holds for a fully unfilled template, since the empty HTTP keys refuse boot. It does not hold for a half-filled one. The goal sentence says "no example loads a placeholder as a credential", so this one-line fix (`openai-api-key: ""`, plus the header wording) is worth making. SC1 and FIRST-01 name only `api_keys[].key`, so I did not mark the phase failed. |
| WR-01 wrong feature names (`storage-sqlite`, `llm-openai` on `paladin-llm`) | Advisory follow-up. WARNING | I confirmed both: the root `Cargo.toml` has no `storage-sqlite` feature, and `paladin-llm` features are `openai` and so on. Both lines are pre-existing (the diff shows only the version moved), so the phase did not introduce them. FIRST-02 is about versions. They would still break a pasted snippet, so fix them soon. |
| WR-03 compose bind-mount creates a `config.yml/` directory | Advisory follow-up. WARNING | New trap from this phase's compose change, hit only if step 1 of the header is skipped. Needs Docker to confirm. |
| WR-04 only one config pinned by test; Treasurer webhook secret not refused | Advisory. Both already recorded | Known residual 4 in the evidence, and CONTEXT Deferred Ideas keeps the webhook secret out of scope. The extra test for the Secret's embedded config is a sensible hardening. |
| WR-05 Python < 3.11 traceback | Advisory | The gate fails closed (non-zero), only the message is poor. |
| IN-01 to IN-04 | Advisory | Ordering in MIGRATION, worker pods mounting admin keys, gate parser edge cases, duplicate `ci.yml` comment, `X.Y.Z` tags. None blocks the goal. |

## Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Gate passes on real tree | `bash scripts/check-version-strings.sh` | exit 0, 128 files, four classes non-zero | PASS |
| Gate red on stale pin | edit `README.md:108` to `"0.5"`, run, restore | `README.md:108: found '0.5', want '0.11.0'`, exit 1 | PASS |
| Placeholder key refused | `cargo test --lib config::agents` | 36 passed | PASS |
| Boot path refuses before bind | `cargo test --features web-server --bin paladin-server placeholder` | 1 passed | PASS |
| All gates and harnesses | `make check-gates`; `make test-shell-guards` | exit 0; 85 assertions passed | PASS |

I did not re-run the full `cargo test`. The orchestrator reports 1479 passed and 2 failed (`run_api_wiring` trace tests). The phase's source diff touches only `src/config/agents.rs` and `src/bin/paladin-server.rs`, with nothing under `src/infrastructure`. The two failures therefore cannot come from this phase's code. I could not reproduce them with the lib filters I tried (0 tests matched), so I rely on the orchestrator's base-commit comparison. WINDOWS row 62 reads `fixed`.

## Human Verification Required

1. **WR-02 decision.** Change the Secret template's provider key to empty (refused at boot) or accept `REPLACE_ME`. This is the only item that touches the phase goal's wording.
2. **Cluster apply.** Apply the filled Secret and both Deployments on kind and check that the pods read the `0440` / `fsGroup 65532` mounted config. Not testable here.
3. **Compose first run.** Check the WR-03 failure mode (step 2 without step 1) with Docker.

## Gaps Summary

There are no blocking gaps. All five ROADMAP success criteria and all nine requirement IDs are met by evidence I re-ran. The gate is real, wired into `make check-gates` and CI, and observed red on a stale string. The status is `human_needed` rather than `passed` because of the three items above. WR-02 is the one I recommend closing before the phase is sealed: it is a one-line change, and without it the goal's "no example loads a placeholder as a credential" is true for HTTP keys only. CR-01 should be written into Phase 58's plan as a pre-tag gate step. WR-01 and WR-03 are cheap fixes for the next doc pass.

---

_Verified: 2026-11-01_
_Verifier: Claude (gsd-verifier)_
