---
phase: 46-docs-currency-hygiene
plan: 01
subsystem: docs
tags: [release, changelog, trusted-publishing, crates-io, curr-22]

requires:
  - phase: 45
    provides: v0.10.0 / v0.10.1 release evidence (tags, run 35659477719)
provides:
  - three release pages that state the twelve publishable crates
  - paladin-eval Trusted Publishing and Credential History rows with an honest evidence class
  - CHANGELOG 0.10.0 / 0.10.1 heading dates equal to the tag commit dates (root + eleven crates)
affects: [46-13, release docs, CHANGELOG]

tech-stack:
  added: []
  patterns: ["history amended by parenthetical, never restated (D-11)"]

key-files:
  created: []
  modified:
    - docs/src/appendix/release-automation.md
    - docs/src/appendix/release-recovery.md
    - docs/src/contributing/development-setup.md
    - CHANGELOG.md
    - crates/paladin-battalion/CHANGELOG.md
    - crates/paladin-content/CHANGELOG.md
    - crates/paladin-core/CHANGELOG.md
    - crates/paladin-eval/CHANGELOG.md
    - crates/paladin-herald/CHANGELOG.md
    - crates/paladin-llm/CHANGELOG.md
    - crates/paladin-memory/CHANGELOG.md
    - crates/paladin-notifications/CHANGELOG.md
    - crates/paladin-ports/CHANGELOG.md
    - crates/paladin-storage/CHANGELOG.md
    - crates/paladin-web/CHANGELOG.md

key-decisions:
  - "paladin-eval publishes after the leaf tier (depends on battalion, llm, storage) and before paladin-ai; tree outranks research F11 (D-00a)"
  - "paladin-eval trust link date is 'on or before 2026-09-21', evidence class 'proven by publish' (run 35659477719, trustpub_data), not operator-reported (D-12)"
  - "Crate changelog heading dates corrected alongside the root (RESEARCH Open Question 1 taken)"
  - "The four 'eleven-crate' code comments are left as written (RESEARCH Open Question 2 declined)"

requirements-completed: [CURR-22]

duration: 25min
completed: 2026-10-24
status: complete
---

# Phase 46 Plan 01: Release-doc currency Summary

**The three release pages now state the twelve publishable crates, `paladin-eval` has Trusted Publishing and Credential History rows proven by its own OIDC publish, and every `[0.10.0]` / `[0.10.1]` CHANGELOG heading carries its tag commit date.**

## Performance

- **Tasks:** 3 (1 tracer, 2 auto)
- **Files modified:** 15

## Accomplishments

- Task 1 (tracer): `release-automation.md` states the twelve-crate publish order with `paladin-eval` in its own tier, the eleven `crates/*` packages and changelogs, the `paladin-eval` trust row ("linked (proven by publish)"), the first-OIDC-publish Credential History row, and "at the time" parentheticals on the historical rc rows.
- Task 2: `development-setup.md` and `release-recovery.md` carry "twelve publishable crates"; the changelog counts read eleven crate files beside the root, and twelve files in total.
- Task 3: `[0.10.0]` is dated 2026-09-18 and `[0.10.1]` 2026-09-21 in the root and all eleven crate changelogs; an `[Unreleased]` Fixed entry records it.

## Task Commits

| Task | Name | Commit |
| ---- | ---- | ------ |
| 1 | Tracer: release-automation appendix twelve-crate set | fd9f2f6c |
| 2 | development-setup and release-recovery name the twelve publishable crates | 477ea740 |
| 3 | CHANGELOG 0.10.0 / 0.10.1 heading dates equal the tag dates | aa528781 |

## Evidence

- `git log -1 --format='%H %cs %ci' v0.10.0` -> `1d4a9724cc219b85856a23012543458d62559e47 2026-09-18 2026-09-18 17:21:45 -0400`
- `git log -1 --format='%H %cs %ci' v0.10.1` -> `f7dae2676580786e9541fefee8f787623b46c70f 2026-09-21 2026-09-21 16:03:34 -0400`
- `gh run view 35659477719 --json createdAt,conclusion` -> `{"conclusion":"success","createdAt":"2026-09-21T21:50:35Z"}`

## Verification

- Task verify commands all pass; the changelog loop passes for all twelve files.
- `make check-doc-config`, `make check-changelogs`, `make check-gates`, `make check-migration-allowlist` exit 0; `cargo fmt --check` clean.
- `cargo test --test legacy_removal_guard --test entity_rename_guard --test treasurer_vocabulary_guard`: 3 + 8 + 3 tests pass. (`--offline` failed because `webpki-roots v1.0.10` is not in the local cache; the same command run online passed.)
- `twelve publishable crates` count: release-automation 2, release-recovery 1, development-setup 1.
- Credential-handling review: no credential, token value or secret text was added; only a public run id and commit sha are cited.
- `make clean-code` and `make api-surface` were not run: the plan changes docs and changelogs only, no Rust source.

## Deviations from Plan

None in substance. The release-recovery line numbers had drifted from the plan (the crate-changelog sentence sat at 223-224 as planned, but the file's line 40 needed rewrapping to stay within 100 columns after the longer phrase; the second line of that paragraph was rewrapped, so the diff touches 7 lines rather than the 6 the acceptance criterion allows). The extra line is wrapping only.

## Noticed and left as written (RESEARCH Open Question 2)

Dated "eleven-crate" statements about the revision that wrote them, outside the three named pages:
- `.github/workflows/release.yml` lines 587, 597, 642
- `scripts/finalize-release-body.sh` line 243
- `scripts/publish-crates.sh` line 114
- `scripts/check-deprecations.sh` (named by the plan; not matched by the `eleven-crate` grep above, so confirm if it matters)

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

Commits fd9f2f6c, 477ea740 and aa528781 are present on claude/laughing-dirac-e0h2ax; all modified files exist.
