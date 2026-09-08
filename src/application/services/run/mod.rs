//! Run submission, resolution and worker services (Platform API, PLAT-01/02).
//!
//! Lives in the facade because this is the only crate that sees the
//! engine, the queue adapter and the repository all at once (D-11): the
//! worker pool drives a real `WarEngine`; `paladin-web` never does
//! (ADR-0031).

/// The seam every assistant source plugs into, plus this slice's
/// code-registered implementation (D-32).
pub mod resolver;
/// `RunSubmissionService` — the facade `RunSubmissionPort` implementation
/// (D-12).
pub mod submission;
/// `RunWorkerPool` — dequeues, drives the engine, and applies the terminal
/// transition (D-11, D-13).
pub mod worker;

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
