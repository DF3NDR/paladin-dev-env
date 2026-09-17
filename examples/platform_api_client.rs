//! Runnable example: an in-process Platform API client -- the whole Phase 24/27
//! Platform API surface (thread state/resume/history, run submit/stream/cancel,
//! assistants, schedules, the dev-ui inspector route, token usage, and queue/store
//! selection) driven from one program.
//!
//! Hermetic -- backed by [`MockLlmAdapter`], reads no provider key and starts no
//! external service:
//! `cargo run --example platform_api_client --features "web-server,dev-ui"`.
//! Both features are required: `dev-ui` resolves to `paladin-web`'s own `dev-ui`
//! feature and does not itself enable the web dependency, so `web-server` must be
//! listed too.
//!
//! Every durable store this program wires -- run repository, assistant repository,
//! run schedule repository, webhook delivery repository, waypoint store, run queue --
//! is the `paladin-storage` **in-memory** adapter for that port, never a file or a
//! network connection, so the whole Platform API pipeline (submission -> queue ->
//! worker pool -> engine) runs fully in this one process, offline.
//!
//! ## What this program cannot demonstrate offline
//!
//! Thread state/resume/history and the dev-ui inspector route need a *Workflow*-kind
//! assistant (a `WarGraph` carrying a human-in-the-loop node) suspended at a parley to
//! show a genuinely paused thread -- building and registering that graph is a separate,
//! substantial demonstration of its own (see `human_in_the_loop_gate.rs` and
//! `war_engine_configuration.rs`). This program's one registered assistant is
//! `Agent`-kind (`Runnable::Agent`), whose run path calls `PaladinPort::execute`
//! directly and never touches the waypoint store. So those routes are called here
//! against a thread with no Waypoint: they are proven **reachable and correctly
//! wired** (their real status codes are printed), not **populated** -- printed and
//! recorded as a scope deviation in the plan's own SUMMARY, never silently dropped.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use paladin::MockLlmAdapter;
use paladin::application::services::assistant::{
    AssistantService, AssistantValidator, ChainedResolver, StoredAssistantResolver,
};
use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::application::services::parley::{GraphRegistry, ParleyPortAdapter};
use paladin::application::services::run::schedule::{ScheduleService, ScheduleServiceOptions};
use paladin::application::services::run::webhook::{
    SsrfGuard, WebhookDeliveryOptions, WebhookDeliveryService,
};
use paladin::application::services::run::{
    AssistantResolver, RunEventBus, RunEventStreamService, RunSubmissionService, RunWorkerOptions,
    RunWorkerPool,
};
use paladin::config::run_store::RunStoreConfig;
use paladin::infrastructure::adapters::auth::InMemoryTokenAuthAdapter;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin::infrastructure::web::app::create_dev_ui_router;
use paladin::infrastructure::web::dev_ui_controller::DevUiState;
use paladin::infrastructure::web::run_api_wiring::{CodeAgentResolver, ErasedWaypointStore};
use paladin::infrastructure::web::{
    AgentApiState, AgentAuthConfig, AgentRegistry, HttpLayersConfig, Principal, RunApiState,
    ThreadApiState, agent_router, run_router, thread_router, with_http_layers,
};
use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::registries::EngineRegistries;
use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_core::platform::container::execution_result::PaladinResult;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::user::UserRole;
use paladin_ports::input::assistant_admin_port::AssistantAdminPort;
use paladin_ports::input::parley_port::ParleyPort;
use paladin_ports::input::run_event_stream_port::RunEventStreamPort;
use paladin_ports::input::run_submission_port::RunSubmissionPort;
use paladin_ports::input::schedule_admin_port::ScheduleAdminPort;
use paladin_ports::output::assistant_repository_port::AssistantRepositoryPort;
use paladin_ports::output::auth_port::AuthPort;
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::paladin_executor_port::PaladinExecutorPort;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::run_queue_port::RunQueuePort;
use paladin_ports::output::run_repository_port::RunRepositoryPort;
use paladin_ports::output::run_schedule_repository_port::RunScheduleRepositoryPort;
use paladin_ports::output::streaming_executor_port::StreamingExecutorPort;
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort;

const API_KEY: &str = "sk-example-admin";

/// Adapts [`PaladinExecutionService`] (backed by [`MockLlmAdapter`]) to the
/// engine-facing [`PaladinPort`] seam `WarEngine`/`RunWorkerPool` expect -- the offline
/// substitute for `paladin_port_from_settings`
/// (`src/infrastructure/web/facade_provisioner.rs`), which resolves a REAL provider
/// credential from the environment and so cannot run hermetically.
struct MockEnginePort(Arc<PaladinExecutionService>);

#[async_trait]
impl PaladinPort for MockEnginePort {
    async fn execute(&self, paladin: &Paladin, input: &str) -> Result<PaladinResult, PaladinError> {
        self.0.execute(paladin, input).await
    }

    async fn execute_stream(
        &self,
        paladin: &Paladin,
        input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        self.0.execute_stream(paladin, input).await
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // -- Agent registry: one mock-backed, code-registered assistant (D-32) ---------------
    let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new());
    let breaker = Arc::new(CircuitBreaker::new(5, 2, Duration::from_secs(30)));
    let paladin_service = Arc::new(PaladinExecutionService::new(
        Arc::clone(&llm),
        breaker,
        None,
        None,
    ));
    let paladin = PaladinBuilder::new(llm)
        .name("researcher")
        .system_prompt("You research topics thoroughly.")
        .model("mock")
        .build()
        .await?;
    let code_registry = Arc::new(AgentRegistry::new());
    let executor: Arc<dyn PaladinExecutorPort> = paladin_service.clone();
    let streamer: Arc<dyn StreamingExecutorPort> = paladin_service.clone();
    code_registry.insert_with_streaming("researcher", Arc::new(paladin), executor, Some(streamer));

    // -- Auth: one admin API key for the agent/thread/run routers, one bearer token from
    // a SEPARATE AuthPort seam for the dev-ui route (EX-104) -----------------------------
    let mut api_keys = HashMap::new();
    api_keys.insert(
        API_KEY.to_string(),
        Principal {
            id: "example".to_string(),
            role: UserRole::Admin,
        },
    );
    let auth = AgentAuthConfig {
        enabled: true,
        api_keys,
        token_verifier: None,
    };
    let auth_port: Arc<dyn AuthPort> = Arc::new(InMemoryTokenAuthAdapter::new());
    let admin_token = auth_port
        .issue_token(Uuid::new_v4(), UserRole::Admin)
        .await?
        .token;

    // -- Platform API stores: every one an in-process, in-memory adapter (D-16, EX-98/99) --
    let waypoint_store: Arc<dyn WaypointPort> =
        Arc::new(paladin_storage::waypoint::in_memory::InMemoryWaypointStore::new());
    let run_repository: Arc<dyn RunRepositoryPort> =
        Arc::new(paladin_storage::run::in_memory::InMemoryRunRepository::new());
    let assistant_repository: Arc<dyn AssistantRepositoryPort> =
        Arc::new(paladin_storage::assistant::in_memory::InMemoryAssistantRepository::new());
    let schedule_repository: Arc<dyn RunScheduleRepositoryPort> =
        Arc::new(paladin_storage::run_schedule::in_memory::InMemoryRunScheduleRepository::new());
    let webhook_repository: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(paladin_storage::webhook::in_memory::InMemoryWebhookDeliveryRepository::new());
    let run_queue: Arc<dyn RunQueuePort> =
        Arc::new(paladin_storage::run_queue::in_memory::InMemoryRunQueue::new());
    println!(
        "run queue: in-memory (paladin_storage::run_queue::in_memory::InMemoryRunQueue is in force) -- EX-98"
    );
    println!(
        "run store: this example wires paladin_storage::run::in_memory::InMemoryRunRepository \
         directly, fully in-process. A deployment's config-driven selector \
         (RunStoreConfig::default().backend) defaults to {:?}; its durable alternatives are \
         RunStoreBackend::Sqlite {{ path }} and RunStoreBackend::Postgres {{ url_env }} -- EX-99",
        RunStoreConfig::default().backend
    );

    // -- Resolvers: stored assistants first, code-registered agents second (D-32) --------
    let engine_registries = Arc::new(EngineRegistries::new());
    let validator = Arc::new(AssistantValidator::new(Arc::clone(&engine_registries)));
    let stored_resolver: Arc<dyn AssistantResolver> = Arc::new(StoredAssistantResolver::new(
        Arc::clone(&assistant_repository),
        Arc::clone(&validator),
    ));
    let code_resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeAgentResolver::new(Arc::clone(&code_registry)));
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(ChainedResolver::new(stored_resolver, code_resolver));

    // -- The run engine's PaladinPort: mock-backed, never `paladin_port_from_settings` ----
    let engine_port: Arc<dyn PaladinPort> = Arc::new(MockEnginePort(Arc::clone(&paladin_service)));

    let erased_store = Arc::new(ErasedWaypointStore::new(Arc::clone(&waypoint_store)));
    let event_bus = Arc::new(RunEventBus::new());
    let shutdown_coordinator = ShutdownCoordinator::new();

    let engine_factory: Arc<
        dyn Fn(CancellationToken) -> WarEngine<ErasedWaypointStore> + Send + Sync,
    > = {
        let engine_port = Arc::clone(&engine_port);
        let store = Arc::clone(&erased_store);
        Arc::new(move |token: CancellationToken| {
            WarEngine::new(Arc::clone(&engine_port), Arc::clone(&store))
                .with_cancellation_token(token)
        })
    };
    let base_engine = Arc::new(WarEngine::new(
        Arc::clone(&engine_port),
        Arc::clone(&erased_store),
    ));

    let pool = Arc::new(
        RunWorkerPool::new(
            base_engine,
            Arc::clone(&erased_store),
            Arc::clone(&run_repository),
            Arc::clone(&run_queue),
            Arc::clone(&resolver),
            Duration::from_secs(5),
        )
        .with_shutdown_coordinator(shutdown_coordinator.clone())
        .with_paladin_port(Arc::clone(&engine_port))
        .with_engine_factory(engine_factory)
        .with_cancellation_probing(Duration::from_millis(20))
        .with_event_bus(Arc::clone(&event_bus))
        .with_webhook_deliveries(Arc::clone(&webhook_repository)),
    );
    let _worker_tasks = Arc::clone(&pool).spawn(RunWorkerOptions {
        concurrency: 2,
        lease: Duration::from_secs(5),
        min_probe_interval: Duration::from_millis(20),
    });

    let submission_service = RunSubmissionService::new(
        Arc::clone(&run_repository),
        Arc::clone(&run_queue),
        Arc::clone(&resolver),
    )
    .with_local_tokens(pool.local_tokens())
    .with_waypoints(Arc::clone(&waypoint_store))
    .with_ssrf_guard(SsrfGuard::new(false));
    let run_submission: Arc<dyn RunSubmissionPort> = Arc::new(submission_service);

    let schedule_service = Arc::new(
        ScheduleService::new(
            Arc::clone(&schedule_repository),
            Arc::clone(&run_submission),
            ScheduleServiceOptions::default(),
        )
        .with_resolver(Arc::clone(&resolver))
        .with_ssrf_guard(SsrfGuard::new(false)),
    );
    let _schedule_task = Arc::clone(&schedule_service).spawn(&shutdown_coordinator);
    let schedules_port: Arc<dyn ScheduleAdminPort> = schedule_service;

    let webhook_service = WebhookDeliveryService::new(
        Arc::clone(&webhook_repository),
        Arc::clone(&run_repository),
        WebhookDeliveryOptions::default(),
    )?;
    let _webhook_task = Arc::new(webhook_service).spawn(&shutdown_coordinator);

    let assistant_service: Arc<dyn AssistantAdminPort> = Arc::new(AssistantService::new(
        Arc::clone(&assistant_repository),
        Arc::clone(&validator),
    ));

    let run_events: Arc<dyn RunEventStreamPort> = Arc::new(RunEventStreamService::new(
        Arc::clone(&event_bus),
        Arc::clone(&run_repository),
        Arc::clone(&waypoint_store),
        Duration::from_millis(50),
    ));

    let run_state = RunApiState::new()
        .with_submission(Arc::clone(&run_submission))
        .with_repository(Arc::clone(&run_repository))
        .with_run_events(run_events)
        .with_assistants(Arc::clone(&assistant_service))
        .with_code_registry(Arc::clone(&code_registry))
        .with_schedules(schedules_port)
        .with_webhook_deliveries(Arc::clone(&webhook_repository))
        .with_auth(auth.clone());

    // -- Thread state: the SAME waypoint store, wired the same way EX-33's fix wires it ---
    let thread_engine = Arc::new(WarEngine::new(
        Arc::clone(&engine_port),
        Arc::clone(&erased_store),
    ));
    // Deliberately empty: this process registers no `WarGraph`s (ADR-0039).
    let graph_registry = Arc::new(GraphRegistry::new());
    let parley_adapter = ParleyPortAdapter::new(
        thread_engine,
        Arc::clone(&erased_store),
        graph_registry,
        shutdown_coordinator.clone(),
    )
    .with_run_repository(Arc::clone(&run_repository))
    .with_run_queue(Arc::clone(&run_queue));
    let parley: Arc<dyn ParleyPort> = Arc::new(parley_adapter);

    let thread_state = ThreadApiState::new()
        .with_waypoints(Arc::clone(&waypoint_store))
        .with_parley(parley)
        .with_runs(Arc::clone(&run_repository))
        .with_run_submission(Arc::clone(&run_submission))
        .with_auth(auth.clone());

    let agent_state = AgentApiState::new(Arc::clone(&code_registry)).with_auth(auth.clone());

    // -- dev-ui router (EX-104): admin-gated AND feature-gated, unwired here (no
    // RunInspectorPort) -- so it answers 501, reachable and correctly gated.
    let dev_ui_state =
        DevUiState::new("https://cdn.jsdelivr.net/npm/mermaid/dist/mermaid.esm.min.mjs");
    let dev_ui_routes = create_dev_ui_router(Arc::clone(&auth_port), dev_ui_state);

    let routes = agent_router(agent_state)
        .merge(thread_router(thread_state))
        .merge(run_router(run_state))
        .merge(dev_ui_routes);
    let app = with_http_layers(routes, &HttpLayersConfig::default());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .await;
    });

    let base = format!("http://{addr}");
    let client = reqwest::Client::new();
    println!("platform_api_client (example) listening on {base}");

    // 1. Submit a run (EX-91).
    let submit: serde_json::Value = client
        .post(format!("{base}/v1/runs"))
        .header("x-api-key", API_KEY)
        .json(&serde_json::json!({ "assistant_id": "researcher", "input": { "input": "Tell me about Rust." } }))
        .send()
        .await?
        .json()
        .await?;
    let run_id = submit["run_id"].as_str().unwrap_or_default().to_string();
    println!("POST /v1/runs -> run_id {run_id}");

    // 2. Stream the run (EX-92): print each wire event name, so the frozen event set is
    // visible. The stream closes once the run reaches a terminal status.
    let stream_body = client
        .get(format!("{base}/v1/runs/{run_id}/stream"))
        .header("x-api-key", API_KEY)
        .header("accept", "text/event-stream")
        .timeout(Duration::from_secs(10))
        .send()
        .await?
        .text()
        .await?;
    let event_names: Vec<&str> = stream_body
        .lines()
        .filter_map(|line| line.strip_prefix("event: "))
        .collect();
    println!("GET /v1/runs/{run_id}/stream -> event(s): {event_names:?}");

    // 3. Cancel a run (EX-93): submit a second run and cancel it immediately.
    let submit2: serde_json::Value = client
        .post(format!("{base}/v1/runs"))
        .header("x-api-key", API_KEY)
        .json(&serde_json::json!({ "assistant_id": "researcher", "input": { "input": "Second run." } }))
        .send()
        .await?
        .json()
        .await?;
    let run_id2 = submit2["run_id"].as_str().unwrap_or_default().to_string();
    let cancel_response = client
        .post(format!("{base}/v1/runs/{run_id2}/cancel"))
        .header("x-api-key", API_KEY)
        .send()
        .await?;
    let cancel_status = cancel_response.status();
    let cancel_body: serde_json::Value = cancel_response
        .json()
        .await
        .unwrap_or(serde_json::Value::Null);
    println!("POST /v1/runs/{run_id2}/cancel -> {cancel_status} {cancel_body}");

    // 4. Assistants (EX-94): create a stored assistant, publish a second version, list
    // its versions, print the version identifiers.
    let create_assistant: serde_json::Value = client
        .post(format!("{base}/v1/assistants"))
        .header("x-api-key", API_KEY)
        .json(&serde_json::json!({
            "assistant_id": "greeter",
            "definition": { "kind": "agent", "body": { "name": "Greeter", "model": "mock", "system_prompt": "You greet visitors." } },
            "note": "v1"
        }))
        .send()
        .await?
        .json()
        .await?;
    println!(
        "POST /v1/assistants -> assistant_id {} version {}",
        create_assistant["assistant_id"], create_assistant["version"]
    );

    let create_version: serde_json::Value = client
        .post(format!("{base}/v1/assistants/greeter/versions"))
        .header("x-api-key", API_KEY)
        .json(&serde_json::json!({
            "definition": { "kind": "agent", "body": { "name": "Greeter", "model": "mock", "system_prompt": "You greet visitors warmly." } },
            "note": "v2"
        }))
        .send()
        .await?
        .json()
        .await?;
    println!(
        "POST /v1/assistants/greeter/versions -> version {}",
        create_version["version"]
    );

    let versions: serde_json::Value = client
        .get(format!("{base}/v1/assistants/greeter/versions"))
        .header("x-api-key", API_KEY)
        .send()
        .await?
        .json()
        .await?;
    let version_ids: Vec<&serde_json::Value> = versions["items"]
        .as_array()
        .map(|items| items.iter().map(|item| &item["version"]).collect())
        .unwrap_or_default();
    println!("GET /v1/assistants/greeter/versions -> version ids {version_ids:?}");

    // 5. Schedules (EX-95): create a cron-driven schedule (disabled, so this short-lived
    // program never fires it), list, print what it would submit.
    let create_schedule: serde_json::Value = client
        .post(format!("{base}/v1/schedules"))
        .header("x-api-key", API_KEY)
        .json(&serde_json::json!({
            "assistant_id": "researcher",
            "cron": "0 * * * *",
            "enabled": false,
            "input": { "input": "Hourly research digest." }
        }))
        .send()
        .await?
        .json()
        .await?;
    println!(
        "POST /v1/schedules -> schedule_id {} would submit assistant_id {} on cron {} (enabled: {})",
        create_schedule["schedule_id"],
        create_schedule["assistant_id"],
        create_schedule["cron"],
        create_schedule["enabled"]
    );

    let schedules: serde_json::Value = client
        .get(format!("{base}/v1/schedules"))
        .header("x-api-key", API_KEY)
        .send()
        .await?
        .json()
        .await?;
    let schedule_count = schedules["items"]
        .as_array()
        .map(|items| items.len())
        .unwrap_or(0);
    println!("GET /v1/schedules -> {schedule_count} schedule(s)");

    // 6. Thread state, resume and history (EX-77, EX-78, EX-79) -- see this file's own
    // header for why "demo-thread" carries no Waypoint in this offline program.
    let thread_state_status = client
        .get(format!("{base}/v1/threads/demo-thread/state"))
        .header("x-api-key", API_KEY)
        .send()
        .await?
        .status();
    println!(
        "GET /v1/threads/demo-thread/state -> {thread_state_status} (no Waypoint exists -- \
         this example's one assistant is Agent-kind, whose run path never touches the \
         waypoint store; a Workflow-kind assistant is required for a populated demo)"
    );

    let resume_status = client
        .post(format!("{base}/v1/threads/demo-thread/resume"))
        .header("x-api-key", API_KEY)
        .json(&serde_json::json!({ "responses": [] }))
        .send()
        .await?
        .status();
    println!(
        "POST /v1/threads/demo-thread/resume -> {resume_status} (same reason -- no suspended \
         parley to resume; see human_in_the_loop_gate.rs for a live Workflow-based demo)"
    );

    let history_status = client
        .get(format!("{base}/v1/threads/demo-thread/history"))
        .header("x-api-key", API_KEY)
        .send()
        .await?
        .status();
    println!("GET /v1/threads/demo-thread/history -> {history_status} (empty thread)");

    // 7. Dev-ui inspector route (EX-104): admin-gated (bearer token, a SEPARATE AuthPort
    // seam from the x-api-key auth above) AND feature-gated (`dev-ui`, compiled in via
    // this program's own required-features); unwired here (no RunInspectorPort), so it
    // answers 501 -- reachable and correctly gated, not populated.
    let dev_ui_status = client
        .get(format!("{base}/v1/dev-ui/threads/demo-thread"))
        .header("authorization", format!("Bearer {admin_token}"))
        .send()
        .await?
        .status();
    println!(
        "GET /v1/dev-ui/threads/demo-thread -> {dev_ui_status} (admin-gated AND feature-gated: \
         dev-ui feature compiled in, admin bearer token required, no RunInspectorPort wired)"
    );

    // 8. Token usage (EX-110): the prompt/completion split, never a bare total beside it.
    let execute: serde_json::Value = client
        .post(format!("{base}/v1/agents/researcher/execute"))
        .header("x-api-key", API_KEY)
        .json(&serde_json::json!({ "input": "Summarize token usage." }))
        .send()
        .await?
        .json()
        .await?;
    println!(
        "POST /v1/agents/researcher/execute -> usage.prompt_tokens={} usage.completion_tokens={}",
        execute["usage"]["prompt_tokens"], execute["usage"]["completion_tokens"]
    );

    // 9. Queue and store selection (EX-98, EX-99) -- printed above, at wiring time.

    // Shut down: cancel every background task's shared coordinator, then the HTTP server.
    shutdown_coordinator.token().cancel();
    let _ = shutdown_tx.send(());
    let _ = server.await;
    Ok(())
}
