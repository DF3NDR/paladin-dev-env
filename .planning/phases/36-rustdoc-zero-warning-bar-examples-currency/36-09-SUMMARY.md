---
phase: 36-rustdoc-zero-warning-bar-examples-currency
plan: 09
subsystem: docs
tags: [examples-gallery, platform-api, http-service-host, webhooks, ssrf, dev-ui, router-parity]

# Dependency graph
requires:
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-01-SUMMARY.md (house example shape -- header/README pair convention,
      mock-adapter/no-external-service offline-first pattern)
  - phase: 36-rustdoc-zero-warning-bar-examples-currency
    provides: 36-07-SUMMARY.md (worktree-serialization scheduling precedent for
      build-heavy plans -- not a code dependency)
  - phase: 34-documentation-currency-audit
    provides: 34-AUDIT.md sec4 (EX-33, EX-55, EX-77..EX-99, EX-104, EX-110 capability rows)
  - phase: 27-platform-api
    provides: the Platform API surface itself (run submission/stream/cancel, assistants,
      schedules, webhook delivery + SSRF guard, thread/run router assembly) this plan
      demonstrates
provides:
  - examples/http_service_host.rs and crates/doc-examples/src/http_service_host.rs --
    router-parity fix: both now mount agent_router + thread_router + run_router (the
    shipped merge order), unwired (matching the server's own off-by-default behavior),
    with the example's drive sequence calling one thread route and one run route
  - examples/platform_api_client.rs -- an in-process, fully in-memory Platform API
    client demonstrating twelve capabilities (submit/stream/cancel, assistants,
    schedules, thread state/resume/history, the dev-ui inspector route, token usage,
    queue/store selection)
  - examples/webhook_receiver.rs -- a webhook receiver demonstrating constant-time
    signature verification over raw captured bytes, a rejected tampered body, and the
    private-address SSRF override against the receiver's own loopback address
  - 36-evidence/36-09-platform.txt -- run output, acceptance-criteria greps, and the
    D-24 closure table for all fifteen EX IDs this plan closes
affects: [36-11, 36-12]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A capability cluster with no existing runnable program gets one dedicated,
      numbered-parts example whose stdout narrates each capability in the order the
      audit row lists it -- proven a seventh time (platform_api_client) and eighth time
      (webhook_receiver) on top of 36-01/36-06/36-07/36-08's precedent (D-15)."
    - "paladin_port_from_settings (src/infrastructure/web/facade_provisioner.rs)
      resolves a REAL provider credential via LlmProviderFactory -- there is no 'mock'
      entry in its provider registry -- so any example that needs the run engine's
      PaladinPort to actually execute a run cannot go through build_run_api and stay
      offline. The offline substitute is a local newtype implementing PaladinPort by
      delegating to a PaladinExecutionService backed by MockLlmAdapter (the same
      pattern http_service_host.rs's agent-router demo already uses one layer up)."
    - "Every paladin-storage port (run, assistant, run_schedule, webhook, waypoint,
      run_queue) ships an in_memory adapter alongside its sqlite/postgres/redis
      adapters -- an example needing the FULL Platform API pipeline offline wires all
      six in-memory adapters directly rather than going through the config-driven
      RunStoreConfig/WaypointStoreConfig selectors (whose backend enum has no InMemory
      variant, only Disabled/Sqlite/Postgres)."
    - "The webhook receiver's own signing scheme is reused verbatim from the shipped
      sender (sign_webhook_body, hmac/sha2), not reimplemented -- a receiver written
      against a hand-rolled construction would silently diverge from the real wire
      format the first time either side changed."

key-files:
  created:
    - examples/platform_api_client.rs
    - examples/webhook_receiver.rs
    - .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-09-platform.txt
  modified:
    - examples/http_service_host.rs
    - crates/doc-examples/src/http_service_host.rs
    - Cargo.toml

key-decisions:
  - "examples/http_service_host.rs and crates/doc-examples/src/http_service_host.rs
    mount ThreadApiState::new()/RunApiState::new() UNWIRED (no waypoint/run store)
    rather than wiring a working backend -- this matches the shipped server's own
    default (RunStoreBackend::Disabled/WaypointStoreBackend::Disabled) and correctly
    demonstrates the EX-33/EX-55 parity claim (routes exist and answer 501, not 404)
    without duplicating platform_api_client.rs's much larger in-memory-pipeline
    demonstration."
  - "platform_api_client.rs bypasses build_run_api entirely and hand-wires the same
    collaborators (repositories, queue, resolvers, worker pool, schedule service,
    webhook delivery drain loop, run event bus) build_run_api wires, substituting a
    local MockEnginePort for paladin_port_from_settings -- the only way to run the
    Platform API's worker pool against a mock LLM, since the provider factory has no
    mock entry."
  - "Thread state/resume/history (EX-77/78/79) and the dev-ui inspector (EX-104) are
    demonstrated against a Waypoint-less thread rather than a genuinely paused
    Workflow-kind assistant -- the registered assistant is Agent-kind, whose run path
    (RunWorkerPool::run_agent) never touches the waypoint store. Building a real
    HITL-paused WarGraph is a separate, substantial demonstration (see
    human_in_the_loop_gate.rs, war_engine_configuration.rs). Recorded as a scope
    deviation per the plan's own D-16 allowance, not silently dropped: the routes are
    proven reachable and correctly wired (real 404/200/501 printed), not populated."
  - "webhook_receiver.rs uses hmac::Mac::verify_slice for the constant-time signature
    comparison rather than a hand-rolled constant-time loop or a plain == -- the
    hmac crate (already a workspace dependency, used by the sender's own
    sign_webhook_body) provides this natively."

requirements-completed: [CURR-13, CURR-14, CURR-15]

coverage:
  - id: D1
    description: "examples/http_service_host.rs and crates/doc-examples/src/http_service_host.rs mount agent_router, thread_router and run_router in the shipped merge order (agent, then thread, then run, then docs), closing the EX-33/EX-55 router-parity defect; the example's drive sequence calls one thread route and one run route and prints the status each got"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "cargo run --example http_service_host --features web-server (exit 0); stdout shows GET /v1/threads/demo-thread/state -> 501 and GET /v1/runs/{run_id} -> 501; ./scripts/check-doc-examples.sh and mdbook build docs/ both exit 0 after the doc-examples module edit"
        status: pass
    human_judgment: false
  - id: D2
    description: "examples/platform_api_client.rs demonstrates the Platform API surface in-process and offline: submit/stream/cancel a run, create+list assistant versions, create+list a schedule, thread state/resume/history, the dev-ui inspector route, token usage (prompt/completion split, never a bare total), and queue/store backend selection (EX-77, EX-78, EX-79, EX-91, EX-92, EX-93, EX-94, EX-95, EX-98, EX-99, EX-104, EX-110)"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example platform_api_client --features \"web-server,dev-ui\" (exit 0); stdout inspected for all twelve capability markers, including the documented thread-state/dev-ui deviation"
        status: pass
    human_judgment: false
  - id: D3
    description: "examples/webhook_receiver.rs demonstrates signature verification over raw captured bytes reusing the shipped sign_webhook_body function, rejects a tampered body via constant-time hmac::Mac::verify_slice, and demonstrates the APP_WEBHOOKS_ALLOW_PRIVATE override against the receiver's own loopback address plus the always-rejected cloud metadata address (EX-96, EX-97)"
    requirement: "CURR-13"
    verification:
      - kind: other
        ref: "env -u OPENAI_API_KEY -u ANTHROPIC_API_KEY -u DEEPSEEK_API_KEY cargo run --example webhook_receiver --features web-server (exit 0); stdout shows a verified genuine delivery, a rejected tampered body, and the SSRF override narration; secret never printed (grep -cE for secret/signing_key print patterns is 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Both new programs are declared as gated example targets (required-features), the bare cargo build --examples selector skips them, and no commit in this plan touches src/ or crates/paladin-* except the doc-examples HTTP-service-host module; make api-surface / check-api-surface.sh reports the surface unchanged across all three commits"
    requirement: "CURR-15"
    verification:
      - kind: other
        ref: "cargo build --examples (exit 0, gated targets skipped); git diff --stat HEAD~3..HEAD -- src/ crates/ (only crates/doc-examples/src/http_service_host.rs); ./scripts/check-api-surface.sh .project/current-exports.txt (unchanged, 3959 items, checked after each commit)"
        status: pass
    human_judgment: false

# Metrics
duration: ~1h30min
completed: 2026-09-17
status: complete
---

# Phase 36 Plan 09: Platform API Client, Webhook Receiver & HTTP-Service-Host Router Parity Summary

**Restored the HTTP-service-host router-parity claim (EX-33, EX-55), shipped a fully offline, in-process Platform API client covering twelve capabilities across the run/assistants/schedules/threads/dev-ui/token-usage surface, and shipped a webhook receiver demonstrating constant-time signature verification and the private-address SSRF override -- three atomic commits, zero `make api-surface` drift.**

## Performance

- **Duration:** ~1h30min
- **Completed:** 2026-09-17T22:34:00Z
- **Tasks:** 3
- **Files modified:** 6 (2 corrected example/doc-examples files, 2 new example binaries, 1 manifest, 1 new evidence file)

## Accomplishments

- Fixed `examples/http_service_host.rs` and `crates/doc-examples/src/http_service_host.rs`,
  both of which previously mounted only the agent router plus the OpenAPI docs router
  while claiming full server parity. Both now merge `agent_router().merge(thread_router()).merge(run_router())`
  in the exact order `src/bin/paladin-server.rs` uses, constructing the thread and run
  states unwired (matching the shipped server's own off-by-default behavior) so their
  routes are reachable and answer `501 not_implemented` rather than being absent
  (`404`). `examples/http_service_host.rs`'s drive sequence now calls one thread route
  and one run route and prints the status each got. Re-ran `./scripts/check-doc-examples.sh`
  and `mdbook build docs/` after the doc-examples module edit -- both green.
- `examples/platform_api_client.rs`: an in-process Platform API client that cannot go
  through `build_run_api` (which resolves a real provider credential via
  `paladin_port_from_settings` and has no "mock" provider entry) and so hand-wires the
  same collaborators `build_run_api` wires -- every durable store the
  `paladin_storage::*::in_memory` adapter for its port, and the run engine's
  `PaladinPort` a local `MockEnginePort` delegating to the same
  `PaladinExecutionService` + `MockLlmAdapter` pair `http_service_host.rs` uses. Drives:
  submit a run and print its id; stream it and print the frozen wire event names;
  submit and cancel a second run; create a stored assistant, publish a second version,
  and list version identifiers; create and list a cron schedule; call thread
  state/resume/history and the admin-and-feature-gated dev-ui inspector route (both
  against a Waypoint-less thread, documented as a scope deviation); print the execute
  response's prompt/completion token split; and print which run-queue/run-store
  implementation is in force.
- `examples/webhook_receiver.rs`: a receiver that captures the raw request body via
  axum's `Bytes` extractor before any deserialization and reuses `sign_webhook_body`
  (the SAME function and `hmac`/`sha2` crates the shipped `WebhookDeliveryService`
  signs with) to recompute the digest, verifying in constant time via
  `hmac::Mac::verify_slice` -- never a plain `==` on the digest bytes. Demonstrates a
  verified genuine delivery, a rejected tampered body under the same signature, and the
  `APP_WEBHOOKS_ALLOW_PRIVATE` override checked against the receiver's own loopback
  address (rejected by default) and the always-rejected cloud metadata address. The
  signing secret is generated in-process from 32 random bytes and is never printed.
- Both new programs are declared as gated example targets in the root manifest
  (`required-features = ["web-server", "dev-ui"]` and `["web-server"]` respectively);
  the bare `cargo build --examples` selector skips both. `./scripts/check-api-surface.sh`
  reports the surface unchanged (3959 items) after every one of the three commits.

## Task Commits

Each task was committed atomically:

1. **Task 1: Router parity for both HTTP-service-host files (EX-33, EX-55)** - `3363d08d` (docs)
2. **Task 2: examples/platform_api_client.rs (EX-77, EX-78, EX-79, EX-91, EX-92, EX-93, EX-94, EX-95, EX-98, EX-99, EX-104, EX-110)** - `64e597a0` (docs)
3. **Task 3: examples/webhook_receiver.rs (EX-96, EX-97)** - `1c21d61c` (docs)

**Plan metadata:** _pending -- this SUMMARY's own commit_

## Files Created/Modified

- `examples/http_service_host.rs` - router-parity fix (three routers merged, one thread route and one run route driven)
- `crates/doc-examples/src/http_service_host.rs` - same router-parity fix inside the anchored region
- `examples/platform_api_client.rs` - new in-process, fully in-memory Platform API client (twelve capabilities)
- `examples/webhook_receiver.rs` - new webhook receiver (constant-time signature verification, SSRF override demo)
- `Cargo.toml` - two new gated example target declarations
- `.planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-09-platform.txt` - run output, acceptance-criteria greps, D-24 closure table

## Closure Table (D-24)

| ID | capability | program or file | commit |
|---|---|---|---|
| EX-33 | HTTP-service-host router parity (example) | examples/http_service_host.rs | 3363d08d |
| EX-55 | HTTP-service-host router parity (doc-examples snippet) | crates/doc-examples/src/http_service_host.rs | 3363d08d |
| EX-77 | Thread state route | examples/platform_api_client.rs | 64e597a0 |
| EX-78 | Thread resume route | examples/platform_api_client.rs | 64e597a0 |
| EX-79 | Thread history route | examples/platform_api_client.rs | 64e597a0 |
| EX-91 | Run submission | examples/platform_api_client.rs | 64e597a0 |
| EX-92 | Run streaming (SSE) | examples/platform_api_client.rs | 64e597a0 |
| EX-93 | Run cancellation | examples/platform_api_client.rs | 64e597a0 |
| EX-94 | Assistants (create/publish/list versions) | examples/platform_api_client.rs | 64e597a0 |
| EX-95 | Schedules (create/list) | examples/platform_api_client.rs | 64e597a0 |
| EX-98 | Run queue backend selection | examples/platform_api_client.rs | 64e597a0 |
| EX-99 | Run store backend selection | examples/platform_api_client.rs | 64e597a0 |
| EX-104 | Dev-ui inspector route | examples/platform_api_client.rs | 64e597a0 |
| EX-110 | Token usage (prompt/completion split) | examples/platform_api_client.rs | 64e597a0 |
| EX-96 | Webhook signature verification | examples/webhook_receiver.rs | 1c21d61c |
| EX-97 | Private-address SSRF override | examples/webhook_receiver.rs | 1c21d61c |

## Decisions Made

- Mounted `ThreadApiState`/`RunApiState` UNWIRED in both HTTP-service-host files rather
  than wiring a working backend there -- matches the shipped server's own
  `Disabled`-by-default behavior and avoids duplicating `platform_api_client.rs`'s much
  larger demonstration in a file meant to stay minimal.
- `platform_api_client.rs` bypasses `build_run_api` and hand-wires the same
  collaborators directly, substituting a local `MockEnginePort` for
  `paladin_port_from_settings` (which resolves a real provider credential and has no
  mock entry) -- the only way to run the Platform API's worker pool against a mock LLM
  offline.
- Thread state/resume/history and the dev-ui inspector are demonstrated against a
  Waypoint-less thread (the registered assistant is Agent-kind, whose run path never
  touches the waypoint store) rather than building a full Workflow-based paused-thread
  demo, which is a separate, substantial undertaking -- recorded as a scope deviation
  per the plan's own D-16 allowance, with real status codes printed and the reason
  stated in the program's own header and stdout, not silently dropped.
- `webhook_receiver.rs` reuses `sign_webhook_body` (the shipped sender's own signing
  function) rather than hand-rolling HMAC construction, and uses
  `hmac::Mac::verify_slice` for constant-time comparison rather than a hand-rolled
  constant-time loop.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed a clippy `manual_is_multiple_of` lint in webhook_receiver.rs**
- **Found during:** Task 3 pre-commit hook (`cargo clippy --workspace --all-targets --all-features -- -D warnings`)
- **Issue:** `hex_decode`'s odd-length check used `s.len() % 2 != 0`, which clippy's
  `manual_is_multiple_of` lint (stable as of the toolchain's clippy version) flags as a
  manual reimplementation of `usize::is_multiple_of`.
- **Fix:** Changed to `!s.len().is_multiple_of(2)`.
- **Files modified:** examples/webhook_receiver.rs
- **Verification:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` exits 0; re-ran the example, behavior unchanged.
- **Committed in:** 1c21d61c (Task 3 commit)

**Scope deviation (recorded, not a defect -- plan's own D-16 allowance):** Thread
state/resume/history (EX-77/EX-78/EX-79) and the dev-ui inspector (EX-104) in
`platform_api_client.rs` are called against a thread carrying no Waypoint, because this
program's one registered assistant is Agent-kind (`Runnable::Agent`), whose run path
(`RunWorkerPool::run_agent`) calls `PaladinPort::execute` directly and never touches
the waypoint store. The routes are proven reachable and correctly wired (real
404/200/501 status codes printed, stated in the program's own header and stdout), not
populated with a genuinely paused thread -- a Workflow-kind assistant (a `WarGraph`
with a human-in-the-loop node) would be needed for that, which is a separate,
substantial demonstration (see `human_in_the_loop_gate.rs`, `war_engine_configuration.rs`).
This was the plan's own stated fallback for exactly this situation, not an
undocumented gap.

---

**Total deviations:** 1 auto-fixed (1 bug -- a lint, caught and fixed by the pre-commit
hook before the commit landed) + 1 documented scope deviation (permitted by the plan's
own D-16 allowance, not a defect).
**Impact on plan:** No scope change beyond the plan's own stated fallback for
Workflow-only capabilities; the clippy fix was necessary for the commit to land at all.

## Issues Encountered

None beyond the clippy lint and the documented scope deviation above.

## User Setup Required

None - no external service configuration required. Both new examples and the corrected
`http_service_host.rs` are fully offline (all three provider-key environment variables
unset, no external service running).

## Next Phase Readiness

- Fifteen more of the fifty-nine Phase 34 audit gap rows are closed (EX-33, EX-55,
  EX-77 through EX-79, EX-91 through EX-99, EX-104, EX-110, EX-96, EX-97). Combined with
  plans 36-01, 36-06, 36-07 and 36-08, forty-four of fifty-nine rows are now closed.
- Plan 36-11 (owns `examples/README.md`) still needs to add a section for
  `platform_api_client.rs` and `webhook_receiver.rs` -- no README edit was made here per
  this plan's own scope note.
- Plan 36-12 (owns CI and `scripts/check-all-examples.sh`) still needs to add the
  `cargo build --example platform_api_client --features "web-server,dev-ui"` and
  `cargo build --example webhook_receiver --features "web-server"` invocations to the
  "Example Muster" CI job and the local examples script, per D-17 -- no CI/script edit
  was made here per this plan's own scope note.
- No blockers for subsequent Phase 36 plans.

---
*Phase: 36-rustdoc-zero-warning-bar-examples-currency*
*Completed: 2026-09-17*

## Self-Check: PASSED

- FOUND: examples/http_service_host.rs
- FOUND: crates/doc-examples/src/http_service_host.rs
- FOUND: examples/platform_api_client.rs
- FOUND: examples/webhook_receiver.rs
- FOUND: .planning/phases/36-rustdoc-zero-warning-bar-examples-currency/36-evidence/36-09-platform.txt
- FOUND commit: 3363d08d
- FOUND commit: 64e597a0
- FOUND commit: 1c21d61c
