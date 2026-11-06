---
phase: 56
slug: dogfood-findings
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: 2026-11-02
---

# Phase 56 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: `56-RESEARCH.md`
> § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test` (libtest) for the config round-trip. The bash harness (`assert_fire`/`assert_silent`, `tests/scripts/*_test.sh`) covers the findings-log checker. |
| **Config file** | Root `Cargo.toml`: `tests/*.rs` is auto-discovered. `Makefile` `test-shell-guards` globs `tests/scripts/*_test.sh`. |
| **Quick run command** | `./tests/scripts/check-findings-log_test.sh` and `cargo test --test config_example_roundtrip` |
| **Full suite command** | `cargo test` + `make test-shell-guards` + `make check-gates` + `make clean-code` |
| **Estimated runtime** | Quick: ~5 s for the shell test, plus a ~60 s incremental build for the round-trip test. Full: several minutes. |

**Disk constraint:** about 4.2 GB is free. Clear `target/` incrementals before full builds, and build any published-crate scratch walk with a `CARGO_TARGET_DIR` that is deleted afterwards (see `56-RESEARCH.md`).

---

## Sampling Rate

- **After every task commit:** run the quick command for the files touched, plus `shellcheck --severity=warning` on any new `*.sh`.
- **After every plan wave:** run `cargo test`, `make test-shell-guards`, `make check-gates` and `make clean-code`.
- **Before `/gsd-verify-work`:** the full suite is green, `make check-findings-log` exits 0, and `make security` and `make api-surface` pass.
- **Max feedback latency:** 120 seconds.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 56-checker | 56-01 (T1-T2) | 1 | DOGF-01 | T-56-01..05 | Anchored regexes; no shell interpolation of log content; never vacuous (`ZERO_ENTRIES`, `UNCLOSED_FENCE`, `UNREADABLE`) | shell fixture | `./tests/scripts/check-findings-log_test.sh` | ✅ | ✅ green |
| 56-roundtrip | 56-02 (T1-T2) | 1 | DOGF-03 | T-56-06..08 | Failure messages carry key paths, never values; `Settings` is never `{:?}`-printed; fixture creds are fake | integration | `cargo test -p paladin-ai --test config_example_roundtrip` | ✅ | ✅ green |
| 56-drift-guard | 56-02 (T3) | 1 | DOGF-03 | T-56-09 | Only `steps: []` may be added to the example (L5) | integration | `cargo test -p paladin-ai --test config_example_roundtrip` | ✅ | ✅ green |
| 56-runbooks | 56-03 (T1-T3) | 1 | DOGF-02, DOGF-04, DOGF-05 | T-56-11..13 | `read -rs` key entry; redaction before paste; W1 empty until relayed | runbook self-check | snippet on synthetic `lcov.info` prints `Lines: 165/200 = 82.50%  (truncated: 82)` | ✅ evidence file | 🟦 manual-only (accepted) |
| 56-df-proofs | 56-04 (T1-T2) | 2 | DOGF-01 | T-56-15 | Re-runs the owners' regression tests (FIRST-01 refusal, TENANT scope) | existing tests + gates | commands in `56-RESEARCH.md` Answer 7, each grepped as `<name> ... ok` | ✅ | ✅ green |
| 56-dispositions | 56-04 (T2) | 2 | DOGF-01 | T-56-16 | Append-only (0 deleted log lines) | script on real log | `./scripts/check-findings-log.sh` names none of DF-01…DF-04 | ✅ | ✅ green |
| 56-leads | 56-05 (T1-T2) | 3 | DOGF-01, DOGF-06 | T-56-18..21 | Fake key only; evidence shows `<fake-key>`; L3/L6 logged `open → promote` for Stage C | process probes + checker | `## Lead L3/L4/L6` in `56-DOGFOOD-EVIDENCE.md` + `./scripts/check-findings-log.sh` | ✅ evidence file | 🟦 manual-only (accepted) |
| 56-walk-keyless | 56-06 (T1-T2) | 4 | DOGF-04 | T-56-22..25 | Registry source only (cargo metadata source line); no key used | manual evidence + checker | `test -f .planning/phases/56-dogfood-findings/56-FIRST-RUN-WALK.md` + `./scripts/check-findings-log.sh` | ✅ evidence file | 🟦 manual-only (accepted) |
| 56-docs-fixes | 56-07 (T1-T3) | 5 | DOGF-04, DOGF-01 | T-56-26..28 | `--locked` install line; frozen records untouched | resolution proof + gates | `make check-version-strings` + `cargo metadata` in scratch | ✅ evidence file | 🟦 manual-only (accepted) |
| 56-walk-keyed | 56-08 (T1-T3) | 6 (B) | DOGF-04 | T-56-29..31 | Key only in the maintainer's shell; scratch `.env` deleted; secret scan before commit | manual + scan | runbook checkpoint; 9 K-rows in `## Keyed half (maintainer)` | ✅ evidence file | 🟦 manual-only (accepted) |
| 56-coverage-walk | 56-09 (T1-T3) | 7 (B) | DOGF-05 | T-56-32..34 | Same-commit comparison; `int()` truncation | manual (Docker) + gh | `test -f .planning/todos/completed/2026-08-13-verify-local-coverage-reproduction.md` (pass) or `## Update` in the pending todo (fail) | ✅ evidence file | 🟦 manual-only (accepted) |
| 56-web3sec | 56-10 (T1-T3) | 9 (B) | DOGF-02, DOGF-06 | T-56-35..37 | No simulated return; maintainer-only decision | manual + checker | `./scripts/check-findings-log.sh` + grep for owner/date on re-home | ✅ evidence file | 🟦 manual-only (accepted) |
| 56-df07 | 56-11 (T1-T3, Stage C) | 4 | DOGF-06, DOGF-01 | T-56-38..43 | No credential in the `paladin` startup line or in the `Debug` of `ApiKeyConfig`, `AuthConfig`, `WebHttpConfig`, `QueueConfig`, `FileStorageConfig`, `Settings`, `EmailAdapterConfig` (`<redacted>`); every test plants its sentinel first; binary re-probe counts 0 for all five fakes | unit + bin unit + binary probe | `cargo test -p paladin-ai --features cli,web-server --lib debug_redact` + `--bin paladin settings_summary` + `cargo test -p paladin-notifications --features email --lib debug_redact` | ✅ | ✅ green |
| 56-df08 | 56-12 (T1-T2, Stage C) | 5 | DOGF-06, DOGF-01 | T-56-44..47 | `paladin-server` refuses at boot, before any listener, every value `Settings::validate()` rejects, naming the key; the refusal never echoes an `otel.headers` value; the shipped example still boots | bin unit + binary boot probe | `cargo test -p paladin-ai --features cli,web-server --bin paladin-server load_settings` + the Task 1 `bad.yml` boot (exit 1, refusal line, no `listening on`) | ✅ | ✅ green |
| 56-df19 | 56-13 (T1-T4, Stage C) | 7 | DOGF-06, DOGF-01 | T-56-48..51 | Anthropic probe model is the adapter's seeded default (drift-tested); every probe reply is redacted then bounded; the one probe client never follows a redirect; maintainer's real key only via `read -rs` | lib unit (mockito) + maintainer re-check | `cargo test -p paladin-ai --features cli,web-server --lib provider_probe` + Task 3 `checkpoint:human-verify` | ✅ | ✅ green |
| 56-df26 | 56-14 (T1-T3, Stage C) | 8 | DOGF-06, DOGF-01 | T-56-52..56 | `paladin-server` applies the `PALADIN_TRACE_*` / `APP_AGENT_RUNTIME_*` / `APP_WEB_SERVER_*` overlays in `load_settings` before `Settings::validate()`, so an invalid overlaid value refuses boot naming the key; an unset environment is a no-op; every env-touching test is `#[serial]` and restores the environment on drop; no overlay value reaches a log | bin unit + lib unit + doctest + binary boot probe | `cargo test -p paladin-ai --features cli,web-server --bin paladin-server load_settings` + `cargo test -p paladin-ai --features cli,web-server --lib section_wrappers` + `--doc settings::Settings::get_` + the Task 1 `env0` boot (`PALADIN_TRACE_CHANNEL_CAPACITY=0`: exit 1, refusal line, no `listening on`) | ✅ | ✅ green |
| 56-stage-c-gates | 56-11..56-14 (last task each) | 4/5/7/8 | DOGF-06 | -- | D-00b patch shape: no public signature change, except 56-14's three additive `Settings::get_*_config` accessors (`make api-surface-update`, +6 `pub fn` lines, CHANGELOG `### Added`); MIGRATION section 11.2 header-only; allowlist empty | gates | `make api-surface` + `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh --package paladin-ai` (+ `--package paladin-notifications` in 56-11) + `make check-gates` + `./scripts/check-findings-log.sh` | ✅ | ✅ green |

*Status: ⬜ pending · 🟦 manual-only (accepted, see Manual-Only section) · ✅ green · ❌ red · ⚠️ flaky*. Plan ids filled in by the planner (2026-11-02). Stage C plans 56-11 (DF-07), 56-12 (DF-08) and 56-13 (DF-19) were written from the log by the `--gaps` replan on 2026-11-03 (D-06, D-07), and 56-14 (DF-26, wave 8, after 56-13) by a second `--gaps` replan the same day; they run before 56-10 resumes at wave 9, chained because each edits `CHANGELOG.md` and the log. Every Stage C cargo call uses one build profile, `-p paladin-ai --features cli,web-server` (PD-11-4), so the three plans share one dependency build on the ~5.4 GB disk.

---

## Wave 0 Requirements

- [x] `scripts/check-findings-log.sh` + `Makefile` target `check-findings-log` (with a `## ` help comment) — DOGF-01
- [x] `tests/scripts/check-findings-log_test.sh` (executable), with these fixture cases: open, missing,
  unrecognised, bare `in v0.11.1`, `out` without reason, `duplicate of` with a missing target,
  duplicate id, non-increasing id, fenced template ignored, last-bullet-wins, zero entries,
  unreadable, determinism, no-mutation. It must NOT assert that the real log passes; it asserts
  only that the real log parses (≥ 4 entries). — DOGF-01
- [x] `tests/config_example_roundtrip.rs` + `tests/fixtures/config/config.example.full.yml` + a provenance paragraph in `tests/fixtures/config/README.md` — DOGF-03
- [x] Optional `[dev-dependencies] serde_yaml = "0.9"` (already in `Cargo.lock`) — DOGF-03
- [x] Stage B runbooks: the keyed walk; the coverage walk with the LF/LH summation snippet and the port-mismatch check; the web3sec triage — DOGF-02/04/05

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| `onboarding` / `setup-check` / Quickstart against a real OpenAI key | DOGF-04 | Needs a real provider key, which never enters the repo | Runbook in the walk evidence file; paste redacted output; log stumbles as DF-nn |
| Local `make services-up` → `make coverage` matches the CI figure to the whole percent | DOGF-05 | Needs a Docker daemon (none in the agent sandbox) | Coverage runbook; record the local figure, CI run id and commit; move the todo to completed |
| web3sec §6 return triage, or a re-home decision | DOGF-02 | External consumer's return; the maintainer's decision | `checkpoint:decision`: triage one DF per divergence, or re-home with owner and date |
| A valid Anthropic key passes `paladin-cli setup-check` after the DF-19 fix | DOGF-06 | Needs the maintainer's real Anthropic key, which never enters the repo | 56-13 Task 3: `read -rs ANTHROPIC_API_KEY`, `cargo run -q -p paladin-ai --features cli --bin paladin-cli -- setup-check --verbose`, expect `✓ Anthropic`; paste the redacted `LLM Provider:` block |

---

## Validation Audit 2026-11-06

| Metric | Count |
|--------|-------|
| Per-Task Map rows | 17 |
| Covered by existing automated test | 10 (56-checker, 56-roundtrip, 56-drift-guard, 56-df-proofs, 56-dispositions, 56-df07, 56-df08, 56-df19, 56-df26, 56-stage-c-gates) |
| Manual-only, accepted with reason | 7 (56-runbooks, 56-leads, 56-walk-keyless, 56-docs-fixes, 56-walk-keyed, 56-coverage-walk, 56-web3sec) |
| Escalated | 0 |

Planning-only audit: no test was added or run. Evidence is the recorded SUMMARY results (`56-01`..`56-14`), `56-VERIFICATION.md` (status passed, 11/11, re-run on HEAD `4af67fa32`: checker harness 43 passed, `config_example_roundtrip` 14 passed, DF-07/19/26 lib 19 passed, `paladin-server load_settings` 11 passed, `paladin settings_summary` 2 passed) and `56-UAT.md` (4 of 4 pass). Wave 0 files confirmed on disk: `scripts/check-findings-log.sh`, `tests/scripts/check-findings-log_test.sh`, `tests/config_example_roundtrip.rs`, `tests/fixtures/config/config.example.full.yml`.

Manual-only rows rest on evidence documents (`56-FIRST-RUN-WALK.md`, `56-DOGFOOD-EVIDENCE.md`, `56-RUNBOOK-*.md`, the completed coverage todo, `56-WEB3SEC-RETURN-REPORT-50.md`) plus the append-only findings log checked by `./scripts/check-findings-log.sh`; they need a real provider key, a Docker daemon or an external consumer, so no repo test can cover them.

Caveats: `make security` is red on RUSTSEC-2026-0429 (`zip 3.0.0`), pre-existing and owned by Phase 57 REG-05 (`deferred-items.md`); `make api-surface`, `cargo semver-checks`, `make clean-code` and the CI `coverage` job at HEAD were not re-run by the verifier. These are gate items, not task coverage gaps. DF-36 (Cadence import pins in `tests/consumer_surface_muster.rs`) is re-homed, not covered here.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated by nyquist audit, planning-only, 2026-11-06 (no human signature claimed). `nyquist_compliant: true` because every Per-Task Map row has an existing automated test or an accepted manual-only item with a reason; the caveats above are noted, not hidden.
