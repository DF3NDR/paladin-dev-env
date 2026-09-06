---
phase: 25-node-level-fault-tolerance
plan: 13
subsystem: infra
tags: [node-cache, aegis, superstep-engine, cache-key, fingerprint, schema-marker, fault-tolerance]

# Dependency graph
requires:
  - phase: 25-node-level-fault-tolerance (plan 25-04)
    provides: "CachedDelta / NODE_CACHE_SCHEMA_VERSION (core), NodeCachePort / NodeCacheKey / NodeCacheError (ports), InMemoryNodeCache + RedisNodeCache (storage) -- the port and backends this plan wires into the engine"
  - phase: 25-node-level-fault-tolerance (plan 25-11)
    provides: "The post-25-10/25-11 retry loop and Succeeded-arm bookkeeping (handled_failure, muster delta-only guard) the hit/miss path is wired around"
  - phase: 25-node-level-fault-tolerance (plan 25-03)
    provides: "The Aegis sidecar, aegis_for resolution, the v5 fingerprint (which already hashes each node's cache policy) and the list-every-offender validation discipline"
  - phase: 25-node-level-fault-tolerance (plan 25-01)
    provides: "CachePolicy { ttl, key: CacheKeySpec::{Default, Fields} } on Aegis"
provides:
  - "paladin_core::platform::container::battlefield::CacheMarker { Allow (default), Deny } and the additive #[serde(default)] FieldSpec.cache field plus FieldSpec::with_cache; BATTLEFIELD_SCHEMA_VERSION unchanged"
  - "WarEngine::with_node_cache(Arc<dyn NodeCachePort>) -- the backend stored on the engine, threaded through superstep::run / run_with_namespace and inherited wholesale by every NodeSpec::Battalion child run via ChildEngineResources"
  - "EngineError::CachePolicyWithoutCacheBackend { nodes, reason } via WarGraph::validate_node_cache_backend(cache_configured) -- fail-closed, walks Battalion children, called by start/resume/resume_with_options/resume_with/fork/replay right after WarGraph::validate"
  - "EngineError::CachePolicyOnDeniedField { offenders, reason } and EngineError::CacheKeyFieldUndeclared { offenders, reason } via WarGraph::validate_aegis_cache_fields (inside WarGraph::validate)"
  - "paladin_battalion::engine::cache_key -- D-28's composition over the length-prefixed push_field discipline with a blake3 digest; NODE_CACHE_KEY_VERSION, graph_prefix(), node_prefix() (exact by construction) for invalidate(prefix) callers"
  - "The hit/miss path in superstep: lookup BEFORE attempt 1 and OUTSIDE the interceptor chain; a hit merges the stored delta as Succeeded / attempt 1 / cache_hit: true with exactly one NodeStarted/NodeFinished { cache_hit: true } pair and no port call; put only after a successful Edges-routed attempt (never a failure, a handler outcome, a routing directive, or a delta touching a Deny field); get error = miss, put error = logged"
  - "Test double engine::test_support::RecordingNodeCache (paused-clock expiry, injectable get/put failure, set_expires_at_for_all for the engine-side boundary case)"
affects: [25-14-config-and-migration-docs]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Cache lookup/store as an Aegis concern wrapping OUTSIDE the interceptor chain (D-14): a hit runs no before/after interceptor because nothing executes; the stored delta is the post-`after` delta so a hit replays exactly what the miss merged"
    - "Key prefix exactness by length-prefixing the human-readable node id (`{k1}:{fingerprint}:{len}:{node}:{digest}`) so node_prefix('a') can never sweep node 'a:b' -- the same collision reasoning as fingerprint v2, applied to the addressable prefix rather than the hash"
    - "Fail-closed at every layer a Deny can be known: Paladin output_field statically at validation, Function-node write set dynamically at store time"
    - "Test-double expiry on tokio's clock (tokio::time::Instant) alongside the chrono expires_at, so the TTL test elapses time with tokio::time::advance under start_paused and the engine-side closed-boundary test moves only the chrono stamp"

key-files:
  created:
    - crates/paladin-battalion/src/engine/cache_key.rs
  modified:
    - crates/paladin-core/src/platform/container/battlefield.rs
    - crates/paladin-battalion/src/engine/graph.rs
    - crates/paladin-battalion/src/engine/mod.rs
    - crates/paladin-battalion/src/engine/superstep.rs
    - crates/paladin-battalion/src/engine/test_support.rs
    - crates/paladin-battalion/Cargo.toml
    - Cargo.lock
    - src/application/services/parley/adapter.rs

key-decisions:
  - "The engine-side backend check is a separate public WarGraph::validate_node_cache_backend(bool) the engine calls immediately after WarGraph::validate at all four entry points, rather than smuggling a non-registry flag into EngineRegistries; it recurses into Battalion children (Arc-identity bounded) because a child run inherits the cache wholesale"
  - "A Function node's write set is not statically knowable, so CachePolicyOnDeniedField covers the Paladin output_field only; the Function-node half of FT-FR-20's Deny guarantee is enforced at store time (store_node_cache refuses a delta touching any non-Allow field), pinned by a_function_delta_touching_a_deny_field_is_never_stored"
  - "Added EngineError::CacheKeyFieldUndeclared (Rule 2): a CacheKeySpec::Fields naming an undeclared field would silently contribute an absent marker to every key and serve stale hits on a typo"
  - "CacheKeySpec::Fields on a Paladin node ADDS the listed fields to the rendered input rather than replacing it -- the key is never narrower than the input the Paladin saw; on a Function node it narrows the full-snapshot default as D-28 specifies"
  - "put only for an Edges-routed successful directive: CachedDelta stores a delta alone, and replaying only the delta of a Goto/End/Parley/Muster directive would silently drop its routing (pinned by a_routing_directive_is_never_stored)"
  - "A hit whose CachedDelta or wrapped StateDelta carries a different schema_version than this build is treated as a miss, alongside the closed expires_at re-check -- defence in depth over whatever a backend served"
  - "The muster task_key is hashed alongside the payload (a worker may read ctx.task_key()); a cosmetic Paladin rename (name) is deliberately NOT in the config fingerprint"
  - "blake3 added as a dependency EDGE on paladin-battalion (same pinned 1.8.2 paladin-core already uses; no new package enters Cargo.lock) rather than abusing GraphFingerprint::from_canonical_bytes as a generic digest"
  - "FieldSpec::new keeps its four-argument shape (cache defaults to Allow) plus a with_cache builder; the two struct-literal FieldSpec sites in src/application/services/parley/adapter.rs gained the field (Rule 3)"

requirements-completed: [FT-06]

coverage:
  - id: D1
    description: "FieldSpec.cache: CacheMarker { Allow (default), Deny }, additive, schema version unchanged"
    requirement: FT-06
    verification:
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battlefield.rs#tests::field_spec_cache_marker_defaults_to_allow"
        status: pass
      - kind: unit
        ref: "crates/paladin-core/src/platform/container/battlefield.rs#tests::schema_version_is_unchanged_by_the_cache_marker"
        status: pass
      - kind: unit
        ref: "crates/paladin-storage (three-backend Waypoint contract suite, cargo test -p paladin-storage --features sqlite --lib: 122 passed)"
        status: pass
    human_judgment: false
  - id: D2
    description: "Fail-closed validation: CachePolicy with no backend (incl. inside a Battalion child), on a Deny output_field, on an undeclared Fields name; every offender in one error; Append at Allow validates"
    requirement: FT-06
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::a_cache_policy_without_an_engine_cache_fails_validation"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::a_cache_policy_on_a_deny_output_field_fails_validation"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::validation_lists_every_cache_offender"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::an_append_dispatch_field_may_still_be_cached_when_marked_allow"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::a_cache_policy_inside_a_battalion_child_fails_without_a_backend"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/mod.rs#tests::node_cache_validation_tests::a_cache_key_spec_naming_an_undeclared_field_fails_validation"
        status: pass
    human_judgment: false
  - id: D3
    description: "Key composition per D-28: graph fingerprint, Paladin config fingerprint, stability, full-snapshot default, Fields narrowing, muster payload, exact prefixes, no cross-kind collision"
    requirement: FT-06
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/cache_key.rs#tests::key_includes_the_graph_fingerprint"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/cache_key.rs#tests::key_changes_when_the_system_prompt_changes"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/cache_key.rs#tests::key_is_stable_across_runs_for_identical_inputs"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/cache_key.rs#tests::function_node_key_defaults_to_the_full_snapshot"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/cache_key.rs#tests::cache_key_spec_fields_narrows_the_key"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/cache_key.rs#tests::muster_payload_is_included_when_present"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/cache_key.rs#tests::node_prefix_is_exact_under_delimiter_bearing_ids"
        status: pass
    human_judgment: false
  - id: D4
    description: "Hit/miss path: hit merges with zero executions (Succeeded/attempt 1/cache_hit true, one NodeStarted/NodeFinished{cache_hit:true} pair, interceptors bypassed); miss executes once and puts once with the TTL; failures, routing directives and Deny-touching deltas never stored; empty delta is a hit"
    requirement: FT-06
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_hit_merges_the_stored_delta_with_no_execution"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_hit_emits_node_started_and_finished_with_cache_hit_true"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_miss_executes_and_stores_the_successful_delta_with_the_ttl"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_failed_attempt_is_never_stored"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::an_empty_cached_delta_is_a_hit_that_merges_nothing"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_function_delta_touching_a_deny_field_is_never_stored"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_hit_bypasses_the_interceptor_chain"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_routing_directive_is_never_stored"
        status: pass
    human_judgment: false
  - id: D5
    description: "Correctness under time, change and backend failure: TTL expiry (paused clock), the closed boundary at exactly expires_at (engine side), prompt change, graph change, put failure -> Completed, get failure -> miss, per-task muster keys, a hit consumes no retry budget"
    requirement: FT-06
    verification:
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::ttl_expiry_re_executes_the_node"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::an_entry_at_exactly_its_expiry_is_a_miss"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::changing_the_system_prompt_re_executes"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::changing_the_graph_re_executes"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_put_failure_leaves_the_run_completed"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_get_failure_is_a_miss_not_an_error"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_cached_node_inside_a_muster_keys_per_task"
        status: pass
      - kind: unit
        ref: "crates/paladin-battalion/src/engine/superstep.rs#tests::a_cache_hit_consumes_no_retry_budget"
        status: pass
    human_judgment: false

duration: 35min
completed: 2026-09-06
status: complete
---

# Phase 25 Plan 13: Node-Cache Engine Integration Summary

**Wired plan 25-04's `NodeCachePort` into the WarEngine: `with_node_cache`, a fail-closed `CachePolicy` validation family (no backend, denied field, undeclared key field), D-28's length-prefixed blake3 key composition over graph fingerprint + node id + resolved input + Paladin config fingerprint, and a lookup-before-attempt-1 hit path that merges a stored delta with zero executions and records `cache_hit: true`, storing only successful `Edges`-routed deltas that touch no `cache: Deny` field.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-09-06T01:31:00Z (worktree spawn)
- **Completed:** 2026-09-06T02:05:00Z
- **Tasks:** 3
- **Files modified:** 9 (1 created, 8 modified)

## Accomplishments

- `FieldSpec` gained `#[serde(default)] cache: CacheMarker { Allow (default), Deny }` (`#[non_exhaustive]`, any non-`Allow` variant treated as a denial), with `FieldSpec::new` keeping its four-argument shape and a `with_cache` builder. `BATTLEFIELD_SCHEMA_VERSION` is unchanged; a pre-marker payload deserializes to `Allow`, and the three-backend Waypoint contract suite in `paladin-storage` stays green (122 passed). The `Append`-dispatch replay hazard on forks is documented on `CacheMarker` for plan 25-14's guide, and an `Append` field left at `Allow` validates (opt-in denial, FT-FR-20).
- `WarEngine::with_node_cache(Arc<dyn NodeCachePort>)` stores the backend on the engine; it is a new always-present parameter of `superstep::run`/`run_with_namespace` (the `shutdown_grace` precedent) and is inherited wholesale by every `NodeSpec::Battalion` child run through `ChildEngineResources::node_cache`.
- Three fail-closed clauses: `WarGraph::validate_node_cache_backend(bool)` (called by `start`, `resume_with_options`, `resume_with`, `fork`/`replay` right after `validate`; recurses into Battalion children, naming a child node `outer/inner`) raises `CachePolicyWithoutCacheBackend { nodes }` listing every offender sorted; `validate_aegis_cache_fields` (inside `validate`, ahead of the per-policy value checks) raises `CachePolicyOnDeniedField` for a Paladin `output_field` marked `Deny` and `CacheKeyFieldUndeclared` for a `CacheKeySpec::Fields` name the schema does not declare.
- `engine::cache_key`: `key = H(graph_fingerprint, node_id, input_component, paladin_config_fingerprint)` over the same `push_field` length-prefixed stream `WarGraph::fingerprint` uses (now `pub(crate)`), blake3-digested, rendered as `k1:{fingerprint}:{len}:{node}:{digest}` so `graph_prefix`/`node_prefix` are exact addressing prefixes for `invalidate`. The input component is the rendered `InputMapping` string for a Paladin node and the full snapshot (schema declaration order, present/absent-tagged, canonical JSON values) for a Function node under `Default`, narrowed by `Fields`; the muster `task_key` + payload are hashed whenever present; the Paladin fingerprint hashes model, system prompt, temperature (exact `f32` bits), `max_loops` (serde-canonical) and stop words.
- The hit/miss path in `superstep`: the lookup runs before attempt 1 and outside the interceptor chain (D-14); a live hit (`is_expired_at(Utc::now())` re-checked, schema versions re-checked) returns a `Succeeded` `Edges` directive on attempt 1 with `cache_hit: true`, `token_count: 0`, emitting exactly one `NodeStarted`/`NodeFinished { cache_hit: true }` pair; `NodeTaskOutput.cache_hit` flows to the `NodeExecutionRecord`. `put` happens only after a successful attempt, after the `after` interceptors, only for an `Edges`-routed directive, and never for a delta touching a non-`Allow` field. A `get` error is a logged miss; a `put` error is logged and the run continues.
- `RecordingNodeCache` (in-crate, `HashMap`-backed) records every `get`/`put`, expires on tokio's clock so `ttl_expiry_re_executes_the_node` uses `tokio::time::advance` under `start_paused` (no `std::thread::sleep` anywhere), fails `get`/`put` on demand, and can move every entry's `expires_at` so `an_entry_at_exactly_its_expiry_is_a_miss` isolates the engine's closed-boundary check from the backend's.
- 32 new tests across core, `cache_key`, `engine::mod` and `engine::superstep`; every workspace verification command passes (see Self-Check).

## Task Commits

1. **Task 1: The schema cache marker, the fail-closed cache validation clauses, `with_node_cache` and the engine wiring** — `08fd78df` (feat)
2. **Task 2: Key composition and the hit/miss path tests** — `18d71e63` (test)
3. **Task 3: Cache correctness under time, change and backend failure** — `2bd0e162` (test)

**Plan metadata:** (this commit, `docs(25-13): ...`)

## Files Created/Modified

- `crates/paladin-core/src/platform/container/battlefield.rs` — `CacheMarker`, `FieldSpec.cache`, `FieldSpec::with_cache`, two tests
- `crates/paladin-battalion/src/engine/cache_key.rs` — key composition, `NODE_CACHE_KEY_VERSION`, `graph_prefix`/`node_prefix`, eight tests
- `crates/paladin-battalion/src/engine/graph.rs` — `validate_aegis_cache_fields`, `validate_node_cache_backend` + `collect_cache_policy_nodes`, `push_field` made `pub(crate)`, one test call site
- `crates/paladin-battalion/src/engine/mod.rs` — `pub mod cache_key`, three `EngineError` variants, `WarEngine.node_cache` + `with_node_cache` (doc-tested), validation and run call sites, `node_cache_validation_tests`
- `crates/paladin-battalion/src/engine/superstep.rs` — `NodeCacheBinding`, `compose_node_cache_key`, `lookup_node_cache`, `store_node_cache`, `NodeTaskOutput.cache_hit`, the hit path and the put site, `ChildEngineResources.node_cache`, 13 test call sites, 16 tests
- `crates/paladin-battalion/src/engine/test_support.rs` — `RecordingNodeCache`
- `crates/paladin-battalion/Cargo.toml` / `Cargo.lock` — `blake3 = "1.8.2"` dependency edge (already-resolved workspace package)
- `src/application/services/parley/adapter.rs` — two test-only `FieldSpec` struct literals gain `cache: CacheMarker::Allow`

## Decisions Made

See `key-decisions` in the frontmatter. In brief: the backend check is a separate `WarGraph` method the engine calls (not a flag on `EngineRegistries`) and recurses into children; `Deny` is enforced statically for Paladin `output_field` and dynamically at store time for Function deltas; `CacheKeyFieldUndeclared` was added so a typo cannot silently narrow a key; `Fields` adds to a Paladin's rendered input and narrows a Function's snapshot; only `Edges`-routed successes are stored; a hit bypasses interceptors (the cache is Aegis, which wraps outside the chain); blake3 is a new dependency edge, not a new package.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `FieldSpec` struct literals outside the plan's file list**
- **Found during:** Task 1
- **Issue:** `src/application/services/parley/adapter.rs` builds `FieldSpec { .. }` by struct literal at two test sites; adding the `cache` field breaks their compilation.
- **Fix:** Added `cache: CacheMarker::Allow` to both literals (and the import). Disjoint from plan 25-12's root `tests/` + root `Cargo.toml` file set.
- **Files modified:** `src/application/services/parley/adapter.rs`
- **Commit:** `08fd78df`

**2. [Rule 2 - Missing critical functionality] `CacheKeySpec::Fields` naming an undeclared field**
- **Found during:** Task 1/2, while composing the snapshot hash
- **Issue:** An undeclared field name would contribute an "absent" marker to every key, so a typo silently narrows the key and serves stale hits.
- **Fix:** New `EngineError::CacheKeyFieldUndeclared` raised by `validate_aegis_cache_fields`, pinned by `a_cache_key_spec_naming_an_undeclared_field_fails_validation`.
- **Files modified:** `graph.rs`, `mod.rs`
- **Commit:** `08fd78df`

**3. [Rule 2 - Missing critical functionality] Store-time guards the plan's text left implicit**
- **Found during:** Task 2
- **Issue:** (a) a Function node's write set is not statically knowable, so `CachePolicyOnDeniedField` alone cannot honour `Deny` for Function nodes; (b) a `CachedDelta` holds a delta only, so caching a `Goto`/`End`/`Parley`/`Muster` directive would replay the delta and drop the routing; (c) a served entry carrying a foreign `schema_version` could mis-merge.
- **Fix:** `store_node_cache` refuses a delta touching any non-`Allow` field and is only called for an `Edges`-routed success; `lookup_node_cache` treats a schema-version mismatch as a miss. Pinned by `a_function_delta_touching_a_deny_field_is_never_stored` and `a_routing_directive_is_never_stored`.
- **Files modified:** `superstep.rs`
- **Commit:** `08fd78df` / `18d71e63`

**4. [Rule 3 - Blocking] A cryptographic digest with no hashing dependency in `paladin-battalion`**
- **Found during:** Task 2
- **Issue:** The key needs a collision-resistant digest (T-25-62); `paladin-battalion` had no hashing crate, and `std`'s `DefaultHasher` (64-bit, fixed keys) is craftable.
- **Fix:** `blake3 = "1.8.2"` added to `crates/paladin-battalion/Cargo.toml` — the same pinned version `paladin-core` already resolves, so `Cargo.lock` gains one line in the `paladin-battalion` dependency list and no new package (the plan's T-25-SC "installs no package" holds). Root `Cargo.toml` untouched.
- **Files modified:** `crates/paladin-battalion/Cargo.toml`, `Cargo.lock`
- **Commit:** `08fd78df`

---

**Total deviations:** 4 auto-fixed (1 compile-blocking edit outside the file list, 2 correctness additions, 1 dependency edge). No architectural changes.

**Sequencing note (as in 25-04/25-11):** each task's tests were written and run before/with its implementation, but Task 1's commit carries the whole engine wiring (validation cannot be exercised without `with_node_cache`, which cannot exist without the threaded parameter) plus `cache_key.rs` and `RecordingNodeCache`; Tasks 2 and 3 are test-pin commits over that implementation. No separate RED commits.

**Plan acceptance greps that cannot hold literally:**
- `! grep -q 'paladin-storage' crates/paladin-battalion/Cargo.toml` was already unsatisfiable before this plan: `paladin-storage` has been a `[dev-dependencies]` entry since Phase 22 Plan 01 (for `InMemoryWaypointStore`). The intent — this plan adds no such dev-dependency and `RecordingNodeCache` is in-crate — holds.
- `git diff HEAD~1 -- battlefield.rs | grep -c BATTLEFIELD_SCHEMA_VERSION` counts 3 added lines that *mention* the constant (a rustdoc link and the two tests asserting it is unchanged); the constant's own definition line (`"1.0.0"`) is untouched.

## Issues Encountered

- None beyond the deviations above. Disk stayed above 145 GB free throughout; no workspace-wide `cargo test --all-targets` was run (per the orchestrator's constraint) — `cargo check --workspace --all-targets --all-features` plus per-crate `--lib`/`--doc` runs cover the same compile surface.

## Threat Flags

None — no new network endpoint, auth path, file access or schema at a trust boundary. The T-25-61..67 mitigations in the plan's register are each pinned by a named test above; T-25-66 (cached delta contents) stays accepted as documented.

## Known Stubs

None.

## For the orchestrator's attention

- **`Cargo.lock` changed** (one dependency-list line under `paladin-battalion`). Plan 25-12 does not touch it, so no conflict is expected; flagging because the orchestrator asked for root-file discipline.
- **Not a persisted-shape change:** `BATTLEFIELD_SCHEMA_VERSION` and every Waypoint shape are unchanged; `FieldSpec` serialises one extra `"cache":"Allow"` key that pre-marker readers ignore only if they tolerate unknown fields (serde's default). `NodeExecutionRecord.cache_hit` (already `#[serde(default)]` since 25-07) is now set `true` on a hit.
- **Deferred (surface for `deferred-items.md`):** `CacheKeySpec::Custom(name)` stays deferred per D-28. A hit inside a `NodeSpec::Battalion` child works through the inherited backend, but a Battalion node itself still cannot carry `cache` (D-12). `invalidate(prefix)` is exposed via `cache_key::graph_prefix`/`node_prefix` but no engine method calls it — an operator-facing eviction command is a later plan's surface.

---
*Phase: 25-node-level-fault-tolerance*
*Completed: 2026-09-06*
