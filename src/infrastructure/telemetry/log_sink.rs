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
//!
//! A disabled target costs neither serialisation nor formatting: `on_event`
//! asks the installed logger whether `paladin::trace` is enabled at `Info`
//! and returns `Ok(())` without touching `serde_json` when it is not
//! (OBS-05, D-17).

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

thread_local! {
    /// Per-thread scratch buffer `write_trace_line` serialises into, reused
    /// across records so the hot path allocates no fresh `String` per record
    /// (OBS-05, D-17 fix 2). A thread-local rather than a field on
    /// [`LogTraceSink`]: any field would break the unit-struct literal and
    /// drop `Copy`, changing the public surface.
    static TRACE_BUF: std::cell::RefCell<Vec<u8>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// The largest buffer capacity retained between records (64 KiB). A record
/// that grew the buffer past this is followed by a release of that memory,
/// so a single oversize record never pins its allocation for the life of the
/// thread (T-45-12: no unbounded buffering).
const TRACE_BUF_RETAIN_MAX: usize = 64 * 1024;

/// Serialize `value` into `buf` (cleared first) and write it as one
/// `log::info!` line under target `paladin::trace`; a serialization failure
/// (or a non-UTF-8 result, which `serde_json` never produces) logs a single
/// `error!` diagnostic naming only the error -- never record content --
/// instead.
fn write_into<T: serde::Serialize>(buf: &mut Vec<u8>, value: &T) {
    buf.clear();
    match serde_json::to_writer(&mut *buf, value) {
        Ok(()) => match std::str::from_utf8(buf) {
            Ok(json) => {
                log::info!(target: "paladin::trace", "{json}");
            }
            Err(error) => {
                log::error!(
                    target: "paladin::trace",
                    "LogTraceSink produced non-UTF-8 JSON: {error}"
                );
            }
        },
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

/// Serialize `value` and write it as one `log::info!` line under target
/// `paladin::trace`; a serialization failure logs a single `error!`
/// diagnostic instead. Factored out of [`LogTraceSink::on_event`] so its
/// error path can be exercised directly with a value engineered to fail
/// serialization (`log_sink_never_returns_err_and_logs_diagnostic`) —
/// `TraceRecord` itself is always serializable in practice (every field is
/// a plain `Serialize` type), so this is the only way to prove the
/// diagnostics-only contract deterministically rather than by assertion.
///
/// Serialises into the per-thread [`TRACE_BUF`], which is borrowed with
/// `try_borrow_mut`: a re-entrant call (a logger that itself emits a trace
/// line) falls back to a fresh local buffer instead of panicking. The borrow
/// is synchronous and never held across an `.await`.
fn write_trace_line<T: serde::Serialize>(value: &T) {
    TRACE_BUF.with(|cell| match cell.try_borrow_mut() {
        Ok(mut buf) => {
            write_into(&mut buf, value);
            if buf.capacity() > TRACE_BUF_RETAIN_MAX {
                *buf = Vec::new();
            }
        }
        Err(_) => write_into(&mut Vec::new(), value),
    });
}

/// The current capacity of this thread's reusable trace buffer, for the
/// bounded-retention test.
#[cfg(test)]
fn trace_buf_capacity() -> usize {
    TRACE_BUF.with(|cell| cell.try_borrow().map(|buf| buf.capacity()).unwrap_or(0))
}

/// Whether the operator's logger currently accepts `Info` records under
/// target `paladin::trace`.
///
/// Reads the real filter through the installed logger's `enabled` (so
/// `RUST_LOG=paladin::trace=off` silences the stream, D-11) rather than
/// mirroring it in config. With no logger installed the `log` crate's max
/// level is `Off`, so this is `false`.
fn trace_target_enabled() -> bool {
    log::log_enabled!(target: "paladin::trace", log::Level::Info)
}

/// Write `value` as one trace line only when the `paladin::trace` target is
/// enabled; returns whether a line was attempted.
///
/// A disabled target costs neither JSON serialisation nor message
/// formatting (OBS-05, D-17 fix 1). The guard deliberately lives here and
/// not inside [`write_trace_line`], so that function's serialization-failure
/// diagnostic stays directly testable with an unserialisable value.
fn write_trace_line_if_enabled<T: serde::Serialize>(value: &T) -> bool {
    if !trace_target_enabled() {
        return false;
    }
    write_trace_line(value);
    true
}

#[async_trait]
impl TraceSink for LogTraceSink {
    async fn on_event(&self, record: TraceRecord) -> Result<(), TraceSinkError> {
        // Enablement guard first: a filtered target must not pay for
        // serialisation (OBS-05). The return value only says whether a line
        // was attempted; the sink never fails either way.
        write_trace_line_if_enabled(&record);
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

    /// A value whose `Serialize` impl counts how many times it is called and
    /// serialises a unit -- the witness that a disabled `paladin::trace`
    /// target never reaches `serde_json`.
    struct CountingValue<'a>(&'a std::sync::atomic::AtomicUsize);
    impl serde::Serialize for CountingValue<'_> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            serializer.serialize_unit()
        }
    }

    /// Sets the process-wide `log` max level for one test and restores
    /// `Trace` on drop, so a failed assertion never leaves the level `Off`
    /// for sibling tests sharing the one logger slot.
    struct MaxLevelGuard;
    impl MaxLevelGuard {
        fn set(level: log::LevelFilter) -> Self {
            log::set_max_level(level);
            Self
        }
    }
    impl Drop for MaxLevelGuard {
        fn drop(&mut self) {
            log::set_max_level(log::LevelFilter::Trace);
        }
    }

    /// Behavior: with the `paladin::trace` target disabled the guard returns
    /// `false`, never invokes `Serialize` and writes no line; with it
    /// enabled it serialises exactly once and writes exactly one line
    /// (OBS-05, D-17 fix 1).
    #[test]
    #[serial_test::serial]
    fn log_sink_skips_serialisation_when_the_trace_target_is_disabled() {
        let logger = install_capturing_logger();
        let calls = std::sync::atomic::AtomicUsize::new(0);

        {
            let _off = MaxLevelGuard::set(log::LevelFilter::Off);
            drain_records(&logger);
            assert!(!write_trace_line_if_enabled(&CountingValue(&calls)));
            assert_eq!(
                calls.load(std::sync::atomic::Ordering::SeqCst),
                0,
                "a disabled target must not serialise the record"
            );
            let records = drain_records(&logger);
            assert!(
                records
                    .iter()
                    .all(|(target, _, _)| target != "paladin::trace"),
                "a disabled target must write no line: {records:?}"
            );
        }

        let _on = MaxLevelGuard::set(log::LevelFilter::Trace);
        drain_records(&logger);
        assert!(write_trace_line_if_enabled(&CountingValue(&calls)));
        assert_eq!(
            calls.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "an enabled target serialises exactly once"
        );
        let records = drain_records(&logger);
        let trace_lines = records
            .iter()
            .filter(|(target, _, _)| target == "paladin::trace")
            .count();
        assert_eq!(trace_lines, 1, "exactly one line when enabled: {records:?}");
    }

    /// Behavior: `on_event` with the target disabled still returns `Ok(())`
    /// (the diagnostics-only contract) and writes no `paladin::trace` line.
    #[tokio::test]
    #[serial_test::serial]
    async fn on_event_writes_no_line_when_the_trace_target_is_disabled() {
        let logger = install_capturing_logger();
        let _off = MaxLevelGuard::set(log::LevelFilter::Off);
        drain_records(&logger);

        let sink = LogTraceSink::new();
        assert!(sink.on_event(sample_record()).await.is_ok());

        let records = drain_records(&logger);
        assert!(
            records
                .iter()
                .all(|(target, _, _)| target != "paladin::trace"),
            "a disabled target must write no line: {records:?}"
        );
    }

    /// The `paladin::trace` lines captured since the last drain, in order.
    fn trace_lines(logger: &CapturingLogger) -> Vec<String> {
        drain_records(logger)
            .into_iter()
            .filter(|(target, level, _)| target == "paladin::trace" && *level == log::Level::Info)
            .map(|(_, _, message)| message)
            .collect()
    }

    fn record_with_seq(seq: u64, event: TraceEvent) -> TraceRecord {
        TraceRecord {
            seq,
            event,
            ..sample_record()
        }
    }

    /// Behavior: two different records written back to back through the
    /// reused per-thread buffer yield two lines, each byte-equal to that
    /// record's own JSON -- no bleed from the first into the second, no
    /// truncation (OBS-05, D-17 fix 2).
    #[test]
    #[serial_test::serial]
    fn log_sink_reuses_its_buffer_without_bleeding_between_records() {
        let logger = install_capturing_logger();
        let _on = MaxLevelGuard::set(log::LevelFilter::Trace);
        drain_records(&logger);

        let long = record_with_seq(
            1,
            TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "a-considerably-longer-fingerprint-value".to_string(),
            },
        );
        let short = record_with_seq(
            2,
            TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "x".to_string(),
            },
        );
        write_trace_line(&long);
        write_trace_line(&short);

        let lines = trace_lines(&logger);
        assert_eq!(lines.len(), 2, "one line per record: {lines:?}");
        assert_eq!(lines[0], serde_json::to_string(&long).unwrap());
        assert_eq!(lines[1], serde_json::to_string(&short).unwrap());
    }

    /// Behavior: a record far larger than `TRACE_BUF_RETAIN_MAX` followed by
    /// a small one produces two exact lines, and the retained buffer never
    /// exceeds the retention bound afterwards (T-45-12: no unbounded
    /// buffering).
    #[test]
    #[serial_test::serial]
    fn log_sink_writes_an_oversize_record_then_a_small_record_exactly() {
        use paladin_core::platform::container::trace::MiddlewareAction;

        let logger = install_capturing_logger();
        let _on = MaxLevelGuard::set(log::LevelFilter::Trace);
        drain_records(&logger);

        let oversize = record_with_seq(
            1,
            TraceEvent::MiddlewareEvent {
                name: "m".repeat(70_000),
                action: MiddlewareAction::Finish,
            },
        );
        let small = record_with_seq(
            2,
            TraceEvent::RunStarted {
                run_id: None,
                graph_fingerprint: "fp".to_string(),
            },
        );
        write_trace_line(&oversize);
        assert!(
            trace_buf_capacity() <= TRACE_BUF_RETAIN_MAX,
            "an oversize record must not leave its capacity retained"
        );
        write_trace_line(&small);

        let lines = trace_lines(&logger);
        assert_eq!(lines.len(), 2, "one line per record");
        assert_eq!(lines[0], serde_json::to_string(&oversize).unwrap());
        assert_eq!(lines[1], serde_json::to_string(&small).unwrap());
        assert!(
            trace_buf_capacity() <= TRACE_BUF_RETAIN_MAX,
            "the retained buffer stays within the bound"
        );
    }
}
