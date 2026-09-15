//! Garrison Adapters — re-exported from `paladin-memory`.
//!
//! All garrison implementations have been extracted into the `paladin-memory`
//! crate. This module provides backward-compatible re-exports.

pub use paladin_memory::garrison::InMemoryGarrison;
pub use paladin_memory::garrison::SqliteGarrison;
#[cfg(feature = "content-processing")]
pub use paladin_memory::garrison::TiktokenCounter;

/// In-memory garrison (backward-compatible sub-module path).
pub mod in_memory_garrison {
    pub use paladin_memory::garrison::InMemoryGarrison;
}

/// SQLite garrison (backward-compatible sub-module path).
pub mod sqlite_garrison {
    pub use paladin_memory::garrison::SqliteGarrison;
}

/// Token counter utilities (backward-compatible sub-module path).
#[cfg(feature = "content-processing")]
pub mod token_counter {
    pub use paladin_memory::garrison::TiktokenCounter;
}
