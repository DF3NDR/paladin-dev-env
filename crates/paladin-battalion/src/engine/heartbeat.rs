//! The engine's local name for the progress-report handle (plan 25-09,
//! D-18, D-19).
//!
//! [`HeartbeatHandle`] is DEFINED in
//! `paladin_core::platform::container::heartbeat` -- beside every other port
//! value type (ADR-0016) -- because `paladin_ports::output::paladin_port::PaladinPort::execute_observed`
//! must name the same type the engine's idle timer subscribes to, and
//! `paladin-ports` can never depend on this crate. This module is a pure
//! re-export so engine code and engine tests can write
//! `crate::engine::heartbeat::HeartbeatHandle`; there is exactly one
//! definition and no duplicated type.
//!
//! # How the engine uses it
//!
//! `engine::superstep`'s per-attempt body creates a FRESH handle per
//! attempt (so a cancelled attempt's stragglers can never reset the next
//! attempt's timer), places it on `NodeContext.heartbeat`, and:
//!
//! - passes it to `PaladinPort::execute_observed` for a `NodeSpec::Paladin`
//!   node (D-19), whose `PaladinExecutionService` implementation beats on
//!   every completed LLM call, every streamed chunk and every Armament
//!   invocation;
//! - lets a `NodeSpec::Function` node beat it via `ctx.heartbeat()`;
//! - beats it once per CHILD superstep for a `NodeSpec::Battalion` node;
//! - when the node's resolved `TimeoutPolicy::idle_timeout` is `Some`,
//!   subscribes an idle timer that fires only if no beat arrives within
//!   that window (a node without an `idle_timeout` never subscribes, so
//!   `beat()` there is a cheap no-op, D-18).

pub use paladin_core::platform::container::heartbeat::HeartbeatHandle;
