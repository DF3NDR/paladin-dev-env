# Phase 56 Dogfood Evidence

Plan 56-04 re-checks the four pre-seeded findings (DF-01 to DF-04) on HEAD `6212e3ecf` (full SHA
`6212e3ecf95f2db612f86a525b85d60f6fa0033c`) on 2026-11-02 (UTC) before any closing disposition is written
(D-00e, D-02, P4). Each row records the command, its exit code and its result line. Every named test was
confirmed as `<name> ... ok` in the captured output, so no row rests on a filter that matched nothing.

## Prove-then-lock (D-00e): DF-01…DF-04

| DF | owner (requirement, phase) | command | exit | result line | HEAD |
|----|----------------------------|---------|------|-------------|------|
| DF-01 | FIRST-01 (Phase 54) | `cargo test -p paladin-ai --lib --features web-server -- validate_refuses_an_unexpanded_placeholder_key validate_refuses_a_placeholder_key_while_auth_is_disabled validate_names_a_later_entry_whose_key_is_a_placeholder validate_reports_a_placeholder_key_before_tenant_and_duplicate_checks yaml_env_placeholder_is_not_expanded` (run together with the four DF-04 root-lib tests below; 9 tests in one run) | 0 | `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 1486 filtered out` (the five DF-01 names each `... ok`) | 6212e3ecf |
| DF-01 | FIRST-01 (Phase 54) | `cargo test -p paladin-ai --bin paladin-server --features web-server -- build_auth_config_refuses_an_unexpanded_placeholder_key` | 0 | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out` | 6212e3ecf |
| DF-01 | FIRST-01 (Phase 54) | `grep -nE '(^\s*\|[{,]\s*\|-\s+)key:\s*"?\$\{' config.example.yml` | 1 | no output: the example ships no `${...}` key | 6212e3ecf |
| DF-01 | FIRST-01 (Phase 54) | `test ! -e k8s/server/configmap.yaml` | 0 | file absent (the ConfigMap that claimed load-time expansion was retired) | 6212e3ecf |
| DF-02 | FIRST-02, FIRST-03 (Phase 54) | `make check-version-strings` | 0 | `OK: 128 file(s) scanned, every copyable version string equals '0.11.0'` | 6212e3ecf |
| DF-02 | FIRST-02, FIRST-03 (Phase 54) | `./tests/scripts/check-version-strings_test.sh` | 0 | `37 assertion(s) passed.` | 6212e3ecf |
| DF-02 | FIRST-02 (Phase 54) | `grep -n 'Current version' README.md` | 0 | `185:Current version: **0.11.0**.` | 6212e3ecf |
| DF-03 | FIXES-01 (Phase 55) | `make check-no-stub-panics` | 0 | `OK: scanned 624 file(s) (19 test-only), 0 finding(s)` | 6212e3ecf |
| DF-03 | FIXES-01 (Phase 55) | `bash tests/scripts/check-no-stub-panics_test.sh` | 0 | `88 assertion(s) passed.` | 6212e3ecf |
| DF-03 | FIXES-01 (Phase 55) | `test ! -e crates/paladin-content/src/services/content_filtering_service.rs` | 0 | file absent | 6212e3ecf |
| DF-04 | TENANT-04, TENANT-05 (Phase 53) | same root-lib run as DF-01 (`cargo test -p paladin-ai --lib --features web-server -- …`), names `cross_tenant_submit_on_a_thread_is_the_unknown_thread_404 cross_tenant_fork_is_the_unknown_thread_404 cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag cancel_of_another_tenants_run_is_not_found_and_writes_nothing` | 0 | `test result: ok. 9 passed; 0 failed` (the four DF-04 names each `... ok`) | 6212e3ecf |
| DF-04 | TENANT-03, TENANT-07 (Phase 53) | `cargo test -p paladin-web --lib -- every_thread_id_route_hides_foreign_threads_behind_the_missing_thread_404 job_poll_matrix_scopes_by_agent_and_tenant_with_an_admin_bypass` | 0 | `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 327 filtered out` | 6212e3ecf |
| DF-04 | TENANT-03 (Phase 53) | `cargo test -p paladin-ai-core --lib -- permits_thread_admits_all_and_only_the_latest_runs_tenant` | 0 | `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 750 filtered out` | 6212e3ecf |

Notes per entry:

- DF-01: the process-level boot refusal and the watched-red record are in
  `.planning/phases/54-first-run-usability/54-FIRST-EVIDENCE.md` (section "Boot refusal (SC1)" and the
  "Prove-then-lock (D-00f)" table). `yaml_env_placeholder_is_not_expanded` still pins that the loader does no
  `${VAR}` expansion, which is why the remedy is a boot-time refusal, the second of the two remedies the entry
  offered.
- DF-02: the gate is the real fix the entry asked for ("a gate is the real fix"); it is green on HEAD and its own
  test script passes. Historical `**Since:** v0.10.0` statements are exempt by design (54-04).
- DF-03: the file that held the `todo!()` constructors was deleted in Phase 55, which meets the entry's remedy
  ("remove or return `Err`"); the stub-panic gate holds the line with 0 findings across 624 scanned files.
- DF-04: the isolation evidence and watched-red record are in
  `.planning/phases/53-tenant-isolation-closure/53-TENANT-EVIDENCE.md` and `53-VERIFICATION.md`; the tests above
  are the same tests re-run on today's tree.

Disk: scoped `-p <pkg>` builds only. Free space on `/` was 2993M before the first cargo command and 2402M after
the last; no cleanup or retry was needed, and no run failed.
