# Phase 57 Register Evidence (plan 57-11)

Recorded 2026-11-04 on branch `claude/laughing-dirac-e0h2ax`. Phase head when the rows below were first
re-run: `7ab20afbacf316a578adb9e45880f3fd363495cb` (57-16's metadata commit). `windows status` before any change in 57-11:
`open_count: 26`, `waived_count: 40`, `fixed_count: 34`, `total_count: 100`.

Rule (D-40): "before" cells are copied from the plan SUMMARYs, which measured them on each plan's own start SHA.
"Head" cells are re-run in 57-11, in this session, on the head above, and are the only cells a WINDOWS row is
closed on. A row whose closing evidence is a Docker-gated CI job is flipped only after that job's log is read
(PD-11-4). Raw logs sit in the session scratchpad and are not committed; the lines that matter are quoted here.

## WINDOWS dispositions

Head for every re-run below: `d0b0022ec` (the plan-time head `7ab20afba` for row 56). Raw logs: session scratchpad, not committed. Row counts: `windows status` before 57-11 `open 26 / waived 40 / fixed 34 / total 100`; after Task 2 `open 4 / waived 53 / fixed 51 / total 108` (the four open rows were exactly 68, 90, 93 and 100, the CI-gated ones, PD-11-4); after Task 3 `open 0 / waived 53 / fixed 55 / total 108`.

| Row | Kind / phase | Disposition | Evidence (re-run on head, or CI run and job id) | Reason cell |
|-----|--------------|-------------|-------------------------------------------------|-------------|
| 56 | lint-warning / 38 | fixed | `cargo doc -p paladin-ai-core --no-deps` (7ab20afba): `Documenting paladin-ai-core v0.11.0 ... Finished`, `warning:` line count 0. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps`: exit 0, `Generated .../target/doc/paladin/index.html and 14 other files`. First measured in `55-CORRECTNESS-EVIDENCE.md` Carry-forward. | Closed by Phase 57 (57-11, D-05): cargo doc -p paladin-ai-core --no-deps prints 0 warning lines and cargo doc --workspace --all-features --no-deps with RUSTDOCFLAGS set to -D warnings exits 0 on 7ab20afba (first measured in 55-CORRECTNESS-EVIDENCE.md); see 57-REGISTER-EVIDENCE.md |
| 64 | deviation / 42 | waived | Waived row, no closing evidence needed (D-03). Reason shape checked by the Task 2 verify (starts `Re-homed: owner=`, has `revisit=` and `basis=`, no pipe). Basis cited and present: ADR-0057 D-08 (decision (d), line 220 of `.planning/decisions/0057-mid-run-halt-contract.md`). | Re-homed: owner=Treasurer maintainers (next feature milestone roadmap), revisit=when worker-dispatched agent-kind runs gain a Waypoint or ALLOW-03 is re-scoped for the agent loop, basis=ADR-0057 D-08 (agent-loop no-Waypoint contract) and v0.11.1 inventory section H. Accepted contract under Phase 57 D-03 and D-00e: a halted agent-kind run resumes by a fresh POST /v1/runs. |
| 65 | deviation / 42 | waived | Waived row, no closing evidence needed (D-03). Reason shape checked by the Task 2 verify (starts `Re-homed: owner=`, has `revisit=` and `basis=`, no pipe). Basis cited and present: ADR-0057 D-01 (decision (a), line 85). | Re-homed: owner=Treasurer maintainers (next feature milestone roadmap), revisit=a per-superstep reservation hold with an estimate rule is designed, basis=ADR-0057 D-01 (check-only mid-run boundary) and v0.11.1 inventory section H. Accepted contract under Phase 57 D-03 and D-00e: overshoot stays bounded to one superstep of spend per run. |
| 66 | deviation / 42 | waived | Waived row, no closing evidence needed (D-03). Reason shape checked by the Task 2 verify (starts `Re-homed: owner=`, has `revisit=` and `basis=`, no pipe). Basis cited and present: ADR-0057 decision (e), the derived agent budget (line 225). | Re-homed: owner=Treasurer maintainers (next feature milestone roadmap), revisit=a provider streaming path reports usage mid-stream or ADR-0057 group e is reopened, basis=ADR-0057 group e (G2, D-12, 42-01 option-b) and v0.11.1 inventory section H. Accepted contract under Phase 57 D-03 and D-00e: the overshoot of a true streamed call is that one call. |
| 68 | unrun-verify / 48 | fixed | `make security` exit 0 on `4b115d511` (`## REG-06 security exceptions`). CI run `41927940508` on `4b115d511`: `Security Audit` job `126917997616` `success`, `License & Dependency Policy` job `126917997392` `success`. | Closed by Phase 57 (57-01, 57-11, D-05): make security exits 0 on 4b115d511 after the RUSTSEC-2026-0429 row (both legs, advisories ok, bans ok, licenses ok, sources ok); CI Security Audit job 126917997616 and License & Dependency Policy job 126917997392 succeeded in run 41927940508 on 4b115d511; see 57-REGISTER-EVIDENCE.md |
| 69 | unrun-verify / 48 | fixed | CI Coverage job `126153522434`, conclusion `success` (re-read with `gh api`), run `41703614636`, head `d9a48cfc6`, after the 48-09 cause; 92.74 percent on `70657919` (`47-CI-EVIDENCE.md`); Phase 56 local 92.79 percent (DOGF-05). The pushed head's Coverage job is appended in Task 3. Pushed head: Coverage job `126917997573` `success` in run `41927940508` on `4b115d511`, `Lines:     172187/185431 = 92.86%` (gate 82 percent). | Closed by Phase 57 (57-11, D-05): CI Coverage job 126153522434 succeeded in run 41703614636 on d9a48cfc6 (after the 48-09 cause) and passed the 82 percent gate with 92.74 percent measured on 70657919 (47-CI-EVIDENCE.md) and 92.79 percent locally in Phase 56 (DOGF-05); pushed head 4b115d511: Coverage job 126917997573 succeeded in run 41927940508 and measured 92.86 percent (172187 of 185431 lines); see 57-REGISTER-EVIDENCE.md |
| 77 | deviation / 50 | waived | Waived row, no closing evidence needed (D-03). Reason shape checked by the Task 2 verify (starts `Re-homed: owner=`, has `revisit=` and `basis=`, no pipe). Basis cited and present: `cost_in_usd_ticks` appears in ADR-0061; `50-USAGE-PROBE.md` exists in the archive. | Re-homed: owner=DF3NDR, revisit=when the Grok adapter parses cost_in_usd_ticks and num_sources_used and compares them with the Treasurer price, basis=ADR-0061 and milestone decision B9 (cross-check data in 50-USAGE-PROBE.md). Owner-accepted vendor-cost deferral of 2026-10-17, kept under Phase 57 D-03 and D-00e. |
| 79 | deviation / 50.1 | waived | Waived row, no closing evidence needed (D-03). Reason shape checked by the Task 2 verify (starts `Re-homed: owner=`, has `revisit=` and `basis=`, no pipe). Basis cited and present: `50.1-SEED-PROBE.md` exists in the archive. | Re-homed: owner=DF3NDR, revisit=an operator-machine seed-probe run before the next model-seed change, basis=50.1-SEED-PROBE.md (probe run in the devcontainer at operator direction). grok-4.7 and the 7 default-rejected candidates stay unseeded until measured end to end (Phase 57 D-03, D-00e). |
| 83 | unrun-verify / 52 | fixed | Workflow Lint job `124029754795` `success` (run `41045889697`, PR #61 merge `84686acfb`, which contains the 52-01 ci.yml edits `be5867729` and `bd1665e4b`); job `124670697434` `success` (main run `41246424413`, `8b23cb4c5`). Both re-read with `gh api`. Pushed head: Workflow Lint job `126917997309` `success` in run `41927940508` on `4b115d511`. | Closed by Phase 57 (57-11, D-05): CI Workflow Lint job 124029754795 succeeded in run 41045889697 on PR 61 merge 84686acfb and job 124670697434 succeeded in main run 41246424413 on 8b23cb4c5, both after the 52-01 ci.yml edits (be5867729, bd1665e4b); pushed head 4b115d511: Workflow Lint job 126917997309 succeeded in run 41927940508; see 57-REGISTER-EVIDENCE.md |
| 84 | unrun-verify / 52 | fixed | Workflow Lint job `124670697434` `success` (main run `41246424413`, `8b23cb4c5`) and job `126363067412` `success` (branch run `41765477891`, `69de7c285`); `2644b974c` (the 52-06 ci.yml edit) is an ancestor of both. Re-read with `gh api`. Pushed head: Workflow Lint job `126917997309` `success` in run `41927940508` on `4b115d511`. | Closed by Phase 57 (57-11, D-05): CI Workflow Lint job 124670697434 succeeded in main run 41246424413 on 8b23cb4c5 and job 126363067412 succeeded in branch run 41765477891 on 69de7c285, both containing the 52-06 ci.yml edits (2644b974c); pushed head 4b115d511: Workflow Lint job 126917997309 succeeded in run 41927940508; see 57-REGISTER-EVIDENCE.md |
| 85 | deviation / 53 | fixed | `cargo test -p paladin-web --lib -- thread_controller::` on d0b0022ec: `test result: ok. 56 passed; 0 failed`; includes `list_threads_skips_a_newer_schema_thread_for_a_tenant` and `newer_schema_owner_row_hides_the_thread_from_a_tenant` `ok`. 57-04 caveat met: ADR-0063 amended (`b57100e63`), MIGRATION and changelogs (`bfee47716`). | Closed by Phase 57 (57-03, 57-04, D-02): on d0b0022ec thread_controller::tests::list_threads_skips_a_newer_schema_thread_for_a_tenant and newer_schema_owner_row_hides_the_thread_from_a_tenant ok (cargo test -p paladin-web --lib thread_controller::, 56 passed); owner rule recorded in ADR-0063 and MIGRATION by 57-04; see 57-REGISTER-EVIDENCE.md |
| 86 | deviation / 53 | fixed | `cargo test -p paladin-storage --features s3,sqlite,redis-cache --lib -- owner_run_for_thread owner_query_reads_exactly minio:: fence_marker_...` on d0b0022ec: `test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 528 filtered out`: `run::in_memory::contract_suite::owner_run_for_thread_decodes_only_the_owner_row` and `run::sqlite::tests::owner_run_for_thread_decodes_only_the_owner_row` `ok`; `cargo test -p paladin-storage --features postgres --lib -- owner_query_reads_exactly_one_row_attributed_first_oldest_first`: `1 passed`; `cargo test --lib --features web-server -- thread_owner_is_the_earliest_attributed_run_over_sqlite`: `test result: ok. 1 passed`. | Closed by Phase 57 (57-02, 57-03, D-02): on d0b0022ec owner_run_for_thread_decodes_only_the_owner_row ok on the in-memory and SQLite legs, the PostgreSQL owner-query shape guard ok, and http_surface_tests::thread_owner_is_the_earliest_attributed_run_over_sqlite ok; see 57-REGISTER-EVIDENCE.md |
| 87 | deviation / 53 | fixed | Same storage run as row 86: `owner_run_for_thread_ignores_later_admin_and_unattributed_runs`, `owner_run_for_thread_returns_the_earliest_attributed_run`, `owner_run_for_thread_falls_back_to_the_earliest_run_when_none_is_attributed` and `owner_run_for_thread_is_none_for_a_thread_with_no_runs` `ok` on in-memory and SQLite; `cargo test -p paladin-ports --lib -- default_impl_of_owner_run_for_thread`: `2 passed`; `cargo test -p paladin-web --lib -- thread_controller::` on d0b0022ec: `test result: ok. 56 passed; 0 failed` (`admin_run_on_a_tenant_thread_does_not_move_its_owner`, `unattributed_run_on_a_tenant_thread_does_not_move_its_owner`, `mixed_tenant_thread_belongs_to_the_earliest_attributed_runs_tenant` `ok`); root run (rows 88/89 below) includes `submission::tests::submit_guard_reads_the_owner_run_not_the_newest_run` `ok`. | Closed by Phase 57 (57-02, 57-03, D-02): on d0b0022ec owner_run_for_thread_ignores_later_admin_and_unattributed_runs and owner_run_for_thread_returns_the_earliest_attributed_run ok on the in-memory and SQLite legs, thread_controller admin_run_on_a_tenant_thread_does_not_move_its_owner and unattributed_run_on_a_tenant_thread_does_not_move_its_owner ok, and submission::tests::submit_guard_reads_the_owner_run_not_the_newest_run ok; see 57-REGISTER-EVIDENCE.md |
| 88 | deviation / 45 | fixed | Root run `test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 1501 filtered out`: `worker_tests::bounded_run_error_redacts_before_bounding` and `worker_tests::agent_kind_run_emits_exactly_one_terminal_event` `ok`. Fix commit `43d740c34`. | Closed by Phase 57 (57-05, D-14): on d0b0022ec worker_tests::bounded_run_error_redacts_before_bounding and agent_kind_run_emits_exactly_one_terminal_event ok (cargo test --lib --features web-server, fix commit 43d740c34); see 57-REGISTER-EVIDENCE.md |
| 89 | deviation / 41 | fixed | Same root run: `config::treasurer::tests::empty_env_operator_webhook_secret_is_treated_as_absent`, `run_api_wiring::tests::operator_webhook_secret_warning_line_names_the_key_and_never_the_url` and `operator_webhook_secret_warning_line_is_silent_with_a_secret` `ok`. Fix commit `d26c871d8`. | Closed by Phase 57 (57-05, D-14): on d0b0022ec config::treasurer::tests::empty_env_operator_webhook_secret_is_treated_as_absent and run_api_wiring::tests::operator_webhook_secret_warning_line_names_the_key_and_never_the_url and operator_webhook_secret_warning_line_is_silent_with_a_secret ok (fix commit d26c871d8); see 57-REGISTER-EVIDENCE.md |
| 90 | deviation / 45 | fixed | Local (Task 2): `minio::tests::list_request_sends_the_limit_as_max_keys_never_as_a_delimiter` `ok` (storage run, `28 passed`); `--no-run` compile exit 0. CI run `41927940508` on `4b115d511`: `Integration Tests` job `126917997844` `success`; step `Run file storage contract suite (RustFS)` log shows `test integration::file_storage_integration_tests::tests::test_list_files_honours_limit_and_drops_no_keys ... ` then `✅ List limit and continuation test passed` and `ok`, and `test result: ok. 12 passed; 0 failed`, `file storage contract suite passed: 12`. | Closed by Phase 57 (57-06, 57-11, D-14): minio::tests::list_request_sends_the_limit_as_max_keys_never_as_a_delimiter ok on d0b0022ec (fix commit b5ba82365); CI Integration Tests job 126917997844 in run 41927940508 on 4b115d511 ran test_list_files_honours_limit_and_drops_no_keys ok against RustFS; see 57-REGISTER-EVIDENCE.md |
| 91 | deviation / 45 | fixed | Same storage run as row 86: `minio::tests::s3_err_summary_never_echoes_the_response_body` and `minio::tests::retry_exhaustion_text_never_echoes_the_response_body` `ok`. Fix commit `6d0de8905`. | Closed by Phase 57 (57-06, D-14): on d0b0022ec minio::tests::s3_err_summary_never_echoes_the_response_body and retry_exhaustion_text_never_echoes_the_response_body ok (cargo test -p paladin-storage --features s3 --lib, fix commit 6d0de8905); see 57-REGISTER-EVIDENCE.md |
| 92 | deviation / 45 | fixed | Doc-only fix `6d0de8905`: rustdoc on `upload_part` (`minio.rs:1069-1072`), `complete_multipart_upload` (`:1104-1105`) and `abort_multipart_upload` says a rejected part leaves the upload open and the caller must abort (read on d0b0022ec). `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` exited 0 on `7ab20afba` (the row 56 measurement above); `git diff --stat 7ab20afba HEAD` touches only `.planning/`. | Closed by Phase 57 (57-06, D-14): doc-only fix 6d0de8905, the rustdoc on upload_part, complete_multipart_upload and abort_multipart_upload says a rejected part leaves the upload open and the caller must abort (read on d0b0022ec), and RUSTDOCFLAGS set to -D warnings cargo doc --workspace --all-features --no-deps exits 0 on 7ab20afba with no code change since; see 57-REGISTER-EVIDENCE.md |
| 93 | deviation / 45 | fixed | Local (Task 2): `--no-run` compile exit 0 and `-- --list` shows the case. CI: same `Integration Tests` job `126917997844`; log shows `test integration::file_storage_integration_tests::tests::test_presigned_urls ... ` then the two `Generated presigned ... URL` lines and `ok` (same `12 passed; 0 failed`). | Closed by Phase 57 (57-06, 57-11, D-14): the integration target compiles with --features integration-tests,s3-storage on d0b0022ec (fix commit ac7841931, five without_url sites); CI Integration Tests job 126917997844 in run 41927940508 on 4b115d511 ran test_presigned_urls ok; see 57-REGISTER-EVIDENCE.md |
| 94 | deviation / 51 | fixed | Root run: `paladin::paladin_builder::tests::effort_refusal_renders_only_the_error_kind_for_a_failed_check`, `effort_refusal_through_a_fallback_chain_renders_only_the_error_kind` and `effort_refusal_for_an_unsupported_parameter_names_model_provider_value_and_reason` `ok`. Fix commit `f55ff310e` (57-12). | Closed by Phase 57 (57-12, D-14): on d0b0022ec effort_refusal_renders_only_the_error_kind_for_a_failed_check and effort_refusal_through_a_fallback_chain_renders_only_the_error_kind and effort_refusal_for_an_unsupported_parameter_names_model_provider_value_and_reason ok (cargo test --lib --features web-server effort_refusal_, fix commit f55ff310e); see 57-REGISTER-EVIDENCE.md |
| 95 | deviation / 45 | fixed | Root run: `worker_tests::agent_completion_whose_status_write_fails_emits_no_terminal_event_and_nacks`, `agent_completion_whose_outcome_write_fails_emits_no_terminal_event_and_nacks`, `agent_completion_whose_ack_fails_emits_no_terminal_event`, `agent_failure_whose_persist_fails_emits_no_terminal_event_and_nacks` and `agent_budget::agent_kind_halt_whose_outcome_write_fails_emits_no_terminal_event` `ok`. Fix commit `2d15a0bdd` (57-13). | Closed by Phase 57 (57-13, D-14): on d0b0022ec the five worker_tests cases agent_completion_whose_status_write_fails and outcome_write_fails and ack_fails and agent_failure_whose_persist_fails and agent_budget::agent_kind_halt_whose_outcome_write_fails emit no terminal event ok (cargo test --lib --features web-server _emits_no_terminal_event, fix commit 2d15a0bdd); see 57-REGISTER-EVIDENCE.md |
| 96 | deviation / 45 | fixed | Root run: `worker_tests::failed_agent_run_reports_the_spend_it_incurred_before_failing`, `failed_agent_run_with_no_billed_call_reports_zero_spend` and `agent_loop_cost_tests::non_llm_failure_after_billed_calls_reports_their_spend` `ok`. Fix commit `25536a97c` (57-13). | Closed by Phase 57 (57-13, D-14): on d0b0022ec worker_tests::failed_agent_run_reports_the_spend_it_incurred_before_failing and failed_agent_run_with_no_billed_call_reports_zero_spend ok, plus agent_loop_cost_tests::non_llm_failure_after_billed_calls_reports_their_spend ok (fix commit 25536a97c); see 57-REGISTER-EVIDENCE.md |
| 97 | deviation / 41 | fixed | Root run: `treasurer::tests::enqueue_failure_gives_the_notice_back_so_the_next_admission_wins_it`, `treasurer::tests::confirm_completes_when_its_caller_is_dropped_mid_enqueue`, `run::submission::tests::a_submission_dropped_before_its_row_is_written_gives_the_notice_back` and `a_submission_dropped_after_its_row_is_written_confirms_the_notice` `ok`. Fix commits `e4563971a` and `78951d7ec` (57-14). | Closed by Phase 57 (57-14, D-14): on d0b0022ec treasurer::tests::enqueue_failure_gives_the_notice_back_so_the_next_admission_wins_it and confirm_completes_when_its_caller_is_dropped_mid_enqueue ok and submission::tests::a_submission_dropped_before_its_row_is_written_gives_the_notice_back and a_submission_dropped_after_its_row_is_written_confirms_the_notice ok (fix commits e4563971a and 78951d7ec); see 57-REGISTER-EVIDENCE.md |
| 98 | deviation / 43 | fixed | `cargo test -p paladin-llm --lib -- cadence::` on d0b0022ec: `test result: ok. 35 passed; 0 failed`; `a_zero_retry_after_gates_at_least_the_base_delay`, `a_past_http_date_retry_after_gates_at_least_the_base_delay`, `a_zero_reset_gates_at_least_the_base_delay`, `repeated_zero_delay_429s_escalate_the_streak` and `a_nonzero_retry_after_below_base_is_still_honoured_exactly` `ok`. Fix commit `5adfda4d9` (57-15). | Closed by Phase 57 (57-15, D-14): on d0b0022ec cadence::tests::a_zero_retry_after_gates_at_least_the_base_delay and a_past_http_date_retry_after_gates_at_least_the_base_delay and a_zero_reset_gates_at_least_the_base_delay and repeated_zero_delay_429s_escalate_the_streak and a_nonzero_retry_after_below_base_is_still_honoured_exactly ok (cargo test -p paladin-llm --lib cadence::, 35 passed, fix commit 5adfda4d9); see 57-REGISTER-EVIDENCE.md |
| 99 | deviation / 43 | fixed | Same cadence run: `a_429_is_recorded_when_the_generate_future_is_dropped_mid_record` and `a_429_is_recorded_when_the_stream_future_is_dropped_mid_record` `ok`. Fix commit `a10f57c7e` (57-15). | Closed by Phase 57 (57-15, D-14): on d0b0022ec cadence::tests::a_429_is_recorded_when_the_generate_future_is_dropped_mid_record and a_429_is_recorded_when_the_stream_future_is_dropped_mid_record ok (fix commit a10f57c7e); see 57-REGISTER-EVIDENCE.md |
| 100 | deviation / 43 | fixed | Local (Task 2): `node_cache::redis::tests::fence_marker_never_outlives_the_counter_that_numbers_it ... ok`. CI run `41927940508` on `4b115d511`: `Redis Node Cache Contract Suite (live server)` job `126917997409` `success`; log line `test node_cache::redis::tests::fenced_write_after_the_counter_expires_is_accepted ... ok`, `test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 303 filtered out`, the guard step printed `All node_cache::redis tests exercised the live server.`, and no `SKIP:` test-output line exists (the only `SKIP:` text in the log is the guard step's own script). | Closed by Phase 57 (57-16, 57-11, D-14): fence_marker_never_outlives_the_counter_that_numbers_it ok on d0b0022ec (fix commit f0898155f, test commit aec444449); CI Redis Node Cache Contract Suite job 126917997409 in run 41927940508 on 4b115d511 ran fenced_write_after_the_counter_expires_is_accepted ok with no SKIP line (21 passed, 0 failed); see 57-REGISTER-EVIDENCE.md |
| 101 | deviation / 38 | waived | Minted by 57-11 for `38-REVIEW WR-01` (class `other`, 57-10 triage option-a, owner reply of 2026-11-04) and waived in the D-03 shape. No closing evidence needed; the register row reads `waived`, Evidence `WINDOWS row 101; 2026-11-04`. | Re-homed: owner=owner (am0rfu5@protonmail.com), revisit=when any producer can build a negative Cost (refunds, credits), basis=38-REVIEW WR-01; register row 38 WR-01 (latent: no current producer builds a negative Cost). Class other, waived at the 57-10 triage (option-a, D-14) on 2026-11-04 and re-homed under Phase 57 D-03. |
| 102 | deviation / 38 | waived | Minted by 57-11 for `38-REVIEW WR-02` (class `other`, 57-10 triage option-a, owner reply of 2026-11-04) and waived in the D-03 shape. No closing evidence needed; the register row reads `waived`, Evidence `WINDOWS row 102; 2026-11-04`. | Re-homed: owner=owner (am0rfu5@protonmail.com), revisit=when multi-tenant pricing or hot price reload is added, basis=38-REVIEW WR-02; pricing.rs doc comment (D-08, process-wide dedup is deliberate). Class other, waived at the 57-10 triage (option-a, D-14) on 2026-11-04 and re-homed under Phase 57 D-03. |
| 103 | deviation / 51 | waived | Minted by 57-11 for `51-REVIEW WR-02` (class `other`, 57-10 triage option-a, owner reply of 2026-11-04) and waived in the D-03 shape. No closing evidence needed; the register row reads `waived`, Evidence `WINDOWS row 103; 2026-11-04`. | Re-homed: owner=owner (am0rfu5@protonmail.com), revisit=next change to check_reasoning or the reasoning refusal wording, basis=51-REVIEW WR-02; register row 51 WR-02 (verdicts agree on is_ok, wording and maintenance drift only). Class other, waived at the 57-10 triage (option-a, D-14) on 2026-11-04 and re-homed under Phase 57 D-03. |
| 104 | deviation / 41 | waived | Minted by 57-11 for `41-REVIEW WR-02` (class `other`, 57-10 triage option-a, owner reply of 2026-11-04) and waived in the D-03 shape. No closing evidence needed; the register row reads `waived`, Evidence `WINDOWS row 104; 2026-11-04`. | Re-homed: owner=owner (am0rfu5@protonmail.com), revisit=when boot-time DNS availability is reported as an operator issue, basis=41-REVIEW WR-02; T-41-38 (fail-closed at boot is the recorded intent, the send-time guard re-checks). Class other, waived at the 57-10 triage (option-a, D-14) on 2026-11-04 and re-homed under Phase 57 D-03. |
| 105 | deviation / 41 | waived | Minted by 57-11 for `41-REVIEW WR-03` (class `other`, 57-10 triage option-a, owner reply of 2026-11-04) and waived in the D-03 shape. No closing evidence needed; the register row reads `waived`, Evidence `WINDOWS row 105; 2026-11-04`. | Re-homed: owner=owner (am0rfu5@protonmail.com), revisit=next configuration-docs pass over docs/src/getting-started/configuration.md lines 405-416, basis=41-REVIEW WR-03; MIGRATION M-B-38 (an unparsable env value is ignored, project-wide convention). Class other, waived at the 57-10 triage (option-a, D-14) on 2026-11-04 and re-homed under Phase 57 D-03. |
| 106 | deviation / 41 | waived | Minted by 57-11 for `41-REVIEW WR-04` (class `other`, 57-10 triage option-a, owner reply of 2026-11-04) and waived in the D-03 shape. No closing evidence needed; the register row reads `waived`, Evidence `WINDOWS row 106; 2026-11-04`. | Re-homed: owner=owner (am0rfu5@protonmail.com), revisit=when a PostgreSQL store clock skew is observed on operator notices, basis=41-REVIEW WR-04; register row 41 WR-04 (bounded delay of a best-effort alert). Class other, waived at the 57-10 triage (option-a, D-14) on 2026-11-04 and re-homed under Phase 57 D-03. |
| 107 | deviation / 43 | waived | Minted by 57-11 for `43-REVIEW WR-02` (class `other`, 57-10 triage option-a, owner reply of 2026-11-04) and waived in the D-03 shape. No closing evidence needed; the register row reads `waived`, Evidence `WINDOWS row 107; 2026-11-04`. | Re-homed: owner=owner (am0rfu5@protonmail.com), revisit=next docs pass (a two-sentence fix), basis=43-REVIEW WR-02; crates/paladin-llm/CHANGELOG.md lines 295-301. Class other, waived at the 57-10 triage (option-a, D-14) on 2026-11-04 and re-homed under Phase 57 D-03. |
| 108 | deviation / 43 | waived | Minted by 57-11 for `43-REVIEW WR-07` (class `other`, 57-10 triage option-a, owner reply of 2026-11-04) and waived in the D-03 shape. No closing evidence needed; the register row reads `waived`, Evidence `WINDOWS row 108; 2026-11-04`. | Re-homed: owner=owner (am0rfu5@protonmail.com), revisit=when a caller can hand-compose a NodeCacheKey, or the marker key is next changed, basis=43-REVIEW WR-07; ADR-0058 line 273 (marker location, unreachable from in-tree key producers). Class other, waived at the 57-10 triage (option-a, D-14) on 2026-11-04 and re-homed under Phase 57 D-03. |

## REG-01 SUMMARY frontmatter

Re-run in 57-11 Task 2 on `d0b0022ec`, from the repo root, `node .claude/gsd-core/bin/gsd-tools.cjs query summary-extract <path> --fields requirements_completed` (paths under `.planning/milestones/v0.11.0-phases/`):

| SUMMARY | `requirements_completed` read back |
|---------|------------------------------------|
| `45-rustfs-swap-platform-observability-deviations/45-01-SUMMARY.md` | `["STORE-02"]` |
| `51-adjustable-reasoning-effort/51-13-SUMMARY.md` | `["REASON-02"]` |
| `51-adjustable-reasoning-effort/51-14-SUMMARY.md` | `["REASON-03","REASON-06"]` |
| `51-adjustable-reasoning-effort/51-15-SUMMARY.md` | `["REASON-05"]` |

These match the values 57-01 set (STORE-02, REASON-02, REASON-03 and REASON-06, REASON-05).

## REG-03 debug archive

Re-run in 57-11 Task 2 on `d0b0022ec`: `node .claude/gsd-core/bin/gsd-tools.cjs audit-open`. Output (header and footer trimmed to the sections it printed):

```
Milestone Close: Open Artifact Audit
Pending Todos (5 pending)
   2026-10-20-openai-responses-api-for-pro-tier.md [llm-adapters]
   2026-11-02-setup-check-first-run-output.md [cli]
   2026-11-02-unquoted-treasurer-price-loses-digits-on-load.md [config]
   2026-11-03-devcontainer-install-cargo-llvm-cov.md [devcontainer]
   2026-11-03-docs-deprecated-gpt-4-model-literal.md [docs]
   ... and 2 more
5 items require decisions before close.
```

No open debug session is listed (REG-03). The only section printed is pending todos, which are not a Phase 57 register: they are the project backlog and were not touched.

## REG-04 judgment read

`57-JUDGMENT-READ.md` `## Totals` (read in 57-11 Task 2 on `d0b0022ec`):

- test 13 (P40-01, 02, 03, 04, 05, 07, 08, 09, 10, 11, 13, 15, 16), code 7 (P40-06, 12, 14, 17, 18, 19, 20), checklist 6 (C1..C6)
- confirm 26 (20 prohibitions + 6 checklist rows), reject 0, pending 0, violated 0

The owner's reply of 2026-11-04 (`57-07: confirm all`, recorded in `57-07-SUMMARY.md` `## Owner reply`) is the only human input. The worksheet has 0 `pending` Human cells. `40-VALIDATION.md` line 76, the Manual-Only row for the ~20 judgment-tier prohibitions, had its Test Instructions cell amended in place to `human read recorded 2026-11-04, see .planning/phases/57-register-hygiene-close-out/57-JUDGMENT-READ.md`; the diff of that file is 1 line added and 1 deleted.

## REG-05 review dispositions

Register: `57-REVIEW-DISPOSITIONS.md`, 51 findings across 13 review files. After Task 2: closed 42, waived 8, open 1 (`**Totals:** closed: 42, waived: 8, open: 1`). Six rows closed on gap-closure fixes (51 WR-01 -> 57-12; 45 WR-04 and WR-06 -> 57-13; 41 WR-01 -> 57-14; 43 WR-04 and WR-05 -> 57-15). Eight rows waived with `WINDOWS row 101` to `108` and the owner (38 WR-01, 38 WR-02, 51 WR-02, 41 WR-02, 41 WR-03, 41 WR-04, 43 WR-02, 43 WR-07). 43 WR-01 stays `open` with its `LIVE ` pointer until Task 3 (PD-11-4).

`./scripts/check-review-dispositions.sh --pending-ok` (Task 2): `checked 51 finding(s) across 13 review file(s); 0 failure(s)`.

`./scripts/check-review-dispositions.sh` strict (Task 2), the only failure line plus the last line:

```
FAIL OPEN 43 WR-01: the finding is still open
checked 51 finding(s) across 13 review file(s); 1 failure(s)
```

After 43 WR-01 closed (Task 3, Redis node-cache job `126917997409` read first), `**Totals:** closed: 43, waived: 8, open: 0` and the strict run, no flags, prints:

```
checked 51 finding(s) across 13 review file(s); 0 failure(s)
```

## REG-06 security exceptions

Re-run in 57-11 Task 3 on `4b115d511` (the pushed head; the tree is identical to `d0b0022ec` plus the Task 2 `.planning/` edits):

- `make security` exit 0. `cargo audit`: `Loaded 1403 security advisories (from /root/.cargo/advisory-db)`, `Scanning Cargo.lock for vulnerabilities (691 crate dependencies)`, `warning: 9 allowed warnings found` (the nine `.cargo/audit.toml` ignore entries, each with a register row), no vulnerability. `cargo deny`: `advisories ok, bans ok, licenses ok, sources ok`. The RUSTSEC-2026-0429 row that 57-01 added keeps both legs green.
- `./scripts/check-advisory-register.sh`: `✅ 16 register row(s) checked against 16 deny.toml and 9 .cargo/audit.toml ignore entries; all clauses satisfied.`
- Also inside `make check-gates` (exit 0): `✅ 37 governed dismissal(s) checked; schema, drift, staleness, uniqueness and reachability clauses all pass.` (`check-codeql-dismissals`).

## Phase gates

All on `4b115d511` (code tree identical to the head the rows were re-run on), `CARGO_INCREMENTAL=0`, disk 6.4 GB free before the heavy gates (`df -BM --output=avail /`). Git: `git fetch origin main` then `git rev-list --count HEAD..origin/main` printed `0` (origin/main is `8b23cb4c5`): 0 behind, no merge.

| Gate | Result |
|------|--------|
| `make clean-code` | exit 0: fmt, clippy `-D warnings`, `✅ shellcheck clean`, `cargo check`, doc-check `✅ doc-check passed (ADR-0033)` (default features zero warnings, all features `-D warnings`), `All 105 D-05 public API entry points carry a plural '# Examples' heading.` No file was reformatted (`git status` clean afterwards). |
| `make check-gates` | exit 0: CHANGELOGs (11 crates), crate names (12 crates), advisory register (16 rows), workflow suppressions, workflow triggers, CodeQL dismissals (37), migration allowlist, publish order (12 crates), version strings (128 files, `0.11.0`), `no-stub-panics` (`scanned 627 file(s) (19 test-only), 0 finding(s)`) |
| `make test-shell-guards` | exit 0: `✅ 85 assertion(s) passed.` |
| `make security` | exit 0 (see `## REG-06 security exceptions`) |
| `make api-surface` | exit 0: `✅ API surface extracted ... (4331 items)`, `✅ API surface unchanged` |
| `make check-migration-allowlist` | exit 0: `Allowlist is set-equal to the MIGRATION.md §11.2 deliberate-breaking register (crate-type pairs).` and `Every crate-wide cargo-semver-checks allow is mirrored by an allowlist entry (crate-lint pairs).` |
| `make check-version-strings` | exit 0: `✅ OK: 128 file(s) scanned, every copyable version string equals '0.11.0'` |
| `make test` (`cargo test --workspace --lib --bins`) | exit 0: 14 `test result:` lines, summed `5593 passed; 0 failed; 0 ignored`; `FAILED` lines: 0 |
| `make test-doc` (`cargo test --workspace --doc`) | exit 0: 13 `test result:` lines, summed `689 passed; 0 failed; 210 ignored` (the pre-existing `ignore` doctests); `FAILED` lines: 0 |
| `SEMVER_PRUNE_LOCAL_CACHE=1 ./scripts/check-semver.sh` | exit 0: `semver: 12 package(s) checked against 0.11.0 at --release-type patch; 0 failure(s)`; each of the 12 packages `223 checks: 223 pass, 31 skip` |
| `cargo test -p paladin-storage --features sqlite --lib run::` | `test result: ok. 84 passed; 0 failed` |
| `cargo test -p paladin-web --lib thread_controller::` | `test result: ok. 56 passed; 0 failed` |
| `cargo test --lib --features web-server application::services::run::` | `test result: ok. 276 passed; 0 failed` |

Not a Phase 57 gate: the `pre-commit` workflow on `d0b0022ec` failed on its `end-of-file-fixer` hook, which wanted one trailing blank line removed from `57-JUDGMENT-READ.md` (a 57-07 artefact). Fixed in the seal commit (see `## Handover`).

## CI on the Phase 57 head

Pushed head: `4b115d511878adabe4110d448a19ae531bb94739` (branch `claude/laughing-dirac-e0h2ax`, pushed with `git push -u origin claude/laughing-dirac-e0h2ax`, no force). CI run `41927940508` (`.github/workflows/ci.yml`). Job list read with `gh run view 41927940508 --repo DF3NDR/paladin-dev-env --json jobs` and `gh api repos/DF3NDR/paladin-dev-env/actions/runs/41927940508/jobs`; every job below reports `head_sha` `4b115d511`.

| Job | Conclusion | Job id |
|-----|------------|--------|
| Security Audit | success | 126917997616 |
| License & Dependency Policy | success | 126917997392 |
| Workflow Lint | success | 126917997309 |
| Unit Tests (stable) | success | 126917997444 |
| Unit Tests (beta) | success | 126917997553 |
| Coverage | success (`Lines:     172187/185431 = 92.86%`, gate 82 percent) | 126917997573 |
| Semver Checks (vs v0.11.0) | success | 126917997911 |
| Postgres Storage Contract Suites (live server) | success (`test result: ok. 175 passed; 0 failed`; 0 test-output `SKIP:` lines) | 126917997320 |
| Redis Node Cache Contract Suite (live server) | success (`fenced_write_after_the_counter_expires_is_accepted ... ok`; `21 passed; 0 failed`; 0 test-output `SKIP:` lines; guard step `All node_cache::redis tests exercised the live server.`) | 126917997409 |
| Integration Tests (the RustFS file-storage suite, step `Run file storage contract suite (RustFS)`) | success (`test_list_files_honours_limit_and_drops_no_keys ... ok`, `test_presigned_urls ... ok`, `test result: ok. 12 passed; 0 failed`) | 126917997844 |
| Redis Run Queue Contract Suite (live server) | success | 126917997283 |
| Redis Cadence Contract Suite (live server) | success | 126917997378 |
| Docker Integration Tests | success | 126917997741 |
| Ollama Integration Tests (live server) | success | 126917997510 |
| API Surface Tracking | success | 126917997136 |
| E2E Platform API | success | 126917997244 |
| Crate Isolation (ten packages) | success (all ten) | per job |
| Generated SDK Clients (Python + TypeScript) smoke | failure, step 12 `Run the SDK smoke (both clients against a live server)` | 126917997570 |

Notes:

- The file-storage job is named `Integration Tests` in `ci.yml`, not `File Storage` as the 57-06 SUMMARY wrote. Its step 9 is the RustFS suite.
- The SDK smoke job is the pre-existing red job outside Phase 57's registers (research A6). It is not required by any Phase 57 row and was not touched.
- The `pre-commit` workflow (a separate workflow, not part of `ci.yml`) is red on `4b115d511` and was red on `d0b0022ec`: its `end-of-file-fixer` hook removes a trailing blank line from `57-JUDGMENT-READ.md`. The seal commit carries that one-line fix; it is outside the required job list.
- The `Docker Build` job (a ~60 minute job) was still `in_progress` when the Task 3 rows were closed; it is not one of the required jobs. See `## Handover` for what it read when the seal commit was pushed.

## Orphaned deferrals

Two Phase 55 deferrals were routed to a `56.x` decimal phase that was never created (Phase 56 became dogfood). Neither is a WINDOWS row, so neither affects `open_count`. Nothing is implemented for them in Phase 57 (PD-11-3). Where each sits now:

| Item | Source | Where it sits now | Owner | Revisit |
|------|--------|-------------------|-------|---------|
| The News API key travels in the URL, and `reqwest` error text can echo the URL (`D-52-03-01`, `crates/paladin-content/src/adapters/input/news_api_fetcher.rs`) | `.planning/phases/52-ci-release-pipeline-repair/deferred-items.md` (D-52-03-01); `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/deferred-items.md` (routed to `56.x`); registered CodeQL row 54 names it in its `why_dismissed` and `revisit_condition` | The 52 `deferred-items.md` entry, plus CodeQL dismissal row 54 | DF3NDR (maintainer) | Before the News API fetcher is next changed, or the next minor that touches a third-party fetcher; the fix is the key in an `X-Api-Key` header, the URL stripped from the mapped error, and `redirect::Policy::none()` |
| A `${...}` placeholder in the treasurer webhook `secret` is not refused (Phase 54 WR-04) | `.planning/phases/54-first-run-usability/54-REVIEW.md` `### WR-04`; `.planning/phases/55-correctness-fixes-test-hygiene-lint-bar/deferred-items.md` (owner-deferred, routed to `56.x`) | `54-REVIEW.md` WR-04, owner-deferred | DF3NDR (maintainer) | The next minor, with a new-behaviour plan that also pins the other shipped example configs by test |

57-10 and 57-11 do not fix, waive or re-open either item.

## Handover

none for the registers: `windows status` reads `open_count: 0` and the strict review-dispositions checker reads `0 failure(s)`.

Two observations for whoever runs the phase verification (neither blocks a row):

1. The `pre-commit` workflow failed on `d0b0022ec` and `4b115d511` on `end-of-file-fixer` for `57-JUDGMENT-READ.md` (57-07 artefact, one extra trailing blank line). Fixed in the seal commit; the workflow runs again on the seal push.
2. The WINDOWS plan-verify clause `all("|" not in x["reason"] for x in entries)` is false on the pre-existing fixed row 21 (an August reason that quotes `status|...` vocabulary). None of the rows Phase 57 touched has a pipe in its reason; row 21 was not edited.
