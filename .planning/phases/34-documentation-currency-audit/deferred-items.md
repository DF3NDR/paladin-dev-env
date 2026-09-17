# Phase 34 Deferred Items Register

Per D-19, this register holds findings this phase surfaced that are **neither** a documentation
gap (`MB-nn`, routed to Phase 35) **nor** a rustdoc/example gap (`RD-nn`/`EX-nn`, routed to
Phase 36). Nothing here is absorbed into the Phase 35 or 36 work lists by convenience — each entry
stays a pointer only. `34-AUDIT.md` §7 (assembled by plan 34-09) links back to this file.

## Plan 34-04, Task 2

1. **Docker-machine coverage walk (CONTEXT.md Folded Todos — the non-documentation remainder).**
   The original todo (2026-08-13, score 0.6) asked to "verify local `make coverage` reproduces
   CI's 82.39% figure." This phase's documentation slice is closed: `contributing/testing-guide.md`
   (§2 row 54, MB-36) is settled against the real `Makefile`/`scripts/coverage.sh`/`ci.yml`
   invocation, and the three-way command comparison is recorded in `34-AUDIT.md`'s "Coverage
   command comparison" subsection. What remains open is the actual end-to-end reproduction —
   running `make services-up` then `make coverage` on a real Docker-capable machine and confirming
   the local figure matches CI's. This devcontainer has no Docker, so this audit cannot perform
   that walk. Remains the maintainer's own item, unchanged, not owned by Phase 35 or 36.

## Plan 34-07, Task 2

1. **`scripts/check-public-api-examples.sh` scope drift and current RED result (RESEARCH.md
   Pitfall P-06, `34-AUDIT.md` §3 "Entry-point `# Examples`-heading gate" subsection).** The
   D-05/D-06 `# Examples`-heading rule was ratified against a frozen 76-item entry-point
   enumeration (`16-DOCS-03-ENTRY-POINTS.md`: 11 Builders + 35 `*Port` traits + 30 `*Service`
   structs, 100% compliant at Phase 16 close). The gate script re-derives its target set live from
   the tree on every run rather than reading that frozen file, and Phases 22-33 added
   `pub *Builder`/`*Port`/`*Service` items faster than anyone re-ran it: this audit's live run
   derives **101** entry points (a +25-item, ≈33% drift), of which **19** are MISSING a plural
   `# Examples` heading (0 SINGULAR), and the gate exits `1`. `grep -rn
   check-public-api-examples .github/workflows/*.yml Makefile` returns nothing — no CI job and no
   make target has ever run this script, so nothing would have caught the drift or the regression
   as it happened. This is a finding about the rule's own apparatus (its scope has silently grown,
   its gate is not wired anywhere, and it is currently failing), not about any one `docs/src`
   page, rustdoc diagnostic, or example program — the phase's `MB-nn`/`RD-nn`/`EX-nn` ID taxonomy
   has no slot for it (per D-19's "neither documentation nor an example" category and RESEARCH.md
   Open Question 1's own recommendation). Per D-00e, this phase does not fix any of the 19 MISSING
   items and does not widen the frozen 76-item entry-point set to 101 — both would silently
   re-litigate a rule scope this phase has no mandate to change. The 19 individual violations
   (item name, file:line) are enumerated in full in `34-AUDIT.md`'s own subsection; this entry is
   the pointer only. Remains open for a maintainer decision: re-wire the script into CI/`make`
   with the current 101-item set as the new baseline, refreeze `16-DOCS-03-ENTRY-POINTS.md` at
   101, fix the 19 MISSING items, or some combination — none of which this phase decides.

## Plan 34-08, Task 1

1. **`ci.yml:538`'s example-file-count comment is stale by one (D-16, D-19).** The step comment
   reads "`examples/` holds 47 .rs files. Exactly 4 are declared `[[example]]` targets in
   Cargo.toml" — this plan's live re-count (`find examples -name '*.rs' | wc -l`) measures **48**,
   not 47. The four declared `[[example]]` targets and their required-features are still correct
   (`vision_analysis`/`vision_battalion` → `vision,llm-openai`; `document_processing` →
   `content-processing`; `http_service_host` → `web-server`), and all four CI invocations still
   cover every file on disk (the 44 undeclared files all build clean under the bare bulk selector,
   confirmed in `34-AUDIT.md` §4's `[[example]]` cross-check). Only the comment's count is stale —
   a CI workflow comment is neither documentation nor an example (D-19), so no `EX-nn` row was
   minted for it. Remains open for a maintainer to bump the comment's "47" to "48" (and confirm
   which file was added since the comment was last true) the next time `ci.yml`'s examples step is
   touched.

## Plan 34-09, Task 2

1. **PROJECT.md's "no crate under `crates/` ships its own `examples/` directory" claim is
   contradicted by the tree.** PROJECT.md's Phase 4 amendment (line 31, dated 2026-08-03, citing
   `04-release-measurement.md`) states plainly: "no crate under `crates/` ships its own `examples/`
   directory." `test -f crates/paladin-llm/examples/live_vendor_smoke.rs` → **exists**;
   `find crates -maxdepth 2 -type d -name examples` → `crates/paladin-llm/examples` — the directory
   the sentence says does not exist, does. This is a **planning-corpus fact**, not a `docs/src` page
   and not an `examples/` program in its own right (the file itself is audited on its own terms as
   part of §4's build/currency sweep, under its own build invocation per D-16 — that is a distinct,
   correct finding, not this one). The error is specifically in PROJECT.md's own summary prose, which
   this phase does not edit (Success Criterion 5, read-only) and which has no `MB-nn`/`RD-nn`/`EX-nn`
   slot per D-19's "neither documentation nor an example" category — PROJECT.md is the planning
   corpus, not the audited surface. Remains open for whichever future phase or quick task next
   touches PROJECT.md's Phase 4 amendment to correct the sentence (e.g. "no crate other than
   `paladin-llm` ships its own `examples/` directory").
2. **The object-store (MinIO → RustFS) evaluation itself is infrastructure work, distinct from the
   documentation slice plan 34-03 closed.** `todos/pending/2026-09-13-evaluate-rustfs-replacement-for-minio.md`
   (2026-09-13, score 0.6) asks to evaluate RustFS as the dev/test object store. Plan 34-03's
   object-store currency sweep (34-AUDIT.md §2, "Object-store currency sweep" subsection) settled
   the documentation-currency half of that todo: `grep -rniE 'minio|dl\.min\.io|quay\.io' docs/src
   examples` found 247 hits across 27 files, every actual image-pin occurrence already carrying the
   current `quay.io/minio/minio:RELEASE.2025-09-07T16-13-09Z.hotfix.7aa24e772` form (9 occurrences
   across 6 files), zero hits naming the retired Docker Hub image or the `dl.min.io` host that
   returns 410 — yielding **zero `MB-nn` items** from that slice. What remains open is the
   evaluation itself — whether RustFS should actually replace MinIO as the project's dev/test object
   store — which is an infrastructure/tooling decision, not a documentation gap: no `docs/src` page
   or `examples/` program is stale because of it, so D-19's taxonomy has no slot for it. The pending
   todo file already tracks this, unchanged by this phase; remains the maintainer's item.

---

*Phase: 34-documentation-currency-audit*
*Register opened: 2026-09-17, plan 34-04*
