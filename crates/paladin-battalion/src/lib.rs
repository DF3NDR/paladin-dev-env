//! # paladin-battalion
//!
//! Multi-agent orchestration runtime for the Paladin framework.
//!
//! This crate provides all eight Battalion execution patterns and the Commander
//! strategy router. It depends only on `paladin-core` (domain types) and
//! `paladin-ports` (port trait contracts) — never on infrastructure SDKs,
//! database drivers, or LLM provider libraries.
//!
//! ## Patterns
//!
//! - [`formation_service`] — Sequential execution: output of agent N feeds input of agent N+1
//! - [`phalanx_service`] — Concurrent execution: all agents run in parallel
//! - [`campaign_service`] — DAG/graph execution: topologically sorted dependency graph
//! - [`chain_of_command_service`] — Hierarchical delegation: commander delegates to sub-agents
//! - [`conclave_execution_service`] — Mixture-of-experts synthesis
//! - [`council_service`] — Multi-agent discussion and consensus
//! - [`grove_service`] — Intelligent semantic routing
//! - [`maneuver`] — Flow DSL execution (domain types, parser, service, visualizer)
//! - [`commander`] — Strategy auto-detection router
//!
//! ## Utilities
//!
//! - [`error_aggregation`] — Collect and summarise errors across parallel agent runs
//! - [`retry`] — Exponential back-off retry helper

#![warn(missing_docs)]

// Execution services
pub mod campaign_service;
pub mod chain_of_command_service;
pub mod commander;
pub mod conclave_execution_service;
pub mod council_service;
/// Registered evaluators for `EdgeCondition::Custom` (BUG-01, CF-01), shared
/// by `campaign_service` and `engine`.
pub mod edge_evaluator;
/// Superstep execution engine over typed `Battlefield` state (Phase 22).
pub mod engine;
pub mod error_aggregation;
pub mod formation_service;
pub mod grove_service;
pub mod maneuver;
pub mod phalanx_service;
pub mod retry;

/// In-memory `PaladinRegistry` implementation (also used by the application facade).
pub mod in_memory_registry;

pub use edge_evaluator::{
    EdgeConditionEvaluator, EdgeContext, EdgeEvaluatorError, EdgeEvaluatorRegistry,
};
