//! Shared `RunTracePort` contract suite.
//!
//! One generic async function per contract clause, each taking `&dyn
//! RunTracePort` and asserting inside -- mirroring
//! `waypoint::contract_tests`'s own convention (D-09). Every backend
//! (`InMemoryRunTraceStore`, `SqliteRunTraceStore`, `PostgresRunTraceStore`)
//! invokes these unchanged from its own `#[tokio::test]`s.
//!
//! One clause -- [`unsupported_schema_version_is_typed`] -- cannot be
//! expressed purely over `&dyn RunTracePort`: writing a row with an
//! unsupported schema version bypasses the normal [`RunTracePort::append`]
//! path (which always stamps the CURRENT schema version), so that clause
//! takes a store implementing the additional [`RawSchemaVersionWriter`]
//! test-only capability instead, and is called directly by each backend's
//! own test module rather than from [`run_all`].
//!
//! This module is plain (not `#[cfg(test)]`) so both unit tests inside each
//! backend crate and future Docker-gated integration tests can call it.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;

use paladin_core::platform::container::trace::{TraceEvent, TraceRecord};
use paladin_core::platform::container::waypoint::{NodeId, ThreadId};
use paladin_ports::output::run_trace_port::{RunTraceError, RunTracePort};

/// Build a `TraceRecord` fixture for `thread` at `seq`, wrapping a
/// `NodeStarted` event stamped with `superstep`. Every contract function
/// (and every backend's own test harness) should build fixtures through
/// this function so all backends exercise identical inputs.
pub fn sample_record(thread: &ThreadId, seq: u64, superstep: u64) -> TraceRecord {
    TraceRecord {
        thread_id: thread.clone(),
        run_id: None,
        seq,
        at: Utc::now(),
        event: TraceEvent::NodeStarted {
            superstep,
            node_id: NodeId::new("n1"),
            attempt: 1,
            muster_task_key: None,
        },
    }
}

fn record_superstep(record: &TraceRecord) -> u64 {
    match &record.event {
        TraceEvent::NodeStarted { superstep, .. } => *superstep,
        other => panic!("sample_record fixtures always construct NodeStarted, got {other:?}"),
    }
}

/// `append` then `read(thread, 0, 100)` returns all five records, ascending
/// by `seq`, each equal to the one appended.
pub async fn append_then_read_round_trips(port: &dyn RunTracePort) {
    let thread = ThreadId::new("contract-append-read-round-trip").unwrap();
    let records: Vec<TraceRecord> = (1..=5).map(|seq| sample_record(&thread, seq, 0)).collect();
    port.append(&records).await.unwrap();

    let read_back = port.read(&thread, 0, 100).await.unwrap();
    assert_eq!(read_back.len(), 5);
    for (expected, actual) in records.iter().zip(read_back.iter()) {
        assert_eq!(expected, actual);
    }
    let seqs: Vec<u64> = read_back.iter().map(|r| r.seq).collect();
    assert_eq!(seqs, vec![1, 2, 3, 4, 5]);
}

/// `read(thread, 0, 2)` then `read(thread, last_seq, 2)` then again walks
/// the five records exactly once, in order, ending with an empty vector.
pub async fn read_paginates_by_after_seq(port: &dyn RunTracePort) {
    let thread = ThreadId::new("contract-read-paginates").unwrap();
    let records: Vec<TraceRecord> = (1..=5).map(|seq| sample_record(&thread, seq, 0)).collect();
    port.append(&records).await.unwrap();

    let mut collected = Vec::new();
    let mut after_seq = 0u64;
    loop {
        let page = port.read(&thread, after_seq, 2).await.unwrap();
        if page.is_empty() {
            break;
        }
        after_seq = page.last().unwrap().seq;
        collected.extend(page);
    }

    let seqs: Vec<u64> = collected.iter().map(|r| r.seq).collect();
    assert_eq!(seqs, vec![1, 2, 3, 4, 5]);

    // One more call past the end returns empty, not an error.
    assert_eq!(port.read(&thread, after_seq, 2).await.unwrap(), vec![]);
}

/// `read` on a thread with no persisted rows returns `Ok(vec![])`.
pub async fn read_of_unknown_thread_is_empty_not_error(port: &dyn RunTracePort) {
    let thread = ThreadId::new("contract-read-unknown-thread").unwrap();
    assert_eq!(port.read(&thread, 0, 100).await.unwrap(), vec![]);
}

/// Appending the same `(thread_id, seq)` twice leaves exactly one row and
/// does not error.
pub async fn append_is_idempotent_on_same_seq(port: &dyn RunTracePort) {
    let thread = ThreadId::new("contract-append-idempotent").unwrap();
    let record = sample_record(&thread, 1, 0);
    port.append(std::slice::from_ref(&record)).await.unwrap();
    port.append(&[record]).await.unwrap();

    let read_back = port.read(&thread, 0, 100).await.unwrap();
    assert_eq!(read_back.len(), 1);
}

/// Two threads' records never appear in each other's `read`.
pub async fn records_are_scoped_by_thread(port: &dyn RunTracePort) {
    let thread_a = ThreadId::new("contract-scope-thread-a").unwrap();
    let thread_b = ThreadId::new("contract-scope-thread-b").unwrap();
    port.append(&[sample_record(&thread_a, 1, 0)])
        .await
        .unwrap();
    port.append(&[sample_record(&thread_b, 1, 0)])
        .await
        .unwrap();

    let a_records = port.read(&thread_a, 0, 100).await.unwrap();
    let b_records = port.read(&thread_b, 0, 100).await.unwrap();
    assert_eq!(a_records.len(), 1);
    assert_eq!(b_records.len(), 1);
    assert_eq!(a_records[0].thread_id, thread_a);
    assert_eq!(b_records[0].thread_id, thread_b);
}

/// `prune_thread(thread, 3)` deletes exactly the rows whose `superstep` is
/// below 3 and returns that count.
pub async fn prune_thread_removes_only_older_supersteps(port: &dyn RunTracePort) {
    let thread = ThreadId::new("contract-prune-older-supersteps").unwrap();
    let records: Vec<TraceRecord> = (0..6u64)
        .map(|superstep| sample_record(&thread, superstep + 1, superstep))
        .collect();
    port.append(&records).await.unwrap();

    let removed = port.prune_thread(&thread, 3).await.unwrap();
    assert_eq!(removed, 3);

    let remaining = port.read(&thread, 0, 100).await.unwrap();
    let supersteps: Vec<u64> = remaining.iter().map(record_superstep).collect();
    assert_eq!(supersteps, vec![3, 4, 5]);
}

/// An extra capability the schema-version clause below needs: writing a
/// row with an arbitrary `schema_version`, bypassing the normal
/// [`RunTracePort::append`] path (which always stamps the CURRENT schema
/// version). Each backend implements this for its own store type -- it is
/// deliberately not part of `RunTracePort` itself, since a caller must
/// never be able to write an unsupported schema version through the
/// port's public contract.
#[async_trait]
pub trait RawSchemaVersionWriter {
    /// Write `record` with `schema_version` verbatim, upserting if the
    /// `(thread_id, seq)` already exists.
    async fn write_with_schema_version(&self, record: &TraceRecord, schema_version: &str);
}

/// A row written with a `schema_version` of `"999"` is surfaced as
/// [`RunTraceError::UnsupportedSchemaVersion`] on read.
pub async fn unsupported_schema_version_is_typed<S>(store: &S)
where
    S: RunTracePort + RawSchemaVersionWriter,
{
    let thread = ThreadId::new("contract-unsupported-schema-version").unwrap();
    let record = sample_record(&thread, 1, 0);
    store.write_with_schema_version(&record, "999").await;

    let err = store.read(&thread, 0, 100).await.unwrap_err();
    match err {
        RunTraceError::UnsupportedSchemaVersion { found } => assert_eq!(found, "999"),
        other => panic!("expected UnsupportedSchemaVersion, got: {other:?}"),
    }
}

/// Run every clause expressible purely over `&dyn RunTracePort` (six of the
/// seven `<behavior>` clauses -- `unsupported_schema_version_is_typed`
/// needs [`RawSchemaVersionWriter`] too and is called directly by each
/// backend's own test module instead).
pub async fn run_all(store: Arc<dyn RunTracePort>) {
    append_then_read_round_trips(store.as_ref()).await;
    read_paginates_by_after_seq(store.as_ref()).await;
    read_of_unknown_thread_is_empty_not_error(store.as_ref()).await;
    append_is_idempotent_on_same_seq(store.as_ref()).await;
    records_are_scoped_by_thread(store.as_ref()).await;
    prune_thread_removes_only_older_supersteps(store.as_ref()).await;
}
