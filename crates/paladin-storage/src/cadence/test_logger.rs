//! A process-wide capturing `log::Log` for the cadence tests (test-only).
//!
//! `log::set_logger` may be called once per process, so every cadence test module that asserts
//! on a log line shares this one logger. Records under [`CADENCE_LOG_TARGET`] are stored per OS
//! thread id, so concurrently running `#[tokio::test]`s (each on its own thread, current-thread
//! runtime) never observe each other's lines.

use std::collections::HashMap;
use std::sync::{Mutex, Once, PoisonError};

use paladin_ports::output::cadence_port::CADENCE_LOG_TARGET;

/// One captured record: its level and its rendered message.
pub(crate) type Line = (log::Level, String);

struct CapturingLogger;

static LOGGER_INIT: Once = Once::new();
static CAPTURED: Mutex<Option<HashMap<std::thread::ThreadId, Vec<Line>>>> = Mutex::new(None);

impl log::Log for CapturingLogger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if record.target() == CADENCE_LOG_TARGET {
            let mut guard = CAPTURED.lock().unwrap_or_else(PoisonError::into_inner);
            guard
                .get_or_insert_with(HashMap::new)
                .entry(std::thread::current().id())
                .or_default()
                .push((record.level(), record.args().to_string()));
        }
    }

    fn flush(&self) {}
}

/// Install the capturing logger (at most once per process) at `Info` verbosity.
pub(crate) fn install() {
    static LOGGER: CapturingLogger = CapturingLogger;
    LOGGER_INIT.call_once(|| {
        log::set_logger(&LOGGER).expect("install the capturing test logger");
        log::set_max_level(log::LevelFilter::Info);
    });
}

/// Every cadence-target record captured on the calling thread so far, oldest first.
pub(crate) fn lines_for_this_thread() -> Vec<Line> {
    CAPTURED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .and_then(|lines| lines.get(&std::thread::current().id()).cloned())
        .unwrap_or_default()
}

/// The messages of the `warn` records captured on the calling thread.
pub(crate) fn warnings_for_this_thread() -> Vec<String> {
    lines_for_this_thread()
        .into_iter()
        .filter(|(level, _)| *level == log::Level::Warn)
        .map(|(_, message)| message)
        .collect()
}
