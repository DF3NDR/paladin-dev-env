//! Run submission, resolution and worker services (Platform API, PLAT-01/02).
//!
//! Lives in the facade because this is the only crate that sees the
//! engine, the queue adapter and the repository all at once (D-11): the
//! worker pool drives a real `WarEngine`; `paladin-web` never does
//! (ADR-0031).

/// Cross-instance cancellation: `DbCancellationProbe` (the debounced
/// `CancellationProbe` adapter) and `LocalRunTokens` (the same-instance
/// fast-path registry), D-14/D-15/D-16.
pub mod cancel;
/// `RunEventBus`, `RunEventBusSink` (the `TraceSink` half of the bus's two
/// producers) and `RunEventStreamService` (the facade `RunEventStreamPort`
/// implementation, live/degraded) -- D-24..D-27, PLAT-FR-07.
pub mod events;
/// The seam every assistant source plugs into, plus this slice's
/// code-registered implementation (D-32).
pub mod resolver;
/// `ScheduleService` — the claim-then-submit tick loop driving persisted
/// cron `RunSchedule`s (PLAT-05, D-36..D-39).
pub mod schedule;
/// `RunSubmissionService` — the facade `RunSubmissionPort` implementation
/// (D-12).
pub mod submission;
/// Durable webhook delivery (PLAT-FR-14/15, D-40..D-43): `SsrfGuard`,
/// `sign_webhook_body`, `build_webhook_client`, `WebhookDeliveryService`.
pub mod webhook;
/// `RunWorkerPool` — dequeues, drives the engine, and applies the terminal
/// transition (D-11, D-13).
pub mod worker;

pub use cancel::{DbCancellationProbe, LocalRunTokens};
pub use events::{RunEventBus, RunEventBusSink, RunEventStreamService, map_trace_event};
pub use resolver::{
    AssistantResolver, CodeWorkflowResolver, ResolveError, ResolvedAssistant, Runnable,
};
pub use submission::RunSubmissionService;
pub use worker::{LeaseHeartbeat, RunWorkerOptions, RunWorkerPool, WorkerDispatch, WorkerError};

/// The end-to-end tracer test proving the whole path: HTTP -> repository ->
/// queue -> worker -> engine -> `Completed` (Task 3, D-11). A `#[cfg(test)]`
/// module rather than a `tests/` target so it counts toward `cargo llvm-cov`
/// (D-54).
#[cfg(test)]
mod tracer_e2e;

/// Kill-mid-run redelivery, `AwaitingInput` ack, resume-with-pending-responses,
/// heartbeat cadence and shutdown-drain tests -- the InMemory twin of PRD 06
/// acceptance 2 (27-04 Task 2). A `#[cfg(test)]` module rather than a
/// `tests/` target so it counts toward `cargo llvm-cov` (D-54).
#[cfg(test)]
mod worker_tests;

/// `DbCancellationProbe` debounce/error behavior, `LocalRunTokens`, and the
/// cross-instance cancellation proof (`cross_instance_cancel_probe`,
/// `local_cancel_signals_token`) -- 27-07 Task 2, D-14/D-15/D-16. A
/// `#[cfg(test)]` module rather than a `tests/` target so it counts toward
/// `cargo llvm-cov` (D-54).
#[cfg(test)]
mod cancel_tests;

/// Live-path (real engine, real bus) and degraded-path (polling, real
/// on-disk SQLite for the cross-instance proof) tests for `GET
/// /v1/runs/{run_id}/stream` (27-10 Task 1, D-24..D-27, PLAT-FR-07). A
/// `#[cfg(test)]` module rather than a `tests/` target so it counts toward
/// `cargo llvm-cov` (D-54).
#[cfg(test)]
mod stream_tests;

/// The router-level ten-concurrent-submits race (PRD acceptance 3, D-52)
/// and the fork-from-waypoint end-to-end proof (D-45) -- 27-15 Task 2. A
/// `#[cfg(test)]` module rather than a `tests/` target so it counts toward
/// `cargo llvm-cov` (D-54).
#[cfg(test)]
mod http_surface_tests;
