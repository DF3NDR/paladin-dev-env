---
phase: 22
slug: battlefield-state-superstep-engine
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: 2026-09-04
---

# Phase 22 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

**Verification mode.** State B (no prior SECURITY.md; 17 PLAN.md files each carried a
`<threat_model>` block, so the register was authored at plan time). ASVS level 1, block
threshold `high`. Every mitigation was verified at L1 grep depth against the implementation on
branch `feature/phase-22` at `807f7c72`; the four SUMMARY.md `## Threat Flags` sections
(22-01, 22-02, 22-04, 22-05) reported no new threats. Because no threat at or above the block
threshold remained open after classification, the workflow short-circuited past the
`gsd-security-auditor` subagent, as the L1 rule prescribes.

The register below de-duplicates the supply-chain entry each plan named `T-22-SC` by
suffixing the plan number (`T-22-SC-01` … `T-22-SC-17`).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| node output → Battlefield | A node's `StateDelta` (possibly LLM-generated) enters shared typed state through `merge` | arbitrary JSON, may carry secrets or PII placed by a node |
| Battlefield → Waypoint payload | In-memory state crosses into a durable, serialized, later-re-read artifact | serialized `Battlefield`, Vanguard, visit counts, frontier |
| stored Waypoint → resume | Previously persisted data (possibly from an older or newer release, or for a different graph) is trusted back into a live run | schema-versioned JSON, `GraphFingerprint`, per-node visit counts |
| custom resolver / interceptor → merge path | Application-supplied executable code runs inside the core merge path on every superstep, and can alter or suppress a node's delta | closures, `InterceptDecision` |
| caller-supplied ThreadId / WaypointId / keep-set → store and SQL | Arbitrary caller strings become persistence keys, bound query parameters, and DELETE predicates | identifiers, id lists |
| store → caller | `list_threads` enumerates every thread the backend holds; no tenancy filter | thread ids and summaries |
| PR author → CI gate configuration | A contributor can weaken the semver gate by editing the allowlist or workflow in the same PR | allowlist entries, workflow YAML |
| crates.io → semver baseline | An externally published version determines what the gate compares against | published crate versions |
| graph author → engine | A caller-authored graph controls spawn count, iteration bound, which nodes ever run, and is accepted or rejected before any node executes | `WarGraph`, `EngineLimits`, edge conditions |
| node code → shared snapshot | Concurrently executing node implementations share one read snapshot | `Arc<Battlefield>` |
| database URL → error strings and logs | A connection string carrying a password can reach an error envelope | credentials |
| Battlefield → Paladin prompt | Shared state is rendered into a string sent to an external LLM provider | field values chosen by the template author |
| engine → trace sink | Run-internal data crosses into a caller-supplied sink that may export it off-process | `TraceEvent` stream |
| external signal → run lifecycle | A cancellation token from outside the run decides when it stops | `CancellationToken` |
| benchmark harness → filesystem | The SQLite benchmark creates a database file outside process memory | temporary database file |
| legacy execution path → bridged execution path | Two implementations must produce identical observable behaviour for the same input | final output, per-paladin inputs |
| CI runner → throwaway postgres-test container; test log → job verdict | Test credentials cross into a localhost container; the green/red decision is parsed from untrusted test stdout | throwaway credentials, test log text |
| interruption point → stored thread contents / resumability | A crash, cancellation or backend failure can land between any two storage operations inside retention | Waypoint rows |
| application-layer policy → storage adapter | The decision about which Waypoints are protected crosses into the adapter as an argument, never as adapter logic | keep-set of `WaypointId` |
| retention schedule → stored Waypoints | A routine that runs unattended decides what to destroy | Waypoint rows |
| run outcome → downstream consumers | Chronicle, trace export and the eval harness treat the reported outcome as fact | `RunOutcome` |
| test suite / audit record → future maintainers | A suite shaped around a defect reports safety it does not provide; what is written down is treated as settled | test assertions, deferred-items records |
| CI run output → gap-closure claim; executor judgment → binding specification | Evidence closing G-22-1 comes from outside this environment; a newly found defect's registration would amend a developer-owned document | CI run logs, `.project/` defect register |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-22-01 | Tampering | `Waypoint` / `Battlefield` deserialization | medium | mitigate | `schema_version` on every persisted type; unknown version → typed `SchemaVersionUnsupported`, never a panic or silent misparse. Evidence: `battlefield.rs:405-473`, `battlefield_error.rs:67`, `waypoint.rs:511`; tests `from_json_rejects_unsupported_schema_version`, `state_delta_round_trips_and_carries_schema_version` | closed |
| T-22-02 | Information Disclosure | `BattlefieldError` variants | medium | mitigate | No variant embeds a `serde_json::Value`; `TypeMismatch` carries type names, `UnknownField`/`MissingRequiredField` carry field names, `DispatchConflict` carries `NodeId`s. Evidence: enum definition in `battlefield_error.rs` | closed |
| T-22-03 | Spoofing | `resume` against a Waypoint for a different graph | high | mitigate | Stored `GraphFingerprint` compared before any node executes; mismatch → `EngineError::GraphMismatch { expected, got }`. Evidence: `engine/mod.rs:883-884`; test `resume_with_graph_mismatch_fails_without_allow_graph_change` | closed |
| T-22-SC-01 | Tampering | cargo dependency additions | high | mitigate | No new crate name; only the `v7` feature on the already-pinned `uuid 1.8.0`. Evidence: workspace `Cargo.toml:23` | closed |
| T-22-04 | Information Disclosure | `BattlefieldError::TypeMismatch` / `UnknownField` | medium | mitigate | Unit test asserts no error `Display` string contains a value written into state, across all six variants. Evidence: `battlefield.rs:1392` `no_error_display_contains_a_value_placed_in_state` | closed |
| T-22-05 | Denial of Service | `Append` / `MergeObject` field growth | low | accept | Bounded per run by `EngineLimits.max_supersteps` / `max_node_visits` and in storage by `WaypointRetentionConfig`. See Accepted Risks AR-22-01 | closed |
| T-22-06 | Tampering | custom dispatch resolver returning arbitrary state | medium | mitigate | Unregistered `Custom(name)` is a hard `CustomDispatchNotRegistered` error, never a silent fallback; resolver output is schema-checked before storage. Evidence: `battlefield.rs:514-515, 680`; test `merge_custom_dispatch_not_registered_errors` | closed |
| T-22-07 | Information Disclosure | `WaypointPort::list_threads` enumerating all threads | medium | accept | Port rustdoc section "`ThreadId` is not an authorization boundary" states any network exposure must add its own authorization layer. Evidence: `waypoint_port.rs:31-37, 216`. See AR-22-02 | closed |
| T-22-08 | Tampering | `ThreadId` accepting arbitrary caller input | medium | mitigate | Validating constructor rejects empty, whitespace-bearing and over-length (>256 bytes) values with distinct `ThreadIdError` variants. Evidence: `waypoint.rs:30-69`; tests at `waypoint.rs:671-708` | closed |
| T-22-09 | Denial of Service | unbounded `history` / `list_threads` result sets | low | mitigate | Both take `limit: Option<u32>` (zero case contract-tested); SQL backends bind it as a `LIMIT` parameter. Evidence: `waypoint_port.rs:193-219`; `sqlite.rs:37-84, 360-367`; `postgres.rs:33-74` | closed |
| T-22-10 | Tampering | `.cargo/semver-checks-allowlist.toml` | high | mitigate | Per-item schema (one crate, one lint per entry); CI step diffs the allowlist crate set against MIGRATION.md §9.2 deliberate-breaking rows in both directions, so a wildcard entry cannot match any §9.2 row and fails the gate. The plan's zero-blanket-suppression grep ran at execution time (22-04 SUMMARY). Evidence: allowlist header; `ci.yml:351-399` | closed |
| T-22-11 | Tampering | semver baseline resolution | high | mitigate | `--baseline-version 0.9.0` passed explicitly rather than "latest published". Evidence: `ci.yml:275-276, 347` | closed |
| T-22-12 | Repudiation | semver job matching zero packages | high | mitigate | Crates addressed by published package name (`paladin-ai-core`, not the directory name). Evidence: `ci.yml:280, 334` | closed |
| T-22-SC-04 | Tampering | `cargo install cargo-semver-checks` in CI | high | mitigate | Pinned `cargo-semver-checks@0.50.0` via `taiki-e/install-action@v2`; CI-only, not a workspace dependency. Evidence: `ci.yml:322-325` | closed |
| T-22-13 | Denial of Service | unbounded superstep iteration on a cyclic graph | high | mitigate | `max_supersteps` enforced at the top of every iteration → `RecursionLimitExceeded`; `max_node_visits` bounds a single node independently → `NodeVisitLimitExceeded`. Evidence: `superstep.rs:919-921, 976-984`; off-by-one tests at `superstep.rs:5037-5090` | closed |
| T-22-14 | Denial of Service | unbounded task spawn for a large vanguard | medium | mitigate | `tokio::sync::Semaphore` bounds in-flight node executions per superstep, default = Vanguard size, configurable via `WarEngine::with_parallelism`. Evidence: `superstep.rs:30, 1092`; `engine/mod.rs:694`; test `parallelism_limit_bounds_in_flight_execution` | closed |
| T-22-15 | Tampering | a node observing or mutating a peer's mid-superstep state | medium | mitigate | Nodes receive an `Arc<Battlefield>` read snapshot; deltas merge only after the join. Evidence: tests `battlefield_cloned_once_per_superstep_arc_ptr_eq` (`superstep.rs:4825`) and `peer_node_observes_pre_superstep_value_not_siblings_write` (`superstep.rs:4771`) | closed |
| T-22-16 | Repudiation | a run that silently loses its checkpoint | high | mitigate | `WaypointDurability::Strict` turns a save failure into `EngineError::WaypointWrite`; `BestEffort` appears outside its match arm only inside `#[cfg(test)]` (superstep.rs tests start at line 2428; BestEffort uses at 4283 and 4901). Evidence: `engine/mod.rs:88-99`; `superstep.rs:2416-2419` | closed |
| T-22-17 | Tampering | SQL injection via `thread_id` / `waypoint_id` | high | mitigate | Every statement uses `.bind(...)` (26 binds in sqlite.rs, 27 in postgres.rs); no `format!` builds a SELECT/INSERT/UPDATE/DELETE (grep: 0 hits). Metacharacter round-trip tests per backend. Evidence: `sqlite.rs:694, 710`; `postgres.rs:783` | closed |
| T-22-18 | Information Disclosure | database password in a `WaypointError::Backend` string | high | mitigate | `redact_database_url_password` applied before any truncation in both stores' error wrapper. Evidence: `waypoint/mod.rs:24-29`, `sqlite.rs:135-141`, `postgres.rs:120-125`; test `connection_error_redacts_password_from_database_url` (`sqlite.rs:740`) | closed |
| T-22-19 | Tampering | deserializing a payload from an unknown newer release | medium | mitigate | Embedded `schema_version` checked on load; mismatch → `WaypointError::SchemaVersionUnsupported`. Evidence: `sqlite.rs:151-159`; `postgres.rs:138-141` | closed |
| T-22-20 | Denial of Service | unbounded `waypoints` table growth | medium | mitigate | `WaypointRetentionConfig` (`max_age_days`, `max_waypoints_per_thread`) applied by the prune routine; per-run count independently bounded by `EngineLimits`. Evidence: `src/config/waypoint_retention.rs:19`; `retention.rs:75-116` | closed |
| T-22-21 | Denial of Service | retention deleting a checkpoint a human is waiting on | high | mitigate | Protected set (thread's latest + every `AwaitingInput`) is computed once in the application layer and handed to the port as a keep-set the primitive never deletes from (superseding 22-06's in-routine invariants per 22-13/22-14). Evidence: `src/application/services/waypoint_retention.rs:51-58`; tests `protected_set_is_exactly_latest_plus_awaiting_input_over_both_neither_and_each_alone`, `protected_set_covers_a_single_waypoint_that_is_both_latest_and_awaiting_input` | closed |
| T-22-SC-06 | Tampering | new cargo feature pulling a database driver | high | mitigate | `postgres = ["dep:sqlx", "sqlx/postgres"]` on the already-pinned `sqlx 0.8`; `cargo tree -e features -p paladin-ai \| grep -i postgres` → 0 matches recorded in 22-06 SUMMARY. Evidence: `paladin-storage/Cargo.toml:23-25` | closed |
| T-22-22 | Denial of Service | a downstream join stranded by a false branch | high | mitigate | Readiness rule resolves every incoming edge to fired or provably-not-firing (transitively via dead sources) before a join waits. Evidence: `superstep.rs:1754-2045`; tests `false_branch_is_proven_not_firing_and_join_still_runs_once` and `node_fed_only_by_an_unreachable_source_never_runs_and_does_not_stall_its_join` (5-second `tokio::time::timeout` guard) | closed |
| T-22-23 | Denial of Service | a panicking or non-terminating custom dispatch closure | medium | accept | Out of scope per PRD (ENG-FR-21 isolates only the `TraceSink`). Accepted; see AR-22-03 — note the promised rustdoc at the registry definition is **not present** in `dispatch_registry.rs` | closed |
| T-22-24 | Tampering | a custom rule silently shadowing a built-in merge semantic | medium | mitigate | `RESERVED_NAMES = ["LastWrite","Append","MergeObject","Sum"]`; registering one → `EngineError::ReservedDispatchName`; unregistered names fail validation. Evidence: `dispatch_registry.rs:17-22, 46-57` | closed |
| T-22-25 | Tampering | nondeterministic merge across replays | high | mitigate | Deltas sorted by `NodeId`, tasks by `task_key`, frontier in registration order; 20-iteration seeded shuffle test asserts byte-identical final Battlefield. Evidence: `superstep.rs:548-605`; tests `eng_fr_08_determinism_over_twenty_randomized_scheduling_iterations` (`superstep.rs:6123`), `task_key_order_is_stable_across_twenty_shuffled_runs` (`superstep.rs:4597`) | closed |
| T-22-26 | Spoofing | `resume` continuing against a graph the Waypoint was not written for | high | mitigate | Fingerprint compared before any node executes; only bypass is explicit `allow_graph_change`, which then fails precisely with `VanguardNodeMissing { node }`. Evidence: `engine/mod.rs:883-904`; tests at `engine/mod.rs:1592, 1626, 1677` | closed |
| T-22-27 | Elevation of Privilege | a resume resetting per-node visit counts | high | mitigate | `visit_counts` persisted on the Waypoint and restored by `resume`. Evidence: `waypoint.rs:520`; test `resume_restores_visit_counts_and_trips_limit_on_next_post_resume_visit` (`engine/mod.rs:1729`, 4-of-5 case) | closed |
| T-22-28 | Information Disclosure | `InputMapping` rendering a secret-bearing field into an LLM prompt | medium | accept | Template author's explicit choice; control: a placeholder naming an undeclared field is `InputMappingError::UndeclaredField`. Evidence: `input_mapping.rs:21-34, 153-198`. See AR-22-04 | closed |
| T-22-29 | Tampering | a placeholder silently substituting empty text | medium | mitigate | Undeclared field → `UndeclaredField`; declared field with no value and no schema default → `NoValueOrDefault`. Evidence: `input_mapping.rs:51-62, 165-167`; tests at `input_mapping.rs:299-428` | closed |
| T-22-30 | Denial of Service | a slow or blocking `TraceSink` stalling every run | high | mitigate | Fire-and-forget via `try_send` over a bounded drop-oldest queue; the engine never awaits the sink. Evidence: `hooks.rs:14, 139-156`; test `permanently_blocking_trace_sink_does_not_stall_a_real_run` (`engine/mod.rs:2346`, timeout-guarded) | closed |
| T-22-31 | Repudiation | trace events silently discarded | medium | mitigate | Drops counted in `AtomicU64`, readable via `TraceDispatcher::dropped_count`. Evidence: `hooks.rs:51-53, 149, 161` | closed |
| T-22-32 | Information Disclosure | `DeltaMerged` events carrying field values off-process | medium | mitigate | Variant carries `field_changes: Vec<FieldName>` only. Evidence: `trace_sink_port.rs:25, 98-106` | closed |
| T-22-33 | Tampering | a `NodeInterceptor` silently altering a delta or suppressing a node | low | accept | Chain is empty by default; a `Skip` is recorded as `NodeOutcomeKind::Skipped { reason }` in the Waypoint. Evidence: `waypoint.rs:412`; `superstep.rs:1397-1404`. See AR-22-05 | closed |
| T-22-34 | Denial of Service | cancellation leaving unresumable mid-merge state | high | mitigate | Token observed at superstep boundaries only; in-flight superstep completes and merges before the `Halted` Waypoint is written. Evidence: `engine/mod.rs:119-126, 747-751`; tests `cancellation_during_superstep_finishes_it_then_halts_before_the_next`, `resume_continues_a_halted_thread_to_normal_completion` (`engine/mod.rs:2800, 2878`) | closed |
| T-22-35 | Denial of Service | benchmark temp database left behind | low | mitigate | Database created under `std::env::temp_dir()`, pool dropped then `remove_file` on completion. Evidence: `benches/engine_benchmarks.rs:118-122` and cleanup block (comment cites T-22-35) | closed |
| T-22-36 | Repudiation | an unmeasured NFR claim shipped as verified | medium | mitigate | Both NFR figures measured and recorded with the measuring commit; the 73.09 ms p50 overshoot of the 10 ms target is recorded, not tuned away. Evidence: 22-10 SUMMARY key-decisions and headline | closed |
| T-22-37 | Tampering | a bridge silently changing legacy output | high | mitigate | Golden tests compare raw, unnormalised strings with plain `assert_eq!` over final output and ordered per-paladin inputs; module doc records the no-trim/no-fold/no-replace rule. Evidence: `tests/integration/golden_bridge_equivalence_test.rs:5-6, 101-106, 159-169, 241-252, 311-312` | closed |
| T-22-38 | Tampering | unstable legacy-Uuid-to-NodeId mapping | high | mitigate | Mapping is a pure function of Campaign content. Evidence: `bridges.rs:148-149`; test `same_campaign_built_twice_yields_identical_node_ids_and_fingerprint` (`bridges.rs:586-594`) | closed |
| T-22-39 | Repudiation | coverage raised by assertion-free tests | medium | mitigate | No tests were added to reach the floor; deliberately unmeasured paths recorded with reasons. Evidence: 22-11 SUMMARY lines 220-222 | closed |
| T-22-40 | Repudiation | a CI job reporting green having exercised nothing | high | mitigate | Three gates: `pg_isready` before tests, early-return marker must be absent, passed-count must equal the module's declared test count. Evidence: `ci.yml:814-882`; executed run 33685990248 recorded in 22-17 SUMMARY (26/26 passed) | closed |
| T-22-41 | Information Disclosure | Postgres credentials in workflow text | low | accept | `paladin:paladin` is the throwaway credential already published in `docker/docker-compose.test.yml:130-131` for a job-scoped tmpfs container. See AR-22-06 | closed |
| T-22-42 | Denial of Service | the started container outliving the job | low | mitigate | `if: always()` step runs `docker compose … down -v`. Evidence: `ci.yml:802-803` | closed |
| T-22-SC-12 | Tampering | new actions or packages pulled by the CI job | high | mitigate | Only `actions/checkout@v5`, `dtolnay/rust-toolchain@stable`, `Swatinem/rust-cache@v2` (all pre-existing) and `postgres:16-alpine` (already in compose). No package-manager install. Evidence: `ci.yml` job steps; `docker-compose.test.yml:125` | closed |
| T-22-43 | Denial of Service | destruction of a protected Waypoint by an interrupted prune | critical | mitigate | `prune_thread` never removes an id present in `keep`; monotone/idempotent/convergent contract documented on the port. Evidence: `waypoint_port.rs:249-295`; contract tests `prune_thread_keeps_named_ids_byte_identical_and_ordered` etc.; fault-injection Part A (`tests/integration/waypoint_retention_fault_injection_test.rs:236`) | closed |
| T-22-44 | Tampering | SQL injection through `thread_id` or a `keep` element | high | mitigate | Every id bound as a parameter in both overrides; per-backend metacharacter round-trip tests. Evidence: `sqlite.rs:710`, `postgres.rs:783` (`prune_thread_thread_id_and_waypoint_id_with_sql_metacharacters_round_trip_as_data`) | closed |
| T-22-45 | Denial of Service | keep-set exceeding the driver's parameter limit | medium | mitigate | Postgres binds the keep-set as one text array; SQLite chunks deletes at 500 ids inside a single transaction (under the 999 historical limit). Evidence: `postgres.rs:405-423`; `sqlite.rs:432-470`; shared 1,200-Waypoint contract test wired in both backends (`sqlite.rs:647`, `postgres.rs:720`) | closed |
| T-22-46 | Elevation of Privilege | retention policy drifting into the storage adapter | medium | mitigate | Port rustdoc: "Policy-free. This port does not know what 'protected' means"; definition lives in `src/application/services/waypoint_retention.rs`. Evidence: `waypoint_port.rs:270-271`; `retention.rs:4-9` | closed |
| T-22-SC-13 | Tampering | package-manager installs | low | accept | No new dependency; overrides use already-pinned `sqlx`. See AR-22-07 | closed |
| T-22-47 | Denial of Service | loss of the checkpoint a suspended thread resumes from | critical | mitigate | Protected set always contains latest + every `AwaitingInput`; port primitive never deletes a keep-set id. Evidence: `waypoint_retention.rs:51-58`; fault-injection Parts A and B (`…fault_injection_test.rs:236, 415`) | closed |
| T-22-48 | Denial of Service | whole-thread delete still reachable from the prune path | high | mitigate | Sequence removed from the routine; `RecordingStore` double asserts `delete_thread_calls().is_empty()` during a prune. Evidence: `retention.rs:19-22, 206-225, 618` | closed |
| T-22-49 | Tampering | protected-set definition drifting as new classes appear | medium | mitigate | One definition, `protected_waypoints`, with the two not-yet-existing classes (unresolved Parley, active fork lineage) named as Phase 24 seams. Evidence: `waypoint_retention.rs:25-49` | closed |
| T-22-50 | Repudiation | claiming crash-safety from a comment rather than a test | high | mitigate | Executable fault-injection sweep (Part A), resume-after-interrupted-prune (Part B), and task-abort sweep against the transactional SQLite override (Part C); module doc rewritten to point at the test. Evidence: `…fault_injection_test.rs:236, 415, 517`; `retention.rs:27-36` | closed |
| T-22-SC-14 | Tampering | package-manager installs | low | accept | No new dependency; service uses `paladin-ports` and the existing config type. See AR-22-08 | closed |
| T-22-51 | Repudiation | a run reporting completion over a node that could never execute | high | mitigate | `validate` rejects every node outside the eligible set (entry ∪ dynamic targets worklist) with `EngineError::UnreachableNode` before execution. Evidence: `graph.rs:294-304, 359, 831-842`; tests at `graph.rs:2326-2594` | closed |
| T-22-52 | Tampering | dynamic-target marker used to silence the check | medium | mitigate | `mark_dynamic_target` rustdoc'd as an explicit declaration whose targets a later phase validates at runtime. Evidence: `graph.rs:281-304, 374-388` | closed |
| T-22-53 | Denial of Service | validation cost on a large graph | low | accept | Single worklist traversal bounded by nodes + edges, run once before execution. See AR-22-09 | closed |
| T-22-54 | Repudiation | unsound inference of jump targets from parser output | medium | mitigate | Rustdoc records that targets are deliberately never inferred from directive parsers because they are runtime values. Evidence: `graph.rs:374-380` | closed |
| T-22-SC-15 | Tampering | package-manager installs | low | accept | Traversal uses `std` collections only. See AR-22-10 | closed |
| T-22-55 | Repudiation | audit satisfying its checkbox while leaving a same-class defect unrecorded | high | mitigate | Nine self-loop fixtures across four files classified into strandedness dodge / readiness dodge / unrelated with per-row evidence; residual defect captured as a runnable reproduction. Evidence: 22-16 SUMMARY; `22-deferred-items.md` classification table | closed |
| T-22-56 | Tampering | pinning current wrong behaviour as expected | high | mitigate | Reproduction `self_looping_node_fed_by_upstream_edge_can_never_take_first_turn` asserts correct behaviour. It shipped `#[ignore]`d (commit `9e9bdb59`); Phase 22.1 fixed the defect (`a64e5b0d`, starvation-release pass) and the test now runs un-ignored and green, still asserting the correct outcome — never inverted. Evidence: `superstep.rs:5138-5150`; run on 2026-09-04: `cargo test -p paladin-battalion --lib self_looping_node_fed_by_upstream_edge_can_never_take_first_turn` → `1 passed; 0 failed; 0 ignored` | closed |
| T-22-57 | Tampering | weakening fixture assertions while cleaning up | medium | mitigate | Comments corrected in place; no test logic changed, no assertion weakened. Evidence: 22-16 SUMMARY line 147 | closed |
| T-22-58 | Elevation of Privilege | amending a binding specification without the developer | medium | mitigate | No file under `.project/` edited by 22-16; disposition deferred to the 22-17 checkpoint. Evidence: 22-16 SUMMARY line 40; 22-17 SUMMARY lines 79-83 | closed |
| T-22-SC-16 | Tampering | package-manager installs | low | accept | Tests, comments and planning records only. See AR-22-11 | closed |
| T-22-59 | Repudiation | closing G-22-1 on a job's existence rather than execution | high | mitigate | Checkpoint recorded run 33685990248 with `pg_isready` "accepting connections", no early-return marker, `26 passed` equal to the declared count. Evidence: 22-17 SUMMARY lines 46-68, 98 | closed |
| T-22-60 | Elevation of Privilege | registering a new defect into a binding spec without the developer | medium | mitigate | Developer decision recorded 2026-09-02: registered and scheduled to inserted Phase 22.1; `.project/` untouched by the checkpoint. Evidence: 22-17 SUMMARY lines 31, 79-83 | closed |
| T-22-SC-17 | Tampering | package-manager installs | low | accept | Reads a CI run and records a decision; installs nothing. See AR-22-12 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-22-01 | T-22-05 | Per-field growth of `Append`/`MergeObject` is bounded per run by `EngineLimits` and in storage by `WaypointRetentionConfig`; the PRD requires no per-field cap | Plan 22-02 threat register | 2026-09-01 |
| AR-22-02 | T-22-07 | No network surface ships in this epic (V4 Access Control not applicable). Control: `waypoint_port.rs` rustdoc states `ThreadId` is not an authorization boundary and any network exposure must add its own authorization layer | Plan 22-03 threat register | 2026-09-01 |
| AR-22-03 | T-22-23 | PRD requires panic/slowness isolation only for the `TraceSink` (ENG-FR-21); wrapping arbitrary custom dispatch closures is out of scope. **Gap:** the plan's accompanying control — documenting at the registry definition that a custom rule runs unguarded on the critical path — is absent from `crates/paladin-battalion/src/engine/dispatch_registry.rs`. Non-blocking (medium); recommended follow-up: add that rustdoc sentence | Plan 22-07 threat register | 2026-09-01 |
| AR-22-04 | T-22-28 | Which fields a template renders into an outbound prompt is the graph author's explicit, visible choice; the engine adds no implicit field. Control: undeclared placeholders are `InputMappingError::UndeclaredField` | Plan 22-08 threat register | 2026-09-01 |
| AR-22-05 | T-22-33 | Interceptor chain is empty by default and application-populated; a `Skip` is recorded as `Skipped { reason }` in the Waypoint, so suppression is auditable | Plan 22-09 threat register | 2026-09-01 |
| AR-22-06 | T-22-41 | `paladin:paladin` is the throwaway credential already published in `docker/docker-compose.test.yml` for a tmpfs-backed container scoped to the CI job; it grants nothing outside the runner | Plan 22-12 threat register | 2026-09-02 |
| AR-22-07 | T-22-SC-13 | No new dependency: overrides use the already-pinned `sqlx` | Plan 22-13 threat register | 2026-09-02 |
| AR-22-08 | T-22-SC-14 | No new dependency: service uses `paladin-ports` and existing config; tests use the present tokio/SQLite stack | Plan 22-14 threat register | 2026-09-02 |
| AR-22-09 | T-22-53 | Single worklist traversal bounded by nodes + edges, run once before execution; negligible beside one LLM call | Plan 22-15 threat register | 2026-09-02 |
| AR-22-10 | T-22-SC-15 | No new dependency: `std` collections only | Plan 22-15 threat register | 2026-09-02 |
| AR-22-11 | T-22-SC-16 | No dependency changes: tests, comments and planning records only | Plan 22-16 threat register | 2026-09-02 |
| AR-22-12 | T-22-SC-17 | No source changes; reads a CI run and records a decision | Plan 22-17 threat register | 2026-09-02 |

*Accepted risks do not resurface in future audit runs.*

---

## Verification Notes

- **Mitigation relocation (T-22-21).** Plan 22-06 placed the "never the latest, never `AwaitingInput`" invariants inside the storage prune routine. Plans 22-13/22-14 deliberately moved that definition to the application layer (`protected_waypoints`) and reduced the storage routine to a policy-free keep-set primitive (T-22-46, T-22-49). The threat is closed by the relocated control, not by the original one.
- **Mitigation evolution (T-22-56).** The `#[ignore]`d reproduction was the plan's control against inverting a test to match wrong behaviour. Phase 22.1 fixed the underlying defect on this branch, so the reproduction now runs in the default suite and passes while still asserting the correct outcome. The threat's intent (never pin wrong behaviour) remains satisfied.
- **Documentary control gap (T-22-23 / AR-22-03).** Accepted risk stands; the promised rustdoc at the registry definition is missing. Recommended one-line follow-up, non-blocking.
- **Wildcard rejection (T-22-10).** No dedicated wildcard grep runs in CI; the set-equality diff against MIGRATION.md §9.2 is the standing structural guard, and the zero-blanket-suppression grep was executed at plan time (22-04 SUMMARY).
- **Not re-run here:** `make security` (cargo-audit + cargo-deny) is a phase-seal control owned by the verification/ship workflow, not by this threat-mitigation audit.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-09-04 | 69 | 69 | 0 | /gsd-secure-phase orchestrator (L1 grep-depth, ASVS L1; auditor short-circuited per plan-time register rule) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-09-04
