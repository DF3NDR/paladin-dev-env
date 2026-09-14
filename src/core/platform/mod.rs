// Re-export container from paladin-core crate (the extracted domain crate).
// A local `pub mod container` is used instead of `pub use paladin_core::platform::container`
// so that `container::battalion` can be extended with the `maneuver` and `parser` sub-modules
// that have been moved from paladin-core into paladin-battalion.
#[allow(missing_docs)]
pub mod container {
    // ── flat file modules from paladin-core ───────────────────────────────────
    pub use paladin_core::platform::container::autonomous_config;
    pub use paladin_core::platform::container::citadel;
    pub use paladin_core::platform::container::citadel_error;
    pub use paladin_core::platform::container::comment;
    pub use paladin_core::platform::container::content;
    pub use paladin_core::platform::container::content_list;
    pub use paladin_core::platform::container::document;
    pub use paladin_core::platform::container::execution_result;
    pub use paladin_core::platform::container::garrison;
    pub use paladin_core::platform::container::garrison_error;
    pub use paladin_core::platform::container::handoff;
    pub use paladin_core::platform::container::heartbeat;
    pub use paladin_core::platform::container::herald;
    pub use paladin_core::platform::container::herald_error;
    pub use paladin_core::platform::container::job;
    pub use paladin_core::platform::container::log;
    pub use paladin_core::platform::container::notification;
    pub use paladin_core::platform::container::orchestration_context;
    pub use paladin_core::platform::container::paladin;
    pub use paladin_core::platform::container::paladin_config;
    pub use paladin_core::platform::container::paladin_error;
    pub use paladin_core::platform::container::planning;
    pub use paladin_core::platform::container::prompt;
    pub use paladin_core::platform::container::queue_config;
    pub use paladin_core::platform::container::queue_item;
    pub use paladin_core::platform::container::registry_error;
    pub use paladin_core::platform::container::sanctum;
    pub use paladin_core::platform::container::schedule;
    pub use paladin_core::platform::container::task;
    pub use paladin_core::platform::container::token_usage;
    pub use paladin_core::platform::container::trigger;
    pub use paladin_core::platform::container::user;
    pub use paladin_core::platform::container::user_group;
    pub use paladin_core::platform::container::vision;
    pub use paladin_core::platform::container::workflow;

    // ── directory modules from paladin-core ──────────────────────────────────
    pub use paladin_core::platform::container::arsenal;

    // ── battalion: re-export paladin-core types + add maneuver/parser shims ──
    pub mod battalion {
        // All existing battalion types (BattalionConfig, BattalionError, etc.)
        // and sub-modules (campaign, chain_of_command, formation, phalanx, etc.)
        pub use paladin_core::platform::container::battalion::*;

        // Restore backward-compatible paths for the moved Maneuver DSL
        pub mod maneuver {
            pub use paladin_battalion::maneuver::*;
            pub mod parser {
                pub use paladin_battalion::maneuver::parser::*;
            }
        }

        // Restore `battalion::parser` as a direct alias (some consumers use
        // `battalion::parser::FlowParser` without going through `maneuver`)
        pub mod parser {
            pub use paladin_battalion::maneuver::parser::*;
        }
    }
}

#[allow(missing_docs)]
pub mod manager;
