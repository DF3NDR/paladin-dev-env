# Phase 22 — Deferred Items

Out-of-scope discoveries made while executing Phase 22 plans. None of these are fixed here —
scope boundary: only auto-fix issues directly caused by the current task's changes.

## 1. `paladin-memory`'s `qdrant` feature fails to build under `--all-features` (found in Plan 22-04, Task 3)

**Found during:** validating the new `semver` CI job (`cargo semver-checks check-release
--package paladin-ai --baseline-version 0.9.0`, run locally against this plan's HEAD).

**Symptom:** `cargo-semver-checks`'s default "enable every feature except unstable/nightly"
heuristic (used when no `--default-features`/`--only-explicit-features`/`--features` flag is
passed) builds `paladin-ai` with every feature on at once, including `qdrant`. That build fails
rustdoc generation with:

```
error[E0063]: missing field `memory` in initializer of `VectorParams`
   --> crates/paladin-memory/src/sanctum/qdrant_adapter.rs:117:57
```

**Root cause:** `Cargo.toml`'s `[workspace.dependencies] qdrant-client = { version = "1.14" }` is
a caret range; `Cargo.lock` currently resolves it to `1.18.0`, and a `qdrant-client` release in
that range added a required `memory` field to `VectorParams` that `qdrant_adapter.rs`'s struct
literal does not set.

**Confirmed pre-existing and unrelated to Phase 22:** `cargo check -p paladin-memory --features
qdrant` (a plain compile, not a rustdoc build) succeeds today — the break is specific to the
combined all-features rustdoc build path `cargo-semver-checks` exercises by default, which no
existing CI job runs (`crate-isolation`'s `paladin-memory` leg uses `--all-features` too but only
`cargo build`/`cargo test`, not `cargo doc`; the `lint` job's `cargo doc --workspace --no-deps`
does not enable `qdrant` since that's a facade-level optional feature not in `paladin-ai`'s
default set). Phase 22 does not touch `crates/paladin-memory/src/sanctum/` at all.

**Workaround applied in this plan:** the new `semver` CI job passes `--default-features` to
`cargo semver-checks check-release` for every package, which sidesteps this break (verified
locally: all 11 publishable crates pass cleanly against the published `0.9.0` baseline under
`--default-features`). This is also arguably the more correct scope for the gate — it matches
the build a crates.io consumer gets by default.

**Recommended fix (deferred, not this phase):** either pin `qdrant-client` to an exact patch
version that predates the `VectorParams.memory` field, or update `qdrant_adapter.rs`'s struct
literal to set the new field (likely `..Default::default()` if `VectorParams` implements
`Default`, or an explicit `memory: None`/equivalent). Owner: whichever future phase next touches
the Sanctum/Qdrant adapter, or a dependency-maintenance pass.

**Impact if left unfixed:** none to Phase 22 itself. A future `cargo doc --all-features` or an
all-features rustdoc-based tool (e.g., docs.rs's own build, which uses default features unless
`[package.metadata.docs.rs] all-features = true` is set — not configured here) could hit the same
break; worth flagging to whoever owns the next `paladin-memory`/Qdrant-adjacent work.

## 2. Phase 22 Plan 16 — ENG-FR-02a acceptance 2a fixture audit (gap G-22-3)

**Scope.** Acceptance 2a of ENG-FR-02a (BUG-02, closed by Plan 22-15) additionally requires every
fixture that arranges its graph to work around the stranded-node defect to be revisited, with the
crash-resume fixture's looping-node arrangement named explicitly. This section is that audit,
covering every fixture found by (a) the four `files_modified` in this plan's frontmatter, (b) the
"Fixtures Handed to Plan 22-16" list in `22-15-SUMMARY.md`, and (c) a fresh tree-wide search for
self-loop constructions and for entry declarations that look chosen rather than natural (searched
via `EdgeSpec { from: X, to: X, ... }` pattern matching across `crates/` and `tests/`, plus a grep
for "stranded"/"workaround"/"unreachable"/"isolated" in the engine test modules). Nine call sites
across four files construct a self-loop; each is classified below into exactly one of three
buckets.

### Per-fixture classification table

| File | Fixture | Bucket | Evidence |
|------|---------|--------|----------|
| `crates/paladin-battalion/src/engine/graph.rs` | `validate_accepts_self_loop` | **Readiness dodge** | Single-node graph; the self-loop is `a`'s only possible incoming edge, and `Frontier::is_ready` (`engine::superstep`) leaves it `Pending` until `a` executes once — a non-entry `a` could never take a first turn, independent of BUG-02 (entry nodes are always eligible either way). Comment added. |
| `crates/paladin-battalion/src/engine/graph.rs` | `self_loop_on_entry_node_still_validates_and_runs` | **Readiness dodge** | Same single-node bootstrap shape as above, and this one actually runs to completion (`run_count() == 2`), proving the entry-bootstrap works. Comment added. |
| `crates/paladin-battalion/src/engine/graph.rs` | `validate_accepts_self_loop_on_node_reachable_from_entry_by_normal_edge` | **Unrelated** | Constructs the harder shape (self-loop **plus** a separate upstream edge into `b`) but calls `validate()` only — never runs the graph, so `Frontier::is_ready` is never consulted and the readiness defect (see Finding, below) never fires. Nothing to repair; comment added pointing at the new reproduction test. |
| `crates/paladin-battalion/src/engine/graph.rs` | `validate_rejects_self_loop_only_stranded_node_naming_it` | **Unrelated** | This fixture (`stranded`, non-entry, self-loop-only) IS the BUG-02 defect demonstration Plan 22-15 wrote — it proves the rejection works, it is not a workaround dodging anything. |
| `crates/paladin-battalion/src/engine/graph.rs` | `validate_rejects_multiple_stranded_nodes_in_one_error_registration_order` | **Unrelated** | Same as above, three non-entry self-loop-only nodes, all correctly rejected — a rejection-path demonstration, not a dodge. |
| `crates/paladin-battalion/src/engine/graph.rs` | `validate_prefers_limit_error_over_unreachable_node` | **Unrelated** | A non-entry self-loop-only `stranded` node is deliberately left unreachable to prove a *different* validation clause (limits) is checked first — the node's strandedness is the fixture's point, not something it dodges. |
| `crates/paladin-battalion/src/engine/graph.rs` | `validate_prefers_unknown_node_error_over_unreachable_node` | **Unrelated** | Same pattern as above, proving the unknown-node clause takes precedence. |
| `crates/paladin-battalion/src/engine/graph.rs` | `validate_prefers_custom_dispatch_error_over_unreachable_node` | **Unrelated** | Same pattern, proving the custom-dispatch clause takes precedence. |
| `crates/paladin-battalion/src/engine/graph.rs` | `validate_accepts_and_runs_stranded_node_once_made_reachable_from_entry` | **Unrelated** | No self-loop at all; the fixture's own existing comment (Plan 22-15) already correctly explains it deliberately avoids combining a self-loop with an external edge on the same node, to isolate "made reachable" cleanly. Already accurate; nothing to change. |
| `crates/paladin-battalion/src/engine/graph.rs` | `validate_accepts_two_node_cycle` | **Unrelated** | Ordinary 2-node cycle (`a -> b -> a`), no self-loop; `a` is entry because a cycle needs to start somewhere, the same requirement any acyclic entry graph has — nothing to do with either defect class. |
| `crates/paladin-battalion/src/engine/mod.rs` | `resume_restores_visit_counts_and_trips_limit_on_next_post_resume_visit` | **Readiness dodge** | **Newly discovered** — not in 22-15's handoff list. Same single-node self-loop-as-entry bootstrap pattern as the two `graph.rs` cases above. Comment added. |
| `crates/paladin-battalion/src/engine/mod.rs` | `resume_allow_graph_change_proceeds_when_vanguard_node_present` (node `"c"`) | **Strandedness dodge — already repaired in Plan 22-15** | Node `c` is declared its own entry point solely to satisfy the new eligible-set check (no self-loop involved). This is the one fixture Plan 22-15's Task 3 already caught and fixed; confirmed here still correctly commented, no further action needed. |
| `crates/paladin-battalion/src/engine/superstep.rs` | `self_loop_graph` helper (used by `self_loop_runs_exactly_three_times_when_approved_on_third_visit`, `self_loop_never_approved_trips_node_visit_limit_at_five`, `self_loop_at_four_visits_does_not_trip`) | **Readiness dodge** | Same single-node self-loop-as-entry bootstrap pattern, shared by three tests via one helper. Comment added to the helper (covers all three call sites). |
| `tests/integration/e2e_crash_resume_test.rs` | `build_graph`'s `loop_gate` (named by ENG-FR-02a acceptance 2a) | **Readiness dodge** | `loop_gate` has no separate upstream feed either (its only incoming edges are its own self-loop and the conditional edge to `researcher`, which is OUTGOING) — it is the same single-node bootstrap case as the others, just with five more downstream nodes after it. The fixture's own doc comment already named the readiness rule correctly (not misattributed to reachability/strandedness) but overgeneralized "every self-loop test in this workspace" to always use the entry arrangement; corrected to note the one exception (`validate_accepts_self_loop_on_node_reachable_from_entry_by_normal_edge`, validate-only) and to point at the new reproduction test. |
| `tests/integration/battalion/campaign_integration_test.rs` | `test_self_loop_detection` | **Unrelated — confirmed by reading, not assumed** | Exercises `CampaignExecutionService`/`Campaign`, the legacy cycle-**rejecting** graph service (`campaign_service.rs`), never `WarGraph` or the superstep engine. It asserts a self-loop **is rejected** — the opposite semantics of `WarGraph`, which explicitly permits self-loops (ENG-FR-02). Out of scope for this audit by design, not by oversight. |

**Acceptance 2a disposition: satisfied for strandedness.** No remaining fixture in the tree arranges
its graph to dodge BUG-02's stranded-node rejection — the three "unrelated" `graph.rs` rejection
fixtures demonstrate the rejection rather than avoid it, and the one already-repaired case
(`resume_allow_graph_change_proceeds_when_vanguard_node_present`) was fixed in Plan 22-15. The nine
readiness-dodge fixtures keep their entry-point arrangement (removing it would deadlock a
single-node self-loop graph permanently, per each fixture's own corrected comment) but no longer
misattribute or leave unexplained *why* that arrangement exists.

**The residual finding below is a different defect class — do not read it as an unmet acceptance
2a criterion.** It was surfaced by this audit but is not what acceptance 2a asked to close.
