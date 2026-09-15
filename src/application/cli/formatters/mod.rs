//! Output formatting utilities for CLI
//!
//! This module provides rich terminal output including colors, tables,
//! progress indicators, and box drawing.

pub mod output;
pub mod progress;
pub mod table;

pub use output::{OutputFormatter, OutputStyle, format_token_usage_summary};
pub use progress::{ProgressIndicator, Spinner};
pub use table::TableFormatter;
