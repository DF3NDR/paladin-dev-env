//! Examples for `docs/src/user-guides/fault-tolerance.md` (Phase 25, D-32).
//!
//! Every `// ANCHOR:` region below is pulled into the Aegis user guide via
//! mdBook `{{#include}}`, so a sample in the guide cannot drift from the
//! landed API: `cargo check -p paladin-doc-examples` compiles all of them.
#![allow(unused_variables, unused_imports, dead_code)]

use std::sync::Arc;
use std::time::Duration;

use crate::support::{create_paladin, mock_paladin_port};

// ANCHOR: attach
use paladin_battalion::engine::{EngineLimits, InputMapping, NodeSpec, WarGraph};
use paladin_core::platform::container::aegis::{Aegis, RetryPolicy, TimeoutPolicy};
use paladin_core::platform::container::battlefield::{
    BattlefieldSchema, DispatchRule, FieldName, FieldSpec,
};
use paladin_core::platform::container::waypoint::NodeId;

/// Attach an `Aegis` to one node, with a graph-wide default for the rest.
pub fn attach_an_aegis() -> Result<WarGraph, Box<dyn std::error::Error>> {
    let draft = FieldName::new("draft")?;
    let review = FieldName::new("review")?;
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(draft.clone(), DispatchRule::LastWrite, None, false),
        FieldSpec::new(review.clone(), DispatchRule::LastWrite, None, false),
    ]);

    let mut graph = WarGraph::new(schema, EngineLimits::default());
    graph.add_node(
        NodeId::new("writer"),
        NodeSpec::paladin(
            create_paladin("Writer"),
            InputMapping::new("Draft a reply to: {draft}"),
            draft,
        ),
    );
    graph.add_node(
        NodeId::new("reviewer"),
        NodeSpec::paladin(
            create_paladin("Reviewer"),
            InputMapping::new("Review this draft: {draft}"),
            review,
        ),
    );

    // Every node with no entry of its own gets three attempts and a
    // 30-second per-attempt wall clock ...
    graph.with_default_aegis(Aegis {
        retry: Some(RetryPolicy::default()),
        timeout: Some(TimeoutPolicy {
            run_timeout: Some(Duration::from_secs(30)),
            idle_timeout: None,
        }),
        ..Default::default()
    });

    // ... while `writer`'s own entry wins WHOLESALE: it is retried up to
    // five times and, because this Aegis sets no `timeout`, it carries no
    // per-attempt bound at all -- the default's 30 s is NOT merged in.
    graph.set_aegis(
        NodeId::new("writer"),
        Aegis {
            retry: Some(RetryPolicy {
                max_attempts: 5,
                ..RetryPolicy::default()
            }),
            ..Default::default()
        },
    );

    Ok(graph)
}
// ANCHOR_END: attach

// ANCHOR: transience
use paladin_core::platform::container::paladin_error::PaladinError;
use paladin_core::platform::container::transience::Transience;
use paladin_ports::output::llm_port::LlmError;

/// Classification is read from typed fields, never parsed out of a message.
pub fn classify() {
    // A 503 from a provider is worth retrying ...
    let overloaded = LlmError::ProviderError {
        provider: "openai".to_string(),
        status: 503,
        message: "upstream overloaded".to_string(),
    };
    assert_eq!(overloaded.transience(), Transience::Transient);

    // ... a rejected credential is not, however many times you resend it.
    let rejected = LlmError::AuthenticationError("invalid api key".to_string());
    assert_eq!(rejected.transience(), Transience::Permanent);

    // A configuration error is permanent by construction.
    let misconfigured = PaladinError::ConfigurationError("no model".to_string());
    assert_eq!(misconfigured.transience(), Transience::Permanent);

    // A bare, untyped message cannot be classified confidently.
    let opaque = PaladinError::ExecutionError("something happened".to_string());
    assert_eq!(opaque.transience(), Transience::Unknown);
}
// ANCHOR_END: transience

// ANCHOR: retry
use paladin_core::platform::container::aegis::RetryPredicate;

/// A retry policy spelled out field by field (these are the defaults).
pub fn retry_policy() -> RetryPolicy {
    RetryPolicy {
        max_attempts: 3,                              // attempts, including the first
        initial_interval: Duration::from_millis(500), // the wait before attempt 2
        backoff_factor: 2.0,                          // 500 ms, 1 s, 2 s, 4 s, ...
        max_interval: Duration::from_secs(60),        // every computed wait is capped here
        jitter: true,                                 // + uniform [0, delay) on each wait
        retry_on: RetryPredicate::TransientOnly,      // Permanent and Unknown get one attempt
    }
}
// ANCHOR_END: retry

// ANCHOR: custom_predicate
use async_trait::async_trait;
use paladin_battalion::engine::WarEngine;
use paladin_battalion::retry_predicate::{RetryPredicateError, RetryPredicateEvaluator};
use paladin_core::platform::container::node_error::{NodeError, NodeErrorSource};
use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

/// Retry only rate limits, and only twice.
struct RateLimitOnly;

#[async_trait]
impl RetryPredicateEvaluator for RateLimitOnly {
    async fn allows(&self, err: &NodeError, attempt: u32) -> Result<bool, RetryPredicateError> {
        let rate_limited = matches!(
            err.source,
            NodeErrorSource::Llm {
                status: Some(429),
                ..
            }
        );
        Ok(rate_limited && attempt <= 3)
    }
}

/// Register the predicate on the engine; a policy names it by string.
pub fn engine_with_custom_predicate() -> WarEngine<InMemoryWaypointStore> {
    WarEngine::new(mock_paladin_port(), Arc::new(InMemoryWaypointStore::new()))
        .with_retry_predicate("rate-limit-only", Arc::new(RateLimitOnly))
}

/// The policy that resolves to the registered predicate above.
pub fn policy_naming_the_predicate() -> RetryPolicy {
    RetryPolicy {
        retry_on: RetryPredicate::Custom("rate-limit-only".to_string()),
        ..RetryPolicy::default()
    }
}
// ANCHOR_END: custom_predicate

// ANCHOR: timeout
/// A per-attempt wall clock and a progress-aware idle bound, together.
pub fn timeout_policy() -> Aegis {
    Aegis {
        timeout: Some(TimeoutPolicy {
            // No single attempt may run longer than this, progress or not.
            run_timeout: Some(Duration::from_secs(120)),
            // ... but an attempt that reports no progress for 10 s is
            // stalled and is cut long before the wall clock would cut it.
            idle_timeout: Some(Duration::from_secs(10)),
        }),
        ..Default::default()
    }
}
// ANCHOR_END: timeout

// ANCHOR: heartbeat
use paladin_battalion::engine::{NodeContext, StateNode, StateNodeError};
use paladin_core::platform::container::battlefield::{Battlefield, StateDelta};
use paladin_core::platform::container::directive::Directive;

/// A long-running Function node that reports progress as it goes.
struct BatchScorer;

#[async_trait]
impl StateNode for BatchScorer {
    async fn run(
        &self,
        state: &Battlefield,
        ctx: &NodeContext,
    ) -> Result<Directive, StateNodeError> {
        let mut delta = StateDelta::new();
        for chunk in 0..10 {
            // ... score one chunk ...
            ctx.heartbeat(); // each beat restarts the idle window
        }
        delta
            .set(
                FieldName::new("scored").map_err(|e| StateNodeError(e.to_string()))?,
                true,
            )
            .map_err(|e| StateNodeError(e.to_string()))?;
        Ok(delta.into())
    }
}
// ANCHOR_END: heartbeat

// ANCHOR: handlers
use paladin_core::platform::container::aegis::ErrorHandlerSpec;

/// The three handler shapes an `Aegis.on_error` can carry.
pub fn handler_specs() -> Result<[ErrorHandlerSpec; 3], Box<dyn std::error::Error>> {
    // Route: write the structured NodeError into `booking_error` and run
    // `cancel` next, in place of the failed node's static successors.
    let route = ErrorHandlerSpec::Route {
        to: NodeId::new("cancel"),
        error_field: FieldName::new("booking_error")?,
    };

    // Absorb: merge this delta instead and carry on down the static edges.
    let mut fallback_delta = StateDelta::new();
    fallback_delta.set(FieldName::new("summary")?, "unavailable")?;
    let absorb = ErrorHandlerSpec::Absorb { fallback_delta };

    // Custom: hand the error to a handler registered by this name.
    let custom = ErrorHandlerSpec::Custom("escalate".to_string());

    Ok([route, absorb, custom])
}
// ANCHOR_END: handlers

// ANCHOR: compensation
/// PRD 04's compensation chain: `book` fails permanently, `cancel` runs.
pub fn compensation_chain(
    cancel: Arc<dyn StateNode>,
) -> Result<WarGraph, Box<dyn std::error::Error>> {
    let booking = FieldName::new("booking")?;
    let booking_error = FieldName::new("booking_error")?;
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(booking.clone(), DispatchRule::LastWrite, None, false),
        // The error field must be declared, with any dispatch but `Sum`.
        FieldSpec::new(booking_error.clone(), DispatchRule::LastWrite, None, false),
    ]);

    let mut graph = WarGraph::new(schema, EngineLimits::default());
    graph.add_node(
        NodeId::new("book"),
        NodeSpec::paladin(
            create_paladin("Booker"),
            InputMapping::new("Book the flight"),
            booking,
        ),
    );
    // `cancel` is reachable ONLY through the handler: no static edge, no
    // `mark_dynamic_target` -- a Route target is eligible by declaration.
    graph.add_node(NodeId::new("cancel"), NodeSpec::Function(cancel));

    graph.set_aegis(
        NodeId::new("book"),
        Aegis {
            // A permanent failure (a rejected credential, say) takes ONE
            // attempt under the default predicate -- the handler runs at
            // once, not after three wasted retries.
            retry: Some(RetryPolicy::default()),
            on_error: Some(ErrorHandlerSpec::Route {
                to: NodeId::new("cancel"),
                error_field: booking_error,
            }),
            ..Default::default()
        },
    );
    Ok(graph)
}
// ANCHOR_END: compensation

// ANCHOR: custom_handler
use paladin_battalion::error_handler::ErrorHandler;
use paladin_core::platform::container::directive::NextStep;

/// A handler that ends the run cleanly instead of failing it.
struct EndTheRun;

#[async_trait]
impl ErrorHandler for EndTheRun {
    async fn handle(&self, err: &NodeError, state: &Battlefield) -> Result<Directive, NodeError> {
        // `err` is the structured NodeError; `state` is the Battlefield as
        // it was BEFORE this superstep (the failed attempt wrote nothing).
        Ok(Directive {
            delta: StateDelta::new(),
            next: NextStep::End,
        })
    }
}

/// Register the handler; `ErrorHandlerSpec::Custom("end-the-run")` names it.
pub fn engine_with_custom_handler() -> WarEngine<InMemoryWaypointStore> {
    WarEngine::new(mock_paladin_port(), Arc::new(InMemoryWaypointStore::new()))
        .with_error_handler("end-the-run", Arc::new(EndTheRun))
}
// ANCHOR_END: custom_handler

// ANCHOR: fallback
use paladin_llm::fallback::FallbackLlmAdapter;
use paladin_llm::mock::MockLlmAdapter;
use paladin_ports::output::llm_port::LlmPort;

/// An ordered chain: try `primary`, then `backup`, then `local`.
pub fn fallback_chain() -> Result<Arc<dyn LlmPort>, Box<dyn std::error::Error>> {
    let primary: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_provider_name("openai"));
    let backup: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_provider_name("anthropic"));
    let local: Arc<dyn LlmPort> = Arc::new(MockLlmAdapter::new().with_provider_name("ollama"));

    let chain = FallbackLlmAdapter::new(vec![primary, backup, local])?;
    assert_eq!(chain.get_provider_name(), "fallback");

    // Hand the chain to a PaladinBuilder / PaladinExecutionService exactly
    // where a single provider adapter would go -- it is a plain `LlmPort`.
    Ok(Arc::new(chain))
}
// ANCHOR_END: fallback

// ANCHOR: cache
use paladin_core::platform::container::aegis::{CacheKeySpec, CachePolicy};
use paladin_core::platform::container::battlefield::CacheMarker;
use paladin_storage::node_cache::InMemoryNodeCache;

/// Wire a cache backend on the engine and a cache policy on one node.
pub fn cached_graph()
-> Result<(WarEngine<InMemoryWaypointStore>, WarGraph), Box<dyn std::error::Error>> {
    // 1. The backend is an engine concern. Without one, ANY `cache` policy
    //    in the graph fails validation before a node runs.
    let engine = WarEngine::new(mock_paladin_port(), Arc::new(InMemoryWaypointStore::new()))
        .with_node_cache(Arc::new(InMemoryNodeCache::new()));

    // 2. A field that must never be served from a cached delta opts out
    //    at the schema: here an `Append` log, which a fork would otherwise
    //    replay one more time on every hit.
    let question = FieldName::new("question")?;
    let answer = FieldName::new("answer")?;
    let audit_log = FieldName::new("audit_log")?;
    let schema = BattlefieldSchema::new(vec![
        FieldSpec::new(question.clone(), DispatchRule::LastWrite, None, false),
        FieldSpec::new(answer.clone(), DispatchRule::LastWrite, None, false),
        FieldSpec::new(audit_log, DispatchRule::Append, None, false).with_cache(CacheMarker::Deny),
    ]);

    // 3. The policy is a node concern: a TTL and a key composition.
    let mut graph = WarGraph::new(schema, EngineLimits::default());
    graph.add_node(
        NodeId::new("answerer"),
        NodeSpec::paladin(
            create_paladin("Answerer"),
            InputMapping::new("Answer: {question}"),
            answer,
        ),
    );
    graph.set_aegis(
        NodeId::new("answerer"),
        Aegis {
            cache: Some(CachePolicy {
                ttl: Duration::from_secs(15 * 60),
                // `Default` keys on the graph fingerprint, the node id, the
                // rendered input and the Paladin's own configuration;
                // `Fields(..)` adds (or, for a Function node, narrows to)
                // the named fields' values.
                key: CacheKeySpec::Fields(vec![question]),
            }),
            ..Default::default()
        },
    );

    Ok((engine, graph))
}
// ANCHOR_END: cache
