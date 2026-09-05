# Parley & Chronicle: Pause, Resume, History and Graceful Shutdown

Human-in-the-loop approval gates, typed resume, an inspectable and forkable execution history, and
cooperative shutdown for the `WarEngine` — built as thin, well-specified layers over the `Waypoint`
substrate the [Control Flow guide](control-flow.md) introduces.

---

## Table of Contents

1. [Building an Approval Gate](#building-an-approval-gate)
2. [Raising a Parley from a Paladin Node](#raising-a-parley-from-a-paladin-node)
3. [Resuming a Suspended Thread](#resuming-a-suspended-thread)
4. [Partial Answers](#partial-answers)
5. [Expiry: `on_expire` Policies](#expiry-on_expire-policies)
6. [Chronicle: History, Replay and Fork](#chronicle-history-replay-and-fork)
7. [Graceful Shutdown from the Embedder's Side](#graceful-shutdown-from-the-embedders-side)
8. [The HTTP Surface](#the-http-surface)
9. [Notifying a Human a Parley Is Waiting](#notifying-a-human-a-parley-is-waiting)

---

## Building an Approval Gate

A **Parley** is a suspension point: a node stops the run, a Waypoint with `status: AwaitingInput`
is persisted, every task/timer/connection for the run is released, and the run returns
`RunOutcome::AwaitingInput`. The primary building block is `NodeSpec::Gate` — a node with no `run`
body of its own that always parleys on its first visit and writes the delivered value on the
post-resume visit.

An approval gate is exactly **one `Gate` node plus two conditional edges** — no custom node code:

```rust,ignore
use std::time::Duration;

use paladin_battalion::engine::graph::{GateRequestTemplate, NodeSpec, WarGraph};
use paladin_battalion::engine::{EdgeCondition, InputMapping};
use paladin_core::platform::container::battlefield::FieldName;
use paladin_core::platform::container::parley::{OnExpire, ParleyKind};

let approved = FieldName::new("approved").unwrap();

let request = GateRequestTemplate::new(
    ParleyKind::Approval,
    InputMapping::new("Deploy build {build_id} to production?"),
)
.with_payload_template(InputMapping::new(r#"{"build_id": "{build_id}"}"#))
.with_expires_in(Duration::from_secs(24 * 60 * 60))
.with_on_expire(OnExpire::FailRun);

graph.add_node("approve", NodeSpec::gate(request, Some(approved.clone())));
graph.add_edge("approve", "deploy", Some(EdgeCondition::Contains(r#""approved":true"#.into())));
graph.add_edge("approve", "cancel", Some(EdgeCondition::Contains(r#""approved":false"#.into())));
```

A few things to note:

- `output_field` (`approved` here) is **required** for `Approval`/`Choice`/`FreeText` gates and
  **must be `None`** for a `StateEdit` gate — `WarGraph::validate` rejects every other combination,
  and checks the field exists in the schema with a type compatible with the gate's `kind`
  (`Approval` → `Bool` or `String`; `Choice`/`FreeText` → `String`).
- `prompt_template`/`payload_template` render from the Battlefield through the same `InputMapping`
  templating every other node uses.
- `Approval` values are normalised before delivery: JSON `true`/`false` or the case-insensitive
  strings `"yes"`/`"no"`/`"approve"`/`"deny"` all resolve to a JSON boolean (or `"true"`/`"false"`
  if `output_field` is a `String` field).
- A `Gate`'s `output_field` is what edge evaluation reads for its source — exactly like a Paladin
  node's own `output_field` — so `Contains`/`Regex`/a registered `Custom` evaluator all work
  unchanged. **Anchor a `Contains` needle to the full `"field":value` pair** (as above), not a bare
  `true`/`false` — `Contains`/`Regex` match against the whole serialized Battlefield JSON, and every
  non-required schema field's own `"required":false` entry contains the bare word `false`.

## Raising a Parley from a Paladin Node

A Paladin node can raise a parley without a declarative `Gate`, through the structured directive
envelope's `next.parley` key:

```json
{
  "delta": {},
  "next": {
    "parley": {
      "kind": "Approval",
      "prompt": "Deploy build #482 to production?",
      "payload": { "build": 482 },
      "expires_in_secs": 86400
    }
  }
}
```

The parser stamps `parley_id`, `node_id` and `created_at`, and computes `expires_at` from
`expires_in_secs` — an author never supplies these. `kind`/`prompt` are required; `payload`,
`choices`, `expires_in_secs` and `on_expire` are optional (`on_expire` defaults to `FailRun`).

On the post-resume re-run, `InputMapping::render` resolves the answer through a `parley.`
namespace — resolved **only from `NodeContext`, never the Battlefield**, exactly like the
`muster.` namespace the Control Flow guide documents:

```rust,ignore
// {parley.value}       -- the submitted/defaulted value
// {parley.prompt}      -- the originating request's own prompt
// {parley.kind}        -- "Approval" | "Choice" | "FreeText" | "StateEdit"
// {parley.responded_by} -- the responder's identity, or empty for a defaulted response
```

`WarGraph::validate` rejects any schema field named with the `parley.` prefix, so a graph's own
state can never shadow this namespace. `NodeContext::parley_response()` gives the same data to
ordinary Rust code:

```rust,ignore
if let Some(response) = ctx.parley_response() {
    // response.value, response.kind, response.responded_by, response.defaulted
}
```

## Resuming a Suspended Thread

`WarEngine::resume_with` is the only path that advances a suspended thread:

```rust,ignore
use paladin_core::platform::container::parley::ParleyResponse;

let outcome = engine.resume_with(&graph, thread_id, responses).await?;
```

Every submitted `ParleyResponse` is validated **totally before anything is persisted** — an error
on any response leaves the thread suspended with no Waypoint written:

| `EngineError` variant | Meaning |
|---|---|
| `ThreadNotAwaitingInput` | A plain `resume`/`resume_with_options` against a suspended thread, or `resume_with` against a thread that is not `AwaitingInput` |
| `UnknownParleyId` | The submitted `parley_id` does not match any outstanding request on this thread |
| `ParleyAlreadyAnswered` | The `parley_id` already has an accepted response |
| `ResponseShapeInvalid { parley_id, reason }` | The value does not match its request's `kind` (e.g. an `Approval` gate submitted `"maybe"`) |
| `ParleyExpired { parley_id, expires_at }` | `expires_at` has passed and `on_expire: FailRun` applies |
| `GraphMismatch` | The graph passed to `resume_with` does not fingerprint-match the one the thread suspended under |

A plain `WarEngine::resume` (no responses) against an `AwaitingInput` thread never guesses — it
fails closed with `EngineError::ThreadAwaitingInput`, naming the still-outstanding parleys.

## Partial Answers

When several nodes parley in the same superstep, **all** of their requests are recorded on one
`AwaitingInput` Waypoint, and **all** must be answered before the run continues. Submitting a
correct answer for only some of them persists a *new* `AwaitingInput` Waypoint at the same
superstep, with `responses` extended and `RunOutcome::AwaitingInput` naming only the still-remaining
requests:

```rust,ignore
// Two parleys outstanding; answer only one.
let outcome = engine.resume_with(&graph, thread_id.clone(), vec![first_response]).await?;
// outcome is RunOutcome::AwaitingInput { parleys: [second_request], .. } -- still suspended.

let outcome = engine.resume_with(&graph, thread_id, vec![second_response]).await?;
// outcome is RunOutcome::Completed { .. } (or whatever the graph reaches next).
```

This partially-answered state is a property of the **persisted Waypoint**, not process memory: it
survives a full process restart and is queryable from a cold store handle. Responses are durably
consumed only when the first post-resume Waypoint actually persists — if the process dies between
validation and that write, the `AwaitingInput` Waypoint just read is still `latest`, and
re-submitting the identical responses is safe.

## Expiry: `on_expire` Policies

A `ParleyRequest` optionally carries `expires_at`, evaluated **lazily at resume time** — there is
no background timer. Each request's own `on_expire` policy decides what happens once the clock has
passed it, independent of whether the caller's own submission even names the expired request:

- **`OnExpire::FailRun`** (the default) — `resume_with` persists a `Failed` Waypoint (its reason
  naming the expired parley and node) and returns `Err(EngineError::ParleyExpired)`. The thread is
  thereafter resumable only via `replay`/`fork` from an earlier Waypoint — a plain `resume` against
  it fails closed with `EngineError::ThreadAlreadyFailed`.
- **`OnExpire::ResumeWithDefault(value)`** — `value` (validated against the request's own `kind` at
  graph-validate time for a `Gate`, or at raise time for a directive-raised parley) is substituted
  as the response, with `responded_by: None` and `defaulted: true` so an audit trail can see the
  substitution happened. The run then proceeds exactly as if that value had been submitted.

## Chronicle: History, Replay and Fork

`ChronicleService` is a thin, port-only read facade (no `paladin-battalion` dependency) over one
thread's Waypoint history:

```rust,ignore
use std::sync::Arc;
use paladin::application::services::chronicle::ChronicleService;

let chronicle = ChronicleService::new(waypoint_port);

// Newest-first summaries, including every branch's lineage.
let page = chronicle.history(&thread_id, 20, None).await?;

// The full snapshot (Battlefield, vanguard, records, status) for one Waypoint.
let waypoint = chronicle.inspect(&thread_id, waypoint_id).await?;

// The newest summary on the branch rooted at `branch_root`, or `None`.
let latest = chronicle.latest_on_branch(&thread_id, branch_root).await?;
```

`WarEngine::replay`/`WarEngine::fork` re-enter the superstep loop from any past Waypoint, each
producing a **new branch** while the original chain stays untouched:

```rust,ignore
// Re-run forward from an earlier Waypoint, unchanged.
let outcome = engine.replay(&graph, &thread_id, from_waypoint_id).await?;

// Re-run forward, but first merge an edit into the starting Battlefield --
// the "what-if" primitive. An edit naming an undeclared field fails closed
// and persists nothing, exactly like a real node's delta would.
let outcome = engine.fork(&graph, &thread_id, from_waypoint_id, edit).await?;
```

**Immutability is a hard, byte-for-byte invariant**: every mainline Waypoint serialises to
identical bytes before and after a `replay`/`fork`, and calling either twice from the same
Waypoint produces two independent branches, disturbing neither each other nor the mainline. A
branch is a queryable attribute — `Waypoint.fork_of`/`WaypointSummary.fork_of` mark the branch
root and every subsequent Waypoint on that branch inherits the same value — so the whole branch
tree reconstructs from `WaypointSummary` alone, with no full-Waypoint loads.

**Subgraph forks never share child Waypoints.** A branch runs its `NodeSpec::Battalion` children
under a thread id derived from *both* the parent thread and the branch root
(`ThreadId::child_on_branch`), so a fork's subgraph child always starts fresh and
`WaypointPort::latest` on that child thread never resolves the mainline child's own history.

## Graceful Shutdown from the Embedder's Side

`WarEngine::with_shutdown_grace(Duration)` (default 30 s) configures how long a mid-superstep
cancellation waits for the in-flight batch of node tasks before giving up on the stragglers:

```rust,ignore
use std::time::Duration;
use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::shutdown::ShutdownCoordinator;

let coordinator = ShutdownCoordinator::new();
let (token, _guard) = coordinator.register();

let engine = WarEngine::new(waypoint_port)
    .with_cancellation_token(token)
    .with_shutdown_grace(Duration::from_secs(30));
```

When the token fires while nodes are in flight, the engine keeps awaiting the WHOLE batch (never
just the one that triggered cancellation) until the grace deadline; nodes still running at the
deadline are aborted and recorded `NodeOutcomeKind::Skipped { reason: "shutdown" }`, their deltas
discarded. Those nodes' ids are **re-listed in the Halted Waypoint's vanguard** alongside the
normally computed next vanguard, so `resume` re-executes exactly them — exactly once — while every
node that finished inside the grace window merges normally. `shutdown_grace = Duration::ZERO`
aborts immediately.

An embedder that wants every in-flight run to drain on process shutdown constructs one
`ShutdownCoordinator`, registers every `WarEngine` run with it (`register()` returns a child token
plus an RAII `RunGuard`), and calls `coordinator.cancel_and_wait(grace)` from its own shutdown
path:

```rust,ignore
// On SIGTERM/SIGINT:
let outcome = coordinator.cancel_and_wait(Duration::from_secs(30)).await;
if outcome.drained() {
    // every registered run finished inside the grace window
} else {
    // the deadline elapsed first; any straggler is Skipped and re-listed for resume
}
```

`paladin-server` and `ServiceRunner` both wire this into SIGTERM/SIGINT already — see
[Kubernetes deployment](../deployment/kubernetes.md) for the operator-facing
`terminationGracePeriodSeconds`/env-var contract. `shutdown_grace_secs`/`graceful_shutdown` are
runtime settings only: they are never hashed into the graph fingerprint and never affect
`resume`'s `GraphMismatch` check.

## The HTTP Surface

`paladin-web` exposes three thread routes behind the same authentication middleware
`/v1/agents/*` already uses:

| Route | Behavior |
|---|---|
| `GET /v1/threads/{id}/state` | The thread's latest status, plus outstanding `parleys`/`responses` when suspended |
| `POST /v1/threads/{id}/resume` | Submits `{ "responses": [{ "parley_id", "value", "responded_by" }] }`; returns **`202 Accepted { thread_id, state_url }`** immediately |
| `GET /v1/threads/{id}/history` | Paginated Chronicle history: `?limit=20&cursor=...` (limit ≤ 100), `{ items, next_cursor }` |

`POST .../resume` never holds the connection open: it validates synchronously (typed
400/404/409 errors on rejection, nothing persisted) and, only for a valid **and complete**
submission, spawns the actual engine continuation as a background task registered with the
process's `ShutdownCoordinator`, returning `202` immediately. A client polls `GET .../state` for
the outcome. Two distinct `409` conflict codes share the same HTTP status:
`thread_not_awaiting_input` (the thread is not suspended) and `graph_not_registered` (the thread's
graph fingerprint has no `WarGraph` registered in this process). Every route answers `501
not_implemented`, naming the config key to set, when no waypoint backend (`APP_WAYPOINT_STORE_
BACKEND=sqlite|postgres`) is wired.

> **Never template a secret or credential into a Gate's payload.** `GET /v1/threads/{id}/state`
> returns that payload verbatim to any authenticated caller. A `payload_template` is
> author-controlled, rendered from the Battlefield exactly like a prompt — treat it with the same
> care you would a log line or an error message, never a place to carry an API key or a database
> credential.

> **Interim authorization posture.** As of this phase, the thread routes accept any
> *authenticated* caller regardless of role — there is no admin/writer scope distinction on who
> may answer an approval gate or inspect a thread's history yet. This is a documented, accepted
> interim posture, not an oversight: admin/writer scopes on these routes are Phase 27's `PLAT-06`.

## Notifying a Human a Parley Is Waiting

Notifying someone that a parley is waiting is **application code composed from the existing
`paladin-notifications` port** — this phase adds no new port for it. A typical composition, called
right after a `RunOutcome::AwaitingInput` comes back from `start`/`resume_with`:

```rust,ignore
use std::sync::Arc;

use paladin_core::platform::container::notification::{
    Notification, NotificationChannel, NotificationContent, NotificationPriority,
    NotificationRecipient,
};
use paladin_ports::output::notification_port::NotificationDeliveryPort;

async fn notify_parley_waiting(
    notifications: Arc<dyn NotificationDeliveryPort>,
    approver_email: &str,
    prompt: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let notification = Notification::new(
        NotificationRecipient::Email(approver_email.to_string()),
        NotificationContent::new(
            "Approval needed".to_string(),
            prompt.to_string(),
            "parley".to_string(),
        ),
        NotificationChannel::Email,
        NotificationPriority::High,
    )?;

    notifications.deliver_notification(notification).await?;
    Ok(())
}
```

Call this from the same application code that calls `WarEngine::start`/`resume_with` — a
`RunOutcome::AwaitingInput { parleys, .. }` names exactly which requests are now waiting, so the
prompt and recipient can be derived from them directly. No engine or port change is required to
add a second channel (Slack, SMS, a webhook): construct a different `NotificationDeliveryPort`
adapter and call it the same way.
