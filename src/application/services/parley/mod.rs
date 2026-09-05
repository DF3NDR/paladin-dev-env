//! Facade `ParleyPort` Implementation (HITL-05, D-25, D-26)
//!
//! Wires `paladin-ports`' core-typed [`ParleyPort`](paladin_ports::input::parley_port::ParleyPort)
//! trait onto a real `WarEngine`: [`registry::GraphRegistry`] resolves a
//! thread's own graph by the [`GraphFingerprint`](paladin_core::platform::container::waypoint::GraphFingerprint)
//! its latest Waypoint carries (D-26), and [`adapter::ParleyPortAdapter`]
//! validates a submission against that graph, then spawns the continuation
//! as a background task registered with `paladin-battalion`'s
//! `ShutdownCoordinator` (D-21) and returns immediately (D-25).
//!
//! Following `src/application/services/waypoint_retention.rs`'s structural
//! convention: a facade module beside the port it implements, depending on
//! `paladin-battalion` directly (unlike `paladin-web`, which never may,
//! ADR-0031) since this module lives in the root crate.

pub mod adapter;
pub mod registry;

pub use adapter::ParleyPortAdapter;
pub use registry::GraphRegistry;
