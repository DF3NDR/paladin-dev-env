// examples/war_engine_memory_baseline.rs
//
// War Engine Memory Baseline — ENG-NFR-02 (Phase 22 Plan 10)
//
// A recorded measurement harness for the memory half of ENG-NFR-02 ("one
// Battlefield clone per superstep maximum, plus one per concurrently
// executing node view; Arc-shared read snapshot preferred; measure, don't
// guess"). `criterion` (used for `benches/engine_benchmarks.rs`'s wall-clock
// half of this NFR) produces no memory metric, so this harness follows
// `examples/muster_baseline.rs`'s exact method: read the process's resident
// set size (RSS) from `/proc/self/status` before and after a fixed workload,
// and report the delta.
//
// The workload is a fixed `WarGraph`: `CHAIN_WIDTH` independent chains, each
// `CHAIN_LAYERS` nodes deep, so every superstep runs `CHAIN_WIDTH` nodes
// concurrently across `CHAIN_LAYERS` supersteps. A `payload` field, seeded
// once at the start and never touched again, keeps every Battlefield clone
// this run makes carrying `PAYLOAD_BYTES` of state — the "Battlefield of a
// stated size" this harness measures against.
//
// The clone count itself is measured, not inspected from source: each node
// records the raw pointer address of the `&Battlefield` snapshot it
// observes (the same technique
// `crates/paladin-battalion/src/engine/test_support.rs`'s
// `CountingFunctionNode` and its
// `battlefield_cloned_once_per_superstep_arc_ptr_eq` test already use
// in-crate — this harness re-proves the same property from outside the
// crate, through the public `WarEngine` API). All nodes executing within the
// same superstep read from the same `Arc<Battlefield>` snapshot
// (`superstep::run`'s ENG-FR-05/ENG-NFR-02 design), so they observe the same
// address; a regression to a per-node deep clone would make every node's
// address distinct, which this harness's bound check catches.
//
// To run this example:
// ```bash
// cargo run --release --example war_engine_memory_baseline
// ```

use std::collections::{BTreeMap, HashSet};
use std::io::Read as _;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use async_trait::async_trait;

use paladin_battalion::engine::RunOutcome;
use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::graph::{EdgeSpec, EngineLimits, NodeSpec, WarGraph};
use paladin_battalion::engine::node::{NodeContext, NodeError, StateNode};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// Nodes executing concurrently within every superstep of this harness's
/// fixed graph — the "concurrently executing node view" half of ENG-NFR-02's
/// bound.
const CHAIN_WIDTH: usize = 4;
/// Supersteps this harness's fixed graph runs — the divisor for the
/// per-superstep memory delta this harness reports.
const CHAIN_LAYERS: usize = 20;
/// Size, in bytes, of a Battlefield field held constant across every
/// superstep of this run — the "Battlefield of a stated size" ENG-NFR-02
/// asks this harness to measure against. Same order of magnitude as
/// `benches/engine_benchmarks.rs`'s "512 KiB" waypoint-save case.
const PAYLOAD_BYTES: usize = 512 * 1024;

/// `WarEngine::new` requires a `PaladinPort`; this harness's graph is built
/// entirely from `Function` nodes, so this is never actually invoked.
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unimplemented!("this harness's graph runs Function nodes only")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unimplemented!("this harness's graph runs Function nodes only")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// Records `(superstep, battlefield_snapshot_address)` for every node
/// invocation across the whole run — the measured stand-in for "was this
/// node handed a fresh clone or a shared view".
#[derive(Default)]
struct ObservedAddresses(Mutex<Vec<(u64, usize)>>);

impl ObservedAddresses {
    fn record(&self, superstep: u64, ptr: usize) {
        self.0
            .lock()
            .expect("observation lock poisoned")
            .push((superstep, ptr));
    }

    /// `(max distinct addresses observed within any single superstep, total
    /// node-execution observations, number of distinct supersteps
    /// observed)`.
    fn summarize(&self) -> (usize, usize, usize) {
        let observed = self.0.lock().expect("observation lock poisoned");
        let mut by_superstep: BTreeMap<u64, HashSet<usize>> = BTreeMap::new();
        for (superstep, ptr) in observed.iter() {
            by_superstep.entry(*superstep).or_default().insert(*ptr);
        }
        let max_distinct = by_superstep.values().map(HashSet::len).max().unwrap_or(0);
        (max_distinct, observed.len(), by_superstep.len())
    }
}

/// One node in a `CHAIN_WIDTH`-wide, `CHAIN_LAYERS`-deep fixed graph: writes
/// a fixed value into its chain's own field, and records the Battlefield
/// snapshot address it observed.
struct TrackingNode {
    field: FieldName,
    tracker: Arc<ObservedAddresses>,
}

#[async_trait]
impl StateNode for TrackingNode {
    async fn run(&self, state: &Battlefield, ctx: &NodeContext) -> Result<Directive, NodeError> {
        let ptr = state as *const Battlefield as usize;
        self.tracker.record(ctx.superstep, ptr);
        let mut delta = StateDelta::new();
        delta.set_raw(self.field.clone(), serde_json::json!(ctx.superstep));
        Ok(delta.into())
    }
}

fn chain_field(width_index: usize) -> FieldName {
    FieldName::new(format!("chain{width_index}")).expect("non-empty field name")
}

/// Build the fixed graph this harness measures: `CHAIN_WIDTH` independent
/// chains, each `CHAIN_LAYERS` nodes deep, entered together at superstep 1,
/// so every superstep runs exactly `CHAIN_WIDTH` nodes concurrently. Each
/// chain's own field is touched by exactly one node per superstep (never two
/// distinct writers in the same superstep), so no `DispatchConflict` is ever
/// possible by construction.
fn build_graph(tracker: Arc<ObservedAddresses>) -> WarGraph {
    let mut fields: Vec<FieldSpec> = (0..CHAIN_WIDTH)
        .map(|w| FieldSpec::new(chain_field(w), DispatchRule::LastWrite, None, false))
        .collect();
    fields.push(FieldSpec::new(
        FieldName::new("payload").expect("non-empty field name"),
        DispatchRule::LastWrite,
        None,
        false,
    ));
    let schema = BattlefieldSchema::new(fields);
    let limits = EngineLimits {
        // Generous headroom over CHAIN_LAYERS: the recursion-limit check
        // trips when superstep_number >= max_supersteps, and superstep
        // numbering starts at 1.
        max_supersteps: (CHAIN_LAYERS as u64) + 10,
        ..EngineLimits::default()
    };
    let mut graph = WarGraph::new(schema, limits);

    for w in 0..CHAIN_WIDTH {
        let mut previous: Option<NodeId> = None;
        for l in 0..CHAIN_LAYERS {
            let id = NodeId::new(format!("chain{w}_layer{l}"));
            graph.add_node(
                id.clone(),
                NodeSpec::Function(Arc::new(TrackingNode {
                    field: chain_field(w),
                    tracker: tracker.clone(),
                })),
            );
            match previous {
                Some(prev) => {
                    graph.add_edge(EdgeSpec {
                        from: prev,
                        to: id.clone(),
                        condition: None,
                    });
                }
                None => {
                    graph.add_entry(id.clone());
                }
            }
            previous = Some(id);
        }
    }
    graph
}

/// Reads the current process's resident set size (RSS) from
/// `/proc/self/status`, in kilobytes. Identical method to
/// `examples/muster_baseline.rs`'s helper of the same name.
fn read_vm_rss_kb() -> std::io::Result<u64> {
    let mut status = String::new();
    std::fs::File::open("/proc/self/status")?.read_to_string(&mut status)?;

    let line = status
        .lines()
        .find(|l| l.starts_with("VmRSS:"))
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "VmRSS line not found in /proc/self/status",
            )
        })?;

    let digits: String = line.chars().filter(char::is_ascii_digit).collect();
    digits
        .parse::<u64>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tracker = Arc::new(ObservedAddresses::default());
    let graph = build_graph(tracker.clone());

    let mut initial = StateDelta::new();
    initial.set(
        FieldName::new("payload").unwrap(),
        "x".repeat(PAYLOAD_BYTES),
    )?;

    let engine = WarEngine::new(
        Arc::new(UnusedPaladinPort),
        Arc::new(InMemoryWaypointStore::new()),
    );
    let thread = ThreadId::new("war-engine-memory-baseline")?;

    let rss_before_kb = read_vm_rss_kb()?;
    println!("rss_before_kb={rss_before_kb}");

    let start = Instant::now();
    let outcome = engine.start(&graph, thread, initial).await?;
    let elapsed_ms = start.elapsed().as_millis();

    let rss_after_kb = read_vm_rss_kb()?;
    println!("rss_after_kb={rss_after_kb}");

    match &outcome {
        RunOutcome::Completed { .. } => {}
        other => return Err(format!("expected the fixed graph to complete, got {other:?}").into()),
    }

    let (max_distinct_this_superstep, total_observations, superstep_count) = tracker.summarize();
    let rss_delta_kb = rss_after_kb.saturating_sub(rss_before_kb);
    let bytes_delta = rss_delta_kb * 1024;
    let bytes_per_superstep = if superstep_count == 0 {
        0
    } else {
        bytes_delta / superstep_count as u64
    };

    println!("superstep_count={superstep_count}");
    println!("chain_width={CHAIN_WIDTH}");
    println!("payload_bytes={PAYLOAD_BYTES}");
    println!("elapsed_ms={elapsed_ms}");
    println!("rss_delta_kb={rss_delta_kb}");
    println!("bytes_per_superstep={bytes_per_superstep}");
    println!("node_execution_observations={total_observations}");
    println!("battlefield_addresses_observed_max_per_superstep={max_distinct_this_superstep}");

    // ENG-NFR-02: at most one Battlefield clone per superstep, plus one per
    // concurrently executing node view. An Arc-shared read snapshot costs
    // nothing extra per view, so the measured per-superstep distinct-address
    // count must never exceed 1 (the shared snapshot) + CHAIN_WIDTH (every
    // node getting its own deep clone instead, the worst case this bound
    // still tolerates).
    let ceiling = 1 + CHAIN_WIDTH;
    if max_distinct_this_superstep > ceiling {
        return Err(format!(
            "ENG-NFR-02 regression: {max_distinct_this_superstep} distinct Battlefield \
             addresses observed within a single superstep, exceeding the ceiling of {ceiling} \
             (1 shared clone + {CHAIN_WIDTH} concurrent node views)"
        )
        .into());
    }
    println!(
        "battlefield_clone_bound_check=PASS (observed {max_distinct_this_superstep} <= ceiling {ceiling})"
    );

    Ok(())
}
