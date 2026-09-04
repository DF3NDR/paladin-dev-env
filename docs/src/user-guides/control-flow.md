# Control Flow: Dynamic Routing, Fan-Out & Subgraphs

Node-authored routing, worker fan-out, subgraph composition and LLM-evaluated edges for the
`WarEngine`, the superstep-based engine built on `WarGraph`, `Battlefield` typed state and
`Waypoint` checkpointing.

---

## Table of Contents

1. [WarGraph in Three Sentences](#wargraph-in-three-sentences)
2. [Directives: Node-Authored Routing](#directives-node-authored-routing)
3. [DirectiveParser: Reading a Paladin's Output](#directiveparser-reading-a-paladins-output)
4. [Muster: Dynamic Fan-Out](#muster-dynamic-fan-out)
5. [Subgraphs: Composing Graphs](#subgraphs-composing-graphs)
6. [LLM-Evaluated Edges](#llm-evaluated-edges)
7. [Migrating from v0.9: M-B-01](#migrating-from-v09-m-b-01)

---

## WarGraph in Three Sentences

A `WarGraph` is a collection of nodes and static edges, executed by a `WarEngine` in
**supersteps**: each superstep runs every currently-ready node concurrently, merges their
`Battlefield` state deltas, and computes the next superstep's ready set. Unlike the legacy
`CampaignExecutionService`'s DAG, a `WarGraph` **permits cycles** — `WarGraph::validate` rejects
only a node that could never become ready (unreachable from `entry` and not marked a dynamic
target or worker template), not a cycle itself, and every run is bounded by `EngineLimits`
(`max_supersteps`, `max_node_visits`). This page assumes that much and no more; the full engine
guide is future documentation (see the `Deferred` note in `23-CONTEXT.md`).

---

## Directives: Node-Authored Routing

A `StateNode::run` returns a `Directive` — the `StateDelta` it contributes, plus a `NextStep`
telling the engine how to route control next:

```rust,ignore
pub struct Directive {
    pub delta: StateDelta,
    pub next: NextStep,
}

pub enum NextStep {
    Edges,               // the default — evaluate this node's static outgoing edges
    Goto(Vec<NodeId>),    // enter these nodes directly next superstep
    Muster(Vec<MusterTask>), // fan out worker tasks (see below)
    End,                  // complete the run after this superstep's merge
    Parley(ParleyRequest), // suspend — not implemented this phase, fails the run
}
```

**`NextStep::Edges`** is the default and the only variant a pre-CF-02 node ever produces
(`impl From<StateDelta> for Directive`) — a graph that never opts in behaves identically to
before this feature existed.

**`NextStep::Goto`** enters the named node(s) directly in the next superstep, bypassing the
normal readiness check. Every target must be a declared node; a target reachable ONLY via `Goto`
(no static incoming edge) must additionally be marked with `WarGraph::mark_dynamic_target` —
an undeclared target fails the run with `EngineError::GotoUnknownNode`. A `Goto` target is still
subject to `EngineLimits::max_node_visits` like any other entry, so a refine loop (writer →
reviewer → `Goto(writer)` until satisfied) is legal and bounded, not an unconditional escape from
the engine's own iteration guarantee. A node authoring its own routing and the graph's static
edges never both fire in the same superstep — every non-`Edges` variant routes the emitting
node's static edges `NotFiring`.

**`NextStep::End`** completes the run after the current superstep's merge. Peers in the same
superstep still merge their own deltas normally; `End` takes precedence over a `Goto` emitted by
another node in the same superstep.

**`NextStep::Parley`** is declared for Doc 03's suspension mechanism but not implemented this
phase: a node returning it fails the run with `EngineError::ParleyNotSupported` rather than
silently pausing — Phase 24 (HITL-01) lands the real behavior.

---

## DirectiveParser: Reading a Paladin's Output

A `NodeSpec::Paladin` node's raw string output is turned into a `Directive` by its
`DirectiveParser`:

```rust,ignore
pub enum DirectiveParser {
    PlainOutput,                              // the default
    StructuredDirective { on_parse_error: OnParseError },
}
```

**`DirectiveParser::PlainOutput`** — the default — writes the raw output to `output_field` and
routes via `NextStep::Edges`, byte-identical to a pre-CF-02 Paladin node.

**`DirectiveParser::StructuredDirective`** parses a JSON envelope out of the output and applies
ONLY the envelope's `delta` — no implicit `output_field` write. See the `DirectiveParser`
rustdoc (`crates/paladin-battalion/src/engine/directive_parser.rs`) for the authoritative
envelope shape rather than duplicating it here, so the two cannot drift. `on_parse_error`
resolves a failed extraction: `OnParseError::FailRun` (the default) fails the run with
`EngineError::DirectiveParseFailed`; `OnParseError::FallbackPlain` degrades to `PlainOutput`
semantics.

```rust,ignore
use paladin_battalion::engine::directive_parser::{DirectiveParser, OnParseError};

let parser = DirectiveParser::StructuredDirective {
    on_parse_error: OnParseError::FailRun,
};
// output: r#"{"delta": {"verdict": "approved"}, "next": "edges"}"#
```

---

## Muster: Dynamic Fan-Out

A node returns `NextStep::Muster(Vec<MusterTask>)` to fan out N worker tasks, each dispatching to
a **worker template** node — one registered via `WarGraph::add_worker_template` rather than
`add_node`, so it can run only as a Muster task and never as a normal graph entry:

```rust,ignore
graph.add_worker_template(
    NodeId::new("summarize_chunk"),
    NodeSpec::Function(Arc::new(SummarizeChunk)),
);
```

Each `MusterTask` carries an isolated `payload` (never merged into the Battlefield — visible
only to that worker) and a caller-chosen `task_key`, used to order worker results
deterministically on aggregation and to reject a duplicate key within one Muster. A worker
node reads its task's payload through `NodeContext::muster` — never through a Battlefield
field — via the `{muster.payload}` / `{muster.task_key}` placeholders in an `InputMapping`
template; graph validation rejects any schema field declared with the `muster.` prefix, so this
namespace can never be shadowed. `EngineLimits::max_muster_tasks` (default 100) bounds a single
Muster directive, enforced at directive-receipt time before any task dispatches
(`EngineError::MusterTaskLimitExceeded`) — raising it is a legitimate operator action, so it is
excluded from the graph fingerprint. A run resumed mid-Muster picks up the outstanding tasks from
the last progress Waypoint, which stores the superstep's *unmerged* delta snapshot rather than a
partially merged Battlefield.

---

## Subgraphs: Composing Graphs

`NodeSpec::Battalion` embeds a child `WarGraph` as a single node, running to completion within
ONE parent superstep regardless of how many supersteps the child itself takes:

```rust,ignore
use paladin_battalion::engine::graph::{NodeSpec, StateMap};

let state_map = StateMap::new()
    .with_input(FieldName::new("topic")?, FieldName::new("child_topic")?)
    .with_output(FieldName::new("child_summary")?, FieldName::new("summary")?);

graph.add_node(
    NodeId::new("sub_workflow"),
    NodeSpec::battalion(Arc::new(child_graph), state_map),
);
```

`StateMap` is the **complete** contract for what crosses the parent/child boundary in either
direction: `inputs` are `(parent field, child field)` pairs seeding the child's initial state
from the parent's superstep snapshot; `outputs` are `(child field, parent field)` pairs returned
as the Battalion node's own delta, merged under the parent's dispatch rules like any other
node's delta. A child field not named in `outputs` never leaves the child — the child's own
schema, nodes and edges stay entirely private, never visible in the parent's Battlefield, this
node's delta, or the parent thread's Waypoint payload (even `Debug` output on `NodeSpec::Battalion`
prints only the child's fingerprint and the two map sizes). A child run gets its own namespaced
Waypoint thread (`ThreadId::child`, carrying `checkpoint_ns`) so its checkpoints never collide
with the parent's or a sibling subgraph's history. `restart_on_resume` (default `false`)
controls whether a resumed parent run restarts this node's child from scratch rather than
continuing a partially-completed child thread.

---

## LLM-Evaluated Edges

Register an `LlmDecisionEvaluator` under `EdgeCondition::Custom("<decision name>")`, exactly
like any other evaluator through `EdgeEvaluatorRegistry`:

```rust,ignore
use std::sync::Arc;
use paladin_battalion::llm_decision::{LlmDecisionEvaluator, OnAmbiguous};
use paladin_core::platform::container::waypoint::NodeId;

let evaluator = LlmDecisionEvaluator::new(
    "route_urgency",
    llm.clone(),
    "gpt-4",
    "Is this urgent? Reply escalate or archive.\n\n{output}",
    vec![
        ("escalate".to_string(), NodeId::new("urgent_handler")),
        ("archive".to_string(), NodeId::new("archive_handler")),
    ],
)
.on_ambiguous(OnAmbiguous::Default("archive".to_string()));

engine.with_edge_evaluator("route_urgency", Arc::new(evaluator));
```

The model is asked **once per decision per superstep** — every outgoing edge sharing the same
source node and rendered prompt consults one memoized answer, so N outgoing edges never become N
independent (and possibly inconsistent) calls. A model answer matching no declared choice is
resolved by `on_ambiguous`: `OnAmbiguous::Fail` (the default) fails the run; `OnAmbiguous::Default`
treats it as a named fallback choice.

Commander's `StrategySelection::Semantic` applies the same idea to Battalion pattern selection:
prompt a model with the strategy catalog and the run's input, parse the answer as a strategy
name, and fall back to `StrategySelection::Heuristic` — deterministically, with the fallback and
its cause recorded in `BattalionResult::strategy_selection_reasoning` — on any LLM error or an
answer naming no catalog strategy.

```rust,ignore
use paladin_battalion::commander::{CommanderBuilder, StrategySelection};

let commander = CommanderBuilder::new(paladin_port)
    .strategy(BattalionStrategy::Auto)
    .paladins(paladins)
    .strategy_selection(StrategySelection::Semantic {
        llm: llm.clone(),
        model: "gpt-4".to_string(),
    })
    .build()?;
```

Both `LlmDecision` and `StrategySelection::Semantic` are **off by default and reached only in
code** — no `APP_*` environment variable, cargo feature, or config-struct field can turn either
on; a v0.9 configuration boots identically.

**Egress boundary:** an `LlmDecisionEvaluator`'s `prompt_template` renders against live
`Battlefield` state (or a legacy Paladin's raw output) and the rendered result is sent, verbatim,
to a third-party model. Whatever the template's placeholders resolve to is exactly what leaves
this process — if a workflow's schema carries secret-like data and the template references that
field, it is sent to the model. This is the workflow author's control point, not something the
evaluator can filter; neither its error paths nor its memoized state ever interpolate the
rendered prompt or the model's raw response body.

---

## Migrating from v0.9: M-B-01

Before this phase, an unregistered `EdgeCondition::Custom(name)` silently evaluated to `true` on
every run — a bug, not a feature (BUG-01). It is now a fail-closed validation error: an
unregistered `Custom` name fails graph/campaign validation before any node executes, naming
every offender. If you have a v0.9 workflow using `EdgeCondition::Custom`, see `MIGRATION.md`
§9.1, entry M-B-01, for the worked before/after example and the exact validation error text.
