//! CLI integration tests

use std::sync::Once;

mod helpers;

// arsenal_config_test, environment_tests, garrison_config_test and
// integration_tests are out of Phase 2's scope per
// .planning/phases/02-functional-gap-closure/02-CONTEXT.md D-09; the D-12
// sweep in plan 02-09 reports on them.
// mod arsenal_config_test;
// mod environment_tests;
// mod garrison_config_test;
// mod integration_tests;

mod error_handling_test;
mod eval_run_test;
mod formation_execution_test;
mod paladin_execution_test;
mod phalanx_execution_test;
mod tool_integration_test;

// CLI output snapshot tests (Task 4.0 - Epic 24)
mod error_output_test;
mod help_output_test;
mod progress_output_test;
mod table_output_test;

static NO_COLOR_INIT: Once = Once::new();

/// Ensure colors are disabled for deterministic snapshot tests.
///
/// Must be called at the start of each test before creating any formatter.
/// Uses `std::sync::Once` to safely set the env var exactly once across
/// parallel test threads.
pub fn ensure_no_color() {
    NO_COLOR_INIT.call_once(|| {
        // SAFETY: Called exactly once before any formatter reads this env var.
        // All snapshot tests in this module require colorless output for
        // deterministic comparisons regardless of terminal capabilities.
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        // Also override the `colored` crate's global state directly,
        // in case it cached its decision before we set the env var.
        colored::control::set_override(false);
    });
}
