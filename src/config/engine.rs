//! Configuration for the superstep engine's bounded-iteration limits and
//! Waypoint persistence durability (X-09, CF-FR-13).

use crate::config::env_utils::{EnvOverridable, read_env};
use paladin_battalion::engine::{EngineLimits, WaypointDurability};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Configuration for the `WarEngine`'s `EngineLimits` and
/// `WaypointDurability` (X-09, CF-FR-13).
///
/// Mirrors [`crate::config::citadel::CitadelConfig`] and
/// [`crate::config::waypoint_retention::WaypointRetentionConfig`]'s shape
/// field-for-field (`Default` + `validate()` + `EnvOverridable`). Every
/// field's default below equals today's `EngineLimits::default()` /
/// `WaypointDurability::Strict` exactly, so a v0.9 deployment that never
/// mentions this struct boots v0.10 with identical engine behavior (X-09's
/// "new tunables default to today's behavior" requirement) --
/// `default_engine_config_matches_todays_engine_defaults` (this module's own
/// tests) asserts it mechanically rather than leaving the claim asserted-but-
/// unchecked.
///
/// `waypoint_durability` keeps its real `WaypointDurability` type rather than
/// widening to `String`: that enum (defined in `paladin-battalion`) derives
/// neither `Serialize` nor `Deserialize`, and this plan may not edit
/// `crates/paladin-battalion/`, so a private `#[serde(with = ...)]` module
/// below (`waypoint_durability_serde`) supplies the (de)serialization this
/// struct's own derive cannot reach through the field's type directly.
///
/// # Examples
///
/// ```
/// use paladin::config::engine::EngineConfig;
///
/// let config = EngineConfig::default();
/// assert!(config.validate().is_ok());
/// assert_eq!(config.max_supersteps, 50);
/// assert_eq!(config.max_muster_tasks, 100);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Maximum number of supersteps before a run fails with
    /// `EngineError::RecursionLimitExceeded`. Must be `>= 1`.
    pub max_supersteps: u64,
    /// Maximum number of times any single node may execute within one run,
    /// before `EngineError::NodeVisitLimitExceeded`. Must be `>= 1`.
    pub max_node_visits: u32,
    /// Optional wall-clock timeout for the whole run, in seconds. `None`
    /// means no timeout (plumbing-only this phase; Doc 04/`FT-03` owns
    /// timeout semantics).
    pub run_timeout_secs: Option<u64>,
    /// Whether a Waypoint persistence failure fails the run (`Strict`,
    /// default; ENG-FR-11) or is logged as a warning while the run continues
    /// (`BestEffort`).
    #[serde(with = "waypoint_durability_serde")]
    pub waypoint_durability: WaypointDurability,
    /// Maximum number of tasks a single `NextStep::Muster` directive may
    /// request (CF-FR-13, D-16). Must be `>= 1`.
    pub max_muster_tasks: u32,
}

// A manual Default impl (no derive macro), colocated with `validate()`'s
// zero-checks, mirroring `WaypointRetentionConfig`'s convention. Unlike that
// struct, most of these field values (50, 25, 100) do NOT coincide with
// their own default value of zero, so clippy's derivable-impls lint has no
// complaint here either way -- the manual form is kept anyway for the same
// explicitness the house convention establishes: the defaults-to-today's-
// behavior contract is stated in code, not left to a derive macro.
#[allow(clippy::derivable_impls)]
impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_supersteps: 50,
            max_node_visits: 25,
            run_timeout_secs: None,
            waypoint_durability: WaypointDurability::Strict,
            max_muster_tasks: 100,
        }
    }
}

impl EngineConfig {
    /// Validates the engine configuration. Rejects `0` for `max_supersteps`,
    /// `max_node_visits` and `max_muster_tasks` (each must be `>= 1`), and
    /// rejects `Some(0)` for `run_timeout_secs`, in the same style
    /// `WaypointRetentionConfig::validate` rejects a zero bound.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::engine::EngineConfig;
    ///
    /// let mut config = EngineConfig::default();
    /// assert!(config.validate().is_ok());
    ///
    /// config.max_supersteps = 0;
    /// assert!(config.validate().is_err());
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if self.max_supersteps == 0 {
            return Err("max_supersteps must be greater than 0".to_string());
        }
        if self.max_node_visits == 0 {
            return Err("max_node_visits must be greater than 0".to_string());
        }
        if self.max_muster_tasks == 0 {
            return Err("max_muster_tasks must be greater than 0".to_string());
        }
        if self.run_timeout_secs == Some(0) {
            return Err("run_timeout_secs must be greater than 0 when set".to_string());
        }
        Ok(())
    }
}

impl EnvOverridable for EngineConfig {
    fn apply_env_overrides(&mut self) {
        if let Some(v) = read_env::<u64>("APP_ENGINE_MAX_SUPERSTEPS") {
            self.max_supersteps = v;
        }
        if let Some(v) = read_env::<u32>("APP_ENGINE_MAX_NODE_VISITS") {
            self.max_node_visits = v;
        }
        if let Some(v) = read_env::<u64>("APP_ENGINE_RUN_TIMEOUT_SECS") {
            self.run_timeout_secs = Some(v);
        }
        if let Some(v) = read_env::<String>("APP_ENGINE_WAYPOINT_DURABILITY") {
            match v.to_ascii_lowercase().replace('_', "").as_str() {
                "strict" => self.waypoint_durability = WaypointDurability::Strict,
                "besteffort" => self.waypoint_durability = WaypointDurability::BestEffort,
                // Unparseable: leave the field at its prior value, matching
                // read_env's own silently-swallowed-parse-error contract for
                // every numeric field above.
                _ => {}
            }
        }
        if let Some(v) = read_env::<u32>("APP_ENGINE_MAX_MUSTER_TASKS") {
            self.max_muster_tasks = v;
        }
    }
}

impl From<EngineConfig> for EngineLimits {
    /// Convert a validated `EngineConfig` into the `EngineLimits` a
    /// `WarGraph` is constructed with. `run_timeout_secs` maps into
    /// `Option<Duration>`; `waypoint_durability` stays reachable on the
    /// source `EngineConfig` value itself (a public field this conversion
    /// does not consume) -- pass it to `WarEngine::with_durability`
    /// separately.
    ///
    /// # Examples
    ///
    /// ```
    /// use paladin::config::engine::EngineConfig;
    /// use paladin_battalion::engine::EngineLimits;
    ///
    /// let config = EngineConfig::default();
    /// let limits: EngineLimits = config.into();
    /// assert_eq!(limits, EngineLimits::default());
    /// ```
    fn from(config: EngineConfig) -> Self {
        Self {
            max_supersteps: config.max_supersteps,
            max_node_visits: config.max_node_visits,
            run_timeout: config.run_timeout_secs.map(Duration::from_secs),
            max_muster_tasks: config.max_muster_tasks,
        }
    }
}

/// `#[serde(with = ...)]` shim for the `waypoint_durability` field (private
/// to this module): `WaypointDurability` (`paladin-battalion`) derives
/// neither `Serialize` nor `Deserialize`, and this plan may not edit that
/// crate. Renders as the strings `"strict"` / `"best_effort"`, parsed back
/// case-insensitively (`replace('_', "")` before matching, so
/// `"BestEffort"`, `"best_effort"` and `"BEST_EFFORT"` all resolve).
mod waypoint_durability_serde {
    use super::WaypointDurability;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &WaypointDurability, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            WaypointDurability::Strict => "strict",
            WaypointDurability::BestEffort => "best_effort",
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<WaypointDurability, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        match raw.to_ascii_lowercase().replace('_', "").as_str() {
            "strict" => Ok(WaypointDurability::Strict),
            "besteffort" => Ok(WaypointDurability::BestEffort),
            other => Err(serde::de::Error::custom(format!(
                "unknown waypoint_durability value: {other}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use serial_test::serial;
    use std::env;
    use std::sync::Arc;

    use paladin_battalion::engine::node::{NodeContext, NodeError, StateNode};
    use paladin_battalion::engine::{EngineError, NodeSpec, RunOutcome, WarEngine, WarGraph};
    use paladin_core::platform::container::battlefield::{
        Battlefield, BattlefieldSchema, StateDelta,
    };
    use paladin_core::platform::container::directive::{Directive, MusterTask, NextStep};
    use paladin_core::platform::container::paladin::Paladin;
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
    use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    #[test]
    fn default_engine_config_matches_todays_engine_defaults() {
        let config = EngineConfig::default();
        assert_eq!(config.waypoint_durability, WaypointDurability::Strict);

        let limits: EngineLimits = config.into();
        assert_eq!(limits, EngineLimits::default());
    }

    #[test]
    fn validate_rejects_zero_limits() {
        let config = EngineConfig {
            max_supersteps: 0,
            ..EngineConfig::default()
        };
        assert!(config.validate().unwrap_err().contains("max_supersteps"));

        let config = EngineConfig {
            max_node_visits: 0,
            ..EngineConfig::default()
        };
        assert!(config.validate().unwrap_err().contains("max_node_visits"));

        let config = EngineConfig {
            max_muster_tasks: 0,
            ..EngineConfig::default()
        };
        assert!(config.validate().unwrap_err().contains("max_muster_tasks"));

        let config = EngineConfig {
            run_timeout_secs: Some(0),
            ..EngineConfig::default()
        };
        assert!(config.validate().unwrap_err().contains("run_timeout_secs"));
    }

    #[test]
    fn validate_accepts_defaults_and_unset_timeout() {
        let config = EngineConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.run_timeout_secs, None);
    }

    #[test]
    #[serial]
    fn env_overrides_apply_for_every_field() {
        let default = EngineConfig::default();

        unsafe { env::set_var("APP_ENGINE_MAX_SUPERSTEPS", "10") };
        let mut config = EngineConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.max_supersteps, 10);
        assert_eq!(config.max_node_visits, default.max_node_visits);
        assert_eq!(config.run_timeout_secs, default.run_timeout_secs);
        assert_eq!(config.waypoint_durability, default.waypoint_durability);
        assert_eq!(config.max_muster_tasks, default.max_muster_tasks);
        unsafe { env::remove_var("APP_ENGINE_MAX_SUPERSTEPS") };

        unsafe { env::set_var("APP_ENGINE_MAX_NODE_VISITS", "5") };
        let mut config = EngineConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.max_node_visits, 5);
        assert_eq!(config.max_supersteps, default.max_supersteps);
        assert_eq!(config.run_timeout_secs, default.run_timeout_secs);
        assert_eq!(config.waypoint_durability, default.waypoint_durability);
        assert_eq!(config.max_muster_tasks, default.max_muster_tasks);
        unsafe { env::remove_var("APP_ENGINE_MAX_NODE_VISITS") };

        unsafe { env::set_var("APP_ENGINE_RUN_TIMEOUT_SECS", "30") };
        let mut config = EngineConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.run_timeout_secs, Some(30));
        assert_eq!(config.max_supersteps, default.max_supersteps);
        assert_eq!(config.max_node_visits, default.max_node_visits);
        assert_eq!(config.waypoint_durability, default.waypoint_durability);
        assert_eq!(config.max_muster_tasks, default.max_muster_tasks);
        unsafe { env::remove_var("APP_ENGINE_RUN_TIMEOUT_SECS") };

        unsafe { env::set_var("APP_ENGINE_WAYPOINT_DURABILITY", "best_effort") };
        let mut config = EngineConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.waypoint_durability, WaypointDurability::BestEffort);
        assert_eq!(config.max_supersteps, default.max_supersteps);
        assert_eq!(config.max_node_visits, default.max_node_visits);
        assert_eq!(config.run_timeout_secs, default.run_timeout_secs);
        assert_eq!(config.max_muster_tasks, default.max_muster_tasks);
        unsafe { env::remove_var("APP_ENGINE_WAYPOINT_DURABILITY") };

        unsafe { env::set_var("APP_ENGINE_MAX_MUSTER_TASKS", "7") };
        let mut config = EngineConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.max_muster_tasks, 7);
        assert_eq!(config.max_supersteps, default.max_supersteps);
        assert_eq!(config.max_node_visits, default.max_node_visits);
        assert_eq!(config.run_timeout_secs, default.run_timeout_secs);
        assert_eq!(config.waypoint_durability, default.waypoint_durability);
        unsafe { env::remove_var("APP_ENGINE_MAX_MUSTER_TASKS") };
    }

    #[test]
    #[serial]
    fn unparseable_env_value_leaves_the_field_at_its_prior_value() {
        unsafe {
            env::set_var("APP_ENGINE_MAX_SUPERSTEPS", "not-a-number");
            env::set_var("APP_ENGINE_MAX_MUSTER_TASKS", "also-not-a-number");
            env::set_var("APP_ENGINE_WAYPOINT_DURABILITY", "not-a-durability");
        }

        let mut config = EngineConfig::default();
        config.apply_env_overrides();

        let default = EngineConfig::default();
        assert_eq!(config.max_supersteps, default.max_supersteps);
        assert_eq!(config.max_muster_tasks, default.max_muster_tasks);
        assert_eq!(config.waypoint_durability, default.waypoint_durability);

        unsafe {
            env::remove_var("APP_ENGINE_MAX_SUPERSTEPS");
            env::remove_var("APP_ENGINE_MAX_MUSTER_TASKS");
            env::remove_var("APP_ENGINE_WAYPOINT_DURABILITY");
        }
    }

    #[test]
    #[serial]
    fn waypoint_durability_parses_both_variants_case_insensitively() {
        for (raw, expected) in [
            ("strict", WaypointDurability::Strict),
            ("STRICT", WaypointDurability::Strict),
            ("Strict", WaypointDurability::Strict),
            ("best_effort", WaypointDurability::BestEffort),
            ("BEST_EFFORT", WaypointDurability::BestEffort),
            ("BestEffort", WaypointDurability::BestEffort),
        ] {
            unsafe { env::set_var("APP_ENGINE_WAYPOINT_DURABILITY", raw) };
            let mut config = EngineConfig::default();
            config.apply_env_overrides();
            assert_eq!(config.waypoint_durability, expected, "input {raw}");
            unsafe { env::remove_var("APP_ENGINE_WAYPOINT_DURABILITY") };
        }
    }

    // --- app_engine_max_muster_tasks_reaches_a_running_engines_limit's test
    // doubles: a minimal `StateNode` planner that musters `task_count` tasks,
    // a no-op worker, and a `PaladinPort` never called because this graph
    // has no `NodeSpec::Paladin` node.

    struct MusteringPlanner {
        worker: NodeId,
        task_count: usize,
    }

    #[async_trait]
    impl StateNode for MusteringPlanner {
        async fn run(
            &self,
            _state: &Battlefield,
            _ctx: &NodeContext,
        ) -> Result<Directive, NodeError> {
            let tasks = (0..self.task_count)
                .map(|i| MusterTask {
                    worker: self.worker.clone(),
                    payload: serde_json::json!(i),
                    task_key: format!("task-{i}"),
                })
                .collect();
            Ok(Directive {
                delta: StateDelta::new(),
                next: NextStep::Muster(tasks),
            })
        }
    }

    struct NoopWorker;

    #[async_trait]
    impl StateNode for NoopWorker {
        async fn run(
            &self,
            _state: &Battlefield,
            _ctx: &NodeContext,
        ) -> Result<Directive, NodeError> {
            Ok(StateDelta::new().into())
        }
    }

    struct UnusedPaladinPort;

    #[async_trait]
    impl PaladinPort for UnusedPaladinPort {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            unreachable!("this test's WarGraph has no NodeSpec::Paladin nodes")
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unreachable!("this test's WarGraph has no NodeSpec::Paladin nodes")
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    #[tokio::test]
    #[serial]
    async fn app_engine_max_muster_tasks_reaches_a_running_engines_limit() {
        unsafe { env::set_var("APP_ENGINE_MAX_MUSTER_TASKS", "2") };

        let mut config = EngineConfig::default();
        config.apply_env_overrides();
        assert_eq!(config.max_muster_tasks, 2);
        config.validate().expect("overridden config is valid");

        let limits: EngineLimits = config.into();
        assert_eq!(limits.max_muster_tasks, 2);

        let schema = BattlefieldSchema::new(vec![]);
        let mut graph = WarGraph::new(schema, limits);
        let planner = NodeId::new("planner");
        let worker = NodeId::new("worker");
        graph.add_node(
            planner.clone(),
            NodeSpec::Function(Arc::new(MusteringPlanner {
                worker: worker.clone(),
                task_count: 3,
            })),
        );
        graph.add_worker_template(worker, NodeSpec::Function(Arc::new(NoopWorker)));
        graph.add_entry(planner.clone());

        let engine = WarEngine::new(
            Arc::new(UnusedPaladinPort),
            Arc::new(InMemoryWaypointStore::new()),
        );
        let thread = ThreadId::new("engine-config-muster-limit").expect("valid thread id");
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .expect("start returns Ok(RunOutcome), even for a Muster-limit failure");

        match outcome {
            RunOutcome::Failed {
                error:
                    EngineError::MusterTaskLimitExceeded {
                        node,
                        requested,
                        limit,
                    },
                ..
            } => {
                assert_eq!(node, planner);
                assert_eq!(requested, 3);
                assert_eq!(limit, 2);
            }
            other => panic!("expected Failed(MusterTaskLimitExceeded), got {other:?}"),
        }

        unsafe { env::remove_var("APP_ENGINE_MAX_MUSTER_TASKS") };
    }
}
