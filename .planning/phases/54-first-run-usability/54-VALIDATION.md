---
phase: 54
slug: first-run-usability
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-31
---

# Phase 54 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Seeded by plan-phase from `54-RESEARCH.md` "Validation Architecture". The per-task map below is
> keyed by requirement; the planner's PLAN.md `<verify>` blocks fill in the task and plan ids, and
> `/gsd-validate-phase` seals it.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in `#[test]`: `src/config/agents.rs` unit tests, `src/bin/paladin-server.rs` bin tests. Bash regression harnesses: `tests/scripts/*_test.sh` (auto-discovered by `make test-shell-guards`) |
| **Config file** | none: the workspace `Cargo.toml` and the `Makefile` targets |
| **Quick run command** | `cargo test --lib config::agents::`, `cargo test --features web-server --bin paladin-server build_auth_config`, `./tests/scripts/check-version-strings_test.sh` |
| **Full suite command** | `cargo test`, then `make test-shell-guards`, `make check-gates`, `make clean-code`, `make api-surface`, `make security`, `make check-doc-config`, `make check-migration-allowlist` |
| **Estimated runtime** | about 60 seconds for the quick commands on a warm cache; about 15 minutes for the full workspace |

---

## Sampling Rate

- **After every task commit:** Run the quick command for the touched area (the `config::agents` unit tests, the `paladin-server` bin test or the gate harness), plus `shellcheck` on any script touched, and `cargo fmt --check` with `cargo clippy -- -D warnings` when Rust changed
- **After every plan wave:** Run `make test-shell-guards` and `make check-gates`, plus `cargo test` when Rust changed
- **Before `/gsd-verify-work`:** The full suite must be green: the D-00a set, including `make api-surface`, `make security`, `make clean-code` and the 82 % coverage floor (ADR-0006)
- **Max feedback latency:** 120 seconds for the quick commands

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 54-01-01 | 54-01 | 1 | FIRST-01 | T-54-01, T-54-02, T-54-03 | A key containing `${` is refused with a message naming the entry but never the value or the variable; `$X` is accepted; the key is refused while auth is disabled too | unit | `cargo test --lib config::agents::` | ❌ W0 (new rows in an existing module) | ⬜ pending |
| 54-01-01 | 54-01 | 1 | FIRST-01 | T-54-01 | The boot path refuses a placeholder key before any listener binds | unit (bin) | `cargo test --features web-server --bin paladin-server build_auth_config` | ❌ W0 | ⬜ pending |
| 54-01-02 | 54-01 | 1 | FIRST-01 | T-54-04 | The shipped `config.example.yml` passes its own validator with zero keys, and its commented entries uncommented unedited are refused as empty | unit | `cargo test --lib config::agents::tests::shipped_example` | ❌ W0 | ⬜ pending |
| 54-02-01, 54-02-02, 54-06-01 | 54-02, 54-06 | 1, 2 | FIRST-01 | T-54-05, T-54-15, T-54-16 | No `http.auth` key is a `${…}` placeholder and no expansion claim remains in the examples, `k8s/`, `docker/` or `docs/src`; the k8s Secret template parses with `key: ""` entries, both Deployments mount it, and no retired ConfigMap or per-caller key variable is referenced | grep / script | `grep -rnE '(^\s*\|[{,]\s*\|-\s+)key:\s*"?\$\{' config.example.yml k8s docs/src docker` and `grep -rn 'expanded by the server' k8s config.example.yml` return nothing (this pattern does not match the `llm` `api_key:` lines); the PyYAML assertions of 54-06 Task 1 (both Deployments' Secret volumes, the worker's included, plus the whole-`k8s/server` grep), re-checked by Task 2's whole-`k8s/` grep | n/a | ⬜ pending |
| 54-03-01, 54-03-02, 54-04-01, 54-04-02, 54-04-03 | 54-03, 54-04 | 1 | FIRST-02 | T-54-08, T-54-10, T-54-31 | Every copyable version string reads the workspace version (the sweep is complete); the ungated deployment-guide image tags (`docker.md`, `kubernetes.md`) name no concrete release | grep, then gate | each task's stale-literal grep (54-04-03: `! grep -nE 'paladin:v?[0-9]\|tag: "v?[0-9]' docs/src/deployment/docker.md docs/src/deployment/kubernetes.md` and `make check-doc-config`); then `./scripts/check-version-strings.sh` exits 0 on the real tree (54-07) | n/a | ⬜ pending |
| 54-07-01, 54-07-02 | 54-07 | 3 | FIRST-03 | T-54-20, T-54-21, T-54-22 | The gate goes red on a stale string and green when current; an exempt file passes; zero files, an unmatched class or a stale exemption fails; it never writes | bash harness | `./tests/scripts/check-version-strings_test.sh` (also run by `make test-shell-guards`) | ❌ W0 | ⬜ pending |
| 54-07-01 | 54-07 | 3 | FIRST-03 | — | The gate is wired into `make check-gates` and CI | grep | `make -n check-gates \| grep -q check-version-strings.sh`; `grep -n 'run: ./scripts/check-version-strings.sh' .github/workflows/ci.yml` | n/a | ⬜ pending |
| 54-06-02, 54-07-02 | 54-06, 54-07 | 2, 3 | FIRST-04 | — | The k8s labels and first-party image tags equal the workspace version, and `make docker-build-server` produces that tag | gate + dry run | `./scripts/check-version-strings.sh`; `make -n docker-build-server` shows both the `:latest` and the version tag | via the gate | ⬜ pending |
| 54-05-01 | 54-05 | 1 | FIRST-05 | T-54-12 | `[Unreleased]` anchors carry the correction, the published `[0.11.0]` text is untouched, and the release tool stamps it as `[0.11.1]` | grep + diff + dry run | scratch `finalize-crate-changelogs.sh --version 0.11.1 --metadata-json`; `make check-changelogs` | n/a | ⬜ pending |
| 54-05-02 | 54-05 | 1 | FIRST-06 | T-54-13 | ADR-0056 cites ADR-0057 (check-only) and no longer claims that Phase 42 closes the race with a reservation; `tests.rs` already cites ADR-0057 | grep | the stale-phrase negative grep of 54-05 Task 2; `grep -n 'ADR-0057' src/application/services/treasurer/tests.rs` | n/a | ⬜ pending |
| 54-07-03, 54-08-01, 54-08-02 | 54-07, 54-08 | 3 | FIRST-07 | T-54-24, T-54-26 | No stale crate counts remain; the `cargo-release` line reads `1.1.7`; the dangling footer link is gone, and its removal breaks no script | grep + harness | the count-phrase negative greps of 54-07 Task 3 and 54-08 Task 2; `grep -n 'cargo-release 1.1.7' release.toml`; `grep -n '^\[Unreleased\]:' CHANGELOG.md` returns nothing; `./tests/scripts/check-release-consistency_test.sh` (its needles re-measured before the 54-08-02 hint edit); `make test-shell-guards` | ✅ (harnesses exist) | ⬜ pending |
| 54-09-02 | 54-09 | 4 | FIRST-08 | T-54-28 | The committed API-surface snapshot is current; the by-name clause is reconciled with evidence | gate + evidence | `make api-surface` (exit 0, "API surface unchanged"); `grep -c 'UnsupportedParameter\|check_reasoning' .project/current-exports.txt` recorded alongside the reconciled requirement text | ✅ | ⬜ pending |
| 54-05-03 | 54-05 | 1 | FIRST-09 | T-54-14 | The node-cache guide states that the server attaches no node cache, documents `NODE_CACHE_KEY_VERSION` (`k1`), and documents the `key_prefix` rotation; the other two pages link to it instead of restating it | grep | the heading, term and link greps of 54-05 Task 3; `make check-doc-config` | ❌ W0 (text) | ⬜ pending |
| 54-09-01 | 54-09 | 4 | FIRST-01, FIRST-03 | T-54-27, T-54-29 | SC1 at process level (the built binary exits 1 on a placeholder key and on the unfilled k8s template) and SC3 on the real tree (deliberately stale README pin, red, restored, green) | recorded run | the `<verify>` of 54-09 Task 1 | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `scripts/check-version-strings.sh`: the new gate, committed with mode `100755`
- [ ] `tests/scripts/check-version-strings_test.sh` and its fixtures, mode `100755`: stale, current, exempt and zero-scan cases, with the real tree run last
- [ ] New table rows in the `src/config/agents.rs` tests and a boot-refusal test in `src/bin/paladin-server.rs`, each watched failing first
- [ ] A skeleton for `54-FIRST-EVIDENCE.md` holding the prove-then-lock rows, the red run on the real tree, the API-surface result and the FIRST-08 reconciliation
- Framework install: none (the existing infrastructure covers the phase)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The gate turns red on the real tree (run and recorded by 54-09 Task 1) | FIRST-03 | SC3 asks for a deliberately stale string on the real tree, which cannot stay committed | Change one string (for example the README pin to `"0.5"`) and run `make check-version-strings`. Expect exit 1 with `README.md:<n>: found '0.5', want '0.11.0'`. Run `git checkout -- README.md`, then record both runs in `54-FIRST-EVIDENCE.md` |
| The k8s quick start works as a copy-paste sequence (recorded by 54-09 Task 1) | FIRST-01, FIRST-04 | `kubectl` and a cluster are not available in the container | Read `k8s/README.md` against the manifests: the build target produces the referenced tag, the Secret template carries `config.yml`, and the documented `kubectl apply` targets exist. Record the read in the evidence file |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
