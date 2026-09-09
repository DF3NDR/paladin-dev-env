//! `LogTraceSink` — the default-on structured log consumer for the trace
//! stream (OBS-02, OBS-FR-04, D-11).
//!
//! Every [`TraceRecord`] becomes exactly one `log::info!` line under target
//! `paladin::trace`, whose message is the record's own JSON serialization
//! (`TraceRecord`'s `#[serde(flatten)]` shape already puts `thread_id`/
//! `seq`/`at`/`kind` first — see `paladin_core::platform::container::trace`'s
//! own module docs). An operator silences the stream with either
//! `trace.log_sink: false` (this sink is never attached) or
//! `RUST_LOG=paladin::trace=off` (the house `env_logger` stack drops the
//! line before it is written) — no code change either way (D-11).

use async_trait::async_trait;
use paladin_ports::output::trace_sink_port::{TraceRecord, TraceSink, TraceSinkError};

/// Writes one `log::info!` line per [`TraceRecord`] under target
/// `paladin::trace`. Diagnostics-only, per [`TraceSink`]'s own contract: a
/// serialization failure (this sink's own malfunction, never the run's)
/// logs a single `error!` line and still returns `Ok(())` — the sink never
/// fails a run.
#[derive(Debug, Default, Clone, Copy)]
pub struct LogTraceSink;

impl LogTraceSink {
    /// Construct a `LogTraceSink`. Stateless — every instance behaves
    /// identically.
    pub fn new() -> Self {
        Self
    }
}

/// Serialize `value` and write it as one `log::info!` line under target
/// `paladin::trace`; a serialization failure logs a single `error!`
/// diagnostic instead. Factored out of [`LogTraceSink::on_event`] so its
/// error path can be exercised directly with a value engineered to fail
/// serialization (`log_sink_never_returns_err_and_logs_diagnostic`) —
/// `TraceRecord` itself is always serializable in practice (every field is
/// a plain `Serialize` type), so this is the only way to prove the
/// diagnostics-only contract deterministically rather than by assertion.
fn write_trace_line<T: serde::Serialize>(value: &T) {
    match serde_json::to_string(value) {
        Ok(json) => {
            log::info!(target: "paladin::trace", "{json}");
        }
        Err(error) => {
            // Diagnostics-only (module docs): a serialization failure is
            // this sink's own malfunction, never a reason to fail the run
            // or the sink's own contract.
            log::error!(
                target: "paladin::trace",
                "LogTraceSink failed to serialize a TraceRecord: {error}"
            );
        }
    }
}

#[async_trait]
impl TraceSink for LogTraceSink {
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
        write_trace_line(&record);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use paladin_core::platform::container::run::RunId;
    use paladin_core::platform::container::waypoint::ThreadId;
    use paladin_ports::output::trace_sink_port::TraceEvent;
    use std::sync::{Arc, Mutex};

    /// A `log::Log` implementation that captures every record emitted at or
    /// above `Info` under a specific target, for asserting exactly what
    /// `LogTraceSink` wrote without depending on a real logger backend.
    /// Installed process-wide via `log::set_boxed_logger` (`log`'s single
    /// global slot). Every test that reads this logger's buffer is tagged
    /// `#[serial_test::serial]` -- including
    /// `super::super::tests::both_configured_fans_out_to_a_composite`
    /// (`mod.rs`), which also exercises `LogTraceSink` as a side effect of
    /// `build_run_sink` -- so no two tests in this test BINARY racing the
    /// one process-wide logger slot can ever interleave their own
    /// capture/assert windows, cross-module or not (`serial_test`'s default
    /// key group is shared crate-wide).
    struct CapturingLogger {
        records: Mutex<Vec<(String, log::Level, String)>>,
    }

    impl log::Log for CapturingLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::Level::Trace
        }

        fn log(&self, record: &log::Record) {
            self.records.lock().unwrap().push((
                record.target().to_string(),
                record.level(),
                record.args().to_string(),
            ));
        }

        fn flush(&self) {}
    }

    // `log`'s global logger can only be installed ONCE per process
    // (`set_boxed_logger` errors on a second call) -- install a single
    // capturing logger lazily and read its buffer per test.
    static CAPTURED: std::sync::OnceLock<Arc<CapturingLogger>> = std::sync::OnceLock::new();

    fn install_capturing_logger() -> Arc<CapturingLogger> {
        Arc::clone(CAPTURED.get_or_init(|| {
            let logger = Arc::new(CapturingLogger {
                records: Mutex::new(Vec::new()),
            });
            let boxed: Box<dyn log::Log> = Box::new(ArcLoggerShim(Arc::clone(&logger)));
            // Ignore "already set" -- another test module in this same test
            // binary may have installed a logger first; this module only
            // needs ITS OWN capturing logger to be the active one for the
            // records it inspects, achieved by the per-test lock plus
            // clearing the buffer before each assertion (see
            // `drain_records`).
            let _ = log::set_boxed_logger(boxed);
            log::set_max_level(log::LevelFilter::Trace);
            logger
        }))
    }

    /// A thin `log::Log` shim forwarding to a shared `Arc<CapturingLogger>`
    /// -- `log::set_boxed_logger` needs an owned `Box<dyn Log>`, but the
    /// test module also needs its own handle to read the buffer back.
    struct ArcLoggerShim(Arc<CapturingLogger>);
    impl log::Log for ArcLoggerShim {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            self.0.enabled(metadata)
        }
        fn log(&self, record: &log::Record) {
            self.0.log(record);
        }
        fn flush(&self) {
            self.0.flush();
        }
    }

    fn drain_records(logger: &CapturingLogger) -> Vec<(String, log::Level, String)> {
        std::mem::take(&mut *logger.records.lock().unwrap())
    }

    fn sample_record() -> TraceRecord {
        TraceRecord {
            thread_id: ThreadId::new("t1").unwrap(),
            run_id: Some(RunId::new_v7()),
            seq: 1,
            at: Utc::now(),
            event: TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "fp".to_string(),
            },
        }
    }

    /// Behavior: one record produces exactly one `info`-level line under
    /// target `paladin::trace` whose message parses as JSON, whose first
    /// three key NAMES to appear (in byte order) are `thread_id`, `seq` and
    /// `kind` -- `TraceRecord`'s own declared field order
    /// (`thread_id, run_id?, seq, at, ..event`) puts `kind` (the
    /// internally-tagged `TraceEvent` discriminant) right after `at`, so
    /// checking BYTE POSITION (not "exactly the first three JSON keys") is
    /// the accurate contract: `thread_id` before `seq` before `kind`,
    /// each still trivially grep-able.
    #[tokio::test]
    #[serial_test::serial]
    async fn log_sink_writes_one_json_line_per_record() {
        let logger = install_capturing_logger();
        drain_records(&logger);

        let sink = LogTraceSink::new();
        sink.on_event(sample_record()).await.unwrap();

        let records = drain_records(&logger);
        let trace_records: Vec<_> = records
            .iter()
            .filter(|(target, _, _)| target == "paladin::trace")
            .collect();
        assert_eq!(
            trace_records.len(),
            1,
            "exactly one paladin::trace line per record: {records:?}"
        );
        let (_, level, message) = trace_records[0];
        assert_eq!(*level, log::Level::Info);

        let parsed: serde_json::Value =
            serde_json::from_str(message).expect("the log line must parse as JSON");
        assert!(parsed.is_object());

        let thread_id_pos = message
            .find("\"thread_id\"")
            .expect("thread_id key present");
        let seq_pos = message.find("\"seq\"").expect("seq key present");
        let kind_pos = message.find("\"kind\"").expect("kind key present");
        assert!(
            thread_id_pos < seq_pos && seq_pos < kind_pos,
            "thread_id, seq and kind must appear in that order, grep-able among the first \
             keys: {message}"
        );
    }

    /// A value engineered to fail `serde_json::to_string` deterministically
    /// -- `TraceRecord` itself always serializes successfully in practice,
    /// so this is how the diagnostics-only error path is exercised for
    /// real, rather than merely asserted.
    struct AlwaysFailsSerialize;
    impl serde::Serialize for AlwaysFailsSerialize {
        fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom(
                "deliberate serialization failure",
            ))
        }
    }

    /// Behavior: `on_event` never returns `Err` -- `LogTraceSink`'s own
    /// method signature has no `Err` arm at all -- and a serialization
    /// failure is swallowed into `Ok(())` with exactly one `error`-level
    /// diagnostic under `paladin::trace`, never a `panic` and never a
    /// second (info-level) line.
    #[test]
    #[serial_test::serial]
    fn log_sink_never_returns_err_and_logs_diagnostic() {
        let logger = install_capturing_logger();
        drain_records(&logger);

        // Exercises the exact same code path `on_event` calls, with a
        // value that deterministically fails to serialize.
        write_trace_line(&AlwaysFailsSerialize);

        let records = drain_records(&logger);
        let trace_records: Vec<_> = records
            .iter()
            .filter(|(target, _, _)| target == "paladin::trace")
            .collect();
        assert_eq!(
            trace_records.len(),
            1,
            "exactly one diagnostic line, never a panic and never a second line: {records:?}"
        );
        assert_eq!(trace_records[0].1, log::Level::Error);
    }

    /// Structural witness that `LogTraceSink::on_event`'s signature commits
    /// to `Result<(), TraceSinkError>` and its own real behavior (proven
    /// above) never constructs the `Err` variant.
    #[tokio::test]
    #[serial_test::serial]
    async fn on_event_always_returns_ok() {
        let sink = LogTraceSink::new();
        assert!(sink.on_event(sample_record()).await.is_ok());
    }
}
