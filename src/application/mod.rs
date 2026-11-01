//! Application Layer (facade)
//!
//! Application-level services and coordination logic for the Paladin facade crate.
//! Port **trait definitions** live in the [`paladin_ports`] crate, not here; this module
//! holds the facade's own application services that wire those ports together.
//!
//! # Dependency Flow
//!
//! ```text
//! Infrastructure ──implements──> Ports (paladin-ports) <──uses── Application services
//!                                                                        │ uses
//!                                                                        ▼
//!                                                                      Core
//! ```
//!
//! # Modules
//!
//! ## \[services\]
//!
//! Application services that implement and orchestrate business workflows:
//!
//! - **Paladin**: [`PaladinBuilder`](crate::application::services::paladin::paladin_builder::PaladinBuilder),
//!   [`PaladinExecutionService`](crate::application::services::paladin::paladin_execution_service::PaladinExecutionService),
//!   plus planning, prompt-generation, temperature, and handoff services.
//! - **Orchestration**: the classic content/job orchestrator, scheduler, listeners, and queue/log/
//!   notification orchestrators.
//! - **Arsenal / Herald / Content / Sanctum**: registry and coordination services that compose the
//!   corresponding leaf-crate adapters.
//!
//! Multi-agent execution services (Formation, Phalanx, Campaign, Chain of Command, Commander) live
//! in the [`paladin_battalion`] crate and are re-exported via
//! [`services::battalion`](crate::application::services::battalion).
//!
//! ## \[errors\]
//!
//! Facade-level cross-cutting error types (e.g. `PlanningError`, `PromptError`).
//!
//! ## \[cli\] (feature `cli`)
//!
//! Command-line interface: command implementations, config loaders, formatters, interactive
//! prompts, and templates.
//!
//! # Example
//!
//! ```ignore
//! use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
//! use std::sync::Arc;
//!
//! let paladin = PaladinBuilder::new(llm_port)
//!     .name("ResearchPaladin")
//!     .system_prompt("You are a research assistant")
//!     .with_garrison(garrison_port)
//!     .build()?;
//! ```

// Internal modules (public for testing, not part of stable API)
#[cfg(feature = "cli")]
#[allow(missing_docs)]
pub mod cli;
#[allow(missing_docs)]
pub mod errors;
#[allow(missing_docs)]
pub mod services;
