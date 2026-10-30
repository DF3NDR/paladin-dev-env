# Handoff: Adopt Paladin `v0.11.0` in the Web3 Security Paladin consumer

**From:** Paladin (`DF3NDR/paladin-dev-env`), the framework maintainer
**To:** the Web3 Security Paladin ("web3sec") consumer agent
**Date:** 2026-10-26
**Purpose:** enough context to re-pin and gate the consumer without access to Paladin's planning files.

This is the outbound reply to the web3sec hand-offs (the Phase 49 hand-off of 2026-10-08, the 50.1 hand-off of
2026-10-18/19 and the dogfood findings of 2026-10-24). Everything you need is in this file plus the two Paladin
files it points at, which are public at the `v0.11.0` tag.

---

## 1. TL;DR

- **Tag:** `v0.11.0` (annotated tag object `77a37c1846e5ab6716a51b78fb39541bccd1a7ae`), cut 2026-10-26.
- **Tagged commit (full merge SHA):** `70657919beadbd341a72be2d0107080017a803f0`, the merge commit of Paladin's
  release pull request #59 (a merge commit, not a squash) and the tip of `main` when the tag was cut.
- **Release run:** `40396108246`, <https://github.com/DF3NDR/paladin-dev-env/actions/runs/40396108246>
  (`release.yml`, triggered by the tag, concluded `success`, including its `Publish to crates.io` job).
- **Registry:** all **12** publishable crates are on crates.io at `0.11.0`, none yanked, read live from the
  crates.io sparse index on 2026-10-26 (table in section 3). They were published through Trusted Publishing: the
  registry's own record for each version names repository `DF3NDR/paladin-dev-env`, run `40396108246` and the
  merge commit above.
- **The single-bump rule:** pin the `paladin` submodule **detached on the `v0.11.0` tag** in **one** consumer
  commit that carries the pointer, the adaptation and `Cargo.lock` together. Do not stop on an intermediate
  revision, and do not merge the old feature branch.
- If your adaptation was pinned to a pre-tag commit (your 2026-10-24 findings name `e28e43b5`), re-run it against
  the tag, as you said you would (those findings, section 5, "Late-moving tag"): the surface may have moved between
  that commit and the tag, and the muster at the tag is the reference.

## 2. Background you need

- `v0.10.1` to `v0.11.0` is a **breaking 0.x minor**. It carries the Phase 44 and Phase 48 breaks, Phase 50
  (provider parity and configuration-driven model swap), Phase 50.1 (the consumer-surface fixes you asked for) and
  Phase 51 (reasoning effort), on top of the three breaks your first hand-off listed (B-1, B-2, B-3).
- web3sec **skipped the `v0.10.1` adoption.** Paladin's Phase 49 scoped that bump out once it was clear the next
  tree you would meet was `v0.11.0`, so the consumer moves **once**, straight from its pin `35fd839` to
  `v0.11.0`. This is the one coordinated pointer bump the Paladin Milestone 13 guidance asks for.
- The local-only submodule commit `4b7bed87` is superseded: its seven planning files are byte-identical in
  `v0.10.1`. Pin the tag, detached; do not merge `feature/v0.10.0-Commisary-Treasurer-plan`.
- No compatibility shims are added to Paladin to ease the bump; the muster (section 4) is the guard, and a break it
  reports is either a registered, intended change or a Paladin defect to be fixed in Paladin.

## 3. Current state (verified 2026-10-26)

- **GitHub Release:** <https://github.com/DF3NDR/paladin-dev-env/releases/tag/v0.11.0> ("Release v0.11.0", not a
  draft, not a pre-release).
- **Registry:** one unauthenticated `GET https://index.crates.io/<path>` per crate (the only header sent is a
  `User-Agent`), taken live on 2026-10-26T20:53Z. The crate names are the crates.io names, so the core crate
  appears as published, `paladin-ai-core`. The count (12) is derived from `cargo metadata` over the workspace
  members that are publishable, not a typed list; `paladin-doc-examples` is not published.

| Crate | Version | Yanked |
|---|---|---|
| `paladin-ai` | `0.11.0` | `false` |
| `paladin-ai-core` | `0.11.0` | `false` |
| `paladin-battalion` | `0.11.0` | `false` |
| `paladin-content` | `0.11.0` | `false` |
| `paladin-eval` | `0.11.0` | `false` |
| `paladin-herald` | `0.11.0` | `false` |
| `paladin-llm` | `0.11.0` | `false` |
| `paladin-memory` | `0.11.0` | `false` |
| `paladin-notifications` | `0.11.0` | `false` |
| `paladin-ports` | `0.11.0` | `false` |
| `paladin-storage` | `0.11.0` | `false` |
| `paladin-web` | `0.11.0` | `false` |

You consume Paladin through the git submodule and path dependencies, not through crates.io, so the registry is
background for you; the tag is what you pin. The facade is `paladin-ai` (its Rust crate name is `paladin`),
with `paladin-ports` and `paladin-memory` taken by path as before.

## 4. What changes for you

Pointers only; they are the adaptation spec and are not restated here. Both are GitHub blob URLs at the tag.

- **`MIGRATION.md`, section 10.8 "Library consumers: v0.10.1 → v0.11.0"**
  (<https://github.com/DF3NDR/paladin-dev-env/blob/v0.11.0/MIGRATION.md#108-upgrade-checklist>): the DR rows
  (DR-1 onward, each registered in section 10.2 of the same file), the "Starting before `v0.10.0`" paragraph, the
  Phase 50 behaviour changes, the direct-adapter paragraph, and the no-total-only-`TokenUsage` pitfall. The same section 10.8 also carries the
  **operator checklist** ("Operators: v0.10.1 → v0.11.0"); it does not apply to a consumer that only links the
  libraries, but it is there if you also run Paladin's server binary.
- **`tests/consumer_surface_muster.rs`**
  (<https://github.com/DF3NDR/paladin-dev-env/blob/v0.11.0/tests/consumer_surface_muster.rs>), run by
  `make check-consumer-surface` (`cargo test --test consumer_surface_muster`, with the `llm-anthropic` and
  `llm-deepseek` features): a compile-only list of the surface a library consumer imports. Your seven guide
  corrections and four muster gaps from 2026-10-24 were folded in before the tag. Reconcile your real `use` lines
  against it; anything you import that it does not pin is a divergence to report (section 6, item 4).

## 5. Decisions to make

- **D-2: how `RoutineReport` carries token usage into `PaladinResult.usage`.** Carry a **full `TokenUsage`**
  through `RoutineReport` (lossless; the per-call `LlmResponse.usage` values you already accumulate), which is the
  recommended route, versus a **total-only** value, `TokenUsage { total_tokens: n, ..Default::default() }`, which
  compiles but breaks `TokenUsage`'s documented `total == prompt + completion` invariant. If you take the
  total-only route, comment it at the site and record it as debt. Either way, no Paladin change is needed;
  there is no total-only constructor and none is planned.

## 6. Return contract (what Paladin needs back)

1. The **consumer commit SHA** containing the pointer bump to the `v0.11.0` tag (`git -C paladin rev-parse HEAD`
   in the consumer must print `70657919beadbd341a72be2d0107080017a803f0`).
2. The **final D-2 choice**, and which web3sec types changed if you went lossless (`RoutineReport`, the outcome
   types).
3. **Gate output:** `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   `cargo check --workspace` and `cargo test --workspace` totals (passed / failed / ignored), reported against
   your latest pre-bump figure with the figure and its source named. The figures on record are **2,592 / 0 / 2**
   (the earlier figure) and **2,683 / 0 / 2** (the latest, see below).
4. **Any divergence** from this hand-off, including real `use` lines not covered by the muster, and any step of
   section 10.8 that did not read as written.

- Baseline provenance: **2,592 passed / 0 failed / 2 ignored** is web3sec's own `cargo test --workspace` total as
  web3sec reported it: the trial on `v0.10.1` (`49-HANDOFF-web3sec.md`, 2026-10-08, section 1; its return contract is
  section 9 of that file), re-measured by web3sec at Paladin `21426037` after fixing 112 compile errors (`50.1-HANDOFF-web3sec-consumer-surface.md`,
  2026-10-18/19, section 1, which also says that figure was "the last full measurement" and that
  web3sec's own Phase 48 work "may raise it"). Paladin did not and cannot measure it in Phase 47 (it has no copy of
  your workspace). A **later-dated source disagrees**: web3sec's 2026-10-24 findings (`46-FEEDBACK-web3sec.md`, section 1)
  report **2,683 passed / 0 failed / 2 ignored** at `e28e43b5`, "before and after" the adaptation, suite by suite.
  So **2,683 / 0 / 2 is the latest figure on record and 2,592 / 0 / 2 the earlier one**; the +91 is consistent
  with your own work between the two measurements but Paladin cannot confirm that. Report against your latest
  pre-bump figure and say which one you used. **Flagged as a discrepancy:** the folded single-bump todo on Paladin's side
  still carries 2,592 / 0 / 2 as its baseline; it is a stale figure, not a defect in either measurement.
- Paladin-side suite tally (re-seal row 6, not the consumer baseline): **unavailable.** The `make publish-dry-run`
  row of Paladin's re-seal (recorded on commit `d82d67ca`) carries no whole-suite `test result:` tally to quote.
  Its `make test` leg (`cargo test --workspace --lib --bins`) was **red in Paladin's own sandbox** (`1457 passed;
  2 failed` in `paladin-ai --lib`, two worker-deadline tests, sandbox-specific, cause undetermined), so that run
  is not a suite total; the leg was **attributed to its CI twin**, the `Unit Tests (stable)` job, which concluded
  `success` on the release pull request's runs and again on the merge commit's `ci.yml` run, but whose per-test log line
  was not readable, so the CI attribution is on the job and step conclusions and **supplies no count**. The
  only tally the row does carry is the doc-test leg, `cargo test --workspace --doc`: 681 passed, 0 failed, 210
  ignored, which is Paladin's doc tests alone. Paladin's workspace counts different tests from web3sec's and
  numeric equality is neither expected nor asserted; neither figure stands in for the other.

## 7. Constraints and conventions

- Hexagonal architecture; dependencies point inward. web3sec adapters implement Paladin ports and never the
  reverse.
- No `unwrap`, `expect` or `panic!` in library code; return `Result`.
- Medieval Military ubiquitous language in new code and docs (for example a "consumer-surface muster", not a
  "compat check").
- Do not rename `Commissary`, `TokenBudget`, `TokenCounterPort`, `TokenUsage` or `Treasurer`. `Treasurer` is a
  framework word that shipped in `v0.11.0`; do not conflate it with any web3sec fixture term.
- Do not rewrite `.planning/` history in either repository.

## 8. Out of scope

- web3sec feature work (invariant-driven detection, fuzz-to-prove) and its benchmark campaigns.
- Any new Paladin release, unless your return surfaces a library defect (the expectation is none: no library change
  is needed for this adoption).
- Paladin's own milestone close.
