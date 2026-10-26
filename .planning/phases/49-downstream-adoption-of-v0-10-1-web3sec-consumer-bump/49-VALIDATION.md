---
phase: 49
slug: downstream-adoption-of-v0-10-1-web3sec-consumer-bump
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-10-14
---

# Phase 49 — Validation Strategy

> **Scope cut 2026-10-14:** plans 49-04..06 were cancelled (see ROADMAP Phase 49). Rows 49-04-*, 49-05-* and 49-06-* below and the `49-CONSUMER-ADOPTION-SPEC.md` / `49-RETURN-report.md` artifacts are void; the kept plans are 49-01..03.

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `cargo test` (libtest) for Rust; shell-script gates (`scripts/check-*.sh`) |
| **Config file** | `Cargo.toml` (`[[test]]` tables); `Makefile`; `.github/workflows/ci.yml` |
| **Quick run command** | `cargo test --offline --test consumer_surface_muster` |
| **Full suite command** | `cargo test --workspace` plus `make clean-code`, `make check-gates`, `make api-surface` (coverage via CI `coverage` job) |
| **Estimated runtime** | ~45 seconds (quick, warm); gates minutes |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --offline --test consumer_surface_muster` (or `make check-gates` for doc-only tasks)
- **After every plan wave:** Run the three guard tests, `cargo clippy --test consumer_surface_muster -- -D warnings`, `cargo fmt --all -- --check`, `make check-gates`
- **Before `/gsd-verify-work`:** Full suite must be green
- **Max feedback latency:** 60 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 49-01-01 | 01 | 1 | ADOPT-04 | T-49-01, T-49-02, T-49-03, T-49-07 | no pin executed; library untouched | compile-only integration test | `cargo test --offline --test consumer_surface_muster && cargo clippy --offline --test consumer_surface_muster -- -D warnings && cargo fmt --all -- --check && cargo test --offline --test entity_rename_guard --test legacy_removal_guard --test treasurer_vocabulary_guard` | ✅ | ⬜ pending |
| 49-01-02 | 01 | 1 | ADOPT-04 | T-49-07 | public API surface unchanged | gate run | `make check-consumer-surface && make check && cargo test --offline --test consumer_surface_muster --no-default-features 2>&1 \| grep -q 'requires the features' && make lint && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |
| 49-01-03 | 01 | 1 | ADOPT-04 | -- | N/A | grep acceptance | `F=.planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-VALIDATION.md; test "$(grep -c '^\| 49-0[1-6]-0[1-3] ' "$F")" = "$(cat .planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-0[1-6]-PLAN.md \| grep -cE '^.task type=')" && grep -q '^status: draft' "$F" && grep -q 'consumer_surface_muster' "$F"` | ✅ | ⬜ pending |
| 49-02-01 | 02 | 1 | ADOPT-05 | T-49-07, T-49-08 | section 9.2 register byte-identical; replacements confirmed | diff assertion | `BASE=$(git log -1 --format=%H -F --grep='docs(49): add phase research and validation strategy'); test "$(awk '/^### Library consumers/,0' MIGRATION.md \| grep -oE '^\\| DR-[0-9]+ ' \| tr -d '\| ' \| paste -sd, -)" = "DR-1,DR-2,DR-3,DR-4,DR-5,DR-6,DR-7,DR-8,DR-9,DR-10,DR-11" && test "$(git diff --numstat "$BASE" -- MIGRATION.md \| awk '{print $2}')" = 0 && diff <(git show "$BASE":MIGRATION.md \| sed -n '/^## 9\.2 /,/^## 9\.3 /p') <(sed -n '/^## 9\.2 /,/^## 9\.3 /p' MIGRATION.md) && git diff --exit-code "$BASE" -- .cargo/semver-checks-allowlist.toml && make check-gates` | ✅ | ⬜ pending |
| 49-02-02 | 02 | 1 | ADOPT-05 | -- | N/A | grep acceptance | `BASE=$(git log -1 --format=%H -F --grep='docs(49): add phase research and validation strategy'); awk '/^### Added/{f=1;next} f && /^- \*\*/{print; exit}' CHANGELOG.md \| grep -q 'Consumer-surface muster and library-consumer upgrade note' && test "$(git diff --numstat "$BASE" -- CHANGELOG.md \| awk '{print $2}')" = 0 && test "$(sed -n '/^## \[Unreleased\]/,/^## \[0\.10\.1\]/p' CHANGELOG.md \| grep -c '^### ')" = 7 && make check-gates && cargo test --offline --test entity_rename_guard --test legacy_removal_guard --test treasurer_vocabulary_guard` | ✅ | ⬜ pending |
| 49-03-01 | 03 | 2 | ADOPT-04 | T-49-04, T-49-07 | tree restored after mutation | scripted mutation | `bash "${TMPDIR:-/tmp}/49-red-proof/red_proof_commissary.sh" "$PWD" \| grep -qx 'RED-PROVEN' && test -z "$(git status --porcelain -- crates src)" && git diff --quiet -- crates src && cargo test --offline --test consumer_surface_muster` | ❌ W0 | ⬜ pending |
| 49-03-02 | 03 | 2 | ADOPT-04 | T-49-04, T-49-07 | tree restored after mutation | scripted mutation | `bash "${TMPDIR:-/tmp}/49-red-proof/red_proof_facade.sh" "$PWD" \| grep -qx 'RED-PROVEN' && test -z "$(git status --porcelain -- crates src)" && cargo test --offline --test consumer_surface_muster && grep -q '^## Restoration' .planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-MUSTER-RED-EVIDENCE.md` | ❌ W0 | ⬜ pending |
| 49-04-01 | 04 | 3 | ADOPT-01, ADOPT-02, ADOPT-03 | T-49-05, T-49-06 | pin by commit SHA; no consumer facts asserted | grep acceptance | `S=.planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-CONSUMER-ADOPTION-SPEC.md; test "$(git rev-parse 'v0.10.1^{commit}')" = f7dae2676580786e9541fefee8f787623b46c70f && test "$(git rev-parse v0.10.1)" = 7593ab4dc1217ef23d6f8c14f8c8b817d42ec0ae && test "$(git show --name-only --format= 4b7bed87 \| grep -c .)" = 7 && git diff --quiet 4b7bed87 v0.10.1 -- $(git show --name-only --format= 4b7bed87) && ! git merge-base --is-ancestor 4b7bed87 v0.10.1 && for t in f7dae2676580786e9541fefee8f787623b46c70f 7593ab4dc1217ef23d6f8c14f8c8b817d42ec0ae 'v0.10.1^{commit}' 'rg -U -N --no-heading -o --type rust' AddAssign 'TokenUsage::new(0, 4242)' include_str 1.95.0 49-REFERENCE-adoption.diff; do grep -qF -- "$t" "$S" \|\| exit 1; done && git show --name-only --format= 4b7bed87 \| while read -r p; do grep -qF -- "$p" "$S" \|\| exit 1; done` | ❌ W0 | ⬜ pending |
| 49-04-02 | 04 | 3 | ADOPT-01, ADOPT-02, ADOPT-03 | T-49-06 | every consumer field PENDING | grep acceptance | `R=.planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-RETURN-report.md; test "$(grep -c '^- \*\*[a-z0-9_]*:\*\* PENDING$' "$R")" = 21 && test "$(grep -cx 'PENDING' "$R")" = 4 && grep -q '^status: awaiting-consumer' "$R" && grep -q '^## Intake (Paladin, plan 49-05)' "$R" && for g in '(49-01)' '(49-02)' '(49-03)'; do s=$(git log -1 --format=%h -F --grep="$g"); test -n "$s" && grep -qF "$s" "$R" \|\| exit 1; done` | ❌ W0 | ⬜ pending |
| 49-04-03 | 04 | 3 | ADOPT-01, ADOPT-02, ADOPT-03 | T-49-03, T-49-07 | library diff against phase base empty | diff assertion | `BASE=$(git log -1 --format=%H -F --grep='docs(49): add phase research and validation strategy'); test -z "$(git diff --name-only "$BASE" -- crates src)" && test "$(git diff --numstat "$BASE" -- Cargo.toml \| awk '{print $2}')" = 0 && git diff --exit-code "$BASE" -- Cargo.lock .project/current-exports.txt .cargo/semver-checks-allowlist.toml && test -z "$(git diff --name-only "$BASE" -- '*.sh')" && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface && make check-gates && make check-api-examples && ! grep -q 'PENDING (filled by 49-04 Task 3)' .planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-RETURN-report.md` | ❌ W0 | ⬜ pending |
| 49-05-01 | 05 | 4 | ADOPT-01, ADOPT-02, ADOPT-03 | T-49-05, T-49-06, T-49-09 | consumer facts only from the filed report | human-action | manual (checkpoint:human-action) -- intake automated in the next task | ❌ W0 | ⬜ pending |
| 49-05-02 | 05 | 4 | ADOPT-01, ADOPT-02, ADOPT-03 | T-49-05, T-49-06, T-49-09, T-49-10 | consumer facts only from the filed report | report intake | `R=.planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-RETURN-report.md; bash "${TMPDIR:-/tmp}/49-intake/intake.sh" "$R" && grep -q '^status: accepted' "$R" && test "$(sed -n '/^## Intake (Paladin, plan 49-05)/,$p' "$R" \| grep -c '^PASS C')" = 14` | ❌ W0 | ⬜ pending |
| 49-06-01 | 06 | 5 | ADOPT-04 | T-49-01, T-49-07, T-49-10 | no pin executed; library untouched | compile-only integration test | `cargo test --offline --test consumer_surface_muster && cargo clippy --offline --test consumer_surface_muster -- -D warnings && cargo fmt --all -- --check && cargo test --offline --test entity_rename_guard --test legacy_removal_guard --test treasurer_vocabulary_guard && make check && grep -q '^## Muster v1.1 reconciliation (Paladin, plan 49-06)' .planning/phases/49-downstream-adoption-of-v0-10-1-web3sec-consumer-bump/49-RETURN-report.md` | ❌ W0 | ⬜ pending |
| 49-06-02 | 06 | 5 | ADOPT-04 | T-49-07 | library diff against phase base empty | diff assertion | `BASE=$(git log -1 --format=%H -F --grep='docs(49): add phase research and validation strategy'); L=$(awk '/^### Library consumers/,0' MIGRATION.md \| grep -oE '^\\| DR-[0-9]+ ' \| tr -dc '0-9\n'); test "$L" = "$(printf '%s\n' $L \| sort -n)" && test "$(printf '%s\n' $L \| head -1)" = 1 && test "$(printf '%s\n' $L \| sort -nu \| wc -l)" = "$(printf '%s\n' $L \| tail -1)" && diff <(git show "$BASE":MIGRATION.md \| sed -n '/^## 9\.2 /,/^## 9\.3 /p') <(sed -n '/^## 9\.2 /,/^## 9\.3 /p' MIGRATION.md) && git diff --exit-code "$BASE" -- .cargo/semver-checks-allowlist.toml && test -z "$(git diff --name-only "$BASE" -- crates src)" && make check-gates && PUBLIC_API_TOOLCHAIN=nightly-2026-09-20 make api-surface` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `tests/consumer_surface_muster.rs` -- the compile-only consumer-surface muster (ADOPT-04); created by plan 49-01
- [x] `[[test]] consumer_surface_muster` table in `Cargo.toml` (ADOPT-04); created by plan 49-01
- [ ] `49-MUSTER-RED-EVIDENCE.md` -- recorded proof the muster turns red on a mutation (ADOPT-04); plan 49-03
- [ ] `49-CONSUMER-ADOPTION-SPEC.md` and `49-RETURN-report.md` (ADOPT-01..03); plan 49-04
- [ ] MIGRATION.md library-consumer note and CHANGELOG `[Unreleased]` entry (ADOPT-05); plan 49-02

*No framework install is needed: `cargo test` and the shell-script gates already exist.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| The consumer pins `paladin` at exactly `v0.10.1` (`f7dae267`), detached, in one commit with the adaptation and `Cargo.lock` | ADOPT-01 | The consumer repository is unreachable from this environment | The consumer follows `49-CONSUMER-ADOPTION-SPEC.md` and files `49-RETURN-report.md`; the plan 49-05 intake (C1-C14) closes it |
| Consumer call sites adapted for B-1, B-2 and B-3, with the D-2 token-usage mapping decided and documented | ADOPT-02 | The consumer call sites live only in the consumer repository | The consumer's `git show --stat` and Step-0 inventory in the filed return report are checked by the 49-05 intake |
| The consumer workspace is green on the new pin (fmt, clippy, check, test with at least 2,592 passed) | ADOPT-03 | Consumer gates run only in the consumer repository | The pasted gate and test output in the filed return report is checked by the 49-05 intake; plan 49-06 reconciles the inventory with the muster |
| The 82 % workspace line-coverage floor (ADR-0006) | n/a (phase-wide gate) | `cargo-llvm-cov` is absent locally | CI's `coverage` job on the branch head is the evidence |


---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
