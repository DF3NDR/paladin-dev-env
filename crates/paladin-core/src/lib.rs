//! # paladin-core
//!
//! Pure domain types for the Paladin framework.
//!
//! This crate contains all domain entities and base primitives with zero
//! dependencies on infrastructure, LLM providers, databases, or HTTP clients.
//! It is the foundational crate that all other Paladin workspace crates depend on.
//!
//! ## Module Structure
//!
//! - [`base`] — Foundation primitives: `Node<T>`, `Collection`, `Field`, `Message`, `Action`, `Event`
//! - [`platform`] — Domain entities: `Paladin`, Battalion types, `Garrison`, `Arsenal`, `Citadel`, `Herald`, `Sanctum`

#![warn(missing_docs)]

// pub mod base;
/// Foundation primitives and framework base types.
#[allow(missing_docs)]
pub mod base;
/// Core platform domain entities and containers.
#[allow(missing_docs)]
pub mod platform;

// --- Crate prelude (D-09): re-exports only `Aegis` and `Transience`, never
// the retry-policy type from either family -- the pre-existing v0.9
// legacy battalion policy struct (see `platform::container::battalion`)
// stays reachable only by its own path, so no glob import of this crate's
// root can silently bind the wrong one (RESEARCH.md Pitfall 5). Do not
// re-export it here, under any name, from this module.
pub use platform::container::aegis::Aegis;
pub use platform::container::transience::Transience;
// `Page` is deliberately NOT re-exported here (D-09): the name is too
// generic to glob-import safely, unlike `Namespace`/`VaultRecord`/
// `VaultError`, which carry no ambiguity with any other type in this crate.
pub use platform::container::vault::{Namespace, VaultError, VaultRecord};
