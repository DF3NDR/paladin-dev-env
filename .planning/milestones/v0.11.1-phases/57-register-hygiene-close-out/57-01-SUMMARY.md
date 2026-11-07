---
phase: 57-register-hygiene-close-out
plan: 01
subsystem: security-governance
tags: [rustsec, cargo-audit, cargo-deny, zip, utoipa-swagger-ui, summary-frontmatter, debug-archive]

requires:
  - phase: 56-dogfood-findings
    provides: "RUSTSEC-2026-0429 recorded as deferred to Phase 57 (56-VERIFICATION.md:168)"
provides:
  - "fourth zip 3.0.0 exception row (RUSTSEC-2026-0429) with both mirrors; make security exits 0"
  - "D-31 re-dating of all four zip rows and the D-32 dated history paragraph"
  - "requirements-completed on four archived SUMMARYs (STORE-02, REASON-02/03/05/06) read back with summary-extract"
  - "three diagnosed debug sessions archived under .planning/debug/resolved/"
affects: [57-02, 57-03, 57-04, 57-05, 57-06, 57-07, 57-08, 57-09, 57-10, 57-11, milestone-audit]

tech-stack:
  added: []
  patterns:
    - "register review_date carries an ISO date prefix followed by the milestone condition so a later staleness check can parse it"
    - "frontmatter amendments to archived SUMMARYs are paired with an appended dated evidence section, never an edited body line"

key-files:
  created:
    - .planning/debug/resolved/kimi-fallback-profile-stale.md
    - .planning/debug/resolved/openai-max-completion-tokens.md
    - .planning/debug/resolved/vendor-token-usage-fidelity.md
  modified:
    - SECURITY-EXCEPTIONS.md
    - .cargo/audit.toml
    - deny.toml
    - .planning/milestones/v0.11.0-phases/45-rustfs-swap-platform-observability-deviations/45-01-SUMMARY.md
    - .planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/51-13-SUMMARY.md
    - .planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/51-14-SUMMARY.md
    - .planning/milestones/v0.11.0-phases/51-adjustable-reasoning-effort/51-15-SUMMARY.md

key-decisions:
  - "PD-01-1: RUSTSEC-2026-0429 gets a fourth zip row under the same measured compensating control; reversible (a register row)"
  - "D-30/D-31: the upstream condition was met on 2026-11-04 (utoipa-swagger-ui 10.0.2 -> zip ^9.0) but not taken in v0.11.1 because it also requires utoipa ^6.0.0; lift deferred to the utoipa 6 migration in the next minor release"
  - "PD-01-2: review_date = '2027-03-31 or the first planning day of the next milestone, whichever is earlier'"
  - "D-41: requirements-completed recorded on the last plan that progressed each id, with appended dated evidence beside it"
  - "D-42: debug sessions archived with git mv, closed by fixed WINDOWS rows 76 / 75 / 74"

patterns-established:
  - "Mirror contract: an advisory id lands in the register, .cargo/audit.toml and deny.toml in one commit, then make security is run separately because the guard never reads the advisory db"

requirements-completed: [REG-06, REG-01, REG-03]

status: complete
duration: "approx. 10 min for the continuation (Tasks 2 and 3); Task 1 ran in the prior executor"
completed: 2026-11-04
---

# Phase 57 Plan 01: Register hygiene (RustSec, SUMMARY frontmatter, debug archive) Summary

RUSTSEC-2026-0429 (zip 3.0.0) is governed by a fourth register row with a re-measured build-time-only control, all four zip rows carry the 2026-11-04 upstream check and a D-31 review date, `make security` is green, four archived SUMMARYs now report their completed requirements, and three resolved debug sessions are archived.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 (tracer) | Govern RUSTSEC-2026-0429: register row plus both mirrors | `ea72092cc` |
| 2 | D-31 re-dating of four zip rows, D-32 history paragraph, mirror comment, prose counts | `10b8282ea` |
| 3 | REG-01 SUMMARY frontmatter and REG-03 debug archive | `33b399b8e` |

## Gates

Before (Task 1, measured by the prior executor): `make security` exited 1. `cargo audit` printed `error: 1 vulnerability found!` on RUSTSEC-2026-0429 (zip 3.0.0, "ZipArchive::extract aborts the process on a symlink entry with a large declared size", 2026-10-22, severity 7.5, patched >= 9.0.0).

After Task 1: `make security` exit 0. `cargo audit` printed `warning: 9 allowed warnings found` with no vulnerabilities; `cargo deny` printed advisories ok, bans ok, licenses ok, sources ok.

After Task 2 (re-run on the re-dated register): `make security` exit 0, ending `advisories ok, bans ok, licenses ok, sources ok`.

Guard, after Tasks 1 and 2: `./scripts/check-advisory-register.sh` printed `16 register row(s) checked against 16 deny.toml and 9 .cargo/audit.toml ignore entries; all clauses satisfied.` (exit 0). The guard never reads the advisory database, so it is not recorded as evidence that `make security` is green (P-01-2); `make security` was run separately.

After Task 3 (plan head `33b399b8e`): `make check-gates` exit 0.

P-01-1 held: `git diff bc0267e5 -- Cargo.lock crates/paladin-web/Cargo.toml` is empty; `Cargo.lock` still pins `utoipa-swagger-ui` 9.0.2 and `zip` 3.0.0.

## Compensating control

Measured from `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/utoipa-swagger-ui-9.0.2/build.rs`:

- line 84 `extract_dist`; line 86 a local `fn extract<R: Seek + Read>(zip: &mut ZipArchive<R>, ...)`; line 93 `zip.by_index(index)?`; line 95 `.enclosed_name()`; lines 143 and 144 call the local helper (no receiver).
- with the `vendored` feature, line 162 opens `ZipArchive::new(io::Cursor::new(utoipa_swagger_ui_vendored::SWAGGER_UI_VENDORED))`, the crate's own embedded bundle.
- `grep -nE '[.]extract[(]' build.rs` prints nothing: no method-call use of the zip crate's extraction API.
- `cargo tree -i zip@3.0.0 --target all --workspace --all-features` prints `zip v3.0.0 [build-dependencies] -> utoipa-swagger-ui v9.0.2 -> paladin-web`; with `-e normal` it prints "nothing to print", so zip is reached only as a build-dependency. The plan's literal `cargo tree -i zip -e build` prints "nothing to print" because the paladin-web to utoipa-swagger-ui edge is a normal edge; the invocation above is the one that resolves.

The control holds, so the row was added (T-57-01 mitigated).

## Upstream check

crates.io, queried 2026-11-04 (Task 2):

- `utoipa-swagger-ui/10.0.2/dependencies`: `zip ^9.0` (kind build, not optional), `utoipa ^6.0.0` (normal), `utoipa-swagger-ui-vendored ^0.2` (build, optional).
- newest `utoipa` is 6.0.0 (max stable 6.0.0); newest `utoipa-swagger-ui` is 10.0.2.

This matches the plan's expected answer, so the D-31 wording was used unchanged. The tree is on utoipa 5.5.0 / utoipa-axum 0.2 / utoipa-swagger-ui 9.0.2, so reaching zip >= 9 is a utoipa 6 migration, deferred to the next minor release.

## Read-backs

`gsd-tools query summary-extract <path> --fields requirements_completed`:

| SUMMARY | Before | After |
|---------|--------|-------|
| 45-01 | `[]` | `["STORE-02"]` |
| 51-13 | `[]` | `["REASON-02"]` |
| 51-14 | `[]` | `["REASON-03","REASON-06"]` |
| 51-15 | `[]` (no key) | `["REASON-05"]` |

`git diff HEAD~1` of the Task 3 commit deletes exactly three lines in the milestone tree (the three `requirements-completed: []` lines) and no `requirements-progress` line. Each of the four files ends with one `## Requirements-completed amended (Phase 57, 2026-11-04)` section.

Before: `gsd-tools audit-open` printed `Debug Sessions (3 open)`. After: it prints only the five pending todos and no debug-sessions section; the plan's verify command (`! audit-open | grep 'Debug Sessions ([1-9]'`) passed.

WINDOWS rows confirmed `fixed` via `gsd-tools windows status` before the `resolved:` lines were written: 74 (vendor token usage), 75 (OpenAI `max_completion_tokens`), 76 (Kimi fallback profile). `find .planning/debug -maxdepth 1 -name '*.md'` prints nothing; the three sessions are git renames into `.planning/debug/resolved/`.

## Handed back

Nothing. RUSTSEC-2026-0429 was the only new advisory `make security` named (EA-57-04), so no other advisory was found and no unreviewed row was added.

## Deviations from Plan

### Checkpoints

- The Task 1 tracer gate was orchestrator-approved (unattended session), not owner-approved. The plan is `autonomous: true`; the orchestrator re-ran `./scripts/check-advisory-register.sh` (exit 0) and confirmed commit `ea72092cc` touches exactly SECURITY-EXCEPTIONS.md, .cargo/audit.toml and deny.toml.

### Auto-fixed Issues

None. The plan's `cargo tree -i zip -e build` invocation does not resolve (noted under Compensating control); the working invocation was recorded instead. This is a measurement-command correction, not a change of outcome.

### Scope notes

- The phrase "not the fifteen the corpus's own record remembers" in the register's sixteen-suppressions paragraph was left alone, as a historical statement about the corpus (the plan listed only the heading and the "Eight vulnerability-class" sentence).
- The orchestrator's pre-existing uncommitted `.planning/STATE.md` "phase begin" edit is carried into the metadata commit, as instructed.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or trust-boundary schema was introduced; no dependency or lockfile change.

## Self-Check: PASSED

- FOUND: `ea72092cc`, `10b8282ea`, `33b399b8e` on `claude/laughing-dirac-e0h2ax`.
- FOUND: `.planning/debug/resolved/kimi-fallback-profile-stale.md`, `openai-max-completion-tokens.md`, `vendor-token-usage-fidelity.md`; no `.md` directly under `.planning/debug/`.
- FOUND: `id = "RUSTSEC-2026-0429"` in SECURITY-EXCEPTIONS.md, `"RUSTSEC-2026-0429"` in .cargo/audit.toml and deny.toml.
