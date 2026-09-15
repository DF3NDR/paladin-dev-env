//! Resident agent registry for the HTTP service-host topology.
//!
//! The [`AgentRegistry`] keeps a set of named agents resident in one long-running
//! process so HTTP handlers can look an agent up by id and run it concurrently. Each
//! entry pairs a `Paladin` with its own `PaladinExecutorPort` implementation
//! (a *per-agent executor*), so different agents may be backed by different execution
//! wiring (circuit breakers, RAG, herald, …).
//!
//! Per the project's dependency-flow rule, this module depends only on the
//! `PaladinExecutorPort` *trait* (`paladin-ports`) and the `Paladin` entity
//! (`paladin-core`) — never on the `paladin-ai` facade that holds the concrete
//! `PaladinExecutionService`. The concrete executor (and the concrete
//! [`AgentProvisioner`]) are injected at composition time by the server binary
//! (Milestone 12, Epic 2).

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::user::UserRole;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
use serde::{Deserialize, Serialize};

/// A registry entry: an agent paired with the executor(s) that run it.
///
/// All handles are `Arc`-shared so [`AgentRegistry::get`] can hand an owned,
/// cheaply-cloned entry to a request handler without holding the registry lock. The
/// `streamer` is optional: an agent without a streaming-capable executor still serves
/// the buffered routes (the SSE endpoint falls back to a buffered single chunk).
#[derive(Clone)]
pub struct AgentEntry {
    /// The resident agent.
    pub paladin: Arc<Paladin>,
    /// Buffered executor (always present).
    pub executor: Arc<dyn PaladinExecutorPort>,
    /// Streaming executor, when the agent's backend supports it.
    pub streamer: Option<Arc<dyn StreamingExecutorPort>>,
    /// Per-agent execution timeout override (seconds); `None` uses the server default.
    pub timeout_secs: Option<u64>,
    /// Roles permitted to invoke this agent; empty ⇒ any authenticated caller.
    pub allowed_roles: Vec<UserRole>,
}

/// Declarative description of an agent to provision at runtime.
///
/// Sent in the body of `POST /agents`. Because `paladin-web` cannot itself build a
/// [`Paladin`] (that needs an `LlmPort` and the builder, which live behind the
/// facade), an [`AgentProvisioner`] turns this spec into a concrete
/// `(Paladin, executor)` pair.
///
/// Only the fields needed to identify and shape an agent are included here; the
/// provisioner is free to apply defaults for everything else. More fields can be
/// added without breaking existing callers (all optional fields use `#[serde(default)]`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AgentSpec {
    /// Registry id (the path segment in `/agents/{id}/…`). Client-supplied and unique.
    pub id: String,
    /// Human-friendly display name.
    pub name: String,
    /// LLM model identifier (e.g. `"gpt-4"`).
    pub model: String,
    /// System prompt defining the agent's behavior.
    pub system_prompt: String,
    /// Optional response randomness (`0.0`–`1.0`); the provisioner applies a default when absent.
    #[serde(default)]
    pub temperature: Option<f32>,
    /// Optional stop words that terminate execution.
    #[serde(default)]
    pub stop_words: Vec<String>,
    /// Optional per-agent execution timeout (seconds); `None` uses the server default.
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
    /// Roles permitted to invoke this agent (`"admin"`/`"user"`); empty ⇒ any caller.
    #[serde(default)]
    #[schema(value_type = Vec<String>)]
    pub allowed_roles: Vec<UserRole>,
}

/// Error returned when an [`AgentProvisioner`] cannot turn a spec into an agent.
#[derive(Debug, thiserror::Error)]
pub enum ProvisionError {
    /// The spec was structurally valid JSON but semantically unusable
    /// (e.g. an unknown model or an out-of-range temperature).
    #[error("invalid agent spec: {0}")]
    InvalidSpec(String),

    /// Provisioning failed while building the agent or its executor
    /// (e.g. the LLM provider could not be wired).
    #[error("provisioning failed: {0}")]
    Failed(String),
}

/// Builds concrete `(Paladin, executor)` pairs from an [`AgentSpec`].
///
/// This is the seam that keeps `paladin-web` decoupled from the facade: the web
/// layer holds an `Arc<dyn AgentProvisioner>` and calls [`provision`](AgentProvisioner::provision)
/// when handling `POST /agents`; the concrete implementation (which uses the
/// `PaladinBuilder` + an `LlmPort`) lives in the composition root (Milestone 12,
/// Epic 2). Implementations must be `Send + Sync` to be shared across requests.
#[async_trait]
pub trait AgentProvisioner: Send + Sync {
    /// Materialize an agent (and its executor(s)) from a spec.
    ///
    /// # Errors
    ///
    /// Returns [`ProvisionError`] if the spec is unusable or the agent/executor
    /// cannot be constructed.
    async fn provision(&self, spec: &AgentSpec) -> Result<ProvisionedAgent, ProvisionError>;
}

/// The output of [`AgentProvisioner::provision`]: a built agent plus its executor(s).
///
/// `streamer` is optional so a provisioner may build buffered-only agents; concrete
/// provisioners that wire a streaming-capable executor set it to `Some`.
pub struct ProvisionedAgent {
    /// The built agent.
    pub paladin: Paladin,
    /// Buffered executor.
    pub executor: Arc<dyn PaladinExecutorPort>,
    /// Streaming executor, when supported.
    pub streamer: Option<Arc<dyn StreamingExecutorPort>>,
}

/// A thread-safe, in-memory registry of resident agents keyed by id.
///
/// Reads (`get`, `list`, `contains`) and mutations (`insert`, `remove`) are all
/// synchronous and take the lock only briefly; no lock is ever held across an
/// `.await`. A lock poisoned by a panicking thread is recovered transparently
/// (`into_inner`) rather than propagated, so a single poisoned operation cannot take
/// the whole service down.
#[derive(Default)]
pub struct AgentRegistry {
    agents: RwLock<HashMap<String, AgentEntry>>,
}

impl AgentRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a registry pre-populated from an initial set of agents.
    ///
    /// Later entries with a duplicate id overwrite earlier ones (construction-time
    /// wins differ from the runtime [`insert`](Self::insert) policy, which refuses to
    /// overwrite).
    pub fn from_agents(
        agents: impl IntoIterator<Item = (String, Arc<Paladin>, Arc<dyn PaladinExecutorPort>)>,
    ) -> Self {
        let map = agents
            .into_iter()
            .map(|(id, paladin, executor)| {
                (
                    id,
                    AgentEntry {
                        paladin,
                        executor,
                        streamer: None,
                        timeout_secs: None,
                        allowed_roles: Vec::new(),
                    },
                )
            })
            .collect();
        Self {
            agents: RwLock::new(map),
        }
    }

    /// Look an agent up by id, returning a cloned [`AgentEntry`].
    ///
    /// Returns `None` if no agent is registered under `id`.
    pub fn get(&self, id: &str) -> Option<AgentEntry> {
        let guard = self.agents.read().unwrap_or_else(|e| e.into_inner());
        guard.get(id).cloned()
    }

    /// Report whether an agent is registered under `id`.
    pub fn contains(&self, id: &str) -> bool {
        let guard = self.agents.read().unwrap_or_else(|e| e.into_inner());
        guard.contains_key(id)
    }

    /// List all registered agents as `(id, agent)` pairs.
    ///
    /// The executor half is omitted because callers list agents to describe them,
    /// not to run them. Order is unspecified.
    pub fn list(&self) -> Vec<(String, Arc<Paladin>)> {
        let guard = self.agents.read().unwrap_or_else(|e| e.into_inner());
        guard
            .iter()
            .map(|(id, entry)| (id.clone(), Arc::clone(&entry.paladin)))
            .collect()
    }

    /// Register a new buffered-only agent (no streaming).
    ///
    /// Returns `true` if the agent was inserted, or `false` if an agent was already
    /// registered under `id` (in which case the existing entry is left untouched).
    /// This non-overwriting policy lets the handler map a duplicate to `409 Conflict`.
    pub fn insert(
        &self,
        id: impl Into<String>,
        paladin: Arc<Paladin>,
        executor: Arc<dyn PaladinExecutorPort>,
    ) -> bool {
        self.insert_with_streaming(id, paladin, executor, None)
    }

    /// Register a new agent, optionally with a streaming-capable executor.
    ///
    /// Same non-overwriting semantics as [`insert`](Self::insert).
    pub fn insert_with_streaming(
        &self,
        id: impl Into<String>,
        paladin: Arc<Paladin>,
        executor: Arc<dyn PaladinExecutorPort>,
        streamer: Option<Arc<dyn StreamingExecutorPort>>,
    ) -> bool {
        self.insert_entry(
            id,
            AgentEntry {
                paladin,
                executor,
                streamer,
                timeout_secs: None,
                allowed_roles: Vec::new(),
            },
        )
    }

    /// Register a fully-formed [`AgentEntry`] (including any per-agent timeout).
    ///
    /// Same non-overwriting semantics as [`insert`](Self::insert).
    pub fn insert_entry(&self, id: impl Into<String>, entry: AgentEntry) -> bool {
        let id = id.into();
        let mut guard = self.agents.write().unwrap_or_else(|e| e.into_inner());
        if guard.contains_key(&id) {
            return false;
        }
        guard.insert(id, entry);
        true
    }

    /// Remove an agent by id.
    ///
    /// Returns `true` if an agent was removed, or `false` if no agent was registered
    /// under `id` (which the handler maps to `404 Not Found`).
    pub fn remove(&self, id: &str) -> bool {
        let mut guard = self.agents.write().unwrap_or_else(|e| e.into_inner());
        guard.remove(id).is_some()
    }

    /// Number of registered agents.
    pub fn len(&self) -> usize {
        let guard = self.agents.read().unwrap_or_else(|e| e.into_inner());
        guard.len()
    }

    /// Whether the registry holds no agents.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use paladin_core::platform::container::execution_result::{PaladinResult, StopReason};
    use paladin_core::platform::container::paladin::{Paladin, PaladinData};
    use paladin_core::platform::container::paladin_error::PaladinError;
    use paladin_core::platform::container::token_usage::TokenUsage;

    /// Minimal in-test executor: returns a fixed output, never touches an LLM.
    struct StubExecutor {
        output: String,
    }

    #[async_trait]
    impl PaladinExecutorPort for StubExecutor {
        async fn execute(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            Ok(PaladinResult::new(
                self.output.clone(),
                TokenUsage::new(1, 0),
                1,
                1,
                StopReason::Completed,
            ))
        }
    }

    fn test_agent(name: &str, model: &str) -> Arc<Paladin> {
        let data = PaladinData {
            system_prompt: "You are a test agent.".to_string(),
            name: name.to_string(),
            model: model.to_string(),
            ..Default::default()
        };
        Arc::new(Paladin::new(data, Some(name.to_string())))
    }

    fn stub_executor() -> Arc<dyn PaladinExecutorPort> {
        Arc::new(StubExecutor {
            output: "ok".to_string(),
        })
    }

    #[test]
    fn new_registry_is_empty() {
        let registry = AgentRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.get("missing").is_none());
        assert!(!registry.contains("missing"));
    }

    #[test]
    fn from_agents_populates_and_lists() {
        let registry = AgentRegistry::from_agents([
            (
                "researcher".to_string(),
                test_agent("Researcher", "gpt-4"),
                stub_executor(),
            ),
            (
                "summarizer".to_string(),
                test_agent("Summarizer", "gpt-4"),
                stub_executor(),
            ),
        ]);

        assert_eq!(registry.len(), 2);
        let mut ids: Vec<String> = registry.list().into_iter().map(|(id, _)| id).collect();
        ids.sort();
        assert_eq!(
            ids,
            vec!["researcher".to_string(), "summarizer".to_string()]
        );
    }

    #[test]
    fn get_returns_agent_with_expected_metadata() {
        let registry = AgentRegistry::from_agents([(
            "r".to_string(),
            test_agent("Researcher", "gpt-4o"),
            stub_executor(),
        )]);

        let entry = registry.get("r").expect("agent should be present");
        assert_eq!(entry.paladin.node.name, "Researcher");
        assert_eq!(entry.paladin.node.model, "gpt-4o");
        assert!(entry.streamer.is_none(), "from_agents leaves streamer None");
        assert!(registry.get("nope").is_none());
    }

    #[test]
    fn insert_with_streaming_attaches_streamer() {
        struct StubStreamer;
        #[async_trait]
        impl StreamingExecutorPort for StubStreamer {
            async fn execute_stream(
                &self,
                _paladin: &Paladin,
                _input: &str,
            ) -> Result<paladin_ports::output::paladin_port::PaladinStream, PaladinError>
            {
                let (_tx, rx) = tokio::sync::mpsc::channel(1);
                Ok(rx)
            }
        }

        let registry = AgentRegistry::new();
        let streamer: Arc<dyn StreamingExecutorPort> = Arc::new(StubStreamer);
        assert!(registry.insert_with_streaming(
            "s",
            test_agent("S", "gpt-4"),
            stub_executor(),
            Some(streamer),
        ));

        let entry = registry.get("s").expect("present");
        assert!(entry.streamer.is_some(), "streamer should be attached");

        // Buffered-only insert leaves streamer None.
        registry.insert("b", test_agent("B", "gpt-4"), stub_executor());
        assert!(registry.get("b").expect("present").streamer.is_none());
    }

    #[test]
    fn insert_adds_new_agent_and_refuses_duplicate() {
        let registry = AgentRegistry::new();

        let inserted = registry.insert("a", test_agent("A", "gpt-4"), stub_executor());
        assert!(inserted, "first insert should succeed");
        assert!(registry.contains("a"));

        let duplicate = registry.insert("a", test_agent("A2", "gpt-4"), stub_executor());
        assert!(!duplicate, "duplicate id must be refused");
        // The original entry must be untouched.
        let entry = registry.get("a").expect("agent present");
        assert_eq!(entry.paladin.node.name, "A");
    }

    #[test]
    fn remove_reports_presence() {
        let registry = AgentRegistry::from_agents([(
            "a".to_string(),
            test_agent("A", "gpt-4"),
            stub_executor(),
        )]);

        assert!(
            registry.remove("a"),
            "removing a present agent returns true"
        );
        assert!(!registry.contains("a"));
        assert!(
            !registry.remove("a"),
            "removing a missing agent returns false"
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_reads_and_mutations_do_not_deadlock_or_panic() {
        // Seed a set of resident agents that readers will keep resolving.
        let registry = Arc::new(AgentRegistry::from_agents((0..10).map(|i| {
            (
                format!("seed-{i}"),
                test_agent(&format!("Seed{i}"), "gpt-4"),
                stub_executor(),
            )
        })));

        let mut handles = Vec::new();

        // Readers: hammer get/list/contains while mutations happen concurrently.
        for _ in 0..8 {
            let registry = Arc::clone(&registry);
            handles.push(tokio::spawn(async move {
                for _ in 0..500 {
                    // Seed agents are never removed, so this must always resolve.
                    assert!(registry.get("seed-0").is_some());
                    let _ = registry.list();
                    let _ = registry.contains("seed-9");
                }
            }));
        }

        // Writers: churn a disjoint id space (insert then remove) so seeds are stable.
        for w in 0..4 {
            let registry = Arc::clone(&registry);
            handles.push(tokio::spawn(async move {
                for i in 0..250 {
                    let id = format!("tmp-{w}-{i}");
                    registry.insert(&id, test_agent("Tmp", "gpt-4"), stub_executor());
                    assert!(registry.remove(&id));
                }
            }));
        }

        for handle in handles {
            handle.await.expect("task should not panic");
        }

        // After all churn, exactly the 10 seeds remain.
        assert_eq!(registry.len(), 10);
        assert!(registry.get("seed-0").is_some());
    }
}
