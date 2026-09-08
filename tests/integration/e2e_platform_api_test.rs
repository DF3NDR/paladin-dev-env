//! PRD 06 (`.project/v0.10.0/06-platform-api.md`) acceptance criterion 1, driven end to end
//! over the REAL Platform API router (27-18 Task 1): create a workflow assistant with a Gate
//! -> submit a run -> SSE shows progress -> the run suspends `AwaitingInput` -> a mockito
//! webhook receives the `awaiting_input` payload with the parley -> resume via
//! `POST /threads/{id}/resume` -> the run completes -> `GET /threads/{id}/history` shows the
//! chain and `POST /threads/{id}/fork` submits a new run that also reaches a terminal status.
//!
//! Setup mirrors `src/bin/paladin-server.rs`'s own wiring: `build_run_api` over a temp-file
//! SQLite run store + an in-memory queue + webhooks with `allow_private = true` (mockito binds
//! `127.0.0.1` -- the one legitimate use of that override in this test suite), merged
//! `agent_router().merge(thread_router()).merge(run_router())` behind a test API key. No
//! Docker: every collaborator is either a temp-file SQLite database or a local mockito server.
//!
//! ## Deviation: this test does not publish
//! `crates/paladin-battalion/tests/fixtures/graph_docs/approval_gate.json` verbatim
//!
//! That fixture's own edges (`review -> writer` on a bare `"false"` contains-match, `review ->
//! review` on a bare `"true"` contains-match) do not reach `Completed` on approval: submitting
//! `true` re-enters the SAME gate node and re-suspends indefinitely rather than completing --
//! confirmed empirically against a real `WarEngine` before writing this test (see
//! `27-18-SUMMARY.md`'s Deviations section for the reproduction). This test defines its own,
//! structurally identical Workflow body (one Paladin `writer` node feeding a `review` Gate,
//! same `parley: "approval"` shape) but with field-qualified `contains` conditions
//! (`"approved":true` / `"approved":false`, mirroring
//! `tests/integration/e2e_approval_gate_test.rs`'s own hand-built graph) and two terminal
//! Paladin leaves (`act`/`cancel`, no outgoing edges), so an approval genuinely reaches
//! `RunOutcome::Completed` as PRD 06 acceptance 1 requires.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use tower::ServiceExt;

use paladin::application::services::parley::ParleyPortAdapter;
use paladin::application::services::parley::adapter::GraphResolver;
use paladin::application::services::parley::registry::GraphRegistry;
use paladin::application::services::run::webhook::{WEBHOOK_SIGNATURE_HEADER, sign_webhook_body};
use paladin::config::assistants::AssistantsConfig;
use paladin::config::run_queue::RunQueueConfig;
use paladin::config::run_store::{RunStoreBackend, RunStoreConfig};
use paladin::config::run_stream::RunStreamConfig;
use paladin::config::run_worker::RunWorkerConfig;
use paladin::config::schedules::SchedulesConfig;
use paladin::config::settings::Settings;
use paladin::config::webhooks::WebhooksConfig;
use paladin::infrastructure::web::run_api_wiring::{
    ErasedWaypointStore, RunApiConfigs, build_run_api,
};
use paladin::infrastructure::web::{
    AgentApiState, AgentAuthConfig, AgentRegistry, Principal, ThreadApiState, agent_router,
    run_router, thread_router,
};
use paladin_battalion::engine::WarEngine;
use paladin_battalion::engine::graph_doc::WarGraphDoc;
use paladin_battalion::engine::registries::EngineRegistries;
use paladin_battalion::engine::shutdown::ShutdownCoordinator;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::user::UserRole;
use paladin_ports::output::paladin_port::{PaladinPort, PaladinResult, PaladinStream};
use paladin_ports::output::waypoint_port::WaypointPort;

/// This process's thread-surface `WarEngine` is constructed but never actually dispatches a
/// `NodeSpec::Paladin` node in this test: `resume_thread` always resolves the durable
/// `parley_extras` re-enqueue path (`ParleyPortAdapter::resume_with`'s `ShadowOutcome::Complete`
/// branch) because every response this test submits answers the ONE outstanding parley
/// completely -- mirrors `src/bin/paladin-server.rs`'s own `NoRegisteredGraphsPaladinPort`
/// precedent for the identical "constructed but unreachable" situation.
struct UnreachableInThisTestPaladinPort;

#[async_trait::async_trait]
impl PaladinPort for UnreachableInThisTestPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!(
            "this test's thread-surface WarEngine is never dispatched: every resume answers \
             the outstanding parley completely, so ParleyPortAdapter::resume_with always takes \
             the durable run_repository/run_queue re-enqueue path"
        )
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("see execute's doc comment")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

fn temp_sqlite_url(label: &str) -> (std::path::PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "e2e_platform_api_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    let url = format!("sqlite://{}", path.display());
    (path, url)
}

fn cleanup_sqlite(path: &std::path::PathBuf) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

/// The Workflow body this test publishes: `writer` (a Paladin node) feeds `review` (a Gate,
/// `parley: "approval"`); `review` routes to the terminal `act` Paladin leaf on
/// `"approved":true` and to the terminal `cancel` Paladin leaf on `"approved":false` -- see
/// this file's own module docs for why the field-qualified `contains` conditions (not the
/// `approval_gate.json` fixture's bare `"true"`/`"false"`) are required to reach `Completed`.
fn workflow_body() -> Value {
    json!({
        "schema_version": "1",
        "entry": ["writer"],
        "nodes": [
            {
                "id": "writer",
                "kind": "paladin",
                "paladin": {
                    "name": "Writer",
                    "model": "gpt-4",
                    "system_prompt": "Draft a short reply about {topic}.",
                    "temperature": 0.7,
                    "max_loops": 1,
                    "stop_words": [],
                    "input_template": "{topic}",
                    "output_field": "draft"
                },
                "defer": false
            },
            {
                "id": "review",
                "kind": "gate",
                "gate": {
                    "parley": "approval",
                    "prompt_template": "Approve this draft? {draft}",
                    "on_expire": { "type": "fail_run" },
                    "output_field": "approved"
                },
                "defer": false
            },
            {
                "id": "act",
                "kind": "paladin",
                "paladin": {
                    "name": "Actor",
                    "model": "gpt-4",
                    "system_prompt": "Act on the approved draft.",
                    "temperature": 0.7,
                    "max_loops": 1,
                    "stop_words": [],
                    "input_template": "act",
                    "output_field": "path"
                },
                "defer": false
            },
            {
                "id": "cancel",
                "kind": "paladin",
                "paladin": {
                    "name": "Canceller",
                    "model": "gpt-4",
                    "system_prompt": "Record the cancellation.",
                    "temperature": 0.7,
                    "max_loops": 1,
                    "stop_words": [],
                    "input_template": "cancel",
                    "output_field": "path"
                },
                "defer": false
            }
        ],
        "edges": [
            { "from": "writer", "to": "review" },
            { "from": "review", "to": "act", "condition": { "type": "contains", "value": "\"approved\":true" } },
            { "from": "review", "to": "cancel", "condition": { "type": "contains", "value": "\"approved\":false" } }
        ],
        "schema": {
            "fields": [
                { "name": "topic", "kind": "string", "reducer": "last_write", "default": "widgets", "required": false },
                { "name": "draft", "kind": "string", "reducer": "last_write", "required": false },
                { "name": "approved", "kind": "boolean", "reducer": "last_write", "default": false, "required": false },
                { "name": "path", "kind": "string", "reducer": "last_write", "required": false }
            ]
        },
        "limits": { "max_supersteps": 50, "max_node_visits": 25, "run_timeout_secs": null, "max_muster_tasks": 100 }
    })
}

/// One captured webhook delivery attempt: the exact raw body bytes mockito received, plus the
/// `X-Paladin-Signature` header value observed alongside it.
type Captured = Arc<Mutex<Vec<(Vec<u8>, String)>>>;

const API_KEY: &str = "e2e-platform-api-test-key";

fn auth_config() -> AgentAuthConfig {
    let mut api_keys = std::collections::HashMap::new();
    api_keys.insert(
        API_KEY.to_string(),
        Principal {
            id: "e2e-tester".to_string(),
            role: UserRole::Admin,
        },
    );
    AgentAuthConfig {
        enabled: true,
        api_keys,
        token_verifier: None,
    }
}

fn json_request(method: &str, path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .header("X-API-Key", API_KEY)
        .body(Body::from(body.to_string()))
        .expect("valid request")
}

fn get_request(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .header("X-API-Key", API_KEY)
        .body(Body::empty())
        .expect("valid request")
}

async fn send(app: &Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("router call succeeds");
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read response body");
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or_else(|e| {
            panic!(
                "response body was not JSON ({e}): {}",
                String::from_utf8_lossy(&bytes)
            )
        })
    };
    (status, body)
}

/// Reads the WHOLE SSE response body (never a partial prefix): the live bus unbinds shortly
/// after this test's single `AwaitingInput` outcome (`TRACE_DRAIN_GRACE_PERIOD`, 27-10), which
/// closes the broadcast channel and ends the stream -- so a full drain via `axum::body::to_bytes`
/// deterministically captures every event up to and including `parley`, mirroring
/// `run_controller.rs`'s own `run_stream_sse` unit test convention rather than adding a new
/// `http-body-util` dependency for incremental frame reads this test does not need.
async fn read_full_sse_body(app: &Router, path: &str) -> String {
    use futures::StreamExt;

    let response = app
        .clone()
        .oneshot(get_request(path))
        .await
        .expect("router call succeeds");
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "SSE stream must open 200"
    );
    // Read incrementally (`axum::body::Body::into_data_stream`, no new dependency needed)
    // and stop as soon as a `parley` event has been observed, rather than draining the whole
    // body: the degraded polling path (D-26) keeps the stream open, re-polling every
    // `poll_interval_ms`, until the run reaches a TERMINAL status -- `AwaitingInput` never
    // closes it, so waiting for the body to end here would hang until this test's own later
    // resume step, deadlocking against itself.
    let mut stream = response.into_body().into_data_stream();
    let mut buffer = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        if buffer.contains("event: parley") {
            return buffer;
        }
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            panic!("SSE stream did not produce a parley event within 20s: {buffer}");
        }
        match tokio::time::timeout(remaining, stream.next()).await {
            Ok(Some(Ok(chunk))) => buffer.push_str(&String::from_utf8_lossy(&chunk)),
            Ok(Some(Err(e))) => panic!("SSE stream error: {e}"),
            Ok(None) => return buffer,
            Err(_) => panic!("SSE stream read timed out waiting for parley: {buffer}"),
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn e2e_platform_api_acceptance_1_full_lifecycle() {
    tokio::time::timeout(Duration::from_secs(60), run_lifecycle())
        .await
        .expect("acceptance-1 lifecycle must complete within its 60s timeout guard");
}

async fn run_lifecycle() {
    // --- Mock the LLM: every `NodeSpec::Paladin` node in `workflow_body()` calls the real
    // `paladin_port_from_settings` provider (D-44's "the run pipeline uses a real port"), so
    // `OPENAI_BASE_URL` is redirected at a local mockito server rather than the real OpenAI
    // API -- `OPENAI_API_KEY` only needs to be PRESENT (construction never validates it), the
    // exact same hermetic pattern `run_api_wiring.rs`'s own
    // `sqlite_and_in_memory_wires_three_tasks_and_every_state_field` test uses.
    let mut llm_server = mockito::Server::new_async().await;
    llm_server
        .mock("POST", "/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            json!({
                "id": "chatcmpl-e2e-platform-api",
                "model": "gpt-4",
                "choices": [{
                    "index": 0,
                    "message": { "role": "assistant", "content": "draft text" },
                    "finish_reason": "stop"
                }],
                "usage": { "prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2 }
            })
            .to_string(),
        )
        .expect_at_least(1)
        .create_async()
        .await;
    // SAFETY: this test binary runs this test in isolation (the ONLY `#[tokio::test]` in this
    // file setting these two variables) -- see the module docs; no concurrent test in this
    // process observes a different value mid-run.
    unsafe {
        std::env::set_var("OPENAI_API_KEY", "sk-e2e-platform-api-test-hermetic");
        std::env::set_var("OPENAI_BASE_URL", llm_server.url());
    }

    // --- Mock the webhook receiver (mockito binds 127.0.0.1 -- the one legitimate
    // `allow_private = true` use in this test suite, per this file's own module docs).
    let mut hook_server = mockito::Server::new_async().await;
    let captured: Captured = Arc::new(Mutex::new(Vec::new()));
    {
        let captured = Arc::clone(&captured);
        hook_server
            .mock("POST", "/hook")
            .with_status_code_from_request(move |req| {
                let body = req.body().cloned().unwrap_or_default();
                let signature = req
                    .header(WEBHOOK_SIGNATURE_HEADER)
                    .first()
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or_default()
                    .to_string();
                captured.lock().unwrap().push((body, signature));
                200
            })
            .expect_at_least(1)
            .create_async()
            .await;
    }
    let webhook_secret = "e2e-platform-api-webhook-secret";
    let webhook_url = format!("{}/hook", hook_server.url());

    // --- Build the run API exactly as `src/bin/paladin-server.rs::run()` does.
    let (run_store_path, run_store_url) = temp_sqlite_url("run_store");
    let (waypoint_path, waypoint_url) = temp_sqlite_url("waypoints");
    let waypoint_store: Arc<dyn WaypointPort> = Arc::new(
        paladin_storage::waypoint::sqlite::SqliteWaypointStore::new(&waypoint_url)
            .await
            .expect("waypoint store opens"),
    );

    let configs = RunApiConfigs {
        run_store: RunStoreConfig {
            backend: RunStoreBackend::Sqlite {
                path: run_store_url,
            },
        },
        run_queue: RunQueueConfig::default(),
        run_worker: RunWorkerConfig::default(),
        run_stream: RunStreamConfig::default(),
        assistants: AssistantsConfig::default(),
        schedules: SchedulesConfig::default(),
        webhooks: WebhooksConfig {
            allow_private: true,
            ..WebhooksConfig::default()
        },
    };

    let auth = auth_config();
    let coordinator = ShutdownCoordinator::new();
    let code_registry = Arc::new(AgentRegistry::new());

    let handles = build_run_api(
        configs,
        &Settings::default(),
        coordinator.clone(),
        Some(Arc::clone(&waypoint_store)),
        auth.clone(),
        Arc::clone(&code_registry),
    )
    .await
    .expect("build_run_api wires successfully over sqlite + in-memory");

    // --- Thread surface: a `GraphRegistry` carrying the SAME workflow graph this test
    // publishes below, compiled the SAME way `AssistantValidator` compiles it
    // (`WarGraphDoc::compile(&EngineRegistries::new())`) -- `WarGraph::fingerprint()` is a
    // deterministic content hash (proven in `paladin-battalion`'s own
    // `fingerprint_is_deterministic_across_calls`/`wargraph_doc_fingerprint_two_process`
    // tests), so this registration resolves the persisted Waypoint's own
    // `graph_fingerprint` on `POST /threads/{id}/resume` without this test needing to share
    // any process state with the run engine that actually executed it.
    let doc: WarGraphDoc =
        serde_json::from_value(workflow_body()).expect("workflow body parses as a WarGraphDoc");
    let graph = doc
        .compile(&EngineRegistries::new())
        .expect("workflow body compiles");
    let graph_registry = GraphRegistry::new();
    graph_registry.register(graph);
    let erased = Arc::new(ErasedWaypointStore::new(Arc::clone(&waypoint_store)));
    let thread_engine = Arc::new(WarEngine::new(
        Arc::new(UnreachableInThisTestPaladinPort),
        Arc::clone(&erased),
    ));
    let mut adapter = ParleyPortAdapter::new(
        thread_engine,
        erased,
        Arc::new(graph_registry) as Arc<dyn GraphResolver>,
        coordinator.clone(),
    );
    if let Some((run_repository, run_queue)) = handles.parley_extras.clone() {
        adapter = adapter
            .with_run_repository(run_repository)
            .with_run_queue(run_queue);
    }
    let parley: Arc<dyn paladin_ports::input::parley_port::ParleyPort> = Arc::new(adapter);

    let mut thread_state = ThreadApiState::new()
        .with_waypoints(Arc::clone(&waypoint_store))
        .with_parley(parley)
        .with_auth(auth.clone());
    if let Some(run_repository) = handles.run_repository.clone() {
        thread_state = thread_state.with_runs(run_repository);
    }
    if let Some(run_submission) = handles.thread_run_submission.clone() {
        thread_state = thread_state.with_run_submission(run_submission);
    }

    let agent_state = AgentApiState::new(Arc::clone(&code_registry)).with_auth(auth.clone());

    let app = agent_router(agent_state)
        .merge(thread_router(thread_state))
        .merge(run_router(handles.run_state));
    let _run_tasks = handles.tasks;

    // --- (1) POST /v1/assistants -- publish the Workflow body.
    let (status, _body) = send(
        &app,
        json_request(
            "POST",
            "/v1/assistants",
            json!({
                "assistant_id": "approval-flow",
                "definition": { "kind": "workflow", "body": workflow_body() }
            }),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "assistant publish must succeed"
    );

    // --- (2) POST /v1/runs -- submit with a webhook subscribed to both events this test
    // observes.
    let (status, body) = send(
        &app,
        json_request(
            "POST",
            "/v1/runs",
            json!({
                "assistant_id": "approval-flow",
                "input": {},
                "webhook": {
                    "url": webhook_url,
                    "secret": webhook_secret,
                    "events": ["awaiting_input", "completed"]
                }
            }),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "run submission must be accepted: {body}"
    );
    let run_id = body["run_id"].as_str().expect("run_id present").to_string();
    let thread_id = body["thread_id"]
        .as_str()
        .expect("thread_id present")
        .to_string();

    // --- (3) GET /v1/runs/{id}/stream -- drain the whole live stream (it closes itself once
    // the worker unbinds after this run's single AwaitingInput outcome, 27-10's documented
    // grace period), then confirm at least one `superstep` was observed strictly before the
    // `parley` event.
    let sse_body = read_full_sse_body(&app, &format!("/v1/runs/{run_id}/stream")).await;
    let superstep_pos = sse_body.find("event: superstep");
    let parley_pos = sse_body
        .find("event: parley")
        .expect("a parley event must appear on the stream");
    if let Some(superstep_pos) = superstep_pos {
        assert!(
            superstep_pos < parley_pos,
            "a superstep event must precede the parley event: {sse_body}"
        );
    }

    // --- (4) GET /v1/runs/{id} -- awaiting_input (the stream only closes AFTER the worker's
    // own repository write, per worker.rs's documented unbind-after-write ordering, so this
    // read is never racing the write).
    let (status, body) = send(&app, get_request(&format!("/v1/runs/{run_id}"))).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body["status"], "awaiting_input",
        "run must be suspended: {body}"
    );

    // --- (5) The mockito webhook received exactly one `awaiting_input` delivery so far, with
    // a non-empty `parleys` list, `attempt == 1`, and a signature that verifies over the raw
    // captured body.
    let (awaiting_body_bytes, awaiting_signature) = wait_for_capture(&captured, 1).await;
    let awaiting_payload: Value =
        serde_json::from_slice(&awaiting_body_bytes).expect("webhook payload is JSON");
    assert_eq!(awaiting_payload["event"], "awaiting_input");
    assert_eq!(awaiting_payload["attempt"], 1);
    let parleys = awaiting_payload["parleys"]
        .as_array()
        .expect("parleys must be a non-empty array on the awaiting_input payload");
    assert!(
        !parleys.is_empty(),
        "parleys must be non-empty: {awaiting_payload}"
    );
    let parley_id = parleys[0]["parley_id"]
        .as_str()
        .expect("parley_id present")
        .to_string();
    let expected_signature = format!(
        "sha256={}",
        hex_hmac(webhook_secret.as_bytes(), &awaiting_body_bytes)
    );
    assert_eq!(
        awaiting_signature, expected_signature,
        "X-Paladin-Signature must verify over the raw captured body"
    );
    // Independently prove the same claim through this crate's own signing primitive, per
    // this plan's own action text.
    assert_eq!(
        awaiting_signature,
        sign_webhook_body(webhook_secret.as_bytes(), &awaiting_body_bytes)
    );

    // --- (6) POST /v1/threads/{id}/resume -- deliver the approval.
    let (status, body) = send(
        &app,
        json_request(
            "POST",
            &format!("/v1/threads/{thread_id}/resume"),
            json!({
                "responses": [{
                    "parley_id": parley_id,
                    "value": true,
                    "responded_by": "e2e-tester"
                }]
            }),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "resume must be accepted: {body}"
    );
    assert_eq!(
        body["run_id"], run_id,
        "resume must re-enqueue the SAME run under the durable parley_extras path"
    );

    // --- (7) Poll until done; the run completes and the completed webhook is delivered at
    // attempt == 2 (this run's second dispatch, after the resume re-enqueue).
    let final_body = poll_until_terminal(&app, &run_id).await;
    assert_eq!(
        final_body["status"], "completed",
        "run must complete: {final_body}"
    );

    let (completed_body_bytes, _completed_signature) = wait_for_capture(&captured, 2).await;
    let completed_payload: Value =
        serde_json::from_slice(&completed_body_bytes).expect("webhook payload is JSON");
    assert_eq!(completed_payload["event"], "completed");
    assert_eq!(completed_payload["attempt"], 2);

    let deliveries = wait_for_delivered_deliveries(&app, &run_id).await;
    assert_eq!(
        deliveries.len(),
        2,
        "both webhook deliveries must be recorded as delivered"
    );

    // --- (8) GET /v1/threads/{id}/history -- at least 3 waypoints, one AwaitingInput.
    let (status, history) = send(
        &app,
        get_request(&format!("/v1/threads/{thread_id}/history")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let items = history["items"].as_array().expect("history items array");
    assert!(
        items.len() >= 3,
        "history must record at least 3 waypoints: {history}"
    );
    assert!(
        items.iter().any(|item| item["status"] == "awaiting_input"),
        "history must record the AwaitingInput suspension: {history}"
    );

    // --- (9) POST /v1/threads/{id}/fork from the newest (per `history`'s own
    // newest-first ordering) waypoint -- the terminal one this run's own completion just
    // wrote -- and confirm the forked run also reaches a terminal status.
    let from_waypoint_id = items[0]["waypoint_id"]
        .as_str()
        .expect("waypoint_id present")
        .to_string();
    let (status, fork_body) = send(
        &app,
        json_request(
            "POST",
            &format!("/v1/threads/{thread_id}/fork"),
            json!({ "from_waypoint_id": from_waypoint_id, "edit": {} }),
        ),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::ACCEPTED,
        "fork must be accepted: {fork_body}"
    );
    let fork_run_id = fork_body["run_id"]
        .as_str()
        .expect("fork run_id present")
        .to_string();
    assert_ne!(fork_run_id, run_id, "fork must create a NEW run");

    let fork_final = poll_until_terminal(&app, &fork_run_id).await;
    assert!(
        ["completed", "failed", "halted", "cancelled"]
            .contains(&fork_final["status"].as_str().unwrap_or_default()),
        "the forked run must reach a terminal status: {fork_final}"
    );

    unsafe {
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("OPENAI_BASE_URL");
    }
    cleanup_sqlite(&run_store_path);
    cleanup_sqlite(&waypoint_path);
}

fn hex_hmac(key: &[u8], body: &[u8]) -> String {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("hmac accepts any key length");
    mac.update(body);
    mac.finalize()
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Poll the shared capture buffer until at least `count` webhook deliveries have landed
/// (the drain loop's default 5s poll interval means this can take a few seconds), returning
/// the `count`-th one. Never sleeps past the outer 60s test timeout in practice.
async fn wait_for_capture(captured: &Captured, count: usize) -> (Vec<u8>, String) {
    loop {
        {
            let guard = captured.lock().unwrap();
            if guard.len() >= count {
                return guard[count - 1].clone();
            }
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Poll `GET /v1/runs/{run_id}` until its status is terminal, returning the final body.
async fn poll_until_terminal(app: &Router, run_id: &str) -> Value {
    loop {
        let (status, body) = send(app, get_request(&format!("/v1/runs/{run_id}"))).await;
        assert_eq!(status, StatusCode::OK);
        if let Some(s) = body["status"].as_str()
            && ["completed", "failed", "halted", "cancelled"].contains(&s)
        {
            return body;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Poll `GET /v1/runs/{run_id}/webhook-deliveries` until both attempts are recorded
/// `delivered` (the send happens synchronously inside the drain loop's `run_once`, but this
/// still races this test's own read).
async fn wait_for_delivered_deliveries(app: &Router, run_id: &str) -> Vec<Value> {
    loop {
        let (status, body) = send(
            app,
            get_request(&format!("/v1/runs/{run_id}/webhook-deliveries")),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let items = body["items"].as_array().cloned().unwrap_or_default();
        let delivered: Vec<Value> = items
            .into_iter()
            .filter(|item| item["status"] == "delivered")
            .collect();
        if delivered.len() >= 2 {
            return delivered;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}
