# Phase 27: Platform API - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-09-07
**Phase:** 27-Platform API
**Mode:** `--auto` — every question was answered by taking the recommended option, with no
interactive prompt. Each row below records what the alternatives were and why the recommendation
won, so a human can audit or reverse any of them.
**Areas discussed:** Run identity & status machine, Queue port & lease mechanics, Worker pool
placement, Cross-instance cancellation, ThreadBusy enforcement, Parley/resume integration, SSE
streaming & degraded mode, Assistants storage & immutability, WarGraphDoc home & schema, Schedules
& tick durability, Webhooks & SSRF, HTTP surface/scopes/pagination, Config/tests/bookkeeping

`[--auto] Selected all gray areas: Run identity & status machine, Queue port & lease mechanics,
Worker pool placement, Cross-instance cancellation, ThreadBusy enforcement, Parley/resume
integration, SSE streaming & degraded mode, Assistants storage & immutability, WarGraphDoc home &
schema, Schedules & tick durability, Webhooks & SSRF, HTTP surface/scopes/pagination,
Config/tests/bookkeeping.`

---

## Run identity, the status machine & persistence

| Option | Description | Selected |
|--------|-------------|----------|
| Core types + CAS transitions + no history table | `Run`/`RunId`/`RunStatus` in `paladin-core`; `update_status(from, to)` compiles to a conditional UPDATE; Waypoints remain the execution history | ✓ |
| Core types + read-modify-write in a transaction | Simpler adapter code; correct on Postgres, racy on SQLite/WAL across processes | |
| Types in `paladin-ports` + append-only `run_status_history` | Full status audit trail; duplicates the Chronicle and violates ADR-0016's core-owns-value-types rule | |

**Selection:** option 1 (recommended default) → D-01…D-05.
**Notes:** CAS was chosen mainly because it is the *same* primitive the busy-thread index (D-17) and
the schedule tick claim (D-37) need — one concurrency idiom across the phase instead of three. The
history table is recorded as promotable later without a breaking change.

---

## Queue port & lease mechanics

| Option | Description | Selected |
|--------|-------------|----------|
| New `RunQueuePort`; Redis sorted-set lease via Lua | PRD §2.2 signature verbatim; atomic claim/expire in one `EVAL`; `script` feature already enabled | ✓ |
| Extend the existing `QueuePort` with lease methods | Fewer traits; forbidden by X-10.4 — a required method on a pre-existing public trait breaks every implementor | |
| New port, Redis Streams + consumer groups | Native redelivery; awkward `extend_lease` / delayed-requeue, and the pending-entries list becomes a second source of truth | |

**Selection:** option 1 (recommended default) → D-06…D-10.
**Notes:** the queue message deliberately carries only a pointer (`run_id`), never the input — a
message with an embedded payload can disagree with the repository after a resume or a cancel.
Heartbeat set at `lease/4` rather than the PRD's `≤ lease/3` ceiling, derived from one config knob.

---

## Worker pool placement & shutdown

| Option | Description | Selected |
|--------|-------------|----------|
| Pool in the facade; web writes via an input port, reads the repository port directly | Matches Phase 24 D-24/D-25 exactly; ADR-0031 satisfied | ✓ |
| Pool in `paladin-web` | Would need `paladin-web → paladin-battalion`, forbidden in the default build by ADR-0031 | |
| Pool in a new crate | A fourth home for orchestration nobody asked for; no FR justifies the split | |

**Selection:** option 1 (recommended default) → D-11…D-13.
**Notes:** the read/write asymmetry (reads straight through `RunRepositoryPort`, writes through
`RunSubmissionPort`) is not a new convention — it is precisely how Phase 24 split `GET …/state`
from `POST …/resume`.

---

## Cross-instance cancellation

| Option | Description | Selected |
|--------|-------------|----------|
| New optional `CancellationProbe` trait object + builder method; infallible; adapter debounces | No signature change, no required trait method; probe errors can never fail a run | ✓ |
| Add a required method to an existing engine trait | Breaks implementors — X-10.4 | |
| Poll the repository inside the engine directly | Puts a storage dependency inside `paladin-battalion`'s hot loop; violates X-01 | |

**Selection:** option 1 (recommended default) → D-14…D-16.
**Notes:** the probe's fallibility question was decided by copying `TraceSink`'s published
"errors are diagnostics only" contract. Debounce policy sits in the adapter (default 1 s) so the
engine seam stays trivial. Cancel writes the durable flag first, signals the local token second.

---

## `409 ThreadBusy` enforcement

| Option | Description | Selected |
|--------|-------------|----------|
| Partial unique index on `runs(thread_id)` where status is active | Holds under 10 concurrent submits and across instances by construction | ✓ |
| Application check-then-insert | Cannot pass PRD acceptance 3; racy even single-instance | |
| In-process mutex per thread id | Single-replica only; the phase's whole point is multi-replica | |

**Selection:** option 1 (recommended default) → D-17…D-19.
**Notes:** **flagged for reviewer** — D-18 includes `AwaitingInput` in the busy set, where PRD 06
§2.1 names only `Queued|Running`. It only ever *adds* 409s, so no stated acceptance test can break,
and the alternative (admitting a second run onto a suspended thread) would corrupt the Waypoint
chain. The 409 body names `resume` as the remedy.

---

## Parley integration & resume

| Option | Description | Selected |
|--------|-------------|----------|
| Keep Phase 24's 202 contract verbatim; swap spawn → enqueue; add `run_id` to the response | The deferral 24-CONTEXT recorded by name; pays the X-10.3 registration cost openly | ✓ |
| Keep the contract and omit `run_id` | Client needs a second round trip for a value the server already holds | |
| New `POST /runs/{id}/resume` alongside the thread route | Two ways to resume one thread; the published route was chosen in Phase 24 for a reason | |

**Selection:** option 1 (recommended default) → D-20…D-23.
**Notes:** `AwaitingInput` acks rather than nacks — a nack would spin, since redelivery would
re-resume a thread with no new responses. One `attempt` counter serves both redelivery and resume;
the preceding status disambiguates the cause.

---

## Run streaming & degraded mode

| Option | Description | Selected |
|--------|-------------|----------|
| `TraceSink` → per-run broadcast bus → SSE; seven wire names frozen, mapping in one function; polling fallback documented | Phase 28 keeps ownership of the authoritative enum and has a single edit point | ✓ |
| Define the authoritative `TraceEvent` enum now | Explicitly OBS-01's deliverable (Phase 28); PRD 06 lists PRD 07 as a soft dependency for this reason | |
| Live path only, error when the run is remote | Fails PLAT-FR-07's "terminal events always eventually delivered" | |

**Selection:** option 1 (recommended default) → D-24…D-27.
**Notes:** the degraded path is documented as having no ordering guarantee relative to the live
path — PLAT-FR-07 asks for a *documented* degraded mode, and naming the limitation is what satisfies
it. `paladin-web` consumes a stream port rather than owning the bus (ADR-0031).

---

## Assistants — storage, immutability, validation

| Option | Description | Selected |
|--------|-------------|----------|
| Tagged envelope over opaque JSON in core; facade compiles and validates | Keeps ports/storage/web free of any battalion dependency; one validation seam | ✓ |
| Typed `Workflow(WarGraphDoc)` arm in core | Drags engine-shaped types into a crate with no engine; violates X-01 | |
| Asymmetric `Agent(PaladinConfigDoc)` + `Workflow(Value)` | Typechecks (core does own `PaladinConfig`) but yields two validation seams and two persistence shapes for one concept | |

**Selection:** option 1 (recommended default) → D-28…D-32.
**Notes:** immutability is enforced structurally — no `update_version` method exists on the adapter
and no `PUT` route is registered, so the code to violate "no PUT, ever" is absent rather than merely
unused. `latest` is resolved inside the run-insert transaction, making freeze-at-submit a database
property. Code-registered agents are exposed by default (`expose_code_registry: true`), since
PLAT-FR-11's "one discovery surface" is only true if it is on.

---

## `WarGraphDoc` home & JSON Schema

| Option | Description | Selected |
|--------|-------------|----------|
| Doc in `paladin-battalion` with the PRD's inherent `compile()`; hand-authored schema + example-corpus drift guard (`jsonschema` dev-dep) | Literal PRD API; no runtime dependency added | ✓ |
| Derive the schema with `schemars` | A runtime derive dependency on a public type for a document published once; against X-11.4 | |
| Doc in `paladin-core` | Core would hold node kinds, edge conditions and aegis policies it has no engine for | |

**Selection:** option 1 (recommended default) → D-33…D-35.
**Notes:** fingerprint stability is asserted across a real process boundary, not a same-process round
trip — "restart-stable" means the canonical encoding contains no `HashMap` iteration order, pointer
or timestamp, and only two processes can catch that. Reuses Phase 24's two-process test shape.

---

## Schedules & tick durability

| Option | Description | Selected |
|--------|-------------|----------|
| Own the schedule loop: persisted `next_tick` claimed by conditional UPDATE; `croner` parsing; existing adapter untouched | Restart safety and multi-replica safety fall out of one mechanism; no leader election | ✓ |
| Persist alongside `tokio-cron-scheduler`, re-registering jobs at boot | The no-duplicate/no-missed-double guarantee would live in two places | |
| Extend `TokioCronSchedulerAdapter` with persistence | Its own module docs record no status query and no persistence; PLAT-FR-13 forbids exactly that | |

**Selection:** option 1 (recommended default) → D-36…D-39.
**Notes:** the existing 5↔6 field cron normalization is extracted and reused rather than rewritten;
the existing adapter's stricter 6-field contract is left as published (X-03). `croner` is already in
`Cargo.lock` transitively, so it is MSRV-proven here before promotion. Skipped `FixedThread` ticks
are counted on the row so `GET /schedules/{id}` can answer "why did nothing run".

---

## Webhooks, HMAC & the SSRF guard

| Option | Description | Selected |
|--------|-------------|----------|
| Persisted delivery queue drained by a service; sign the exact bytes sent; SSRF guard at write+send; redirects disabled | The attempts table is required anyway, so the retry schedule is a column, not a sleeping task | ✓ |
| Spawn a delivery task per run completion | Loses deliveries on restart; still needs the attempts table | |
| Follow redirects on the webhook client | Would forward the signature credential to an attacker-chosen host and bypass the write-time SSRF check | |

**Selection:** option 1 (recommended default) → D-40…D-43.
**Notes:** **flagged for reviewer** — D-42 disables redirect-following, a tightening beyond the PRD's
literal text, justified by `security.instructions.md`'s standing rule for credential-carrying
clients. DNS-rebinding resolve-then-connect pinning is documented as a known limitation rather than
implemented, which PRD 06 PLAT-FR-15 explicitly permits. Retry backoff is tested under
`tokio::time::pause`, so no test sleeps for a minute.

---

## HTTP surface, scopes, pagination & generated clients

| Option | Description | Selected |
|--------|-------------|----------|
| One new `RunApiState`/`run_router`; `ThreadApiState` gains fields + `#[non_exhaustive]`; scopes map onto existing two roles by operation shape; Phase 24's cursor pagination verbatim; one pinned `openapi-generator-cli` for both languages | Mirrors the Phase 24 precedent; no new role variant; one pagination shape across six endpoints | ✓ |
| Add a `Writer` variant to `UserRole` | An X-10.2 break on a core public enum for no FR | |
| A parallel fork/delete router to avoid touching `ThreadApiState` | Two routers over one resource is worse than one registered struct change | |
| Path-filter the `sdk-clients` CI job | A required check that does not run on a PR blocks its merge on GitHub | |

**Selection:** option 1 (recommended default) → D-44…D-49.
**Notes:** the scope mapping follows `agent_controller`'s actual two-tier convention — invocation
gated per-target (`authorize_invoke`), registry mutation gated by `require_admin`. So run
submit/cancel/resume/fork are open to any authenticated principal, while assistant and schedule
CRUD require admin. Finer-grained scopes are deferred and §9.6 will say so.

---

## Config, test tiers & program bookkeeping

| Option | Description | Selected |
|--------|-------------|----------|
| Six new config structs, all defaulting OFF; Tier 1/Tier 2 split carried forward; MIGRATION §9 rows filled as work lands | Makes SHIP-02's v0.9-config boot test pass by construction rather than by patch | ✓ |
| Default the new subsystems on for convenience | Breaks the "legacy behavior, all new subsystems disabled" acceptance test in Phase 29 | |
| Sweep MIGRATION §9 registrations into a final plan | X-10.1 diffs the public API and treats every unregistered change as a finding | |

**Selection:** option 1 (recommended default) → D-50…D-54.
**Notes:** Docker is unavailable in this devcontainer, so the Redis and Postgres suites — including
PRD acceptance 2's worker-death redelivery test — are Tier 2 and must never be marked passed
locally; an InMemory twin covers the logic in Tier 1. Recorded again for planners: doc tests do not
count toward `cargo llvm-cov` and are skipped by `--tests`, so the 82% floor cannot lean on them.

---

## Claude's Discretion

Auto mode resolved every gray area, so nothing was handed back to Claude by a user. The following
were nonetheless left deliberately open in CONTEXT.md rather than decided prematurely:

- Exact module and file names inside the crates that were fixed, and how the SQL migrations split.
- Whether the run row's `webhook` spec is an inline JSON column or a joined table.
- Plan decomposition and count, beyond PRD 06 §5's TDD ordering.
- The precise `RunStreamEvent` payload fields, within the seven frozen wire event names.
- Whether `ipnet` is needed at all, or `std::net::IpAddr` classification suffices.
- Whether `run_status_history` is promoted to a real table if research surfaces an FR reading it.

## Deferred Ideas

Recorded in full in CONTEXT.md's `<deferred>` section. Summary: the authoritative `TraceEvent` enum
and OTel/graph-export work (Phase 28); `MIGRATION.md` §9 finalisation, the v0.9-config boot test and
the version bump (Phase 29); finer-grained scopes and multi-tenant RBAC; a multi-replica-safe auth
token store (ADR-0041's known scope limit, newly visible once workers run multi-replica);
resolve-then-connect DNS-rebinding pinning; an append-only `run_status_history` table; `schemars`-
generated schemas; autoscaling orchestration; hand-polished SDKs; resuming a `Failed` thread; parley
propagation through nested Battalions; and the standing 22-/24-/25-REVIEW follow-ups.

### Reviewed Todos (not folded)

- "Verify local make coverage reproduces CI's 82.39% figure" (score 0.2, matched only on the keyword
  "local") — a maintainer-owned local-tooling check, unrelated to the Platform API and below the
  `--auto` fold threshold of 0.4. Left pending, as in Phases 24-26.
