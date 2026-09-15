//! Garrison adapters — conversation history storage.
//!
//! Re-exports available adapters based on enabled feature flags.

pub mod in_memory_garrison;
pub use in_memory_garrison::InMemoryGarrison;

#[cfg(feature = "sqlite")]
pub mod sqlite_garrison;
#[cfg(feature = "sqlite")]
pub use sqlite_garrison::SqliteGarrison;

#[cfg(feature = "content-processing")]
pub mod token_counter;
#[cfg(feature = "content-processing")]
pub use token_counter::TiktokenCounter;
