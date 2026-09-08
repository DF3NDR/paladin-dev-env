/// Build a `paladin-web` agent registry from configuration (Milestone 12, Epic 2).
#[cfg(feature = "web-server")]
pub mod agent_host;
/// Concrete `AgentProvisioner` for runtime agent registration (Milestone 12, Epic 2).
#[cfg(feature = "web-server")]
pub mod facade_provisioner;
/// Wires the whole Platform API (Phase 27) into a running server from configuration:
/// stores, queue, worker pool, scheduler, webhook delivery, resolvers and the routers.
#[cfg(feature = "web-server")]
pub mod run_api_wiring;

#[cfg(feature = "web-server")]
pub use paladin_web::*;
