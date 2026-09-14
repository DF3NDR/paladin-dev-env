//! Application-layer services in `paladin-llm`: composable orchestration built on top
//! of the crate's port-facing primitives (provider adapters, [`TokenCounterPort`]),
//! rather than a provider adapter itself.
//!
//! [`TokenCounterPort`]: paladin_ports::output::token_counter_port::TokenCounterPort

/// The Commissary — a v0.10.0-native prompt-budgeting service composing
/// [`paladin_ports::output::llm_port::LlmPort::get_capabilities`] and
/// [`paladin_ports::output::token_counter_port::TokenCounterPort`]: a fail-loud
/// pre-flight guard plus a bounded, priority-ordered allocator with explicit
/// truncation markers. See the module's own doc for the full behavioral contract.
pub mod commissary;
