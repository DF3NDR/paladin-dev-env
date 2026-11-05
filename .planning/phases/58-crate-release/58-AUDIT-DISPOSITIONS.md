# Phase 58 Pre-tag Audit Disposition Register (SHIP-10)

**Written:** 2026-11-05 (plan 58-05). **Head the re-proofs ran on:** `9eba21d9ba26745f9b8c902b3fc2a94e0e03787f` (`9eba21d9b`).
**Scope:** Phases 52-57 plus this phase's own pre-tag findings, and the v0.11.0 audit's 40 `tech_debt` items. SHIP-08, SHIP-09
and SHIP-10 are out of scope and proven after the tag (RESEARCH Open Question 5; the v0.11.0 precedent).

## How to read this register

- **Three verdicts, one per row (D-06).** `fixed`: a pointer a reader can resolve (a commit, a `58-CI-EVIDENCE.md` reference, or a
  requirement id with its phase VERIFICATION). `waived`: a reason of at least twenty characters naming who accepted it and where.
  `re-homed`: an owner (`repo maintainer` or `v0.12.0 Phase 1`), a date, and a pending todo (D-09).
- **The milestone audit's `tech_debt` may cite only rows of this file** (plan 58-10), by id.
- **Prove-then-lock (D-06).** Every row's Re-proof cell names the command run or the file read, on the head above, before the
  verdict was written. A figure copied from another document is labelled as read, not as measured.
- **Dates and owners.** Every `re-homed` row reads `date: pending 58-05 decision` until the maintainer answers plan 58-05 Task 3
  (`DATE`); plan 58-06 writes the dated todo for each, and no date or owner is invented here (P-10-2). The three key rows
  (`KEY-LITERAL` fixed; liveness and purge `re-homed` to the maintainer) take their final verdicts from Task 3.
- **Scoping defaults, stated for objection.** (a) Info-class review findings of Phases 52, 53, 54, 55 and 56 are grouped in one
  `waived` row per review file, as Phase 57 D-12 scoped info items out; warnings, criticals and any info item a source names
  individually (54 IN-02, 57 IN-01..05) have their own rows. (b) Where a v0.11.0 `tech_debt` item has no section H bullet, the
  waiver cites the inventory triage row it rests on (B11, C11) and says so in the cell.
- **Cell format.** No cell contains a pipe character; `58-check-register.sh` splits rows on it. Verify with
  `bash .planning/phases/58-crate-release/58-check-register.sh --draft`.

## Sources

(filled in Task 2)

## Carried items

| ID | Item | Source | Re-proof | Verdict | Pointer / reason / owner + date | Todo |
|----|------|--------|----------|---------|---------------------------------|------|
| R-01 | 57-REVIEW CR-01: the worker's nack-and-redeliver loses a run's outcome when only the second write fails | 57-REVIEW.md:75 (CR-01); STATE Blockers/Concerns (Phase 57 close) | read src/application/services/run/worker.rs on 9eba21d9b: the completion block writes update_status (:1816) before record_outcome (:1824) and persist_failure writes Failed (:2101) before record_outcome (:2104); the status-first order the review names is unchanged | re-homed | owner: repo maintainer; date: pending 58-05 decision; target v0.12.0 (D-07, maintainer-owned): a behaviour change in a published crate | to be written in 58-06 |
| R-02 | 57-REVIEW WR-01: an empty APP_TREASURER_ALLOWANCE_WEBHOOK_SECRET silently wipes a YAML-configured secret | 57-REVIEW.md:105 (WR-01); STATE Blockers/Concerns (Phase 57 close) | read src/config/treasurer.rs:869-875 on 9eba21d9b: webhook.secret = Some(v).filter(non-empty) still assigns None for an empty env value, discarding a YAML secret | re-homed | owner: repo maintainer; date: pending 58-05 decision; target v0.12.0 (D-07, maintainer-owned): a behaviour change in a published crate | to be written in 58-06 |
| R-03 | 57-REVIEW WR-02: check-review-dispositions.sh accepts non-evidence as a closed pointer | 57-REVIEW.md:128 (WR-02); STATE Blockers/Concerns (Phase 57 close) | ran ./scripts/check-review-dispositions.sh on 9eba21d9b: checked 51 finding(s) across 13 review file(s); 0 failure(s); ran make test-shell-guards: 85 assertion(s) passed, exit 0 (includes the nine near-miss and guard cases) | fixed | commit 5d143727b fix(58-02) (RED fixtures cb2021567, plan 58-02) | - |
| R-04 | 57-REVIEW IN-01: a doc comment is split across the wrong item in worker.rs | 57-REVIEW.md:150 (IN-01) | read src/application/services/run/worker.rs:139-145 on 9eba21d9b: the line about the graph_fingerprint a RunStarted event carries still sits directly above RUN_ERROR_CHAR_BUDGET's docs | re-homed | owner: v0.12.0 Phase 1; date: pending 58-05 decision; src/application/services/run/worker.rs doc-comment placement | to be written in 58-06 |
| R-05 | 57-REVIEW IN-02: stale 'latest run' wording left behind by the D-02 rename | 57-REVIEW.md:162 (IN-02) | read crates/paladin-core/src/platform/container/principal.rs:10-16 on 9eba21d9b: the module doc still says a thread belongs to the tenant of its latest run (:14) | re-homed | owner: v0.12.0 Phase 1; date: pending 58-05 decision; crates/paladin-core/src/platform/container/principal.rs module doc | to be written in 58-06 |
| R-06 | 57-REVIEW IN-03: AdmissionSettlement::drop can refund an admission for a run that then commits | 57-REVIEW.md:168 (IN-03) | read src/application/services/treasurer/mod.rs:25-26 on 9eba21d9b: the documented residuals name only a process crash and a drop inside admit; the AdmissionSettlement drop race (submission.rs:194-262) is not listed | re-homed | owner: v0.12.0 Phase 1; date: pending 58-05 decision; src/application/services/run/submission.rs and treasurer/mod.rs residual list | to be written in 58-06 |
| R-07 | 57-REVIEW IN-04: s3_error_code embeds up to 64 characters of server-controlled text unsanitised | 57-REVIEW.md:174 (IN-04) | read crates/paladin-storage/src/minio.rs:107-112 on 9eba21d9b: s3_error_code still takes chars().take(64) with no character filter | re-homed | owner: v0.12.0 Phase 1; date: pending 58-05 decision; crates/paladin-storage/src/minio.rs s3_error_code character filter | to be written in 58-06 |
| R-08 | 57-REVIEW IN-05: the fence marker TTL cap reopens stale-writer overwrite for long-lived node-cache entries | 57-REVIEW.md:180 (IN-05) | read crates/paladin-storage/src/redis_fence.rs:45-64 on 9eba21d9b: the 5 to 30 minute clamp and its trade-off for entry TTLs above 15 minutes are documented as accepted by ADR-0058 (g); no code changed | re-homed | owner: v0.12.0 Phase 1; date: pending 58-05 decision; crates/paladin-storage/src/redis_fence.rs: no change if ADR-0058 (g) holds; record an ADR exception only if a node can produce a non-deterministic result | to be written in 58-06 |


## v0.11.0 carry-over reconciliation

| ID | Item | Source | Re-proof | Verdict | Pointer / reason / owner + date | Todo |
|----|------|--------|----------|---------|---------------------------------|------|


## Closed by this phase after the register is locked

Items whose closing act is a later plan of this phase. They are not counted by `58-check-register.sh`.

| Item | Closing plan |
|------|--------------|
| The two stale `docs/src/api-reference` version statements (RESEARCH Open Question 6) | 58-07, bump commit 3 |
| The release-body size guard's first live run (52 D-12; STATE Blockers/Concerns, Phase 52 close) | 58-13 (read at the tag) |
| Phases 54, 55, 56 and 57 reaching `main` (STATE Blockers/Concerns, Phase 54-57 close notes) | 58-12 (merge) and 58-13 |
| The `release`-labelled End-to-End Tests run on the release pull request | 58-10 |
| The full `ci.yml` conclusion on the tagged SHA, including the SDK smoke (F-58-02, F-58-04) | 58-12 (`check-release-consistency` clause 3) |
| STATE.md `### Pending Todos` and `### Blockers/Concerns` entries that are stale (three of four listed todos are already in `todos/completed/`) | 58-14 |
| SHIP-08, SHIP-09 and SHIP-10 ticks, proven by the post-tag registry read and the audit | 58-14 |
