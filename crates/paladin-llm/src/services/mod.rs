//! Services that compose port traits — the `Quartermaster` prompt-budgeting service and
//! any future services following the same shape.
//!
//! A service in this module is not a provider adapter: it holds no HTTP client, no API
//! key, no wire-format knowledge. It composes port traits (e.g. [`TokenCounter`](
//! paladin_ports::output::token_counter_port::TokenCounter),
//! [`LlmPort`](paladin_ports::output::llm_port::LlmPort)) to provide framework-level
//! capability that every adapter benefits from without any adapter having to implement it
//! itself. See [`llm_analysis_service`](crate::llm_analysis_service) for the precedent
//! this module follows.

/// The Quartermaster — measures an assembled prompt against a provider's declared
/// context window, enforces it pre-flight, and apportions a bounded allotment under
/// caller-supplied priority.
pub mod quartermaster;
