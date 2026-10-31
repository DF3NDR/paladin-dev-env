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
| — | — | — | FIRST-01 | T-54 (placeholder credential) | A key containing `${` is refused with a message naming the entry but never the value or the variable; `$X` is accepted; the key is refused while auth is disabled too | unit | `cargo test --lib config::agents::` | ❌ W0 (new rows in an existing module) | ⬜ pending |
| — | — | — | FIRST-01 | T-54 (placeholder credential) | The boot path refuses a placeholder key before any listener binds | unit (bin) | `cargo test --features web-server --bin paladin-server build_auth_config` | ❌ W0 | ⬜ pending |
| — | — | — | FIRST-01 | — | The shipped `config.example.yml` passes its own validator and carries no placeholder key | unit | `cargo test --lib config::agents::` (example row) | ❌ W0 | ⬜ pending |
| — | — | — | FIRST-01 | T-54 (placeholder credential) | No `${…}` key and no expansion claim remain in the examples, `k8s/` or `docs/src`; the k8s Secret template parses, both Deployments mount it, and no `paladin-server-config` ConfigMap or `PALADIN_API_KEY_*` reference remains | grep / script | `rg -n 'key:\s*["'"'"']?\$\{' config.example.yml k8s docs/src` and `rg -n 'expanded by the server' k8s config.example.yml` both return nothing; a YAML parse of the k8s manifests is recorded in the evidence file | n/a | ⬜ pending |
| — | — | — | FIRST-02 | — | Every copyable version string reads the workspace version (the sweep is complete) | gate | `./scripts/check-version-strings.sh` exits 0 on the real tree | ❌ W0 | ⬜ pending |
| — | — | — | FIRST-03 | — | The gate goes red on a stale string and green when current; an exempt file passes; scanning zero files fails | bash harness | `./tests/scripts/check-version-strings_test.sh` (also run by `make test-shell-guards`) | ❌ W0 | ⬜ pending |
| — | — | — | FIRST-03 | — | The gate is wired into `make check-gates` and CI | grep | `make -n check-gates` lists `check-version-strings`; `rg -n check-version-strings .github/workflows/ci.yml Makefile` | n/a | ⬜ pending |
| — | — | — | FIRST-04 | — | The k8s labels and first-party image tags equal the workspace version, and `make docker-build-server` produces that tag | gate + dry run | `./scripts/check-version-strings.sh`; `make -n docker-build-server` shows both the `:latest` and the version tag | via the gate | ⬜ pending |
| — | — | — | FIRST-05 | — | `[Unreleased]` anchors carry the correction, and the published `[0.11.0]` text is untouched | grep + diff | `rg -n '^## \[Unreleased\]' crates/paladin-memory/CHANGELOG.md crates/paladin-content/CHANGELOG.md`; the `git diff` shows additions only above `## [0.11.0]`; `make check-changelogs` | n/a | ⬜ pending |
| — | — | — | FIRST-06 | — | ADR-0056 cites ADR-0057 (check-only) and no longer claims that Phase 42 closes the race with a reservation; `tests.rs` already cites ADR-0057 | grep | `rg -n 'reserv' .planning/decisions/0056-allowance-admission-model.md` (every hit reads as superseded); `rg -n 'ADR-0057' src/application/services/treasurer/tests.rs` | n/a | ⬜ pending |
| — | — | — | FIRST-07 | — | No stale crate counts remain; the `cargo-release` line reads `1.1.7`; the dangling footer link is gone, and its removal breaks no script | grep + harness | `rg -n 'ten (per-crate\|crate)\|all ten' release.toml scripts/finalize-crate-changelogs.sh .github/workflows/ci.yml` returns nothing; `rg -n 'cargo-release 1.1.7' release.toml`; `rg -n '^\[Unreleased\]:' CHANGELOG.md` returns nothing; `make test-shell-guards` | ✅ (harnesses exist) | ⬜ pending |
| — | — | — | FIRST-08 | — | The committed API-surface snapshot is current; the by-name clause is reconciled with evidence | gate + evidence | `make api-surface` (exit 0, "API surface unchanged"); `rg -c 'UnsupportedParameter\|check_reasoning' .project/current-exports.txt` recorded alongside the reconciled requirement text | ✅ | ⬜ pending |
| — | — | — | FIRST-09 | — | The node-cache guide states that the server attaches no node cache, documents `NODE_CACHE_KEY_VERSION` (`k1`), and documents the `key_prefix` rotation; the other two pages link to it instead of restating it | grep | `rg -n 'attaches no node cache' docs/src/user-guides/fault-tolerance.md`; `rg -n 'NODE_CACHE_KEY_VERSION' docs/src/user-guides/fault-tolerance.md`; `rg -n 'key_prefix' docs/src/user-guides/fault-tolerance.md`; `make check-doc-config` | ❌ W0 (text) | ⬜ pending |

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
| The gate turns red on the real tree | FIRST-03 | SC3 asks for a deliberately stale string on the real tree, which cannot stay committed | Change one string (for example the README pin to `"0.5"`) and run `make check-version-strings`. Expect exit 1 with `README.md:<n>: found '0.5', want '0.11.0'`. Run `git checkout -- README.md`, then record both runs in `54-FIRST-EVIDENCE.md` |
| The k8s quick start works as a copy-paste sequence | FIRST-01, FIRST-04 | `kubectl` and a cluster are not available in the container | Read `k8s/README.md` against the manifests: the build target produces the referenced tag, the Secret template carries `config.yml`, and the documented `kubectl apply` targets exist. Record the read in the evidence file |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
