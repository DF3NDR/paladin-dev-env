//! Convenient re-exports of the most commonly used Paladin types.
//!
//! Import everything you need to build and run Paladin agents in one line:
//!
//! ```rust,no_run
//! use paladin::prelude::*;
//! ```
//!
//! This module re-exports the types used in the majority of Paladin programs.
//! For less common types, import them directly from their source modules.
//!
//! ## Example
//!
//! ```rust,no_run
//! use paladin::prelude::*;
//! // Verify core types are in scope
//! let _status = PaladinStatus::Idle;
//! ```

pub use crate::{
    // Battalion / orchestration types (still have short-path aliases)
    BattalionConfig,
    BattalionError,
    // Agent types (still have short-path aliases)
    Paladin,
    PaladinConfig,
    PaladinData,
    PaladinStatus,
};

// Types no longer re-exported as short-path aliases — import from crate roots directly
pub use crate::core::platform::container::arsenal::{Armament, ArsenalError};
pub use crate::core::platform::container::battalion::campaign::Campaign;
pub use crate::core::platform::container::battalion::chain_of_command::ChainOfCommand;
pub use crate::core::platform::container::battalion::council::CouncilBuilder;
pub use crate::core::platform::container::battalion::formation::Formation;
pub use crate::core::platform::container::battalion::grove::GroveBuilder;
pub use crate::core::platform::container::battalion::phalanx::Phalanx;
pub use crate::core::platform::container::battalion::{BattalionResult, BattalionStatus};

pub use paladin_ports::output::arsenal_port::{ArsenalPort, ArsenalRegistry};
pub use paladin_ports::output::garrison_port::{GarrisonError, GarrisonPort};
pub use paladin_ports::output::llm_port::{LlmError, LlmPort, LlmRequest, LlmResponse};
pub use paladin_ports::output::paladin_port::{PaladinResult, StopReason};
pub use paladin_ports::output::sanctum_port::{SanctumError, SanctumPort};

pub use paladin_memory::garrison::InMemoryGarrison;
pub use paladin_memory::sanctum::InMemorySanctum;

pub use crate::application::services::battalion::commander::CommanderBuilder;
pub use crate::application::services::paladin::error::PaladinError;
pub use crate::application::services::paladin::middleware::{
    ExecutionMiddleware, FinalResult, LlmResponseView, MiddlewareFlow, ModelCallContext,
    PromptAssembly, PromptSection, ToolCallContext, ToolFlow,
};
pub use crate::application::services::paladin::paladin_builder::PaladinBuilder;
