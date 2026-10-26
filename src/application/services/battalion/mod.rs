//! Battalion Use Cases
//!
//! This module re-exports all battalion orchestration types from the
//! `paladin-battalion` crate.  It is kept as a thin shim so that existing
//! `use paladin::application::services::battalion::*` import paths continue
//! to resolve without modification.

pub use paladin_battalion::campaign_service;
pub use paladin_battalion::chain_of_command_service;
pub use paladin_battalion::commander;
pub use paladin_battalion::conclave_execution_service;
pub use paladin_battalion::council_service;
pub use paladin_battalion::error_aggregation;
pub use paladin_battalion::formation_service;
pub use paladin_battalion::grove_service;
pub use paladin_battalion::maneuver;
pub use paladin_battalion::phalanx_service;

/// Compatibility shim: flow_visualizer is now at maneuver::visualizer
pub use paladin_battalion::maneuver as flow_visualizer_compat;

/// Compatibility shim: maneuver types from new location
pub mod flow_visualizer {
    pub use paladin_battalion::maneuver::visualizer::*;
}

/// Compatibility shim: ManeuverExecutionService from new location
pub mod maneuver_service {
    pub use paladin_battalion::maneuver::service::*;
}
