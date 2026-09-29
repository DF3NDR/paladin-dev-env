// benches/engine_benchmarks.rs
//
// Engine Benchmarks — ENG-NFR-01 / ENG-NFR-02 (Phase 22 Plan 10), sink-variant
// overhead (28-06, D-37, PRD 07 acceptance 6)
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
//   28-06 extends this group with three SINK-VARIANT cases (`none`,
//   `log_sink`, `composite`) at a fixed representative width, reusing
//   `build_width_graph` unchanged, measuring the cost of default-on tracing
//   against the untraced path (PRD 07 acceptance 6's <=3% bar, D-37). Kept
//   OUT of any CI gate: criterion numbers on a shared runner are noise, and
//   the recorded evidence file (not this bench's own pass/fail) is the gate.
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
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{
    FrontierSnapshot, GraphFingerprint, NodeId, ThreadId, Waypoint, WaypointStatus,
};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::trace_sink_port::{
    CompositeSink, TraceRecord, TraceSink, TraceSinkError,
};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_storage::waypoint::sqlite::SqliteWaypointStore;

use paladin::infrastructure::telemetry::LogTraceSink;

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
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), self.value.clone());
        Ok(delta.into())
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

// ── 28-06: sink-variant overhead (D-37, PRD 07 acceptance 6) ────────────

/// A `TraceSink` that does nothing -- the second child of the `composite`
/// variant's `CompositeSink`, standing in for a second real consumer (e.g.
/// the D-24 SSE bus sink) without this bench depending on that subsystem.
struct NoopTraceSink;

#[async_trait]
impl TraceSink for NoopTraceSink {
    async fn on_event(&self, _record: TraceRecord) -> Result<(), TraceSinkError> {
        Ok(())
    }
}

/// A logger that formats every `Info`-or-finer record and throws the bytes
/// away.
///
/// The `paladin::trace` target is only "enabled" in the operator's sense
/// once a logger that accepts it is installed (`RUST_LOG` naming
/// `paladin::trace`). This stand-in pays the formatting cost a real logger
/// pays (`record.args()` is rendered into `std::io::sink()`) while keeping
/// the bench free of any I/O, so the enabled rows measure the trace path's
/// own cost rather than a terminal's or a file's.
struct DiscardLogger;

impl log::Log for DiscardLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::Level::Info
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            use std::io::Write;
            let _ = write!(std::io::sink(), "{}", record.args());
        }
    }

    fn flush(&self) {}
}

static LOGGER: DiscardLogger = DiscardLogger;

/// Installs [`DiscardLogger`] (once per process; a second call is a no-op)
/// and sets the process-wide max level so the `paladin::trace` target is
/// either enabled (`Info`) or disabled (`Off`).
fn set_trace_target(enabled: bool) {
    // `set_logger` fails once a logger is installed; that is the expected
    // path on every call after the first, so the error is deliberately
    // ignored.
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(if enabled {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Off
    });
}

/// The fixed width every sink-variant case runs at: wide enough (8 nodes,
/// the same "many nodes" case `bench_superstep_cost` already measures
/// above) that a per-record dispatcher overhead has more than one node's
/// worth of trace records to show up against, without this bench needing
/// its own new fixture builder (`build_width_graph` is reused unchanged).
const SINK_VARIANT_WIDTH: usize = 8;

/// Benchmarks one `WarEngine::start` superstep of `build_width_graph(
/// SINK_VARIANT_WIDTH)` under three sink configurations -- `none` (no
/// `TraceSink` attached, the engine's own untraced path, D-10), `log_sink`
/// (a single `LogTraceSink`, the facade's default-on consumer), and
/// `composite` (`LogTraceSink` + `NoopTraceSink` fanned through a
/// `CompositeSink`, mirroring `build_run_sink`'s own two-sink case) -- so
/// the measured overhead of default-on tracing against the untraced
/// baseline can be evaluated against PRD 07 acceptance 6's <=3% bar
/// (D-37). See `.planning/phases/28-observability-tooling/28-BENCH-EVIDENCE.md`
/// for the recorded numbers and verdict.
///
/// Phase 45 (OBS-05, D-18) measures each variant under two conditions on the
/// same fixture. The rows named `engine/bench_superstep_cost_sinks_{label}`
/// (the three Phase 28 IDs, unchanged) run with the `paladin::trace` target
/// ENABLED -- a discarding `Info`-level logger is installed, which is the
/// operator-visible default when `RUST_LOG` includes `paladin::trace`. The
/// rows named `..._target_off` run with the target disabled: no output and,
/// once the enablement guard lands, no serialisation either. Before that
/// guard they reproduce Phase 28's condition (no logger installed,
/// serialisation still paid, output discarded by the `log!` macro).
fn bench_superstep_cost_sink_variants(c: &mut Criterion) {
    let rt = Runtime::new().expect("tokio runtime for async criterion benches");
    let graph = build_width_graph(SINK_VARIANT_WIDTH);

    let variants: Vec<(&str, Option<Arc<dyn TraceSink>>)> = vec![
        ("none", None),
        (
            "log_sink",
            Some(Arc::new(LogTraceSink::new()) as Arc<dyn TraceSink>),
        ),
        (
            "composite",
            Some(Arc::new(CompositeSink::new(vec![
                Arc::new(LogTraceSink::new()),
                Arc::new(NoopTraceSink),
            ])) as Arc<dyn TraceSink>),
        ),
    ];

    // Enabled condition first (the Phase 28 IDs), disabled condition second
    // (`_target_off`). `set_trace_target` runs immediately before each
    // `bench_function` because criterion executes them sequentially.
    for (trace_target_enabled, suffix) in [(true, ""), (false, "_target_off")] {
        for (label, sink) in &variants {
            let mut engine = WarEngine::new(
                Arc::new(UnusedPaladinPort),
                Arc::new(InMemoryWaypointStore::new()),
            );
            if let Some(sink) = sink {
                engine = engine.with_trace_sink(Arc::clone(sink));
            }

            set_trace_target(trace_target_enabled);

            // The `bench_superstep_cost` substring is deliberately part of
            // the benchmark ID (not just this Rust function's name):
            // criterion's own CLI filter argument matches against the ID
            // string, and this phase's own `<verify>` command
            // (`cargo bench -- bench_superstep_cost --test`) needs this
            // filter to actually select these variants.
            c.bench_function(
                &format!("engine/bench_superstep_cost_sinks_{label}{suffix}"),
                |b| {
                    b.to_async(&rt).iter_batched(
                        || {
                            ThreadId::new(format!(
                                "engine-bench-sinks-{label}{suffix}-{}",
                                Uuid::new_v4()
                            ))
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
                },
            );
        }
    }

    // Leave the process the way the other groups expect it: no trace output.
    set_trace_target(false);
}

criterion_group!(
    engine_benches,
    bench_waypoint_save,
    bench_superstep_cost,
    bench_superstep_cost_sink_variants
);
criterion_main!(engine_benches);
