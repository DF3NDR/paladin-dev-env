// benches/engine_benchmarks.rs
//
// Engine Benchmarks — ENG-NFR-01 / ENG-NFR-02 (Phase 22 Plan 10)
//
// Two criterion groups:
//
// - `waypoint_save`: the marginal cost of `SqliteWaypointStore::save` for a
//   Battlefield at three sizes (1 KiB, 512 KiB, just under 1 MiB), measuring
//   ENG-NFR-01 ("< 10 ms p50 overhead per superstep on the SQLite backend for
//   a Battlefield <= 1 MiB"). Database construction, migration, and Waypoint
//   payload construction all happen in `iter_batched`'s setup closure (or
//   before the benchmark group is defined at all, for the DB/migration) — the
//   timed region is `SqliteWaypointStore::save` alone.
// - `superstep_cost`: wall-clock cost of one `WarEngine::start` superstep for
//   a fixed graph at two Vanguard widths (1 node, 8 nodes), against an
//   `InMemoryWaypointStore` (no disk I/O), so the per-node execution cost is
//   separable from the fixed per-superstep engine overhead this bench alone
//   measures — the persistence cost is `waypoint_save`'s job, not this one's.
//
// This project reports p50, not criterion's default mean/CI, by reading
// criterion's own per-iteration `sample.json` after a real (non `--test`) run
// — the same method used for `benches/config_benchmarks.rs`'s predecessor
// suites (see STATE.md, "Phase 3 Plan 04"). `cargo bench --bench
// engine_benchmarks -- --test` (criterion's smoke mode) is what CI runs; it
// is not a substitute for that real run when a p50 figure is needed.
//
// To run this benchmark:
// ```bash
// cargo bench --bench engine_benchmarks
// ```

use std::sync::Arc;

use async_trait::async_trait;
use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use tokio::runtime::Runtime;
use uuid::Uuid;

use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::graph::{EngineLimits, NodeSpec, WarGraph};
use paladin_battalion::engine::node::{NodeContext, NodeError, StateNode};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{
    FrontierSnapshot, GraphFingerprint, NodeId, ThreadId, Waypoint, WaypointStatus,
};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

/// 1 KiB — the "small" case, cheap enough that the per-row fixed cost (not
/// the payload) dominates.
const SMALL_PAYLOAD_BYTES: usize = 1024;
/// 512 KiB — the midpoint case, so the reported figure shows how cost scales
/// toward the stated 1 MiB ceiling rather than only at one point.
const MEDIUM_PAYLOAD_BYTES: usize = 512 * 1024;
/// Just under 1 MiB: `1 MiB - 8 KiB`, leaving headroom for the rest of the
/// Waypoint's JSON envelope (ids, timestamps, schema, status) so the whole
/// stored row — not just this one field — stays at or under the 1 MiB
/// ENG-NFR-01 states the ceiling in terms of.
const LARGE_PAYLOAD_BYTES: usize = 1024 * 1024 - 8 * 1024;

fn payload_schema() -> BattlefieldSchema {
    BattlefieldSchema::new(vec![FieldSpec::new(
        FieldName::new("payload").expect("non-empty field name"),
        DispatchRule::LastWrite,
        None,
        false,
    )])
}

/// Build a Waypoint whose Battlefield carries one `payload` field of
/// `byte_len` ASCII bytes. Built directly via `Waypoint::new_root` (not the
/// shared `contract_tests::sample_waypoint` fixture, which is deliberately
/// schema-empty) because this benchmark needs a schema carrying the sized
/// payload field itself.
fn fresh_waypoint_with_payload(payload: &str) -> Waypoint {
    let schema = payload_schema();
    let mut delta = StateDelta::new();
    delta
        .set(FieldName::new("payload").unwrap(), payload)
        .expect("payload string serializes");
    let battlefield =
        Battlefield::initialize(schema, &delta).expect("payload field is declared in schema");

    // A fresh ThreadId per Waypoint so every timed `save()` call is a real
    // INSERT (the engine's actual per-superstep pattern — a brand new
    // WaypointId every time), not a repeated UPSERT of the same row.
    let thread = ThreadId::new(format!("engine-bench-save-{}", Uuid::new_v4()))
        .expect("uuid-suffixed thread id is valid");

    Waypoint::new_root(
        thread,
        0,
        GraphFingerprint::from_canonical_bytes(b"engine-bench-fixture"),
        battlefield,
        vec![],
        vec![],
        WaypointStatus::Running,
        std::collections::BTreeMap::new(),
        FrontierSnapshot::default(),
    )
}

fn bench_waypoint_save(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime for async criterion benches");

    for (label, byte_len) in [
        ("1kib", SMALL_PAYLOAD_BYTES),
        ("512kib", MEDIUM_PAYLOAD_BYTES),
        ("just_under_1mib", LARGE_PAYLOAD_BYTES),
    ] {
        let db_path = std::env::temp_dir().join(format!(
            "paladin_engine_bench_waypoint_save_{label}_{}.sqlite",
            Uuid::new_v4()
        ));
        let url = format!("sqlite://{}", db_path.display());

        // --- Setup OUTSIDE the timed region: construct the database file
        // and apply the migration once per payload size, before the
        // benchmark's iterations start.
        let store = rt.block_on(async {
            SqliteWaypointStore::new(&url)
                .await
                .expect("sqlite waypoint store constructs and migrates")
        });

        // Precompute the payload string once per size group: cloning it
        // per-iteration inside `iter_batched`'s setup closure (below) is
        // itself outside the timed region.
        let payload = "x".repeat(byte_len);

        c.bench_function(&format!("engine/waypoint_save_sqlite_{label}"), |b| {
            b.to_async(&rt).iter_batched(
                // Setup (untimed): build a fresh Waypoint for this iteration.
                || fresh_waypoint_with_payload(&payload),
                // Routine (timed): the save call alone.
                |wp| {
                    let store = &store;
                    async move {
                        store.save(&wp).await.expect("waypoint save succeeds");
                    }
                },
                BatchSize::SmallInput,
            );
        });

        // Clean up: drop the pool before removing the file so no handle is
        // still open, then remove the temporary database (T-22-35) — no
        // artifact left in the repository tree after this benchmark runs.
        drop(store);
        let _ = std::fs::remove_file(&db_path);
    }
}

// ── ENG-NFR-02 superstep wall-clock cost ─────────────────────────────────

/// A `StateNode` that writes a fixed value into its own declared field —
/// nothing else. Used to build fixed-width, single-superstep graphs whose
/// wall-clock cost isolates the engine's own per-superstep overhead from any
/// per-node work (there is none here).
struct FixedValueNode {
    field: FieldName,
    value: serde_json::Value,
}

#[async_trait]
impl StateNode for FixedValueNode {
    async fn run(&self, _state: &Battlefield, _ctx: &NodeContext) -> Result<StateDelta, NodeError> {
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), self.value.clone());
        Ok(delta)
    }
}

/// `WarEngine::new` requires a `PaladinPort`; this benchmark's graphs are
/// `Function`-node only, so this is never actually invoked.
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unimplemented!("this benchmark's graphs run Function nodes only")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unimplemented!("this benchmark's graphs run Function nodes only")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Build a single-superstep, `width`-wide graph: `width` independent entry
/// nodes, each writing a distinct field, no edges (so the run completes
/// after exactly one superstep).
fn build_width_graph(width: usize) -> WarGraph {
    let fields: Vec<FieldSpec> = (0..width)
        .map(|i| {
            FieldSpec::new(
                FieldName::new(format!("f{i}")).unwrap(),
                DispatchRule::LastWrite,
                None,
                false,
            )
        })
        .collect();
    let schema = BattlefieldSchema::new(fields);
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    for i in 0..width {
        let id = NodeId::new(format!("n{i}"));
        let field = FieldName::new(format!("f{i}")).unwrap();
        graph.add_node(
            id.clone(),
            NodeSpec::Function(Arc::new(FixedValueNode {
                field,
                value: serde_json::json!(i),
            })),
        );
        graph.add_entry(id);
    }
    graph
}

fn bench_superstep_cost(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime for async criterion benches");

    for width in [1usize, 8usize] {
        let engine = WarEngine::new(
            Arc::new(UnusedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let graph = build_width_graph(width);

        c.bench_function(&format!("engine/superstep_cost_width_{width}"), |b| {
            b.to_async(&rt).iter_batched(
                || {
                    ThreadId::new(format!("engine-bench-superstep-{}", Uuid::new_v4()))
                        .expect("uuid-suffixed thread id is valid")
                },
                |thread| {
                    let engine = &engine;
                    let graph = &graph;
                    async move {
                        engine
                            .start(graph, thread, StateDelta::new())
                            .await
                            .expect("single-superstep graph completes");
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
}

criterion_group!(engine_benches, bench_waypoint_save, bench_superstep_cost);
criterion_main!(engine_benches);
