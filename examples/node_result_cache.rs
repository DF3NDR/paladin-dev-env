// examples/node_result_cache.rs
//
// Node-Result Caching (EX-81, EX-82) -- Doc 04 FT-FR-18..20, D-27/D-28/D-29
//
// Demonstrates:
//   1. constructing the Redis-backed node-result cache adapter the
//      `redis-cache` feature enables, and wiring it onto a `WarEngine` via
//      `with_node_cache` (EX-81),
//   2. running a small graph twice under different threads so the second
//      run's node is served from the cache instead of re-executing, and
//   3. toggling the `APP_NODE_CACHE_ENABLED` configuration variable off and
//      re-running to show the cache bypassed entirely -- when the toggle is
//      off, this program builds a graph with no cache policy attached and
//      an engine with no cache backend wired, exactly what a config-driven
//      deployment would do (attaching a `CachePolicy` to a node whose
//      engine has no cache backend is a typed validation error, never a
//      silent no-op) (EX-82).
//
// This program needs a running Redis server -- `make services-up` brings
// up the project's dev stack (Redis + MinIO). It is therefore
// build-verified only: never executed by any automated check here or in CI
// (D-16).
//
// Build it with:
//   cargo build --example node_result_cache --features "redis-cache"
//
// Run it (once Redis is reachable) with:
//   cargo run --example node_result_cache --features "redis-cache"
//
// No LLM provider key is read or needed -- the demo graph uses a pure
// Function node, never a Paladin.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use async_trait::async_trait;

use paladin::config::env_utils::EnvOverridable;
use paladin::config::node_cache::NodeCacheConfig;
use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::cache_key::node_prefix;
use paladin_battalion::engine::graph::{EngineLimits, NodeSpec, WarGraph};
use paladin_battalion::engine::node::{NodeContext, StateNode, StateNodeError};
use paladin_core::platform::container::aegis::{Aegis, CacheKeySpec, CachePolicy};
use paladin_core::platform::container::battlefield::{
    Battlefield, BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
};
use paladin_core::platform::container::directive::{Directive, NextStep};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_storage::node_cache::{RedisNodeCache, RedisNodeCacheConfig};
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// `WarEngine::new` requires a `PaladinPort`; every graph below runs a
/// single `Function` node, so this is never actually invoked.
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!("this program's graph runs a Function node only")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this program's graph runs a Function node only")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A `StateNode` that increments a shared counter every time it actually
/// executes and writes a fixed value to `field` -- the smallest node that
/// makes a cache hit (no increment) observably different from a cache miss
/// (an increment) from outside the engine.
struct CountingNode {
    field: FieldName,
    counter: Arc<AtomicU64>,
}

#[async_trait]
impl StateNode for CountingNode {
    async fn run(
        &self,
        _state: &Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        self.counter.fetch_add(1, Ordering::SeqCst);
        let mut delta = StateDelta::new();
        delta
            .set(self.field.clone(), "computed-once")
            .map_err(|e| StateNodeError(e.to_string()))?;
        Ok(Directive {
            delta,
            next: NextStep::Edges,
        })
    }
}

/// Build the demo graph's single `computer` node. `with_cache_policy`
/// mirrors what a config-driven deployment would do: attach a
/// `CachePolicy` only when the node-cache toggle is on -- attaching one
/// unconditionally, then relying on `with_node_cache` alone to gate
/// caching, would make a disabled-toggle run fail validation instead of
/// bypassing the cache (`EngineError::CachePolicyWithoutCacheBackend`).
fn build_graph(
    counter: Arc<AtomicU64>,
    with_cache_policy: bool,
) -> Result<(WarGraph, NodeId), Box<dyn std::error::Error>> {
    let result_field = FieldName::new("result")?;
    let schema = BattlefieldSchema::new(vec![FieldSpec::new(
        result_field.clone(),
        DispatchRule::LastWrite,
        None,
        false,
    )]);
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    let node_id = NodeId::new("computer");
    graph.add_node(
        node_id.clone(),
        NodeSpec::Function(Arc::new(CountingNode {
            field: result_field,
            counter,
        })),
    );
    graph.add_entry(node_id.clone());
    if with_cache_policy {
        graph.set_aegis(
            node_id.clone(),
            Aegis {
                cache: Some(CachePolicy {
                    ttl: Duration::from_secs(15 * 60),
                    key: CacheKeySpec::Default,
                }),
                ..Aegis::default()
            },
        );
    }
    Ok((graph, node_id))
}

/// Read back whether the node in `thread`'s latest Waypoint was served
/// from the cache.
async fn latest_cache_hit(
    store: &InMemoryWaypointStore,
    thread: &ThreadId,
) -> Result<bool, Box<dyn std::error::Error>> {
    let waypoint = store
        .latest(thread)
        .await?
        .ok_or("expected a waypoint after a completed run")?;
    Ok(waypoint
        .completed
        .first()
        .map(|record| record.cache_hit)
        .unwrap_or(false))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== node_result_cache: EX-81, EX-82 ===\n");

    // Part 1 (EX-81): construct the Redis-backed node cache adapter the
    // `redis-cache` feature enables.
    println!("--- Part 1: constructing the Redis-backed node cache (EX-81) ---");
    let redis_config = RedisNodeCacheConfig::default();
    println!(
        "Connecting to Redis at {}:{} (bring it up with `make services-up` if this hangs \
         or errors)...",
        redis_config.redis_host, redis_config.redis_port
    );
    let cache = Arc::new(RedisNodeCache::new(redis_config).await?);
    println!("Connected. RedisNodeCache constructed.\n");

    let store = Arc::new(InMemoryWaypointStore::new());
    let counter = Arc::new(AtomicU64::new(0));

    // ---- Toggle the node cache ON, via APP_NODE_CACHE_ENABLED. ----
    // Safety: this process is single-threaded at this point in `main`, and
    // no other code reads process environment concurrently here.
    unsafe {
        std::env::set_var("APP_NODE_CACHE_ENABLED", "true");
    }
    let mut node_cache_config = NodeCacheConfig::default();
    node_cache_config.apply_env_overrides();
    println!(
        "APP_NODE_CACHE_ENABLED={} -> node cache wired",
        node_cache_config.enabled
    );

    let (graph_cached, node_id) = build_graph(counter.clone(), node_cache_config.enabled)?;
    let key_prefix = node_prefix(&graph_cached.fingerprint(), &node_id);
    println!(
        "Cache key prefix for node '{}': {key_prefix} (the engine appends a content digest \
         after this prefix to form the full key)\n",
        node_id.as_str()
    );

    let engine_cached = {
        let mut engine = WarEngine::new(Arc::new(UnusedPaladinPort), store.clone());
        if node_cache_config.enabled {
            engine = engine.with_node_cache(cache.clone());
        }
        engine
    };

    let thread1 = ThreadId::new("cache-run-1")?;
    engine_cached
        .start(&graph_cached, thread1.clone(), StateDelta::new())
        .await?;
    let hit1 = latest_cache_hit(&store, &thread1).await?;
    println!(
        "Run 1 (thread '{}'): node executed, cache_hit={hit1}, total executions so far={}",
        thread1.as_str(),
        counter.load(Ordering::SeqCst)
    );

    let thread2 = ThreadId::new("cache-run-2")?;
    engine_cached
        .start(&graph_cached, thread2.clone(), StateDelta::new())
        .await?;
    let hit2 = latest_cache_hit(&store, &thread2).await?;
    println!(
        "Run 2 (thread '{}'): served from the cache, cache_hit={hit2}, total executions so \
         far={} (still 1 -- the node did not re-execute)\n",
        thread2.as_str(),
        counter.load(Ordering::SeqCst)
    );

    // Part 2 (EX-82): toggle the enable variable off and re-run.
    println!("--- Part 2: toggling APP_NODE_CACHE_ENABLED off (EX-82) ---");
    unsafe {
        std::env::set_var("APP_NODE_CACHE_ENABLED", "false");
    }
    node_cache_config.apply_env_overrides();
    println!(
        "APP_NODE_CACHE_ENABLED={} -> node cache bypassed",
        node_cache_config.enabled
    );

    let (graph_uncached, _node_id2) = build_graph(counter.clone(), node_cache_config.enabled)?;
    let engine_uncached = WarEngine::new(Arc::new(UnusedPaladinPort), store.clone());

    let thread3 = ThreadId::new("cache-run-3")?;
    engine_uncached
        .start(&graph_uncached, thread3.clone(), StateDelta::new())
        .await?;
    let hit3 = latest_cache_hit(&store, &thread3).await?;
    println!(
        "Run 3 (thread '{}'): node re-executed, cache_hit={hit3}, total executions so far={} \
         (the cache was bypassed entirely -- no CachePolicy attached, no backend wired)\n",
        thread3.as_str(),
        counter.load(Ordering::SeqCst)
    );

    // Restore: the variable was unset before this program ran.
    unsafe {
        std::env::remove_var("APP_NODE_CACHE_ENABLED");
    }
    println!("APP_NODE_CACHE_ENABLED restored to unset.");

    println!("\n=== node_result_cache complete ===");
    Ok(())
}
