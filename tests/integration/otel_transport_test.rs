//! Hermetic OTLP transport assertion (28-09 Task 2, D-13b): proves the
//! `OtelTraceSink` built in Task 1 (`src/infrastructure/telemetry/otel_sink.rs`)
//! actually leaves the process over real OTLP/HTTP -- a request reaches a
//! stub server with `Content-Type: application/x-protobuf`, the configured
//! headers, and a body decoding to a resource carrying the configured
//! `service.name` -- and that its no-redirect client (T-28-09-01) really
//! never follows a `3xx`. No Docker, no external network, no fixed port:
//! an axum stub on an ephemeral `127.0.0.1` port, torn down at test end
//! (the Phase 12.1 `FixtureServer` pattern,
//! `tests/integration/mcp_streamable_http_test.rs`).
//!
//! Both tests use `#[tokio::test(flavor = "multi_thread")]` -- see
//! `otel_sink.rs`'s own module docs: `SimpleSpanProcessor` (this sink's
//! export path) plus an async `reqwest` client deadlocks under
//! `#[tokio::test]`'s DEFAULT `current_thread` runtime (empirically
//! verified, 28-09); `multi_thread` is required.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use tokio_util::sync::CancellationToken;

use paladin::config::trace::OtelConfig;
use paladin::infrastructure::telemetry::OtelTraceSink;
use paladin_ports::output::trace_sink_port::{RunFinishStatus, TraceEvent, TraceRecord, TraceSink};

/// One captured `POST /v1/traces` request: its headers and raw protobuf body.
#[derive(Clone)]
struct CapturedRequest {
    headers: HeaderMap,
    body: Bytes,
}

/// Shared capture state a stub's handler appends to.
type Captured = Arc<Mutex<Vec<CapturedRequest>>>;

async fn capture_handler(
    State(captured): State<Captured>,
    headers: HeaderMap,
    body: Bytes,
) -> &'static str {
    captured
        .lock()
        .unwrap()
        .push(CapturedRequest { headers, body });
    "ok"
}

/// Spawns a stub `POST /v1/traces` server on an ephemeral `127.0.0.1` port,
/// recording every request's headers and body. Returns its base URL (no
/// trailing path), the capture handle, and a `CancellationToken` the test
/// uses for graceful shutdown -- mirrors
/// `tests/integration/mcp_streamable_http_test.rs`'s `spawn_fixture_server`.
async fn spawn_capturing_stub() -> (String, Captured, CancellationToken) {
    let captured: Captured = Arc::new(Mutex::new(Vec::new()));
    let router = Router::new()
        .route("/v1/traces", post(capture_handler))
        .with_state(captured.clone());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind an ephemeral localhost port");
    let addr = listener.local_addr().expect("resolve bound local addr");

    let ct = CancellationToken::new();
    tokio::spawn({
        let ct = ct.clone();
        async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move { ct.cancelled_owned().await })
                .await;
        }
    });

    (format!("http://{addr}"), captured, ct)
}

/// A stub that answers every `POST /v1/traces` with a literal `302 Found`
/// redirecting to `target` -- for [`otlp_client_does_not_follow_redirects`].
async fn spawn_redirecting_stub(target: String) -> (String, CancellationToken) {
    async fn redirect_handler(State(target): State<Arc<String>>) -> Response {
        (StatusCode::FOUND, [(header::LOCATION, target.as_str())]).into_response()
    }

    let router = Router::new()
        .route("/v1/traces", post(redirect_handler))
        .with_state(Arc::new(target));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind an ephemeral localhost port");
    let addr = listener.local_addr().expect("resolve bound local addr");

    let ct = CancellationToken::new();
    tokio::spawn({
        let ct = ct.clone();
        async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move { ct.cancelled_owned().await })
                .await;
        }
    });

    (format!("http://{addr}"), ct)
}

/// Runs one minimal run+node fixture through `sink`, exercising the
/// `SimpleSpanProcessor`'s synchronous export path (`otel_sink.rs`'s own
/// module docs: export happens on `Span::end_with_timestamp`, no separate
/// flush call is needed).
async fn export_one_fixture_run(sink: &OtelTraceSink) {
    use paladin_core::platform::container::waypoint::{NodeId, NodeOutcomeKind, ThreadId};

    let thread_id = ThreadId::new("t-transport").unwrap();
    let node_id = NodeId::new("transport-node");
    let at = chrono::Utc::now();

    sink.on_event(TraceRecord {
        thread_id: thread_id.clone(),
        run_id: None,
        seq: 1,
        at,
        event: TraceEvent::RunStarted {
            run_id: None,
            graph_fingerprint: "fp".into(),
        },
    })
    .await
    .unwrap();

    sink.on_event(TraceRecord {
        thread_id: thread_id.clone(),
        run_id: None,
        seq: 2,
        at,
        event: TraceEvent::NodeStarted {
            superstep: 1,
            node_id: node_id.clone(),
            attempt: 1,
            muster_task_key: None,
        },
    })
    .await
    .unwrap();

    sink.on_event(TraceRecord {
        thread_id: thread_id.clone(),
        run_id: None,
        seq: 3,
        at,
        event: TraceEvent::NodeFinished {
            superstep: 1,
            node_id,
            attempt: 1,
            outcome: NodeOutcomeKind::Succeeded,
            duration_ms: 1,
            token_count: 0,
            cache_hit: false,
        },
    })
    .await
    .unwrap();

    sink.on_event(TraceRecord {
        thread_id,
        run_id: None,
        seq: 4,
        at,
        event: TraceEvent::RunFinished {
            status: RunFinishStatus::Completed,
            total_supersteps: 1,
            total_tokens: 0,
            duration_ms: 1,
            trace_dropped_total: 0,
        },
    })
    .await
    .unwrap();
}

/// Behavior: `otlp_export_reaches_the_stub` -- a fixture run's export
/// reaches a real axum stub with `Content-Type: application/x-protobuf`,
/// every header configured on `OtelConfig`, and a body decoding to a
/// resource whose `service.name` is the configured value.
#[tokio::test(flavor = "multi_thread")]
async fn otlp_export_reaches_the_stub() {
    let (base_url, captured, ct) = spawn_capturing_stub().await;

    let mut headers = HashMap::new();
    headers.insert(
        "authorization".to_string(),
        "Bearer transport-test-secret".to_string(),
    );
    headers.insert("x-paladin-test".to_string(), "otel-transport".to_string());
    let config = OtelConfig {
        enabled: true,
        endpoint: format!("{base_url}/v1/traces"),
        headers: headers.clone().into_iter().collect(),
        service_name: "paladin-transport-test".to_string(),
    };

    let sink = OtelTraceSink::new(&config).expect("sink builds against a valid stub endpoint");
    export_one_fixture_run(&sink).await;

    // SimpleSpanProcessor exports synchronously on `end_with_timestamp`
    // (`otel_sink.rs` module docs), but the request still crosses a real
    // socket to the spawned server task -- give it a moment to be accepted
    // and processed before asserting.
    tokio::time::sleep(Duration::from_millis(200)).await;
    ct.cancel();

    let requests = captured.lock().unwrap().clone();
    // `SimpleSpanProcessor` (this sink's export path, per `otel_sink.rs`'s
    // own module docs) exports on every `Span::end_with_timestamp` call
    // individually -- one HTTP POST per span, not batched. The fixture run
    // above ends two spans (the run root and the one node's attempt), so
    // exactly two requests reach the stub.
    let body_lengths: Vec<usize> = requests.iter().map(|r| r.body.len()).collect();
    assert_eq!(
        requests.len(),
        2,
        "expected one export POST per ended span, body lengths: {body_lengths:?}"
    );

    use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
    use prost::Message;

    let mut found_service_name = false;
    for request in &requests {
        assert_eq!(
            request
                .headers
                .get("content-type")
                .map(|v| v.to_str().unwrap()),
            Some("application/x-protobuf"),
            "OTLP/HTTP protobuf content type on every request"
        );
        for (key, value) in &headers {
            assert_eq!(
                request
                    .headers
                    .get(key.as_str())
                    .map(|v| v.to_str().unwrap()),
                Some(value.as_str()),
                "configured header `{key}` must reach the collector on every request"
            );
        }

        let decoded = ExportTraceServiceRequest::decode(request.body.as_ref())
            .expect("body decodes as ExportTraceServiceRequest");
        assert_eq!(
            decoded.resource_spans.len(),
            1,
            "one resource in the export batch"
        );
        let resource = decoded.resource_spans[0]
            .resource
            .as_ref()
            .expect("resource present");
        let service_name = resource
            .attributes
            .iter()
            .find(|kv| kv.key == "service.name")
            .and_then(|kv| kv.value.as_ref())
            .expect("service.name attribute present");
        let service_name_value = match &service_name.value {
            Some(opentelemetry_proto::tonic::common::v1::any_value::Value::StringValue(s)) => {
                s.clone()
            }
            other => panic!("expected a string service.name value, got {other:?}"),
        };
        assert_eq!(service_name_value, "paladin-transport-test");
        found_service_name = true;
    }
    assert!(
        found_service_name,
        "at least one request must have been inspected"
    );
}

/// Behavior: `otlp_client_does_not_follow_redirects` -- a stub answering
/// `302` with a `Location` pointing at a second stub leaves the second
/// stub with zero requests (T-28-09-01).
#[tokio::test(flavor = "multi_thread")]
async fn otlp_client_does_not_follow_redirects() {
    let (second_base_url, second_captured, second_ct) = spawn_capturing_stub().await;
    let (redirect_base_url, redirect_ct) =
        spawn_redirecting_stub(format!("{second_base_url}/v1/traces")).await;

    let config = OtelConfig {
        enabled: true,
        endpoint: format!("{redirect_base_url}/v1/traces"),
        headers: std::collections::BTreeMap::new(),
        service_name: "paladin-redirect-test".to_string(),
    };

    let sink = OtelTraceSink::new(&config).expect("sink builds against a valid endpoint");
    export_one_fixture_run(&sink).await;

    tokio::time::sleep(Duration::from_millis(200)).await;
    redirect_ct.cancel();
    second_ct.cancel();

    let second_requests = second_captured.lock().unwrap().clone();
    assert_eq!(
        second_requests.len(),
        0,
        "the redirect target must receive ZERO requests -- the client must never follow the 3xx"
    );
}
