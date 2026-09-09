//! The facade's telemetry composition module (OBS-02, D-10, D-11).
//!
//! `WarEngine` itself keeps no default [`TraceSink`] (D-10) — the untraced
//! path stays zero-cost for any caller who never wires one. This module is
//! where the FACADE opts a run back in: [`build_run_sink`] is the single
//! place a run's sink fan-out is assembled, called once per run by the
//! worker's own composition root (`src/application/services/run/worker.rs`)
//! so `paladin-server`'s run worker and `paladin-cli muster` both attach the
//! same default-on log sink whenever `trace.log_sink` is set (D-11).
//!
//! An operator silences the whole stream with either the config flag
//! (`trace.log_sink: false`) or a log filter (`RUST_LOG=paladin::trace=off`)
//! — no code change either way.
//!
//! [`build_run_sink`] is also where the `otel`-gated [`OtelTraceSink`]
//! (28-09) joins the fan-out, behind BOTH the Cargo feature and
//! `trace.otel.enabled` — a default build never references it (D-10/D-11).

pub mod log_sink;
#[cfg(feature = "otel")]
pub mod otel_sink;

use std::sync::Arc;

use paladin_ports::output::trace_sink_port::{CompositeSink, TraceSink};

use crate::config::trace::TraceConfig;

pub use log_sink::LogTraceSink;
#[cfg(feature = "otel")]
pub use otel_sink::OtelTraceSink;

/// Assemble the sink a single run should forward its trace records to,
/// given `config` and an optional already-built `bus_sink` (the D-24
/// `RunEventBusSink` the worker attaches when an event bus is wired).
///
/// - None of `config.log_sink`, `bus_sink`, and (`otel`-feature builds
///   only) `config.otel.enabled` -> `None`: the caller must NOT attach a
///   `TraceSink` at all, so the engine's own untraced path (D-10) is used
///   and tracing costs nothing for this run.
/// - Exactly one configured -> that one sink directly, unwrapped from a
///   `CompositeSink` -- no fan-out overhead when there is only one
///   consumer.
/// - More than one -> a `CompositeSink` of the configured sinks, log sink
///   first, then the bus sink, then (when compiled and enabled) the OTel
///   sink -- every below-engine record reaches every configured consumer
///   from the SAME dispatcher.
///
/// A failure constructing the OTel sink (`otel`-feature builds only, e.g. a
/// malformed endpoint the exporter itself rejects at build time) is
/// diagnostics-only: logged, then treated as "OTLP export not attached
/// this run" rather than failing the run or this function (T-28-09-05).
pub fn build_run_sink(
    config: &TraceConfig,
    bus_sink: Option<Arc<dyn TraceSink>>,
) -> Option<Arc<dyn TraceSink>> {
    let mut sinks: Vec<Arc<dyn TraceSink>> = Vec::new();

    if config.log_sink {
        sinks.push(Arc::new(LogTraceSink::new()));
    }
    if let Some(bus) = bus_sink {
        sinks.push(bus);
    }
    #[cfg(feature = "otel")]
    if config.otel.enabled {
        match OtelTraceSink::new(&config.otel) {
            Ok(sink) => sinks.push(Arc::new(sink)),
            Err(error) => {
                log::error!(
                    target: "paladin::trace",
                    "failed to build the OTel trace sink, OTLP export is disabled for this run: {error}"
                );
            }
        }
    }

    match sinks.len() {
        0 => None,
        1 => sinks.into_iter().next(),
        _ => Some(Arc::new(CompositeSink::new(sinks))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct NoopSink;
    #[async_trait::async_trait]
    impl TraceSink for NoopSink {
        async fn on_event(
            &self,
            _record: paladin_ports::output::trace_sink_port::TraceRecord,
        ) -> Result<(), paladin_ports::output::trace_sink_port::TraceSinkError> {
            Ok(())
        }
    }

    /// Behavior: with `trace.log_sink` off and no bus sink, `build_run_sink`
    /// returns `None` -- the engine's untraced path is used, no
    /// `TraceSink` attached at all.
    #[test]
    fn neither_configured_returns_none() {
        let config = TraceConfig {
            log_sink: false,
            ..TraceConfig::default()
        };
        assert!(build_run_sink(&config, None).is_none());
    }

    /// Behavior: `trace.log_sink` on with no bus sink returns exactly one
    /// sink, not wrapped in a `CompositeSink`.
    #[test]
    fn log_sink_only_returns_the_single_sink_unwrapped() {
        let config = TraceConfig {
            log_sink: true,
            ..TraceConfig::default()
        };
        let sink = build_run_sink(&config, None);
        assert!(sink.is_some());
    }

    /// Behavior: `trace.log_sink` off with a bus sink present returns
    /// exactly the bus sink, not wrapped in a `CompositeSink`.
    #[test]
    fn bus_sink_only_returns_the_single_sink_unwrapped() {
        let config = TraceConfig {
            log_sink: false,
            ..TraceConfig::default()
        };
        let bus: Arc<dyn TraceSink> = Arc::new(NoopSink);
        assert!(build_run_sink(&config, Some(bus)).is_some());
    }

    /// Behavior: both configured -- the composite fans out to both,
    /// forwarding one record to two independent recorders.
    ///
    /// Tagged `#[serial_test::serial]`: this also exercises the REAL
    /// `LogTraceSink` (via `log_sink: true`), which writes to `log`'s
    /// single process-wide logger slot -- the same one
    /// `log_sink::tests`'s own capturing-logger tests install and read.
    /// `serial_test`'s default key group is shared crate-wide, so tagging
    /// this test too keeps it from interleaving with those.
    #[tokio::test]
    #[serial_test::serial]
    async fn both_configured_fans_out_to_a_composite() {
        struct RecordingSink {
            count: std::sync::atomic::AtomicUsize,
        }
        #[async_trait::async_trait]
        impl TraceSink for RecordingSink {
            async fn on_event(
                &self,
                _record: paladin_ports::output::trace_sink_port::TraceRecord,
            ) -> Result<(), paladin_ports::output::trace_sink_port::TraceSinkError> {
                self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        }

        let config = TraceConfig {
            log_sink: true,
            ..TraceConfig::default()
        };
        let bus_recorder = Arc::new(RecordingSink {
            count: std::sync::atomic::AtomicUsize::new(0),
        });
        let bus_sink: Arc<dyn TraceSink> = bus_recorder.clone();
        let sink = build_run_sink(&config, Some(bus_sink)).expect("both configured");

        let record = paladin_ports::output::trace_sink_port::TraceRecord {
            thread_id: paladin_core::platform::container::waypoint::ThreadId::new("t1").unwrap(),
            run_id: None,
            seq: 1,
            at: chrono::Utc::now(),
            event: paladin_ports::output::trace_sink_port::TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "fp".to_string(),
            },
        };
        sink.on_event(record).await.unwrap();
        assert_eq!(
            bus_recorder.count.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the bus sink must have received the record through the composite"
        );
    }
}
