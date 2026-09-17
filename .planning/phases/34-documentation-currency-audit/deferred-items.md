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

---

*Phase: 34-documentation-currency-audit*
*Register opened: 2026-09-17, plan 34-04*
