# WarEngine: Battlefield State & Superstep Execution

**Since:** v0.10.0 (Phase 22)
**Crates:** `paladin-battalion` (`WarEngine`, `WarGraph`), `paladin-core` (`Battlefield`,
`Waypoint`), `paladin-storage` (the `WaypointPort` backends)

> Every code example targets the current **v0.10.0** workspace. The substantive examples are
> real, compiled code pulled from the `paladin-doc-examples` crate via mdBook `{{#include}}`, so
> they are checked against the live API; a few illustrative fragments are marked `rust,ignore`.
> The API forms are verified against `crates/paladin-battalion/` and `crates/paladin-core/`.

The **WarEngine** is the superstep engine every Paladin Battalion pattern in this book ultimately
runs on: it executes a `WarGraph` of nodes over a typed `Battlefield`, in bounded **supersteps**
that checkpoint automatically after each one and resume with zero re-execution after a crash.
Unlike the legacy Campaign graph, a `WarGraph` permits cycles — including self-loops — so
iterative workflows (retry-and-refine, evaluate-optimize loops) are expressible directly.

## Building a Graph

A `WarGraph` is constructed from a `BattlefieldSchema` and a set of `EngineLimits`, then populated
with nodes and edges. The graph below is deliberately small and cyclic: one node self-loops over a
`(count, status)` Battlefield a few times before falling out of the loop — the same shape that
makes cyclic execution useful.

```rust,ignore
{{#include ../../../crates/doc-examples/src/superstep_engine.rs:build_graph}}
```
