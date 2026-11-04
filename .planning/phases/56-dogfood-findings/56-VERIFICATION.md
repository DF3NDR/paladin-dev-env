---
phase: 56-dogfood-findings
verified: 2026-11-04T12:00:00Z
status: human_needed
score: 11/11 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification: false
gaps: []
deferred: []
behavior_unverified_items: []
human_verification:
  - test: "DF-18 severity call: confirm S3 / out (re-homed) is acceptable for 'The Quickstart `.model(\"gpt-4\")` fails against a real OpenAI key'"
    expected: "Maintainer either accepts S3 (docs literal, clear provider error, 109-site sweep re-homed to .planning/todos/pending/2026-11-03-docs-deprecated-gpt-4-model-literal.md) or regrades to S2 and the one-line Quickstart fix (docs/src/getting-started/quickstart.md:53) lands before the tag"
    why_human: "Severity is a judgement (D-04). The first program a new user copies exits 1 on a working key, and v0.11.1 will ship that Quickstart line unchanged. The triager's reason (fixing one of 109 leaves the root cause) is recorded, but whether that outweighs a broken first run is the maintainer's call."
  - test: "DOGF-05 scope: confirm a devcontainer walk (no Docker socket, compose siblings) satisfies 'on a Docker-capable machine'"
    expected: "Maintainer accepts the D-17 pass (local 92.79 percent, CI 92.86 percent, both truncate to 92, both >= 82), knowing guide step 1 (`make services-up`) and the bare-host password-protected Redis path (trap 2) were NOT exercised"
    why_human: "The measured figure is relayed maintainer evidence, not reproducible here, and the walk's own SUMMARY says the bare-host path is still unwalked. The todo was closed under the D-17 pass rule, which is met as written."
  - test: "Judgment-tier prohibitions P1, P4, P11 (and test-tier P3): record explicit human resolution"
    expected: "P1 (no simulated web3sec return), P4 (no web3sec disposition without evidence) and P11 (agent did not pick the D-16 option) are each read and resolved by the maintainer; P3 (log append-only) gets a standing enforcement test or an accepted waiver"
    why_human: "ADR-550 D4: judgment-tier verdicts here are non-authoritative LLM-judge verdicts, flagged `unverified-prohibition - human review recommended`. P3 is test-tier with no wired enforcement, so it fails closed to flagged even though measured evidence shows it held (see Prohibitions)."
  - test: "Relayed maintainer evidence: keyed first-run walk (K1..K9) and DF-19 Anthropic re-check"
    expected: "Maintainer confirms K2/K6/K7/K8 output is theirs and that `setup-check --verbose` read `Anthropic` ok after the DF-19 fix on 2026-11-03"
    why_human: "Real provider keys; the agent and verifier cannot run them. K1 (environment block) was recorded as 'not captured', derived from K6/K7."
---

# Phase 56: Dogfood Findings Verification Report

**Phase Goal:** The defects v0.11.0 actually shows when used for real, in the maintainer's own runs, on the published `0.11.0` crates and in the web3sec consumer's return, are found, logged, dispositioned and, where S1 or S2, fixed before the tag.
**Verified:** 2026-11-04
**Status:** human_needed (no gaps; 4 human-decision items, none a blocker)
**Re-verification:** No, initial verification (no prior VERIFICATION.md). The 14 plans include Stage C gap-closure plans 56-11..14, written from the findings log rather than from a prior verification.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: every `DF-nn` in the log, incl. DF-01..04, carries a final disposition; none reads `open` | VERIFIED | `./scripts/check-findings-log.sh` run by me: `OK ... parsed 36 entries`. My own independent parse (last `**Disposition...:**` bullet per `### DF-nn`) gives 36 entries, 0 open: 24 `in v0.11.1`, 10 `out (...)`, 2 `duplicate of DF-02`. Heading count (`grep -c '^### DF-'` = 37) includes the fenced format template. `tests/scripts/check-findings-log_test.sh`: 43 assertions pass, including a real-log parse and no-mutation check. 7 re-homed `out` entries each point at a todo; all 6 todo files exist in `.planning/todos/pending/` (DF-20, 18, 16, 6, 34, 36). |
| 2 | SC2: web3sec return (SHA, D-2 choice, gate totals, divergences) triaged, one entry per divergence | VERIFIED | `56-WEB3SEC-RETURN-REPORT-50.md` is a substantive consumer document (C1 `3ad5d03f`, pin `70657919...`, lossless `TokenUsage` D-2, 2,821/0/2 vs re-measured 2,766/0/2). Every section 5 and 7 item maps to DF-27..DF-36 (5.1+7.1 -> DF-27, 5.2+7.3 -> DF-28, 5.3 -> 29, 5.4 -> 30, 5.5 -> 31, 5.6+7.2 -> 32, 7.4 -> 33, 7.5 -> 34, 7.6 -> 35, section 4 -> 36). Re-checked: `grep -c 'build_cadence\|with_cadence\|CadenceConfig' tests/consumer_surface_muster.rs` = 0 (DF-36 premise true); DF-27/DF-28 doc notes present (`switching-providers.md:178`, `upgrading.md:100`). SHA equals the roadmap pin. Gate total 2,821 exceeds every recorded figure including 2,683, so no regression entry is owed. D-16 `confirm-triage` recorded as the maintainer's, in `56-10-SUMMARY.md` `## Decision`; todo moved to `completed/`; DOGF-02 `[x]`. |
| 3 | SC3: each `config.example.yml` section round-trips through the loader with every optional key present, as a passing test; rejected/ignored keys logged | VERIFIED | I ran `cargo test -p paladin-ai --features cli,web-server --test config_example_roundtrip`: 14 passed, 0 failed. Covers `Settings::load_from_file` and `Settings::new`, the full fixture (`tests/fixtures/config/config.example.full.yml`), the Redis-cadence fixture, a drift guard, and non-vacuity probes (`walker_reports_a_planted_typo_key_through_the_loader`, `exempt_and_known_gap_rows_go_stale_loudly`). Findings logged: DF-05 (omitted `steps`, fixed in the example), DF-06 (19-digit price, re-homed). `KNOWN_GAPS` is empty and `EXEMPT` holds two documented `skip_serializing` secrets, which matches "zero unexplained keys". |
| 4a | SC4a: first-run path walked on the published `0.11.0` crates, every stumble logged | VERIFIED | `56-FIRST-RUN-WALK.md`: scratch crate outside any repo, no `path =`/`[patch]`, `cargo metadata` shows `paladin-ai 0.11.0 registry+crates.io`, published README byte-diffed against the tag. Stumbles became DF-10..DF-17. Keyed half (`56-RUNBOOK-KEYED-WALK.md` K2, K6, K7, K8) is filled with real output (the Quickstart run exits 1 on deprecated `gpt-4`, then 0 with `gpt-4o-mini`), giving DF-18 and DF-19. Secret scan of phase, todo and log files for key-shaped strings: only the fake `Bearer test-key-not-a-secret-df08-otel` sentinel. Relayed human evidence, see Human Verification item 4. |
| 4b | SC4b: maintainer's local `make coverage` walk closes todo `2026-08-13-verify-local-coverage-reproduction` with the measured figure | VERIFIED (scope caveat) | Todo is in `.planning/todos/completed/` with `## Resolution (2026-11-03)`: local `170650/183901 = 92.79%`, CI run `41703614636` `170767/183901 = 92.86%`, both truncate to 92, both >= 82, tool pin 0.8.7 both sides. Walk found DF-20..DF-25; DF-21/22/23/24/25 fixed in docs/Makefile, DF-20 re-homed. Caveat: ran in the devcontainer (no Docker socket), so step 1 and the bare-host path were not exercised (Human Verification item 2). |
| 5 | SC5: every S1/S2 promoted `in v0.11.1` is fixed and verified before close | VERIFIED | Seven S1/S2 entries, none open: DF-01 (S2), DF-03 (S2), DF-04 (S1), DF-07 (S1), DF-08 (S2), DF-19 (S2), DF-26 (S2). Phase 53/54/55 owners cited for DF-01/03/04. Stage C fixes verified below (all 7 fix commits exist as commits and each message cites its `DF-nn`). I re-ran the named regression tests on HEAD: 19 lib tests (redaction, `anthropic_probe_*`, `section_wrappers_*`), 11 `paladin-server` `load_settings_*`, 2 `paladin` `settings_summary_*`: all pass. |
| 6 | D-01 / P3: log is append-only; DF-01..04 original lines byte-intact | VERIFIED | `git diff 8e02a457e HEAD -- .project/v0.11.1/02-dogfood-findings-log.md \| grep -E '^-[^-]'` prints nothing (zero deleted or changed lines since the seed commit). DF-01..04 keep `open -> recommend` lines with dated disposition bullets appended. |
| 7 | D-00c: frozen records untouched | VERIFIED | `git diff 3f5cc1e63 HEAD` on `MIGRATION.md` (additions only: M-B-36..38 rows in 11.1, 11.5 bullet, 11.8 steps 8-10, none in 9/10), root `CHANGELOG.md` (additions under `[Unreleased]` only, `[0.11.0]` untouched) and `crates/paladin-notifications/CHANGELOG.md` (new `[Unreleased]` above `[0.11.0]`). Plan evidence also hashes section 10.8 against the tag. |
| 8 | D-15: no simulated return; triage starts from the consumer's real relayed text | VERIFIED | Close-gate records show the first two snapshots said `no web3sec return`, no web3sec entries were written until the report arrived; return is filed verbatim and every web3sec DF quotes it in a code block. Authenticity of relay itself is the maintainer's (orchestrator fact). Flagged as judgment-tier prohibition P1. |
| 9 | D-07/DOGF-06 close gate: `## Close gate` tables every entry with the checker's output and lists each S1/S2 with fix and test | VERIFIED | `56-DOGFOOD-EVIDENCE.md` `## Close gate` (three dated snapshots, append-only) ends `Close gate: OK`, 36-row table, S1/S2 fix table with commit hashes (all `git cat-file -t` = commit) and regression test names (all found by me in the run output above). |
| 10 | D-06 backstop: Stage C plans take promoted entries in DF-id order and every fix commit cites its `DF-nn` | VERIFIED (explicit evidence; `verification: backstop`) | Plans: DF-07 -> 56-11, DF-08 -> 56-12, DF-19 -> 56-13, DF-26 -> 56-14, ascending. Commit subjects (git log): `fix(56-11)... (DF-07)` x2, `fix(56-12)... (DF-08)`, `fix(56-13)... (DF-19)` x2, `fix(56-14)... (DF-26)` / `feat(56-14)... (DF-26)`, `docs(56-10)... (DF-27, DF-28, DOGF-02)`. |
| 11 | D-00a/D-00b gates: patch-shaped, offline gates green | VERIFIED (partial scope, see Gates) | `make check-gates` (migration allowlist set-equal and zero entries, version strings `0.11.0` over 128 files, stub-panics 0 findings, crate names, publish order, advisory register): all OK. `make check-doc-config` 161 blocks 0 failed. `make test-shell-guards` all pass. Public change is additive: `.project/current-exports.txt` diff is exactly three new `Settings::get_*_config` methods (+6 items) and no removals. |

**Score:** 11/11 truths verified (0 present-but-behavior-unverified)

### Stage C fix inspection (SC5, anti-stub, level 4)

| Finding | Code evidence | Status |
|---------|---------------|--------|
| DF-07 (S1) | `src/main.rs`: `info!("{}", settings_summary(&config))`, no `{:?}` of `Settings`; summary prints only `llm_type`, host:port, agent count, auth enabled and key count. `src/config/redaction.rs` `RedactedSecret` (crate-private) used in hand-written `Debug` for `ApiKeyConfig`, `QueueConfig`, `FileStorageConfig`, `Settings.llm_api_key`; `EmailAdapterConfig` redacts in paladin-notifications. `grep` finds no remaining `info!/debug!/...("{:?}", config/settings)` in `src`. | FLOWING / VERIFIED |
| DF-08 (S2) | `src/bin/paladin-server.rs::load_settings` calls `settings.validate()` and `run()` calls `load_settings`; refusal message `invalid configuration: <key> ...`. | WIRED |
| DF-26 (S2) | Same `load_settings` applies `get_trace_config()`, `get_agent_runtime_config()`, `get_web_server_config()` overlays before `validate()`. Tests include env-driven refusal (`PALADIN_TRACE_CHANNEL_CAPACITY=0`). | WIRED |
| DF-19 (S2) | `src/application/cli/commands/provider_probe.rs` shared by `setup_check.rs` and `onboarding.rs`; `claude-3-haiku` literal gone from `src`; client built with `redirect::Policy::none()`; failure test names model and redacted reply; no non-test `unwrap/expect/panic`. | WIRED |

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `scripts/check-findings-log.sh` | VERIFIED | Runs, exits 0 on real log, 43-assertion shell test, `make check-findings-log` target, correctly absent from `check-gates` (Makefile comment cites D-03). |
| `tests/config_example_roundtrip.rs` + fixtures | VERIFIED | 14 tests green, fixtures present with provenance README update. |
| `.project/v0.11.1/02-dogfood-findings-log.md` | VERIFIED | 36 entries, append-only, all disposed. |
| `56-DOGFOOD-EVIDENCE.md` `## Close gate` | VERIFIED | Present, ends `Close gate: OK`. |
| `56-RUNBOOK-{COVERAGE,KEYED-WALK,WEB3SEC}.md` | VERIFIED | Slots C1..C6, K1..K9, W1..W2 filled with relayed text. |
| `56-FIRST-RUN-WALK.md` | VERIFIED | Keyless walk with provenance proving published crates. |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| Close gate | `scripts/check-findings-log.sh` | quotes final output | WIRED |
| `paladin-server::run` | `Settings::validate` | `load_settings` | WIRED |
| `setup_check` / `onboarding` | `provider_probe` | shared probes | WIRED |
| `Makefile` `test-shell-guards` | `check-findings-log_test.sh` | glob over `tests/scripts/*_test.sh` | WIRED |
| REQUIREMENTS DOGF-02 row | todo | resolution note in `completed/` | WIRED (confirm-triage branch, no re-home row needed) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Findings log has no open entry | `./scripts/check-findings-log.sh` | OK, 36 entries | PASS |
| Checker's own harness | `bash tests/scripts/check-findings-log_test.sh` | 43 passed | PASS |
| Config round-trip | `cargo test -p paladin-ai --features cli,web-server --test config_example_roundtrip` | 14 passed | PASS |
| DF-07/19/26 lib regressions | `cargo test -p paladin-ai --features cli,web-server --lib -- redacts settings_summary load_settings anthropic_probe section_wrappers` | 19 passed | PASS |
| DF-08/26 server boot refusals | `... --bin paladin-server -- load_settings` | 11 passed | PASS |
| DF-07 binary startup line | `... --bin paladin -- settings_summary` | 2 passed | PASS |
| Offline gate set | `make check-gates`, `make check-doc-config`, `make test-shell-guards` | all OK | PASS |

### Probe Execution

Step 7c: SKIPPED. No `scripts/*/tests/probe-*.sh` is declared by the phase plans.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| DOGF-01 | 56-01, 04, 05, 07, 10..14 | every DF entry disposed, none open | SATISFIED | Truth 1, 6 |
| DOGF-02 | 56-03, 56-10 | web3sec return triaged one entry per divergence | SATISFIED | Truth 2, 8 |
| DOGF-03 | 56-02 | example round-trips through loader as a test | SATISFIED | Truth 3 |
| DOGF-04 | 56-03, 06, 07, 08 | published-crate first-run walk, stumbles logged | SATISFIED (relayed keyed half) | Truth 4a |
| DOGF-05 | 56-03, 56-09 | maintainer coverage walk closes the todo | SATISFIED with scope caveat | Truth 4b |
| DOGF-06 | 56-05, 10..14 | S1/S2 fixed before the tag | SATISFIED | Truth 5, 10 |

All six IDs appear in plan frontmatter and in REQUIREMENTS.md (`[x]`, traceability `Complete`). No orphaned requirement: REQUIREMENTS.md maps no other ID to Phase 56.

### Prohibitions (ADR-550 D4 routing)

| Prohibition | Tier | Verdict | Flag |
|-------------|------|---------|------|
| P1: no simulated/inferred web3sec return | judgment | Held (non-authoritative): no web3sec entry until a filed verbatim report existed; entries quote it | `unverified-prohibition - human review recommended` |
| P4: no web3sec final disposition without the named triage evidence | judgment | Held (non-authoritative): each DF-27..36 cites source, test or doc line; DF-36 and DF-27 premises re-checked by me | `unverified-prohibition - human review recommended` |
| P11: agent did not select the D-16 option; no close while S1/S2 lacks fix or checker not OK | judgment | Held (non-authoritative): decision recorded as maintainer-taken, checker was OK before it | `unverified-prohibition - human review recommended` |
| P3: never edit/reorder/delete a log line | test | Measured held (zero deleted lines in `git diff` from seed), but no standing test enforces it (the shell test only proves the checker does not mutate the log). Per fail-closed default this counts as unverified | `unverified-prohibition` (test-tier, no wired enforcement) |

None of these is silently absorbed: all four are in the human-verification list.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (phase-modified code) | - | `TBD`/`FIXME`/`XXX` | none found | Debt-marker gate clean |
| `src/bin/paladin-server.rs`, `src/main.rs`, `tests/config_example_roundtrip.rs` | various | `unwrap/expect/panic!` | Info | All inside `#[cfg(test)]` / test files; `provider_probe.rs` non-test code has none |
| `56-12/13-SUMMARY.md`, `56-01/03/05/06-SUMMARY.md` | frontmatter | `requirements-completed: []` | Info | Hygiene only (Phase 57 SUMMARY reconciliation); REQUIREMENTS.md and plan frontmatter are authoritative and complete |
| `56-*-SUMMARY.md` | - | No statement on WINDOWS.md rows minted/closed | Info | CONTEXT asks the SUMMARY to state any; `git diff` shows `WINDOWS.md` unchanged, so Phase 57's sweep gains nothing from this phase |

### Human Verification Required

#### 1. DF-18 severity (`gpt-4` Quickstart literal)

**Test:** Read DF-18 and the re-home todo. Decide whether the Quickstart's `.model("gpt-4")` failing on a real OpenAI key (exit 1) is S3 as logged or S2.
**Expected:** Accept S3 and the re-home, or regrade and land the one-line change in `docs/src/getting-started/quickstart.md:53` before the tag (the todo itself calls it "a contained one-line edit").
**Why human:** Severity is a D-04 judgement. If S2, DOGF-06 would require a fix before the tag, so this is the one item that could turn into a gap.

#### 2. DOGF-05 devcontainer scope

**Test:** Confirm that the walk inside the devcontainer meets the intent of "Docker-capable machine".
**Expected:** Acceptance of the 92.79 / 92.86 agreement, with trap 2 (bare-host `make services-up` and password-protected Redis) left as a known unwalked path.
**Why human:** The measurement is relayed, and the SUMMARY states guide step 1 was not exercised.

#### 3. Prohibition resolutions (P1, P3, P4, P11)

**Test:** Read the four prohibitions above against the log and `56-10-SUMMARY.md`.
**Expected:** Explicit human resolution for each; for P3, add a standing check (for example a checker clause that fails when an earlier byte of the log changes) or accept the gap.
**Why human:** Judgment-tier verdicts are non-authoritative, and the test-tier item has no enforcement wired.

#### 4. Relayed keyed evidence

**Test:** Confirm K2/K6/K7/K8 and the 2026-11-03 Anthropic re-check are the maintainer's real output.
**Expected:** `setup-check --verbose` reads the Anthropic row as passing with a valid key.
**Why human:** Needs real provider keys.

### Gaps Summary

No gaps. Every roadmap success criterion and plan-level truth resolved to VERIFIED on evidence I re-ran or re-read on HEAD `4af67fa32`, not on SUMMARY claims. The status is `human_needed` only because four judgement or relayed-evidence items remain; item 1 (DF-18 severity) is the one decision that could reopen DOGF-06.

Not run by me, so not claimed: `make api-surface` and `cargo semver-checks` (only the baseline text diff was inspected: three additive methods, no removals), `make security` (known red on RUSTSEC-2026-0429 `zip 3.0.0`, owned by Phase 57 REG-05), `make clean-code`, and the CI `coverage` job at HEAD. The three red CI jobs on the branch are pre-existing and outside this phase. `56-WEB3SEC-RETURN-REQUEST.md` is a superseded cover prompt left as filed.

---

_Verified: 2026-11-04_
_Verifier: Claude (gsd-verifier)_
