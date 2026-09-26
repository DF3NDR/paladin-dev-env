//! `HeraldTraceSink` — the engine-path `ExecutionMetadata` producer (D-12, D-11b).
//!
//! Every other `TraceSink` in this module writes a below-the-engine record for
//! observability (`LogTraceSink`) or durability (`PersistingTraceSink`). This one is
//! different in purpose, not contract: it watches the trace stream for
//! [`TraceEvent::RunFinished`](paladin_core::platform::container::trace::TraceEvent::RunFinished),
//! builds an [`ExecutionMetadata`] from it through
//! [`ExecutionMetadata::from_run_finished`], and hands that to a [`Herald`], writing the
//! rendered summary to the `paladin::herald` log target at `info`.
//!
//! One summary per engine dispatch (`start`, `resume`, `resume_with`, `fork`), covering
//! that dispatch's own calls — the same one-`TraceDispatcher`-per-run cardinality every
//! other sink in this module already relies on. This is the engine's counterpart to the
//! agent loop's own streamed-completion producer
//! (`PaladinExecutionService::stream_execution_metadata`, 38-02): together they close
//! D-12 on both run paths.
//!
//! Renders metadata only — model, usage, duration, cost, run status — never prompt or
//! response content (T-38-26). A herald error is diagnostics-only
//! ([`TraceSinkError::Failed`]), per [`TraceSink`]'s own contract: it never fails the run
//! (T-38-27).

use std::sync::Arc;

use async_trait::async_trait;
use paladin_core::platform::container::herald::{ExecutionMetadata, Herald};
use paladin_ports::output::trace_sink_port::{TraceRecord, TraceSink, TraceSinkError};

/// The `log` target [`HeraldTraceSink`] writes its rendered summary to (D-11b), at `info`.
pub const HERALD_LOG_TARGET: &str = "paladin::herald";

/// A [`TraceSink`] that hands every [`TraceEvent::RunFinished`](paladin_core::platform::container::trace::TraceEvent::RunFinished)
/// record it sees to a [`Herald`] (D-12): builds an [`ExecutionMetadata`] via
/// [`ExecutionMetadata::from_run_finished`], calls
/// [`Herald::finalize_stream`], and logs the result under [`HERALD_LOG_TARGET`] at
/// `info`. Every other event is a no-op — see the module docs for the "one summary per
/// engine dispatch" cardinality this relies on.
pub struct HeraldTraceSink {
    herald: Arc<dyn Herald>,
    model_used: String,
}

impl HeraldTraceSink {
    /// Construct a sink that hands every `RunFinished` record it sees to `herald`,
    /// labeling the produced [`ExecutionMetadata`] with `model_used` (the engine run's
    /// single declared model, `"mixed"`, or `"none"` — see
    /// `RunWorkerPool`'s own `run_model_label`).
    pub fn new(herald: Arc<dyn Herald>, model_used: impl Into<String>) -> Self {
        Self {
            herald,
            model_used: model_used.into(),
        }
    }
}

impl std::fmt::Debug for HeraldTraceSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HeraldTraceSink")
            .field("herald", &self.herald.name())
            .field("model_used", &self.model_used)
            .finish()
    }
}

#[async_trait]
impl TraceSink for HeraldTraceSink {
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
        let Some(metadata) =
            ExecutionMetadata::from_run_finished(&record, self.model_used.as_str())
        else {
            return Ok(());
        };

        match self.herald.finalize_stream(&metadata) {
            Ok(text) => {
                log::info!(target: HERALD_LOG_TARGET, "{text}");
                Ok(())
            }
            Err(error) => Err(TraceSinkError::Failed(format!(
                "herald finalize_stream failed: {error}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Duration;

    use chrono::Utc;
    use paladin_battalion::engine::{
        EngineLimits, InputMapping, NodeSpec, RunOutcome, WarEngine, WarGraph,
    };
    use paladin_core::base::entity::node::Node;
    use paladin_core::platform::container::battlefield::{
        BattlefieldSchema, DispatchRule, FieldName, FieldSpec, StateDelta,
    };
    use paladin_core::platform::container::cost::{Cost, CurrencyCode, PriceRow, PriceTable};
    use paladin_core::platform::container::herald::{
        BattalionResult, HeraldError, PaladinError, PaladinResult, StreamChunk,
    };
    use paladin_core::platform::container::paladin::{MaxLoops, Paladin, PaladinData};
    use paladin_core::platform::container::run::RunId;
    use paladin_core::platform::container::token_usage::TokenUsage;
    use paladin_core::platform::container::trace::{RunFinishStatus, TraceEvent};
    use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
    use paladin_llm::mock::MockLlmAdapter;
    use paladin_llm::pricing::with_pricing;
    use paladin_ports::output::llm_port::LlmPort;
    use paladin_ports::output::paladin_port::{PaladinPort, PaladinStream};
    use paladin_storage::waypoint::in_memory::InMemoryWaypointStore;

    use crate::application::services::paladin::paladin_execution_service::PaladinExecutionService;
    use crate::infrastructure::adapters::herald::MarkdownHerald;
    use crate::infrastructure::adapters::herald::markdown_herald::MarkdownHeraldConfig;
    use crate::infrastructure::resilience::circuit_breaker::CircuitBreaker;

    /// A recording `Herald` test double: captures every `ExecutionMetadata` handed to
    /// `finalize_stream`, so a test can inspect exactly what `HeraldTraceSink` produced.
    #[derive(Default)]
    struct RecordingHerald {
        captured: Mutex<Vec<ExecutionMetadata>>,
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

        fn format_battalion_result(
            &self,
            _result: &BattalionResult,
        ) -> Result<String, HeraldError> {
            Ok(String::new())
        }

        fn format_stream_chunk(&self, _chunk: &StreamChunk) -> Result<Option<String>, HeraldError> {
            Ok(None)
        }

        fn finalize_stream(&self, metadata: &ExecutionMetadata) -> Result<String, HeraldError> {
            self.captured.lock().unwrap().push(metadata.clone());
            Ok(format!("captured {}", metadata.execution_id))
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

    fn run_finished_record(cost: Option<Cost>) -> TraceRecord {
        TraceRecord {
            thread_id: ThreadId::new("herald-sink-unit").unwrap(),
            run_id: Some(RunId::new_v7()),
            seq: 1,
            at: Utc::now(),
            event: TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 1,
                usage: TokenUsage::new(1_000, 2_000),
                cost,
                duration_ms: 10,
                trace_dropped_total: 0,
            },
        }
    }

    #[tokio::test]
    async fn herald_sink_hands_run_finished_to_the_herald() {
        let herald = Arc::new(RecordingHerald::default());
        let sink = HeraldTraceSink::new(Arc::clone(&herald) as Arc<dyn Herald>, "gpt-4");

        let cost = Cost::new(22_500_000, CurrencyCode::new("USD").unwrap());
        let record = run_finished_record(Some(cost));

        sink.on_event(record).await.expect("herald sink succeeds");

        let captured = herald.captured();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].cost_estimate, Some(0.0225));
        assert_eq!(captured[0].cost_currency(), Some("USD"));
    }

    /// Adapts a [`PaladinExecutionService`] to the engine-facing [`PaladinPort`] seam,
    /// mirroring `tracer_e2e.rs`'s own local adapter (no production adapter of this shape
    /// exists elsewhere in the tree).
    struct PaladinPortAdapter(Arc<PaladinExecutionService>);

    #[async_trait]
    impl PaladinPort for PaladinPortAdapter {
        async fn execute(
            &self,
            paladin: &Paladin,
            input: &str,
        ) -> Result<PaladinResult, PaladinError> {
            self.0.execute(paladin, input).await
        }

        async fn execute_stream(
            &self,
            _paladin: &Paladin,
            _input: &str,
        ) -> Result<PaladinStream, PaladinError> {
            unreachable!("this test's WarGraph never streams")
        }

        fn validate(&self, _paladin: &Paladin) -> Result<(), PaladinError> {
            Ok(())
        }
    }

    /// `max_loops: Fixed(1)` — otherwise `PaladinData::default()`'s `Fixed(3)` calls the
    /// (mocked) LLM three times per node (no tool call ever satisfies an early-finish
    /// middleware in this bare harness), tripling the priced cost this test asserts.
    fn make_paladin(name: &str, model: &str) -> Paladin {
        let data = PaladinData {
            name: name.to_string(),
            model: model.to_string(),
            max_loops: MaxLoops::Fixed(1),
            ..Default::default()
        };
        Node::new(data, Some(name.to_string()))
    }

    fn build_single_paladin_graph(model: &str) -> Arc<WarGraph> {
        let schema = BattlefieldSchema::new(vec![FieldSpec::new(
            FieldName::new("summary").unwrap(),
            DispatchRule::LastWrite,
            None,
            false,
        )]);
        let mut graph = WarGraph::new(schema, EngineLimits::default());
        let node_id = NodeId::new("summarizer");
        graph.add_node(
            node_id.clone(),
            NodeSpec::paladin(
                make_paladin("summarizer", model),
                InputMapping::new("summarize this"),
                FieldName::new("summary").unwrap(),
            ),
        );
        graph.add_entry(node_id);
        Arc::new(graph)
    }

    fn gpt4_price_table() -> Arc<PriceTable> {
        Arc::new(PriceTable::new(CurrencyCode::new("USD").unwrap()).with_row(
            "gpt-4",
            PriceRow::new(2_500_000_000, 10_000_000_000).expect("valid price row"),
        ))
    }

    /// Bounded wait for `HeraldTraceSink`'s async trace consumer to have delivered the
    /// run's `RunFinished` record, mirroring the engine crate's own
    /// `tokio::time::sleep`-based polling for its background dispatcher.
    async fn wait_for_capture(herald: &RecordingHerald) -> Vec<ExecutionMetadata> {
        for _ in 0..50 {
            let captured = herald.captured();
            if !captured.is_empty() {
                return captured;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        herald.captured()
    }

    #[tokio::test]
    async fn priced_engine_run_reaches_the_herald() {
        let table = gpt4_price_table();
        let mock: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_response("a short summary")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let priced_llm = with_pricing(mock, &table);

        let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
        let service = Arc::new(PaladinExecutionService::new(
            priced_llm,
            circuit_breaker,
            None,
            None,
        ));
        let paladin_port: Arc<dyn PaladinPort> = Arc::new(PaladinPortAdapter(service));
        let waypoints = Arc::new(InMemoryWaypointStore::new());

        let herald = Arc::new(RecordingHerald::default());
        let engine = WarEngine::new(paladin_port, waypoints).with_trace_sink(Arc::new(
            HeraldTraceSink::new(Arc::clone(&herald) as Arc<dyn Herald>, "gpt-4"),
        ));

        let graph = build_single_paladin_graph("gpt-4");
        let thread = ThreadId::new("priced-engine-run").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .expect("engine run succeeds");
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let captured = wait_for_capture(&herald).await;
        assert_eq!(captured.len(), 1, "exactly one RunFinished summary");
        assert_eq!(captured[0].cost_estimate, Some(0.0225));
        assert_eq!(captured[0].cost_currency(), Some("USD"));

        let markdown = MarkdownHerald::with_config(MarkdownHeraldConfig {
            include_colors: false,
            heading_level: 2,
        });
        let rendered = markdown
            .finalize_stream(&captured[0])
            .expect("markdown herald renders the captured metadata");
        assert!(
            rendered.contains("0.0225 USD"),
            "rendered summary must contain the priced cost: {rendered}"
        );
    }

    #[tokio::test]
    async fn unpriced_engine_run_reports_no_cost() {
        let table = gpt4_price_table();
        let mock: Arc<dyn LlmPort> = Arc::new(
            MockLlmAdapter::new()
                .with_response("a short summary")
                .with_token_usage_struct(TokenUsage::new(1_000, 2_000)),
        );
        let priced_llm = with_pricing(mock, &table);

        let circuit_breaker = Arc::new(CircuitBreaker::new(3, 2, Duration::from_secs(30)));
        let service = Arc::new(PaladinExecutionService::new(
            priced_llm,
            circuit_breaker,
            None,
            None,
        ));
        let paladin_port: Arc<dyn PaladinPort> = Arc::new(PaladinPortAdapter(service));
        let waypoints = Arc::new(InMemoryWaypointStore::new());

        let herald = Arc::new(RecordingHerald::default());
        let engine = WarEngine::new(paladin_port, waypoints).with_trace_sink(Arc::new(
            HeraldTraceSink::new(
                Arc::clone(&herald) as Arc<dyn Herald>,
                "p38-engine-unpriced",
            ),
        ));

        // No price row for this model -- the price table only carries "gpt-4".
        let graph = build_single_paladin_graph("p38-engine-unpriced");
        let thread = ThreadId::new("unpriced-engine-run").unwrap();
        let outcome = engine
            .start(&graph, thread, StateDelta::new())
            .await
            .expect("engine run succeeds");
        assert!(matches!(outcome, RunOutcome::Completed { .. }));

        let captured = wait_for_capture(&herald).await;
        assert_eq!(captured.len(), 1, "exactly one RunFinished summary");
        assert_eq!(captured[0].cost_estimate, None);
        assert_eq!(captured[0].cost_currency(), None);
    }
}
