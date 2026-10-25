# v0.11.0 Re-seal and Tag Sign-off (Phase 47)

v0.11.0 is the Treasurer release: spend governance (a price table, a cost ledger, per-tenant and per-key
allowances that can halt a run mid-flight), Cadence rate pacing after a provider 429, provider parity across the
nine adapters, and reasoning effort, with four clean breaks and no deprecation cycle (`MIGRATION.md` section 10).
This section re-seals the release-gate set at `0.11.0` on the candidate final head, in the eight-row shape every
re-seal since Phase 29 has used, and mints the sign-off box for the `v0.11.0` tag. It covers the release that
actually publishes, not merely releasability: it is the local half of the pre-merge wave, and the CI twins and the
coverage figure are added from the pull request run (plan 47-06).

**Head SHA this section's gates were measured on:** `d82d67cac926412a21cccd72278fe8fe9df672c0` (`SHA-R`), the tip of
`claude/laughing-dirac-e0h2ax` at this plan's (47-04) start, confirmed with `git rev-parse HEAD` before the first
gate ran, with `git status --porcelain` empty at that same moment. The code head is `SHA-H`
`c945684647c4f9c78fdde2fbe2fc8fc2e8806106` (the OpenAPI baseline regeneration, the last of the three build-wave
`chore(release)` commits); `git diff --name-only c945684647c4f9c78fdde2fbe2fc8fc2e8806106
d82d67cac926412a21cccd72278fe8fe9df672c0 -- . ':!.planning' ':!.project'` printed nothing, so **the measured tree is
identical, outside `.planning/` and `.project/`, to the tree the build wave produced**. Later commits made by this
plan itself (this section and `47-CI-EVIDENCE.md`) are documentation-only under `.planning/`, and no gate below reads
them. From the first gate onward D-00j was in force: any red row halts the phase with nothing fixed.

| # | Gate | Command | SHA | Result | Evidence |
|---|------|---------|-----|--------|----------|
| 1 | `MIGRATION.md` no-TBD marker + sections 9.2 and 10.2 registers vs the semver-checks allowlist set-equality | `grep -c TBD MIGRATION.md` then `make check-migration-allowlist` | `d82d67ca` | `COUNT=0`; the script reads section 9.2 and section 10.2 together: 56 `crate\|type` pairs on the `MIGRATION.md` side (15 in section 9.2, 47 in section 10.2, 6 in both) and 56 in `.cargo/semver-checks-allowlist.toml`, set-equal in both directions, and every crate-wide `allow` lint is mirrored by an allowlist entry. **PASS** | `47-CI-EVIDENCE.md` Local sweep rows 49-50. **No PR run reconfirms this row**: `check-migration-allowlist` is not wired into `ci.yml` (D-14); this local row is the only proof for this gate. |
| 2 | `v0_9_config_boot` frozen compat target | `cargo test --features web-server --test v0_9_config_boot` | `d82d67ca` | not yet run | `47-CI-EVIDENCE.md` Local sweep (Task 2) |
| 3 | OpenAPI golden diff (path-restricted) | `cargo test -p paladin-web --test openapi_golden_v0_9` | `d82d67ca` | not yet run | `47-CI-EVIDENCE.md` Local sweep (Task 2) |
| 4 | `cargo semver-checks` for the eleven baselined crates vs `0.9.0` | `cargo semver-checks check-release --package <pkg> --default-features --baseline-version 0.9.0`, once per crate, list derived live from `cargo metadata` minus `paladin-eval` | `d82d67ca` | not yet run | `47-CI-EVIDENCE.md` Local sweep (Task 2) |
| 5 | MSRV floor, toolchain 1.88 | `env RUSTUP_TOOLCHAIN=1.88 cargo check --workspace --all-features --all-targets` | `d82d67ca` | not yet run | `47-CI-EVIDENCE.md` Local sweep (Task 2) |
| 6 | `make publish-dry-run`, twelve crates, dependency order | `make publish-dry-run` | `d82d67ca` | not yet run | `47-CI-EVIDENCE.md` Local sweep (Task 2) |
| 7 | `CHANGELOG.md` `[0.11.0]` completeness and release-body size | `make check-gates` (nine offline guards) plus the R7 hard-assertion greps and the B-1 body-size assertion (`scripts/extract-changelog-section.sh --version 0.11.0`, at most 110,000 bytes) | `d82d67ca` | `make check-gates` exit 0, all nine guards OK individually; in each of the 12 changelogs `## [Unreleased]` count `0`, `## [0.11.0] - 2026-10-25` count `1` and it is the first `## [` heading (R7=a); release body **108926 bytes** (limit 110,000). **PASS** | `47-CI-EVIDENCE.md` Local sweep rows 51-53. CI twin to be named from the PR run in plan 47-06. |
| 8 | Publish-order gate, invoked through the `make` target CI also invokes | `make check-publish-order` (= `./scripts/check-publish-order.sh`) | `d82d67ca` | `✅ OK: 12 crate(s) checked in publish order; every versioned workspace dependency (normal, dev and build) resolves to an earlier CRATES position, and CRATES exactly matches the cargo-metadata publishable set.` **PASS** | `47-CI-EVIDENCE.md` Local sweep row 54. CI twin: `License & Dependency Policy` step `Check publish order`, to be filled from the PR run in plan 47-06. |

*(Closing paragraphs: written by plan 47-04 Task 3.)*

**Findings:**

*(Findings list: written by plan 47-04 Task 3.)*

**The `v0.11.0` tag may be cut once a human reads this section and the linked evidence:**

- [ ] **The `v0.11.0` tag may be cut** — evidence: this section in full, plus `.planning/phases/47-v0-11-0-crate-release/47-CI-EVIDENCE.md`'s complete Local sweep table and its CI-run table (including the `coverage` job this sandbox cannot measure). Unticked, as designed — this box belongs to this release alone. It is closed by a human — the maintainer, and nobody else — at this phase's own sign-off checkpoint, never by this audit or any agent, under any mode (Phase 29 D-17, Phase 37 D-00a, this phase's D-00a).

**Restart rule.** If a red gate forces a return to the build wave, this box is not edited; a new dated section at the new head carries the box the merge waits on.
