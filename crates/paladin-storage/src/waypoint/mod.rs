//! Waypoint storage adapters.
//!
//! Implementations of `paladin_ports::output::waypoint_port::WaypointPort`.

/// In-memory implementation, always available (no feature gate, D-01): used
/// for tests and local development.
pub mod in_memory;

/// Shared `WaypointPort` contract suite (D-09): generic async functions every
/// backend runs, unchanged, from its own `#[tokio::test]`s. See
/// `contract_tests::run_all` for a single-call smoke aggregate.
pub mod contract_tests;

/// SQLite implementation of `WaypointPort`, over a versioned migration
/// (Plan 22-06, ENG-05).
#[cfg(feature = "sqlite")]
pub mod sqlite;

/// PostgreSQL implementation of `WaypointPort`, behind the `postgres`
/// feature, over a versioned migration (Plan 22-06, ENG-05, D-01).
#[cfg(feature = "postgres")]
pub mod postgres;

/// Credential redaction for connection-string-derived error text (T-22-18),
/// shared by the `sqlite` and `postgres` backends. Gated the same as its
/// consumers so a build with neither SQL feature enabled does not warn about
/// unused code.
#[cfg(any(feature = "sqlite", feature = "postgres"))]
pub(crate) mod redact;

/// Retention/cleanup routine for pruning old Waypoints, bounded by age
/// and/or per-thread count, with hard exclusions for a thread's latest
/// Waypoint and any `AwaitingInput` Waypoint (ENG-FR-18). Backend-agnostic:
/// built entirely over the existing `WaypointPort` surface, so it runs
/// unchanged over `InMemoryWaypointStore`, `SqliteWaypointStore`, and
/// `PostgresWaypointStore`.
pub mod retention;
