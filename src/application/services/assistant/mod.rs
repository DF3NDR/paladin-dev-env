//! Assistant validation, publishing and resolution facade (PLAT-04, D-28..D-33).
//!
//! Assistants become usable end-to-end here: [`AssistantValidator`] makes compile the
//! validation gate (D-31); [`AssistantService`] implements the
//! [`paladin_ports::input::assistant_admin_port::AssistantAdminPort`] every
//! `POST/GET/DELETE /assistants*` route drives; [`StoredAssistantResolver`] and
//! [`ChainedResolver`] extend `services::run::resolver::AssistantResolver` so
//! `POST /runs` can run a stored definition with `latest` frozen at submit (D-30); and
//! [`DocGraphRegistry`] lets a resume find a suspended run's graph through that same
//! frozen version (D-33).

/// Resolves a suspended thread's graph from the run row's frozen
/// `(assistant_id, version)` (D-33).
pub mod doc_registry;
/// `StoredAssistantResolver` + `ChainedResolver` -- stored-first, code-second assistant
/// resolution (D-32).
pub mod resolver;
/// `AssistantService` — the facade `AssistantAdminPort` implementation.
pub mod service;
/// Compile-is-validation for assistant definitions (D-31).
pub mod validator;

pub use doc_registry::DocGraphRegistry;
pub use resolver::{ChainedResolver, StoredAssistantResolver};
pub use service::AssistantService;
pub use validator::{AgentDefinition, AssistantValidator, Validated};

/// The full assistant module's own test suite (validator cases, service
/// create/version/list/delete, resolver cache, freeze-at-submit,
/// code/stored disjointness, resume-via-doc-registry) -- a `#[cfg(test)]`
/// module rather than a `tests/` target so it counts toward `cargo llvm-cov`
/// (D-54).
#[cfg(test)]
mod tests;
