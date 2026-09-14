# ADR-0051: Token-economy phases land as clean breaks inside the untagged v0.10.0

## Status

Accepted

**Date:** 2026-09-14

## Context

v0.10.0 corpus rule **X-03** (`.project/v0.10.0/00-program-overview.md` line 44) requires
existing public APIs to keep compiling and behaving identically, permits `#[deprecated]` but
forbids removals, and declares any other behavioral change a stop-and-flag event rather than a
judgment call. Milestone 13's own overview takes the opposite position for the token-economy
work: `.project/Milestone_13-Token-Economy/overview/Milestone-13_Token-Economy.md` §5 item 1
states that clean break is preferred over shims, because the framework is pre-1.0 and has
exactly one consumer — the downstream Web3 Security Paladin app — which pins its own submodule
pointer and adopts the whole milestone's breaking changes in one coordinated step rather than
cherry-picking. Carrying deprecation shims pre-1.0 was judged to ship cruft while the API is
still being shaped.

Taken at face value, these two documents disagree, and Phases 31-33 (which build on this
phase) proceed on the assumption that the clean-break policy governs their own work. Today that
assumption rests on an operator conversation and a milestone-overview paragraph, with no
citable record narrowing X-03 for the phases that will actually remove public API. This ADR is
that record.

## Decision

**X-03 is superseded for Phases 31, 32 and 33 only**, on the operator's 2026-09-14 decision. The
exception extends to no other phase, no other milestone, and no other public API anywhere in
v0.10.0 — every other in-tree public API continues to be governed by X-03 exactly as written.

Two named breaks the milestone already anticipates:

- The token-carrier change (Phase 31 / Milestone 13 Epic 2 — the lossless `TokenUsage`
  prompt/completion/cache/reasoning split).
- Dropping `Commissary::new`'s `is_exact_counter` argument (Phase 32 / Milestone 13 Epic 3).

**What still applies.** Every break in Phases 31-33 still gets a `MIGRATION.md` §9.2 row **and**
a `cargo semver-checks` allowlist row, row-level set-equal in both directions per Phase 29 D-04
— **as documentation for the downstream refactor, never as a compatibility shim.** X-10 (semver
hygiene — the rule that "additive" is a semantic promise that several Rust edits break in
practice) is untouched by this supersession and still governs which additive-looking edits in
those phases count as breaking. The `0.10.0` tag is cut only after Phase 33 re-seals the Phase
29 release gates (COMM-04); `0.10.0` is bumped on the feature branch with no tag until then
(Phase 29 D-18/D-21).

**This phase's own position.** Phase 30 changes no public Rust API, so it registers **zero**
`MIGRATION.md` §9.2 rows and **zero** `cargo semver-checks` allowlist rows. This is the boundary
that keeps the exception from reading as retroactive — the supersession applies going forward,
to the three named phases, not backward to this one.

**Version identity.** Write **v0.10.0** everywhere. Milestone 13 overview §5 item 2's own
wording, which speaks of cutting a later minor release once the downstream pointer bump is
coordinated, is superseded by the operator's instruction that this work ships inside the current
untagged v0.10.0 — noted here in one sentence so a reader comparing the two documents is not
left guessing which one currently governs.

## Considered Options

- **Deprecation shims across all three phases** (rejected — ships cruft while the API is still being shaped, pre-1.0, and the single downstream consumer adopts the whole milestone in lockstep anyway, so a shim protects no one who would actually use it).
- **Defer the breaks to a later minor release after tagging v0.10.0** (rejected — the operator's version-identity decision puts this work inside the current untagged v0.10.0, not a subsequent release).
- **Clean break with migration-register documentation** (chosen — matches Milestone 13 overview §5's own stated policy, keeps a citable audit trail via `MIGRATION.md` §9.2 and the semver allowlist without functioning as a compatibility shim).
- **A blanket X-03 waiver for all of v0.10.0** (rejected — far broader than the operator granted; every other public API in v0.10.0 stays governed by X-03 exactly as written).

## Code Locations

- `.project/v0.10.0/00-program-overview.md` — X-03 (backward compatibility, the rule narrowed here) and X-10 (semver hygiene, unaffected and still governing).
- `.project/Milestone_13-Token-Economy/overview/Milestone-13_Token-Economy.md` §5 — the clean-break policy and the coordinated-pointer-bump constraint this ADR adopts and cites.
- `MIGRATION.md` §9.2 — the register Phases 31-33 write rows into; this phase adds no row.
- The `cargo semver-checks` allowlist (CI `semver` job) — the per-item allowlist Phases 31-33 extend; this phase makes no change to it.
- `.planning/ROADMAP.md` — the Phase 31, 32 and 33 entries this supersession scopes to.

## Code Conformance

conforms

This phase makes no public-API change; the ADR records a forward-looking policy exception for
later phases rather than instructing any code change of its own.

## Downstream Consumers

- **Phase 31** — the token-carrier change, the first of the two named breaks.
- **Phase 32** — the `Commissary::new` `is_exact_counter` removal, the second named break.
- **Phase 33** — the Phase 29 release-gate re-seal (COMM-04) that gates cutting the `0.10.0` tag.
- **The downstream Web3 Security Paladin app's coordinated refactor** — the single consumer that adopts all three phases' breaking changes in one lockstep submodule-pointer bump.
