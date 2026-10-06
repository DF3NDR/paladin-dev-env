//! The router-level ten-concurrent-submits race (PRD acceptance 3, D-52)
//! and the fork-from-waypoint end-to-end proof (D-45), both driven through
//! the real facade services -- 27-15 Task 2.
//!
//! `ten_concurrent_submits_one_accepted` is Tier 1 per D-51:
//! `SqliteRunRepository` over a real on-disk temp file (never `:memory:` --
//! the one-active-run-per-thread invariant is a database uniqueness
//! constraint, D-17, and proving it holds under real concurrent writers is
//! the whole point), no Docker. `fork_run_completes_from_waypoint` uses
//! InMemory adapters, mirroring `worker_tests.rs`'s own Tier 1 convention
//! for behavior that does not need to cross a real process boundary.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

use paladin_battalion::engine::{
    EdgeSpec, EngineLimits, NodeContext, NodeSpec, StateNode, StateNodeError, WarEngine, WarGraph,
};
use paladin_core::platform::container::battlefield::{BattlefieldSchema, StateDelta};
use paladin_core::platform::container::directive::Directive;
use paladin_core::platform::container::execution_result::PaladinResult;
use paladin_core::platform::container::herald::{
    BattalionResult, ExecutionMetadata, Herald, HeraldError, StreamChunk,
};
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::principal::{PrincipalRef, RunAttribution, TenantId};
use paladin_core::platform::container::run::{RunId, RunStatus};
use paladin_core::platform::container::trace::TraceEvent;
use paladin_core::platform::container::treasury_ledger::{LedgerScope, SettlementKey};
use paladin_core::platform::container::user::UserRole;
use paladin_core::platform::container::waypoint::NodeId;
use paladin_ports::input::run_submission_port::{ForkRun, RunSubmissionPort, SubmitRun};
use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
use paladin_ports::output::run_queue_port::RunQueuePort;
use paladin_ports::output::run_repository_port::{RunQuery, RunRepositoryPort};
use paladin_ports::output::run_trace_port::RunTracePort;
use paladin_ports::output::treasury_ledger_port::TreasuryLedgerPort;
use paladin_ports::output::treasury_notice_port::TreasuryNoticePort;
use paladin_ports::output::waypoint_port::WaypointPort;
use paladin_ports::output::webhook_delivery_port::WebhookDeliveryRepositoryPort;
use paladin_storage::run::in_memory::InMemoryRunRepository;
use paladin_storage::run::sqlite::SqliteRunRepository;
use paladin_storage::run_queue::in_memory::InMemoryRunQueue;
use paladin_storage::run_trace::in_memory::InMemoryRunTraceStore;
use paladin_storage::treasury::contract_tests::{settle_request, usd};
use paladin_storage::treasury::sqlite::SqliteTreasuryLedger;
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;
use paladin_storage::webhook::sqlite::SqliteWebhookDeliveryRepository;
use paladin_web::agent_auth::{AgentAuthConfig, Principal};
use paladin_web::run_controller::{RunApiState, run_router};

use super::resolver::{AssistantResolver, CodeWorkflowResolver};
use super::submission::RunSubmissionService;
use super::webhook::{SsrfGuard, WebhookDeliveryOptions, WebhookDeliveryService};
use super::worker::RunWorkerPool;
use crate::application::services::treasurer::{OperatorNoticeTarget, Treasurer, window_for};
use crate::config::trace::TraceConfig;
use crate::config::treasurer::TreasurerConfig;

/// A [`PaladinPort`] that must never be called -- every graph in this
/// module is `Function`-only, mirroring `worker_tests.rs`'s/
/// `cancel_tests.rs`'s own `UnusedPaladinPort` precedent (a small, local
/// double per module rather than a shared `pub(crate)` one).
struct UnusedPaladinPort;

#[async_trait]
impl PaladinPort for UnusedPaladinPort {
    async fn execute(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinResult, PaladinError> {
        unreachable!("this module's WarGraphs have no NodeSpec::Paladin nodes")
    }

    async fn execute_stream(
        &self,
        _paladin: &Paladin,
        _input: &str,
    ) -> Result<PaladinStream, PaladinError> {
        unreachable!("this module's WarGraphs have no NodeSpec::Paladin nodes")
    }

    fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
        Ok(())
    }
}

/// A trivial `StateNode` that advances with an empty delta -- one
/// superstep, no state, no delay. Mirrors `worker_tests.rs`'s
/// `DelayedCountingNode` with `delay: Duration::ZERO` and no counter.
struct NoopNode;

#[async_trait]
impl StateNode for NoopNode {
    async fn run(
        &self,
        _state: &paladin_core::platform::container::battlefield::Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        Ok(StateDelta::new().into())
    }
}

/// Build a linear `count`-node chain (`n0 -> n1 -> ... -> n{count-1}`) of
/// [`NoopNode`]s, one superstep per node, with an EMPTY Battlefield schema
/// (D-45's `fork_run_completes_from_waypoint` forks with `edit: None` --
/// `fork_edit_to_state_delta`'s own merge logic is already unit-tested in
/// `worker.rs` in isolation, so this integration test does not also need a
/// declared schema field to exercise it).
fn build_chain_graph(count: usize) -> Arc<WarGraph> {
    let mut graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
    let mut ids = Vec::with_capacity(count);
    for i in 0..count {
        let id = NodeId::new(format!("n{i}"));
        graph.add_node(id.clone(), NodeSpec::Function(Arc::new(NoopNode)));
        ids.push(id);
    }
    for pair in ids.windows(2) {
        graph.add_edge(EdgeSpec {
            from: pair[0].clone(),
            to: pair[1].clone(),
            condition: None,
        });
    }
    graph.add_entry(ids[0].clone());
    Arc::new(graph)
}

/// A fresh on-disk SQLite URL under the system temp dir, cleaned up by the
/// caller -- mirrors `cancel_tests.rs`'s own `temp_sqlite_url` helper
/// (module-local, not `pub`, so duplicated here rather than reached for
/// across a `#[cfg(test)]` module boundary).
fn temp_sqlite_url(label: &str) -> (std::path::PathBuf, String) {
    let path = std::env::temp_dir().join(format!(
        "paladin_http_surface_test_{label}_{}.sqlite",
        uuid::Uuid::new_v4()
    ));
    let url = format!("sqlite://{}", path.display());
    (path, url)
}

fn cleanup(path: &std::path::PathBuf) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}-wal", path.display()));
    let _ = std::fs::remove_file(format!("{}-shm", path.display()));
}

/// The Phase 41 admission tracer (ALLOW-01, ALLOW-02, D-01, D-02, D-04, D-05, D-06, D-12, D-13,
/// D-14): an operator-configured per-API-key rolling-window allowance travels
/// `TreasurerConfig` (config) -> `AllowancePolicy` -> `Treasurer` (facade service,
/// `AllowanceAdmissionPort`) -> `TreasuryLedgerPort::store_now` + `balance` (SQLite) ->
/// `RunSubmissionService::submit` refusing before any row -> `429 allowance_exhausted` with a
/// store-clock `Retry-After` -- driven through the real `run_router` over an on-disk SQLite
/// store shared by the run repository and the ledger (production shares one run-store file).
///
/// Pitfall 10: windows are epoch-aligned, so a UTC window boundary crossed between seeding and
/// asserting would make the test lie. The window is read from the store clock before seeding
/// and after the POST; when they differ the whole scenario is re-run once on a fresh store.
#[tokio::test(flavor = "multi_thread")]
async fn allowance_admission_tracer() {
    tokio::time::timeout(Duration::from_secs(30), async {
        for attempt in 1..=2 {
            if run_allowance_tracer_once().await {
                return;
            }
            eprintln!("allowance_admission_tracer: a window boundary was crossed (attempt {attempt}); re-running");
        }
        panic!("the allowance window boundary was crossed on both attempts");
    })
    .await
    .expect("allowance_admission_tracer timed out");
}

/// One full run of the tracer scenario. Returns `false` (having asserted nothing about the
/// refusal) when the allowance window rolled over while the scenario ran.
async fn run_allowance_tracer_once() -> bool {
    let (path, url) = temp_sqlite_url("allowance");
    let repository: Arc<dyn RunRepositoryPort> =
        Arc::new(SqliteRunRepository::new(&url).await.unwrap());
    let ledger = Arc::new(SqliteTreasuryLedger::new(&url).await.unwrap());
    let ledger_port: Arc<dyn TreasuryLedgerPort> = ledger.clone();
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("allowance-wf", build_chain_graph(1)));

    // D-02: the allowance grammar, deserialized exactly as an operator would write it.
    let config: TreasurerConfig = serde_json::from_value(serde_json::json!({
        "currency": "USD",
        "allowance": { "api_keys": { "svc-a": { "period": "1d", "amount": "2.50" } } }
    }))
    .unwrap();
    let policy = config.allowance_policy().unwrap();
    let treasurer = Treasurer::new(policy, Arc::clone(&ledger_port));
    let submission: Arc<dyn RunSubmissionPort> = Arc::new(
        RunSubmissionService::new(repository.clone(), queue.clone(), resolver.clone())
            .with_treasurer(Arc::new(treasurer)),
    );

    let mut api_keys = HashMap::new();
    api_keys.insert(
        "tracer-key-a".to_string(),
        Principal::new("svc-a", UserRole::User, TenantId::new("acme").unwrap()),
    );
    api_keys.insert(
        "tracer-key-b".to_string(),
        Principal::new("svc-b", UserRole::User, TenantId::new("acme").unwrap()),
    );
    let auth = AgentAuthConfig {
        enabled: true,
        api_keys,
        token_verifier: None,
        bearer_tenant: None,
    };
    let app = run_router(
        RunApiState::new()
            .with_submission(submission)
            .with_repository(repository.clone())
            .with_auth(auth),
    );

    // Seed: svc-a's own key has already spent its whole 2.50 USD allowance in this window.
    let window_before = window_for(ledger.store_now().await.unwrap(), 86_400).unwrap();
    ledger
        .settle(settle_request(
            LedgerScope::new("acme", "svc-a"),
            SettlementKey::new(RunId::new_v7(), 0, 0),
            2_500_000_000,
            usd(),
            "gpt-4",
        ))
        .await
        .unwrap();

    let post = |key: &'static str| {
        let app = app.clone();
        async move {
            app.oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs")
                    .header("content-type", "application/json")
                    .header("x-api-key", key)
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "assistant_id": "allowance-wf",
                            "input": {}
                        }))
                        .unwrap(),
                    ))
                    .expect("request builds"),
            )
            .await
            .expect("router responds")
        }
    };

    let refused = post("tracer-key-a").await;
    let window_after = window_for(ledger.store_now().await.unwrap(), 86_400).unwrap();
    if window_before != window_after {
        cleanup(&path);
        return false;
    }

    // (a) 429, (b) Retry-After, (c) the dedicated body code.
    assert_eq!(refused.status(), StatusCode::TOO_MANY_REQUESTS);
    let retry_after: u64 = refused
        .headers()
        .get("retry-after")
        .expect("a window refusal carries Retry-After")
        .to_str()
        .unwrap()
        .parse()
        .expect("Retry-After is whole seconds");
    assert!(
        (1..=86_400).contains(&retry_after),
        "Retry-After must lie inside one window, got {retry_after}"
    );
    let bytes = axum::body::to_bytes(refused.into_body(), usize::MAX)
        .await
        .expect("read refusal body");
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("refusal body is JSON");
    assert_eq!(body["error"]["code"], "allowance_exhausted");

    // (d) exactly the six D-13 keys, with the refused ceiling's own figures.
    let details = body["error"]["details"]
        .as_object()
        .expect("details object");
    let mut keys: Vec<&str> = details.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "balance",
            "ceiling",
            "kind",
            "scope",
            "window_end",
            "window_start"
        ]
    );
    assert_eq!(details["scope"], "api_key");
    assert_eq!(details["kind"], "window");
    assert_eq!(details["balance"], "2.5000 USD");
    assert_eq!(details["ceiling"], "2.5000 USD");
    let start = chrono::DateTime::parse_from_rfc3339(details["window_start"].as_str().unwrap())
        .expect("window_start is RFC 3339");
    let end = chrono::DateTime::parse_from_rfc3339(details["window_end"].as_str().unwrap())
        .expect("window_end is RFC 3339");
    assert_eq!(end - start, chrono::Duration::days(1));

    // (e) D-13 / D-00g: the body never repeats the tenant, the key name or the key value.
    let raw = String::from_utf8_lossy(&bytes);
    for forbidden in ["acme", "svc-a", "tracer-key-a"] {
        assert!(
            !raw.contains(forbidden),
            "the refusal body must not contain {forbidden:?}: {raw}"
        );
    }

    // (f) Nothing was persisted: no run row, no queue entry.
    let listed = repository.list(RunQuery::default()).await.unwrap();
    assert!(
        listed.items.is_empty(),
        "a refused submit writes no run row"
    );
    assert_eq!(queue.depth().await.unwrap(), 0);

    // (g) Another key of the same tenant has no allowance entry and is admitted (D-03).
    let admitted = post("tracer-key-b").await;
    assert_eq!(admitted.status(), StatusCode::ACCEPTED);

    cleanup(&path);
    true
}

/// A recording [`Herald`] double: captures every [`ExecutionMetadata`] handed to
/// `finalize_stream`, so the warn-path tracer can inspect the herald leg (the house
/// one-double-per-module precedent, mirroring `herald_sink.rs`'s own `RecordingHerald`).
#[derive(Default)]
struct RecordingHerald {
    captured: std::sync::Mutex<Vec<ExecutionMetadata>>,
}

impl RecordingHerald {
    fn captured(&self) -> Vec<ExecutionMetadata> {
        self.captured.lock().unwrap().clone()
    }
}

impl Herald for RecordingHerald {
    fn format_paladin_result(&self, _result: &PaladinResult) -> Result<String, HeraldError> {
        Ok(String::new())
    }

    fn format_battalion_result(&self, _result: &BattalionResult) -> Result<String, HeraldError> {
        Ok(String::new())
    }

    fn format_stream_chunk(&self, _chunk: &StreamChunk) -> Result<Option<String>, HeraldError> {
        Ok(None)
    }

    fn finalize_stream(&self, metadata: &ExecutionMetadata) -> Result<String, HeraldError> {
        self.captured.lock().unwrap().push(metadata.clone());
        Ok(String::new())
    }

    fn format_error(&self, error: &PaladinError) -> String {
        error.to_string()
    }

    fn name(&self) -> &str {
        "recording"
    }

    fn mime_type(&self) -> &str {
        "text/plain"
    }
}

/// `sha256=<hex>` HMAC of `body` under `key` -- a receiver's own recomputation.
fn recompute_signature(key: &[u8], body: &[u8]) -> String {
    use hmac::{Hmac, Mac};
    let mut mac = <Hmac<sha2::Sha256> as Mac>::new_from_slice(key).expect("any key length");
    mac.update(body);
    format!(
        "sha256={}",
        mac.finalize()
            .into_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}

/// The Phase 41 warn-path tracer (ALLOW-04, D-15, D-16, D-17, D-18; roadmap success
/// criterion 3): ONE admission at exactly the 80% warn threshold is observed exactly once in
/// each of three legs -- the durable notice row, the signed operator webhook and the run's
/// own trace stream (plus the herald line) -- without blocking the run, and a second
/// admission in the same window adds nothing to any of them.
///
/// 41-06, 41-07 and the earlier 41-08 tasks proved each leg separately; this test drives all
/// three from one admission over ONE on-disk SQLite file (production shares one run-store
/// file), through the real `run_router`, `Treasurer` (config-built), `RunSubmissionService`,
/// `WebhookDeliveryService` and `RunWorkerPool`. `build_run_api` spawns its own worker and
/// drain loop whose trace output is not observable, so this test assembles the same pieces
/// the builder does; `build_run_api_wires_the_allowance_warn_path` covers the builder itself.
///
/// Pitfall 10: windows are epoch-aligned, so a UTC boundary crossed mid-scenario re-runs it
/// once on a fresh store.
#[tokio::test(flavor = "multi_thread")]
async fn allowance_warn_path_tracer() {
    tokio::time::timeout(Duration::from_secs(30), async {
        for attempt in 1..=2 {
            if run_warn_path_tracer_once().await {
                return;
            }
            eprintln!(
                "allowance_warn_path_tracer: a window boundary was crossed (attempt {attempt}); re-running"
            );
        }
        panic!("the allowance window boundary was crossed on both attempts");
    })
    .await
    .expect("allowance_warn_path_tracer timed out");
}

/// One full run of the warn-path scenario. Returns `false` when the allowance window rolled
/// over while it ran (nothing further asserted).
async fn run_warn_path_tracer_once() -> bool {
    let (path, url) = temp_sqlite_url("allowance-warn");
    let repository: Arc<dyn RunRepositoryPort> =
        Arc::new(SqliteRunRepository::new(&url).await.unwrap());
    let ledger = Arc::new(SqliteTreasuryLedger::new(&url).await.unwrap());
    let ledger_port: Arc<dyn TreasuryLedgerPort> = ledger.clone();
    let notices: Arc<dyn TreasuryNoticePort> = ledger.clone();
    let deliveries: Arc<dyn WebhookDeliveryRepositoryPort> =
        Arc::new(SqliteWebhookDeliveryRepository::new(&url).await.unwrap());
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("allowance-wf", build_chain_graph(1)));

    // The operator receiver: captures raw body, signature and event header of every POST.
    type Captured = Arc<std::sync::Mutex<Vec<(Vec<u8>, String, String)>>>;
    let mut receiver = mockito::Server::new_async().await;
    let captured: Captured = Arc::new(std::sync::Mutex::new(Vec::new()));
    {
        let sink = Arc::clone(&captured);
        receiver
            .mock("POST", "/hook")
            .with_status_code_from_request(move |req| {
                let header = |name: &str| {
                    req.header(name)
                        .first()
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or_default()
                        .to_string()
                };
                sink.lock().unwrap().push((
                    req.body().cloned().unwrap_or_default(),
                    header("x-paladin-signature"),
                    header("x-paladin-event"),
                ));
                200
            })
            .expect_at_least(0)
            .create_async()
            .await;
    }

    // The Treasurer is built from config, never by hand (D-02, D-17).
    let hook_url = format!("{}/hook", receiver.url());
    let config: TreasurerConfig = serde_json::from_value(serde_json::json!({
        "currency": "USD",
        "allowance": {
            "warn_at": 80,
            "api_keys": { "svc-w": { "period": "1d", "amount": "1.00" } },
            "webhook": { "url": hook_url, "secret": "op-secret" }
        }
    }))
    .unwrap();
    let treasurer = Treasurer::new(config.allowance_policy().unwrap(), ledger_port)
        .with_notices(Arc::clone(&notices))
        .with_operator_webhook(OperatorNoticeTarget::new(
            hook_url.clone(),
            Arc::clone(&deliveries),
        ));
    let submission: Arc<dyn RunSubmissionPort> = Arc::new(
        RunSubmissionService::new(repository.clone(), queue.clone(), resolver.clone())
            .with_treasurer(Arc::new(treasurer)),
    );

    let mut api_keys = HashMap::new();
    api_keys.insert(
        "warn-key".to_string(),
        Principal::new("svc-w", UserRole::User, TenantId::new("acme").unwrap()),
    );
    let auth = AgentAuthConfig {
        enabled: true,
        api_keys,
        token_verifier: None,
        bearer_tenant: None,
    };
    let app = run_router(
        RunApiState::new()
            .with_submission(submission)
            .with_repository(repository.clone())
            .with_webhook_deliveries(Arc::clone(&deliveries))
            .with_auth(auth),
    );

    // Seed exactly 80% of the 1.00 USD ceiling (800_000_000 nano-units).
    let window_before = window_for(ledger.store_now().await.unwrap(), 86_400).unwrap();
    ledger
        .settle(settle_request(
            LedgerScope::new("acme", "svc-w"),
            SettlementKey::new(RunId::new_v7(), 0, 0),
            800_000_000,
            usd(),
            "gpt-4",
        ))
        .await
        .unwrap();

    let post = || {
        let app = app.clone();
        async move {
            app.oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs")
                    .header("content-type", "application/json")
                    .header("x-api-key", "warn-key")
                    .body(Body::from(
                        serde_json::to_vec(&serde_json::json!({
                            "assistant_id": "allowance-wf",
                            "input": {}
                        }))
                        .unwrap(),
                    ))
                    .expect("request builds"),
            )
            .await
            .expect("router responds")
        }
    };
    let read_json = |response: axum::response::Response| async move {
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice::<serde_json::Value>(&bytes).expect("JSON body")
    };

    // (a) The warning never blocks the run: 202 Accepted (D-15).
    let first = post().await;
    let window_after = window_for(ledger.store_now().await.unwrap(), 86_400).unwrap();
    if window_before != window_after {
        cleanup(&path);
        return false;
    }
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    let first_body = read_json(first).await;
    let run_id = RunId::parse(first_body["run_id"].as_str().unwrap()).unwrap();
    let thread_id = paladin_core::platform::container::waypoint::ThreadId::new(
        first_body["thread_id"].as_str().unwrap(),
    )
    .unwrap();

    // (b) Exactly one notice row, carrying the admitted run id and the D-14 figures.
    let rows = notices.notices_for_run(&run_id).await.unwrap();
    assert_eq!(rows.len(), 1, "exactly one notice row: {rows:?}");
    let warning = &rows[0].warning;
    assert_eq!(
        warning.scope_kind,
        paladin_core::platform::container::allowance::AllowanceScopeKind::ApiKey
    );
    assert_eq!(
        warning.limit_kind,
        paladin_core::platform::container::allowance::AllowanceLimitKind::Window
    );
    assert_eq!(
        paladin_core::platform::container::treasury_ledger::format_cost(&warning.balance),
        "0.8000 USD"
    );
    assert_eq!(
        paladin_core::platform::container::treasury_ledger::format_cost(&warning.ceiling),
        "1.0000 USD"
    );
    assert_eq!(warning.warn_at, 80);
    assert_eq!(rows[0].run_id.as_ref(), Some(&run_id));
    let recorded_at = rows[0].recorded_at;

    // (c) The operator notice is never listed under the admitting run (C3, D-17).
    let listing = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/v1/runs/{run_id}/webhook-deliveries"))
                .header("x-api-key", "warn-key")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(listing.status(), StatusCode::OK);
    let listing = read_json(listing).await;
    assert_eq!(
        listing["items"].as_array().map(Vec::len),
        Some(0),
        "the admitting run's delivery list never shows the operator notice: {listing}"
    );

    // (d) One delivery pass sends exactly one signed operator POST.
    let delivery_service = WebhookDeliveryService::new(
        Arc::clone(&deliveries),
        repository.clone(),
        WebhookDeliveryOptions::default(),
    )
    .unwrap()
    .with_guard(SsrfGuard::new(true))
    .with_operator_notice_secret(Some("op-secret".to_string()));
    let pass_at = recorded_at + chrono::Duration::seconds(1);
    assert_eq!(delivery_service.run_once(pass_at).await, 1);
    {
        let seen = captured.lock().unwrap();
        assert_eq!(seen.len(), 1, "exactly one operator POST");
        let (raw, signature, event) = &seen[0];
        assert_eq!(event, "allowance_warning");
        assert_eq!(
            signature,
            &recompute_signature(b"op-secret", raw),
            "the HMAC verifies over the captured raw bytes with the operator secret"
        );
        let payload: serde_json::Value = serde_json::from_slice(raw).unwrap();
        assert_eq!(payload["run_id"], serde_json::json!(run_id));
        // The twelve documented keys (D-17 amended at the 41-01 checkpoint, option-b).
        let mut keys: Vec<&str> = payload
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "api_key_id",
                "balance",
                "ceiling",
                "event",
                "kind",
                "run_id",
                "scope",
                "tenant_id",
                "timestamp",
                "warn_at",
                "window_end",
                "window_start",
            ]
        );
        assert_eq!(payload["tenant_id"], "acme");
        assert_eq!(payload["api_key_id"], "svc-w");
        let raw_text = String::from_utf8_lossy(raw);
        assert!(
            !raw_text.contains("warn-key") && !raw_text.contains("op-secret"),
            "the payload carries no key value and no secret: {raw_text}"
        );
    }

    // (e) One worker dispatch of the admitted run: exactly one trace record before
    // `RunStarted` and exactly one herald allowance line. The pool is built on the
    // `with_engine_factory` path, the only path the trace config, run-trace port and herald
    // attach on (the builder's own composition, `build_run_api`).
    let waypoints = Arc::new(InMemoryWaypointStore::new());
    let factory_store = waypoints.clone();
    let engine_factory: Arc<
        dyn Fn(tokio_util::sync::CancellationToken) -> WarEngine<InMemoryWaypointStore>
            + Send
            + Sync,
    > = Arc::new(move |token| {
        WarEngine::new(Arc::new(UnusedPaladinPort), factory_store.clone())
            .with_cancellation_token(token)
    });
    let traces = Arc::new(InMemoryRunTraceStore::new());
    let herald = Arc::new(RecordingHerald::default());
    let pool = RunWorkerPool::new(
        Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoints.clone(),
        )),
        waypoints.clone(),
        repository.clone(),
        queue.clone(),
        resolver.clone(),
        Duration::from_secs(30),
    )
    .with_engine_factory(engine_factory)
    .with_trace_config(TraceConfig {
        log_sink: false,
        persist: true,
        ..TraceConfig::default()
    })
    .with_run_trace_port(traces.clone())
    .with_herald(herald.clone() as Arc<dyn Herald>)
    .with_treasury_notices(Arc::clone(&notices));

    assert!(pool.run_once().await.unwrap());
    assert_eq!(
        repository.get(&run_id).await.unwrap().unwrap().status,
        RunStatus::Completed,
        "the warned run still completes"
    );
    let read_traces = |thread: paladin_core::platform::container::waypoint::ThreadId| {
        let traces = traces.clone();
        async move {
            let mut rows = Vec::new();
            for _ in 0..100 {
                rows = traces.read(&thread, 0, 100).await.unwrap();
                if rows
                    .iter()
                    .any(|r| matches!(r.event, TraceEvent::RunFinished { .. }))
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
            rows
        }
    };
    let rows = read_traces(thread_id.clone()).await;
    let warnings: Vec<_> = rows
        .iter()
        .filter(|r| matches!(r.event, TraceEvent::AllowanceWarning { .. }))
        .collect();
    assert_eq!(warnings.len(), 1, "exactly one allowance_warning: {rows:?}");
    let started = rows
        .iter()
        .find(|r| matches!(r.event, TraceEvent::RunStarted { .. }))
        .expect("the run has a RunStarted row");
    assert!(
        warnings[0].seq < started.seq,
        "the warning ({}) precedes RunStarted ({})",
        warnings[0].seq,
        started.seq
    );
    let metadata = herald.captured();
    assert_eq!(metadata.len(), 1, "one herald summary for the run");
    let display = metadata[0]
        .allowance_warning_display()
        .expect("the herald metadata carries the allowance warning");
    assert_eq!(
        display.matches("allowance:").count(),
        1,
        "exactly one allowance line: {display}"
    );

    // (f) A second admission in the same window adds nothing to any leg.
    let second = post().await;
    let window_after = window_for(ledger.store_now().await.unwrap(), 86_400).unwrap();
    if window_before != window_after {
        cleanup(&path);
        return false;
    }
    assert_eq!(second.status(), StatusCode::ACCEPTED);
    let second_body = read_json(second).await;
    let second_run = RunId::parse(second_body["run_id"].as_str().unwrap()).unwrap();
    let second_thread = paladin_core::platform::container::waypoint::ThreadId::new(
        second_body["thread_id"].as_str().unwrap(),
    )
    .unwrap();
    assert_ne!(second_run, run_id);
    assert!(
        notices
            .notices_for_run(&second_run)
            .await
            .unwrap()
            .is_empty(),
        "no notice row for the second run"
    );
    assert_eq!(
        delivery_service
            .run_once(pass_at + chrono::Duration::seconds(1))
            .await,
        0,
        "no further operator delivery is due"
    );
    assert_eq!(
        captured.lock().unwrap().len(),
        1,
        "the receiver count stays 1"
    );
    assert!(pool.run_once().await.unwrap());
    let second_rows = read_traces(second_thread).await;
    assert!(
        !second_rows
            .iter()
            .any(|r| matches!(r.event, TraceEvent::AllowanceWarning { .. })),
        "no allowance_warning in the second run's trace: {second_rows:?}"
    );
    let metadata = herald.captured();
    assert_eq!(metadata.len(), 2);
    assert!(
        metadata[1].allowance_warning_display().is_none(),
        "the second run's herald metadata carries no allowance line"
    );

    cleanup(&path);
    true
}

/// Edge ALLOW-02/concurrency (41-04): eight concurrent `POST /v1/runs` by ONE exhausted API
/// key through the real `run_router` all answer `429 allowance_exhausted`, and the run
/// repository and queue stay empty -- admission is a check before any write, so racing
/// refused callers cannot leak a row or a queue entry between them.
///
/// The ledger is the in-memory adapter and the key carries a lifetime cap alongside its window
/// so a window boundary crossed mid-test cannot make the balance read zero.
#[tokio::test(flavor = "multi_thread")]
async fn parallel_submissions_by_an_exhausted_key_are_all_refused_and_write_nothing() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let ledger: Arc<dyn TreasuryLedgerPort> =
            Arc::new(paladin_storage::treasury::in_memory::InMemoryTreasuryLedger::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("parallel-wf", build_chain_graph(1)));

        let config: TreasurerConfig = serde_json::from_value(serde_json::json!({
            "currency": "USD",
            "allowance": { "api_keys": { "svc-a": {
                "period": "1d", "amount": "2.50", "lifetime": "2.50"
            } } }
        }))
        .unwrap();
        let treasurer = Treasurer::new(config.allowance_policy().unwrap(), Arc::clone(&ledger));
        let submission: Arc<dyn RunSubmissionPort> = Arc::new(
            RunSubmissionService::new(repository.clone(), queue.clone(), resolver.clone())
                .with_treasurer(Arc::new(treasurer)),
        );

        let mut api_keys = HashMap::new();
        api_keys.insert(
            "parallel-key-a".to_string(),
            Principal::new("svc-a", UserRole::User, TenantId::new("acme").unwrap()),
        );
        let app = run_router(
            RunApiState::new()
                .with_submission(submission)
                .with_repository(repository.clone())
                .with_auth(AgentAuthConfig {
                    enabled: true,
                    api_keys,
                    token_verifier: None,
                    bearer_tenant: None,
                }),
        );

        ledger
            .settle(settle_request(
                LedgerScope::new("acme", "svc-a"),
                SettlementKey::new(RunId::new_v7(), 0, 0),
                2_500_000_000,
                usd(),
                "gpt-4",
            ))
            .await
            .unwrap();

        let mut handles = Vec::new();
        for _ in 0..8 {
            let app = app.clone();
            handles.push(tokio::spawn(async move {
                app.oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("content-type", "application/json")
                        .header("x-api-key", "parallel-key-a")
                        .body(Body::from(
                            serde_json::to_vec(&serde_json::json!({
                                "assistant_id": "parallel-wf",
                                "input": {}
                            }))
                            .unwrap(),
                        ))
                        .expect("request builds"),
                )
                .await
                .expect("router responds")
                .status()
            }));
        }
        for handle in handles {
            assert_eq!(
                handle.await.unwrap(),
                StatusCode::TOO_MANY_REQUESTS,
                "every concurrent submission by the exhausted key is refused"
            );
        }

        let listed = repository.list(RunQuery::default()).await.unwrap();
        assert!(listed.items.is_empty(), "refused submissions write no row");
        assert_eq!(queue.depth().await.unwrap(), 0);
    })
    .await
    .expect(
        "parallel_submissions_by_an_exhausted_key_are_all_refused_and_write_nothing did not hang",
    );
}

/// PRD acceptance 3 / D-52: ten concurrent `POST /v1/runs` for ONE thread,
/// through the real `run_router` (oneshot, cloned router) over
/// `SqliteRunRepository` on a temp file -- exactly one `202` and nine `409
/// thread_busy`, proving D-17's partial-unique-index invariant holds under
/// real concurrent writers, not just a single-threaded check-then-insert.
#[tokio::test(flavor = "multi_thread")]
async fn ten_concurrent_submits_one_accepted() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("race");
        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new(&repo_url).await.unwrap());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("race-wf", build_chain_graph(1)));
        let submission: Arc<dyn RunSubmissionPort> = Arc::new(RunSubmissionService::new(
            repository.clone(),
            queue.clone(),
            resolver.clone(),
        ));
        let state = RunApiState::new()
            .with_submission(submission)
            .with_repository(repository.clone());
        let app = run_router(state);

        let mut handles = Vec::with_capacity(10);
        for _ in 0..10 {
            let app = app.clone();
            handles.push(tokio::spawn(async move {
                let body = serde_json::to_vec(&serde_json::json!({
                    "assistant_id": "race-wf",
                    "thread_id": "race-thread",
                    "input": {}
                }))
                .unwrap();
                app.oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("content-type", "application/json")
                        .body(Body::from(body))
                        .unwrap(),
                )
                .await
                .unwrap()
                .status()
            }));
        }

        let mut accepted = 0;
        let mut busy = 0;
        for handle in handles {
            match handle.await.unwrap() {
                StatusCode::ACCEPTED => accepted += 1,
                StatusCode::CONFLICT => busy += 1,
                other => panic!("unexpected status from a concurrent submit: {other}"),
            }
        }

        assert_eq!(
            accepted, 1,
            "exactly one of ten concurrent submits must be accepted"
        );
        assert_eq!(busy, 9, "the other nine must be 409 thread_busy");

        cleanup(&repo_path);
    })
    .await
    .expect("ten_concurrent_submits_one_accepted did not hang");
}

/// D-45: a run to completion, forked from its second superstep's Waypoint
/// with an edit, completes and its history shows a Waypoint recording
/// `fork_of == Some(wp2)`.
#[tokio::test(flavor = "multi_thread")]
async fn fork_run_completes_from_waypoint() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let repository: Arc<dyn RunRepositoryPort> = Arc::new(InMemoryRunRepository::new());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let waypoint_store = Arc::new(InMemoryWaypointStore::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("fork-wf", build_chain_graph(3)));

        let engine = Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoint_store.clone(),
        ));
        let worker = RunWorkerPool::new(
            engine,
            waypoint_store.clone(),
            repository.clone(),
            queue.clone(),
            resolver.clone(),
            Duration::from_secs(30),
        );

        let waypoints_port: Arc<dyn WaypointPort> = waypoint_store.clone();
        let submission =
            RunSubmissionService::new(repository.clone(), queue.clone(), resolver.clone())
                .with_waypoints(waypoints_port);

        let accepted = submission
            .submit(SubmitRun {
                assistant_id: "fork-wf".to_string(),
                version: None,
                thread_id: None,
                input: serde_json::json!({}),
                webhook: None,
                requested_by: None,
                attributed_to: None,
            })
            .await
            .unwrap();
        let thread_id = accepted.thread_id.clone();

        // The whole 3-superstep chain runs synchronously inside one
        // dispatch (`WarEngine::start` drives the graph to `Completed`
        // internally; the worker never re-dequeues mid-graph).
        assert!(worker.run_once().await.unwrap());

        let original_run = repository.get(&accepted.run_id).await.unwrap().unwrap();
        assert_eq!(original_run.status, RunStatus::Completed);

        let history = waypoint_store
            .history(&thread_id, None, None)
            .await
            .unwrap();
        let wp2 = history
            .iter()
            .find(|s| s.superstep == 2)
            .expect("a superstep-2 waypoint exists")
            .waypoint_id;

        // D-08: the fork is attributed to the FORKING principal (svc-a of acme), not
        // to whoever submitted the original (unattributed) run; the fork's own
        // latest-run lookup on the thread is unscoped (D-12/Pitfall 8) and still
        // finds the unattributed original. The forking principal is an Admin
        // (`RunReadScope::All`) because a tenant-scoped `User` may not fork a thread
        // whose latest run is unattributed -- the tenant guard treats it exactly
        // like a hidden run (phase 40 review WR-01).
        let forking_principal =
            PrincipalRef::new("svc-a", TenantId::new("acme").unwrap(), UserRole::Admin);
        let forked = submission
            .fork(ForkRun {
                thread_id: thread_id.clone(),
                from_waypoint_id: wp2,
                edit: None,
                webhook: None,
                requested_by: Some(forking_principal),
            })
            .await
            .unwrap();
        assert_eq!(forked.thread_id, thread_id);
        assert_ne!(forked.run_id, accepted.run_id);
        let forked_at_submit = repository.get(&forked.run_id).await.unwrap().unwrap();
        assert_eq!(
            forked_at_submit.submitted_by,
            Some(RunAttribution::new(TenantId::new("acme").unwrap(), "svc-a")),
            "a forked run is attributed to the forking principal (D-08)"
        );

        // Drive the fork dispatch -- `WorkerDispatch::decide` sees
        // `run.fork_from` on the freshly-enqueued forked run and the
        // latest Waypoint's `fork_of` not yet matching `wp2`, so it
        // selects `Fork` and calls `WarEngine::fork`.
        assert!(worker.run_once().await.unwrap());

        let forked_run = repository.get(&forked.run_id).await.unwrap().unwrap();
        assert_eq!(forked_run.status, RunStatus::Completed);

        let history_after_fork = waypoint_store
            .history(&thread_id, None, None)
            .await
            .unwrap();
        let fork_waypoint = history_after_fork
            .iter()
            .find(|s| s.fork_of == Some(wp2))
            .expect("a waypoint records fork_of == Some(wp2)");
        assert_eq!(fork_waypoint.fork_of, Some(wp2));
    })
    .await
    .expect("fork_run_completes_from_waypoint did not hang");
}

/// Phase 40's own tracer (TENANT-01, TENANT-02, PLAT-07, D-01, D-02, D-08, D-09, D-11,
/// D-12): an API key's configured tenant travels config -> `Principal.tenant_id`
/// (paladin-web) -> `PrincipalRef` on `SubmitRun` (paladin-ports) -> `Run.submitted_by`
/// (paladin-core) -> the `008` columns written by `SqliteRunRepository` (paladin-storage)
/// -> `GET /v1/runs/{id}` gated by `load_visible_run` (`RunReadScope::permits`) -- driven
/// through the real `run_router` over an on-disk SQLite store.
#[tokio::test(flavor = "multi_thread")]
async fn tenant_scoped_run_read_tracer() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("tenant");
        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new(&repo_url).await.unwrap());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("tenant-wf", build_chain_graph(1)));
        let submission: Arc<dyn RunSubmissionPort> = Arc::new(RunSubmissionService::new(
            repository.clone(),
            queue.clone(),
            resolver.clone(),
        ));

        let mut api_keys = HashMap::new();
        api_keys.insert(
            "tracer-key-a".to_string(),
            Principal::new("svc-a", UserRole::User, TenantId::new("acme").unwrap()),
        );
        api_keys.insert(
            "tracer-key-b".to_string(),
            Principal::new("svc-b", UserRole::User, TenantId::new("globex").unwrap()),
        );
        api_keys.insert(
            "tracer-key-ops".to_string(),
            Principal::new("ops", UserRole::Admin, TenantId::new("ops-tenant").unwrap()),
        );
        let auth = AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        };

        let state = RunApiState::new()
            .with_submission(submission)
            .with_repository(repository.clone())
            .with_auth(auth);
        let app = run_router(state);

        // (a) POST /v1/runs with key tracer-key-a, plus a spoofed tenant on every
        // client-controlled surface (body field, query param, header) -- the tenant must
        // still come only from AgentAuthConfig (D-02).
        let submit_body = serde_json::to_vec(&serde_json::json!({
            "assistant_id": "tenant-wf",
            "thread_id": "tenant-thread-1",
            "input": {},
            "tenant_id": "globex",
            "tenant": "globex"
        }))
        .unwrap();
        let submit_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs?tenant_id=globex")
                    .header("content-type", "application/json")
                    .header("x-api-key", "tracer-key-a")
                    .header("x-tenant-id", "globex")
                    .body(Body::from(submit_body))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(submit_response.status(), StatusCode::ACCEPTED);
        let submit_bytes = axum::body::to_bytes(submit_response.into_body(), usize::MAX)
            .await
            .expect("read submit body");
        let submit_json: serde_json::Value =
            serde_json::from_slice(&submit_bytes).expect("submit body is JSON");
        let run_id = RunId::parse(submit_json["run_id"].as_str().expect("run_id string"))
            .expect("run_id parses");

        // (b) The repository round trip: the run is attributed to the KEY's configured
        // tenant (acme), never the spoofed globex from the request (D-02, D-09).
        let stored = repository
            .get(&run_id)
            .await
            .expect("repository read succeeds")
            .expect("run exists");
        let expected_attribution = RunAttribution::new(TenantId::new("acme").unwrap(), "svc-a");
        assert_eq!(stored.submitted_by, Some(expected_attribution.clone()));

        // (c) A second submission by the same principal on a different thread carries
        // IDENTICAL attribution (edge TENANT-02/adjacency).
        let second_submit_body = serde_json::to_vec(&serde_json::json!({
            "assistant_id": "tenant-wf",
            "thread_id": "tenant-thread-2",
            "input": {}
        }))
        .unwrap();
        let second_submit_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs")
                    .header("content-type", "application/json")
                    .header("x-api-key", "tracer-key-a")
                    .body(Body::from(second_submit_body))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(second_submit_response.status(), StatusCode::ACCEPTED);
        let second_submit_bytes =
            axum::body::to_bytes(second_submit_response.into_body(), usize::MAX)
                .await
                .expect("read second submit body");
        let second_submit_json: serde_json::Value =
            serde_json::from_slice(&second_submit_bytes).expect("second submit body is JSON");
        let second_run_id = RunId::parse(
            second_submit_json["run_id"]
                .as_str()
                .expect("run_id string"),
        )
        .expect("run_id parses");
        let second_stored = repository
            .get(&second_run_id)
            .await
            .expect("repository read succeeds")
            .expect("run exists");
        assert_eq!(second_stored.submitted_by, Some(expected_attribution));

        // (d) GET /v1/runs/{run_id}: the owner (svc-a) and an Admin of another tenant
        // (ops) both see it (D-11's Admin arm).
        let owner_get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{run_id}"))
                    .header("x-api-key", "tracer-key-a")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(owner_get.status(), StatusCode::OK);

        let admin_get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{run_id}"))
                    .header("x-api-key", "tracer-key-ops")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(admin_get.status(), StatusCode::OK);

        // (e) GET /v1/runs/{run_id} with a DIFFERENT tenant's key (svc-b, globex) is
        // the SAME missing-run 404 a genuinely unknown run id gets (PLAT-07, D-12): no
        // 403, no shape/timing difference between "hidden" and "missing".
        let hidden_get = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{run_id}"))
                    .header("x-api-key", "tracer-key-b")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(hidden_get.status(), StatusCode::NOT_FOUND);
        let hidden_bytes = axum::body::to_bytes(hidden_get.into_body(), usize::MAX)
            .await
            .expect("read hidden body");

        let missing_run_id = RunId::new_v7();
        let missing_get = app
            .oneshot(
                Request::builder()
                    .uri(format!("/v1/runs/{missing_run_id}"))
                    .header("x-api-key", "tracer-key-b")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(missing_get.status(), StatusCode::NOT_FOUND);
        let missing_bytes = axum::body::to_bytes(missing_get.into_body(), usize::MAX)
            .await
            .expect("read missing body");

        let hidden_text = String::from_utf8(hidden_bytes.to_vec()).expect("utf8 body");
        let missing_text = String::from_utf8(missing_bytes.to_vec()).expect("utf8 body");
        let normalized_hidden =
            hidden_text.replace(&run_id.to_string(), &missing_run_id.to_string());
        assert_eq!(
            normalized_hidden, missing_text,
            "a hidden run's 404 body must be byte-identical to a genuinely missing run's, \
             once the two ids are swapped (PLAT-07, D-12)"
        );

        cleanup(&repo_path);
    })
    .await
    .expect("tenant_scoped_run_read_tracer did not hang");
}

/// Phase 40's list half of PLAT-07 (D-02, D-11, D-12, D-14): `GET /v1/runs` through the
/// real `run_router` over an on-disk `SqliteRunRepository` returns only the calling
/// principal's tenant's runs (across two API keys of the same tenant), every run for
/// an Admin, an exact empty page for a tenant with no runs, an empty page when another
/// tenant's `thread_id` is requested, and a gap-free `?limit=1` keyset walk -- with a
/// `tenant_id` query parameter changing nothing.
#[tokio::test(flavor = "multi_thread")]
async fn tenant_scoped_run_list_e2e() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("tenant-list");
        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new(&repo_url).await.unwrap());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("tenant-wf", build_chain_graph(1)));
        let submission: Arc<dyn RunSubmissionPort> = Arc::new(RunSubmissionService::new(
            repository.clone(),
            queue.clone(),
            resolver.clone(),
        ));

        let mut api_keys = HashMap::new();
        api_keys.insert(
            "tracer-key-a".to_string(),
            Principal::new("svc-a", UserRole::User, TenantId::new("acme").unwrap()),
        );
        api_keys.insert(
            "tracer-key-a2".to_string(),
            Principal::new("svc-a2", UserRole::User, TenantId::new("acme").unwrap()),
        );
        api_keys.insert(
            "tracer-key-b".to_string(),
            Principal::new("svc-b", UserRole::User, TenantId::new("globex").unwrap()),
        );
        api_keys.insert(
            "tracer-key-ops".to_string(),
            Principal::new("ops", UserRole::Admin, TenantId::new("ops-tenant").unwrap()),
        );
        api_keys.insert(
            "tracer-key-empty".to_string(),
            Principal::new(
                "svc-empty",
                UserRole::User,
                TenantId::new("empty-tenant").unwrap(),
            ),
        );
        let auth = AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        };

        let state = RunApiState::new()
            .with_submission(submission)
            .with_repository(repository.clone())
            .with_auth(auth);
        let app = run_router(state);

        async fn submit(app: &axum::Router, key: &str, thread: &str) -> String {
            let body = serde_json::to_vec(&serde_json::json!({
                "assistant_id": "tenant-wf",
                "thread_id": thread,
                "input": {}
            }))
            .unwrap();
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/runs")
                        .header("content-type", "application/json")
                        .header("x-api-key", key)
                        .body(Body::from(body))
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(
                response.status(),
                StatusCode::ACCEPTED,
                "submit on {thread}"
            );
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read submit body");
            let json: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
            json["run_id"].as_str().expect("run_id string").to_string()
        }

        async fn list(app: &axum::Router, key: &str, uri: &str) -> (Vec<u8>, serde_json::Value) {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(uri)
                        .header("x-api-key", key)
                        .body(Body::empty())
                        .expect("request builds"),
                )
                .await
                .expect("router responds");
            assert_eq!(response.status(), StatusCode::OK, "GET {uri} with {key}");
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .expect("read list body");
            let json: serde_json::Value = serde_json::from_slice(&bytes).expect("JSON");
            (bytes.to_vec(), json)
        }

        fn thread_ids(page: &serde_json::Value) -> Vec<String> {
            let mut ids: Vec<String> = page["items"]
                .as_array()
                .expect("items array")
                .iter()
                .map(|item| item["thread_id"].as_str().expect("thread_id").to_string())
                .collect();
            ids.sort();
            ids
        }

        // Two acme runs from two DIFFERENT acme keys (edge PLAT-07/adjacency), one
        // globex run. No worker runs, so every run stays Queued.
        let run_a1 = submit(&app, "tracer-key-a", "list-a-1").await;
        let run_a2 = submit(&app, "tracer-key-a2", "list-a-2").await;
        let run_b1 = submit(&app, "tracer-key-b", "list-b-1").await;

        // (a) A user key lists exactly its own tenant's runs, including the run
        // submitted by the OTHER acme key.
        let (_, page_a) = list(&app, "tracer-key-a", "/v1/runs").await;
        assert_eq!(thread_ids(&page_a), vec!["list-a-1", "list-a-2"]);
        assert!(page_a["next_cursor"].is_null());

        let (_, page_b) = list(&app, "tracer-key-b", "/v1/runs").await;
        assert_eq!(thread_ids(&page_b), vec!["list-b-1"]);
        assert_eq!(page_b["items"][0]["run_id"].as_str(), Some(run_b1.as_str()));

        // (b) An Admin key of an unrelated tenant lists every run (D-11).
        let (_, page_ops) = list(&app, "tracer-key-ops", "/v1/runs").await;
        assert_eq!(
            thread_ids(&page_ops),
            vec!["list-a-1", "list-a-2", "list-b-1"]
        );

        // (c) Edge PLAT-07/empty: a tenant with no runs is 200 with EXACTLY
        // `{"items":[],"next_cursor":null}`, never 404.
        let (empty_bytes, _) = list(&app, "tracer-key-empty", "/v1/runs").await;
        assert_eq!(
            String::from_utf8(empty_bytes).expect("utf8 body"),
            r#"{"items":[],"next_cursor":null}"#
        );

        // (d) D-14: another tenant's thread id under a user scope is an empty page.
        let (_, cross) = list(&app, "tracer-key-b", "/v1/runs?thread_id=list-a-1").await;
        assert!(cross["items"].as_array().expect("items").is_empty());
        assert!(cross["next_cursor"].is_null());

        // (e) D-02: `?tenant_id=` is neither a filter nor an override.
        let (_, spoofed) = list(&app, "tracer-key-a", "/v1/runs?tenant_id=globex").await;
        assert_eq!(thread_ids(&spoofed), vec!["list-a-1", "list-a-2"]);

        // (f) A `?limit=1` keyset walk under a user scope yields both acme runs, one
        // per page, then `next_cursor: null` -- the tenant predicate is inside the
        // SQL, so pages stay full and the cursor stays correct (D-12, T-40-09).
        let (_, first) = list(&app, "tracer-key-a", "/v1/runs?limit=1").await;
        assert_eq!(first["items"].as_array().expect("items").len(), 1);
        let cursor = first["next_cursor"]
            .as_str()
            .expect("first page has a cursor");
        let (_, second) = list(
            &app,
            "tracer-key-a",
            &format!("/v1/runs?limit=1&cursor={cursor}"),
        )
        .await;
        assert_eq!(second["items"].as_array().expect("items").len(), 1);
        assert!(second["next_cursor"].is_null(), "second page is the last");
        let mut walked = vec![
            first["items"][0]["run_id"]
                .as_str()
                .expect("run_id")
                .to_string(),
            second["items"][0]["run_id"]
                .as_str()
                .expect("run_id")
                .to_string(),
        ];
        walked.sort();
        let mut expected = vec![run_a1, run_a2];
        expected.sort();
        assert_eq!(
            walked, expected,
            "the walk covers exactly the two acme runs"
        );

        cleanup(&repo_path);
    })
    .await
    .expect("tenant_scoped_run_list_e2e did not hang");
}

/// Phase 40's cross-tenant cancel proof (D-13, T-40-19, PLAT-07) over a real on-disk
/// `SqliteRunRepository` through the real `run_router` and `RunSubmissionService`: key b's
/// `POST /v1/runs/{id}/cancel` on key a's run is the missing-run 404 and writes NOTHING --
/// the row's `cancel_requested` stays false -- while key a's own cancel then answers 202
/// and sets the flag. A foreign cancel is a cross-tenant mutation, not just a leak, so the
/// visibility gate must answer before `RunSubmissionPort::cancel` is ever reached.
#[tokio::test(flavor = "multi_thread")]
async fn cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let (repo_path, repo_url) = temp_sqlite_url("cross-tenant-cancel");
        let repository: Arc<dyn RunRepositoryPort> =
            Arc::new(SqliteRunRepository::new(&repo_url).await.unwrap());
        let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());
        let resolver: Arc<dyn AssistantResolver> =
            Arc::new(CodeWorkflowResolver::new().register("cancel-wf", build_chain_graph(1)));
        let submission: Arc<dyn RunSubmissionPort> = Arc::new(RunSubmissionService::new(
            repository.clone(),
            queue.clone(),
            resolver.clone(),
        ));

        let mut api_keys = HashMap::new();
        api_keys.insert(
            "cancel-key-a".to_string(),
            Principal::new("svc-a", UserRole::User, TenantId::new("acme").unwrap()),
        );
        api_keys.insert(
            "cancel-key-b".to_string(),
            Principal::new("svc-b", UserRole::User, TenantId::new("globex").unwrap()),
        );
        let auth = AgentAuthConfig {
            enabled: true,
            api_keys,
            token_verifier: None,
            bearer_tenant: None,
        };

        let state = RunApiState::new()
            .with_submission(submission)
            .with_repository(repository.clone())
            .with_auth(auth);
        let app = run_router(state);

        // Key a submits; no worker runs, so the run stays Queued (non-terminal) and a
        // cancel is admissible for whoever may see it.
        let submit_body = serde_json::to_vec(&serde_json::json!({
            "assistant_id": "cancel-wf",
            "thread_id": "cancel-a-1",
            "input": {}
        }))
        .unwrap();
        let submit_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/runs")
                    .header("content-type", "application/json")
                    .header("x-api-key", "cancel-key-a")
                    .body(Body::from(submit_body))
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(submit_response.status(), StatusCode::ACCEPTED);
        let submit_bytes = axum::body::to_bytes(submit_response.into_body(), usize::MAX)
            .await
            .expect("read submit body");
        let submit_json: serde_json::Value =
            serde_json::from_slice(&submit_bytes).expect("submit body is JSON");
        let run_id = RunId::parse(submit_json["run_id"].as_str().expect("run_id string"))
            .expect("run_id parses");

        // Key b (globex) cancelling acme's run: the missing-run 404, and the row is
        // untouched.
        let foreign_cancel = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/runs/{run_id}/cancel"))
                    .header("x-api-key", "cancel-key-b")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(
            foreign_cancel.status(),
            StatusCode::NOT_FOUND,
            "a foreign tenant's cancel must be the missing-run 404 (D-13, PLAT-07)"
        );
        let after_foreign = repository
            .get(&run_id)
            .await
            .expect("repository read succeeds")
            .expect("run exists");
        assert!(
            !after_foreign.cancel_requested,
            "a foreign cancel must never reach RunSubmissionPort::cancel or write the flag"
        );
        assert_eq!(after_foreign.status, RunStatus::Queued);

        // Key a (the owner) cancels: 202, and the flag is now set.
        let owner_cancel = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/v1/runs/{run_id}/cancel"))
                    .header("x-api-key", "cancel-key-a")
                    .body(Body::empty())
                    .expect("request builds"),
            )
            .await
            .expect("router responds");
        assert_eq!(owner_cancel.status(), StatusCode::ACCEPTED);
        let after_owner = repository
            .get(&run_id)
            .await
            .expect("repository read succeeds")
            .expect("run exists");
        assert!(
            after_owner.cancel_requested,
            "the owner's cancel must set cancel_requested on the row"
        );

        cleanup(&repo_path);
    })
    .await
    .expect("cross_tenant_cancel_is_a_404_and_writes_no_cancel_flag did not hang");
}

/// A test node standing in for a priced superstep (ALLOW-03, Phase 42): it counts its own runs
/// and, when `amount_nanos` is positive, settles that many nano-units against `scope` through
/// the real ledger -- exactly what the engine's settlement does after a metered model call.
struct SpendingNode {
    ledger: Arc<dyn TreasuryLedgerPort>,
    scope: LedgerScope,
    amount_nanos: i64,
    runs: Arc<std::sync::atomic::AtomicUsize>,
}

#[async_trait]
impl StateNode for SpendingNode {
    async fn run(
        &self,
        _state: &paladin_core::platform::container::battlefield::Battlefield,
        _ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        self.runs.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.amount_nanos > 0 {
            self.ledger
                .settle(settle_request(
                    self.scope.clone(),
                    SettlementKey::new(RunId::new_v7(), 0, 0),
                    self.amount_nanos,
                    usd(),
                    "gpt-4",
                ))
                .await
                .map_err(|e| StateNodeError(format!("test settlement failed: {e}")))?;
        }
        Ok(StateDelta::new().into())
    }
}

/// The Phase 42 engine-path halt tracer (ALLOW-03, D-00a, D-01, D-04, D-05): an allowance
/// exhausted by a run's own first superstep halts that run at its next superstep boundary,
/// end to end through every layer the halt touches -- `TreasurerConfig` -> `AllowancePolicy` ->
/// `Treasurer` (the shared ceiling evaluation and `TreasurerSpendGuard`) -> the `SpendGuard`
/// port -> the `WarEngine` boundary check writing a `Halted` Waypoint -> `RunOutcome::Halted
/// { cause: Spend }` -> the worker's `map_outcome` -> the run row in the real SQLite run
/// repository -> `GET /v1/runs/{id}` answering `halted` with no error.
///
/// Pitfall 10: windows are epoch-aligned, so a UTC boundary crossed mid-scenario would make the
/// halt vanish and the test lie; the scenario re-runs once on a fresh store when it happens.
#[tokio::test(flavor = "multi_thread")]
async fn engine_spend_halt_tracer() {
    tokio::time::timeout(Duration::from_secs(30), async {
        for attempt in 1..=2 {
            if run_spend_halt_tracer_once(true).await {
                return;
            }
            eprintln!(
                "engine_spend_halt_tracer: a window boundary was crossed (attempt {attempt}); \
                 re-running"
            );
        }
        panic!("the allowance window boundary was crossed on both attempts");
    })
    .await
    .expect("engine_spend_halt_tracer timed out");
}

/// One full run of the halt tracer. `attach_treasurer` is `false` only for the red check of
/// the plan (the pool built without `with_treasurer` must NOT halt). Returns `false` (having
/// asserted nothing about the halt) when the allowance window rolled over mid-scenario.
async fn run_spend_halt_tracer_once(attach_treasurer: bool) -> bool {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let (path, url) = temp_sqlite_url("spend_halt");
    let repository: Arc<dyn RunRepositoryPort> =
        Arc::new(SqliteRunRepository::new(&url).await.unwrap());
    let ledger = Arc::new(SqliteTreasuryLedger::new(&url).await.unwrap());
    let ledger_port: Arc<dyn TreasuryLedgerPort> = ledger.clone();
    let queue: Arc<dyn RunQueuePort> = Arc::new(InMemoryRunQueue::new());

    // Three-node chain: n0 settles exactly 1.00 USD under the submitter's own scope; n1 and n2
    // settle nothing and must never run once the allowance is exhausted.
    let counters: Vec<Arc<AtomicUsize>> = (0..3).map(|_| Arc::new(AtomicUsize::new(0))).collect();
    let mut graph = WarGraph::new(BattlefieldSchema::new(vec![]), EngineLimits::default());
    let ids: Vec<NodeId> = (0..3).map(|i| NodeId::new(format!("n{i}"))).collect();
    for (i, id) in ids.iter().enumerate() {
        graph.add_node(
            id.clone(),
            NodeSpec::Function(Arc::new(SpendingNode {
                ledger: Arc::clone(&ledger_port),
                scope: LedgerScope::new("acme", "svc-h"),
                amount_nanos: if i == 0 { 1_000_000_000 } else { 0 },
                runs: Arc::clone(&counters[i]),
            })),
        );
    }
    for pair in ids.windows(2) {
        graph.add_edge(EdgeSpec {
            from: pair[0].clone(),
            to: pair[1].clone(),
            condition: None,
        });
    }
    graph.add_entry(ids[0].clone());
    let resolver: Arc<dyn AssistantResolver> =
        Arc::new(CodeWorkflowResolver::new().register("halt-wf", Arc::new(graph)));

    // D-02: the allowance grammar, deserialized exactly as an operator would write it.
    let config: TreasurerConfig = serde_json::from_value(serde_json::json!({
        "currency": "USD",
        "allowance": { "api_keys": { "svc-h": { "period": "1d", "amount": "1.00" } } }
    }))
    .unwrap();
    let treasurer = Arc::new(Treasurer::new(
        config.allowance_policy().unwrap(),
        Arc::clone(&ledger_port),
    ));
    let submission: Arc<dyn RunSubmissionPort> = Arc::new(
        RunSubmissionService::new(repository.clone(), queue.clone(), resolver.clone())
            .with_treasurer(Arc::clone(&treasurer)
                as Arc<
                    dyn paladin_ports::input::allowance_admission_port::AllowanceAdmissionPort,
                >),
    );

    let mut api_keys = HashMap::new();
    api_keys.insert(
        "halt-key".to_string(),
        Principal::new("svc-h", UserRole::User, TenantId::new("acme").unwrap()),
    );
    let auth = AgentAuthConfig {
        enabled: true,
        api_keys,
        token_verifier: None,
        bearer_tenant: None,
    };
    let app = run_router(
        RunApiState::new()
            .with_submission(submission)
            .with_repository(repository.clone())
            .with_auth(auth),
    );

    // The pool, built the way the warn-path tracer builds it (an engine factory carrying the
    // per-run cancellation token) plus the Treasurer, which attaches the per-run spend guard.
    let waypoints = Arc::new(InMemoryWaypointStore::new());
    let factory_store = waypoints.clone();
    let engine_factory: Arc<
        dyn Fn(tokio_util::sync::CancellationToken) -> WarEngine<InMemoryWaypointStore>
            + Send
            + Sync,
    > = Arc::new(move |token| {
        WarEngine::new(Arc::new(UnusedPaladinPort), factory_store.clone())
            .with_cancellation_token(token)
    });
    let mut pool = RunWorkerPool::new(
        Arc::new(WarEngine::new(
            Arc::new(UnusedPaladinPort),
            waypoints.clone(),
        )),
        waypoints.clone(),
        repository.clone(),
        queue.clone(),
        resolver.clone(),
        Duration::from_secs(30),
    )
    .with_engine_factory(engine_factory);
    if attach_treasurer {
        pool = pool.with_treasurer(Arc::clone(&treasurer));
    }

    let window_before = window_for(ledger.store_now().await.unwrap(), 86_400).unwrap();

    // (a) The submission is admitted: nothing has been spent yet.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/runs")
                .header("content-type", "application/json")
                .header("x-api-key", "halt-key")
                .body(Body::from(
                    serde_json::to_vec(&serde_json::json!({
                        "assistant_id": "halt-wf",
                        "input": {}
                    }))
                    .unwrap(),
                ))
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(response.status(), StatusCode::ACCEPTED);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read submit body");
    let submitted: serde_json::Value = serde_json::from_slice(&bytes).expect("submit body is JSON");
    let run_id = submitted["run_id"].as_str().expect("run_id").to_string();
    let thread_id = paladin_core::platform::container::waypoint::ThreadId::new(
        submitted["thread_id"].as_str().expect("thread_id"),
    )
    .unwrap();

    // (b) One worker dispatch runs the graph: n0 spends the whole allowance, the guard halts
    // the run at the second boundary.
    assert!(pool.run_once().await.unwrap());

    // (c) The run row reads halted with no error, through the real router.
    let got = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/runs/{run_id}"))
                .header("x-api-key", "halt-key")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    let window_after = window_for(ledger.store_now().await.unwrap(), 86_400).unwrap();
    if window_before != window_after {
        cleanup(&path);
        return false;
    }
    assert_eq!(got.status(), StatusCode::OK);
    let raw = axum::body::to_bytes(got.into_body(), usize::MAX)
        .await
        .expect("read run body");
    let body: serde_json::Value = serde_json::from_slice(&raw).expect("run body is JSON");
    assert_eq!(
        body["status"], "halted",
        "an exhausted allowance halts the run at its next boundary: {body}"
    );
    assert!(
        body["error"].is_null(),
        "a halt is a resume point, not a failure: {body}"
    );

    // (c2) D-06, G6, G7: the row carries the typed reason and the fork point. The reason is the
    // Phase 41 `429` figures plus `reason`, rendered from integer nano-units by the one wire
    // builder (D-00h).
    let halt_reason = body["halt_reason"]
        .as_object()
        .unwrap_or_else(|| panic!("a spend-halted run carries a halt_reason object: {body}"));
    let mut reason_keys: Vec<&str> = halt_reason.keys().map(String::as_str).collect();
    reason_keys.sort_unstable();
    assert_eq!(
        reason_keys,
        [
            "balance",
            "ceiling",
            "kind",
            "reason",
            "scope",
            "window_end",
            "window_start"
        ]
    );
    assert_eq!(halt_reason["reason"], "allowance_exhausted");
    assert_eq!(halt_reason["scope"], "api_key");
    assert_eq!(halt_reason["kind"], "window");
    assert_eq!(halt_reason["balance"], "1.0000 USD");
    assert_eq!(halt_reason["ceiling"], "1.0000 USD");
    assert!(halt_reason["window_start"].is_string());
    assert!(halt_reason["window_end"].is_string());

    // (d) The thread's latest Waypoint is the Halted restart point, vanguard [n1].
    let latest = waypoints
        .latest(&thread_id)
        .await
        .unwrap()
        .expect("the halted run left a Waypoint");
    assert_eq!(
        latest.status,
        paladin_core::platform::container::waypoint::WaypointStatus::Halted
    );
    assert_eq!(latest.vanguard, vec![ids[1].clone()]);
    // G7: `final_waypoint_id` IS the Halted Waypoint, the fork point to resume from.
    assert_eq!(
        body["final_waypoint_id"],
        latest.waypoint_id.to_string(),
        "final_waypoint_id must be the Halted Waypoint's id: {body}"
    );

    // (d2) The tenant-scoped list serves the identical reason and fork point for the same run.
    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/runs?thread_id={thread_id}"))
                .header("x-api-key", "halt-key")
                .body(Body::empty())
                .expect("request builds"),
        )
        .await
        .expect("router responds");
    assert_eq!(listed.status(), StatusCode::OK);
    let list_raw = axum::body::to_bytes(listed.into_body(), usize::MAX)
        .await
        .expect("read list body");
    let list_body: serde_json::Value =
        serde_json::from_slice(&list_raw).expect("list body is JSON");
    let items = list_body["items"]
        .as_array()
        .unwrap_or_else(|| panic!("the run list has an items array: {list_body}"));
    let row = items
        .iter()
        .find(|item| item["run_id"] == run_id.as_str())
        .unwrap_or_else(|| panic!("the halted run is listed: {list_body}"));
    assert_eq!(row["status"], "halted");
    assert_eq!(row["halt_reason"], body["halt_reason"]);
    assert_eq!(row["final_waypoint_id"], body["final_waypoint_id"]);
    let list_text = String::from_utf8_lossy(&list_raw);
    assert!(
        !list_text.contains("halt-key"),
        "the run list must not contain a key value: {list_text}"
    );

    // (e) n0 ran once; n1 and n2 never ran.
    let ran: Vec<usize> = counters.iter().map(|c| c.load(Ordering::SeqCst)).collect();
    assert_eq!(ran, [1, 0, 0], "no node of the halted superstep may run");

    // (f) D-00g: the response carries no key value.
    let raw_text = String::from_utf8_lossy(&raw);
    assert!(
        !raw_text.contains("halt-key"),
        "the run body must not contain a key value: {raw_text}"
    );

    cleanup(&path);
    true
}
