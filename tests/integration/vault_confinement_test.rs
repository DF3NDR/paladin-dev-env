//! PRD 05 §3.4's namespace-confinement attack test, end to end through the
//! whole path a scripted, hostile model actually has to get past (Doc 05
//! RT-04, D-19, D-20, D-39, D-41):
//!
//! ```text
//! scripted model -> reasoning loop -> composite arsenal -> VaultTools
//!   -> ConfinedVault -> the store
//! ```
//!
//! Plan 26-13 already unit-tests `ConfinedVault` the decorator in isolation
//! (`crates/paladin-ports/src/output/vault_confined.rs`); the value here is
//! proving the layers actually compose when the caller is a real
//! `PaladinExecutionService` run driven by a scripted `LlmPort`, not a
//! direct method call on the decorator.
//!
//! # Mechanism used to drive the hostile tool call
//!
//! Plan 26-19's `ToolCallProtocolMiddleware` has not landed in this tree at
//! the time this test was written. The hostile tool call is driven through
//! [`MockLlmAdapter::with_script`], which populates
//! [`FunctionCall`](paladin_ports::output::llm_port::FunctionCall) directly
//! on the mocked response -- exactly the consumer-supplied-`LlmPort`
//! fallback the plan names. The property under test is confinement, not how
//! the tool call was produced, so this substitution does not weaken the
//! attack test: `PaladinExecutionService::execute_internal`'s dispatch from
//! `response_view.function_call` to `handle_tool_call` is unchanged either
//! way.
//!
//! # Zero-inner-call proof
//!
//! [`CountingVault`] wraps a real
//! [`InMemoryVault`](paladin_memory::vault::InMemoryVault), counting every
//! call it forwards. This is what makes "the store's call count is 0" a
//! real, direct assertion rather than an inference from "the call
//! returned an error" -- the property [`ConfinedVault`] exists to provide.

use async_trait::async_trait;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use paladin::application::services::paladin::paladin_builder::PaladinBuilder;
use paladin::application::services::paladin::paladin_execution_service::PaladinExecutionService;
use paladin::infrastructure::resilience::circuit_breaker::CircuitBreaker;
use paladin_core::platform::container::paladin::Paladin;
use paladin_core::platform::container::run_scope::RunScope;
use paladin_llm::mock::{MockLlmAdapter, MockScriptEntry};
use paladin_ports::output::llm_port::LlmPort;
use paladin_ports::output::vault_port::{
    Namespace, Page, ScoredVaultRecord, VaultError, VaultPort, VaultRecord,
};

/// Wraps a real [`InMemoryVault`](paladin_memory::vault::InMemoryVault),
/// counting every call forwarded to it -- the difference between "the call
/// failed" and "the call never reached the backend" (module documentation
/// above).
struct CountingVault {
    inner: paladin_memory::vault::InMemoryVault,
    calls: AtomicUsize,
}

impl CountingVault {
    fn new() -> Self {
        Self {
            inner: paladin_memory::vault::InMemoryVault::new(),
            calls: AtomicUsize::new(0),
        }
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl VaultPort for CountingVault {
    async fn put(&self, ns: &Namespace, key: &str, value: Value) -> Result<(), VaultError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.put(ns, key, value).await
    }

    async fn get(&self, ns: &Namespace, key: &str) -> Result<Option<VaultRecord>, VaultError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.get(ns, key).await
    }

    async fn delete(&self, ns: &Namespace, key: &str) -> Result<bool, VaultError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.delete(ns, key).await
    }

    async fn list(
        &self,
        ns: &Namespace,
        prefix: Option<&str>,
        page: Page,
    ) -> Result<Vec<VaultRecord>, VaultError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.list(ns, prefix, page).await
    }

    async fn search(
        &self,
        ns: &Namespace,
        query: &str,
        top_k: u32,
    ) -> Result<Vec<ScoredVaultRecord>, VaultError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.inner.search(ns, query, top_k).await
    }
}

fn test_circuit_breaker() -> Arc<CircuitBreaker> {
    Arc::new(CircuitBreaker::new(5, 3, Duration::from_secs(60)))
}

/// Builds a `PaladinExecutionService` with vault tools opted in, `store` as
/// its Vault backend (no service-default namespace -- every test's grant
/// comes from its own `RunScope`), and `llm` as the scripted model.
fn build_service(store: Arc<dyn VaultPort>, llm: Arc<dyn LlmPort>) -> PaladinExecutionService {
    PaladinExecutionService::new(llm, test_circuit_breaker(), None, None)
        .with_vault(store, None)
        .enable_vault_tools()
}

async fn build_paladin(llm: Arc<dyn LlmPort>, max_loops: u32) -> Paladin {
    PaladinBuilder::new(llm)
        .system_prompt("You are a test agent with Vault tools.")
        .max_loops(max_loops)
        .build()
        .await
        .expect("test Paladin should build")
}

fn hostile_put(namespace_json: Value) -> MockScriptEntry {
    MockScriptEntry::ToolCall {
        name: "vault_put".to_string(),
        arguments: json!({
            "namespace": namespace_json,
            "key": "x",
            "value": 1
        })
        .to_string(),
    }
}

/// Test 1: grant `["user","alice"]`; a scripted model emits
/// `vault_put {"namespace": ["user","bob"], ...}`. Denied, fed back as a
/// tool error, the run completes, and the store's call count is exactly 0.
#[tokio::test]
async fn hostile_tool_call_to_a_sibling_namespace_is_denied() {
    let store = Arc::new(CountingVault::new());
    // A single deny call on the run's only loop: `accumulated_output` is
    // overwritten (not accumulated) at the start of every loop iteration
    // (`PaladinExecutionService::execute_internal`'s own documented
    // per-iteration reset), so putting the hostile call on the FINAL loop
    // is what makes its fed-back tool-error text observable in the
    // returned `PaladinResult.output` below.
    let llm: Arc<dyn LlmPort> =
        Arc::new(MockLlmAdapter::new().with_script(vec![hostile_put(json!(["user", "bob"]))]));
    let service = build_service(store.clone(), llm.clone());
    let paladin = build_paladin(llm, 1).await;
    let scope = RunScope::default().with_vault_namespace(Namespace::parse("user/alice").unwrap());

    let result = service
        .execute_scoped(&paladin, "attempt a sibling write", None, &scope)
        .await
        .expect("the run must complete even though the tool call was denied");

    assert!(
        result
            .output
            .contains("outside this agent's granted namespace"),
        "the denial must reach the model as a tool error, got: {}",
        result.output
    );
    assert_eq!(
        store.call_count(),
        0,
        "the store must never be touched when the namespace is denied"
    );
}

/// Test 2: the string-prefix trap -- `["user","alice2"]` is NOT a
/// descendant of `["user","alice"]` even though it shares a string prefix.
#[tokio::test]
async fn hostile_tool_call_to_a_lookalike_sibling_is_denied() {
    let store = Arc::new(CountingVault::new());
    let llm: Arc<dyn LlmPort> =
        Arc::new(MockLlmAdapter::new().with_script(vec![hostile_put(json!(["user", "alice2"]))]));
    let service = build_service(store.clone(), llm.clone());
    let paladin = build_paladin(llm, 1).await;
    let scope = RunScope::default().with_vault_namespace(Namespace::parse("user/alice").unwrap());

    let result = service
        .execute_scoped(&paladin, "attempt a lookalike-sibling write", None, &scope)
        .await
        .expect("the run must complete even though the tool call was denied");

    assert!(
        result
            .output
            .contains("outside this agent's granted namespace"),
        "a string-prefix lookalike must still be denied, got: {}",
        result.output
    );
    assert_eq!(store.call_count(), 0);
}

/// Test 3: the parent -- `["user"]` is an ancestor of the grant, not a
/// descendant, and must be denied.
#[tokio::test]
async fn hostile_tool_call_to_the_parent_is_denied() {
    let store = Arc::new(CountingVault::new());
    let llm: Arc<dyn LlmPort> =
        Arc::new(MockLlmAdapter::new().with_script(vec![hostile_put(json!(["user"]))]));
    let service = build_service(store.clone(), llm.clone());
    let paladin = build_paladin(llm, 1).await;
    let scope = RunScope::default().with_vault_namespace(Namespace::parse("user/alice").unwrap());

    let result = service
        .execute_scoped(&paladin, "attempt a parent write", None, &scope)
        .await
        .expect("the run must complete even though the tool call was denied");

    assert!(
        result
            .output
            .contains("outside this agent's granted namespace"),
        "the parent namespace must be denied, got: {}",
        result.output
    );
    assert_eq!(store.call_count(), 0);
}

/// Test 4: a traversal segment, an empty segment and an over-long segment
/// each fail `Namespace::new`'s own invariants BEFORE `ConfinedVault` is
/// ever consulted -- a different layer than Tests 1-3 on purpose (defence
/// in depth: namespace validity is checked before namespace authorization).
#[tokio::test]
async fn a_traversal_segment_never_constructs() {
    // 1. A `..` segment.
    {
        let store = Arc::new(CountingVault::new());
        let llm: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new().with_script(vec![hostile_put(json!(["user", "alice", ".."]))]),
        );
        let service = build_service(store.clone(), llm.clone());
        let paladin = build_paladin(llm, 1).await;
        let scope =
            RunScope::default().with_vault_namespace(Namespace::parse("user/alice").unwrap());

        let result = service
            .execute_scoped(&paladin, "attempt a traversal write", None, &scope)
            .await
            .expect("the run must complete");
        assert!(
            result.output.contains("FAILED"),
            "a `..` segment must fail, got: {}",
            result.output
        );
        assert_eq!(
            store.call_count(),
            0,
            "Namespace::new must reject this before the store is touched"
        );
    }

    // 2. An empty segment.
    {
        let store = Arc::new(CountingVault::new());
        let llm: Arc<dyn LlmPort> =
            Arc::new(MockLlmAdapter::new().with_script(vec![hostile_put(json!(["user", ""]))]));
        let service = build_service(store.clone(), llm.clone());
        let paladin = build_paladin(llm, 1).await;
        let scope =
            RunScope::default().with_vault_namespace(Namespace::parse("user/alice").unwrap());

        let result = service
            .execute_scoped(&paladin, "attempt an empty-segment write", None, &scope)
            .await
            .expect("the run must complete");
        assert!(
            result.output.contains("FAILED"),
            "an empty segment must fail, got: {}",
            result.output
        );
        assert_eq!(store.call_count(), 0);
    }

    // 3. A 65-character segment (over the 64-char bound).
    {
        let store = Arc::new(CountingVault::new());
        let over_long = "a".repeat(65);
        let llm: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new().with_script(vec![hostile_put(json!(["user", over_long]))]),
        );
        let service = build_service(store.clone(), llm.clone());
        let paladin = build_paladin(llm, 1).await;
        let scope =
            RunScope::default().with_vault_namespace(Namespace::parse("user/alice").unwrap());

        let result = service
            .execute_scoped(&paladin, "attempt an over-long-segment write", None, &scope)
            .await
            .expect("the run must complete");
        assert!(
            result.output.contains("FAILED"),
            "an over-long segment must fail, got: {}",
            result.output
        );
        assert_eq!(store.call_count(), 0);
    }
}

/// Test 5: a denied call does not poison the run -- a subsequent in-grant
/// `vault_put` still succeeds.
#[tokio::test]
async fn a_granted_call_still_works_after_a_denied_one() {
    let store = Arc::new(CountingVault::new());
    let allowed_put = MockScriptEntry::ToolCall {
        name: "vault_put".to_string(),
        arguments: json!({
            "namespace": ["user", "alice"],
            "key": "color",
            "value": "green"
        })
        .to_string(),
    };
    // The denied call runs on loop 1, the allowed call on loop 2 (the run's
    // LAST loop) -- `accumulated_output` is overwritten, not accumulated,
    // at the start of each loop iteration, so only the final loop's text
    // survives into `PaladinResult.output` below. The denial's effect is
    // therefore verified against the store directly (no `bob` record),
    // not against the final output text.
    let llm: Arc<dyn LlmPort> = Arc::new(
        MockLlmAdapter::new().with_script(vec![hostile_put(json!(["user", "bob"])), allowed_put]),
    );
    let service = build_service(store.clone(), llm.clone());
    let paladin = build_paladin(llm, 2).await;
    let alice = Namespace::parse("user/alice").unwrap();
    let scope = RunScope::default().with_vault_namespace(alice.clone());

    let result = service
        .execute_scoped(&paladin, "deny then allow", None, &scope)
        .await
        .expect("the run must complete");

    assert!(
        result.output.contains("SUCCESS"),
        "the allowed write (the run's final loop) must succeed, got: {}",
        result.output
    );

    // Verify directly against the backend that the allowed write landed,
    // and the denied write never created anything under `bob`.
    let stored = store.inner.get(&alice, "color").await.unwrap();
    assert_eq!(stored.map(|r| r.value().clone()), Some(json!("green")));

    let bob = Namespace::parse("user/bob").unwrap();
    let bob_page = Page::new(10, None).unwrap();
    let bob_records = store.inner.list(&bob, None, bob_page).await.unwrap();
    assert!(
        bob_records.is_empty(),
        "the denied write must never have created a record under `bob`"
    );
}

/// Test 6 (X-05): N concurrent runs, each granted a distinct namespace and
/// each invoking `vault_put` M times, produce exactly M records per
/// namespace and zero records under any other namespace. Multi-thread
/// flavor with an explicit timeout guard and exact-count assertions
/// (`listener.rs`'s pattern, D-39).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_confined_tool_writes_produce_zero_cross_namespace_records() {
    const RUNS: usize = 5;
    const WRITES_PER_RUN: usize = 20;

    let store: Arc<dyn VaultPort> = Arc::new(paladin_memory::vault::InMemoryVault::new());

    let mut handles = Vec::with_capacity(RUNS);
    for run_index in 0..RUNS {
        let store = store.clone();
        handles.push(tokio::spawn(async move {
            let namespace = Namespace::parse(&format!("tenant/{run_index}")).unwrap();

            let script: Vec<MockScriptEntry> = (0..WRITES_PER_RUN)
                .map(|write_index| MockScriptEntry::ToolCall {
                    name: "vault_put".to_string(),
                    arguments: json!({
                        "namespace": ["tenant", run_index.to_string()],
                        "key": format!("item-{write_index}"),
                        "value": run_index
                    })
                    .to_string(),
                })
                .collect();

            let llm: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_script(script));
            let service = build_service(store, llm.clone());
            let paladin = build_paladin(llm, WRITES_PER_RUN as u32).await;
            let scope = RunScope::default().with_vault_namespace(namespace.clone());

            service
                .execute_scoped(&paladin, "concurrent writes", None, &scope)
                .await
                .expect("each concurrent run must complete");

            namespace
        }));
    }

    let namespaces = tokio::time::timeout(Duration::from_secs(30), async {
        let mut namespaces = Vec::with_capacity(RUNS);
        for handle in handles {
            namespaces.push(handle.await.expect("task must not panic"));
        }
        namespaces
    })
    .await
    .expect("the concurrent sweep must finish within the timeout guard");

    for (run_index, namespace) in namespaces.iter().enumerate() {
        let page = Page::new(1000, None).unwrap();
        let records = store.list(namespace, None, page).await.unwrap();
        assert_eq!(
            records.len(),
            WRITES_PER_RUN,
            "namespace {namespace} must hold exactly {WRITES_PER_RUN} records"
        );
        for record in &records {
            assert_eq!(
                record.value(),
                &json!(run_index),
                "every record under {namespace} must have been written by ITS OWN run"
            );
        }
    }
}
