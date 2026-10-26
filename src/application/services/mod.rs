pub mod analysis;
pub mod arsenal;
/// Assistant validation, publishing and resolution facade (PLAT-04, D-28..D-33).
pub mod assistant;
pub mod battalion;
pub mod chronicle;
pub mod content;
pub mod herald;
pub mod log_orchestrator;
pub mod notification_orchestrator;
pub mod orchestration;
pub mod paladin;
pub mod parley;
pub mod queue_orchestrator;
/// Run submission, resolution and worker services (Platform API, PLAT-01/02).
pub mod run;
pub mod sanctum;
/// The Treasurer's admission slice (ALLOW-01/02/04): per-tenant and per-API-key allowances checked before a run is persisted.
pub mod treasurer;
pub mod waypoint_retention;
