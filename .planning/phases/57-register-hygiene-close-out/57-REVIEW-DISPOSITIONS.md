# Phase 57 Review Dispositions (REG-05)

**Date:** 2026-11-04
**Gate:** `./scripts/check-review-dispositions.sh` (derives every expected id from the archived `NN-REVIEW.md` headings, never from this file)

## Scope

- Source: `.planning/milestones/v0.11.0-phases/*/NN-REVIEW.md` (13 files; Phases 39 and 49 have no review).
- Every `### WR-<n>:` and `### CR-<n>:` heading (47 warnings + 2 criticals: PD-08-1, `38-REVIEW` CR-01 and `45-REVIEW` CR-01), plus Phase 40's `IN-01` and `IN-04` (D-12): 51 findings in total.
- Phase 50: 0 warnings, 0 criticals (`50-REVIEW.md` frontmatter `warning: 0`); it has no rows, and carries the appended pointer section only.
- Phase 42: the archived file is review iteration 2 and holds `WR-5` and `WR-6` (iteration 1's WR-1..4 were fixed before it was written, see `42-REVIEW-FIX.md`).
- Other info items are out of scope (D-12).

## Vocabulary (D-11)

- `closed`: fixed and still fixed; Evidence names a commit, test, requirement or REVIEW-FIX entry that was checked in this phase.
- `waived`: accepted by the owner; Evidence is the reason and the Owner column is filled.
- `open`: still live in the tree. Evidence reads `LIVE <file:line> — <correctness|security|other>: <one line>` (PD-08-2). The 57-10 triage decides fix or waive (D-14); 57-11 runs the checker strict, where an `open` row is a failure.

## Evidence rule (D-13)

The archive is read-only. Every `closed` pointer was checked in this phase: a commit that exists (`git cat-file`, or `gh api repos/DF3NDR/paladin-dev-env/commits/<sha>` for commits older than the shallow clone), a test re-run on the head, a requirement with evidence. No closure is asserted from memory.

## Dispositions

| Phase | Id | Title | Disposition | Evidence | Owner (if waived) |
|-------|----|-------|-------------|----------|-------------------|
| 40 | WR-01 | POST /v1/runs {thread_id} and fork run against another tenant's thread | closed | TENANT-04; submit_on_another_tenants_thread_is_rejected_as_unknown_thread and fork_on_another_tenants_thread_is_rejected_as_unknown_thread (re-run 2026-11-04: ok); 40-REVIEW-FIX 6d7efb16; 53 locks 2b16763d0, 66002e74b | |
| 40 | WR-02 | Tenant isolation for cancel enforced only in the HTTP controller | closed | TENANT-05; cancel_of_another_tenants_run_is_not_found_and_writes_nothing (re-run 2026-11-04: ok); 40-REVIEW-FIX fa054746 | |
| 40 | WR-03 | A corrupt or half-attributed row is a 500 that breaks hidden equals missing | closed | TENANT-06; corrupt_run_row_is_the_missing_run_404_for_a_tenant_principal (re-run 2026-11-04: ok); 40-REVIEW-FIX 05612748, 9afaef2b; migration crates/paladin-storage/migrations/postgres/009_add_run_attribution_check.sql; 53 commits 30b532813, 839a8119a | |
| 40 | WR-04 | Serialization messages surface raw backend text to API clients | closed | TENANT-06; get_run_repository_error_is_a_generic_500_without_backend_text (re-run 2026-11-04: ok); 40-REVIEW-FIX 05612748 | |
| 40 | IN-01 | Reserved literals accepted as real tenant and key names | closed | TENANT-09; validate_rejects_reserved_literals_in_every_position (re-run 2026-11-04: ok); commit 4de0bcbc2 | |
| 40 | IN-04 | GET /agents/{id}/jobs/{job_id} ignores the principal and tenant | closed | TENANT-07; job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass (re-run 2026-11-04: ok); commits cc0077527, 010ee3158 | |
| 42 | WR-5 | Replay stale-record skip covers only a Completed or Failed row | closed | 42-REVIEW-FIX 89182bad (commit verified with gh api); later_record_of_run_exists at src/application/services/run/events.rs:802; replay_skips_a_drained_halted_record_when_the_run_later_halted_on_spend, replay_skips_a_drained_halted_record_when_the_run_later_cancelled, replay_keeps_a_last_reasonless_halted_record_beside_a_cancelled_row and replay_skips_a_stale_halted_record_found_at_a_page_boundary (re-run 2026-11-04: 5 ok with WR-6's) | |
| 42 | WR-6 | A second run on a thread loses the trace records whose seq the first run used | closed | 42-REVIEW-FIX 7f1c4970 (commit verified with gh api); with_seq_origin at crates/paladin-battalion/src/engine/hooks.rs:464, max_seq at crates/paladin-ports/src/output/run_trace_port.rs:272, trace_seq_origin at src/application/services/run/worker.rs:1066; a_second_run_on_a_thread_keeps_its_trace_records, seq_origin_starts_the_sequence_after_the_origin and default_max_seq_pages_through_read (re-run 2026-11-04: ok). Residuals recorded in 42-REVIEW-FIX: Postgres override not run (Docker-gated), concurrent same-thread dispatch can still collide | |
| 46 | WR-01 | Maneuver warning and docs wrong once an explicit ManeuverConfig is set | closed | 46-REVIEW-FIX df85c999 (commit exists); maneuver_warned_aegis_policies at crates/paladin-battalion/src/commander.rs:593; maneuver_warning_fires_only_on_the_derived_config_path, error_strategy_keeps_the_aegis_derived_maneuver_timeout and error_strategy_and_maneuver_config_compose_in_call_order (re-run 2026-11-04: ok) | |
| 46 | WR-02 | Release notes say a v0.10 document with removed keys still deserializes | closed | 46-REVIEW-FIX 065d4542 (commit exists); CHANGELOG.md:288 and crates/paladin-core/CHANGELOG.md:154 now say the key is refused at load (read 2026-11-04); no "still deserializes" text remains in either file | |
| 46 | WR-03 | execute_with_cancellation rustdoc overstates what survives cancellation for FirstSuccess | closed | 46-REVIEW-FIX 5186f128 (commit exists); per-strategy wording at crates/paladin-battalion/src/phalanx_service.rs:183-188 (read 2026-11-04; doc-only fix) | |
| 48 | WR-01 | Rule 5 graph freeze is satisfied by unrelated occurrences | closed | 48-REVIEW-FIX cd6e95c0 (commit exists); declares at tests/entity_rename_guard.rs:551; scanner_reports_graph_declarations_that_only_appear_elsewhere (re-run 2026-11-04: ok) | |
| 48 | WR-02 | The guard silently skips every dot-directory including .github | closed | 48-REVIEW-FIX 6a48b71e (commit exists); SKIPPED_DOT_DIRECTORIES at tests/entity_rename_guard.rs:239 lists only .git, .planning and .project; scanner_reports_planted_entity_rename_violations (re-run 2026-11-04: ok) | |
| 48 | WR-03 | The no docs directory carve-out disables the schema freeze in any docs-less tree | closed | 48-REVIEW-FIX cdefd67c (commit exists); DOCKER_EXCLUDED_DIRS at tests/entity_rename_guard.rs:163; scanner_reports_a_missing_schema_when_only_docs_is_absent and scanner_accepts_a_build_context_without_docs (re-run 2026-11-04: ok) | |
| 48 | WR-04 | production_lines truncates at any line starting with mod tests | closed | 48-REVIEW-FIX ceeb148e (commit exists); opens_test_module at tests/entity_rename_guard.rs:495; production_scan_stops_only_at_an_inline_test_module (re-run 2026-11-04: ok) | |
| 48 | WR-05 | Rule 4 declaration needles are formatting and visibility sensitive | closed | 48-REVIEW-FIX 41837b56 (commit exists); normalize_declaration at tests/entity_rename_guard.rs:349 and blank_non_code at tests/entity_rename_guard.rs:393; whole guard cargo test --test entity_rename_guard (re-run 2026-11-04: 8 passed) | |
| 50.1 | WR-01 | new(api_key, model) docs promise a default-model fallback no adapter implements | closed | 50.1-REVIEW-FIX 9d0eaa3d (commit exists); doc-only fix, reworded text read at crates/paladin-llm/src/kimi/adapter.rs:84 and :206 on 2026-11-04 | |
| 50.1 | WR-02 | The grok-4.7 exception misstates the transcript evidence | closed | 50.1-REVIEW-FIX ec3fdc92 (commit exists); doc-only fix, corrected exception text read at crates/paladin-llm/src/registry/seeded.rs:2348 on 2026-11-04 | |
| 50.1 | WR-03 | with_base_url docs promise http(s) validation seven of nine adapters do not perform | closed | 50.1-REVIEW-FIX 5ebd315b (commit exists); is_http_url at crates/paladin-llm/src/registry/mod.rs:169 used in all nine adapters; a_builder_set_non_http_base_url_fails_openai_construction and its eight siblings (re-run 2026-11-04 with --all-features: 9 ok) | |
| 50.1 | WR-04 | CI muster job comment contradicts the Makefile and the zero-test guard can exit silently | closed | 50.1-REVIEW-FIX c5bdea75 (commit exists); the executed= pipeline carries the guard at .github/workflows/ci.yml:1664 (read 2026-11-04) | |

**Totals:** closed: 20, waived: 0, open: 0

## Live in tree

None among Phases 40, 42, 46, 48, 50 and 50.1: every finding was found fixed in the tree. Phases 38, 41, 43, 44, 45, 47 and 51 are dispositioned by 57-09 and 57-10.

Documented residuals, not live findings (recorded in `42-REVIEW-FIX.md`): 42 WR-6's PostgreSQL `max_seq` override and its contract test were Docker-gated and not run by the fixer; two runs dispatched concurrently on one thread can still read the same max and collide (the fork and requeue paths the finding describes are sequential).
