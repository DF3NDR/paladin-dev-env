---
phase: 58-crate-release
plan: 04
subsystem: release-prep
tags: [changelog, finalize, consumer-surface-muster, cadence, devcontainer, cargo-llvm-cov, ship-08, ship-09, ship-10]

requires:
  - phase: 58-crate-release
    provides: plan 58-01 CI-health baseline and the registry probe
provides:
  - "`## [Unreleased]` anchors with log-derived entries in the paladin-battalion, paladin-eval and paladin-herald changelogs, so bump commit 2 can stamp all twelve changelogs"
  - "A throwaway-worktree proof that finalize-crate-changelogs and check-release-consistency are satisfied for 12 packages at 0.11.1"
  - "Three Cadence import pins (build_cadence, with_cadence, CadenceConfig::default()) in tests/consumer_surface_muster.rs, the gate the web3sec hand-off points at"
  - "cargo-llvm-cov 0.8.7 (--locked) installed in .devcontainer/Dockerfile.dev, the same pin as CI"
affects: [58-05, 58-06, 58-07, 58-14]

tech-stack:
  added: []
  patterns:
    - "A release-prep claim is proven in a detached scratch worktree that is removed and pruned, so the real tree is never bumped"
    - "A consumer import is pinned by fn-pointer coercion in a never-called function, compile time only"

key-files:
  created: []
  modified:
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-eval/CHANGELOG.md
    - crates/paladin-herald/CHANGELOG.md
    - tests/consumer_surface_muster.rs
    - .devcontainer/Dockerfile.dev

key-decisions:
  - "Entries cite the commit they derive from by short SHA and phase requirement id; the herald paragraph is the no-source-change line only because its v0.11.0..HEAD log is empty"
  - "The eval entry sits under ### Changed (rustdoc, schema description strings and a guide row; no public item added or changed)"
  - "cargo-llvm-cov is installed beside the cargo-semver-checks pin, after the llvm-tools-preview component it needs"

patterns-established:
  - "The Cadence pin names with_cadence through its full signature, so a changed parameter list turns the muster red"

requirements-completed: []
requirements-progress: "SHIP-08, SHIP-09 and SHIP-10 are earned by plan 58-14 after the tag; this plan only prepares the anchors, the muster pin and the devcontainer install"

duration: not measured
completed: 2026-11-05
status: complete

coverage:
  - id: D1
    description: "Each of battalion, eval and herald changelogs carries exactly one [Unreleased] anchor above [0.11.0], with entries drawn from its own git log, additions only"
    requirement: SHIP-08
    verification:
      - kind: other
        ref: "per-file grep count of '## [Unreleased]' is 1 and precedes '## [0.11.0] - 2026-10-26'; git diff f7a6ee80a~1 f7a6ee80a removed-line count is 0"
        status: pass
  - id: D2
    description: "With the anchors in place a bump to 0.11.1 finalizes all twelve changelogs and passes the release-consistency gate"
    requirement: SHIP-08
    verification:
      - kind: other
        ref: "throwaway worktree: finalize-crate-changelogs.sh --version 0.11.1 -> OK: 12 publishable package(s); check-release-consistency.sh --tag v0.11.1 -> OK: 12 publishable package(s)"
        status: pass
  - id: D3
    description: "The muster pins the three Cadence import paths without changing the public surface"
    requirement: SHIP-10
    verification:
      - kind: other
        ref: "make check-consumer-surface -> 1 passed; cargo clippy --test consumer_surface_muster --features llm-anthropic,llm-deepseek -- -D warnings clean; make api-surface -> unchanged (4331 items)"
        status: pass
  - id: D4
    description: "The devcontainer image installs cargo-llvm-cov at the CI pin"
    requirement: SHIP-10
    verification:
      - kind: other
        ref: "grep of the exact RUN line in Dockerfile.dev and of cargo-llvm-cov@0.8.7 in ci.yml; Dockerfile and README.md unchanged. The image build is not runnable here and no coverage figure is claimed"
        status: pass
---

# Phase 58 Plan 04: Changelog anchors, Cadence muster pin and devcontainer coverage tool Summary

Three pre-bump items land as three commits: log-derived `[Unreleased]` anchors in the three anchor-less crate changelogs (proven by a throwaway finalize run across all twelve publishable packages), the Cadence import pins web3sec takes in the consumer-surface muster, and the pinned `cargo-llvm-cov` 0.8.7 install in the devcontainer image.

## Commits

| Task | Commit | Message |
| ---- | ------ | ------- |
| 1 | `f7a6ee80a` | `docs(58-04): add log-derived [Unreleased] anchors to the battalion, eval and herald changelogs (D-15)` |
| 2 | `364f08607` | `test(58-04): pin the three Cadence import paths web3sec takes in the consumer-surface muster` |
| 3 | `86213aee2` | `chore(58-04): install the pinned cargo-llvm-cov 0.8.7 in the devcontainer image` |

## Task 1: anchors and entries (tracer)

`git log v0.11.0..HEAD -- crates/<crate>/` worked in this clone (the `v0.11.0` tag is local).

| Crate | Commits in `v0.11.0..HEAD` | Entry written |
| ----- | -------------------------- | ------------- |
| paladin-battalion | `2d3cf119a` (CIREL-02, removes the crate-wide `cargo-semver-checks` allows from `Cargo.toml`), `79cd06bf7` (TESTS-06, doc lines in `commander.rs` tests) | `### Changed`: one entry per commit; each states that no public item changed |
| paladin-eval | `f70f03deb` (FIXES-04, rustdoc, schema description strings, guide row for the cancelled-run gap) | `### Changed`: one entry, documentation only |
| paladin-herald | none | the "Patch release carried by the workspace-wide version bump ... No source change in this crate" paragraph |

Additions only: 28 lines added, 0 removed; no `[0.11.0]` or older section was touched (P-04-2). No stock line and no root-changelog line was copied (P-04-1).

### Throwaway finalize proof (D-00q)

A detached worktree of `f7a6ee80a` was created in the session scratchpad, then, inside it:

| Step | Result line |
| ---- | ----------- |
| `cargo release version 0.11.1 --execute --no-confirm --workspace` | 13 manifests upgraded `0.11.0` to `0.11.1` (cargo-release 1.1.7) |
| `./scripts/finalize-crate-changelogs.sh --version 0.11.1` | `OK: 12 publishable package(s) checked for version '0.11.1' (date 2026-11-05)`, all 12 `finalized` |
| `make check-changelogs` | `11 publishable crate(s) checked, all have a CHANGELOG.md` |
| `./scripts/check-release-consistency.sh --tag v0.11.1` | `OK: 12 publishable package(s) checked, all match tag version '0.11.1' with a changelog section for it` (the D-10 CI-conclusion clause was not checked: no `--sha`; this is a local pass, not a full-gate pass) |

In the three changed crates the finalized files read `[Unreleased]`, then `[0.11.1] - 2026-11-05`, then `[0.11.0]`. The worktree was then removed with `git worktree remove --force` and pruned; `git worktree list` shows only the main tree, and the real workspace still reads `0.11.0`. The finalize run was not committed anywhere.

### Sanity table for the other eight crates and the root (D-15, 47-REVIEW WR-01)

Counts read from `git log v0.11.0..HEAD -- <path>` and the lines of the `## [Unreleased]` section. Commits outnumber entries because many commits are test-only, plan-docs or gate work that a changelog does not carry; an entry is a bulleted item.

| Package | Commits | `[Unreleased]` entries | Contradiction |
| ------- | ------- | ---------------------- | ------------- |
| paladin-content | 5 | 3 | no |
| paladin-core | 13 | 4 | no |
| paladin-llm | 8 | 2 | no |
| paladin-memory | 1 | 1 | no |
| paladin-notifications | 3 | 1 | no |
| paladin-ports | 5 | 1 | no |
| paladin-storage | 11 | 6 | no |
| paladin-web | 19 | 9 | no |
| root (`src/`) | 36 | 28 | no |

The `paladin-memory` (line 12) and `paladin-content` (line 12) "Correction to the `[0.11.0]` entry" lines are present. Checked in detail for `paladin-core`, the one crate where entries trail commits most: the Phase 55 base-layer commits (`4eb02f0b9`, `6e459a5bb`, `36d658b73`) are covered by its single `FieldVersionService` / `CollectionVersionService` entry, and `290ec10b4` (the `PlanningService` expected-output change, with a 3-line change in core's `planning.rs`) is covered by the root entry at `CHANGELOG.md:26`; neither contradicts anything. No file outside this task's three was edited.

## Task 2: Cadence muster pin

`tests/consumer_surface_muster.rs` gains `use paladin::config::treasurer::CadenceConfig;`, `use paladin::infrastructure::cadence::{CadenceWiring, build_cadence, with_cadence};` and one never-called `cadence_consumer_imports_pin`, which coerces `build_cadence` to `fn(&CadenceConfig) -> Result<Option<CadenceWiring>, String>`, coerces `with_cadence` to `fn(Arc<dyn LlmPort>, Option<&CadenceWiring>) -> Arc<dyn LlmPort>` and returns `CadenceConfig::default()`. Nothing executes; its doc comment names the folded todo and the web3sec return-report section 4.

| Check | Result |
| ----- | ------ |
| `cargo fmt --check` | clean |
| `make check-consumer-surface` | `the_consumer_surface_still_resolves ... ok`, 1 passed |
| `cargo clippy --test consumer_surface_muster --features llm-anthropic,llm-deepseek -- -D warnings` | clean |
| `make api-surface` | `API surface unchanged` (4331 items); `.project/current-exports.txt` untouched |
| commit file list | `tests/consumer_surface_muster.rs` only |

## Task 3: devcontainer cargo-llvm-cov

`.devcontainer/Dockerfile.dev` gains `RUN cargo install --locked --version 0.8.7 cargo-llvm-cov` with a three-line comment, placed after the `cargo-semver-checks` pin (and so after the `llvm-tools-preview` component it needs). It equals `tool: cargo-llvm-cov@0.8.7` at `.github/workflows/ci.yml:1489`. `.devcontainer/Dockerfile` (the `vhs` pin) and `.devcontainer/README.md` are unchanged. No coverage figure is claimed (D-00e); the image build is the proof and is not runnable in this sandbox.

### Finding for the plan 58-05 sweep (recorded, not fixed here)

`.devcontainer/Dockerfile.dev` pins `cargo-release` at `--version 1.1.2`, while the release process since plan 47-01 runs `cargo-release 1.1.7` (`release.toml:49`; this sandbox, where the throwaway proof ran, has 1.1.7). A rebuilt devcontainer would therefore bump with an older cargo-release than the one the release evidence was produced with. The same drift is already listed as C7 in `.project/v0.11.1/01-carried-gaps-inventory.md` for `release.toml:49`; the Dockerfile line is not named there.

## Deviations from Plan

None. The plan executed as written.

Notes, not deviations:
- The throwaway proof ran on the committed anchors (`f7a6ee80a`) rather than before the commit, because a worktree of `HEAD` only sees committed text. Had it failed, the commit would have been amended before any later commit.
- The plan's `<verify>` for Tasks 1 and 3 compares against `HEAD~1`, which only holds while that task's commit is `HEAD`; each was checked at its own commit.
- Scratchpad logs from this plan are `58-04-muster.log`, `58-04-clippy.log` and `58-04-api.log`.

## Known Stubs

None. No stub, placeholder or unwired data was introduced. The two-line herald paragraph is the plan's prescribed text, not a placeholder.

## Threat Flags

None. No new endpoint, auth path, file access or trust-boundary schema change. The `cargo install` in the devcontainer is the plan's T-58-SC (exact version, `--locked`, equal to the CI pin).

## Issues Encountered

None blocking. Free disk stayed at 5-6 GB throughout; no `cargo clean` was needed.

## Self-Check: PASSED

- `f7a6ee80a`, `364f08607` and `86213aee2` exist in `git log`.
- All five modified files exist; the three changelogs each carry one `## [Unreleased]` above `## [0.11.0] - 2026-10-26`; additions only.
- `git worktree list` lists only the main tree; the real workspace version reads `0.11.0`.
