# Phase 29 Acceptance Audit — Pointer

The program acceptance audit lives in the corpus, not the phase directory, because the corpus
(`.project/v0.10.0/`) is the program's source of truth and is not archived per milestone the way
`.planning/phases/` is (D-10, `29-CONTEXT.md`).

**Corpus document:** `.project/v0.10.0/09-program-acceptance-audit.md`

Ten `##` sections mirror doc-08's ten-step verification protocol
(`.project/v0.10.0/08-traceability-matrix.md` lines 98-108). Sections 1-5 are filled by plan 29-04;
sections 6-9 by plan 29-07; section 10 by plan 29-09.

**Overall verdict:** PASS with findings. All ten sections complete: sections 1-5 (plan 29-04),
6-9 (plan 29-07), 10 (plan 29-09) — every section carries a `PASS` or `PASS with findings` verdict,
zero `pending`. Two carried, non-blocking items remain outside this audit's own fix scope: the
Phase 28 tracing-overhead deviation (§10's "Accepted deviation" section, ACCEPTED per maintainer
sign-off) and the pre-existing 72-warning `cargo doc --workspace --no-deps` condition (§8,
`29-CI-EVIDENCE.md` row 16 — not a Phase 29 blocker per SHIP-04's own requirement text). The seven
maintainer sign-off checkboxes (§10's "Maintainer sign-off" section) remain unticked, as designed —
they are closed by a human at the phase's UAT / `/gsd-verify-work` step, never by this audit.

**Re-sealed on `69500c9b51a37f11215037c49318d76ea017dab3`, 2026-09-16.** Phases 31, 32 and 33
changed public API after this audit's ten sections were sealed above, so Phase 33 (COMM-04)
re-ran the full release-gate list on its own final commit and appended the result as a new
`## 11. Re-seal after Phases 30-33 (Phase 33, COMM-04)` section to the corpus document — this
pointer's ten-section scope is otherwise unchanged. Full verbatim evidence lives in
`.planning/phases/33-commissary-in-tree-adoption/33-CI-EVIDENCE.md`. This is the precondition
ADR-0051 sets for cutting the `v0.10.0` tag; §11 adds one further unticked, human-only sign-off box
for that decision, alongside — never in place of — the seven boxes named above.

**Re-sealed on `522ab1d4c4c4b5a62a8bbbbc5e234b0a29edbadd`, 2026-09-18.** Phases 34, 35, 36 and
36.1 landed after §11 above, and Phase 37 (SHIP-05) re-ran the full release-gate list for the
release itself — not merely for releasability — on that local head, appending the result as
`## 12. Re-seal for v0.10.0 release (Phase 37, SHIP-05)` in the corpus document. This pointer's
ten-section scope is otherwise unchanged. Full verbatim evidence lives in
`.planning/phases/37-v0-10-0-crate-release/37-CI-EVIDENCE.md`. Section 12 adds **no** new
sign-off box: the human-only box for cutting the tag is §11's own, still unticked, and it is
closed by the maintainer at the phase's UAT / checkpoint step, never by an agent.

**Re-sealed on `fbdefec9c13f5d28d2127aa2da6da990a5285482`, 2026-09-21.** Tag `v0.10.0` (cut on
merge commit `1d4a9724`) published only 3 of 12 crates and failed deterministically at
`paladin-battalion` — a tag that cannot complete forward, since `release.yml` reads manifests and
the `CRATES` order from the (immovable) tag ref. Phase 37.1 (SHIP-06) fixed the
`paladin-battalion` publish-order defect and the `create-or-reuse-release.sh` EPIPE race, added a
new publish-order gate proven both red on the real tagged tree and green on the fix, bumped all
fourteen manifests `0.10.0` → `0.10.1`, and re-ran the full release-gate list — plus the new
eighth row for the publish-order gate — on that corrected head, appending the result as
`## 13. Re-seal for the v0.10.1 release (Phase 37.1, SHIP-06)` in the corpus document. This
pointer's ten-section scope is otherwise unchanged. Full verbatim evidence lives in
`.planning/phases/37.1-v0-10-1-patch-release/37.1-CI-EVIDENCE.md`. **Unlike §12, Section 13 mints
its own new, fresh sign-off box** — "The `v0.10.1` tag may be cut" — rather than reusing §11's:
per D-09, §11's box belongs to a version (`v0.10.0`) that was tagged but only partially published,
and re-pointing or ticking it for `v0.10.1` would overwrite that history. §13's box is closed by
the maintainer alone, at this phase's own sign-off checkpoint, never by an agent.

**Re-sealed a second time on `d2f1a81131fa8d504065cceed412909c06143b95`, 2026-09-21.** The
maintainer ticked §13's box on commit `529e7078`, and then the required `API Surface Tracking`
check went red on that same commit: CI's toolchain install for that job was an unpinned, floating
`nightly`, and the 2026-09-21 nightly rendered derived return types differently from the nightly
§13's own PR run used — a rendering-only diff with zero public items actually added, removed or
changed. The maintainer's reply, "Pin it.", authorized returning to the build wave under D-08; the
fix (`PUBLIC_API_TOOLCHAIN: nightly-2026-09-20`, pinned in `ci.yml` and
`scripts/extract-public-api.sh`, with a regression harness) landed at `d2f1a811`, and Phase 37.1
re-ran the full release-gate list — all eight numbered rows plus the house sweep — on that
corrected head, appending the result as `## 14. Re-seal after the API-surface nightly pin (Phase
37.1, SHIP-06)` in the corpus document. This pointer's ten-section scope is otherwise unchanged.
Full verbatim evidence lives in `.planning/phases/37.1-v0-10-1-patch-release/37.1-CI-EVIDENCE.md`'s
"Re-seal #2 on d2f1a811" section. **Section 14 mints its own fresh, unticked sign-off box**,
distinct from §13's already-ticked one: §13's box stands as history and is not reused, re-pointed
or overwritten — a required check went red on that commit after the tick, so §14's box is the one
a fresh confirmation on the corrected head now waits on. It is closed by the maintainer alone, at
this phase's own sign-off checkpoint, never by an agent.
