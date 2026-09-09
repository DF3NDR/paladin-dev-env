//! CLI commands module

// Original Epic 18 commands
pub mod council;
pub mod features;
pub mod muster;
pub mod onboarding;
pub mod setup_check;

// Migrated commands from src/cli/commands/
pub mod agent;
pub mod arsenal;
pub mod battalion;
pub mod maneuver;

// Evaluation harness CLI (28-12, D-33): `paladin-cli eval run`.
pub mod eval;

// Graph/run export CLI (28-13, D-23): `paladin-cli graph export` / `paladin-cli run export`.
pub mod graph;
pub mod run;
