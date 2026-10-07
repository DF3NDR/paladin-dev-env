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
//! The sink is stateful in one narrow way (Phase 41 D-18): it records every
//! [`AllowanceWarning`] event it sees and folds them into the metadata it hands the herald at
//! `RunFinished` ([`ExecutionMetadata::with_allowance_warnings`]), so each herald renders one
//! allowance line through the shared [`ExecutionMetadata::allowance_warning_display`] helper.
//! The worker builds one sink per run dispatch (never shared across runs), so the recorded
//! warnings are always the run's own.
//!
//! A run the Treasurer halted (Phase 42 D-19) carries its [`HaltReason`](paladin_core::platform::container::allowance::HaltReason) on its own
//! `RunFinished`; the sink folds it in through [`ExecutionMetadata::with_halt_reason`], so each
//! herald renders one halt line through [`ExecutionMetadata::halt_reason_display`] beside the
//! warning line. A run without a halt reason renders exactly as before.
//!
//! Renders metadata only — model, usage, duration, cost, allowance and halt figures, run status —
//! never prompt or response content (T-38-26). A herald error is diagnostics-only
//! ([`TraceSinkError::Failed`]), per [`TraceSink`]'s own contract: it never fails the run
//! (T-38-27).

use std::sync::{Arc, Mutex, MutexGuard};

use async_trait::async_trait;
use paladin_core::platform::container::allowance::AllowanceWarning;
use paladin_core::platform::container::herald::{ExecutionMetadata, Herald};
use paladin_core::platform::container::trace::TraceEvent;
use paladin_ports::output::trace_sink_port::{TraceRecord, TraceSink, TraceSinkError};

/// The `log` target [`HeraldTraceSink`] writes its rendered summary to (D-11b), at `info`.
pub const HERALD_LOG_TARGET: &str = "paladin::herald";

/// A [`TraceSink`] that hands every [`TraceEvent::RunFinished`](paladin_core::platform::container::trace::TraceEvent::RunFinished)
/// record it sees to a [`Herald`] (D-12): builds an [`ExecutionMetadata`] via
/// [`ExecutionMetadata::from_run_finished`], calls
/// [`Herald::finalize_stream`], and logs the result under [`HERALD_LOG_TARGET`] at
/// `info`. A [`TraceEvent::AllowanceWarning`](paladin_core::platform::container::trace::TraceEvent::AllowanceWarning)
/// is recorded and folded into that metadata (D-18); every other event is a no-op — see the
/// module docs for the "one summary per engine dispatch" cardinality this relies on.
pub struct HeraldTraceSink {
    herald: Arc<dyn Herald>,
    model_used: String,
    /// Allowance warnings seen on this run's stream, drained at `RunFinished` (D-18).
    warnings: Mutex<Vec<AllowanceWarning>>,
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
            warnings: Mutex::new(Vec::new()),
        }
    }

    /// Lock the warning list, recovering a poisoned lock: a panic elsewhere must not stop a
    /// herald summary (the list is only ever pushed to or drained).
    fn recorded_warnings(&self) -> MutexGuard<'_, Vec<AllowanceWarning>> {
        self.warnings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
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
        if let Some(warning) = AllowanceWarning::from_trace_event(&record.event) {
            self.recorded_warnings().push(warning);
            return Ok(());
        }

        let Some(mut metadata) =
            ExecutionMetadata::from_run_finished(&record, self.model_used.as_str())
        else {
            return Ok(());
        };
        let drained = std::mem::take(&mut *self.recorded_warnings());
        metadata.with_allowance_warnings(&drained);
        // D-19: a spend halt names its reason on the run's own `RunFinished`; fold it beside
        // the warning line so every herald renders one halt line. A finish without a reason
        // (or any non-halt status) leaves the metadata exactly as it was.
        if let TraceEvent::RunFinished {
            halt_reason: Some(reason),
            ..
        } = &record.event
        {
            metadata.with_halt_reason(reason);
        }

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

    use chrono::{TimeZone, Utc};
    use paladin_battalion::engine::{
        EngineLimits, InputMapping, NodeSpec, RunOutcome, WarEngine, WarGraph,
    };
    use paladin_core::base::entity::node::Node;
    use paladin_core::platform::container::allowance::HaltReason;
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
                halt_reason: None,
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

    fn allowance_warning_record(seq: u64) -> TraceRecord {
        use paladin_core::platform::container::allowance::{
            AllowanceLimitKind, AllowanceScopeKind,
        };
        let usd = CurrencyCode::new("USD").unwrap();
        TraceRecord {
            thread_id: ThreadId::new("herald-sink-unit").unwrap(),
            run_id: Some(RunId::new_v7()),
            seq,
            at: Utc::now(),
            event: TraceEvent::from(AllowanceWarning {
                scope_kind: AllowanceScopeKind::ApiKey,
                limit_kind: AllowanceLimitKind::Window,
                balance: Cost::new(20_500_000_000, usd.clone()),
                ceiling: Cost::new(25_000_000_000, usd),
                window_start: Utc.with_ymd_and_hms(2026, 10, 2, 0, 0, 0).single(),
                window_end: Utc.with_ymd_and_hms(2026, 10, 3, 0, 0, 0).single(),
                warn_at: 80,
            }),
        }
    }

    /// D-18: an allowance warning seen before `RunFinished` is folded into the metadata the
    /// herald receives, and only into that one summary.
    #[tokio::test]
    async fn herald_sink_folds_allowance_warnings_into_run_finished_metadata() {
        let herald = Arc::new(RecordingHerald::default());
        let sink = HeraldTraceSink::new(Arc::clone(&herald) as Arc<dyn Herald>, "gpt-4");

        sink.on_event(allowance_warning_record(1))
            .await
            .expect("recording a warning succeeds");
        assert!(
            herald.captured().is_empty(),
            "a warning alone renders nothing"
        );
        sink.on_event(run_finished_record(None))
            .await
            .expect("herald sink succeeds");

        let captured = herald.captured();
        assert_eq!(captured.len(), 1);
        assert_eq!(
            captured[0].allowance_warning_display().as_deref(),
            Some(
                "\u{26A0} allowance: 82% of 25.0000 USD (api_key, window resets 2026-10-03T00:00:00Z)"
            )
        );

        // Drained: a second dispatch's summary through the same sink carries no stale line.
        sink.on_event(run_finished_record(None))
            .await
            .expect("herald sink succeeds");
        assert_eq!(herald.captured()[1].allowance_warning_display(), None);
    }

    #[tokio::test]
    async fn herald_sink_without_warnings_renders_as_before() {
        let herald = Arc::new(RecordingHerald::default());
        let sink = HeraldTraceSink::new(Arc::clone(&herald) as Arc<dyn Herald>, "gpt-4");

        sink.on_event(run_finished_record(None))
            .await
            .expect("herald sink succeeds");

        let captured = herald.captured();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].allowance_warning_display(), None);
        assert!(!captured[0].metadata.contains_key(
            paladin_core::platform::container::herald::ALLOWANCE_WARNING_METADATA_KEY
        ));
    }

    fn halted_run_finished_record(halt_reason: Option<HaltReason>) -> TraceRecord {
        TraceRecord {
            thread_id: ThreadId::new("herald-sink-unit").unwrap(),
            run_id: Some(RunId::new_v7()),
            seq: 1,
            at: Utc::now(),
            event: TraceEvent::RunFinished {
                status: RunFinishStatus::Halted,
                total_supersteps: 1,
                usage: TokenUsage::new(1_000, 2_000),
                cost: None,
                halt_reason,
                duration_ms: 10,
                trace_dropped_total: 0,
            },
        }
    }

    /// D-19: a `RunFinished` carrying a halt reason is folded into the metadata as exactly
    /// one line; a finish without one leaves the metadata exactly as it was.
    #[tokio::test]
    async fn herald_sink_folds_a_halt_reason_into_one_line() {
        use paladin_core::platform::container::allowance::{
            AllowanceLimitKind, AllowanceRefusal, AllowanceScopeKind,
        };
        use paladin_core::platform::container::herald::HALT_REASON_METADATA_KEY;

        let herald = Arc::new(RecordingHerald::default());
        let sink = HeraldTraceSink::new(Arc::clone(&herald) as Arc<dyn Herald>, "gpt-4");
        let usd = CurrencyCode::new("USD").unwrap();
        let reason = HaltReason::AllowanceExhausted(AllowanceRefusal {
            scope_kind: AllowanceScopeKind::ApiKey,
            limit_kind: AllowanceLimitKind::Window,
            balance: Cost::new(25_000_000_000, usd.clone()),
            ceiling: Cost::new(25_000_000_000, usd),
            window: Utc
                .with_ymd_and_hms(2026, 10, 5, 0, 0, 0)
                .single()
                .zip(Utc.with_ymd_and_hms(2026, 10, 6, 0, 0, 0).single()),
            evaluated_at: Utc.with_ymd_and_hms(2026, 10, 5, 12, 0, 0).unwrap(),
        });

        sink.on_event(halted_run_finished_record(Some(reason)))
            .await
            .expect("herald sink succeeds");
        sink.on_event(halted_run_finished_record(None))
            .await
            .expect("herald sink succeeds");

        let captured = herald.captured();
        assert_eq!(captured.len(), 2);
        assert_eq!(
            captured[0].halt_reason_display().as_deref(),
            Some(
                "\u{26D4} halted: allowance exhausted \u{2014} 25.0000 of 25.0000 USD \
                 (api_key, window resets 2026-10-06T00:00:00Z)"
            )
        );
        assert_eq!(captured[1].halt_reason_display(), None);
        assert!(!captured[1].metadata.contains_key(HALT_REASON_METADATA_KEY));
    }

    /// 45-02 (D-14): a legacy agent run has no graph, so `RunWorkerPool::run_agent`
    /// feeds this sink `RunStarted { graph_fingerprint: "agent" }`, one superstep-0
    /// node pair and `RunFinished { total_supersteps: 0 }`. The sink must produce
    /// exactly one summary for that shape -- and never panic on it.
    #[tokio::test]
    async fn herald_sink_summarises_an_agent_shaped_run() {
        use paladin_core::platform::container::waypoint::NodeOutcomeKind;

        let herald = Arc::new(RecordingHerald::default());
        let sink = HeraldTraceSink::new(Arc::clone(&herald) as Arc<dyn Herald>, "gpt-4");
        let thread_id = ThreadId::new("herald-agent-shaped").unwrap();
        let run_id = RunId::new_v7();
        let record = |seq: u64, event: TraceEvent| TraceRecord {
            thread_id: thread_id.clone(),
            run_id: Some(run_id.clone()),
            seq,
            at: Utc::now(),
            event,
        };

        let events = vec![
            TraceEvent::RunStarted {
                run_id: Some(run_id.clone()),
                graph_fingerprint: "agent".to_string(),
            },
            TraceEvent::NodeStarted {
                superstep: 0,
                node_id: NodeId::new("code-agent"),
                attempt: 1,
                muster_task_key: None,
            },
            TraceEvent::NodeFinished {
                superstep: 0,
                node_id: NodeId::new("code-agent"),
                attempt: 1,
                outcome: NodeOutcomeKind::Succeeded,
                duration_ms: 5,
                usage: TokenUsage::new(11, 7),
                cost: None,
                cache_hit: false,
            },
            TraceEvent::RunFinished {
                status: RunFinishStatus::Completed,
                total_supersteps: 0,
                usage: TokenUsage::new(11, 7),
                cost: None,
                halt_reason: None,
                duration_ms: 5,
                trace_dropped_total: 0,
            },
        ];
        for (index, event) in events.into_iter().enumerate() {
            sink.on_event(record(index as u64 + 1, event))
                .await
                .expect("herald sink tolerates every agent-shaped record");
        }

        let captured = herald.captured();
        assert_eq!(captured.len(), 1, "exactly one summary per agent run");
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
